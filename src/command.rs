//! Higher level command interface

use crate::packet::{OpCode, Packet};

pub enum Command {
    Stop,
    Reset,
    Scan,
    /// 5 byte payload, only 1 is used for working mode
    /// followed by 4 zeros
    ExpressScan(WorkingMode),
    ForceScan,
    GetInfo,
    GetHealth,
    GetSampleRate,
    GetLidarConf(LidarConf),
}

impl Command {
    pub fn to_packet(&self) -> &[u8] {
        let packet = match self {
            Self::Stop => Packet::new(OpCode::Stop, &[]),
            Self::Reset => Packet::new(OpCode::Reset, &[]),
            Self::Scan => Packet::new(OpCode::Scan, &[]),
            _ => todo!(),
        };

        packet.to_bytes()
    }
}

pub enum WorkingMode {
    Legacy,
    Extended(u8),
}

pub enum LidarConf {
    ScanModeCount,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfOpCode {
    ScanModeCount = 0x70,
}
