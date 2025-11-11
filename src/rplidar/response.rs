//! Data Response Format

use std::mem::MaybeUninit;

use crate::rplidar::packet::OpCode;

pub enum DataResponse {
    ScanResponse(ScanResponse),
    UltraCapsuleResponse(UltraCapsuleResponse),
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

#[derive(Clone, Copy, PartialEq, Default, Debug)]
pub struct Scan {
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

        let new = s == 1;

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

pub struct UltraCapsuleResponse {
    pub s: u8,
    pub start_angle_q6: u16,
    pub ultra_cabins: [u32; 32],
}

impl UltraCapsuleResponse {
    pub fn try_from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() != 132 {
            return None;
        }

        let f1 = bytes[0] & 0xF0;
        let f2 = (bytes[1] & 0xF0) >> 4;

        if f1 | f2 != 0xA5 {
            return None;
        }

        // todo: Checksum
        // let c1 = bytes[0] & 0x0F;
        // let c2 = bytes[1] & 0x0F;

        let angle_l = bytes[2] as u16;
        let angle_h = (bytes[3] & 0x7F) as u16;

        let s = bytes[3] & 0x80 >> 7;

        let start_angle_q6 = angle_h << 4 | angle_l;

        let (cabins, extra) = bytes[4..].as_chunks::<128>();

        if extra.len() != 0 || cabins.len() != 1 {
            return None;
        }

        let cabin_bytes: [u8; 128] = cabins[0];

        #[allow(clippy::uninit_assumed_init)]
        let mut ultra_cabins: [u32; 32] = unsafe { MaybeUninit::uninit().assume_init() };

        for (idx, cabin) in cabin_bytes.chunks(4).enumerate() {
            ultra_cabins[idx] = u32::from_le_bytes([cabin[0], cabin[1], cabin[2], cabin[3]]);
        }

        Some(Self {
            s,
            start_angle_q6,
            ultra_cabins,
        })
    }
}
