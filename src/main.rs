use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
};

async fn handle_stream(mut stream: TcpStream) {
    let mut buff = [0; 1024];

    // Handle multiple PINGs from same conenction
    loop {
        let bytes_recv = stream.read(&mut buff).unwrap_or(0);

        if bytes_recv == 0 {
            break;
        }

        let req = String::from_utf8(buff[..bytes_recv].to_vec())
            .expect("Error during parsing buffer as String");
        println!("Request: {:?}", req);

        let pong_resp = "+PONG\r\n";
        stream
            .write_all(pong_resp.as_bytes())
            .expect("Write to TCP stream failed");
    }
    stream.flush().unwrap();
}

#[tokio::main]
async fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    let listener = TcpListener::bind("127.0.0.1:6379").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("accepted new connection");
                tokio::spawn(async move {
                    handle_stream(stream).await;
                });
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
