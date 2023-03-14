use std::collections::{HashSet, HashMap};
use std::ops::Deref;
use std::sync::{RwLock, Arc};
use std::time::Instant;

use crossbeam::sync::ShardedLock;
use futures_util::StreamExt;
use once_cell::sync::Lazy;
use regex::Regex;
use uuid::Uuid;

use crate::gateway::shard::GatewayShard;
use crate::gateway::types::GatewayIntents;
use lazy_regex::regex;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

mod dapi;

#[derive(Default)]
struct SharedData {
    guilds: ShardedLock<HashMap<uuid::Uuid, HashSet<String>>>,
    used_codes: ShardedLock<HashSet<String>>
}

static SHARED: Lazy<SharedData> = Lazy::new(|| Default::default());

pub struct GiftScanner {
    id: uuid::Uuid,
    shard: GatewayShard,
    redeem_token: String,
    ignore: bool,
    log_id: String,
    ready_at: Option<Instant>,
    last_msg: Option<serde_json::Value>
}

impl GiftScanner {
    pub async fn new(token: impl Into<String>, redeem_token: impl Into<String>, ignore: bool, log_id: impl Into<String>) -> Result<Self> {
        let intents = 
            GatewayIntents::GUILDS |
            GatewayIntents::MESSAGE_CONTENT |
            GatewayIntents::GUILD_MESSAGES  |
            GatewayIntents::DIRECT_MESSAGES;
            
        let shard = GatewayShard::new(token, intents, true).await?;

        let id = Uuid::new_v4();
        SHARED.guilds.write().unwrap().insert(id, Default::default());

        Ok(Self { 
            id, 
            shard, 
            redeem_token: redeem_token.into(), 
            ignore, 
            log_id: log_id.into(), 
            ready_at: None,
            last_msg: None
        })
    }

    pub async fn start(&mut self) -> Result<()> {
        let mut recv = self.shard.get_event_stream().unwrap();
        while let Some(e) = recv.next().await {
            match e {
                Ok(e) => {
                    if let Some(ev_type) = e.t {
                        let payload = e.d.unwrap();
                        match ev_type.as_str() {
                            "READY" => self.handle_ready(payload),
                            "GUILD_CREATE" => self.handle_guild_create(payload),
                            "GUILD_DELETE" => self.handle_guild_delete(payload),
                            "MESSAGE_CREATE" => self.handle_message_create(payload),
                            _ => continue
                        }
                    } 
                }

                Err(why) => return Err(why.into()) 
            }
        }

        Ok(())
    }

    fn handle_message_create(&mut self, payload: serde_json::Value) {
        if matches!(payload["guild_id"].as_str(), Some(id) if id == self.log_id) {
            self.handle_command(payload);
            return;
        }

        self.last_msg = Some(payload);
        let payload = self.last_msg.as_ref().unwrap();

        let content = payload["content"].as_str().unwrap_or("");
        let Some(gift_code) = regex!(r"discord\.gift/([\d\w]{1,19})(?: |$)"im).captures(content)
            .and_then(|c| c.get(1).map(|c| c.as_str())) else {
                return;
            };
    
        if SHARED.used_codes.read().unwrap().contains(gift_code) {
            return;
        }
        SHARED.used_codes.write().unwrap().insert(gift_code.to_owned());

        match gift_code.len() {
            16 => self.redeem_code(gift_code),
            16.. => self.redeem_code(&gift_code[..16]),
            _ => {
                let sanitized = regex!("^[0-9A-Za-z ]").replace_all(content, "");
                let codes = sanitized.split(' ')
                    .filter(|s| gift_code.len() + s.len() == 16)
                    .map(|s| format!("{}{}", gift_code, s));
                for code in codes {
                    self.redeem_code(&code);
                }
            }
        }
        
        let safe_content = regex!("(?:@everyone)|(?:@here)").replace_all(content, "");
        //send(safe_content);
        //this.logChannel.send(`od: **@${msg.author.tag}**\nw **#${(msg.channel as TextChannel)?.name || 'DM'}**\nna **${msg.guild?.name || 'DM'}**\nping **${this.ws.ping} ms**`);
    }

    fn redeem_code(&self, code: &str) {

    }
    
    fn handle_command(&self, msg: serde_json::Value) {

    }

    fn handle_guild_delete(&mut self, payload: serde_json::Value) {
        let id = payload["id"].as_str().unwrap();
        SHARED.guilds.write().unwrap().entry(self.id).and_modify(|set| { set.remove(id); });
    }

    fn handle_guild_create(&mut self, payload: serde_json::Value) {
        let joined_id = payload["id"].as_str().unwrap();
        {
            let map = SHARED.guilds.read().unwrap();
            let scanners = map.iter().filter(|(id, _)| **id != self.id);
            for (_, guilds) in scanners {
                if guilds.contains(joined_id) {
                    //send a warning message
                    //leave the guild
                    return;
                }
            }
        }
        SHARED.guilds.write().unwrap().entry(self.id).and_modify(|set| { set.insert(joined_id.to_owned()); });
    }

    fn handle_ready(&mut self, payload: serde_json::Value) {
        if self.ready_at == None {
            let guilds = payload["guilds"].as_array().unwrap().iter().map(|go| go["id"].as_str().unwrap().to_owned());
            SHARED.guilds.write().unwrap().entry(self.id).and_modify(|set| set.extend(guilds));
        }

        //println!("{}: {}", );
        self.ready_at = Some(Instant::now());
    }
}

impl Drop for GiftScanner {
    fn drop(&mut self) {
        SHARED.guilds.write().unwrap().remove(&self.id);
    }
}