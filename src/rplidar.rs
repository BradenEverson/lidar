//! RPLIDAR Struct :)

use std::{error::Error, fmt::Display};

use rppal::{
    pwm::Pwm,
    uart::{Parity, Uart},
};

use crate::{
    command::Command,
    payload_parser::PayloadParser,
    rd_parser::{FlatResponse, ResponseDescriptorParser},
    response::ScanResponse,
};

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

    rd_parser: ResponseDescriptorParser,
    p_parser: PayloadParser,
    curr_resp: Option<FlatResponse>,
    buf: [u8; 1024],

    scan_handler: Option<fn(ScanResponse)>,
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
            scan_handler: None,

            rd_parser: ResponseDescriptorParser::default(),
            p_parser: PayloadParser::default(),
            curr_resp: None,
            buf: [0; 1024],
        })
    }

    pub fn set_scan_handler(&mut self, handler: fn(ScanResponse)) {
        self.scan_handler = Some(handler);
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
            self.curr_resp = None;
            Ok(())
        }
    }

    pub fn scan(&mut self) {
        if let Ok(n) = self.com.read(&mut self.buf) {
            for byte in &self.buf[0..n] {
                if self.curr_resp.is_some() {
                    if let Some(payload) = self.p_parser.feed(*byte) {
                        if let Some(sr) = ScanResponse::try_from_bytes(&payload) {
                            if let Some(handler) = self.scan_handler {
                                handler(sr);
                            }
                        }
                    }
                } else {
                    self.curr_resp = self.rd_parser.feed(*byte);
                    if let Some(ref resp) = self.curr_resp {
                        self.p_parser.set_payload_len(resp.payload_len as usize);
                    }
                }
            }
        }
    }

    pub fn scan_blocking(&mut self) -> Result<(), LidarError> {
        self.send_command(Command::Scan)?;

        loop {
            self.scan()
        }
    }
}
