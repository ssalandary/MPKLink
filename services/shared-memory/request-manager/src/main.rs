use std::env;
use std::io::{Read, BufReader};
use shared_memory::{Shmem, ShmemConf, ShmemError};

const SHMEM_REQUEST_FLINK: &str = "/tmp/request.shm";
const SHMEM_RESPONSE_FLINK: &str = "/tmp/response.shm";

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

fn send_data(shmem: &Shmem, s: &str) -> Result<(), std::io::Error> {
    let raw_ptr = shmem.as_ptr();
    let writer = unsafe { std::slice::from_raw_parts_mut(raw_ptr, s.len() + 2) };

    // Add an "SB" to the beginning of the shared memory to indicate that the data is ready
    let metadata = [83, 66] as [u8; 2];
    let body = s.as_bytes() as &[u8];
    let data = [&metadata[..], &body].concat();
    // println!("Data: {:?}", data);

    // Copy the data into the shared memory
    writer.copy_from_slice(&data);

    // println!("Sent request: {:?}", s);

    Ok(())
}

fn recv_response(shmem: &Shmem) -> Result<String, std::io::Error> {
    let mut ready = false;
    let mut response = String::new();
    
    while !ready {
        response.clear();
        let raw_ptr = shmem.as_ptr();
        let reader = unsafe { std::slice::from_raw_parts(raw_ptr, shmem.len()) };
        let mut buf_reader = BufReader::new(reader);
        buf_reader.read_to_string(&mut response)?;
        
        // Read the first two bytes to check if the data is ready
        if response.len() > 2 && response.as_bytes()[0] == 83 && response.as_bytes()[1] == 66 {
            ready = true;
        }
    }

    Ok(response[2..].to_string().replace("\0", ""))
}

fn main() -> Result<(), std::io::Error> {
    let args = env::args().collect::<Vec<String>>();
    if args.len() != 2 {
        eprintln!("Usage: {} <file>", args[0]);
        std::process::exit(1);
    }

    let file = &args[1];
    if !std::path::Path::new(file).exists() {
        eprintln!("File does not exist: {}", file);
        std::process::exit(1);
    }

    let contents = std::fs::read_to_string(file)?;

    // println!("Starting request-manager...");

    // Send a request
    let request = r#"{"type": "total", "string": ""#.to_string() + &contents + r#""}"#;
    let shmem_request = create_shared_memory(SHMEM_REQUEST_FLINK, request.len())?;
    send_data(&shmem_request, request.as_str())?;

    // Receive and print the response
    while !std::path::Path::new(SHMEM_RESPONSE_FLINK).exists() {}
    let shmem_response = create_shared_memory(SHMEM_RESPONSE_FLINK, 1024)?;

    let response = recv_response(&shmem_response)?;
    println!("Received response: {}", response);

    Ok(())
}

