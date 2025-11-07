//! Data Response Format

use std::mem::MaybeUninit;

use crate::rplidar::packet::OpCode;

pub enum DataResponse {
    ScanResponse(ScanResponse),
    ExpressResponse(ExpressResponse),
}

impl DataResponse {
    pub fn try_from_packet(sent_from: OpCode, bytes: &[u8]) -> Option<Self> {
        match sent_from {
            OpCode::Scan => ScanResponse::try_from_bytes(bytes).map(ScanResponse::wrap),
            OpCode::ExpressScan => {
                ExpressResponse::try_from_bytes(bytes).map(ExpressResponse::wrap)
            }
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

#[derive(Debug, Clone, Copy)]
pub struct ExpressResponse {
    pub s: u8,
    pub checksum: u8,
    pub start_angle_q6: u16,
    pub cabins: [Scan; 96],
}

impl ExpressResponse {
    pub fn try_from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() != 132 {
            return None;
        }

        let f1 = bytes[0] & 0xF0;
        let f2 = (bytes[1] & 0xF0) >> 4;

        if f1 | f2 != 0xA5 {
            return None;
        }

        let c1 = bytes[0] & 0x0F;
        let c2 = bytes[1] & 0x0F;

        let start_angle_q6 = u16::from_le_bytes([bytes[2], bytes[3]]) & 0x7FFF;

        let s = bytes[3] & 0x80 >> 7;

        let checksum = (c2 << 4) | c1;

        #[allow(clippy::uninit_assumed_init)]
        let cabins: [Scan; 96] = unsafe { MaybeUninit::uninit().assume_init() };

        let cabin_bytes = &bytes[4..];

        for (k, cabin) in cabin_bytes.chunks(4).enumerate() {
            println!("{k} - {cabin:X?}");
            // let dist = RawCabin {
            //     data: [cabin[0], cabin[1]],
            // }
            // .to_dist();
            //
            // cabins[k].dist = dist as f32 / 4.0;
            // cabins[k].angle = w as f32 / 64.0;
        }

        Some(Self {
            s,
            checksum,
            start_angle_q6,
            cabins,
        })
    }

    pub fn fix_angles(&mut self, next_start_angle_q6: u16) {
        let current_angle = self.start_angle_q6 as f32 / 64.0;
        let next_angle = next_start_angle_q6 as f32 / 64.0;

        let mut angle_diff = next_angle - current_angle;
        if angle_diff < 0.0 {
            angle_diff += 360.0;
        }

        let angle_increment = angle_diff / 40.0;

        for (i, cabin) in self.cabins.iter_mut().enumerate() {
            cabin.angle = current_angle + angle_increment * i as f32;
            if cabin.angle >= 360.0 {
                cabin.angle -= 360.0;
            }
        }
    }

    pub fn wrap(self) -> DataResponse {
        DataResponse::ExpressResponse(self)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct RawCabin {
    data: [u8; 2],
}

impl RawCabin {
    pub fn to_dist(&self) -> u16 {
        u16::from_be_bytes(self.data)
    }
}
