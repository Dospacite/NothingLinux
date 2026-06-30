use super::{COMMAND_TIMEOUT, FrameTransport, SessionError};
use nothing_protocol::{
    BatteryState, ConnectionState, DeviceCommand, DeviceEvent, DeviceSnapshot, Frame, FrameDecoder,
    command, decode_event, encode_command,
};
use std::{collections::HashMap, time::Duration};
use tokio::{
    sync::mpsc,
    time::{Instant, interval},
};

struct Pending {
    command: DeviceCommand,
    deadline: Instant,
}

pub async fn run_session<T: FrameTransport>(
    transport: &mut T,
    events: &mpsc::UnboundedSender<DeviceEvent>,
    commands: &mut mpsc::Receiver<DeviceCommand>,
) -> Result<(), SessionError> {
    let mut sequence = 0_u8;
    let mut decoder = FrameDecoder::new();
    let mut snapshot = DeviceSnapshot {
        connection: ConnectionState::Activating,
        ..DeviceSnapshot::default()
    };
    let mut pending = HashMap::<u8, Pending>::new();
    let mut supported = false;
    let mut sync_started = false;
    let activation_deadline = Instant::now() + COMMAND_TIMEOUT;
    let mut identification_deadline = None;
    let mut low_battery = LowBatteryCycles::default();
    events
        .send(DeviceEvent::ConnectionChanged(ConnectionState::Activating))
        .ok();
    send_raw(transport, command::QUERY_PROTOCOL, &[], &mut sequence).await?;
    let mut timeout_tick = interval(Duration::from_millis(200));
    let mut buffer = [0_u8; 1024];

    loop {
        tokio::select! {
            read = transport.read(&mut buffer) => {
                let count = read?;
                if count == 0 {
                    events.send(DeviceEvent::ConnectionChanged(ConnectionState::Disconnected)).ok();
                    return Ok(());
                }
                for frame in decoder.push(&buffer[..count])? {
                    let normalized = normalize(frame.command);
                    if normalized == command::QUERY_PROTOCOL {
                        let activation = encode_command(&DeviceCommand::Activate, next_sequence(&mut sequence))?;
                        transport.write_all(&activation.encode()?).await?;
                        continue;
                    }
                    if normalized == command::ACTIVATE && !sync_started {
                        sync_started = true;
                        identification_deadline = Some(Instant::now() + COMMAND_TIMEOUT);
                        snapshot.connection = ConnectionState::Syncing;
                        events.send(DeviceEvent::ConnectionChanged(ConnectionState::Syncing)).ok();
                        send_sync_queries(transport, &mut sequence).await?;
                    }
                    if normalized == command::QUERY_REMOTE_CONFIG {
                        if identify_model(&frame.payload).as_deref() == Some("B171") {
                            supported = true;
                            snapshot.model = Some("B171".into());
                            snapshot.connection = ConnectionState::Ready;
                            events.send(DeviceEvent::Snapshot(snapshot.clone())).ok();
                            events.send(DeviceEvent::ConnectionChanged(ConnectionState::Ready)).ok();
                        } else {
                            snapshot.connection = ConnectionState::Unsupported;
                            events.send(DeviceEvent::ConnectionChanged(ConnectionState::Unsupported)).ok();
                        }
                        continue;
                    }
                    if let Some(event) = decode_event(&frame)? {
                        if let DeviceEvent::Acknowledged { sequence: ack_sequence, .. } = &event
                            && let Some(pending_command) = pending.remove(ack_sequence)
                            && let Some(query) = confirmation_query(&pending_command.command)
                        {
                            send_command(transport, &query, &mut sequence).await?;
                        }
                        apply_event(&mut snapshot, &event);
                        if let DeviceEvent::Battery(state) = &event {
                            low_battery.update(state);
                        }
                        events.send(event).ok();
                    }
                }
            }
            maybe_command = commands.recv() => {
                let command_value = maybe_command.ok_or(SessionError::CommandsClosed)?;
                if is_mutation(&command_value) && !supported {
                    events.send(DeviceEvent::CommandFailed {
                        command: command_value,
                        reason: "Writes are blocked until the device identifies as B171".into(),
                    }).ok();
                    continue;
                }
                let command_sequence = next_sequence(&mut sequence);
                let frame = encode_command(&command_value, command_sequence);
                match frame {
                    Ok(frame) => {
                        transport.write_all(&frame.encode()?).await?;
                        if is_mutation(&command_value) {
                            pending.insert(command_sequence, Pending {
                                command: command_value,
                                deadline: Instant::now() + COMMAND_TIMEOUT,
                            });
                        }
                    }
                    Err(error) => {
                        events.send(DeviceEvent::CommandFailed {
                            command: command_value,
                            reason: error.to_string(),
                        }).ok();
                    }
                }
            }
            _ = timeout_tick.tick() => {
                let now = Instant::now();
                if !sync_started && now >= activation_deadline {
                    return Err(SessionError::ActivationTimeout);
                }
                if !supported && identification_deadline.is_some_and(|deadline| deadline <= now) {
                    snapshot.connection = ConnectionState::Unsupported;
                    events.send(DeviceEvent::ConnectionChanged(ConnectionState::Unsupported)).ok();
                    identification_deadline = None;
                }
                let expired: Vec<u8> = pending
                    .iter()
                    .filter_map(|(sequence, value)| (value.deadline <= now).then_some(*sequence))
                    .collect();
                for sequence in expired {
                    if let Some(value) = pending.remove(&sequence) {
                        events.send(DeviceEvent::CommandFailed {
                            command: value.command,
                            reason: "The earbuds did not acknowledge the change; the previous value was kept".into(),
                        }).ok();
                    }
                }
            }
        }
    }
}

fn next_sequence(sequence: &mut u8) -> u8 {
    *sequence = sequence.wrapping_add(1);
    *sequence
}

fn normalize(command_id: u16) -> u16 {
    if command_id & 0xe000 == 0xe000 {
        command_id
    } else {
        command_id | 0x8000
    }
}

async fn send_raw<T: FrameTransport>(
    transport: &mut T,
    id: u16,
    payload: &[u8],
    sequence: &mut u8,
) -> Result<(), SessionError> {
    let frame = Frame::new(id, next_sequence(sequence), payload.to_vec())?;
    transport.write_all(&frame.encode()?).await
}

async fn send_command<T: FrameTransport>(
    transport: &mut T,
    command_value: &DeviceCommand,
    sequence: &mut u8,
) -> Result<(), SessionError> {
    let frame = encode_command(command_value, next_sequence(sequence))?;
    transport.write_all(&frame.encode()?).await
}

async fn send_sync_queries<T: FrameTransport>(
    transport: &mut T,
    sequence: &mut u8,
) -> Result<(), SessionError> {
    send_raw(transport, command::QUERY_REMOTE_CONFIG, &[], sequence).await?;
    let queries = [
        DeviceCommand::QueryBattery,
        DeviceCommand::QueryWear,
        DeviceCommand::QueryAnc,
        DeviceCommand::QueryEq,
        DeviceCommand::QueryCustomEq,
        DeviceCommand::QueryAdvancedEqProfile,
        DeviceCommand::QueryFirmware,
        DeviceCommand::QueryGestures,
        DeviceCommand::QueryInEarDetection,
        DeviceCommand::QueryLowLag,
        DeviceCommand::QueryBassEnhance,
        DeviceCommand::QueryAdvancedEq,
        DeviceCommand::QueryAudioCodec,
        DeviceCommand::QueryDualConnection,
    ];
    for query in queries {
        send_command(transport, &query, sequence).await?;
    }
    Ok(())
}

fn identify_model(payload: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(payload);
    if text.contains("B171") {
        return Some("B171".into());
    }
    let configuration = payload
        .get(7..)
        .map(String::from_utf8_lossy)
        .unwrap_or_default();
    let serial = text
        .lines()
        .chain(configuration.lines())
        .find_map(|line| {
            let mut parts = line.split(',');
            let _device = parts.next()?;
            let kind = parts.next()?;
            let value = parts.next()?.trim_matches(char::from(0));
            (kind == "4").then_some(value)
        })
        .unwrap_or(text.trim_matches(char::from(0)).trim());
    if serial.is_ascii()
        && serial.len() >= 6
        && matches!(&serial[4..6], "61" | "62" | "69" | "70" | "74" | "75")
    {
        Some("B171".into())
    } else {
        None
    }
}

fn is_mutation(command: &DeviceCommand) -> bool {
    !matches!(
        command,
        DeviceCommand::Activate
            | DeviceCommand::QueryBattery
            | DeviceCommand::QueryWear
            | DeviceCommand::QueryAnc
            | DeviceCommand::QueryEq
            | DeviceCommand::QueryCustomEq
            | DeviceCommand::QueryAdvancedEqProfile
            | DeviceCommand::QueryFirmware
            | DeviceCommand::QueryGestures
            | DeviceCommand::QueryInEarDetection
            | DeviceCommand::QueryLowLag
            | DeviceCommand::QueryBassEnhance
            | DeviceCommand::QueryAdvancedEq
            | DeviceCommand::QueryAudioCodec
            | DeviceCommand::QueryDualConnection
    )
}

fn confirmation_query(command: &DeviceCommand) -> Option<DeviceCommand> {
    match command {
        DeviceCommand::SetAnc { .. } => Some(DeviceCommand::QueryAnc),
        DeviceCommand::SetEqPreset(_) => Some(DeviceCommand::QueryEq),
        DeviceCommand::SetCustomEq(_) => Some(DeviceCommand::QueryCustomEq),
        DeviceCommand::SetAdvancedEqEnabled(_) => Some(DeviceCommand::QueryAdvancedEq),
        DeviceCommand::SetAdvancedEqProfile(_) => Some(DeviceCommand::QueryAdvancedEqProfile),
        DeviceCommand::SetGesture { .. } => Some(DeviceCommand::QueryGestures),
        DeviceCommand::SetBassEnhance(_) => Some(DeviceCommand::QueryBassEnhance),
        DeviceCommand::SetInEarDetection(_) => Some(DeviceCommand::QueryInEarDetection),
        DeviceCommand::SetLowLag(_) => Some(DeviceCommand::QueryLowLag),
        DeviceCommand::SetAudioCodec(_) => Some(DeviceCommand::QueryAudioCodec),
        DeviceCommand::SetDualConnection(_) => Some(DeviceCommand::QueryDualConnection),
        _ => None,
    }
}

fn apply_event(snapshot: &mut DeviceSnapshot, event: &DeviceEvent) {
    match event {
        DeviceEvent::Battery(value) => snapshot.battery = value.clone(),
        DeviceEvent::Wear(value) => snapshot.wear = *value,
        DeviceEvent::Anc { mode, level } => {
            snapshot.anc_mode = *mode;
            snapshot.anc_level = *level;
        }
        DeviceEvent::Eq(value) => snapshot.eq_preset = *value,
        DeviceEvent::CustomEq(value) => snapshot.custom_eq = *value,
        DeviceEvent::AdvancedEqEnabled(value) => snapshot.advanced_eq_enabled = Some(*value),
        DeviceEvent::AdvancedEqProfile(value) => snapshot.advanced_eq_profile = Some(value.clone()),
        DeviceEvent::Gestures(value) => snapshot.gestures = value.clone(),
        DeviceEvent::BassEnhance(value) => snapshot.bass_enhance = *value,
        DeviceEvent::InEarDetection(value) => snapshot.in_ear_detection = Some(*value),
        DeviceEvent::LowLag(value) => snapshot.low_lag = Some(*value),
        DeviceEvent::AudioCodec(value) => snapshot.audio_codec = Some(*value),
        DeviceEvent::DualConnection(value) => snapshot.dual_connection = Some(*value),
        DeviceEvent::Firmware(value) => snapshot.firmware = Some(value.clone()),
        DeviceEvent::ConnectionChanged(value) => snapshot.connection = *value,
        _ => {}
    }
}

#[derive(Default)]
struct LowBatteryCycles {
    left: bool,
    right: bool,
    case: bool,
}

impl LowBatteryCycles {
    fn update(&mut self, battery: &BatteryState) {
        Self::slot("Left earbud", battery.left, &mut self.left);
        Self::slot("Right earbud", battery.right, &mut self.right);
        Self::slot("Case", battery.case, &mut self.case);
    }

    fn slot(name: &str, level: Option<nothing_protocol::ChargeLevel>, notified: &mut bool) {
        let Some(level) = level else {
            return;
        };
        if level.percent <= 20 && !level.charging && !*notified {
            *notified = true;
            let _ = notify_rust::Notification::new()
                .summary("Nothing Linux — low battery")
                .body(&format!("{name}: {}% remaining", level.percent))
                .icon("battery-caution-symbolic")
                .show();
        } else if level.percent > 25 || level.charging {
            *notified = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt, DuplexStream, duplex},
        sync::mpsc,
    };

    struct TestTransport(DuplexStream);

    #[async_trait]
    impl FrameTransport for TestTransport {
        async fn read(&mut self, buffer: &mut [u8]) -> Result<usize, SessionError> {
            Ok(self.0.read(buffer).await?)
        }

        async fn write_all(&mut self, bytes: &[u8]) -> Result<(), SessionError> {
            self.0.write_all(bytes).await?;
            Ok(())
        }
    }

    fn encode_test_custom_eq(gains: [f32; 3]) -> Vec<u8> {
        let mut payload = vec![
            3, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0x75, 0x44, 0xc3, 0xf5, 0x28, 0x3f, 2, 0, 0, 0, 0,
            0, 0xc0, 0x5a, 0x45, 0, 0, 0x80, 0x3f, 0, 0, 0, 0, 0, 0, 0, 0x0c, 0x43, 0xcd, 0xcc,
            0x4c, 0x3f, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        let headroom = gains.into_iter().fold(0.0_f32, f32::max);
        payload[1..5].copy_from_slice(&(-headroom).to_le_bytes());
        for (index, gain) in gains.into_iter().enumerate() {
            let offset = 6 + index * 13;
            payload[offset..offset + 4].copy_from_slice(&gain.to_le_bytes());
        }
        payload
    }

    fn encode_test_advanced_eq_profile() -> Vec<u8> {
        let frequencies = [
            55.0_f32, 110.0, 220.0, 440.0, 1_320.0, 3_300.0, 6_600.0, 13_200.0,
        ];
        let mut payload = Vec::with_capacity(110);
        payload.push(0);
        payload.push(frequencies.len() as u8);
        payload.extend_from_slice(&0.0_f32.to_le_bytes());
        for frequency in frequencies {
            payload.push(1);
            payload.extend_from_slice(&0.0_f32.to_le_bytes());
            payload.extend_from_slice(&frequency.to_le_bytes());
            payload.extend_from_slice(&1.0_f32.to_le_bytes());
        }
        payload
    }

    async fn respond(mut device: DuplexStream) {
        let mut decoder = FrameDecoder::new();
        let mut buffer = [0_u8; 512];
        while let Ok(count) = device.read(&mut buffer).await {
            if count == 0 {
                break;
            }
            for frame in decoder.push(&buffer[..count]).unwrap_or_default() {
                let (id, payload) = match frame.command {
                    command::QUERY_PROTOCOL => (0x4001, b"1.0".to_vec()),
                    command::ACTIVATE => (0x7001, vec![]),
                    command::QUERY_REMOTE_CONFIG => (0x4006, b"1,4,SH00610000000000\n".to_vec()),
                    command::QUERY_BATTERY => (0x4007, vec![3, 2, 80, 3, 79, 4, 60]),
                    command::QUERY_WEAR => (0x400a, vec![2, 2, 0x84, 3, 0x80]),
                    command::QUERY_ANC => (0x401e, vec![1, 5, 0]),
                    command::QUERY_EQ => (0x401f, vec![0]),
                    command::QUERY_CUSTOM_EQ => (0x4044, encode_test_custom_eq([0.0; 3])),
                    command::QUERY_ADVANCED_EQ_PROFILE => {
                        (0x404d, encode_test_advanced_eq_profile())
                    }
                    command::QUERY_FIRMWARE => (0x4042, b"1.2.3".to_vec()),
                    command::QUERY_GESTURES => (0x4018, vec![0]),
                    command::QUERY_IN_EAR => (0x400e, vec![1, 1, 1]),
                    command::QUERY_LOW_LAG => (0x4041, vec![2]),
                    command::QUERY_BASS => (0x404e, vec![1, 6]),
                    command::QUERY_ADVANCED_EQ => (0x404c, vec![0]),
                    command::QUERY_AUDIO_CODEC => (0x4029, vec![2]),
                    command::QUERY_DUAL_CONNECTION => (0x4027, vec![1]),
                    command::SET_ANC => (0x700f, vec![]),
                    command::SET_AUDIO_CODEC => (0x701c, vec![]),
                    command::SET_DUAL_CONNECTION => (0x701a, vec![]),
                    _ => continue,
                };
                let response = Frame::new(id, frame.sequence, payload)
                    .unwrap_or_else(|e| panic!("{e}"))
                    .encode()
                    .unwrap_or_else(|e| panic!("{e}"));
                if device.write_all(&response).await.is_err() {
                    return;
                }
            }
        }
    }

    #[tokio::test]
    async fn activation_sync_and_acknowledged_write() {
        let (host, device) = duplex(4096);
        tokio::spawn(respond(device));
        let mut transport = TestTransport(host);
        let (event_tx, mut event_rx) = mpsc::unbounded_channel();
        let (command_tx, mut command_rx) = mpsc::channel(4);
        let task =
            tokio::spawn(
                async move { run_session(&mut transport, &event_tx, &mut command_rx).await },
            );
        let ready = tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                if matches!(
                    event_rx.recv().await,
                    Some(DeviceEvent::ConnectionChanged(ConnectionState::Ready))
                ) {
                    break;
                }
            }
        })
        .await;
        assert!(ready.is_ok());
        command_tx
            .send(DeviceCommand::SetAnc {
                mode: nothing_protocol::AncMode::Off,
                level: nothing_protocol::AncLevel::High,
            })
            .await
            .unwrap_or_else(|e| panic!("{e}"));
        let ack = tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                if matches!(
                    event_rx.recv().await,
                    Some(DeviceEvent::Acknowledged {
                        command: command::SET_ANC,
                        ..
                    })
                ) {
                    break;
                }
            }
        })
        .await;
        assert!(ack.is_ok());
        task.abort();
    }

    #[test]
    fn model_identification_is_not_name_based() {
        assert_eq!(
            identify_model(b"1,4,SH00690000000000\n").as_deref(),
            Some("B171")
        );
        assert_eq!(identify_model(b"1,4,SH00630000000000\n"), None);
    }

    #[test]
    fn set_requires_readback_mapping() {
        assert_eq!(
            confirmation_query(&DeviceCommand::SetEqPreset(
                nothing_protocol::EqPreset::Balanced,
            )),
            Some(DeviceCommand::QueryEq)
        );
        assert_eq!(
            confirmation_query(&DeviceCommand::SetCustomEq([0.0; 3])),
            Some(DeviceCommand::QueryCustomEq)
        );
    }
}
