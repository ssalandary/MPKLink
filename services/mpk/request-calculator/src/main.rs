use shared_memory::{Shmem, ShmemConf, ShmemError};
use libc::{ftruncate, shm_open};
use libc::{O_RDWR, O_CREAT, S_IRUSR, S_IWUSR, S_IRGRP, S_IWGRP};
use std::io::{Read, BufReader};
use std::ffi::CString;
use std::io;
use std::collections::HashMap;
use serde_json::Value;

use pkey_mprotect::*;

const SHMEM_REQUEST_FLINK: &str = "/request_mem";
const SHMEM_RESPONSE_FLINK: &str = "/response_mem";

const SHMEM_REQUESTMPK_FLINK: &str = "/request_mpk";
const SHMEM_RESPONSEMPK_FLINK: &str = "/response_mpk";

// Service 2 Functions
fn create_shared_memory(id: &str, length: usize, ) -> Result<Shmem, std::io::Error> {
    // println!("Creating shared memory...");
    match ShmemConf::new().size(length).os_id(id).create() {
        Ok(m) => Ok(m),
        Err(ShmemError::MappingIdExists) => {
            println!("Opened mapping ID...");
            Ok(ShmemConf::new().os_id(id).open().unwrap())
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

fn recv_request(protected_region: &ProtectedRegion<&str>, mpkshmem: &Shmem) -> Result<String, std::io::Error> {
    let mut ready = false;
    let mut request = String::new();

    while !ready {
        ready = check_request_mpk(&mpkshmem).unwrap();
    }
    
    {
        // Lock the region
        let locked_region = protected_region.lock();
        // Read from the locked region
        request = locked_region.clone().to_string();
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

    let shmem_request_mpk = create_shared_memory(SHMEM_REQUESTMPK_FLINK, 1)?; // read only

    let shm_name = CString::new(SHMEM_REQUEST_FLINK).expect("CString::new failed");
    let shm_size: libc::size_t = 4096;

    // Create and open the shared memory object
    let fd = unsafe {
        shm_open(
            shm_name.as_ptr(),
            O_CREAT | O_RDWR,
            S_IRUSR | S_IWUSR | S_IRGRP | S_IWGRP,
        )
    };

    if fd == -1 {
        return Err(io::Error::last_os_error());
    }
    
    // Resize the shared memory segment
    let result = unsafe { ftruncate(fd, shm_size as libc::off_t) };
    if result == -1 {
        return Err(io::Error::last_os_error());
    }

    let pkey = ProtectionKeys::new(false).unwrap();
    let protected_region = pkey.make_region_fd("test string", fd).unwrap();
    
    let request = recv_request(&protected_region, &shmem_request_mpk)?;
    println!("Received request: {}", request);
    let response = process_request(request);
    let shmem_response = create_shared_memory(SHMEM_RESPONSE_FLINK, 4096)?;
    
    
    let shmem_response_mpk = create_shared_memory(SHMEM_RESPONSEMPK_FLINK, 1)?; // write to when ready
    send_response(&shmem_response, &response, &shmem_response_mpk)?;

    Ok(())
}