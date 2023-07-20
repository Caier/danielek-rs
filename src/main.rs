use std::sync::Arc;

use futures_util::future::select_all;

use scanner::{message_relay::MessageRelay, GiftScanner};

mod dapi;
mod gateway;
mod scanner;

#[tokio::main]
async fn main() {
    env_logger::init();
    dotenv::dotenv().ok();

    let vars = ["TOKENS", "REDTOKEN", "WEBHOOK", "COMMAND_GUILD_CHANNEL"]
        .map(|v| std::env::var(v).map_err(|_| v));

    if vars.iter().any(|r| r.is_err()) {
        panic!(
            "Could not load following env variables: {:?}",
            vars.iter()
                .filter(|r| r.is_err())
                .map(|r| r.as_ref().unwrap_err())
                .collect::<Vec<_>>()
        );
    }

    let vars = vars.map(|r| r.unwrap());
    let (webhook_id, webhook_token) = vars[2]
        .split_once('/')
        .expect("Invalid WEBHOOK format (should be \"id/token\"");
    let (cmd_guild, cmd_channel) = vars[3]
        .split_once('/')
        .expect("Invalid COMMAND_GUILD_CHANNEL format (should be \"id/id\")");
    let relay = Arc::new(MessageRelay::new(webhook_id, webhook_token).unwrap());

    let mut tasks = vec![];
    for token in vars[0].split(',') {
        let mut scanner = GiftScanner::new(
            token,
            &vars[1],
            false,
            cmd_channel,
            cmd_guild,
            Arc::clone(&relay),
        )
        .await
        .unwrap();
        tasks.push(tokio::spawn(async move {
            scanner.start().await //should never return
        }));
    }

    match select_all(tasks).await {
        (Ok(Err(e)), _, _) => {
            relay.log_error("main", &*e, Some("Fatal shard error")).await;
            panic!("A scanner failed with: {e}");
        }
        _ => unreachable!()
    }
}