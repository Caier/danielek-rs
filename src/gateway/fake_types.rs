#![allow(non_camel_case_types)]
//incomplete or untruthful gateway type definitions, useful only for danielek purposes

use derive_builder::Builder;
use serde::{Serialize, Deserialize};
use smartstring::alias::String;

use crate::dapi::routes::{v10::types::{Message, User, Channel, Guild, GuildMember}, common_types::Snowflake};

use super::types::{GatewayOpcode, GatewayIntents, GatewayPresenceSend};

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum GatewayDispatchEventName {
    READY,
    CHANNEL_CREATE,
    CHANNEL_UPDATE,
    CHANNEL_DELETE,
    GUILD_CREATE,
    GUILD_UPDATE,
    GUILD_DELETE,
    MESSAGE_CREATE,
    MESSAGE_UPDATE,
    #[serde(other)]
    Other
}

#[derive(Serialize, Debug, Clone)]
pub struct GatewayEvent {
    pub op: GatewayOpcode,
    pub d: Option<GatewayData>,
    pub s: Option<i64>,
    pub t: Option<GatewayDispatchEventName>,
}

impl GatewayEvent {
    pub fn new(op: GatewayOpcode) -> Self {
        Self {
            op,
            d: None,
            s: None,
            t: None
        }
    }
}

impl<'de> Deserialize<'de> for GatewayEvent { //to avoid #[serde(untagged)], untagged serialization remains fine though
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct GatewayEventProxy<'a> {
            op: GatewayOpcode,
            #[serde(borrow)]
            d: Option<&'a serde_json::value::RawValue>,
            s: Option<i64>,
            t: Option<GatewayDispatchEventName>
        }

        let ev = GatewayEventProxy::deserialize(deserializer)?;
        let d_as_str = ev.d
            .map(|v| v.get())
            .ok_or(serde::de::Error::custom("expected GatewayData not be null"));

        macro_rules! inner {
            () => { serde_json::from_str(d_as_str?).map_err(serde::de::Error::custom)? };
        }
        use {GatewayOpcode as OP, GatewayData as GD, GatewayDispatchEventName as GE};

        let d = match (ev.op, ev.t) {
            (OP::HEARTBEAT, _) if ev.d.is_some() =>     Some(GD::SendHeartbeat(inner!())),
            (OP::IDENTIFY, _) =>                        Some(GD::SendIdentify(inner!())),
            (OP::PRESENCE_UPDATE, _) =>                 Some(GD::SendUpdatePresence(inner!())),
            (OP::RESUME, _) =>                          Some(GD::SendResume(inner!())),
            (OP::INVALID_SESSION, _) =>                 Some(GD::InvalidSession(inner!())),
            (OP::HELLO, _) =>                           Some(GD::Hello(inner!())),
            (OP::DISPATCH, Some(GE::READY)) =>          Some(GD::Ready(inner!())),
            (OP::DISPATCH, Some(GE::CHANNEL_CREATE)) => Some(GD::ChannelCreate(inner!())),
            (OP::DISPATCH, Some(GE::CHANNEL_UPDATE)) => Some(GD::ChannelUpdate(inner!())),
            (OP::DISPATCH, Some(GE::CHANNEL_DELETE)) => Some(GD::ChannelDelete(inner!())),
            (OP::DISPATCH, Some(GE::GUILD_CREATE)) =>   Some(GD::GuildCreate(inner!())),
            (OP::DISPATCH, Some(GE::GUILD_UPDATE)) =>   Some(GD::GuildUpdate(inner!())),
            (OP::DISPATCH, Some(GE::GUILD_DELETE)) =>   Some(GD::GuildDelete(inner!())),
            (OP::DISPATCH, Some(GE::MESSAGE_CREATE)) => Some(GD::MessageCreate(inner!())),
            (OP::DISPATCH, Some(GE::MESSAGE_UPDATE)) => Some(GD::MessageUpdate(inner!())),
            _ => None
        };

        Ok(Self {
            op: ev.op,
            d,
            s: ev.s,
            t: ev.t
        })
    }
}

#[derive(Serialize, Debug, Clone)]
#[serde(untagged)]
pub enum GatewayData {
    //send events
    SendIdentify(Box<GatewayIdentifyPayload>),
    SendResume(Box<GatewayResumePayload>),
    SendHeartbeat(i64),
    SendUpdatePresence(Box<GatewayPresenceSend>),

    //connection-related events
    Hello(GatewayHelloPayload),
    InvalidSession(bool),

    //data events op = 0
    Ready(Box<GatewayReadyPayload>),
    ChannelCreate(Box<Channel>),
    ChannelUpdate(Box<Channel>),
    ChannelDelete(Box<Channel>),
    GuildCreate(Box<GatewayGuildCreatePayload>),
    GuildUpdate(Box<Guild>),
    GuildDelete(UnavailableGuild),
    MessageCreate(Box<MessageExtra>),
    MessageUpdate(Box<MessageExtra>)
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GatewayConnectionProperties {
    pub os: String,
    pub browser: String,
    pub device: String
}

#[derive(Serialize, Deserialize, Clone, Debug, Builder)]
#[builder(setter(into, strip_option))]
pub struct GatewayIdentifyPayload {
    pub token: String,
    pub properties: GatewayConnectionProperties,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compress: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub large_threshold: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shard: Option<[i32; 2]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence: Option<GatewayPresenceSend>,
    pub intents: GatewayIntents
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GatewayResumePayload {
    pub token: String,
    pub session_id: String,
    pub seq: i64
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct GatewayHelloPayload {
    pub heartbeat_interval: i64
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GatewayGuild {
    pub joined_at: Option<iso8601_timestamp::Timestamp>,
    pub large: bool,
    pub unavailable: Option<bool>,
    pub member_count: i64,
    //pub voice_states: partial voice states... idc,
    //pub members: Vec<GuildMember>,
    pub channels: Vec<Channel>,
    pub threads: Vec<Channel>,
    //pub presences: Vec<GatewayPresence>, this bitch can be partial
    //pub stage_instances: Vec<StageInstance> tf is a stage,
    //pub guild_scheduled_events: Vec<GuildScheduledEvent> me not caring
    #[serde(flatten)]
    pub guild_info: Guild
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UnavailableGuild {
    pub id: Snowflake,
    pub unavailable: Option<bool>
}

#[derive(Serialize, Clone, Debug)]
#[serde(untagged)]
pub enum GatewayGuildCreatePayload {
    Unavailable(UnavailableGuild),
    Available(GatewayGuild),
}

impl<'de> Deserialize<'de> for GatewayGuildCreatePayload { //untagged will not work by itself if I don't implement every single field on GatewayGuild
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw: &serde_json::value::RawValue = Deserialize::deserialize(deserializer)?;
        let res = serde_json::from_str::<GatewayGuild>(raw.get());
        match res {
            Err(_) => Ok(Self::Unavailable(serde_json::from_str(raw.get()).map_err(serde::de::Error::custom)?)),
            Ok(res) => Ok(Self::Available(res))
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GatewayReadyPayload {
    pub v: i32,
    pub user: User,
    pub guilds: Vec<GatewayGuildCreatePayload>,
    pub session_id: String,
    pub resume_gateway_url: String,
    pub shard: Option<[i32; 2]>,
    //pub application: literally who cares??? 
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MessageExtra {
    pub guild_id: Option<Snowflake>,
    pub member: Option<GuildMember>,
    //pub mentions array of user objects, with an additional partial member field
    #[serde(flatten)]
    pub rest: Message
}