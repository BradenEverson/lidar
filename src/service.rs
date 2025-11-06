//! Lidar HTTP Websocket Service implementation

//! Service implementation

use std::{fs::File, future::Future, io::Read, pin::Pin, sync::Arc};

use futures::{SinkExt, StreamExt};
use http_body_util::Full;
use hyper::{
    Method, Request, Response, StatusCode,
    body::{self, Bytes},
    service::Service,
};
use hyper_tungstenite::{is_upgrade_request, upgrade};
use tokio::sync::{Mutex, mpsc::UnboundedReceiver};
use tokio_tungstenite::tungstenite::Message;

use crate::rplidar::response::ScanResponse;

/// Lidar websocket communication manager
pub struct LidarService {
    /// The receiving end of a scan response sender
    recv: Arc<Mutex<UnboundedReceiver<ScanResponse>>>,
}

impl LidarService {
    /// Creates a new Lidar Service
    pub fn new(rx: Arc<Mutex<UnboundedReceiver<ScanResponse>>>) -> Self {
        Self { recv: rx }
    }
}

#[allow(tail_expr_drop_order)]
impl Service<Request<body::Incoming>> for LidarService {
    type Response = Response<Full<Bytes>>;
    type Error = hyper::http::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&self, mut req: Request<body::Incoming>) -> Self::Future {
        if is_upgrade_request(&req) {
            let (response, websocket) = upgrade(&mut req, None).expect("Failed to upgrade to WS");
            let rx = self.recv.clone();

            tokio::spawn(async move {
                let (mut ws_write, _) = websocket.await.expect("Await websocket").split();

                while let Some(msg) = { rx.lock().await.recv().await } {
                    if msg.dist > 0.0 {
                        let mut payload = vec![];
                        payload.push(if msg.new { 1 } else { 0 });
                        payload.extend_from_slice(&msg.angle.to_be_bytes());
                        payload.extend_from_slice(&msg.dist.to_be_bytes());

                        let _ = ws_write.send(Message::binary(payload)).await.map_err(|_| {
                            println!("ERROR: Failed to send");
                        });
                    }
                }
            });

            Box::pin(async { Ok(response) })
        } else {
            let response = Response::builder();
            let res = match (req.method().clone(), req.uri().path()) {
                (Method::GET, "/") => {
                    let mut buf = vec![];
                    let mut page = File::open("frontend/index.html").expect("Failed to find file");
                    page.read_to_end(&mut buf)
                        .expect("Failed to read to buffer");
                    response
                        .status(StatusCode::OK)
                        .body(Full::new(Bytes::copy_from_slice(&buf)))
                }

                _ => response
                    .status(StatusCode::METHOD_NOT_ALLOWED)
                    .body(Full::new(Bytes::from_static(b"Method Not Allowed"))),
            };

            Box::pin(async { res })
        }
    }
}
