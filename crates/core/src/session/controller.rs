use super::{CONNECT_TIMEOUT, REPAIR_COOLDOWN, RfcommTransport, SessionError, run_session};
use nothing_protocol::{ConnectionState, DeviceCommand, DeviceEvent};
use std::{io::ErrorKind, time::Duration};
use tokio::{
    sync::mpsc,
    time::{Instant, timeout},
};

#[derive(Clone)]
pub struct ControllerHandle {
    commands: mpsc::Sender<DeviceCommand>,
}

impl ControllerHandle {
    pub async fn send(&self, command: DeviceCommand) -> Result<(), SessionError> {
        self.commands
            .send(command)
            .await
            .map_err(|_| SessionError::CommandsClosed)
    }
}

pub struct Controller {
    pub handle: ControllerHandle,
    pub events: mpsc::UnboundedReceiver<DeviceEvent>,
}

impl Controller {
    #[must_use]
    pub fn spawn(address: String) -> Self {
        Self::spawn_inner(address, None)
    }

    /// Starts a controller that can repair a stale BlueZ ACL connection when
    /// RFCOMM reports `ENOTCONN` after a successful socket connect.
    #[must_use]
    pub fn spawn_managed(device: bluer::Device) -> Self {
        Self::spawn_inner(device.address().to_string(), Some(device))
    }

    fn spawn_inner(address: String, device: Option<bluer::Device>) -> Self {
        let (command_tx, mut command_rx) = mpsc::channel(32);
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        tokio::spawn(async move {
            let delays = [1_u64, 2, 5, 10, 30];
            let mut attempt = 0_usize;
            let mut last_repair = None;
            loop {
                let state = if attempt == 0 {
                    ConnectionState::Connecting
                } else {
                    ConnectionState::Recovering
                };
                let _ = event_tx.send(DeviceEvent::ConnectionChanged(state));
                if let Some(device) = &device
                    && let Err(error) = ensure_bluez_connected(device).await
                {
                    tracing::warn!(error = %error, "BlueZ device connection failed");
                }
                let mut stale_link = false;
                match RfcommTransport::connect(&address).await {
                    Ok(mut transport) => {
                        match run_session(&mut transport, &event_tx, &mut command_rx).await {
                            Ok(()) => attempt = 0,
                            Err(error) => {
                                stale_link = is_not_connected(&error);
                                tracing::warn!(error = %error, "control session ended");
                            }
                        }
                    }
                    Err(error) => {
                        stale_link = is_not_connected(&error);
                        tracing::warn!(error = %error, "RFCOMM connection failed");
                    }
                }
                let _ = event_tx.send(DeviceEvent::ConnectionChanged(ConnectionState::Recovering));
                let repair_allowed = last_repair
                    .is_none_or(|when| Instant::now().duration_since(when) >= REPAIR_COOLDOWN);
                if stale_link
                    && repair_allowed
                    && let Some(device) = &device
                {
                    last_repair = Some(Instant::now());
                    tracing::info!("repairing stale BlueZ connection after RFCOMM ENOTCONN");
                    match repair_bluez_connection(device).await {
                        Ok(()) => {
                            attempt = 0;
                            tokio::time::sleep(Duration::from_secs(1)).await;
                            continue;
                        }
                        Err(error) => {
                            tracing::warn!(error = %error, "BlueZ connection repair failed")
                        }
                    }
                }
                let delay = delays[attempt.min(delays.len() - 1)];
                attempt = attempt.saturating_add(1);
                tokio::time::sleep(Duration::from_secs(delay)).await;
            }
        });
        Self {
            handle: ControllerHandle {
                commands: command_tx,
            },
            events: event_rx,
        }
    }
}

fn is_not_connected(error: &SessionError) -> bool {
    matches!(error, SessionError::Io(error) if error.kind() == ErrorKind::NotConnected)
}

async fn ensure_bluez_connected(device: &bluer::Device) -> Result<(), SessionError> {
    if !device.is_connected().await? {
        match timeout(CONNECT_TIMEOUT, device.connect())
            .await
            .map_err(|_| SessionError::RepairTimeout)?
        {
            Ok(()) => {}
            Err(error) if error.kind == bluer::ErrorKind::AlreadyConnected => {}
            Err(error) => return Err(error.into()),
        }
        wait_for_bluez_state(device, true, CONNECT_TIMEOUT).await?;
    }
    Ok(())
}

async fn repair_bluez_connection(device: &bluer::Device) -> Result<(), SessionError> {
    if device.is_connected().await? {
        timeout(CONNECT_TIMEOUT, device.disconnect())
            .await
            .map_err(|_| SessionError::RepairTimeout)??;
        wait_for_bluez_state(device, false, CONNECT_TIMEOUT).await?;
    }
    tokio::time::sleep(Duration::from_millis(300)).await;
    match timeout(CONNECT_TIMEOUT, device.connect())
        .await
        .map_err(|_| SessionError::RepairTimeout)?
    {
        Ok(()) => {}
        Err(error) if error.kind == bluer::ErrorKind::AlreadyConnected => {}
        Err(error) => return Err(error.into()),
    }
    wait_for_bluez_state(device, true, CONNECT_TIMEOUT).await
}

async fn wait_for_bluez_state(
    device: &bluer::Device,
    connected: bool,
    timeout_duration: Duration,
) -> Result<(), SessionError> {
    let deadline = Instant::now() + timeout_duration;
    loop {
        if device.is_connected().await? == connected {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(SessionError::RepairTimeout);
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_not_connected_errors_trigger_link_repair() {
        let stale = SessionError::Io(std::io::Error::from(ErrorKind::NotConnected));
        let refused = SessionError::Io(std::io::Error::from(ErrorKind::ConnectionRefused));
        assert!(is_not_connected(&stale));
        assert!(!is_not_connected(&refused));
        assert!(!is_not_connected(&SessionError::ActivationTimeout));
    }
}
