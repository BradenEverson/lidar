//! RPLIDAR Struct :)

use std::{error::Error, fmt::Display};

use rppal::{
    pwm::Pwm,
    uart::{Parity, Uart},
};

use crate::command::Command;

#[derive(Debug)]
pub enum LidarError {
    PwmError(rppal::pwm::Error),
    UartError(rppal::uart::Error),
    CommandSendFailure,
}

impl Display for LidarError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PwmError(perr) => write!(f, "{perr}"),
            Self::UartError(uerr) => write!(f, "{uerr}"),
            Self::CommandSendFailure => write!(f, "Failed to send command fully"),
        }
    }
}

impl Error for LidarError {}

pub struct RpLidar {
    motor_ctrl: Pwm,
    com: Uart,
}

impl RpLidar {
    pub fn init(chip: u8, idx: u8) -> Result<Self, LidarError> {
        let pwm = Pwm::with_pwmchip(chip, idx).map_err(|e| LidarError::PwmError(e))?;
        pwm.set_frequency(1000.0, 0.0)
            .map_err(|e| LidarError::PwmError(e))?;
        pwm.enable().map_err(|e| LidarError::PwmError(e))?;

        let uart = Uart::new(115_200, Parity::None, 8, 1).map_err(|e| LidarError::UartError(e))?;

        Ok(Self {
            motor_ctrl: pwm,
            com: uart,
        })
    }

    pub fn set_speed(&mut self, speed: f64) -> Result<(), LidarError> {
        self.motor_ctrl
            .set_duty_cycle(speed)
            .map_err(|e| LidarError::PwmError(e))
    }

    pub fn send_command(&mut self, cmd: Command) -> Result<(), LidarError> {
        let packet = cmd.to_packet();
        let sent = self
            .com
            .write(packet)
            .map_err(|e| LidarError::UartError(e))?;

        if sent != packet.len() {
            Err(LidarError::CommandSendFailure)
        } else {
            Ok(())
        }
    }
}
