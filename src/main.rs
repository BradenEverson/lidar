use lidar::payload_parser::PayloadParser;
use lidar::rd_parser::ResponseDescriptorParser;
use rppal::pwm::Pwm;
use rppal::uart::{Parity, Uart};

fn main() {
    let pwm = Pwm::with_pwmchip(0, 1).expect("Failed to initialize PWM");
    pwm.set_frequency(1000.0, 0.0).expect("Failed to set freq");
    pwm.enable().expect("Failed to enable");

    let mut uart = Uart::new(115_200, Parity::None, 8, 1).expect("Failed to initialize UART");

    pwm.set_duty_cycle(1.0).expect("Failed to set duty cycle");

    let mut rd_parser = ResponseDescriptorParser::default();
    let mut p_parser = PayloadParser::default();

    let mut curr_resp = None;

    uart.write(&[0xA5, 0x20]).expect("Failed to write");

    let mut buf = [0; 1024];

    loop {
        if let Ok(n) = uart.read(&mut buf) {
            for byte in &buf[0..n] {
                if curr_resp.is_some() {
                    if let Some(payload) = p_parser.feed(*byte) {
                        println!("Payload: {:?}", payload);
                    }
                } else {
                    curr_resp = rd_parser.feed(*byte);
                    if let Some(ref resp) = curr_resp {
                        p_parser.set_payload_len(resp.payload_len as usize);
                        println!("{:?}", resp)
                    }
                }
            }
        }
    }
}
