use std::io::{Read, BufReader};
use std::collections::HashMap;
use serde_json::Value;
use shared_memory::{Shmem, ShmemConf, ShmemError};
use pkey_mprotect::*;

const SHMEM_REQUEST_FLINK: &str = "/tmp/request.shm";
const SHMEM_RESPONSE_FLINK: &str = "/tmp/response.shm";

const SHMEM_REQUESTMPK_FLINK: &str = "/tmp/request_mpk.shm";
const SHMEM_RESPONSEMPK_FLINK: &str = "/tmp/response_mpk.shm";

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

fn check_request_mpk(shmem: &Shmem) -> Result<bool, std::io::Error> {
    let mut request = String::new();
    let raw_ptr = shmem.as_ptr();
    let reader = unsafe { std::slice::from_raw_parts(raw_ptr, shmem.len()) };
    let mut buf_reader = BufReader::new(reader);
    buf_reader.read_to_string(&mut request)?;

    // Read the first two bytes to check if the data is ready ("D" if ready.)
    if request.len() >= 1 && request.as_bytes()[0] == 68 {
        return Ok(true);
    }
    return Ok(false)
}

fn recv_request(protected_region: &ProtectedRegion<String>, mpkshmem: &Shmem) -> Result<String, std::io::Error> {
    let mut ready = false;
    let mut request = String::new();

    while !ready {
        ready = check_request_mpk(&mpkshmem).unwrap();
    }
    
    {
        // Lock the region
        let locked_region = protected_region.lock();
        // Read from the locked region
        request = locked_region.clone();
    }

    Ok(request)
}

fn send_response(shmem: &Shmem, s: &str, mpkshmem: &Shmem) -> Result<(), std::io::Error> {
    let raw_ptr = shmem.as_ptr();
    let writer = unsafe { std::slice::from_raw_parts_mut(raw_ptr, s.len()) };
    // write "N" to mpkshmem to indicate not ready
    let mpk_raw_ptr = mpkshmem.as_ptr();
    let mpk_writer = unsafe { std::slice::from_raw_parts_mut(mpk_raw_ptr, 1) };
    let metadata = [78] as [u8; 1];
    mpk_writer.copy_from_slice(&metadata);

    // Write data
    let body = s.as_bytes() as &[u8];
    let data = body;
    println!("Data: {:?}", data);

    // Copy the data into the shared memory
    writer.copy_from_slice(&data);

    // write "D" to mpkshmem to indicate ready.
    let new_metadata = [68] as [u8; 1];
    mpk_writer.copy_from_slice(&new_metadata);

    
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

    let shmem_request = create_shared_memory(SHMEM_REQUEST_FLINK, 4096)?;
    let shmem_request_mpk = create_shared_memory(SHMEM_REQUESTMPK_FLINK, 1)?; // read only

    let shared_memory_ptr = shmem_request.as_ptr() as *mut libc::c_void;
    let pkey = ProtectionKeys::new(false).unwrap();
    let protected_region = ProtectedRegion::<String>::from_shared_memory(&pkey, shared_memory_ptr, 4096).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    let request = recv_request(&protected_region, &shmem_request_mpk)?;
    println!("Received request: {}", request);
    let response = process_request(request);
    let shmem_response = create_shared_memory(SHMEM_RESPONSE_FLINK, 4096)?;
    
    
    let shmem_response_mpk = create_shared_memory(SHMEM_RESPONSEMPK_FLINK, 1)?; // write to when ready
    send_response(&shmem_response, &response, &shmem_response_mpk)?;

    Ok(())
}