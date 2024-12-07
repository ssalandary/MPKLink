use shared_memory::{Shmem, ShmemConf, ShmemError};
use std::io::{Read, BufReader};
use pkey_mprotect::*;

const SHMEM_REQUEST_FLINK: &str = "/tmp/request.shm";
const SHMEM_RESPONSE_FLINK: &str = "/tmp/response.shm";

const SHMEM_REQUESTMPK_FLINK: &str = "/tmp/request_mpk.shm";
const SHMEM_RESPONSEMPK_FLINK: &str = "/tmp/response_mpk.shm";

// Add separate region (1 bit) to check for mpk

// Service 1 Functions
fn create_shared_memory(flink_name: &str, length: usize) -> Result<Shmem, std::io::Error> {
    // println!("Creating shared memory...");
    match ShmemConf::new().size(length).flink(flink_name).create() {
        Ok(m) => Ok(m),
        Err(ShmemError::LinkExists) => {
            println!("Opened link...");
            Ok(ShmemConf::new().flink(flink_name).open().unwrap())
        },
        Err(e) => Err(std::io::Error::new(std::io::ErrorKind::Other, e)),
    }
}

fn send_data(shmem: &Shmem, s: &str, mpkshmem: &Shmem) -> Result<(), std::io::Error> {
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

    
    println!("Sent request: {:?}", s);
    Ok(())
}

fn check_response_mpk(shmem: &Shmem) -> Result<bool, std::io::Error> {
    let mut response = String::new();
    response.clear();
    let raw_ptr = shmem.as_ptr();
    let reader = unsafe { std::slice::from_raw_parts(raw_ptr, shmem.len()) };
    let mut buf_reader = BufReader::new(reader);
    buf_reader.read_to_string(&mut response)?;
    
    // Read the first two bytes to check if the data is ready ("D" if ready.)
    if response.len() >= 1 && response.as_bytes()[0] == 68 {
        return Ok(true);
    }
    return Ok(false)
}

fn recv_response(shmem: &Shmem, mpkshmem: &Shmem) -> Result<String, std::io::Error> {
    let mut ready = false;
    let mut response = String::new();
    
    while !ready {
        ready = check_response_mpk(&mpkshmem).unwrap();
    }

    let raw_ptr = shmem.as_ptr();
    let reader = unsafe { std::slice::from_raw_parts(raw_ptr, shmem.len()) };
    let mut buf_reader = BufReader::new(reader);
    buf_reader.read_to_string(&mut response)?;

    Ok(response.to_string())
}

fn main() -> Result<(), std::io::Error> {
    println!("Starting request-manager...");

    // Send a request
    let request = r#"{"type": "total", "string": "hello world hello"}"#;
    let shmem_request = create_shared_memory(SHMEM_REQUEST_FLINK, request.len())?;
    let shmem_request_mpk = create_shared_memory(SHMEM_REQUESTMPK_FLINK, 1)?;
    let shmem_response_mpk = create_shared_memory(SHMEM_RESPONSEMPK_FLINK, 1)?;

    send_data(&shmem_request, request, &shmem_request_mpk)?;

    // Receive and print the response
    let shmem_response = create_shared_memory(SHMEM_RESPONSE_FLINK, 48)?;
    let response = recv_response(&shmem_response, &shmem_response_mpk)?;
    println!("Received response: {}", response);

    std::fs::remove_file(SHMEM_REQUEST_FLINK)?;
    std::fs::remove_file(SHMEM_RESPONSE_FLINK)?;
    std::fs::remove_file(SHMEM_REQUESTMPK_FLINK)?;
    std::fs::remove_file(SHMEM_RESPONSEMPK_FLINK)?;

    Ok(())
}

