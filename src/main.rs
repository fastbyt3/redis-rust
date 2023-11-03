use redis_starter_rust::handle_stream;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Logs from your program will appear here!");

    let listener = TcpListener::bind("127.0.0.1:6379").await?;

    loop {
        let (tcp_stream, _) = listener.accept().await?;
        println!("[*] Accepted new client.");

        tokio::spawn(async move {
            if let Err(e) = handle_stream(tcp_stream).await {
                eprintln!("Error during handling stream: {}", e);
            }
        });
    }
}
