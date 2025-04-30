#[allow(unused_imports)]
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::prelude::*;
use std::net::TcpListener;
use std::net::TcpStream;

mod shared;
use shared::thread_pool::ThreadPool;

struct Reqeuest {
    method: RequestMethod,
    uri: String,
    version: String,
    headers: Vec<HashMap<String, String>>,
    body: String,
}

enum RequestMethod {
    GET,
    POST,
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
        let method = request_line.next().unwrap();
        let method = match method {
            "GET" => RequestMethod::GET,
            "POST" => RequestMethod::POST,
            _ => panic!("Unsupported request method"),
        };
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

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();
    let pool = ThreadPool::new(5);

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                pool.execute(|| {
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

    // Set directory for response files
    let mut args = env::args().skip(1);
    let res_file_dir = match args.next() {
        Some(dir_option) if dir_option == "--directory" => args.next(),
        _ => None,
    };

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
            // Get the subpath after /echo/
            let mut iter = path.split("/");
            let sub_path = iter.nth(2).unwrap();

            // Get the Accept-Encoding header
            let accept_encodings = request
                .headers
                .iter()
                .find(|header| header.contains_key("Accept-Encoding"))
                .and_then(|header| header.get("Accept-Encoding").cloned())
                .unwrap_or("".to_string());

            // Check if the Accept-Encoding header contains gzip
            if accept_encodings
                .split(", ")
                .any(|encoding| encoding == "gzip")
            {
                // If it does, return the response with gzip encoding
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Encoding: gzip\r\nContent-Length: {}\r\n\r\n{}",
                    sub_path.len(),
                    sub_path
                );
                stream.write(response.as_bytes()).unwrap();
            } else {
                // If it doesn't, return the response without gzip encoding
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
                    sub_path.len(),
                    sub_path
                );
                stream.write(response.as_bytes()).unwrap();
            }
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
        _ if path.starts_with("/files") => {
            match res_file_dir {
                // Check if the directory is provided
                Some(dir) => {
                    match request.method {
                        RequestMethod::GET => {
                            // Get the filename and contents of file
                            let mut iter = path.split("/");
                            let file_name = iter.nth(2).unwrap();
                            let file_path = format!("{}/{}", dir, file_name);
                            let file_content = fs::read(file_path);
                            match file_content {
                                Ok(content) => {
                                    let response = format!(
                                                "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\n\r\n{}",
                                                content.len(),
                                                String::from_utf8_lossy(&content)
                                            );
                                    stream.write(response.as_bytes()).unwrap();
                                }
                                Err(_) => {
                                    stream
                                        .write("HTTP/1.1 404 Not Found\r\n\r\n".as_bytes())
                                        .unwrap();
                                }
                            }
                        }
                        RequestMethod::POST => {
                            let content_type = request
                                .headers
                                .iter()
                                .find(|header| header.contains_key("Content-Type"))
                                .and_then(|header| header.get("Content-Type").cloned())
                                .unwrap_or("".to_string());

                            // Check if the content type is application/octet-stream
                            if content_type == "application/octet-stream" {
                                // Get the Content-Length header of the request
                                let content_length = request
                                    .headers
                                    .iter()
                                    .find(|header| header.contains_key("Content-Length"))
                                    .and_then(|header| header.get("Content-Length").cloned())
                                    .unwrap_or("0".to_string())
                                    .parse::<usize>()
                                    .unwrap_or(0);

                                // Get the filename
                                let mut iter = path.split("/");
                                let file_name = iter.nth(2).unwrap();
                                let file_path = format!("{}/{}", dir, file_name);
                                //Create the file and write the contents
                                let mut file = fs::File::create(file_path).unwrap();
                                file.write_all(&request.body.as_bytes()[..content_length])
                                    .unwrap();
                                stream
                                    .write("HTTP/1.1 201 Created\r\n\r\n".as_bytes())
                                    .unwrap();
                            } else {
                                // If the content type is not application/octet-stream, return 415
                                stream
                                    .write("HTTP/1.1 415 Unsupported Media Type\r\n\r\n".as_bytes())
                                    .unwrap();
                            }
                        }
                    }
                }
                None => {
                    stream
                        .write("HTTP/1.1 404 Not Found\r\n\r\n".as_bytes())
                        .unwrap();
                }
            }
        }
        _ => {
            stream
                .write("HTTP/1.1 404 Not Found\r\n\r\n".as_bytes())
                .unwrap();
        }
    }
}
