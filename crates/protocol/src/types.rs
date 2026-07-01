use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceDescriptor {
    pub address: String,
    pub alias: String,
    pub model: Option<String>,
    pub paired: bool,
    pub connected: bool,
    pub vendor_service: bool,
}

impl DeviceDescriptor {
    pub fn redacted_address(&self) -> String {
        let suffix = self.address.rsplit(':').next().unwrap_or("??");
        format!("XX:XX:XX:XX:XX:{suffix}")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DualConnectionDevice {
    pub address: String,
    pub address_bytes: [u8; 6],
    pub name: String,
    pub connected: bool,
    pub owner_device: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceCapabilities {
    pub model: String,
    pub anc: bool,
    pub adaptive_anc: bool,
    pub personalized_anc: bool,
    pub eq_presets: bool,
    pub custom_eq: bool,
    pub advanced_eq: bool,
    pub gestures: bool,
    pub bass_enhance: bool,
    pub in_ear_detection: bool,
    pub low_lag: bool,
    pub high_quality_audio: bool,
    pub dual_connection: bool,
    pub find_buds: bool,
    pub fit_test: bool,
}

impl DeviceCapabilities {
    #[must_use]
    pub fn b171() -> Self {
        Self {
            model: "B171".into(),
            anc: true,
            adaptive_anc: true,
            personalized_anc: false,
            eq_presets: true,
            custom_eq: true,
            advanced_eq: true,
            gestures: true,
            bass_enhance: true,
            in_ear_detection: true,
            low_lag: true,
            high_quality_audio: true,
            dual_connection: true,
            find_buds: true,
            fit_test: true,
        }
    }

    #[must_use]
    pub fn unknown(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            anc: false,
            adaptive_anc: false,
            personalized_anc: false,
            eq_presets: false,
            custom_eq: false,
            advanced_eq: false,
            gestures: false,
            bass_enhance: false,
            in_ear_detection: false,
            low_lag: false,
            high_quality_audio: false,
            dual_connection: false,
            find_buds: false,
            fit_test: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ConnectionState {
    #[default]
    Disconnected,
    Connecting,
    Activating,
    Syncing,
    Ready,
    Recovering,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChargeLevel {
    pub percent: u8,
    pub charging: bool,
}

impl ChargeLevel {
    pub fn new(percent: u8, charging: bool) -> Result<Self, ProtocolError> {
        if percent > 100 {
            return Err(ProtocolError::InvalidValue("battery percentage"));
        }
        Ok(Self { percent, charging })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct BatteryState {
    pub left: Option<ChargeLevel>,
    pub right: Option<ChargeLevel>,
    pub case: Option<ChargeLevel>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct WearState {
    pub left: bool,
    pub right: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AncMode {
    #[default]
    Off,
    Transparency,
    NoiseCancellation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AncLevel {
    #[default]
    High,
    Mid,
    Low,
    Adaptive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum EqPreset {
    #[default]
    Balanced,
    MoreBass,
    MoreTreble,
    Voice,
    Custom,
    Advanced,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AudioCodec {
    #[default]
    Default,
    Lhdc,
    Ldac,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EqBand {
    pub frequency_hz: f32,
    pub gain_db: f32,
    pub q: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdvancedEqProfile {
    pub name: String,
    pub bands: Vec<EqBand>,
}

impl AdvancedEqProfile {
    pub const FREQUENCIES: [f32; 8] = [
        55.0, 110.0, 220.0, 440.0, 1_320.0, 3_300.0, 6_600.0, 13_200.0,
    ];
    pub const FREQUENCY_RANGES: [(f32, f32); 8] = [
        (20.0, 99.0),
        (100.0, 199.0),
        (200.0, 399.0),
        (400.0, 999.0),
        (1_000.0, 2_999.0),
        (3_000.0, 5_999.0),
        (6_000.0, 11_999.0),
        (12_000.0, 20_000.0),
    ];
    pub fn validate(&self) -> Result<(), ProtocolError> {
        if self.name.trim().is_empty() || self.name.chars().count() > 48 {
            return Err(ProtocolError::InvalidValue("profile name"));
        }
        if self.bands.len() != 8 {
            return Err(ProtocolError::InvalidValue("advanced EQ band count"));
        }
        for (index, band) in self.bands.iter().enumerate() {
            let (min_frequency, max_frequency) = Self::FREQUENCY_RANGES[index];
            if !band.frequency_hz.is_finite()
                || !band.gain_db.is_finite()
                || !band.q.is_finite()
                || !(min_frequency..=max_frequency).contains(&band.frequency_hz)
                || !(-12.0..=12.0).contains(&band.gain_db)
                || !(0.1..=10.0).contains(&band.q)
            {
                return Err(ProtocolError::InvalidValue("advanced EQ band"));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum EarbudSide {
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Gesture {
    SinglePinch,
    DoublePinch,
    TriplePinch,
    PinchAndHold,
    DoublePinchAndHold,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GestureAction {
    PlayPause,
    SkipBack,
    SkipForward,
    VoiceAssistant,
    NoiseControl,
    VolumeUp,
    VolumeDown,
    None,
}

impl Gesture {
    #[must_use]
    pub fn allows(self, action: GestureAction) -> bool {
        use GestureAction as A;
        match self {
            Self::SinglePinch => action == A::PlayPause,
            Self::DoublePinch | Self::TriplePinch => matches!(
                action,
                A::SkipBack | A::SkipForward | A::VoiceAssistant | A::None
            ),
            Self::PinchAndHold | Self::DoublePinchAndHold => matches!(
                action,
                A::NoiseControl | A::VolumeUp | A::VolumeDown | A::VoiceAssistant | A::None
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceSnapshot {
    pub connection: ConnectionState,
    pub battery: BatteryState,
    pub wear: WearState,
    pub anc_mode: AncMode,
    pub anc_level: AncLevel,
    pub eq_preset: EqPreset,
    pub custom_eq: [f32; 3],
    pub advanced_eq_enabled: Option<bool>,
    pub advanced_eq_profile: Option<AdvancedEqProfile>,
    pub gestures: BTreeMap<(EarbudSide, Gesture), GestureAction>,
    pub bass_enhance: Option<u8>,
    pub in_ear_detection: Option<bool>,
    pub low_lag: Option<bool>,
    pub audio_codec: Option<AudioCodec>,
    pub dual_connection: Option<bool>,
    pub dual_devices: Vec<DualConnectionDevice>,
    pub firmware: Option<String>,
    pub model: Option<String>,
}

impl Default for DeviceSnapshot {
    fn default() -> Self {
        Self {
            connection: ConnectionState::Disconnected,
            battery: BatteryState::default(),
            wear: WearState::default(),
            anc_mode: AncMode::Off,
            anc_level: AncLevel::High,
            eq_preset: EqPreset::Balanced,
            custom_eq: [0.0; 3],
            advanced_eq_enabled: None,
            advanced_eq_profile: None,
            gestures: BTreeMap::new(),
            bass_enhance: None,
            in_ear_detection: None,
            low_lag: None,
            audio_codec: None,
            dual_connection: None,
            dual_devices: Vec::new(),
            firmware: None,
            model: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DeviceCommand {
    Activate,
    QueryBattery,
    QueryWear,
    QueryAnc,
    QueryEq,
    QueryCustomEq,
    QueryAdvancedEqProfile,
    QueryFirmware,
    QueryGestures,
    QueryInEarDetection,
    QueryLowLag,
    QueryBassEnhance,
    QueryAdvancedEq,
    QueryAudioCodec,
    QueryDualConnection,
    QueryDualConnectionDevices {
        page: u16,
    },
    SetAnc {
        mode: AncMode,
        level: AncLevel,
    },
    SetEqPreset(EqPreset),
    SetCustomEq([f32; 3]),
    SetAdvancedEqEnabled(bool),
    SetAdvancedEqProfile(Box<AdvancedEqProfile>),
    SetGesture {
        side: EarbudSide,
        gesture: Gesture,
        action: GestureAction,
    },
    SetBassEnhance(Option<u8>),
    SetInEarDetection(bool),
    SetLowLag(bool),
    FindBud {
        side: EarbudSide,
        ringing: bool,
    },
    StartFitTest,
    CancelFitTest,
    SetDualConnection(bool),
    SetDualConnectionDevice {
        connect: bool,
        address: [u8; 6],
    },
    SetAudioCodec(AudioCodec),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DeviceEvent {
    ConnectionChanged(ConnectionState),
    Snapshot(DeviceSnapshot),
    Battery(BatteryState),
    Wear(WearState),
    Anc {
        mode: AncMode,
        level: AncLevel,
    },
    Eq(EqPreset),
    CustomEq([f32; 3]),
    AdvancedEqEnabled(bool),
    AdvancedEqProfile(AdvancedEqProfile),
    Gestures(BTreeMap<(EarbudSide, Gesture), GestureAction>),
    BassEnhance(Option<u8>),
    InEarDetection(bool),
    LowLag(bool),
    AudioCodec(AudioCodec),
    DualConnection(bool),
    DualConnectionDevicePage {
        total: u8,
        current: u8,
        devices: Vec<DualConnectionDevice>,
    },
    DualConnectionDevices(Vec<DualConnectionDevice>),
    DualConnectionDeviceChanged {
        connected: bool,
        address: String,
        need_update: bool,
    },
    DualConnectionSwitchChanged,
    Firmware(String),
    FitTestResult {
        left_ok: bool,
        right_ok: bool,
    },
    Acknowledged {
        sequence: u8,
        command: u16,
    },
    CommandStarted {
        sequence: u8,
        command: DeviceCommand,
    },
    CommandConfirmed {
        sequence: u8,
        command: DeviceCommand,
    },
    CommandFailed {
        sequence: Option<u8>,
        command: DeviceCommand,
        reason: String,
    },
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ProtocolError {
    #[error("payload exceeds the {0}-byte safety bound")]
    FrameTooLarge(usize),
    #[error("declared frame length is malformed")]
    MalformedLength,
    #[error("frame checksum mismatch: received {received:#06x}, expected {expected:#06x}")]
    BadCrc { received: u16, expected: u16 },
    #[error("unsupported command: {0}")]
    UnsupportedCommand(&'static str),
    #[error("invalid {0}")]
    InvalidValue(&'static str),
    #[error("malformed response for command {0:#06x}")]
    MalformedResponse(u16),
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn b171_gesture_allowlist_rejects_invalid_combinations() {
        assert!(Gesture::DoublePinch.allows(GestureAction::SkipForward));
        assert!(!Gesture::DoublePinch.allows(GestureAction::VolumeUp));
        assert!(Gesture::SinglePinch.allows(GestureAction::PlayPause));
        assert!(!Gesture::SinglePinch.allows(GestureAction::None));
    }

    #[test]
    fn validates_advanced_eq() {
        let profile = AdvancedEqProfile {
            name: "Desk".into(),
            bands: AdvancedEqProfile::FREQUENCIES
                .into_iter()
                .map(|frequency_hz| EqBand {
                    frequency_hz,
                    gain_db: 0.0,
                    q: 1.0,
                })
                .collect(),
        };
        assert_eq!(profile.validate(), Ok(()));
    }
}
