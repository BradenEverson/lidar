use lidar::{response::ScanResponse, rplidar::RpLidar};

fn main() {
    let mut rplidar = RpLidar::init(0, 1).expect("Failed to init RpLidar");
    rplidar.set_scan_handler(read_scan);

    rplidar.set_speed(1.0).expect("Failed to set speed");

    rplidar.scan_blocking().expect("Scanning failed");
}

fn read_scan(s: ScanResponse) {
    if s.dist != 0.0 {
        let (x, y) = polar_to_rectangular(s.dist, s.angle);
        println!("{:.2}, {:.2}, {}", x, y, s.quality);
    }
}

fn polar_to_rectangular(r: f32, theta: f32) -> (f32, f32) {
    (r.to_radians() * theta.cos(), r.to_radians() * theta.sin())
}
