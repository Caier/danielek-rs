use serde::{Deserialize, Serialize};

pub type Snowflake<T = String> = T;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum IntOrStr {
    Integer(i64),
    String(String),
}

#[derive(Serialize, Deserialize, Clone, Debug, display_json::DisplayAsJsonPretty)]
pub struct DiscordApiError {
    pub code: u32,
    pub message: String,
    pub errors: Option<serde_json::Value>,
}
