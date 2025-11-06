use std::{env, sync::Arc};

use hyper::server::conn::http1;
use hyper_util::rt::TokioIo;
use lidar::{
    rplidar::{RpLidar, response::ScanResponse},
    service::LidarService,
};
use tokio::{
    net::TcpListener,
    sync::{
        Mutex,
        mpsc::{UnboundedReceiver, UnboundedSender},
    },
};

#[tokio::main]
async fn main() {
    let mut rplidar = RpLidar::init(0, 1).expect("Failed to init RpLidar");

    let (tx, rx): (
        UnboundedSender<ScanResponse>,
        UnboundedReceiver<ScanResponse>,
    ) = tokio::sync::mpsc::unbounded_channel();

    let port = env::var("PORT").unwrap_or_else(|_| "7878".to_string());
    let listener = TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .expect("Failed to bind to server");

    println!("Listening on Port {port}");

    let rx = Arc::new(Mutex::new(rx));
    tokio::spawn(async move {
        loop {
            let (socket, _) = listener
                .accept()
                .await
                .expect("Error accepting incoming connection");

            let io = TokioIo::new(socket);
            let service = LidarService::new(rx.clone());

            tokio::spawn(async move {
                http1::Builder::new()
                    .serve_connection(io, service)
                    .with_upgrades()
                    .await
                    .expect("Failed to serve connection")
            });
        }
    });

    rplidar.set_speed(1.0).expect("Failed to set speed");
    rplidar.set_scan_sender(tx);

    rplidar.scan_blocking().expect("Scanning failed");
}

pub fn read_scan(s: ScanResponse) {
    if s.dist != 0.0 {
        let (x, y) = polar_to_rectangular(s.dist, s.angle);
        println!("{:.2}, {:.2}, {}", x, y, s.quality);
    }
}

fn polar_to_rectangular(r: f32, theta: f32) -> (f32, f32) {
    let theta = theta.to_radians();
    (r * theta.cos(), r * theta.sin())
}
