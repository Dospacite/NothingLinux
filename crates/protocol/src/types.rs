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
        20.0, 63.0, 250.0, 1_000.0, 4_000.0, 8_000.0, 12_000.0, 20_000.0,
    ];
    pub fn validate(&self) -> Result<(), ProtocolError> {
        if self.name.trim().is_empty() || self.name.chars().count() > 48 {
            return Err(ProtocolError::InvalidValue("profile name"));
        }
        if self.bands.len() != 8 {
            return Err(ProtocolError::InvalidValue("advanced EQ band count"));
        }
        for (index, band) in self.bands.iter().enumerate() {
            if !band.frequency_hz.is_finite()
                || !band.gain_db.is_finite()
                || !band.q.is_finite()
                || !(20.0..=20_000.0).contains(&band.frequency_hz)
                || !(-12.0..=12.0).contains(&band.gain_db)
                || !(0.1..=10.0).contains(&band.q)
                || (band.frequency_hz - Self::FREQUENCIES[index]).abs() > f32::EPSILON
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
    pub gestures: BTreeMap<(EarbudSide, Gesture), GestureAction>,
    pub bass_enhance: Option<u8>,
    pub in_ear_detection: Option<bool>,
    pub low_lag: Option<bool>,
    pub high_quality_audio: Option<bool>,
    pub dual_connection: Option<bool>,
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
            gestures: BTreeMap::new(),
            bass_enhance: None,
            in_ear_detection: None,
            low_lag: None,
            high_quality_audio: None,
            dual_connection: None,
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
    QueryFirmware,
    QueryGestures,
    QueryInEarDetection,
    QueryLowLag,
    QueryBassEnhance,
    QueryAdvancedEq,
    QueryHighQualityAudio,
    QueryDualConnection,
    SetAnc {
        mode: AncMode,
        level: AncLevel,
    },
    SetEqPreset(EqPreset),
    SetCustomEq([f32; 3]),
    SetAdvancedEqEnabled(bool),
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
    SetHighQualityAudio(bool),
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
    Gestures(BTreeMap<(EarbudSide, Gesture), GestureAction>),
    BassEnhance(Option<u8>),
    InEarDetection(bool),
    LowLag(bool),
    HighQualityAudio(bool),
    DualConnection(bool),
    Firmware(String),
    FitTestResult {
        left_ok: bool,
        right_ok: bool,
    },
    Acknowledged {
        sequence: u8,
        command: u16,
    },
    CommandFailed {
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
