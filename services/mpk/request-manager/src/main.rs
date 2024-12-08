use shared_memory::{Shmem, ShmemConf, ShmemError};
use libc::{ftruncate, shm_open};
use libc::{O_RDWR, O_CREAT, S_IRUSR, S_IWUSR, S_IRGRP, S_IWGRP, MAP_SHARED, PROT_READ, PROT_WRITE};
use std::io::{Read, BufReader};
use std::ffi::{CString, CStr};
use std::io;
use std::ptr;
use std::slice;

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

fn send_data(protected_region: &ProtectedRegion<&String>, s: &String, mpkshmem: &Shmem) -> Result<(), std::io::Error> {
    // write "N" to mpkshmem to indicate not ready
    let mpk_raw_ptr = mpkshmem.as_ptr();
    let mpk_writer = unsafe { std::slice::from_raw_parts_mut(mpk_raw_ptr, 1) };
    let metadata = [78] as [u8; 1];
    mpk_writer.copy_from_slice(&metadata);

    // Write data
    protected_region.modify(s);

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

fn recv_response(protected_region: &ProtectedRegion<&String>, mpkshmem: &Shmem) -> Result<String, std::io::Error> {
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
    let shmem_request_mpk = create_shared_memory(SHMEM_REQUESTMPK_FLINK, 1)?;
    let shmem_response_mpk = create_shared_memory(SHMEM_RESPONSEMPK_FLINK, 1)?;

    // Send a request
    let request = r#"{"type": "total", "string": "hello world hello"}"#;
    let shmem_request = create_shared_memory(SHMEM_REQUEST_FLINK, request.len())?;

    let req_shm_name = CString::new(SHMEM_REQUEST_FLINK).expect("CString::new failed");
    let shm_size: libc::size_t = 4096;

    // Create and open the shared memory object
    let ffd = unsafe {
        shm_open(
            req_shm_name.as_ptr(),
            O_CREAT | O_RDWR,
            S_IRUSR | S_IWUSR | S_IRGRP | S_IWGRP,
        )
    };

    if ffd == -1 {
        return Err(io::Error::last_os_error());
    }
    
    // Resize the shared memory segment
    let res = unsafe { ftruncate(ffd, shm_size as libc::off_t) };
    if res == -1 {
        return Err(io::Error::last_os_error());
    }

    let pkey = ProtectionKeys::new(false).unwrap();
    let req = request.to_string();
    let protected_region = pkey.make_region_fd(&req, ffd).unwrap();
    send_data(&protected_region, &req, &shmem_request_mpk)?;

    // Response shenanigans:
    let shm_name = CString::new(SHMEM_RESPONSE_FLINK).expect("CString::new failed");

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

    
    let real_pkey = ProtectionKeys::new(false).unwrap();
    let test = "test string".to_string();
    let real_protected_region = real_pkey.make_region_fd(&test, fd).unwrap();

    let response = recv_response(&real_protected_region, &shmem_response_mpk)?;
    println!("Received response: {}", response);

    std::fs::remove_file("/dev/shm".to_owned() + SHMEM_REQUEST_FLINK)?;
    std::fs::remove_file("/dev/shm".to_owned() +  SHMEM_RESPONSE_FLINK)?;
    std::fs::remove_file("/dev/shm".to_owned()+ SHMEM_REQUESTMPK_FLINK)?;
    std::fs::remove_file("/dev/shm".to_owned() +  SHMEM_RESPONSEMPK_FLINK)?;

    Ok(())
}

