use std::{net::{TcpListener, TcpStream}, io::{Read, Write}};

fn handle_stream(mut stream: TcpStream) {
    let mut buff = [0; 1024];
    let bytes_recv = stream.read(&mut buff).expect("Unable to read stream into buffer");
    let req = String::from_utf8(buff[..bytes_recv].to_vec()).expect("Error during parsing buffer as String");
    println!("Request: {}", req);

    let pong_resp = "+PONG\r\n";
    stream.write_all(pong_resp.as_bytes()).unwrap();
    stream.flush().unwrap();
}

fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    let listener = TcpListener::bind("127.0.0.1:6379").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("accepted new connection");
                handle_stream(stream)
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
