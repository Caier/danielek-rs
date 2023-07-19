#![allow(unused)]

use crate::dapi::{
    routes::v10::types::{Message, MessagePayload},
    types::{dapi_endpoint, DApiDELETE, DApiGET, DApiPOST, DApiVersion},
    versions::v10,
};

use super::common_types::Snowflake;

pub mod types;

dapi_endpoint! {
    version = v10,
    DApiPOST = (Message, MessagePayload);

    pub fn channel_messages(channel_id: Snowflake<impl AsRef<str>>) {
        format!("/channels/{}/messages", channel_id.as_ref())
    }
}

pub enum GetChannelMessagesAnchorParam<T: AsRef<str>> {
    Around(Snowflake<T>),
    Before(Snowflake<T>),
    After(Snowflake<T>),
}

dapi_endpoint! {
    version = v10,
    DApiGET = (Vec<Message>);

    pub fn channel_messages_get(channel_id: Snowflake<impl AsRef<str>>, limit: u32, anchor: Option<GetChannelMessagesAnchorParam<impl AsRef<str>>>) {
        let p = format!("/channels/{}/messages?limit={}", channel_id.as_ref(), limit);
        if let Some(a) = anchor {
            p + &match a {
                GetChannelMessagesAnchorParam::Around(s) => format!("&around={}", s.as_ref()),
                GetChannelMessagesAnchorParam::Before(s) => format!("&before={}", s.as_ref()),
                GetChannelMessagesAnchorParam::After(s) => format!("&after={}", s.as_ref())
            }
        } else { p }
    }
}

dapi_endpoint! {
    version = v10,
    DApiDELETE = ((), ());

    pub fn users_guilds_leave(guild_id: Snowflake<impl AsRef<str>>) {
        format!("/users/@me/guilds/{}", guild_id.as_ref())
    }
}

dapi_endpoint! {
    version = v10,
    DApiPOST = (Option<Message>, MessagePayload);

    pub fn webhook_execute(webhook_id: impl AsRef<str>, webhook_token: impl AsRef<str>, wait: bool, thread_id: Option<Snowflake<impl AsRef<str>>>) {
        let p = format!("/webhooks/{}/{}?wait={}", webhook_id.as_ref(), webhook_token.as_ref(), wait);
        if let Some(id) = thread_id {
            p + &format!("&thread_id={}", id.as_ref())
        } else { p }
    }
}
