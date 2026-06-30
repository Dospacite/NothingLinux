mod controller;
mod runner;
mod transport;

use std::time::Duration;
use thiserror::Error;

pub use controller::{Controller, ControllerHandle};
pub use runner::run_session;
pub use transport::{FrameTransport, RfcommTransport};

const COMMAND_TIMEOUT: Duration = Duration::from_secs(4);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(8);
const SOCKET_READY_TIMEOUT: Duration = Duration::from_secs(2);
const REPAIR_COOLDOWN: Duration = Duration::from_secs(30);

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("Bluetooth transport error: {0}")]
    Bluetooth(#[from] bluer::Error),
    #[error("transport I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("protocol error: {0}")]
    Protocol(#[from] nothing_protocol::ProtocolError),
    #[error("invalid Bluetooth address")]
    InvalidAddress,
    #[error("control channel closed")]
    CommandsClosed,
    #[error("device did not answer the activation handshake")]
    ActivationTimeout,
    #[error("RFCOMM connection attempt timed out")]
    ConnectTimeout,
    #[error("Bluetooth connection repair timed out")]
    RepairTimeout,
}
