use crate::ProtocolError;

pub const START_BYTE: u8 = 0x55;
pub const HOST_CONTROL: u16 = 0x0160;
pub const CRC_FLAG: u16 = 0x0020;
pub const MAX_PAYLOAD: usize = 4096;
const HEADER_LEN: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub control: u16,
    pub command: u16,
    pub sequence: u8,
    pub payload: Vec<u8>,
}

impl Frame {
    pub fn new(command: u16, sequence: u8, payload: Vec<u8>) -> Result<Self, ProtocolError> {
        if payload.len() > MAX_PAYLOAD {
            return Err(ProtocolError::FrameTooLarge(payload.len()));
        }
        Ok(Self {
            control: HOST_CONTROL,
            command,
            sequence,
            payload,
        })
    }

    pub fn encode(&self) -> Result<Vec<u8>, ProtocolError> {
        if self.payload.len() > MAX_PAYLOAD {
            return Err(ProtocolError::FrameTooLarge(self.payload.len()));
        }
        let length = u16::try_from(self.payload.len())
            .map_err(|_| ProtocolError::FrameTooLarge(self.payload.len()))?;
        let crc_len = usize::from(self.control & CRC_FLAG != 0) * 2;
        let mut bytes = Vec::with_capacity(HEADER_LEN + self.payload.len() + crc_len);
        bytes.push(START_BYTE);
        bytes.extend_from_slice(&self.control.to_le_bytes());
        bytes.extend_from_slice(&self.command.to_le_bytes());
        bytes.extend_from_slice(&length.to_le_bytes());
        bytes.push(self.sequence);
        bytes.extend_from_slice(&self.payload);
        if crc_len != 0 {
            bytes.extend_from_slice(&crc16_arc(&bytes).to_le_bytes());
        }
        Ok(bytes)
    }
}

#[must_use]
pub fn crc16_arc(data: &[u8]) -> u16 {
    let mut crc = 0xffff_u16;
    for byte in data {
        crc ^= u16::from(*byte);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xa001
            } else {
                crc >> 1
            };
        }
    }
    crc
}

#[derive(Debug, Default)]
pub struct FrameDecoder {
    buffer: Vec<u8>,
}

impl FrameDecoder {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, bytes: &[u8]) -> Result<Vec<Frame>, ProtocolError> {
        self.buffer.extend_from_slice(bytes);
        let mut frames = Vec::new();
        loop {
            let Some(start) = self.buffer.iter().position(|byte| *byte == START_BYTE) else {
                self.buffer.clear();
                break;
            };
            if start != 0 {
                self.buffer.drain(..start);
            }
            if self.buffer.len() < HEADER_LEN {
                break;
            }
            let control = u16::from_le_bytes([self.buffer[1], self.buffer[2]]);
            let command = u16::from_le_bytes([self.buffer[3], self.buffer[4]]);
            let length = usize::from(u16::from_le_bytes([self.buffer[5], self.buffer[6]]));
            if length > MAX_PAYLOAD {
                self.buffer.remove(0);
                return Err(ProtocolError::FrameTooLarge(length));
            }
            let crc_len = usize::from(control & CRC_FLAG != 0) * 2;
            let total = HEADER_LEN
                .checked_add(length)
                .and_then(|v| v.checked_add(crc_len))
                .ok_or(ProtocolError::MalformedLength)?;
            if self.buffer.len() < total {
                break;
            }
            if crc_len != 0 {
                let received = u16::from_le_bytes([
                    self.buffer[HEADER_LEN + length],
                    self.buffer[HEADER_LEN + length + 1],
                ]);
                let expected = crc16_arc(&self.buffer[..HEADER_LEN + length]);
                if received != expected {
                    self.buffer.drain(..total);
                    return Err(ProtocolError::BadCrc { received, expected });
                }
            }
            frames.push(Frame {
                control,
                command,
                sequence: self.buffer[7],
                payload: self.buffer[HEADER_LEN..HEADER_LEN + length].to_vec(),
            });
            self.buffer.drain(..total);
        }
        Ok(frames)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_crc_vector() {
        assert_eq!(crc16_arc(b"123456789"), 0x4b37);
    }

    #[test]
    fn exact_empty_frame_vector() {
        let frame = Frame::new(0xc001, 1, vec![]).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            frame.encode().unwrap_or_else(|e| panic!("{e}")),
            vec![0x55, 0x60, 0x01, 0x01, 0xc0, 0, 0, 1, 0x24, 0xdf]
        );
    }

    #[test]
    fn partial_and_multiple_frames() {
        let first = Frame::new(0xc007, 254, vec![])
            .unwrap_or_else(|e| panic!("{e}"))
            .encode()
            .unwrap_or_else(|e| panic!("{e}"));
        let second = Frame::new(0xc01f, 255, vec![1])
            .unwrap_or_else(|e| panic!("{e}"))
            .encode()
            .unwrap_or_else(|e| panic!("{e}"));
        let mut decoder = FrameDecoder::new();
        assert!(
            decoder
                .push(&first[..3])
                .unwrap_or_else(|e| panic!("{e}"))
                .is_empty()
        );
        let mut remaining = first[3..].to_vec();
        remaining.extend(second);
        let decoded = decoder.push(&remaining).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(decoded.len(), 2);
        assert_eq!(decoded[1].sequence, 255);
    }

    #[test]
    fn rejects_crc_and_oversized_lengths() {
        let mut bad = Frame::new(0xc007, 1, vec![1])
            .unwrap_or_else(|e| panic!("{e}"))
            .encode()
            .unwrap_or_else(|e| panic!("{e}"));
        let last = bad.len() - 1;
        bad[last] ^= 0xff;
        assert!(matches!(
            FrameDecoder::new().push(&bad),
            Err(ProtocolError::BadCrc { .. })
        ));
        let header = [START_BYTE, 0x60, 1, 7, 0xc0, 1, 0x10, 1];
        assert_eq!(
            FrameDecoder::new().push(&header),
            Err(ProtocolError::FrameTooLarge(4097))
        );
    }
}
