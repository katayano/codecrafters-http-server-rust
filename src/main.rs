#[allow(unused_imports)]
use std::{
    collections::HashMap,
    env, fs,
    io::prelude::*,
    net::{TcpListener, TcpStream},
};

use flate2::{write::GzEncoder, Compression};

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
            _ => panic!("Unsupported request method: {}", method),
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

fn handle_connection(stream: TcpStream) {
    println!("accepted new connection");

    // Set directory for response files
    let mut args = env::args().skip(1);
    let res_file_dir = match args.next() {
        Some(dir_option) if dir_option == "--directory" => args.next(),
        _ => None,
    };

    loop {
        // Read the Request
        let request = read_request(&stream);

        // Create the Response
        let finished_connection = create_response(&stream, request, &res_file_dir);
        if finished_connection {
            break;
        }
    }
}

fn read_request(mut stream: &TcpStream) -> Reqeuest {
    let mut buffer = [0; 1024];
    stream.read(&mut buffer).unwrap();
    println!("Request: {}", String::from_utf8_lossy(&buffer[..]));

    Reqeuest::new(&buffer)
}

fn create_response(
    mut stream: &TcpStream,
    request: Reqeuest,
    res_file_dir: &Option<String>,
) -> bool {
    // Check if the connection should be closed
    let finished_connection = request
        .headers
        .iter()
        .find(|header| header.contains_key("Connection"))
        .and_then(|header| header.get("Connection").cloned())
        .unwrap_or("".to_string())
        == "close";
    let path = request.uri.as_str();
    match path {
        "/" => {
            stream.write("HTTP/1.1 200 OK".as_bytes()).unwrap();
            if finished_connection {
                stream.write("\r\nConnection: close".as_bytes()).unwrap();
            }
            stream.write("\r\n\r\n".as_bytes()).unwrap();
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
                // gzip encoding
                let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
                encoder.write_all(sub_path.as_bytes()).unwrap();
                let compress_data = encoder.finish().unwrap();

                // If it does, return the response with gzip encoding
                let response = if finished_connection {
                    format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Encoding: gzip\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    compress_data.len()
                    )
                } else {
                    format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Encoding: gzip\r\nContent-Length: {}\r\n\r\n",
                    compress_data.len()
                    )
                };
                stream.write(response.as_bytes()).unwrap();
                stream.write(&compress_data).unwrap();
            } else {
                // If it doesn't, return the response without gzip encoding
                let response = if finished_connection {
                    format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    sub_path.len(),
                    sub_path
                    )
                } else {
                    format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
                    sub_path.len(),
                    sub_path
                    )
                };
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
            let response = if finished_connection {
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    user_agent.len(),
                    user_agent
                )
            } else {
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
                    user_agent.len(),
                    user_agent
                )
            };
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
                                    let response = if finished_connection {
                                        format!(
                                                "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                                                content.len(),
                                                String::from_utf8_lossy(&content)
                                            )
                                    } else {
                                        format!(
                                                "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\n\r\n{}",
                                                content.len(),
                                                String::from_utf8_lossy(&content)
                                            )
                                    };
                                    stream.write(response.as_bytes()).unwrap();
                                }
                                Err(_) => {
                                    if finished_connection {
                                        stream
                                            .write("HTTP/1.1 404 Not Found\r\nConnection: close\r\n\r\n".as_bytes())
                                            .unwrap();
                                    } else {
                                        stream
                                            .write("HTTP/1.1 404 Not Found\r\n\r\n".as_bytes())
                                            .unwrap();
                                    }
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

                                if finished_connection {
                                    stream
                                        .write(
                                            "HTTP/1.1 201 Created\r\nConnection: close\r\n\r\n"
                                                .as_bytes(),
                                        )
                                        .unwrap();
                                } else {
                                    stream
                                        .write("HTTP/1.1 201 Created\r\n\r\n".as_bytes())
                                        .unwrap();
                                }
                            } else {
                                // If the content type is not application/octet-stream, return 415
                                if finished_connection {
                                    stream
                                        .write(
                                            "HTTP/1.1 415 Unsupported Media Type\r\nConnection: close\r\n\r\n"
                                                .as_bytes(),
                                        )
                                        .unwrap();
                                } else {
                                    stream
                                        .write(
                                            "HTTP/1.1 415 Unsupported Media Type\r\n\r\n"
                                                .as_bytes(),
                                        )
                                        .unwrap();
                                }
                            }
                        }
                    }
                }
                None => {
                    if finished_connection {
                        stream
                            .write("HTTP/1.1 404 Not Found\r\nConnection: close\r\n\r\n".as_bytes())
                            .unwrap();
                    } else {
                        stream
                            .write("HTTP/1.1 404 Not Found\r\n\r\n".as_bytes())
                            .unwrap();
                    }
                }
            }
        }
        _ => {
            if finished_connection {
                stream
                    .write("HTTP/1.1 404 Not Found\r\nConnection: close\r\n\r\n".as_bytes())
                    .unwrap();
            } else {
                stream
                    .write("HTTP/1.1 404 Not Found\r\n\r\n".as_bytes())
                    .unwrap();
            }
        }
    }

    finished_connection
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{thread, vec};

    #[test]
    fn test_handle_connection_success() {
        let listener = start_local_server();
        let addr = listener.local_addr().unwrap();

        // Run Http Server
        let _ = thread::spawn(move || {
            if let Ok(stream) = listener.accept() {
                handle_connection(stream.0);
            }
        });

        // Create a test request (Client)
        let mut client_stream = TcpStream::connect(addr).unwrap();
        let request = "GET / HTTP/1.1\r\nHost: localhost:4221\r\n\r\n";
        client_stream.write(request.as_bytes()).unwrap();
        // Read the response
        let mut response = [0; 1024];
        let read_size = client_stream.read(&mut response).unwrap();
        let response_str = String::from_utf8_lossy(&response[..read_size]);

        assert_eq!(response_str, "HTTP/1.1 200 OK\r\n\r\n");
    }

    #[test]
    fn test_handle_connection_404() {
        let listener = start_local_server();
        let addr = listener.local_addr().unwrap();

        // Run Http Server
        let _ = thread::spawn(move || {
            if let Ok(stream) = listener.accept() {
                handle_connection(stream.0);
            }
        });

        // Create a test request (Client)
        let mut client_stream = TcpStream::connect(addr).unwrap();
        let request = "GET /abcdefg HTTP/1.1\r\nHost: localhost\r\n\r\n";
        client_stream.write(request.as_bytes()).unwrap();
        // Read the response
        let mut response = [0; 1024];
        let read_size = client_stream.read(&mut response).unwrap();
        let response_str = String::from_utf8_lossy(&response[..read_size]);

        assert_eq!(response_str, "HTTP/1.1 404 Not Found\r\n\r\n");
    }

    #[test]
    fn test_handle_connection_echo() {
        let listener = start_local_server();
        let addr = listener.local_addr().unwrap();

        // Run Http Server
        let _ = thread::spawn(move || {
            if let Ok(stream) = listener.accept() {
                handle_connection(stream.0);
            }
        });

        // Create a test request (Client)
        let mut client_stream = TcpStream::connect(addr).unwrap();
        let request = "GET /echo/abc HTTP/1.1\r\nHost: localhost\r\n\r\n";
        client_stream.write(request.as_bytes()).unwrap();
        // Read the response
        let mut response = [0; 1024];
        let read_size = client_stream.read(&mut response).unwrap();
        let response_str = String::from_utf8_lossy(&response[..read_size]);

        assert_eq!(
            response_str,
            "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 3\r\n\r\nabc"
        );
    }

    #[test]
    fn test_handle_connection_user_agent() {
        let listener = start_local_server();
        let addr = listener.local_addr().unwrap();

        // Run Http Server
        let _ = thread::spawn(move || {
            if let Ok(stream) = listener.accept() {
                handle_connection(stream.0);
            }
        });

        // Create a test request (Client)
        let mut client_stream = TcpStream::connect(addr).unwrap();
        let request =
            "GET /user-agent HTTP/1.1\r\nHost: localhost\r\nUser-Agent: foobar/1.2.3\r\n\r\n";
        client_stream.write(request.as_bytes()).unwrap();
        // Read the response
        let mut response = [0; 1024];
        let read_size = client_stream.read(&mut response).unwrap();
        let response_str = String::from_utf8_lossy(&response[..read_size]);

        assert_eq!(
            response_str,
            "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 12\r\n\r\nfoobar/1.2.3"
        );
    }

    #[test]
    fn test_handle_connection_files() {
        let listener = start_local_server();
        let addr = listener.local_addr().unwrap();

        // Run Http Server
        let _ = thread::spawn(move || {
            if let Ok(stream) = listener.accept() {
                let request = read_request(&stream.0);
                create_response(&stream.0, request, &Option::Some(String::from("/tmp")));
            }
        });

        // Create a file in the directory
        let mut file = fs::File::create("/tmp/foo").unwrap();
        file.write_all(&"Hello, World!".as_bytes()[..13]).unwrap();

        // Create a test request (Client)
        let mut client_stream = TcpStream::connect(addr).unwrap();
        let request = "GET /files/foo HTTP/1.1\r\nHost: localhost\r\n\r\n";
        client_stream.write(request.as_bytes()).unwrap();
        // Read the response
        let mut response = [0; 1024];
        let read_size = client_stream.read(&mut response).unwrap();
        let response_str = String::from_utf8_lossy(&response[..read_size]);
        // Clean up the file
        fs::remove_file("/tmp/foo").unwrap();

        assert_eq!(
            response_str,
            "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: 13\r\n\r\nHello, World!"
        );
    }

    #[test]
    fn test_handle_connection_files_404() {
        let listener = start_local_server();
        let addr = listener.local_addr().unwrap();

        // Run Http Server
        let _ = thread::spawn(move || {
            if let Ok(stream) = listener.accept() {
                let request = read_request(&stream.0);
                create_response(&stream.0, request, &Option::Some(String::from("/tmp")));
            }
        });

        // Create a test request (Client)
        let mut client_stream = TcpStream::connect(addr).unwrap();
        let request = "GET /files/non_existant_file HTTP/1.1\r\nHost: localhost\r\n\r\n";
        client_stream.write(request.as_bytes()).unwrap();
        // Read the response
        let mut response = [0; 1024];
        let read_size = client_stream.read(&mut response).unwrap();
        let response_str = String::from_utf8_lossy(&response[..read_size]);

        assert_eq!(response_str, "HTTP/1.1 404 Not Found\r\n\r\n");
    }

    #[test]
    fn test_handle_connection_read_body() {
        let listener = start_local_server();
        let addr = listener.local_addr().unwrap();

        // Run Http Server
        let _ = thread::spawn(move || {
            if let Ok(stream) = listener.accept() {
                let request = read_request(&stream.0);
                create_response(&stream.0, request, &Option::Some(String::from("/tmp")));
            }
        });

        // Create a test request (Client)
        let mut client_stream = TcpStream::connect(addr).unwrap();
        let request = "POST /files/file_123 HTTP/1.1\r\nHost: localhost\r\n\
                                        Content-Type: application/octet-stream\r\n\
                                        Content-Length: 5\r\n\r\n12345";
        client_stream.write(request.as_bytes()).unwrap();
        // Read the response
        let mut response = [0; 1024];
        let read_size = client_stream.read(&mut response).unwrap();
        let response_str = String::from_utf8_lossy(&response[..read_size]);

        assert_eq!(response_str, "HTTP/1.1 201 Created\r\n\r\n");

        // Check if the file was created
        let file_content = fs::read("/tmp/file_123").unwrap();
        assert_eq!(file_content, vec![49, 50, 51, 52, 53]);

        // Clean up the file
        fs::remove_file("/tmp/file_123").unwrap();
    }

    #[test]
    fn test_handle_connection_accept_encoding() {
        let listener = start_local_server();
        let addr = listener.local_addr().unwrap();

        // Run Http Server
        let _ = thread::spawn(move || {
            if let Ok(stream) = listener.accept() {
                handle_connection(stream.0);
            }
        });

        // Create a test request (Client)
        let mut client_stream = TcpStream::connect(addr).unwrap();
        let request = "GET /echo/abc HTTP/1.1\r\nHost: localhost\r\nAccept-Encoding: gzip\r\n\r\n";
        client_stream.write(request.as_bytes()).unwrap();
        // Read the response
        let mut response = [0; 1024];
        let read_size = client_stream.read(&mut response).unwrap();
        let response_str = String::from_utf8_lossy(&response[..read_size]);

        assert!(response_str.starts_with("HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Encoding: gzip\r\nContent-Length: 23\r\n\r\n"));

        // gzip encoding
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all("abc".as_bytes()).unwrap();
        let compress_data = encoder.finish().unwrap();
        // Check if the response ends with the compressed data
        assert!(response[..read_size].ends_with(&compress_data));
    }

    #[test]
    fn test_handle_connection_multiple_encoding() {
        let listener = start_local_server();
        let addr = listener.local_addr().unwrap();

        // Run Http Server
        let _ = thread::spawn(move || {
            if let Ok(stream) = listener.accept() {
                handle_connection(stream.0);
            }
        });

        // Create a test request (Client)
        let mut client_stream = TcpStream::connect(addr).unwrap();
        let request = "GET /echo/abc HTTP/1.1\r\nHost: localhost\r\nAccept-Encoding: invalid-encoding-1, gzip, invalid-encoding-2\r\n\r\n";
        client_stream.write(request.as_bytes()).unwrap();
        // Read the response
        let mut response = [0; 1024];
        let read_size = client_stream.read(&mut response).unwrap();
        let response_str = String::from_utf8_lossy(&response[..read_size]);

        assert!(response_str.starts_with("HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Encoding: gzip\r\nContent-Length: 23\r\n\r\n"));

        // gzip encoding
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all("abc".as_bytes()).unwrap();
        let compress_data = encoder.finish().unwrap();
        // Check if the response ends with the compressed data
        assert!(response[..read_size].ends_with(&compress_data));
    }

    #[test]
    fn test_handle_connection_invalid_encoding() {
        let listener = start_local_server();
        let addr = listener.local_addr().unwrap();

        // Run Http Server
        let _ = thread::spawn(move || {
            if let Ok(stream) = listener.accept() {
                handle_connection(stream.0);
            }
        });

        // Create a test request (Client)
        let mut client_stream = TcpStream::connect(addr).unwrap();
        let request = "GET /echo/abc HTTP/1.1\r\nHost: localhost\r\nAccept-Encoding: invalid-encoding-1, invalid-encoding-2\r\n\r\n";
        client_stream.write(request.as_bytes()).unwrap();
        // Read the response
        let mut response = [0; 1024];
        let read_size = client_stream.read(&mut response).unwrap();
        let response_str = String::from_utf8_lossy(&response[..read_size]);

        assert_eq!(
            response_str,
            "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 3\r\n\r\nabc"
        );
    }

    #[test]
    fn test_handle_connection_persistent() {
        let listener = start_local_server();
        let addr = listener.local_addr().unwrap();

        // Run Http Server
        let _ = thread::spawn(move || {
            if let Ok(stream) = listener.accept() {
                let request = read_request(&stream.0);
                assert!(create_response(&stream.0, request, &Option::None));
            }
        });

        // Create a test request (Client)
        let mut client_stream = TcpStream::connect(addr).unwrap();
        let request = "GET /echo/abc HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n";
        client_stream.write(request.as_bytes()).unwrap();
        // Read the response
        let mut response = [0; 1024];
        let read_size = client_stream.read(&mut response).unwrap();
        let response_str = String::from_utf8_lossy(&response[..read_size]);

        assert_eq!(
            response_str,
            "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 3\r\nConnection: close\r\n\r\nabc"
        );
    }

    fn start_local_server() -> TcpListener {
        // Port 0 means the OS will assign a free port
        TcpListener::bind("127.0.0.1:0").unwrap()
    }
}
