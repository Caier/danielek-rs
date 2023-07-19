#![allow(non_camel_case_types, unused)]

use derive_builder::Builder;
use serde::{Deserialize, Serialize};

use crate::dapi::routes::common_types::{IntOrStr, Snowflake};

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub enum EmbedType {
    rich,
    image,
    video,
    gifv,
    article,
    link,
}

#[derive(Serialize, Deserialize, Clone, Builder, Debug)]
#[builder(setter(strip_option, into))]
pub struct EmbedFooter {
    pub text: String,
    #[builder(default)]
    pub icon_url: Option<String>,
    #[builder(default)]
    pub proxy_icon_url: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Builder, Debug)]
#[builder(setter(strip_option, into))]
pub struct EmbedImage {
    pub url: String,
    #[builder(default)]
    pub proxy_url: Option<String>,
    #[builder(default)]
    pub height: Option<u32>,
    #[builder(default)]
    pub width: Option<u32>,
}

#[derive(Serialize, Deserialize, Clone, Builder, Debug)]
#[builder(setter(strip_option, into))]
pub struct EmbedThumbnail {
    pub url: String,
    #[builder(default)]
    pub proxy_url: Option<String>,
    #[builder(default)]
    pub height: Option<u32>,
    #[builder(default)]
    pub width: Option<u32>,
}

#[derive(Serialize, Deserialize, Clone, Builder, Debug, Default)]
#[builder(setter(strip_option, into), default)]
pub struct EmbedVideo {
    pub url: Option<String>,
    pub proxy_url: Option<String>,
    pub height: Option<u32>,
    pub width: Option<u32>,
}

#[derive(Serialize, Deserialize, Clone, Builder, Debug, Default)]
#[builder(setter(strip_option, into), default)]
pub struct EmbedProvider {
    pub name: Option<String>,
    pub url: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Builder, Debug)]
#[builder(setter(strip_option, into))]
pub struct EmbedAuthor {
    pub name: String,
    #[builder(default)]
    pub url: Option<String>,
    #[builder(default)]
    pub icon_url: Option<String>,
    #[builder(default)]
    pub proxy_icon_url: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Builder, Debug)]
#[builder(setter(strip_option, into))]
pub struct EmbedField {
    pub name: String,
    pub value: String,
    #[builder(default)]
    pub inline: Option<bool>,
}

#[derive(Serialize, Deserialize, Clone, Builder, Debug, Default)]
#[builder(setter(strip_option, into), default)]
pub struct Embed {
    pub title: Option<String>,
    pub r#type: Option<String>,
    pub description: Option<String>,
    pub url: Option<String>,
    pub timestamp: Option<iso8601_timestamp::Timestamp>,
    pub color: Option<i32>,
    pub footer: Option<EmbedFooter>,
    pub image: Option<EmbedImage>,
    pub thumbnail: Option<EmbedThumbnail>,
    pub video: Option<EmbedVideo>,
    pub provider: Option<EmbedProvider>,
    pub author: Option<EmbedAuthor>,
    pub fields: Option<Vec<EmbedField>>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub enum AllowedMentionTypes {
    roles,
    users,
    everyone,
}

#[derive(Serialize, Deserialize, Clone, Builder, Debug, Default)]
#[builder(default)]
pub struct AllowedMentions {
    pub parse: Vec<AllowedMentionTypes>,
    pub roles: Vec<Snowflake>,
    pub users: Vec<Snowflake>,
    pub replied_user: bool,
}

#[derive(Serialize, Deserialize, Clone, Builder, Debug, Default)]
#[builder(setter(strip_option, into), default)]
pub struct MessageReference {
    pub message_id: Option<Snowflake>,
    pub channel_id: Option<Snowflake>,
    pub guild_id: Option<Snowflake>,
    pub fail_if_not_exists: Option<bool>,
}

#[derive(Serialize, Deserialize, Clone, Builder, Debug)]
#[builder(setter(strip_option, into))]
pub struct Attachment {
    pub id: Snowflake,
    pub filename: String,
    #[builder(default)]
    pub description: Option<String>,
    #[builder(default)]
    pub content_type: Option<String>,
    pub size: u64,
    pub url: String,
    pub proxy_url: String,
    #[builder(default)]
    pub height: Option<u32>,
    #[builder(default)]
    pub width: Option<u32>,
    #[builder(default)]
    pub ephemeral: Option<bool>,
}

bitflags::bitflags! {
    #[derive(Serialize, Deserialize)]
    #[serde(transparent)]
    pub struct MessageFlags: u32 {
        const CROSSPOSTED = 1 << 0;
        const IS_CROSSPOST = 1 << 1;
        const SUPPRESS_EMBEDS = 1 << 2;
        const SOURCE_MESSAGE_DELETED = 1 << 3;
        const URGENT = 1 << 4;
        const HAS_THREAD = 1 << 5;
        const EPHEMERAL = 1 << 6;
        const LOADING = 1 << 7;
        const FAILED_TO_MENTION_SOME_ROLES_IN_THREAD = 1 << 8;
        const SUPPRESS_NOTIFICATIONS = 1 << 12;
    }
}

#[derive(Serialize, Deserialize, Clone, Builder, Debug, Default)]
#[builder(default, setter(strip_option, into))]
pub struct MessagePayload {
    pub content: Option<String>,
    pub nonce: Option<IntOrStr>,
    pub tts: Option<bool>,
    pub embeds: Option<Vec<Embed>>,
    pub allowed_mentions: Option<AllowedMentions>,
    pub message_reference: Option<MessageReference>,
    //pub components: Option<Vec<MessageComponent>> ... not insane enough to type that yet
    pub sticker_ids: Option<Vec<Snowflake>>,
    pub attachments: Option<Vec<Attachment>>,
    pub flags: Option<MessageFlags>,
}

bitflags::bitflags! {
    #[derive(Serialize, Deserialize)]
    #[serde(transparent)]
    pub struct UserFlags: u32 {
        const STAFF = 1 << 0;
        const PARTNER = 1 << 1;
        const HYPESQUAD = 1 << 2;
        const BUG_HUNTER_LEVEL_1 = 1 << 3;
        const HYPESQUAD_ONLINE_HOUSE_1 = 1 << 6;
        const HYPESQUAD_ONLINE_HOUSE_2 = 1 << 7;
        const HYPESQUAD_ONLINE_HOUSE_3 = 1 << 8;
        const PREMIUM_EARLY_SUPPORTER = 1 << 9;
        const TEAM_PSEUDO_USER = 1 << 10;
        const BUG_HUNTER_LEVEL_2 = 1 << 14;
        const VERIFIED_BOT = 1 << 16;
        const VERIFIED_DEVELOPER = 1 << 17;
        const CERTIFIED_MODERATOR = 1 << 18;
        const BOT_HTTP_INTERACTIONS = 1 << 19;
        const ACTIVE_DEVELOPER = 1 << 22;
    }
}

#[derive(Serialize, Deserialize, Clone, Builder, Debug)]
#[builder(setter(strip_option, into))]
pub struct User {
    pub id: Snowflake,
    pub username: String,
    pub discriminator: String,
    #[builder(default)]
    pub avatar: Option<String>,
    #[builder(default)]
    pub bot: Option<bool>,
    #[builder(default)]
    pub system: Option<bool>,
    #[builder(default)]
    pub mfa_enabled: Option<bool>,
    #[builder(default)]
    pub banner: Option<String>,
    #[builder(default)]
    pub accent_color: Option<u32>,
    #[builder(default)]
    pub locale: Option<String>,
    #[builder(default)]
    pub verified: Option<bool>,
    #[builder(default)]
    pub email: Option<String>,
    #[builder(default)]
    pub flags: Option<UserFlags>,
    #[builder(default)]
    pub premium_type: Option<u32>, //not really
    #[builder(default)]
    pub public_flags: Option<UserFlags>,
}

#[derive(Serialize, Deserialize, Clone, Builder, Debug)]
#[builder(setter(into))]
pub struct ChannelMention {
    pub id: Snowflake,
    pub guild_id: Snowflake,
    pub r#type: u32, //not really
    pub name: String,
}

#[derive(Serialize, Deserialize, Clone, Builder, Debug, Default)]
#[builder(default, setter(strip_option, into))]
pub struct Emoji {
    pub id: Option<Snowflake>,
    pub name: Option<String>,
    pub roles: Option<Snowflake>,
    pub user: Option<User>,
    pub require_colons: Option<bool>,
    pub managed: Option<bool>,
    pub animated: Option<bool>,
    pub available: Option<bool>,
}

#[derive(Serialize, Deserialize, Clone, Builder, Debug)]
#[builder(setter(into))]
pub struct Reaction {
    pub count: u64,
    pub me: bool,
    pub emoji: Emoji,
}

#[derive(Serialize, Deserialize, Clone, Builder, Debug)]
#[builder(setter(into, strip_option))]
pub struct MessageActivity {
    pub r#type: u32, //not really
    #[builder(default)]
    pub party_id: Option<String>,
}

bitflags::bitflags! {
    #[derive(Serialize, Deserialize)]
    pub struct GuildMemberFlags: u32 {
        const DID_REJOIN = 1 << 0;
        const COMPLETED_ONBOARDING = 1 << 1;
        const BYPASSES_VERIFICATION = 1 << 2;
        const STARTED_ONBOARDING = 1 << 3;
    }
}

#[derive(Serialize, Deserialize, Clone, Builder, Debug)]
#[builder(setter(into, strip_option))]
pub struct GuildMember {
    #[builder(default)]
    pub user: Option<User>,
    #[builder(default)]
    pub nick: Option<String>,
    #[builder(default)]
    pub avatar: Option<String>,
    #[builder(default)]
    pub roles: Vec<Snowflake>,
    pub joined_at: iso8601_timestamp::Timestamp,
    #[builder(default)]
    pub premium_since: Option<iso8601_timestamp::Timestamp>,
    pub deaf: bool,
    pub mute: bool,
    pub flags: GuildMemberFlags,
    #[builder(default)]
    pub permissions: Option<String>,
    #[builder(default)]
    pub communication_disabled_until: Option<iso8601_timestamp::Timestamp>,
}

#[derive(Serialize, Deserialize, Clone, Builder, Debug)]
#[builder(setter(into, strip_option))]
pub struct MessageInteraction {
    pub id: Snowflake,
    pub r#type: u32, //not really
    pub name: String,
    pub user: User,
    #[builder(default)]
    pub member: Option<GuildMember>,
}

#[derive(Serialize, Deserialize, Clone, Builder, Debug)]
#[builder(setter(into, strip_option))]
pub struct StickerItem {
    pub id: Snowflake,
    pub name: String,
    pub format_type: u32, //not really
}

#[derive(Serialize, Deserialize, Clone, Builder, Debug)]
#[builder(setter(into, strip_option))]
pub struct RoleSubscriptionData {
    pub role_subscription_listing_id: Snowflake,
    pub tier_name: String,
    pub total_months_subscribed: u32,
    pub is_renewal: bool,
}

#[derive(Serialize, Deserialize, Clone, Builder, Debug)]
#[builder(setter(into, strip_option))]
pub struct Message {
    pub id: Snowflake,
    pub channel_id: Snowflake,
    #[builder(default)]
    pub author: Option<User>,
    pub content: String,
    pub timestamp: iso8601_timestamp::Timestamp,
    #[builder(default)]
    pub edited_timestamp: Option<iso8601_timestamp::Timestamp>,
    pub tts: bool,
    pub mention_everyone: bool,
    #[builder(default)]
    pub mentions: Vec<User>,
    #[builder(default)]
    pub mention_roles: Vec<Snowflake>,
    #[builder(default)]
    pub mention_channels: Option<Vec<ChannelMention>>,
    #[builder(default)]
    pub attachments: Vec<Attachment>,
    #[builder(default)]
    pub embeds: Vec<Embed>,
    #[builder(default)]
    pub reactions: Option<Vec<Reaction>>,
    #[builder(default)]
    pub nonce: Option<IntOrStr>,
    pub pinned: bool,
    #[builder(default)]
    pub webhook_id: Option<Snowflake>,
    #[builder(default)]
    pub r#type: Option<u32>, //not really
    #[builder(default)]
    pub activity: Option<MessageActivity>,
    //pub application: Option<Application>; nope idc
    #[builder(default)]
    pub application_id: Option<Snowflake>,
    #[builder(default)]
    pub message_reference: Option<MessageReference>,
    #[builder(default)]
    pub flags: Option<MessageFlags>,
    #[builder(default)]
    pub referenced_message: Option<Box<Message>>,
    #[builder(default)]
    pub interaction: Option<MessageInteraction>,
    //pub thread: Option<Channel>, nope
    //pub components: Option<Vec<MessageComponent>>, nope
    #[builder(default)]
    pub sticker_items: Option<Vec<StickerItem>>,
    //pub stickers: deprecated???
    #[builder(default)]
    pub position: Option<i64>,
    #[builder(default)]
    pub role_subscription_data: Option<RoleSubscriptionData>,
}
