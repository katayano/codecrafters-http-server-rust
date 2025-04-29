#[allow(unused_imports)]
use std::io::prelude::*;
use std::net::TcpListener;
use std::net::TcpStream;

struct Reqeuest {
    method: String,
    uri: String,
    version: String,
}

impl Reqeuest {
    fn new(request: &[u8]) -> Self {
        let request_str = String::from_utf8_lossy(request);
        let request_info = request_str.split_whitespace().collect::<Vec<&str>>();

        let method = request_info[0].to_string();
        let uri = request_info[1].to_string();
        let version = request_info[2].to_string();

        Reqeuest {
            method,
            uri,
            version,
        }
    }
}

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

    let request = Reqeuest::new(&buffer);

    // Write the Response
    let path = request.uri.as_str();
    match path {
        "/" => {
            stream.write("HTTP/1.1 200 OK\r\n\r\n".as_bytes()).unwrap();
        }
        _ if path.starts_with("/echo/") => {
            let mut iter = path.split("/");
            let sub_path = iter.nth(2).unwrap();
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
                sub_path.len(),
                sub_path
            );
            stream.write(response.as_bytes()).unwrap();
        }
        _ => {
            stream
                .write("HTTP/1.1 404 Not Found\r\n\r\n".as_bytes())
                .unwrap();
        }
    }
}
