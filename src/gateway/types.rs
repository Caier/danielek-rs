use serde::{Serialize, Deserialize};

bitflags::bitflags! {
    pub struct GatewayIntents: u64 {
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

#[derive(Serialize, Deserialize, Debug)]
pub enum GatewayOpcode {
    Dispatch = 0,
    Heartbeat = 1,
    Identify = 2,
    PresenceUpdate = 3,
    VoiceStateUpdate = 4,
    Resume = 6,
    Reconnect = 7,
    RequestGuildMembers = 8,
    InvalidSession = 9,
    Hello = 10,
    HeartbeatACK = 11,
}

#[allow(unused)]
#[derive(Serialize, Deserialize, Debug)]
pub struct GatewayEvent {
    pub op: i32,
    pub d: Option<serde_json::Value>,
    pub s: Option<i32>,
    pub t: Option<String>,
}

impl GatewayEvent {
    pub fn new(opcode: GatewayOpcode) -> Self {
        Self {
            op: opcode as i32,
            d: None,
            s: None,
            t: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ResumeInfo {
    pub session_id: String,
    pub gateway_url: String
}

