mod frame;
mod connection;

use connection::Connection;
use frame::Frame;
use tokio::net::TcpListener;
use tracing::{error, info, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("failed to set tracing subscriber");

    let addr = "127.0.0.1:6379";

    let listener = match TcpListener::bind(addr).await {
        Ok(listener) => {
            info!("listening on {}", addr);
            listener
        }
        Err(e) => {
            error!("failed to bind to {}: {}", addr, e);
            return;
        }
    };

    loop {
        match listener.accept().await {
            Ok((socket, peer_addr)) => {
                tokio::spawn(async move {
                    info!("accepted connection from: {}", peer_addr);

                    let mut connection = Connection::new(socket);

                    while let Ok(Some(frame)) = connection.read_frame().await {
                        info!("received frame: {:?}", frame);

                        let response = match frame {
                            Frame::Array(ref frames) if !frames.is_empty() => {
                                match &frames[0] {
                                    Frame::Bulk(cmd) => {
                                        let cmd_str = String::from_utf8_lossy(cmd).to_uppercase();
                                        match cmd_str.as_str() {
                                            "PING" => Frame::Simple("PONG".to_string()),
                                            _ => Frame::Error(format!("ERR unknown command '{}'", cmd_str)),
                                        }
                                    }
                                    _ => Frame::Error("ERR invalid command format".to_string()),
                                }
                            }
                            _ => Frame::Error("ERR invalid command format".to_string()),
                        };

                        if let Err(e) = connection.write_frame(&response).await {
                            error!("failed to write response: {}", e);
                            break;
                        }
                    }

                    info!("connection closed from: {}", peer_addr);
                });
            }
            Err(e) => {
                error!("failed to accept connection: {}", e);
            }
        }
    }
}
