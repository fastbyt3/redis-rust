use std::sync::Arc;

use clap::Parser;
use redis_starter_rust::{config::Config, handle_stream, rdb::read_rdb_file, store::Store};
use tokio::net::TcpListener;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long, default_value = "127.0.0.1:6379")]
    addr: String,

    #[arg(long = "dir")]
    rdb_dir: Option<String>,

    #[arg(long = "dbfilename")]
    rdb_file: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let config = Config::new(args.addr, args.rdb_dir, args.rdb_file);

    // Read data from RDB file into a HASHMAP
    // then add it to state store
    let rdb_kv_data = read_rdb_file(&config);
    println!("{:?}", rdb_kv_data);

    let listener = TcpListener::bind(config.get_addr_string()).await?;
    let store = Arc::new(Store::new(rdb_kv_data));

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
