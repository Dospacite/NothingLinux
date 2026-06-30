//! Bounded, safe codec for the independently implemented Nothing Ear RFCOMM protocol.

mod commands;
mod frame;
mod types;

pub use commands::{command, decode_event, encode_command};
pub use frame::{CRC_FLAG, Frame, FrameDecoder, HOST_CONTROL, MAX_PAYLOAD, START_BYTE, crc16_arc};
pub use types::*;
