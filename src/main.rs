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
                    
                    let _ = socket;
                });
            }
            Err(e) => {
                error!("failed to accept connection: {}", e);
            }
        }
    }
}
