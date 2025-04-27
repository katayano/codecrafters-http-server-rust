#[allow(unused_imports)]
use std::io::prelude::*;
use std::net::TcpListener;
use std::net::TcpStream;

fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    // Uncomment this block to pass the first stage

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                handle_connection(stream);
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}

fn handle_connection(mut stream: TcpStream) {
    println!("accepted new connection");

    // Read the Request
    let mut buffer = [0; 1024];
    stream.read(&mut buffer).unwrap();
    println!("Request: {}", String::from_utf8_lossy(&buffer[..]));

    // 200 OK Pattern
    let get = b"GET / HTTP/1.1\r\n";

    // echo Pattern
    let echo = b"GET /echo/";

    // Write the Response
    if buffer.starts_with(get) {
        stream.write("HTTP/1.1 200 OK\r\n\r\n".as_bytes()).unwrap();
    } else if buffer.starts_with(echo) {
        let req_str: Vec<&[u8]> = buffer.split(|&b| b == b' ').collect();
        let uri = req_str[1];
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
            uri[6..].len(),
            String::from_utf8_lossy(&uri[6..])
        );
        stream.write(response.as_bytes()).unwrap();
    } else {
        stream
            .write("HTTP/1.1 404 Not Found\r\n\r\n".as_bytes())
            .unwrap();
    }
}
