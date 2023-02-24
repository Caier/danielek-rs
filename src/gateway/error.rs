use std::{error::Error as StdError, fmt::{Display}, borrow::Cow};

use tokio_tungstenite::tungstenite::{protocol::CloseFrame, error::Error as WSError};

#[derive(Debug)]
pub enum GCError<'a> {
    GatewayURLFetch(Box<dyn StdError + Send + Sync>),
    InitialHandshake(Box<dyn StdError + Send + Sync>),
    InternalChannelError(Box<dyn StdError + Send + Sync>),
    UnexpectedClose(Option<CloseFrame<'a>>),
    UnreconnectableClose(CloseFrame<'a>),
    ReconnectableClose(Option<CloseFrame<'a>>),
    SendError(WSError),                     
    ConnectError(WSError),
    WSInternal(WSError),
    Shutdown,
    NoHeartbeat,
    Misc(Option<Box<dyn StdError + Send + Sync>>, Cow<'a, str>)
}

pub type GCResult<T> = Result<T, GCError<'static>>;

impl<'a> Display for GCError<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use GCError::*;

        match self {
            GatewayURLFetch(e) => write!(f, "Fetching the gateway URL from API failed: {}", e),
            InitialHandshake(e) => write!(f, "Cannot initiate the connection: {}", e),
            InternalChannelError(e) => write!(f, "An unhandled error occured while trying to use internal channels: {}", e),
            UnexpectedClose(Some(ref cf)) => write!(f, "The connection with the gateway unexpectedly closed with the following frame: {}", cf),
            UnexpectedClose(None) => write!(f, "The connection with the gateway unexpectedly closed without a frame"),
            UnreconnectableClose(cf) => write!(f, "The connection with the gateway was remotely closed: {}", cf),
            ReconnectableClose(Some(ref cf)) => write!(f, "The connection dropped or should drop, however it should be Resumed: {}", cf),
            ReconnectableClose(None) => write!(f, "The connection dropped or should drop, however it should be Resumed"),
            SendError(we) => write!(f, "Sending an event to the gateway failed with: {}", we),
            Shutdown => write!(f, "The connection thread should shut down, this error should've been handled"),
            ConnectError(we) => write!(f, "Connecting with the remote websocket failed: {}", we),
            WSInternal(we) => write!(f, "Unexpected WS error: {}", we),
            NoHeartbeat => write!(f, "Didn't receive a Heartbeat ACK in time"),
            Misc(Some(e), desc) => write!(f, "{}", format!("{}: {}", desc, e)),
            Misc(None, desc) => write!(f, "{}", desc)
        }
    }
}

impl<'a> StdError for GCError<'a> {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        use GCError::*;

        match self {
            GatewayURLFetch(e) 
            | InitialHandshake(e) 
            | Misc(Some(e), _)
            | InternalChannelError(e) => Some(&**e),

            SendError(we) | ConnectError(we) | WSInternal(we) => Some(we),

            _ => None
        }
    }
}

impl<'a> From<WSError> for GCError<'a> {
    fn from(we: WSError) -> Self {
        GCError::WSInternal(we)
    }
}