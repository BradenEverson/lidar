//! Data Response Format

use crate::rplidar::packet::OpCode;

pub enum DataResponse {
    ScanResponse(ScanResponse),
}

impl DataResponse {
    pub fn try_from_packet(sent_from: OpCode, bytes: &[u8]) -> Option<Self> {
        match sent_from {
            OpCode::Scan => ScanResponse::try_from_bytes(bytes).map(ScanResponse::wrap),
            _ => todo!("Implement other data packet parsing modes"),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Default, Debug)]
pub struct ScanResponse {
    pub new: bool,
    pub quality: u8,
    pub angle: f32,
    pub dist: f32,
}

impl ScanResponse {
    pub fn try_from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() != 5 {
            return None;
        }

        let s = bytes[0] & 1;
        let s_bar = (bytes[0] & 2) >> 1;

        if s == s_bar {
            return None;
        }

        let new = if s == 1 { true } else { false };

        let quality = bytes[0] >> 2;

        let c = bytes[1] & 1;

        if c != 1 {
            return None;
        }

        let mut angle = u16::from_le_bytes([bytes[1], bytes[2]]);
        angle >>= 1;

        let angle = angle as f32 / 64.0;

        let dist = u16::from_le_bytes([bytes[3], bytes[4]]);
        let dist = dist as f32 / 4.0;

        Some(Self {
            new,
            quality,
            angle,
            dist,
        })
    }

    pub fn wrap(self) -> DataResponse {
        DataResponse::ScanResponse(self)
    }
}
