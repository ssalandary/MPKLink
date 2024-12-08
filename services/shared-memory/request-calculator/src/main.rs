use std::io::{Read, BufReader};
use std::collections::HashMap;
use serde_json::Value;
use shared_memory::{Shmem, ShmemConf, ShmemError};

const SHMEM_REQUEST_FLINK: &str = "/tmp/request.shm";
const SHMEM_RESPONSE_FLINK: &str = "/tmp/response.shm";

// Service 2 Functions
fn create_shared_memory(flink_name: &str, length: usize) -> Result<Shmem, std::io::Error> {
    match ShmemConf::new().size(length).flink(flink_name).create() {
        Ok(m) => Ok(m),
        Err(ShmemError::LinkExists) => {
            println!("Opened link...");
            Ok(ShmemConf::new().flink(flink_name).open().unwrap())
        },
        Err(e) => Err(std::io::Error::new(std::io::ErrorKind::Other, e)),
    }
}

fn recv_request(shmem: &Shmem) -> Result<String, std::io::Error> {
    let mut ready = false;
    let mut request = String::new();

    while !ready {
        let raw_ptr = shmem.as_ptr();
        let reader = unsafe { std::slice::from_raw_parts(raw_ptr, shmem.len()) };
        let mut buf_reader = BufReader::new(reader);
        buf_reader.read_to_string(&mut request)?;

        // Read the first two bytes to check if the data is ready
        if request.len() > 2 && request.as_bytes()[0] == 83 && request.as_bytes()[1] == 66 {
            ready = true;
        }
    }

    Ok(request[2..].to_string().replace("\0", "") + r#""}"#)
}

fn send_response(shmem: &Shmem, s: &str) -> Result<(), std::io::Error> {
    let raw_ptr = shmem.as_ptr();
    let writer = unsafe { std::slice::from_raw_parts_mut(raw_ptr, s.len() + 2) };
    
    // Add an "SB" to the beginning of the shared memory to indicate that the data is ready
    let metadata = [83, 66] as [u8; 2];
    let body = s.as_bytes() as &[u8];
    let data = [&metadata[..], &body].concat();
    println!("Data: {:?}", data);

    // Copy the data into the shared memory
    writer.copy_from_slice(&data);

    println!("Sent response: {:?}", s);
    Ok(())
}

fn process_request(request: String) -> String {
    println!("Processing request: {}", request);
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
        Err(e) => e.to_string(),
    }
}

fn main() -> Result<(), std::io::Error> {
    println!("Starting request-calculator...");

    while !std::path::Path::new(SHMEM_REQUEST_FLINK).exists() {}
    let shmem_request = create_shared_memory(SHMEM_REQUEST_FLINK, 48)?;
    let request = recv_request(&shmem_request)?;
    println!("Received request: {}", request);
    // Process request minus first two bytes
    let response = process_request(request);
    let shmem_response = create_shared_memory(SHMEM_RESPONSE_FLINK, response.len() + 5)?;
    send_response(&shmem_response, &response)?;

    std::fs::remove_file(SHMEM_REQUEST_FLINK)?;

    Ok(())
}