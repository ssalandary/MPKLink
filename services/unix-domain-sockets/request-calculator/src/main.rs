use std::os::unix::net::{UnixStream,UnixListener};
use std::io::{Write, BufReader, BufRead};
use std::collections::HashMap;
use serde_json::Value;

const UNIX_SOCKET_REQUEST: &str = "/tmp/request.sock";
const UNIX_SOCKET_RESPONSE: &str = "/tmp/response.sock";

fn setup_sockets() -> Result<(), std::io::Error> {
    std::fs::remove_file(UNIX_SOCKET_REQUEST).ok();
    std::fs::remove_file(UNIX_SOCKET_RESPONSE).ok();
    Ok(())
}

fn recv_request() -> Result<String, std::io::Error> {
    let listener = UnixListener::bind(UNIX_SOCKET_REQUEST)?;
    let (stream, _) = listener.accept()?;
    let mut reader = BufReader::new(stream);
    let mut request = String::new();
    reader.read_line(&mut request)?;
    Ok(request.trim().to_string())
}

fn send_response(response: &str) -> Result<(), std::io::Error> {
    let mut stream = UnixStream::connect(UNIX_SOCKET_RESPONSE)?;
    writeln!(stream, "{}", response)?;
    stream.write_all(response.as_bytes())?;
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
    println!("Starting request-calculator...");

    // Create sockets
    setup_sockets().expect("Failed to create sockets");

    // Process requests in a loop
    loop {
        let request = recv_request()?;
        println!("Received request: {}", request);
        let response = process_request(request);
        send_response(&response)?;
    }
}