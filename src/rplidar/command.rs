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

#[cfg(test)]
mod tests {
    use crate::rplidar::command::Command;

    #[test]
    fn several_packets() {
        let c1 = Command::Reset;
        let c2 = Command::Scan;
        let c3 = Command::Stop;

        assert_eq!(c1.to_packet(), &[0xA5, 0x40]);
        assert_eq!(c2.to_packet(), &[0xA5, 0x20]);
        assert_eq!(c3.to_packet(), &[0xA5, 0x25]);
    }
}
