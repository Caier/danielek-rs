use std::{
    sync::{atomic::AtomicU64, Arc},
    time::Duration,
};

use log::{debug, error, warn};
use tokio::{
    sync::{mpsc, oneshot},
    time::Instant,
};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_tungstenite::{connect_async_with_config, tungstenite::protocol::WebSocketConfig};

use crate::gateway::{error::GCError, util::try_x_times};

use super::{
    connection::{GatewayConnection, GatewayThreadMessage},
    error::GCResult,
    types::{GatewayEvent, GatewayIntents},
    util::fetch_wss_url,
};

pub struct GatewayShard {
    comm_tx: mpsc::Sender<GatewayThreadMessage>,
    evnt_rx: Option<UnboundedReceiverStream<GCResult<GatewayEvent>>>,
    ping: Arc<AtomicU64>,
}

impl GatewayShard {
    pub async fn new(
        token: impl Into<String>,
        intents: GatewayIntents,
        force_reconnect: bool,
    ) -> GCResult<GatewayShard> {
        let ws_config = WebSocketConfig {
            max_send_queue: None,
            max_message_size: Some(1 << 30),
            max_frame_size: Some(1 << 28),
            accept_unmasked_frames: false,
        };

        let wss_url = fetch_wss_url().await?;
        let (ws, _) = connect_async_with_config(wss_url, Some(ws_config)).await?;

        let (comm_tx, comm_rx) = tokio::sync::mpsc::channel(32);
        let (evnt_tx, evnt_rx) = mpsc::unbounded_channel();

        let ping = Arc::new(AtomicU64::new(999));

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
            force_reconnect,
            ping: Arc::clone(&ping),
        };

        tokio::spawn(async move {
            'mainl: loop {
                let res = conn.conn_loop().await;
                match res {
                    Ok(_) => unreachable!(),
                    Err(why) => match why {
                        //non-fatal connection close, handle as per documentation
                        GCError::ReconnectableClose(_) | GCError::NoHeartbeat => loop {
                            debug!("Attempting resume because of {}", why);
                            if let Err(why) = try_x_times!(5, conn.resume().await) {
                                error!("Resuming failed with: {}", why);
                                if !conn.force_reconnect {
                                    conn.evnt_tx.send(Err(why)).ok();
                                    break 'mainl;
                                }
                            } else {
                                break;
                            }
                        },

                        //fatal, but documented close, will not reconnect (ex. Invalid token)
                        GCError::UnreconnectableClose(_) => {
                            error!("Connection failed with {}", why);
                            conn.evnt_tx.send(Err(why)).ok();
                            break;
                        }

                        GCError::Shutdown => {
                            conn.evnt_tx.send(Err(GCError::Shutdown)).ok();
                            break;
                        }

                        //other unexpected and undocumented errors ex. no internet, Protocol(ResetWithoutClosingHandshake). try reconnect
                        e => loop {
                            warn!("Unexpected connection error: {}", e);
                            if let Err(why) = try_x_times!(5, conn.reconnect().await) {
                                error!("Reconnecting failed with: {}", why);
                                if !conn.force_reconnect {
                                    conn.evnt_tx.send(Err(why)).ok();
                                    break 'mainl;
                                }
                            } else {
                                break;
                            }
                        },
                    },
                }
            }
            debug!("Closed shard thread");
        });

        Ok(GatewayShard {
            comm_tx,
            evnt_rx: Some(UnboundedReceiverStream::new(evnt_rx)),
            ping,
        })
    }

    pub async fn send(&mut self, event: GatewayEvent) -> GCResult<()> {
        let (tx, rx) = oneshot::channel();
        self.comm_tx
            .send(GatewayThreadMessage::SendEvent(event, tx))
            .await
            .unwrap();
        rx.await.unwrap()
    }

    pub fn get_event_stream(&mut self) -> Option<UnboundedReceiverStream<GCResult<GatewayEvent>>> {
        self.evnt_rx.take()
    }

    #[allow(unused)]
    pub fn get_event_stream_mut(
        &mut self,
    ) -> Option<&mut UnboundedReceiverStream<GCResult<GatewayEvent>>> {
        self.evnt_rx.as_mut()
    }

    pub fn get_ping(&self) -> u64 {
        self.ping.load(std::sync::atomic::Ordering::Relaxed)
    }
}
