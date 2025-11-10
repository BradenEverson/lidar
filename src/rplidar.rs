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
    response::{ExpressResponse, Scan, ScanResponse},
    ultra_capsule_parser::{HQNode, UltraCapsuleParser},
};

pub mod command;
pub mod packet;
pub mod payload_parser;
pub mod rd_parser;
pub mod response;
pub mod ultra_capsule_parser;

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

    scan_handler: Option<fn(ScanResponse)>,
    scan_sender: Option<UnboundedSender<ExpressResponse>>,
    ultra_scan_sender: Option<UnboundedSender<Vec<HQNode>>>,
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
            scan_handler: None,
            scan_sender: None,
            ultra_scan_sender: None,

            rd_parser: ResponseDescriptorParser::default(),
            p_parser: PayloadParser::default(),
            ultra_parser: UltraCapsuleParser::new(),
            curr_resp: None,
            buf: [0; 1024],
        })
    }

    pub fn set_scan_handler(&mut self, handler: fn(ScanResponse)) {
        self.scan_handler = Some(handler);
    }

    pub fn set_scan_sender(&mut self, tx: UnboundedSender<ExpressResponse>) {
        self.scan_sender = Some(tx)
    }

    pub fn set_ultra_scan_sender(&mut self, tx: UnboundedSender<Vec<HQNode>>) {
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
            self.ultra_parser.reset();
            Ok(())
        }
    }

    pub fn extended_scan_loop(&mut self, working_mode: u8) -> Result<(), LidarError> {
        self.send_command(Command::ExpressScan(WorkingMode::Extended(working_mode)))?;

        loop {
            self.extended_scan()
        }
    }

    pub fn ultra_scan_loop(&mut self) -> Result<(), LidarError> {
        self.send_command(Command::ExpressScan(WorkingMode::Standard))?;

        loop {
            self.ultra_scan()
        }
    }

    pub fn extended_scan(&mut self) {
        if let Ok(n) = self.com.read(&mut self.buf) {
            for byte in &self.buf[0..n] {
                if self.curr_resp.is_some() {
                    if let Some(payload) = self.p_parser.feed(*byte)
                        && let Some(sr) = ExpressResponse::try_from_bytes(&payload)
                        && let Some(sender) = &mut self.scan_sender
                    {
                        sender.send(sr).expect("Failed to send");
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

    pub fn ultra_scan(&mut self) {
        if let Ok(n) = self.com.read(&mut self.buf) {
            let nodes = self.ultra_parser.on_data(&self.buf[0..n]);

            if !nodes.is_empty() {
                if let Some(sender) = &self.ultra_scan_sender {
                    sender.send(nodes).expect("Failed to send ultra scan nodes");
                }

                if let Some(express_sender) = &self.scan_sender {
                    let express_responses = self.convert_hq_nodes_to_express(&nodes);
                    for response in express_responses {
                        express_sender
                            .send(response)
                            .expect("Failed to send converted express response");
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
                    {
                        if let Some(handler) = self.scan_handler {
                            handler(sr);
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

    fn convert_hq_nodes_to_express(&self, nodes: &[HQNode]) -> Vec<ExpressResponse> {
        let mut express_responses = Vec::new();

        for chunk in nodes.chunks(32) {
            if chunk.len() == 32 {
                let mut cabins = [Scan::default(); 96];

                for (i, node) in chunk.iter().enumerate() {
                    if i < cabins.len() {
                        cabins[i] = Scan {
                            angle: (node.angle_z_q14 as f32 * 90.0) / (1 << 14) as f32,
                            dist: (node.dist_mm_q2 as f32) / 4.0,
                        };
                    }
                }

                let response = ExpressResponse {
                    s: 0,
                    checksum: 0,
                    start_angle_q6: 0,
                    cabins,
                };

                express_responses.push(response);
            }
        }

        express_responses
    }
}

impl From<HQNode> for ScanResponse {
    fn from(node: HQNode) -> Self {
        let angle_degrees = (node.angle_z_q14 as f32 * 90.0) / (1 << 14) as f32;

        ScanResponse {
            new: (node.flag & 0x1) != 0,
            quality: (node.quality >> 2) as u8, // Adjust quality conversion as needed
            angle: angle_degrees,
            dist: (node.dist_mm_q2 as f32) / 4.0,
        }
    }
}
