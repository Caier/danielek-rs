use crate::dapi::routes::common_types::Snowflake;
use crate::dapi::routes::{v10 as v10Routes, v6 as v6Routes};
use crate::dapi::versions::{v10, v6};
use crate::dapi::{DApi, DApiError};
use crate::gateway::error::GCResult;
use crate::gateway::shard::GatewayShard;
use crate::gateway::types::{
    GatewayActivityBuilder, GatewayActivityType, GatewayEvent, GatewayIntents, GatewayOpcode,
    GatewayPresenceBuilder, GatewayStatus,
};
use futures_util::StreamExt;
use lazy_regex::regex;
use log::info;
use once_cell::sync::Lazy;
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use uuid::Uuid;

use self::message_relay::{GiftRedeemAttempt, GiftReport, MessageRelay};

pub mod message_relay;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Default, Debug)]
struct SharedData {
    guilds: Mutex<HashMap<uuid::Uuid, HashSet<String>>>,
    used_codes: Mutex<HashSet<String>>,
}

static SHARED: Lazy<SharedData> = Lazy::new(Default::default);

pub struct GiftScanner {
    dapi: DApi<v10>,
    redeem_dapi: DApi<v6>,
    relay: Arc<MessageRelay>,
    id: uuid::Uuid,
    username: String,
    shard: GatewayShard,
    ignore: bool,
    command_channel: Snowflake,
    command_guild: Snowflake,
    ready_at: Option<Instant>,
    last_msg: Option<serde_json::Value>,
    guild_channel_names: HashMap<Snowflake, String>,
}

impl GiftScanner {
    pub async fn new(
        token: impl Into<String>,
        redeem_token: impl Into<String>,
        ignore: bool,
        command_channel: impl Into<String>,
        command_guild: impl Into<String>,
        relay: Arc<MessageRelay>,
    ) -> Result<Self> {
        let intents = GatewayIntents::GUILDS
            | GatewayIntents::MESSAGE_CONTENT
            | GatewayIntents::GUILD_MESSAGES
            | GatewayIntents::DIRECT_MESSAGES;

        let token = token.into();
        let shard = GatewayShard::new(token.clone(), intents, false).await?;

        let id = Uuid::new_v4();
        SHARED.guilds.lock().unwrap().insert(id, Default::default());

        let mut this = Self {
            dapi: DApi::new()?,
            redeem_dapi: DApi::new()?,
            relay,
            id,
            username: String::new(),
            shard,
            ignore,
            command_channel: command_channel.into(),
            command_guild: command_guild.into(),
            ready_at: None,
            last_msg: None,
            guild_channel_names: HashMap::new(),
        };

        this.dapi.set_token(token);
        this.redeem_dapi.set_token(redeem_token);

        Ok(this)
    }

    pub async fn start(&mut self) -> Result<()> {
        let mut recv = self
            .shard
            .get_event_stream()
            .ok_or("Cannot get gateway event stream")?;
        while let Some(e) = recv.next().await {
            match e {
                Ok(e) => {
                    if let Some(ev_type) = e.t {
                        let payload = e.d.unwrap_or(serde_json::Value::Null);
                        match ev_type.as_str() {
                            "READY" => self.handle_ready(&payload).await?,
                            "GUILD_CREATE" => self.handle_guild_create(&payload).await,
                            "GUILD_DELETE" => self.handle_guild_delete(&payload).await,
                            "MESSAGE_CREATE" => self.handle_message_create(payload).await,
                            "CHANNEL_CREATE" | "CHANNEL_UPDATE" | "GUILD_UPDATE" => {
                                self.handle_guild_or_channel_update(&payload)
                            }
                            _ => continue,
                        }
                    }
                }

                Err(why) => return Err(why.into()),
            }
        }

        Ok(())
    }

    fn handle_guild_or_channel_update(&mut self, payload: &serde_json::Value) {
        let id = payload["id"].as_str();
        let name = payload["name"].as_str();
        if let Some(id) = id {
            self.guild_channel_names
                .insert(id.to_owned(), name.unwrap_or("???").to_owned());
        }
    }

    async fn handle_message_create(&mut self, payload: serde_json::Value) {
        if matches!(payload["channel_id"].as_str(), Some(id) if id == self.command_channel) {
            self.handle_command(payload["content"].as_str().unwrap_or(""))
                .await;
            return;
        }

        self.last_msg = Some(payload);
        let payload = self.last_msg.as_ref().unwrap();

        let content = payload["content"].as_str().unwrap_or("");
        let Some(gift_code) = regex!(r"discord\.gift/([\d\w]{1,19})(?: |$)"im).captures(content)
            .and_then(|c| c.get(1).map(|c| c.as_str())) else {
                return;
            };

        {
            let mut code_lock = SHARED.used_codes.lock().unwrap();
            if code_lock.contains(gift_code) {
                return;
            }
            code_lock.insert(gift_code.to_owned());
        }

        let results = match gift_code.len() {
            16.. => vec![self.redeem_code(Cow::Borrowed(&gift_code[..16])).await],
            _ => {
                let sanitized = regex!("^[0-9A-Za-z ]").replace_all(content, "");
                let codes = sanitized
                    .split(' ')
                    .filter(|s| gift_code.len() + s.len() == 16)
                    .map(|s| format!("{}{}", gift_code, s))
                    .take(20);
                let mut v = Vec::with_capacity(4);
                for c in codes {
                    v.push(self.redeem_code(c.into()).await);
                }
                v
            }
        };

        let channel_name = payload["channel_id"]
            .as_str()
            .and_then(|id| self.guild_channel_names.get(id).map(|s| s.as_str()))
            .unwrap_or("??");
        let guild_name = payload["guild_id"]
            .as_str()
            .and_then(|id| self.guild_channel_names.get(id).map(|s| s.as_str()))
            .unwrap_or("??");
        let safe_content = regex!("(?:@everyone)|(?:@here)").replace_all(content, "");

        let mut report = GiftReport {
            from: payload["author"]["username"]
                .as_str()
                .unwrap_or("??")
                .into(),
            channel: channel_name.into(),
            guild: guild_name.into(),
            ping: self.shard.get_ping(),
            content: safe_content,
            attempts: vec![],
        };

        for res in results {
            match res {
                Err(e) => {
                    self.relay
                        .log_error(&self.username, &*e, Some("While redeeming code"))
                        .await
                }
                Ok(att) => report.attempts.push(att),
            }
        }

        self.relay.gift_report(&self.username, report).await;
    }

    async fn redeem_code<'a>(
        &self,
        code: Cow<'a, str>,
    ) -> std::result::Result<(Cow<'a, str>, GiftRedeemAttempt), Box<dyn Error + Send + Sync>> {
        if self.ignore {
            let res = self
                .redeem_dapi
                .get(&v6Routes::entitlements_giftcode(&code))
                .await;
            match res {
                Err(DApiError::ApiError(e)) => {
                    if e.code == 10038 {
                        return Ok((code, GiftRedeemAttempt::Invalid { info: e.to_string().into() }));
                    } else {
                        return Err(format!("Could not get gift info: {e}").into());
                    }
                }
                Err(e) => {
                    return Err(format!("Could not get gift info: {e}").into());
                }
                Ok(info) => {
                    let name = info["store_listing"]["sku"]["name"].as_str();
                    if name == Some("Nitro Classic") || name.is_none() {
                        return Ok((code, GiftRedeemAttempt::Ignored));
                    }
                }
            }
        }

        let res = self
            .redeem_dapi
            .post(
                &v6Routes::entitlements_giftcode_redeem(&code),
                &Default::default(),
            )
            .await;

        match res {
            Err(DApiError::ApiError(e)) => {
                if e.code == 50050 {
                    let res = self
                        .redeem_dapi
                        .get(&v6Routes::entitlements_giftcode(&code))
                        .await;
                    match res {
                        Ok(val) => Ok((
                            code,
                            GiftRedeemAttempt::Claimed {
                                info: e.to_string().into(),
                                gifter: val["user"]["username"]
                                    .as_str()
                                    .unwrap_or("???")
                                    .to_owned()
                                    .into(),
                            },
                        )),
                        Err(e) => Err(format!("Could not get gift info: {e}").into()),
                    }
                } else if e.code == 10038 {
                    return Ok((code, GiftRedeemAttempt::Invalid { info: e.to_string().into() }));
                } else {
                    return Err(e.to_string().into());
                }
            }
            Err(e) => Err(format!("DApi error while trying to claim gift: {e}").into()),
            Ok(gift) => Ok((
                code,
                GiftRedeemAttempt::Success { info: format!("{:#}", gift).into() },
            )),
        }
    }

    async fn handle_command(&mut self, msg: &str) {
        if msg.starts_with("...ping") {
            self.relay
                .command_ping(&self.username, self.shard.get_ping())
                .await;
        } else if msg.starts_with("...stats") {
            let guilds = SHARED.guilds.lock().unwrap().get(&self.id).unwrap().len();
            self.relay
                .command_stats(
                    &self.username,
                    self.shard.get_ping(),
                    self.last_msg
                        .as_ref()
                        .and_then(|v| v["content"].as_str())
                        .unwrap_or(""),
                    self.ignore,
                    guilds,
                    self.guild_channel_names.len() - guilds,
                )
                .await;
        } else if msg.starts_with("...ignore") {
            self.ignore = !self.ignore;
            self.relay.command_ignore(self.ignore).await;
        }
    }

    async fn handle_guild_delete(&mut self, payload: &serde_json::Value) {
        let id = payload["id"].as_str().unwrap();
        SHARED
            .guilds
            .lock()
            .unwrap()
            .entry(self.id)
            .and_modify(|set| {
                set.remove(id);
            });
        self.guild_channel_names.remove(id);
    }

    async fn handle_guild_create(&mut self, payload: &serde_json::Value) {
        self.handle_guild_or_channel_update(payload);
        let joined_id = payload["id"].as_str().unwrap();
        if joined_id == self.command_guild {
            return;
        }

        let mut should_leave = false;
        {
            let map = SHARED.guilds.lock().unwrap();
            let mut scanners = map.iter().filter(|(id, _)| **id != self.id);
            if scanners.any(|(_, g)| g.contains(joined_id)) {
                should_leave = true;
            }
        }
        if should_leave {
            let res = self
                .dapi
                .delete(&v10Routes::users_guilds_leave(joined_id), &())
                .await;
            let name = self
                .guild_channel_names
                .get(joined_id)
                .map(|s| s.as_str())
                .unwrap_or("???");
            self.relay.duplicate_guilds(&self.username, name).await;
            if let Err(e) = res {
                self.relay
                    .log_error(&self.username, &e, Some("Failed to leave guild"))
                    .await;
            }
        } else {
            SHARED
                .guilds
                .lock()
                .unwrap()
                .entry(self.id)
                .and_modify(|set| {
                    set.insert(joined_id.to_owned());
                });
        }
    }

    async fn set_status(&mut self) -> GCResult<()> {
        self.shard
            .send(GatewayEvent {
                d: Some(
                    serde_json::value::to_value(
                        GatewayPresenceBuilder::default()
                            .status(GatewayStatus::online)
                            .activities([GatewayActivityBuilder::default()
                                .r#type(GatewayActivityType::WATCHING)
                                //.emoji(GatewayActivityEmoji { name: "moyai".into(), id: None, animated: None }) //seems not working with custom
                                .name("y'all")
                                .build()
                                .unwrap()])
                            .build()
                            .unwrap(),
                    )
                    .unwrap(),
                ),
                ..GatewayEvent::new(GatewayOpcode::PRESENCE_UPDATE)
            })
            .await
    }

    async fn handle_ready(&mut self, payload: &serde_json::Value) -> Result<()> {
        let name = payload["user"]["username"].as_str().ok_or("No username")?;
        if self.ready_at.is_none() {
            let guilds = payload["guilds"]
                .as_array()
                .ok_or("Could not get guild array")?;
            let mut lock = SHARED.guilds.lock().unwrap();
            for g in guilds {
                let id = g["id"].as_str().ok_or("Guild lacks id")?.to_owned();
                lock.entry(self.id).and_modify(|set| {
                    set.insert(id.clone());
                });
                self.guild_channel_names
                    .insert(id, g["name"].as_str().ok_or("Guild lacks name")?.to_owned());
                for c in g["channels"].as_array().ok_or("Guild lacks channels")? {
                    self.guild_channel_names.insert(
                        c["id"].as_str().ok_or("Channel lacks id")?.to_owned(),
                        c["name"]
                            .as_str()
                            .map(|s| s.to_owned())
                            .unwrap_or_else(|| String::from("??")),
                    );
                }
            }
            let self_guilds = lock.get(&self.id).unwrap();
            for guilds in lock.iter().filter(|e| *e.0 != self.id).map(|e| e.1) {
                let dups: Vec<_> = self_guilds
                    .intersection(guilds)
                    .filter(|g| **g != self.command_guild)
                    .map(|g| self.guild_channel_names.get(g).unwrap_or(g))
                    .collect();
                if !dups.is_empty() {
                    return Err(format!("@{}: Found duplicate guilds: {:?}", name, dups).into());
                }
            }
            info!("Logged in as @{name}");
        } else {
            info!("Relogged as @{name}");
        }

        self.username = name.to_owned();
        self.ready_at = Some(Instant::now());
        self.set_status().await?;
        Ok(())
    }
}

impl Drop for GiftScanner {
    fn drop(&mut self) {
        SHARED.guilds.lock().unwrap().remove(&self.id);
    }
}
