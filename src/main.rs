use std::sync::Arc;

use clap::Parser;
use redis_starter_rust::{config::Config, handle_stream, store::Store};
use tokio::net::TcpListener;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long, default_value = "127.0.0.1:6379")]
    addr: String,

    #[arg(long = "dir", default_value = "./persistance")]
    rdb_dir: String,

    #[arg(long = "dbfilename", default_value = "data.rdb")]
    rdb_file: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let config = Config::new(args.addr, args.rdb_dir, args.rdb_file);

    let listener = TcpListener::bind(config.get_addr_string()).await?;
    let store = Arc::new(Store::new());

    loop {
        let (tcp_stream, _) = listener.accept().await?;
        let store_clone = store.clone();
        let config_clone = config.clone();
        println!("[*] Accepted new client.");

        tokio::spawn(async move {
            if let Err(e) = handle_stream(tcp_stream, store_clone, config_clone).await {
                eprintln!("Error during handling stream: {}", e);
            }
        });
    }
}
