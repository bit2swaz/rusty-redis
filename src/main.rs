mod frame;
mod connection;
mod db;
mod cmd;

use connection::Connection;
use frame::Frame;
use db::Db;
use cmd::Command;
use tokio::net::TcpListener;
use tracing::{error, info, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("failed to set tracing subscriber");

    let addr = "127.0.0.1:6379";
    let db = Db::new();

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
                let db = db.clone();
                tokio::spawn(async move {
                    info!("accepted connection from: {}", peer_addr);

                    let mut connection = Connection::new(socket);

                    while let Ok(Some(frame)) = connection.read_frame().await {
                        info!("received frame: {:?}", frame);

                        let response = match cmd::from_frame(frame) {
                            Ok(command) => {
                                info!("parsed command: {:?}", command);
                                match command {
                                    Command::Ping => Frame::Simple("PONG".to_string()),
                                    Command::Set { key, value, expiry_seconds } => {
                                        let duration = expiry_seconds.map(std::time::Duration::from_secs);
                                        db.set(key, value, duration);
                                        Frame::Simple("OK".to_string())
                                    }
                                    Command::Get { key } => {
                                        match db.get(&key) {
                                            Some(value) => Frame::Bulk(value),
                                            None => Frame::Null,
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                error!("parse error: {}", e);
                                Frame::Error(format!("ERR {}", e))
                            }
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
