use std::env::var;

use futures_util::StreamExt;
use gateway::{shard::GatewayShard, types::GatewayIntents};

use crate::gateway::types::GatewayEvent;


mod gateway;
mod scanner;

#[tokio::main]
async fn main() {
    env_logger::init();
    dotenv::dotenv().ok();

    // {
    //     let intents = 
    //         GatewayIntents::GUILDS |
    //         GatewayIntents::MESSAGE_CONTENT |
    //         GatewayIntents::GUILD_MESSAGES  |
    //         GatewayIntents::DIRECT_MESSAGES;

    //     let mut shard = GatewayShard::new(var("TOKENS").unwrap(), intents, true).await.unwrap();
        
    //     while let Some(Ok(e)) = shard.get_event_stream_mut().unwrap().next().await {
    //         println!("OP: {}, t: {:?}", e.op, e.t);
    //         if let Some("READY") = e.t.as_ref().map(|s| s.as_str()) {
    //             println!("{:#?}", e.d.unwrap()["guilds"][0]);
    //         }
    //     }
    // }
}
