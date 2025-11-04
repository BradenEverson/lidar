//! Higher level command interface

pub enum Command {
    Stop,
    Reset,
    Start,
    /// 5 byte payload, only 1 is used for working mode
    /// followed by 4 zeros
    ExpressScan(WorkingMode),
    ForceScan,
    GetInfo,
    GetHealth,
    GetSampleRate,
    GetLidarConf(LidarConf),
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
