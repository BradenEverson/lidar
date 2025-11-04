use std::time::Duration;

use rppal::gpio::Gpio;

pub const MOTOR_CTRL: u8 = 26;

fn main() {
    let gpio = Gpio::new().expect("Failed to startup GPIO");
    let mut pin = gpio
        .get(MOTOR_CTRL)
        .expect("Failed to assign pin")
        .into_output();

    pin.set_high();

    std::thread::sleep(Duration::from_secs(5));

    pin.set_low();
}
