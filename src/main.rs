use std::sync::{Arc, RwLock};

use redis_starter_rust::{handle_stream, store::Store};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:6379").await?;
    let store = Arc::new(RwLock::new(Store::new()));

    loop {
        let (tcp_stream, _) = listener.accept().await?;
        let store_clone = store.clone();
        println!("[*] Accepted new client.");

        tokio::spawn(async move {
            if let Err(e) = handle_stream(tcp_stream, store_clone).await {
                eprintln!("Error during handling stream: {}", e);
            }
        });
    }
}
