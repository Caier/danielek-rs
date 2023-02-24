use std::collections::HashSet;
use std::time::Instant;

use futures_util::StreamExt;

use crate::gateway::shard::GatewayShard;
use crate::gateway::types::GatewayIntents;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub struct GiftScanner {
    shard: GatewayShard,
    redeem_token: String,
    ignore: bool,
    log_id: String,
    guilds: HashSet<String>,
    ready_at: Option<Instant>
}

impl GiftScanner {
    pub async fn new(token: impl Into<String>, redeem_token: impl Into<String>, ignore: bool, log_id: impl Into<String>) -> Result<Self> {
        let intents = 
            GatewayIntents::GUILDS |
            GatewayIntents::MESSAGE_CONTENT |
            GatewayIntents::GUILD_MESSAGES  |
            GatewayIntents::DIRECT_MESSAGES;
            
        let mut shard = GatewayShard::new(token, intents, true).await?;

        Ok(Self { shard, redeem_token: redeem_token.into(), ignore, log_id: log_id.into(), guilds: HashSet::new(), ready_at: None })
    }

    pub async fn start(&mut self) -> Result<()> {
        let mut recv = self.shard.get_event_stream().unwrap();
        while let Some(e) = recv.next().await {
            match e {
                Ok(e) => {
                    if let Some(ev_type) = e.t {
                        match ev_type.as_str() {
                            "READY" => self.handle_ready(&e.d.as_ref().unwrap()),
                            "GUILD_CREATE" => self.handle_guild_create(&e.d.as_ref().unwrap()),
                            _ => continue
                        }
                    } 
                }

                Err(why) => return Err(why.into()) 
            }
        }

        Ok(())
    }

    fn handle_guild_create(&mut self, payload: &serde_json::Value) {

    }

    fn handle_ready(&mut self, payload: &serde_json::Value) {
        if self.ready_at == None {
            let guilds = payload["guilds"].as_array().unwrap().iter().map(|go| go["id"].as_str().unwrap().to_owned());
            self.guilds.extend(guilds);
        }

        self.ready_at = Some(Instant::now());
    }
}