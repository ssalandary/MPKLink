use std::os::unix::net::{UnixStream, UnixListener};
use std::io::{Write, BufReader, BufRead};
use std::collections::HashMap;
use serde_json::Value;

const UNIX_SOCKET: &str = "/tmp/service.sock";

// Service 2 Functions
fn create_socket(path: &str) -> Result<UnixListener, std::io::Error> {
    match UnixListener::bind(path) {
        Ok(listener) => {
            // println!("Socket created at path: {}", path);
            Ok(listener)
        }
        Err(e) => {
            match e.raw_os_error() {
                Some(48) => {
                    // println!("Socket already exists at path: {}", path);
                    std::fs::remove_file(path).expect("Failed to remove file");
                    create_socket(path)
                }
                _ => Err(e),
            }
        }
    }
}

fn recv_request(stream: &UnixStream) -> Result<String, std::io::Error> {
    let mut reader = BufReader::new(stream);
    let mut request = String::new();
    reader.read_line(&mut request)?;
    Ok(request)
}

fn send_response(mut stream: &UnixStream, response: &str) -> Result<(), std::io::Error> {
    stream.write_all(response.as_bytes())?;
    stream.write_all(b"\n")?; // Add newline to signal end of message
    Ok(())
}

fn process_request(request: String) -> String {
    match serde_json::from_str::<Value>(&request) {
        Ok(parsed) => {
            let req_type = parsed["type"].as_str().unwrap_or_default();
            let input = parsed["string"].as_str().unwrap_or_default();
            match req_type {
                "total" => {
                    let total_count = input.split_whitespace().count() as i32;
                    total_count.to_string()
                }
                "counts" => {
                    let mut counts: HashMap<String, i32> = HashMap::new();
                    for word in input.split_whitespace() {
                        *counts.entry(word.to_string()).or_insert(0) += 1;
                    }
                    serde_json::to_string(&counts).unwrap_or_default()
                }
                _ => "Unknown request type".to_string(),
            }
        }
        Err(_) => "Failed to parse request".to_string(),
    }
}

fn main() -> Result<(), std::io::Error> {
    // println!("Starting request-calculator...");

    let listener = create_socket(UNIX_SOCKET)?;
    let (stream, _) = listener.accept()?;

    // Process requests in a loop
    loop {
        let request = recv_request(&stream)?;
        // println!("Received request: {}", request);
        let response = process_request(request);
        send_response(&stream, &response)?;
    }
}