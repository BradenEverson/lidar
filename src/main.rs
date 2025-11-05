use lidar::rplidar::RpLidar;
use rppal::pwm::Pwm;

fn main() {
    let pwm = Pwm::with_pwmchip(0, 1).expect("Failed to initialize PWM");
    pwm.set_frequency(1000.0, 0.0).expect("Failed to set freq");
    pwm.enable().expect("Failed to enable");

    let mut rplidar = RpLidar::init(0, 1).expect("Failed to init RpLidar");
    rplidar.set_speed(1.0).expect("Failed to set speed");

    rplidar.scan_blocking().expect("Scanning failed");
}
