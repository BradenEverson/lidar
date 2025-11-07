//! Higher level command interface

use crate::rplidar::packet::{OpCode, Packet};

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
            Self::ExpressScan(wm) => Packet::new(OpCode::ExpressScan, &[wm.to_byte(), 0, 0, 0, 0]),
            _ => todo!(),
        };

        packet.to_bytes()
    }
}

pub enum WorkingMode {
    Legacy,
    Extended(u8),
}

impl WorkingMode {
    pub fn to_byte(&self) -> u8 {
        match self {
            Self::Legacy => 0,
            Self::Extended(w) => *w,
        }
    }
}

pub enum LidarConf {
    ScanModeCount,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfOpCode {
    ScanModeCount = 0x70,
}

#[cfg(test)]
mod tests {
    use crate::rplidar::command::{Command, WorkingMode};

    #[test]
    fn several_packets() {
        let c1 = Command::Reset;
        let c2 = Command::Scan;
        let c3 = Command::Stop;

        assert_eq!(c1.to_packet(), &[0xA5, 0x40]);
        assert_eq!(c2.to_packet(), &[0xA5, 0x20]);
        assert_eq!(c3.to_packet(), &[0xA5, 0x25]);
    }

    #[test]
    fn extended() {
        let express = Command::ExpressScan(WorkingMode::Legacy);
        let expected = [0xA5, 0x82, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x22];

        assert_eq!(express.to_packet(), &expected);
    }
}
