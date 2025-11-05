//! RPLIDAR Struct :)

use std::{error::Error, fmt::Display};

use rppal::{pwm::Pwm, uart::Uart};

#[derive(Debug, PartialEq, Eq)]
pub enum LidarError {}

impl Display for LidarError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, ":(")
    }
}

impl Error for LidarError {}

pub struct RpLidar {
    pub motor_ctrl: Pwm,
    pub com: Uart,
}

impl RpLidar {
    pub fn init(_chip: u8, _port: u8) -> Result<Self, LidarError> {
        todo!();
    }
}
