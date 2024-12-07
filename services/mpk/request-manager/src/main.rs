use shared_memory::{Shmem, ShmemConf, ShmemError};
use libc::{ftruncate, shm_open};
use libc::{O_RDWR, O_CREAT, S_IRUSR, S_IWUSR, S_IRGRP, S_IWGRP};
use std::io::{Read, BufReader};
use std::ffi::CString;
use std::io;

use pkey_mprotect::*;

const SHMEM_REQUEST_FLINK: &str = "/request_mem";
const SHMEM_RESPONSE_FLINK: &str = "/response_mem";

const SHMEM_REQUESTMPK_FLINK: &str = "/request_mpk";
const SHMEM_RESPONSEMPK_FLINK: &str = "/response_mpk";

// Add separate region (1 bit) to check for mpk

// Service 1 Functions
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

fn recv_response(protected_region: &ProtectedRegion<&str>, mpkshmem: &Shmem) -> Result<String, std::io::Error> {
    let mut ready = false;
    let mut response = String::new();
    
    while !ready {
        ready = check_response_mpk(&mpkshmem).unwrap();
    }

    {
        // Lock the region
        let locked_region = protected_region.lock();
        // Read from the locked region
        response = locked_region.clone().to_string();
    }
    Ok(response)
}

fn main() -> Result<(), std::io::Error> {
    println!("Starting request-manager...");

    // Send a request
    let request = r#"{"type": "total", "string": "hello world hello"}"#;
    let shmem_request = create_shared_memory(SHMEM_REQUEST_FLINK, request.len())?;
    let shmem_request_mpk = create_shared_memory(SHMEM_REQUESTMPK_FLINK, 1)?;
    let shmem_response_mpk = create_shared_memory(SHMEM_RESPONSEMPK_FLINK, 1)?;

    send_data(&shmem_request, request, &shmem_request_mpk)?;

    // Response shenanigans:

    let shm_name = CString::new(SHMEM_RESPONSE_FLINK).expect("CString::new failed");
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

    let pkey = ProtectionKeys::new(true).unwrap();
    let protected_region = pkey.make_region_fd("test string", fd).unwrap();

    let response = recv_response(&protected_region, &shmem_response_mpk)?;
    println!("Received response: {}", response);

    Ok(())
}

