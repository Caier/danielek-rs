use std::collections::{HashSet, HashMap};
use std::ops::Deref;
use std::sync::{RwLock, Arc};
use std::time::Instant;

use crossbeam::sync::ShardedLock;
use futures_util::StreamExt;
use once_cell::sync::Lazy;
use uuid::Uuid;

use crate::gateway::shard::GatewayShard;
use crate::gateway::types::GatewayIntents;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

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
    ready_at: Option<Instant>
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
        })
    }

    pub async fn start(&mut self) -> Result<()> {
        let mut recv = self.shard.get_event_stream().unwrap();
        while let Some(e) = recv.next().await {
            match e {
                Ok(e) => {
                    if let Some(ev_type) = e.t {
                        let payload = e.d.as_ref().unwrap();
                        match ev_type.as_str() {
                            "READY" => self.handle_ready(payload),
                            "GUILD_CREATE" => self.handle_guild_create(payload),
                            "GUILD_DELETE" => self.handle_guild_delete(payload),
                            _ => continue
                        }
                    } 
                }

                Err(why) => return Err(why.into()) 
            }
        }

        Ok(())
    }

    fn handle_guild_delete(&mut self, payload: &serde_json::Value) {
        let id = payload["id"].as_str().unwrap();
        SHARED.guilds.write().unwrap().get_mut(&self.id).unwrap().remove(id);
    }

    fn handle_guild_create(&mut self, payload: &serde_json::Value) {
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
        SHARED.guilds.write().unwrap().get_mut(&self.id).unwrap().insert(joined_id.to_owned());
    }

    fn handle_ready(&mut self, payload: &serde_json::Value) {
        if self.ready_at == None {
            let guilds = payload["guilds"].as_array().unwrap().iter().map(|go| go["id"].as_str().unwrap().to_owned());
            SHARED.guilds.write().unwrap().get_mut(&self.id).unwrap().extend(guilds);
        }

        self.ready_at = Some(Instant::now());
    }
}

impl Drop for GiftScanner {
    fn drop(&mut self) {
        SHARED.guilds.write().unwrap().remove(&self.id);
    }
}