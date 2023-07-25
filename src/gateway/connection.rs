use std::{
    sync::{atomic::AtomicU64, Arc},
    time::Duration
};

use futures_util::{SinkExt, StreamExt};
use log::debug;
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
use smartstring::alias::String;

use crate::gateway::{types::GatewayOpcode, fake_types::{GatewayData, GatewayResumePayload}};

use super::{
    error::{GCError, GCResult},
    types::{ResumeInfo, GatewayIntents},
    fake_types::{GatewayEvent, GatewayIdentifyPayload, GatewayConnectionProperties},
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
    pub last_sequence: i64,
    pub token: String,
    pub intents: GatewayIntents,
    pub resume_info: Option<ResumeInfo>,
    pub websocket_config: WebSocketConfig,
    pub force_reconnect: bool,
    pub ping: Arc<AtomicU64>,
}

impl GatewayConnection {
    const RECONNECTABLE_CLOSES: [u16; 8] = [4000, 4001, 4002, 4003, 4005, 4007, 4008, 4009];
    const UNRECONNECTABLE_CLOSES: [u16; 6] = [4004, 4010, 4011, 4012, 4013, 4014];

    async fn _connect(&mut self, url: impl Into<std::string::String>) -> GCResult<()> {
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
            d: Some(GatewayData::SendResume(GatewayResumePayload {
                token: self.token.clone(),
                session_id: info.session_id,
                seq: self.last_sequence
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
        ident.d = Some(GatewayData::SendIdentify(GatewayIdentifyPayload {
            token: self.token.clone(),
            properties: GatewayConnectionProperties {
                os: "Windows".into(),
                browser: "danielek".into(),
                device: "danielek".into()
            },
            intents: self.intents,
            presence: None,
            compress: None,
            large_threshold: None,
            shard: None
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
            .send(Message::Text(serde_json::to_string(event).map_err(GCError::Serialization)?))
            .await
            .map_err(GCError::SendError)
    }

    async fn handle_ws_msg(&mut self, msg: Message) -> GCResult<()> {
        match msg {
            Message::Text(msg) => {
                let event = serde_json::from_str::<GatewayEvent>(&msg)
                    .map_err(|e| GCError::Deserialization(
                        format_serde_error::SerdeError::new(msg, e)
                    ))?;
                if let Some(GatewayData::Hello(h)) = event.d {
                    self.heartbeat_interval = Duration::from_millis(h.heartbeat_interval as u64);
                    self.next_heartbeat = Instant::now() + self.heartbeat_interval / 2;
                    if self.resume_info.is_none() {
                        //when resuming we shouldn't identify
                        self.identify().await?;
                    }
                }

                if let Some(s) = event.s.as_ref() {
                    self.last_sequence = *s;
                }

                if let Some(GatewayData::Ready(ref rdy)) = event.d {
                    self.resume_info = Some(ResumeInfo {
                        gateway_url: rdy.resume_gateway_url.clone(),
                        session_id: rdy.session_id.clone()
                    });
                }

                if event.op == GatewayOpcode::RECONNECT || matches!(event.d, Some(GatewayData::InvalidSession(rec)) if rec) {
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
            d: Some(GatewayData::SendHeartbeat(self.last_sequence)),
            s: None,
            t: None,
        };

        self.send_event(&pay).await?;

        self.last_heartbeat = Instant::now();
        self.next_heartbeat += self.heartbeat_interval;
        Ok(())
    }
}
