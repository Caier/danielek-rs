use std::{error::Error as StdError, time::Duration};

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
    }};
}
use once_cell::sync::Lazy;
pub(crate) use try_x_times;

use super::error::{GCError, GCResult};

pub async fn fetch_wss_url() -> GCResult<String> {
    static HTTP: Lazy<reqwest::Client> = Lazy::new(|| {
        reqwest::ClientBuilder::new()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap()
    });

    async {
        Ok::<String, Box<dyn StdError + Send + Sync>>(format!(
            "{}/?v=10",
            HTTP.get("https://discord.com/api/v10/gateway")
                .send()
                .await?
                .error_for_status()?
                .json::<serde_json::Value>()
                .await?["url"]
                .as_str()
                .ok_or("Invalid json")?
                .to_owned()
        ))
    }
    .await
    .map_err(|e| GCError::GatewayURLFetch(e))
}
