use super::{CONNECT_TIMEOUT, SOCKET_READY_TIMEOUT, SessionError};
use async_trait::async_trait;
use bluer::{
    Address,
    rfcomm::{SocketAddr, Stream},
};
use std::{str::FromStr, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    time::Instant,
};

const RFCOMM_CHANNEL: u8 = 15;

#[async_trait]
pub trait FrameTransport: Send {
    async fn read(&mut self, buffer: &mut [u8]) -> Result<usize, SessionError>;
    async fn write_all(&mut self, bytes: &[u8]) -> Result<(), SessionError>;
}

pub struct RfcommTransport(Stream);

impl RfcommTransport {
    pub async fn connect(address: &str) -> Result<Self, SessionError> {
        let address = Address::from_str(address).map_err(|_| SessionError::InvalidAddress)?;
        let stream = tokio::time::timeout(
            CONNECT_TIMEOUT,
            Stream::connect(SocketAddr::new(address, RFCOMM_CHANNEL)),
        )
        .await
        .map_err(|_| SessionError::ConnectTimeout)??;

        // On some kernel/BlueZ combinations an RFCOMM socket becomes writable with
        // SO_ERROR=0 before the DLC is fully attached.  A write in that window fails
        // with ENOTCONN.  getpeername() is the reliable readiness check.
        let deadline = Instant::now() + SOCKET_READY_TIMEOUT;
        loop {
            match stream.peer_addr() {
                Ok(_) => return Ok(Self(stream)),
                Err(error) if Instant::now() < deadline => {
                    tracing::debug!(error = %error, "waiting for RFCOMM peer attachment");
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
                Err(error) => return Err(SessionError::Io(error)),
            }
        }
    }
}

#[async_trait]
impl FrameTransport for RfcommTransport {
    async fn read(&mut self, buffer: &mut [u8]) -> Result<usize, SessionError> {
        Ok(self.0.read(buffer).await?)
    }

    async fn write_all(&mut self, bytes: &[u8]) -> Result<(), SessionError> {
        self.0.write_all(bytes).await?;
        Ok(())
    }
}
