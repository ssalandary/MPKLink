use shared_memory::{Shmem, ShmemConf, ShmemError};
use std::io::{Read, BufReader};
use std::fs::File;
use nix::fcntl::{Flock, FlockArg};

const SHMEM_REQUEST_FLINK: &str = "/tmp/request.shm";
const SHMEM_RESPONSE_FLINK: &str = "/tmp/response.shm";

// Service 1 Functions
fn grab_lock() -> Result<File, std::io::Error> {
    match File::open("/tmp/service.lock") {
        Ok(f) => Ok(f),
        // If the lock file does not exist, create it
        Err(_) => File::create("/tmp/service.lock"),
    }
}

fn create_shared_memory(flink_name: &str, length: usize) -> Result<Shmem, std::io::Error> {
    println!("Creating shared memory...");
    match ShmemConf::new().size(length).flink(flink_name).create() {
        Ok(m) => Ok(m),
        Err(ShmemError::LinkExists) => Ok(ShmemConf::new().flink(flink_name).open().unwrap()),
        Err(e) => Err(std::io::Error::new(std::io::ErrorKind::Other, e)),
    }
}

fn send_data(shmem: &Shmem, s: &str) -> Result<(), std::io::Error> {
    let raw_ptr = shmem.as_ptr();
    let writer = unsafe { std::slice::from_raw_parts_mut(raw_ptr, s.len()) };
    writer.copy_from_slice(s.as_bytes());
    Ok(())
}

fn recv_response(shmem: &Shmem) -> Result<String, std::io::Error> {
    let raw_ptr = shmem.as_ptr();
    let reader = unsafe { std::slice::from_raw_parts(raw_ptr, shmem.len()) };
    let mut buf_reader = BufReader::new(reader);
    let mut response = String::new();
    buf_reader.read_to_string(&mut response)?;
    Ok(response)
}

fn main() -> Result<(), std::io::Error> {
    println!("Starting request-manager...");

    let file = grab_lock()?;

    // Send a request
    let request = r#"{"type": "total", "string": "hello world hello"}"#;

    let shmem_request = create_shared_memory(SHMEM_REQUEST_FLINK, request.len())?;
    let lock = Flock::lock(file, FlockArg::LockExclusive).unwrap();
    send_data(&shmem_request, request)?;
    lock.unlock().unwrap();
    // Receive and print the response
    let shmem_response = create_shared_memory(SHMEM_RESPONSE_FLINK, 1024)?;
    let file2 = grab_lock()?;
    let lock2 = Flock::lock(file2, FlockArg::LockExclusive).unwrap();
    let response = recv_response(&shmem_response)?;
    lock2.unlock().unwrap();
    println!("Received response: {}", response);

    // Remove the lock file and shared memory
    std::fs::remove_file("/tmp/service.lock")?;
    std::fs::remove_file(SHMEM_REQUEST_FLINK)?;
    std::fs::remove_file(SHMEM_RESPONSE_FLINK)?;

    Ok(())
}

