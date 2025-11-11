//! RPLIDAR Struct :)

use std::{error::Error, fmt::Display};

use rppal::{
    pwm::Pwm,
    uart::{Parity, Uart},
};
use tokio::sync::mpsc::UnboundedSender;

use crate::rplidar::{
    command::{Command, WorkingMode},
    payload_parser::PayloadParser,
    rd_parser::{FlatResponse, ResponseDescriptorParser},
    response::{ScanResponse, UltraCapsuleResponse},
    ultra::{ParsedUltraCapsule, UltraCapsuleParser},
};

pub mod command;
pub mod packet;
pub mod payload_parser;
pub mod rd_parser;
pub mod response;
pub mod ultra;

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
    ultra_parser: UltraCapsuleParser,
    curr_resp: Option<FlatResponse>,
    buf: [u8; 1024],

    ultra_scan_sender: Option<UnboundedSender<ParsedUltraCapsule>>,
    scan_sender: Option<UnboundedSender<ScanResponse>>,
}

impl RpLidar {
    pub fn init(chip: u8, idx: u8) -> Result<Self, LidarError> {
        let pwm = Pwm::with_pwmchip(chip, idx).map_err(LidarError::PwmError)?;
        pwm.set_frequency(1000.0, 0.0)
            .map_err(LidarError::PwmError)?;
        pwm.enable().map_err(LidarError::PwmError)?;

        let uart = Uart::new(115_200, Parity::None, 8, 1).map_err(LidarError::UartError)?;

        Ok(Self {
            motor_ctrl: pwm,
            com: uart,
            ultra_scan_sender: None,
            scan_sender: None,

            ultra_parser: UltraCapsuleParser::default(),
            rd_parser: ResponseDescriptorParser::default(),
            p_parser: PayloadParser::default(),
            curr_resp: None,
            buf: [0; 1024],
        })
    }

    pub fn set_scan_sender(&mut self, tx: UnboundedSender<ScanResponse>) {
        self.scan_sender = Some(tx)
    }

    pub fn set_ultra_scan_sender(&mut self, tx: UnboundedSender<ParsedUltraCapsule>) {
        self.ultra_scan_sender = Some(tx)
    }

    pub fn set_speed(&mut self, speed: f64) -> Result<(), LidarError> {
        self.motor_ctrl
            .set_duty_cycle(speed)
            .map_err(LidarError::PwmError)
    }

    pub fn stop(&mut self) -> Result<(), LidarError> {
        self.send_command(Command::Stop)
    }

    pub fn send_command(&mut self, cmd: Command) -> Result<(), LidarError> {
        let packet = cmd.to_packet();
        let sent = self.com.write(packet).map_err(LidarError::UartError)?;

        if sent != packet.len() {
            Err(LidarError::CommandSendFailure)
        } else {
            self.curr_resp = None;
            Ok(())
        }
    }

    pub fn extended_scan_loop(&mut self, working_mode: u8) -> Result<(), LidarError> {
        self.send_command(Command::ExpressScan(WorkingMode::Extended(working_mode)))?;

        loop {
            self.extended_scan()
        }
    }

    pub fn extended_scan(&mut self) {
        if let Ok(n) = self.com.read(&mut self.buf) {
            for byte in &self.buf[0..n] {
                if self.curr_resp.is_some() {
                    if let Some(payload) = self.p_parser.feed(*byte)
                        && let Some(sr) = UltraCapsuleResponse::try_from_bytes(&payload)
                        && let Some(sender) = &mut self.ultra_scan_sender
                        && let Some(ultra) = self.ultra_parser.on_scan_node_capsule_data(sr)
                    {
                        sender.send(ultra).expect("Failed to send");
                    }
                } else {
                    self.curr_resp = self.rd_parser.feed(*byte);
                    if let Some(ref resp) = self.curr_resp {
                        println!("Payload Size: {}", resp.payload_len);
                        self.p_parser.set_payload_len(resp.payload_len as usize);
                    }
                }
            }
        }
    }

    pub fn scan(&mut self) {
        if let Ok(n) = self.com.read(&mut self.buf) {
            for byte in &self.buf[0..n] {
                if self.curr_resp.is_some() {
                    if let Some(payload) = self.p_parser.feed(*byte)
                        && let Some(sr) = ScanResponse::try_from_bytes(&payload)
                        && let Some(sender) = &mut self.scan_sender
                    {
                        sender.send(sr).expect("Failed to send")
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
