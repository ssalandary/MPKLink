use std::io::{Read, BufReader};
use std::collections::HashMap;
use serde_json::Value;

use shared_memory::{Shmem, ShmemConf, ShmemError};

const SHMEM_REQUEST_FLINK: &str = "/tmp/abc/request.shm";
const SHMEM_RESPONSE_FLINK: &str = "/tmp/abc/response.shm";

// Service 2 Functions
fn create_shared_memory(flink_name: &str, length: usize) -> Result<Shmem, std::io::Error> {
    println!("Creating shared memory...");
    match ShmemConf::new().size(length).flink(flink_name).create() {
        Ok(m) => Ok(m),
        Err(ShmemError::LinkExists) => Ok(ShmemConf::new().flink(flink_name).open().unwrap()),
        Err(e) => Err(std::io::Error::new(std::io::ErrorKind::Other, e)),
    }
}

fn recv_request(shmem: &Shmem) -> Result<String, std::io::Error> {
    let raw_ptr = shmem.as_ptr();
    let reader = unsafe { std::slice::from_raw_parts(raw_ptr, shmem.len()) };
    let mut buf_reader = BufReader::new(reader);
    let mut request = String::new();
    buf_reader.read_to_string(&mut request)?;
    Ok(request)
}

fn send_response(shmem: &Shmem, s: &str) -> Result<(), std::io::Error> {
    let raw_ptr = shmem.as_ptr();
    let writer = unsafe { std::slice::from_raw_parts_mut(raw_ptr, s.len()) };
    writer.copy_from_slice(s.as_bytes());
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

    let mut shmem_request = create_shared_memory(SHMEM_REQUEST_FLINK, 1024)?;
    while shmem_request.set_owner(true) { /* Spin lock */};
    println!("Received ownership of shared memory");
    let request = recv_request(&shmem_request)?;
    shmem_request.set_owner(false);
    println!("Received request: {}", request);
    let response = process_request(request);
    let mut shmem_response = create_shared_memory(SHMEM_RESPONSE_FLINK, response.len())?;
    shmem_response.set_owner(true);
    send_response(&shmem_response, &response)?;
    shmem_response.set_owner(false);
    Ok(())
}