#[allow(unused_imports)]
use std::collections::HashMap;
use std::io::prelude::*;
use std::net::TcpListener;
use std::net::TcpStream;
use std::thread;

struct Reqeuest {
    method: String,
    uri: String,
    version: String,
    headers: Vec<HashMap<String, String>>,
    body: String,
}

impl Reqeuest {
    fn new(request: &[u8]) -> Self {
        let request_str = String::from_utf8_lossy(request);
        // Split the request into body and (headers + request line)
        let mut request_info = request_str.split("\r\n\r\n");

        let req_line_and_headers = request_info.next().unwrap();
        let mut lines = req_line_and_headers.lines();

        // Request line
        let line = lines.next().unwrap();
        let mut request_line = line.split_whitespace();
        let method = request_line.next().unwrap().to_string();
        let uri = request_line.next().unwrap().to_string();
        let version = request_line.next().unwrap().to_string();

        // Headers
        let mut headers = Vec::new();
        while let Some(line) = lines.next() {
            let mut header = HashMap::new();
            let (key, value) = line.split_once(": ").unwrap();
            header.insert(key.to_string(), value.to_string());
            headers.push(header);
        }

        // Body
        let body = request_info.next().unwrap_or("").to_string();

        Reqeuest {
            method,
            uri,
            version,
            headers,
            body,
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
                thread::spawn(|| {
                    handle_connection(stream);
                });
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
        _ if path.starts_with("/user-agent") => {
            let user_agent = request
                .headers
                .iter()
                .find(|header| header.contains_key("User-Agent"))
                .and_then(|header| header.get("User-Agent").cloned())
                .unwrap_or("".to_string());
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
                user_agent.len(),
                user_agent
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
