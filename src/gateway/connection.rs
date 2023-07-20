use std::{
    sync::{atomic::AtomicU64, Arc},
    time::Duration,
};

use futures_util::{SinkExt, StreamExt};
use log::debug;
use serde_json::json;
use tokio::{
    net::TcpStream,
    select,
    sync::{mpsc, oneshot},
    time::Instant,
};
use tokio_tungstenite::{
    connect_async_with_config,
    tungstenite::{protocol::WebSocketConfig, Message},
    MaybeTlsStream, WebSocketStream,
};

use crate::gateway::types::GatewayOpcode;

use super::{
    error::{GCError, GCResult},
    types::{GatewayEvent, ResumeInfo},
    util::fetch_wss_url,
};

#[derive(Debug)]
pub enum GatewayThreadMessage {
    SendEvent(GatewayEvent, oneshot::Sender<GCResult<()>>),
}

pub struct GatewayConnection {
    pub comm_rx: mpsc::Receiver<GatewayThreadMessage>,
    pub evnt_tx: mpsc::UnboundedSender<GCResult<GatewayEvent>>,
    pub ws: WebSocketStream<MaybeTlsStream<TcpStream>>,
    pub next_heartbeat: Instant,
    pub last_heartbeat: Instant,
    pub last_ack: Instant,
    pub heartbeat_interval: Duration,
    pub last_sequence: i32,
    pub token: String,
    pub intents: u64,
    pub resume_info: Option<ResumeInfo>,
    pub websocket_config: WebSocketConfig,
    pub force_reconnect: bool,
    pub ping: Arc<AtomicU64>,
}

impl GatewayConnection {
    const RECONNECTABLE_CLOSES: [u16; 8] = [4000, 4001, 4002, 4003, 4005, 4007, 4008, 4009];
    const UNRECONNECTABLE_CLOSES: [u16; 6] = [4004, 4010, 4011, 4012, 4013, 4014];

    async fn _connect(&mut self, url: impl Into<String>) -> GCResult<()> {
        (self.ws, _) = connect_async_with_config(url.into(), Some(self.websocket_config))
            .await
            .map_err(GCError::ConnectError)?;
        self.next_heartbeat = Instant::now() + Duration::from_secs(3600);
        self.last_heartbeat = Instant::now();
        self.last_ack = Instant::now();
        self.heartbeat_interval = Duration::from_secs(3600);

        Ok(())
    }

    pub async fn resume(&mut self) -> GCResult<()> {
        let info = self
            .resume_info
            .as_ref()
            .ok_or(GCError::Misc(
                None,
                "Cannot resume, lacking Resume Info from the Ready event".into(),
            ))?
            .clone();
        self._connect(format!("{}/?v=10&encoding=json", info.gateway_url))
            .await?;

        self.send_event(&GatewayEvent {
            d: Some(json!({
                "token": self.token,
                "session_id": info.session_id,
                "seq": self.last_sequence
            })),
            ..GatewayEvent::new(GatewayOpcode::RESUME)
        })
        .await?;

        debug!("Resumed connection with the gateway");

        Ok(())
    }

    pub async fn reconnect(&mut self) -> GCResult<()> {
        self.resume_info = None;
        self._connect(fetch_wss_url().await?).await?;
        Ok(())
    }

    async fn identify(&mut self) -> GCResult<()> {
        let mut ident = GatewayEvent::new(GatewayOpcode::IDENTIFY);
        ident.d = Some(json!({
            "token": self.token,
            "properties": {
                "os": "Windows",
                "browser": "danielek",
                "device": "danielek"
            },
            "intents": self.intents
        }));

        self.send_event(&ident).await
    }

    async fn handle_thread_message(&mut self, msg: GatewayThreadMessage) -> GCResult<()> {
        match msg {
            GatewayThreadMessage::SendEvent(e, res) => {
                res.send(self.send_event(&e).await).unwrap();
                Ok(())
            }
        }
    }

    pub async fn conn_loop(&mut self) -> GCResult<()> {
        loop {
            if self.last_heartbeat - self.last_ack >= Duration::from_secs(5)
                && self.last_heartbeat.elapsed() >= Duration::from_secs(5)
            {
                Err(GCError::NoHeartbeat)?;
            }

            select! {
                _ = tokio::time::sleep_until(self.next_heartbeat) => {
                    self.send_heartbeat().await?;
                }

                msg = self.comm_rx.recv() => {
                    match msg {
                        None => Err(GCError::Shutdown)?,
                        Some(msg) => self.handle_thread_message(msg).await?
                    }
                }

                ormsg = self.ws.next() => {
                    match ormsg {
                        None => Err(GCError::Misc(None, "Trying to read from a closed connection".into()))?,
                        Some(Ok(msg)) => self.handle_ws_msg(msg).await?,
                        Some(Err(why)) => Err(why)?
                    }
                }

                _ = tokio::time::sleep(Duration::from_secs(4)) => ()
            }
        }
    }

    pub async fn send_event(&mut self, event: &GatewayEvent) -> GCResult<()> {
        self.ws
            .send(Message::Text(serde_json::to_string(event).map_err(
                |e| GCError::Misc(Some(e.into()), "Stringifying GatewayEvent failed".into()),
            )?))
            .await
            .map_err(GCError::SendError)
    }

    async fn handle_ws_msg(&mut self, msg: Message) -> GCResult<()> {
        match msg {
            Message::Text(msg) => {
                match serde_json::from_str::<GatewayEvent>(&msg) {
                    Err(why) => Err(GCError::Misc(
                        Some(why.into()),
                        format!("Error while deserializing {:#?} into GatewayEvent", msg).into(),
                    ))?,
                    Ok(event) => {
                        if event.op == GatewayOpcode::HELLO {
                            self.heartbeat_interval = Duration::from_millis(
                                event
                                    .d
                                    .as_ref()
                                    .and_then(|d| d["heartbeat_interval"].as_u64())
                                    .ok_or(GCError::InvalidPayload(
                                        "Hello payload has no heartbeat information".into(),
                                    ))?,
                            );
                            self.next_heartbeat = Instant::now() + self.heartbeat_interval / 2;
                            if self.resume_info.is_none() {
                                //when resuming we shouldn't identify
                                self.identify().await?;
                            }
                        }

                        if let Some(s) = event.s.as_ref() {
                            self.last_sequence = *s;
                        }

                        if let Some(t) = event.t.as_ref() {
                            if t == "READY" {
                                self.resume_info = Some(ResumeInfo {
                                    gateway_url: event
                                        .d
                                        .as_ref()
                                        .and_then(|d| d["resume_gateway_url"].as_str())
                                        .ok_or(GCError::InvalidPayload(
                                            "READY payload has no ResumeInfo information".into(),
                                        ))?
                                        .to_owned(),
                                    session_id: event
                                        .d
                                        .as_ref()
                                        .and_then(|d| d["session_id"].as_str())
                                        .ok_or(GCError::InvalidPayload(
                                            "READY payload has no ResumeInfo information".into(),
                                        ))?
                                        .to_owned(),
                                });
                            }
                        }

                        if event.op == GatewayOpcode::RECONNECT
                            || event.op == GatewayOpcode::INVALID_SESSION
                                && event
                                    .d
                                    .as_ref()
                                    .is_some_and(|d| d.as_bool().unwrap_or(false))
                        {
                            Err(GCError::ReconnectableClose(None))?;
                        } else if event.op == GatewayOpcode::INVALID_SESSION {
                            Err(GCError::UnexpectedClose(None))?;
                        }

                        if event.op == GatewayOpcode::HEARTBEAT_ACK {
                            self.last_ack = Instant::now();
                            self.ping.store(
                                (self.last_ack - self.last_heartbeat).as_millis() as u64,
                                std::sync::atomic::Ordering::Relaxed,
                            );
                        }

                        if event.op == GatewayOpcode::HEARTBEAT {
                            self.next_heartbeat = Instant::now();
                        }

                        self.evnt_tx
                            .send(Ok(event))
                            .map_err(|e| GCError::InternalChannelError(e.into()))?;
                    }
                }
            }

            Message::Close(msg) => {
                if let Some(frame) = msg {
                    if Self::RECONNECTABLE_CLOSES.contains(&frame.code.into()) {
                        Err(GCError::ReconnectableClose(Some(frame)))?;
                    } else if Self::UNRECONNECTABLE_CLOSES.contains(&frame.code.into()) {
                        Err(GCError::UnreconnectableClose(frame))?;
                    } else {
                        Err(GCError::UnexpectedClose(Some(frame)))?;
                    }
                } else {
                    Err(GCError::UnexpectedClose(None))?;
                }
            }

            _ => Err(GCError::Misc(
                None,
                format!(
                    "Received unexpected type of a Websocket message: {:#?}",
                    msg
                )
                .into(),
            ))?,
        }

        Ok(())
    }

    async fn send_heartbeat(&mut self) -> GCResult<()> {
        let pay = GatewayEvent {
            op: GatewayOpcode::HEARTBEAT,
            d: Some(json!(self.last_sequence)),
            s: None,
            t: None,
        };

        self.send_event(&pay).await?;

        self.last_heartbeat = Instant::now();
        self.next_heartbeat += self.heartbeat_interval;
        Ok(())
    }
}
