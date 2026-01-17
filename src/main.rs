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

                        match cmd::from_frame(frame) {
                            Ok(command) => {
                                info!("parsed command: {:?}", command);
                                match command {
                                    Command::Ping => {
                                        let response = Frame::Simple("PONG".to_string());
                                        if let Err(e) = connection.write_frame(&response).await {
                                            error!("failed to write response: {}", e);
                                            break;
                                        }
                                    }
                                    Command::Set { key, value, expiry_seconds } => {
                                        let duration = expiry_seconds.map(std::time::Duration::from_secs);
                                        db.set(key, value, duration);
                                        let response = Frame::Simple("OK".to_string());
                                        if let Err(e) = connection.write_frame(&response).await {
                                            error!("failed to write response: {}", e);
                                            break;
                                        }
                                    }
                                    Command::Get { key } => {
                                        let response = match db.get(&key) {
                                            Some(value) => Frame::Bulk(value),
                                            None => Frame::Null,
                                        };
                                        if let Err(e) = connection.write_frame(&response).await {
                                            error!("failed to write response: {}", e);
                                            break;
                                        }
                                    }
                                    Command::Publish { channel, message } => {
                                        let num_receivers = db.publish(channel, message);
                                        let response = Frame::Integer(num_receivers as i64);
                                        if let Err(e) = connection.write_frame(&response).await {
                                            error!("failed to write response: {}", e);
                                            break;
                                        }
                                    }
                                    Command::Subscribe { channel } => {
                                        let confirmation = Frame::Array(vec![
                                            Frame::Bulk("subscribe".into()),
                                            Frame::Bulk(channel.clone().into()),
                                            Frame::Integer(1),
                                        ]);
                                        if let Err(e) = connection.write_frame(&confirmation).await {
                                            error!("failed to write subscription confirmation: {}", e);
                                            break;
                                        }

                                        let mut rx = db.subscribe(channel.clone());
                                        
                                        loop {
                                            tokio::select! {
                                                result = rx.recv() => {
                                                    match result {
                                                        Ok(msg) => {
                                                            let message_frame = Frame::Array(vec![
                                                                Frame::Bulk("message".into()),
                                                                Frame::Bulk(channel.clone().into()),
                                                                Frame::Bulk(msg),
                                                            ]);
                                                            if let Err(e) = connection.write_frame(&message_frame).await {
                                                                error!("failed to write message: {}", e);
                                                                break;
                                                            }
                                                        }
                                                        Err(e) => {
                                                            error!("broadcast channel error: {}", e);
                                                            break;
                                                        }
                                                    }
                                                }

                                                result = connection.read_frame() => {
                                                    match result {
                                                        Ok(Some(_frame)) => {
                                                            info!("client sent command in subscription mode, exiting");
                                                            break;
                                                        }
                                                        Ok(None) => {
                                                            info!("client disconnected");
                                                            break;
                                                        }
                                                        Err(e) => {
                                                            error!("error reading frame: {}", e);
                                                            break;
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        break;
                                    }
                                }
                            }
                            Err(e) => {
                                error!("parse error: {}", e);
                                let response = Frame::Error(format!("ERR {}", e));
                                if let Err(e) = connection.write_frame(&response).await {
                                    error!("failed to write response: {}", e);
                                    break;
                                }
                            }
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
