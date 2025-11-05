use lidar::parser::ResponseDescriptorParser;
use rppal::pwm::Pwm;
use rppal::uart::{Parity, Uart};
use std::time::Duration;

fn main() {
    let pwm = Pwm::with_pwmchip(0, 1).expect("Failed to initialize PWM");
    pwm.set_frequency(1000.0, 0.0).expect("Failed to set freq");
    pwm.enable().expect("Failed to enable");

    let mut uart = Uart::new(115_200, Parity::None, 8, 1).expect("Failed to initialize UART");
    uart.set_read_mode(1, Duration::from_millis(100))
        .expect("Failed to set read mode");

    pwm.set_duty_cycle(1.0).expect("Failed to set duty cycle");

    let mut rd_parser = ResponseDescriptorParser::default();
    let mut curr_resp = None;

    uart.write(&[0xA5, 0x20]).expect("Failed to write");

    loop {
        let mut buf = [0; 1024];
        if let Ok(n) = uart.read(&mut buf) {
            for byte in &buf[0..n] {
                if let Some(ref resp) = curr_resp {
                    // Parse payload
                } else {
                    curr_resp = rd_parser.feed(*byte);
                    if let Some(ref resp) = curr_resp {
                        println!("{:?}", resp)
                    }
                }
            }
        }
    }
}
