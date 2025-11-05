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
    motor_ctrl: Pwm,
    com: Uart,
}

impl RpLidar {
    pub fn init(chip: u8, port: u8) -> Result<Self, LidarError> {
        todo!();
    }
}
