use std::{time::Duration, env::var, error::Error as StdError};

macro_rules! try_x_times {
    ($times:expr, $what:expr) => {{
        let mut cnt = 1;
        loop {
            if let Err(why) = $what {
                cnt += 1;
                if cnt >= $times {
                    break Err(why);
                }
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            } else {
                break Ok(());
            }
        }
    }}
}
pub(crate) use try_x_times;

use super::error::{GCResult, GCError};

lazy_static::lazy_static! {
    static ref HTTP: reqwest::Client = reqwest::ClientBuilder::new()
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap();
}

pub async fn fetch_wss_url() -> GCResult<String> {
    async {
        Ok::<String, Box<dyn StdError + Send + Sync>>(
            format!("{}/?v=10" , HTTP
            .get(format!("{}/gateway", var("API_BASE")?))
            .send()
            .await?
            .error_for_status()?
            .json::<serde_json::Value>()
            .await?["url"]
            .as_str()
            .ok_or("Invalid json")?
            .to_owned()))
    }.await.map_err(|e| GCError::GatewayURLFetch(e))
}