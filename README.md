# danielek

Danielek is a low-abstraction implementation of the Discord [**gateway client**](https://discord.com/developers/docs/topics/gateway) and the **HTTP API client** for purposes where generic client implementations for bots are either not enough, too much or missing the point entirely, for example in self-bot specific scenarios, or when using undocumented API calls.

## How to use?
Fork and adapt. This is not a library. This is not meant to be a library. If you just want to create a generic discord bot you can for example use [serenity-rs](https://github.com/serenity-rs/serenity).

### Gateway client
- `src/gateway/*`

The gateway websocket client conforms to the [v10 API](https://discord.com/developers/docs/reference#api-versioning) specification. It handles creating and maintaining the connection to the gateway; receives, deserializes events and sends them through a stream for further processing; allows you to send events to the gateway. It is mostly complete, although you may want to add support for other non-documented payloads in `types.rs` or `fake_types.rs`.

#### Examples
<details>
<summary>Starting a connection, receiving the READY event and changing the user's status to "Do Not Disturb"</summary>

```rs
use futures_util::StreamExt;
use gateway::{
    fake_types::{GatewayData, GatewayEvent},
    shard::GatewayShard,
    types::{GatewayIntents, GatewayOpcode, GatewayPresenceSend, GatewayStatus},
};

#[tokio::main]
async fn main() {
    let mut shard = GatewayShard::new(
            "your discord bot/user token", 
            GatewayIntents::NONE, 
            true
        ).await
        .unwrap();

    let ready_payload = shard
        .get_event_stream()
        .unwrap()
        .filter_map(|e| async move {
            if let Ok(GatewayEvent {
                d: Some(GatewayData::Ready(payload)), ..
            }) = e { Some(payload) } else { None }
        })
        .boxed()
        .next()
        .await
        .unwrap();

    println!("Logged in as {}", ready_payload.user.username);

    shard.send(GatewayEvent {
        d: Some(GatewayData::SendUpdatePresence(
            Box::new(GatewayPresenceSend {
                since: None,
                activities: vec![],
                status: GatewayStatus::dnd,
                afk: false
            }))),
        ..GatewayEvent::new(GatewayOpcode::PRESENCE_UPDATE)
    })
    .await
    .unwrap();
}
```
</details>

### HTTP API client
- `src/dapi/*`

The HTTP API client is generic over different API versions, although you should probably always aim to use the latest version. The client is rate-limit aware and waits until the limit is resolved before it sends the request. The client uses a generic browser user-agent by default. This is something you probably want when using a user account, but you will get cloudflare blocked when using a bot account. If you're using a bot account make sure to set the user-agent according with the [guidelines](https://discord.com/developers/docs/reference#user-agent).

The API is extremely extensive and only a handful of endpoints are implemented as routes. You can look at `routes/v10/mod.rs` to see how different routes are implemented.

#### Examples
<details>
<summary>Sending a POST request to send a message to a channel</summary>

```rs
use dapi::{routes::v10::types::MessagePayloadBuilder, versions::v10, DApi};

#[tokio::main]
async fn main() {
    let mut dapi = DApi::<v10>::new().unwrap();
    dapi.set_token("your bot/user token");
    dapi.set_user_agent("example (v1)");

    let msg = MessagePayloadBuilder::default()
        .content("hello!!")
        .build()
        .unwrap();

    dapi.post(&dapi::routes::v10::channel_messages("channel ID"), &msg)
        .await
        .unwrap();
}
```
</details>

<details>
<summary>Implementing a route for an API endpoint</summary>

```rs
//dapi/routes/*/mod.rs

dapi_endpoint! {
    // a type implementing dapi::types::DApiVersion specifying the API version applicable, here v10
    version = v10, 

    // http method specification in form of:
    // DApiMETHOD = (Response Type, Request Body Type);
    // or specific to GET requests:
    // DApiGET = (Response Type);
    // multiple method specifications can be present in one endpoint, the last one has to finish its line with a semicolon
    DApiPOST = (Message, MessagePayload); 

    // a function that returns the endpoint as an impl AsRef<str>
    pub fn channel_messages(channel_id: Snowflake<impl AsRef<str>>) {
        format!("/channels/{}/messages", channel_id.as_ref())
    }
}
```
</details>

### More examples
- `src/scanner/*`
- `src/main.rs`

A user bot handling multiple accounts concurrently using the gateway and HTTP API clients. Made to scan all received messages in search of a specific URL.