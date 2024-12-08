use nix::unistd::mkfifo;
use nix::sys::stat::Mode;
use nix::errno::Errno;
use std::fs::OpenOptions;
use std::io::{Write, BufReader, BufRead};
use std::collections::HashMap;
use serde_json::Value;

const PIPE_REQUEST: &str = "/tmp/request-pipe-request";
const PIPE_RESPONSE: &str = "/tmp/request-pipe-response";

// Service 2 Functions
fn create_pipe(pipe_path: &str) -> Result<(), nix::Error> {
    match mkfifo(pipe_path, Mode::S_IRUSR | Mode::S_IWUSR | Mode::S_IRGRP | Mode::S_IWGRP) {
        Ok(_) => Ok(()), // Successfully created the pipe
        Err(e) if e == nix::Error::from(Errno::EEXIST) => {
            // The pipe already exists; proceed as normal
            // println!("Pipe already exists at {}", pipe_path);
            Ok(())
        }
        Err(e) => Err(e), // Propagate other errors
    }
}

fn setup_pipes() -> Result<(), nix::Error> {
    create_pipe(PIPE_REQUEST)?; // Pipe for sending requests
    create_pipe(PIPE_RESPONSE)?; // Pipe for receiving responses
    Ok(())
}

fn recv_request() -> Result<String, std::io::Error> {
    let request_pipe = OpenOptions::new()
        .read(true)
        .open(PIPE_REQUEST)?;
    let mut reader = BufReader::new(request_pipe);
    let mut request = String::new();
    reader.read_line(&mut request)?;
    Ok(request.trim().to_string())
}

fn send_response(response: &str) -> Result<(), std::io::Error> {
    let mut response_pipe = OpenOptions::new()
        .write(true)
        .open(PIPE_RESPONSE)?;
    writeln!(response_pipe, "{}", response)?;
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

    // Create pipes
    setup_pipes().expect("Failed to create pipes");

    // Process requests in a loop
    loop {
        let request = recv_request()?;
        // println!("Received request: {}", request);
        let response = process_request(request);
        send_response(&response)?;
    }
}