use crate::{
    AdvancedEqProfile, AncLevel, AncMode, AudioCodec, BatteryState, ChargeLevel, DeviceCommand,
    DeviceEvent, EarbudSide, EqBand, EqPreset, Frame, Gesture, GestureAction, ProtocolError,
    WearState,
};
use std::collections::BTreeMap;

pub mod command {
    pub const QUERY_PROTOCOL: u16 = 0xc001;
    pub const QUERY_REMOTE_CONFIG: u16 = 0xc006;
    pub const QUERY_BATTERY: u16 = 0xc007;
    pub const QUERY_WEAR: u16 = 0xc00a;
    pub const QUERY_IN_EAR: u16 = 0xc00e;
    pub const QUERY_GESTURES: u16 = 0xc018;
    pub const QUERY_ANC: u16 = 0xc01e;
    pub const QUERY_EQ: u16 = 0xc01f;
    pub const QUERY_LOW_LAG: u16 = 0xc041;
    pub const QUERY_FIRMWARE: u16 = 0xc042;
    pub const QUERY_CUSTOM_EQ: u16 = 0xc044;
    pub const QUERY_ADVANCED_EQ: u16 = 0xc04c;
    pub const QUERY_ADVANCED_EQ_PROFILE: u16 = 0xc04d;
    pub const QUERY_BASS: u16 = 0xc04e;
    pub const QUERY_AUDIO_CODEC: u16 = 0xc029;
    pub const QUERY_DUAL_CONNECTION: u16 = 0xc027;
    pub const ACTIVATE: u16 = 0xf001;
    pub const FIND_BUD: u16 = 0xf002;
    pub const SET_GESTURE: u16 = 0xf003;
    pub const SET_IN_EAR: u16 = 0xf004;
    pub const SET_ANC: u16 = 0xf00f;
    pub const SET_EQ: u16 = 0xf010;
    pub const START_FIT_TEST: u16 = 0xf014;
    pub const SET_LOW_LAG: u16 = 0xf040;
    pub const SET_CUSTOM_EQ: u16 = 0xf041;
    pub const SET_ADVANCED_EQ: u16 = 0xf04f;
    pub const SET_ADVANCED_EQ_PROFILE: u16 = 0xf050;
    pub const SET_AUDIO_CODEC: u16 = 0xf01c;
    pub const SET_BASS: u16 = 0xf051;
    pub const SET_DUAL_CONNECTION: u16 = 0xf01a;
    pub const EVENT_BATTERY: u16 = 0xe001;
    pub const EVENT_WEAR: u16 = 0xe002;
    pub const EVENT_ANC: u16 = 0xe003;
    pub const EVENT_FIT_TEST: u16 = 0xe00d;
}

pub fn encode_command(command_value: &DeviceCommand, sequence: u8) -> Result<Frame, ProtocolError> {
    use DeviceCommand as C;
    let (id, payload) = match command_value {
        C::Activate => (command::ACTIVATE, vec![]),
        C::QueryBattery => (command::QUERY_BATTERY, vec![]),
        C::QueryWear => (command::QUERY_WEAR, vec![]),
        C::QueryAnc => (command::QUERY_ANC, vec![3]),
        C::QueryEq => (command::QUERY_EQ, vec![]),
        C::QueryCustomEq => (command::QUERY_CUSTOM_EQ, vec![]),
        C::QueryAdvancedEqProfile => (command::QUERY_ADVANCED_EQ_PROFILE, vec![0]),
        C::QueryFirmware => (command::QUERY_FIRMWARE, vec![]),
        C::QueryGestures => (command::QUERY_GESTURES, vec![]),
        C::QueryInEarDetection => (command::QUERY_IN_EAR, vec![]),
        C::QueryLowLag => (command::QUERY_LOW_LAG, vec![]),
        C::QueryBassEnhance => (command::QUERY_BASS, vec![]),
        C::QueryAdvancedEq => (command::QUERY_ADVANCED_EQ, vec![]),
        C::QueryAudioCodec => (command::QUERY_AUDIO_CODEC, vec![]),
        C::QueryDualConnection => (command::QUERY_DUAL_CONNECTION, vec![]),
        C::SetAnc { mode, level } => (command::SET_ANC, vec![1, anc_wire(*mode, *level), 0]),
        C::SetEqPreset(preset) => (command::SET_EQ, vec![preset_wire(*preset)?]),
        C::SetCustomEq(gains) => (command::SET_CUSTOM_EQ, encode_custom_eq(*gains)?),
        C::SetAdvancedEqEnabled(enabled) => (command::SET_ADVANCED_EQ, vec![u8::from(*enabled)]),
        C::SetAdvancedEqProfile(profile) => (
            command::SET_ADVANCED_EQ_PROFILE,
            encode_advanced_eq(profile)?,
        ),
        C::SetGesture {
            side,
            gesture,
            action,
        } => {
            if *gesture == Gesture::SinglePinch || !gesture.allows(*action) {
                return Err(ProtocolError::InvalidValue("gesture action"));
            }
            (
                command::SET_GESTURE,
                vec![
                    1,
                    side_wire(*side),
                    1,
                    gesture_wire(*gesture),
                    action_wire(*action),
                ],
            )
        }
        C::SetBassEnhance(level) => {
            if level.is_some_and(|value| !(1..=5).contains(&value)) {
                return Err(ProtocolError::InvalidValue("Bass Enhance level"));
            }
            (
                command::SET_BASS,
                vec![u8::from(level.is_some()), level.unwrap_or(0) * 2],
            )
        }
        C::SetInEarDetection(enabled) => (command::SET_IN_EAR, vec![1, 1, u8::from(*enabled)]),
        C::SetLowLag(enabled) => (command::SET_LOW_LAG, vec![if *enabled { 1 } else { 2 }, 0]),
        C::FindBud { side, ringing } => (
            command::FIND_BUD,
            vec![side_wire(*side), u8::from(*ringing)],
        ),
        C::StartFitTest => (command::START_FIT_TEST, vec![1]),
        C::CancelFitTest => (command::START_FIT_TEST, vec![0]),
        C::SetDualConnection(enabled) => (command::SET_DUAL_CONNECTION, vec![u8::from(*enabled)]),
        C::SetAudioCodec(codec) => (command::SET_AUDIO_CODEC, vec![codec_wire(*codec)]),
    };
    Frame::new(id, sequence, payload)
}

fn anc_wire(mode: AncMode, level: AncLevel) -> u8 {
    match mode {
        AncMode::Off => 5,
        AncMode::Transparency => 7,
        AncMode::NoiseCancellation => match level {
            AncLevel::High => 1,
            AncLevel::Mid => 2,
            AncLevel::Low => 3,
            AncLevel::Adaptive => 4,
        },
    }
}

fn preset_wire(preset: EqPreset) -> Result<u8, ProtocolError> {
    match preset {
        EqPreset::Balanced => Ok(0),
        EqPreset::Voice => Ok(1),
        EqPreset::MoreTreble => Ok(2),
        EqPreset::MoreBass => Ok(3),
        EqPreset::Custom => Ok(5),
        EqPreset::Advanced => Err(ProtocolError::UnsupportedCommand(
            "select advanced EQ with SetAdvancedEqEnabled",
        )),
    }
}

fn codec_wire(codec: AudioCodec) -> u8 {
    match codec {
        AudioCodec::Default => 0,
        AudioCodec::Lhdc => 1,
        AudioCodec::Ldac => 2,
    }
}

fn side_wire(side: EarbudSide) -> u8 {
    match side {
        EarbudSide::Left => 2,
        EarbudSide::Right => 3,
    }
}
fn gesture_wire(gesture: Gesture) -> u8 {
    match gesture {
        Gesture::SinglePinch => 1,
        Gesture::DoublePinch => 2,
        Gesture::TriplePinch => 3,
        Gesture::PinchAndHold => 7,
        Gesture::DoublePinchAndHold => 9,
    }
}
fn action_wire(action: GestureAction) -> u8 {
    match action {
        GestureAction::PlayPause => 2,
        GestureAction::SkipBack => 8,
        GestureAction::SkipForward => 9,
        GestureAction::VoiceAssistant => 11,
        GestureAction::NoiseControl => 10,
        GestureAction::VolumeUp => 18,
        GestureAction::VolumeDown => 19,
        GestureAction::None => 1,
    }
}

fn encode_custom_eq(gains: [f32; 3]) -> Result<Vec<u8>, ProtocolError> {
    if gains
        .iter()
        .any(|gain| !gain.is_finite() || !(-6.0..=6.0).contains(gain))
    {
        return Err(ProtocolError::InvalidValue("custom EQ gain"));
    }
    let mut payload = vec![
        3, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0x75, 0x44, 0xc3, 0xf5, 0x28, 0x3f, 2, 0, 0, 0, 0, 0,
        0xc0, 0x5a, 0x45, 0, 0, 0x80, 0x3f, 0, 0, 0, 0, 0, 0, 0, 0x0c, 0x43, 0xcd, 0xcc, 0x4c,
        0x3f, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ];
    let headroom = gains.into_iter().fold(0.0_f32, f32::max);
    payload[1..5].copy_from_slice(&(-headroom).to_le_bytes());
    for (index, gain) in gains.into_iter().enumerate() {
        let offset = 6 + index * 13;
        payload[offset..offset + 4].copy_from_slice(&gain.to_le_bytes());
    }
    Ok(payload)
}

fn encode_advanced_eq(profile: &AdvancedEqProfile) -> Result<Vec<u8>, ProtocolError> {
    profile.validate()?;
    let headroom = profile
        .bands
        .iter()
        .map(|band| band.gain_db)
        .fold(0.0_f32, f32::max);
    let mut payload = Vec::with_capacity(6 + profile.bands.len() * 13);
    payload.push(0);
    payload.push(profile.bands.len() as u8);
    payload.extend_from_slice(&(-headroom).to_le_bytes());
    for band in &profile.bands {
        payload.push(1);
        payload.extend_from_slice(&band.gain_db.to_le_bytes());
        payload.extend_from_slice(&band.frequency_hz.to_le_bytes());
        payload.extend_from_slice(&band.q.to_le_bytes());
    }
    Ok(payload)
}

pub fn decode_event(frame: &Frame) -> Result<Option<DeviceEvent>, ProtocolError> {
    let id = if frame.command & 0xe000 == 0xe000 {
        frame.command
    } else {
        frame.command | 0x8000
    };
    let payload = frame.payload.as_slice();
    let event = match id {
        command::QUERY_BATTERY | command::EVENT_BATTERY => {
            DeviceEvent::Battery(parse_battery(payload)?)
        }
        command::QUERY_WEAR | command::EVENT_WEAR => DeviceEvent::Wear(parse_wear(payload)?),
        command::QUERY_ANC | command::EVENT_ANC => {
            let (mode, level) = parse_anc(payload)?;
            DeviceEvent::Anc { mode, level }
        }
        command::QUERY_EQ => DeviceEvent::Eq(parse_eq(payload)?),
        command::QUERY_CUSTOM_EQ => DeviceEvent::CustomEq(parse_custom_eq(payload)?),
        command::QUERY_ADVANCED_EQ => DeviceEvent::AdvancedEqEnabled(parse_bool(payload, id)?),
        command::QUERY_ADVANCED_EQ_PROFILE => {
            DeviceEvent::AdvancedEqProfile(parse_advanced_eq(payload)?)
        }
        command::QUERY_FIRMWARE => DeviceEvent::Firmware(parse_string(payload, id)?),
        command::QUERY_GESTURES => DeviceEvent::Gestures(parse_gestures(payload)?),
        command::QUERY_IN_EAR => DeviceEvent::InEarDetection(
            *payload.get(2).ok_or(ProtocolError::MalformedResponse(id))? != 0,
        ),
        command::QUERY_LOW_LAG => DeviceEvent::LowLag(
            *payload
                .first()
                .ok_or(ProtocolError::MalformedResponse(id))?
                == 1,
        ),
        command::QUERY_BASS => {
            let enabled = *payload
                .first()
                .ok_or(ProtocolError::MalformedResponse(id))?
                != 0;
            let level = *payload.get(1).ok_or(ProtocolError::MalformedResponse(id))? / 2;
            DeviceEvent::BassEnhance(enabled.then_some(level))
        }
        command::QUERY_AUDIO_CODEC => DeviceEvent::AudioCodec(parse_audio_codec(payload, id)?),
        command::QUERY_DUAL_CONNECTION => DeviceEvent::DualConnection(parse_bool(payload, id)?),
        command::EVENT_FIT_TEST => DeviceEvent::FitTestResult {
            left_ok: *payload
                .first()
                .ok_or(ProtocolError::MalformedResponse(id))?
                == 1,
            right_ok: *payload.get(1).ok_or(ProtocolError::MalformedResponse(id))? == 1,
        },
        command::ACTIVATE
        | command::SET_ANC
        | command::SET_EQ
        | command::SET_GESTURE
        | command::SET_IN_EAR
        | command::SET_LOW_LAG
        | command::SET_CUSTOM_EQ
        | command::SET_ADVANCED_EQ
        | command::SET_ADVANCED_EQ_PROFILE
        | command::SET_AUDIO_CODEC
        | command::SET_BASS
        | command::SET_DUAL_CONNECTION
        | command::FIND_BUD
        | command::START_FIT_TEST => DeviceEvent::Acknowledged {
            sequence: frame.sequence,
            command: id,
        },
        _ => return Ok(None),
    };
    Ok(Some(event))
}

fn parse_bool(payload: &[u8], command_id: u16) -> Result<bool, ProtocolError> {
    Ok(*payload
        .first()
        .ok_or(ProtocolError::MalformedResponse(command_id))?
        != 0)
}

fn parse_audio_codec(payload: &[u8], command_id: u16) -> Result<AudioCodec, ProtocolError> {
    match payload
        .first()
        .ok_or(ProtocolError::MalformedResponse(command_id))?
    {
        0 => Ok(AudioCodec::Default),
        1 => Ok(AudioCodec::Lhdc),
        2 => Ok(AudioCodec::Ldac),
        _ => Err(ProtocolError::InvalidValue("audio codec")),
    }
}

fn parse_battery(payload: &[u8]) -> Result<BatteryState, ProtocolError> {
    let count = usize::from(
        *payload
            .first()
            .ok_or(ProtocolError::MalformedResponse(command::QUERY_BATTERY))?,
    );
    if payload.len() < 1 + count * 2 {
        return Err(ProtocolError::MalformedResponse(command::QUERY_BATTERY));
    }
    let mut state = BatteryState::default();
    for pair in payload[1..1 + count * 2].chunks_exact(2) {
        let level = ChargeLevel::new(pair[1] & 0x7f, pair[1] & 0x80 != 0)?;
        match pair[0] {
            2 => state.left = Some(level),
            3 => state.right = Some(level),
            4 => state.case = Some(level),
            _ => {}
        }
    }
    Ok(state)
}

fn parse_wear(payload: &[u8]) -> Result<WearState, ProtocolError> {
    let count = usize::from(
        *payload
            .first()
            .ok_or(ProtocolError::MalformedResponse(command::QUERY_WEAR))?,
    );
    if payload.len() < 1 + count * 2 {
        return Err(ProtocolError::MalformedResponse(command::QUERY_WEAR));
    }
    let mut wear = WearState::default();
    for pair in payload[1..1 + count * 2].chunks_exact(2) {
        match pair[0] {
            2 => wear.left = pair[1] & 4 != 0,
            3 => wear.right = pair[1] & 4 != 0,
            _ => {}
        }
    }
    Ok(wear)
}

fn parse_anc(payload: &[u8]) -> Result<(AncMode, AncLevel), ProtocolError> {
    if payload.len() < 2 {
        return Err(ProtocolError::MalformedResponse(command::QUERY_ANC));
    }
    let mut mode = AncMode::Off;
    let mut level = AncLevel::High;
    for triplet in payload.chunks(3) {
        if triplet.len() < 2 {
            continue;
        }
        if triplet[0] == 1 {
            match triplet[1] {
                5 | 0 => mode = AncMode::Off,
                7 => mode = AncMode::Transparency,
                1 => {
                    mode = AncMode::NoiseCancellation;
                    level = AncLevel::High;
                }
                2 => {
                    mode = AncMode::NoiseCancellation;
                    level = AncLevel::Mid;
                }
                3 => {
                    mode = AncMode::NoiseCancellation;
                    level = AncLevel::Low;
                }
                4 => {
                    mode = AncMode::NoiseCancellation;
                    level = AncLevel::Adaptive;
                }
                _ => return Err(ProtocolError::InvalidValue("ANC mode")),
            }
        }
    }
    Ok((mode, level))
}

fn parse_eq(payload: &[u8]) -> Result<EqPreset, ProtocolError> {
    match payload.first() {
        Some(0) => Ok(EqPreset::Balanced),
        Some(1) => Ok(EqPreset::Voice),
        Some(2) => Ok(EqPreset::MoreTreble),
        Some(3) => Ok(EqPreset::MoreBass),
        Some(5) => Ok(EqPreset::Custom),
        _ => Err(ProtocolError::MalformedResponse(command::QUERY_EQ)),
    }
}

fn parse_custom_eq(payload: &[u8]) -> Result<[f32; 3], ProtocolError> {
    if payload.len() < 36 {
        return Err(ProtocolError::MalformedResponse(command::QUERY_CUSTOM_EQ));
    }
    let mut values = [0.0; 3];
    for (index, value) in values.iter_mut().enumerate() {
        let offset = 6 + index * 13;
        *value = f32::from_le_bytes(
            payload[offset..offset + 4]
                .try_into()
                .map_err(|_| ProtocolError::MalformedResponse(command::QUERY_CUSTOM_EQ))?,
        );
    }
    Ok(values)
}

fn parse_advanced_eq(payload: &[u8]) -> Result<AdvancedEqProfile, ProtocolError> {
    if payload.len() < 6 {
        return Err(ProtocolError::MalformedResponse(
            command::QUERY_ADVANCED_EQ_PROFILE,
        ));
    }
    let count = usize::from(payload[1]);
    if count != AdvancedEqProfile::FREQUENCIES.len() || payload.len() < 6 + count * 13 {
        return Err(ProtocolError::MalformedResponse(
            command::QUERY_ADVANCED_EQ_PROFILE,
        ));
    }
    let mut bands = Vec::with_capacity(count);
    for index in 0..count {
        let offset = 6 + index * 13;
        let gain_db =
            f32::from_le_bytes(payload[offset + 1..offset + 5].try_into().map_err(|_| {
                ProtocolError::MalformedResponse(command::QUERY_ADVANCED_EQ_PROFILE)
            })?);
        let frequency_hz =
            f32::from_le_bytes(payload[offset + 5..offset + 9].try_into().map_err(|_| {
                ProtocolError::MalformedResponse(command::QUERY_ADVANCED_EQ_PROFILE)
            })?);
        let q =
            f32::from_le_bytes(payload[offset + 9..offset + 13].try_into().map_err(|_| {
                ProtocolError::MalformedResponse(command::QUERY_ADVANCED_EQ_PROFILE)
            })?);
        bands.push(EqBand {
            frequency_hz,
            gain_db,
            q,
        });
    }
    let profile = AdvancedEqProfile {
        name: "Device profile".into(),
        bands,
    };
    profile.validate()?;
    Ok(profile)
}

fn parse_string(payload: &[u8], command_id: u16) -> Result<String, ProtocolError> {
    let value = String::from_utf8_lossy(payload)
        .trim_matches(char::from(0))
        .trim()
        .to_owned();
    if value.is_empty() {
        Err(ProtocolError::MalformedResponse(command_id))
    } else {
        Ok(value)
    }
}

fn parse_gestures(
    payload: &[u8],
) -> Result<BTreeMap<(EarbudSide, Gesture), GestureAction>, ProtocolError> {
    let count = usize::from(
        *payload
            .first()
            .ok_or(ProtocolError::MalformedResponse(command::QUERY_GESTURES))?,
    );
    if payload.len() < 1 + count * 4 {
        return Err(ProtocolError::MalformedResponse(command::QUERY_GESTURES));
    }
    let mut map = BTreeMap::new();
    for item in payload[1..1 + count * 4].chunks_exact(4) {
        let side = match item[0] {
            2 => EarbudSide::Left,
            3 => EarbudSide::Right,
            _ => continue,
        };
        let gesture = match item[2] {
            1 => Gesture::SinglePinch,
            2 => Gesture::DoublePinch,
            3 => Gesture::TriplePinch,
            7 => Gesture::PinchAndHold,
            9 => Gesture::DoublePinchAndHold,
            _ => continue,
        };
        let action = match item[3] {
            2 => GestureAction::PlayPause,
            8 => GestureAction::SkipBack,
            9 => GestureAction::SkipForward,
            10 | 20 | 21 | 22 => GestureAction::NoiseControl,
            11 => GestureAction::VoiceAssistant,
            18 => GestureAction::VolumeUp,
            19 => GestureAction::VolumeDown,
            _ => GestureAction::None,
        };
        map.insert((side, gesture), action);
    }
    Ok(map)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_verified_b171_payloads_encode() {
        let commands = [
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
            DeviceCommand::SetAnc {
                mode: AncMode::NoiseCancellation,
                level: AncLevel::Adaptive,
            },
            DeviceCommand::SetEqPreset(EqPreset::Voice),
            DeviceCommand::SetCustomEq([-2.0, 0.0, 3.0]),
            DeviceCommand::SetAdvancedEqEnabled(true),
            DeviceCommand::SetBassEnhance(Some(5)),
            DeviceCommand::SetInEarDetection(true),
            DeviceCommand::SetLowLag(false),
            DeviceCommand::SetAudioCodec(AudioCodec::Ldac),
            DeviceCommand::SetDualConnection(true),
            DeviceCommand::FindBud {
                side: EarbudSide::Left,
                ringing: true,
            },
            DeviceCommand::StartFitTest,
        ];
        for (index, command) in commands.iter().enumerate() {
            assert!(encode_command(command, index as u8).is_ok(), "{command:?}");
        }
    }

    #[test]
    fn parses_battery_and_gestures() {
        let battery =
            Frame::new(0x4007, 1, vec![3, 2, 80, 3, 0x94, 4, 60]).unwrap_or_else(|e| panic!("{e}"));
        let Some(DeviceEvent::Battery(state)) =
            decode_event(&battery).unwrap_or_else(|e| panic!("{e}"))
        else {
            panic!("wrong event")
        };
        assert_eq!(state.left.map(|v| v.percent), Some(80));
        assert_eq!(state.right.map(|v| v.charging), Some(true));
        let gestures = Frame::new(0x4018, 2, vec![2, 2, 1, 2, 9, 3, 1, 7, 18])
            .unwrap_or_else(|e| panic!("{e}"));
        let Some(DeviceEvent::Gestures(map)) =
            decode_event(&gestures).unwrap_or_else(|e| panic!("{e}"))
        else {
            panic!("wrong event")
        };
        assert_eq!(map.len(), 2);
    }

    #[test]
    fn parses_live_wear_event() {
        let wear =
            Frame::new(0xe002, 3, vec![2, 2, 0x80, 3, 0x84]).unwrap_or_else(|e| panic!("{e}"));
        let Some(DeviceEvent::Wear(state)) = decode_event(&wear).unwrap_or_else(|e| panic!("{e}"))
        else {
            panic!("wrong event")
        };
        assert!(!state.left);
        assert!(state.right);
    }

    #[test]
    fn encodes_high_quality_and_dual_connection_controls() {
        let high_quality = encode_command(&DeviceCommand::SetAudioCodec(AudioCodec::Ldac), 7)
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(high_quality.command, command::SET_AUDIO_CODEC);
        assert_eq!(high_quality.payload, [2]);

        let dual_connection = encode_command(&DeviceCommand::SetDualConnection(false), 8)
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(dual_connection.command, command::SET_DUAL_CONNECTION);
        assert_eq!(dual_connection.payload, [0]);
    }

    #[test]
    fn encodes_fixed_eq_payloads_from_b171_app() {
        let more_bass = encode_command(&DeviceCommand::SetEqPreset(EqPreset::MoreBass), 1)
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(more_bass.payload, [3]);

        let custom = encode_command(&DeviceCommand::SetEqPreset(EqPreset::Custom), 2)
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(custom.payload, [5]);

        let advanced = encode_command(&DeviceCommand::SetAdvancedEqEnabled(true), 3)
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(advanced.payload, [1]);
    }

    #[test]
    fn sequence_rollover_is_caller_safe() {
        let mut seq = 255_u8;
        seq = seq.wrapping_add(1);
        assert_eq!(
            encode_command(&DeviceCommand::QueryBattery, seq)
                .unwrap_or_else(|e| panic!("{e}"))
                .sequence,
            0
        );
    }
}
