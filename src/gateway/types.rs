#![allow(unused, non_camel_case_types)]

use derive_builder::Builder;
use serde::{Deserialize, Serialize};
use smartstring::alias::String;

use crate::dapi::routes::common_types::Snowflake;

bitflags::bitflags! {
    #[derive(Serialize, Deserialize)]
    #[serde(transparent)]
    pub struct GatewayIntents: u64 {
        const NONE =                               0;
        const GUILDS =                        1 << 0;
        const GUILD_MEMBERS =                 1 << 1;
        const GUILD_MODERATION =              1 << 2;
        const GUILD_EMOJIS_AND_STICKERS =     1 << 3;
        const GUILD_INTEGRATIONS =            1 << 4;
        const GUILD_WEBHOOKS =                1 << 5;
        const GUILD_INVITES =                 1 << 6;
        const GUILD_VOICE_STATES =            1 << 7;
        const GUILD_PRESENCES =               1 << 8;
        const GUILD_MESSAGES =                1 << 9;
        const GUILD_MESSAGE_REACTIONS =       1 << 10;
        const GUILD_MESSAGE_TYPING =          1 << 11;
        const DIRECT_MESSAGES =               1 << 12;
        const DIRECT_MESSAGE_REACTIONS =      1 << 13;
        const DIRECT_MESSAGE_TYPING =         1 << 14;
        const MESSAGE_CONTENT =               1 << 15;
        const GUILD_SCHEDULED_EVENTS =        1 << 16;
        const AUTO_MODERATION_CONFIGURATION = 1 << 20;
        const AUTO_MODERATION_EXECUTION =     1 << 21;
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct GatewayOpcode(i32);

impl GatewayOpcode {
    pub const DISPATCH: Self = Self(0);
    pub const HEARTBEAT: Self = Self(1);
    pub const IDENTIFY: Self = Self(2);
    pub const PRESENCE_UPDATE: Self = Self(3);
    pub const VOICE_STATE_UPDATE: Self = Self(4);
    pub const RESUME: Self = Self(6);
    pub const RECONNECT: Self = Self(7);
    pub const REQUEST_GUILD_MEMBERS: Self = Self(8);
    pub const INVALID_SESSION: Self = Self(9);
    pub const HELLO: Self = Self(10);
    pub const HEARTBEAT_ACK: Self = Self(11);
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GatewayEvent {
    pub op: GatewayOpcode,
    pub d: Option<serde_json::Value>,
    pub s: Option<i32>,
    pub t: Option<String>,
}

impl GatewayEvent {
    pub fn new(opcode: GatewayOpcode) -> Self {
        Self {
            op: opcode,
            d: None,
            s: None,
            t: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ResumeInfo {
    pub session_id: String,
    pub gateway_url: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct GatewayActivityType(u8);

impl GatewayActivityType {
    pub const PLAYING: Self = Self(0);
    pub const STREAMING: Self = Self(1);
    pub const LISTENING: Self = Self(2);
    pub const WATCHING: Self = Self(3);
    pub const CUSTOM: Self = Self(4);
    pub const COMPETING: Self = Self(5);
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default)]
pub struct GatewayActivityTimestamps {
    pub start: Option<u64>,
    pub end: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GatewayActivityEmoji {
    pub name: String,
    pub id: Option<Snowflake>,
    pub animated: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct GatewayActivityParty {
    pub id: Option<String>,
    pub size: Option<[i64; 2]>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct GatewayActivityAssets {
    pub large_image: Option<String>,
    pub large_text: Option<String>,
    pub small_image: Option<String>,
    pub small_text: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct GatewayActivitySecrets {
    pub join: Option<String>,
    pub spectate: Option<String>,
    pub r#match: Option<String>,
}

bitflags::bitflags! {
    #[derive(Serialize, Deserialize)]
    #[serde(transparent)]
    pub struct GatewayActivityFlags: u32 {
        const INSTANCE =                    1 << 0;
        const JOIN =                        1 << 1;
        const SPECTATE =                    1 << 2;
        const JOIN_REQUEST =                1 << 3;
        const SYNC =                        1 << 4;
        const PLAY =                        1 << 5;
        const PARTY_PRIVACY_FRIENDS =       1 << 6;
        const PARTY_PRIVACY_VOICE_CHANNEL = 1 << 7;
        const EMBEDDED =                    1 << 8;
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum GatewayActivityButton {
    WithUrl { label: String, url: String },
    WithoutUrl(String),
}

#[derive(Serialize, Deserialize, Debug, Clone, Builder)]
#[builder(setter(strip_option, into))]
pub struct GatewayActivity {
    pub name: String,
    pub r#type: GatewayActivityType,
    #[builder(default)]
    pub url: Option<String>,
    #[builder(
        default = "std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64"
    )]
    pub created_at: u64,
    #[builder(default)]
    pub timestamps: Option<GatewayActivityTimestamps>,
    #[builder(default)]
    pub application_id: Option<Snowflake>,
    #[builder(default)]
    pub details: Option<String>,
    #[builder(default)]
    pub state: Option<String>,
    #[builder(default)]
    pub emoji: Option<GatewayActivityEmoji>,
    #[builder(default)]
    pub party: Option<GatewayActivityParty>,
    #[builder(default)]
    pub assets: Option<GatewayActivityAssets>,
    #[builder(default)]
    pub secrets: Option<GatewayActivitySecrets>,
    #[builder(default)]
    pub instance: Option<bool>,
    #[builder(default)]
    pub flags: Option<GatewayActivityFlags>,
    #[builder(default)]
    pub buttons: Option<Vec<GatewayActivityButton>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum GatewayStatus {
    online,
    dnd,
    idle,
    invisible,
    offline,
}

#[derive(Serialize, Deserialize, Debug, Clone, Builder)]
#[builder(setter(strip_option, into))]
pub struct GatewayPresenceSend {
    #[builder(default)]
    pub since: Option<u64>,
    #[builder(default)]
    pub activities: Vec<GatewayActivity>,
    pub status: GatewayStatus,
    #[builder(default)]
    pub afk: bool,
}
