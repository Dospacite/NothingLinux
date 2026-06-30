use nothing_core::{ConnectionState, DeviceCapabilities, DeviceEvent, DeviceSnapshot};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewState {
    pub status: String,
    pub controls_enabled: bool,
    pub spinner: bool,
    pub unsupported_reason: Option<String>,
}

#[must_use]
pub fn map_snapshot(snapshot: &DeviceSnapshot, capabilities: &DeviceCapabilities) -> ViewState {
    let (status, spinner) = match snapshot.connection {
        ConnectionState::Disconnected => ("Disconnected", false),
        ConnectionState::Connecting => ("Connecting…", true),
        ConnectionState::Activating => ("Activating secure control…", true),
        ConnectionState::Syncing => ("Reading settings…", true),
        ConnectionState::Ready => ("Connected", false),
        ConnectionState::Recovering => ("Reconnecting…", true),
        ConnectionState::Unsupported => ("Unsupported device", false),
    };
    let unsupported_reason = (snapshot.connection == ConnectionState::Unsupported).then(|| {
        format!(
            "{} has no verified write profile",
            snapshot.model.as_deref().unwrap_or("This model")
        )
    });
    ViewState {
        status: status.into(),
        controls_enabled: snapshot.connection == ConnectionState::Ready
            && capabilities.model == "B171",
        spinner,
        unsupported_reason,
    }
}

#[must_use]
pub fn failure_message(event: &DeviceEvent) -> Option<String> {
    if let DeviceEvent::CommandFailed { reason, .. } = event {
        Some(reason.clone())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gates_controls_until_ready() {
        let mut snapshot = DeviceSnapshot::default();
        assert!(!map_snapshot(&snapshot, &DeviceCapabilities::b171()).controls_enabled);
        snapshot.connection = ConnectionState::Ready;
        assert!(map_snapshot(&snapshot, &DeviceCapabilities::b171()).controls_enabled);
    }
    #[test]
    fn unknown_profile_never_enables_writes() {
        let snapshot = DeviceSnapshot {
            connection: ConnectionState::Ready,
            ..DeviceSnapshot::default()
        };
        assert!(!map_snapshot(&snapshot, &DeviceCapabilities::unknown("B999")).controls_enabled);
    }
}
