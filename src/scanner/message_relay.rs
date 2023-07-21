use std::{borrow::Cow, error::Error};

use log::error;
use tokio::sync::Mutex;

use crate::dapi::{
    routes::v10::{
        types::{
            EmbedAuthorBuilder, EmbedBuilder, EmbedField, Message, MessagePayload,
            MessagePayloadBuilder,
        },
        webhook_execute,
    },
    versions::v10,
    DApi, DApiError, DApiPOST,
};

pub enum GiftRedeemAttempt<'a> {
    Success { info: Cow<'a, str> },
    Claimed { info: Cow<'a, str>, gifter: Cow<'a, str> },
    Invalid { info: Cow<'a, str> },
    Ignored,
}

#[derive(Default)]
pub struct GiftReport<'a> {
    pub from: Cow<'a, str>,
    pub channel: Cow<'a, str>,
    pub guild: Cow<'a, str>,
    pub content: Cow<'a, str>,
    pub ping: u64,
    pub attempts: Vec<(Cow<'a, str>, GiftRedeemAttempt<'a>)>,
}

pub struct MessageRelay {
    dapi: DApi<v10>,
    fcfs: Mutex<()>, //message exclusion lock
    route: Box<
        dyn DApiPOST<v10, Body = MessagePayload, Response = Option<Message>>
            + Send
            + Sync
            + 'static,
    >,
}

impl MessageRelay {
    const GIFT_OK_COLOR: i32 = 0x20e916;
    const GIFT_FAIL_COLOR: i32 = 0xffff47;
    const INFO_COLOR: i32 = 0x00b3fa;
    const ERROR_COLOR: i32 = 0xff1a1a;

    pub fn new(
        webhook_id: impl Into<String>,
        webhook_token: impl Into<String>,
    ) -> Result<Self, DApiError> {
        Ok(Self {
            dapi: DApi::new()?,
            fcfs: Mutex::new(()),
            route: Box::new(webhook_execute(
                webhook_id.into(),
                webhook_token.into(),
                true,
                None::<&str>,
            )),
        })
    }

    async fn send(&self, msg: &MessagePayload) {
        let res = self.dapi.post(&*self.route, msg).await;
        if let Err(e) = res {
            error!("Sending message failed with: {e}");
        }
    }

    pub async fn gift_report(&self, scanner: &str, report: GiftReport<'_>) {
        let has_success = report
            .attempts
            .iter()
            .any(|att| matches!(att.1, GiftRedeemAttempt::Success {..}));
        let mut embeds = vec![EmbedBuilder::default()
            .author(EmbedAuthorBuilder::default().name(scanner).build().unwrap())
            .title("Gift Report")
            .description(if report.content.len() > 2040 {
                report.content.chars().take(2040).collect()
            } else {
                report.content
            })
            .color(if has_success {
                Self::GIFT_OK_COLOR
            } else {
                Self::GIFT_FAIL_COLOR
            })
            .fields([
                EmbedField {
                    name: "From".into(),
                    value: format!("@{}", &report.from),
                    inline: Some(true),
                },
                EmbedField {
                    name: "Channel".into(),
                    value: format!("#{}", &report.channel),
                    inline: Some(true),
                },
                EmbedField {
                    name: "Guild".into(),
                    value: report.guild.into_owned(),
                    inline: Some(true),
                },
                EmbedField {
                    name: "Ping".into(),
                    value: format!("{} ms", report.ping),
                    inline: Some(true),
                },
            ])
            .build()
            .unwrap()];
        embeds.extend(report.attempts.into_iter().map(|mut att| {
            let mut emb = EmbedBuilder::default();
            if let GiftRedeemAttempt::Claimed { gifter: g, .. } = &mut att.1 {
                emb.fields([EmbedField {
                    name: "Gifter".into(),
                    value: std::mem::take(g).into_owned(),
                    inline: Some(true),
                }]);
            }
            emb.author(
                EmbedAuthorBuilder::default()
                    .name("Redeem Attempt")
                    .build()
                    .unwrap(),
            )
            .title(att.0)
            .color(if matches!(att.1, GiftRedeemAttempt::Success {..}) {
                Self::GIFT_OK_COLOR
            } else {
                Self::GIFT_FAIL_COLOR
            })
            .description(match att.1 {
                GiftRedeemAttempt::Success { info }
                | GiftRedeemAttempt::Claimed { info, .. }
                | GiftRedeemAttempt::Invalid { info } => Cow::Owned(format!("```json\n{info}\n```")),
                GiftRedeemAttempt::Ignored => {
                    Cow::Borrowed("Nitro Classic or invalid code. Ignored")
                }
            })
            .build()
            .unwrap()
        }));

        let msg = MessagePayloadBuilder::default()
            .embeds(embeds)
            .build()
            .unwrap();

        self.send(&msg).await;

        if has_success {
            self.send(
                &MessagePayloadBuilder::default()
                    .content("@everyone")
                    .build()
                    .unwrap(),
            )
            .await;
        }
    }

    pub async fn duplicate_guilds(&self, scanner: &str, guild_name: &str) {
        self.send(
            &MessagePayloadBuilder::default()
                .embeds([EmbedBuilder::default()
                    .color(Self::ERROR_COLOR)
                    .author(EmbedAuthorBuilder::default().name(scanner).build().unwrap())
                    .description(format!(
                        "❌ There are other spies on **{}**.\nI'm leaving that guild.",
                        guild_name
                    ))
                    .build()
                    .unwrap()])
                .build()
                .unwrap(),
        )
        .await;
    }

    pub async fn command_ping(&self, scanner: &str, ping: u64) {
        self.send(
            &MessagePayloadBuilder::default()
                .embeds([EmbedBuilder::default()
                    .color(Self::INFO_COLOR)
                    .author(EmbedAuthorBuilder::default().name(scanner).build().unwrap())
                    .description(format!("**{} ms** 🏓", ping))
                    .build()
                    .unwrap()])
                .build()
                .unwrap(),
        )
        .await;
    }

    pub async fn command_stats(
        &self,
        scanner: &str,
        ping: u64,
        last_msg: &str,
        ignore: bool,
        guilds: usize,
        channels: usize,
    ) {
        self.send(
            &MessagePayloadBuilder::default()
                .embeds([EmbedBuilder::default()
                    .color(Self::INFO_COLOR)
                    .author(EmbedAuthorBuilder::default().name(scanner).build().unwrap())
                    .fields([
                        EmbedField {
                            name: "Guilds".into(),
                            value: guilds.to_string(),
                            inline: Some(true),
                        },
                        EmbedField {
                            name: "Channels".into(),
                            value: channels.to_string(),
                            inline: Some(true),
                        },
                        EmbedField {
                            name: "Ping".into(),
                            value: ping.to_string(),
                            inline: Some(true),
                        },
                        EmbedField {
                            name: "Ignore".into(),
                            value: ignore.to_string(),
                            inline: Some(true),
                        },
                        EmbedField {
                            name: "Last message".into(),
                            value: if last_msg.len() > 250 {
                                last_msg.chars().take(250).collect()
                            } else {
                                last_msg.to_owned()
                            },
                            inline: Some(true),
                        },
                    ])
                    .build()
                    .unwrap()])
                .build()
                .unwrap(),
        )
        .await;
    }

    pub async fn command_ignore(&self, current: bool) {
        let lock = self.fcfs.try_lock();
        if lock.is_err() {
            return;
        }

        self.send(
            &MessagePayloadBuilder::default()
                .embeds([EmbedBuilder::default()
                    .color(Self::INFO_COLOR)
                    .description(if current {
                        "Nitro Classic is now being **ignored**"
                    } else {
                        "Nitro Classic is now being **collected**"
                    })
                    .build()
                    .unwrap()])
                .build()
                .unwrap(),
        )
        .await;
    }

    pub async fn log_error(
        &self,
        scanner: &str,
        error: &(dyn Error + Send + Sync + 'static),
        context: Option<&str>,
    ) {
        let context = context.unwrap_or("Unexpected Error");
        error!("{scanner}: {context}: {error}");

        self.send(
            &MessagePayloadBuilder::default()
                .embeds([EmbedBuilder::default()
                    .color(Self::ERROR_COLOR)
                    .author(EmbedAuthorBuilder::default().name(scanner).build().unwrap())
                    .title(context)
                    .description(format!("```rs\n{}\n```", error))
                    .build()
                    .unwrap()])
                .build()
                .unwrap(),
        )
        .await;
    }
}
