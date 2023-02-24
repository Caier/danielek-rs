use std::time::Duration;

use log::debug;
use tokio::{sync::{mpsc, oneshot}, time::Instant};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_tungstenite::{tungstenite::protocol::WebSocketConfig, connect_async_with_config};

use crate::{gateway::{util::try_x_times, error::GCError}};

use super::{connection::{GatewayThreadMessage, GatewayConnection}, types::{GatewayEvent, GatewayIntents}, util::fetch_wss_url, error::GCResult};

pub struct GatewayShard {
    comm_tx: mpsc::Sender<GatewayThreadMessage>,
    evnt_rx: Option<UnboundedReceiverStream<GCResult<GatewayEvent>>>
}

impl GatewayShard {
    pub async fn new(token: impl Into<String>, intents: GatewayIntents, force_reconnect: bool) -> GCResult<GatewayShard> {
        let ws_config = WebSocketConfig {
            max_send_queue: None,
            max_message_size: Some(1 << 30),
            max_frame_size: Some(1 << 28),
            accept_unmasked_frames: false
        };

        let wss_url = fetch_wss_url().await?;
        let (ws, _) = connect_async_with_config(wss_url, Some(ws_config)).await?;

        let (comm_tx, comm_rx) = tokio::sync::mpsc::channel(32);
        let (evnt_tx, evnt_rx) = mpsc::unbounded_channel();
        
        let mut conn = GatewayConnection {
            comm_rx,
            evnt_tx,
            ws,
            next_heartbeat: Instant::now() + Duration::from_secs(3600),
            last_heartbeat: Instant::now(),
            last_ack: Instant::now(),
            heartbeat_interval: Duration::from_secs(3600),
            last_sequence: 0,
            token: token.into(),
            intents: intents.bits(),
            resume_info: None,
            websocket_config: ws_config,
            force_reconnect
        };

        tokio::spawn(async move {
            'mainl: loop {
                let res = conn.conn_loop().await;
                dbg!(&res);
                match res {
                    Ok(_) => unreachable!(),
                    Err(why) => match why {
                        GCError::ReconnectableClose(_) | GCError::NoHeartbeat => {
                            loop {
                                debug!("Attempting resume...");
                                if let Err(why) = try_x_times!(5, conn.resume().await) {
                                    debug!("Resuming failed with: {}", why);
                                    if !conn.force_reconnect {
                                        conn.evnt_tx.send(Err(why)).ok();
                                        break 'mainl;
                                    }
                                } else {
                                    break;
                                }
                            }
                        }

                        GCError::UnexpectedClose(_) => {
                            loop {
                                debug!("Attempting reconnect...");
                                if let Err(why) = try_x_times!(5, conn.reconnect().await) {
                                    debug!("Reconnecting failed with: {}", why);
                                    if !conn.force_reconnect {
                                        conn.evnt_tx.send(Err(why)).ok();
                                        break 'mainl;
                                    }
                                } else {
                                    break;
                                }
                            }
                        }

                        GCError::Shutdown => {
                            conn.evnt_tx.send(Err(GCError::Shutdown)).ok();
                            break;
                        }

                        e => {
                            loop {
                                debug!("Connection irrecoverably failed with: {}", e);
                                if !conn.force_reconnect {
                                    conn.evnt_tx.send(Err(e)).ok();
                                    break 'mainl;
                                } else {
                                    debug!("Attempting reconnect...");
                                    if let Ok(_) = try_x_times!(5, conn.reconnect().await) {
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            dbg!("Closed shard thread");
        });

        Ok(GatewayShard { comm_tx, evnt_rx: Some(UnboundedReceiverStream::new(evnt_rx)) })
    }

    pub async fn send(&mut self, event: GatewayEvent) -> GCResult<()> {
        let (tx, rx) = oneshot::channel();
        self.comm_tx.send(GatewayThreadMessage::SendEvent(event, tx)).await.unwrap();
        rx.await.unwrap()
    }

    pub fn get_event_stream(&mut self) -> Option<UnboundedReceiverStream<GCResult<GatewayEvent>>> {
        self.evnt_rx.take()
    }

    pub fn get_event_stream_mut(&mut self) -> Option<&mut UnboundedReceiverStream<GCResult<GatewayEvent>>> {
        self.evnt_rx.as_mut()
    }
}