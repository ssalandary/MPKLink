use std::thread;
use shared_memory::{Shmem, ShmemConf, ShmemError};
use std::io::{Read, BufReader};
use std::io;
use std::env;
use std::path::Path;
use std::fs;
use std::boxed::{Box};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use serde_json::Value;

use pkey_mprotect::*;


const SHMEM_REQUESTMPK_FLINK: &str = "/request_mpk";
const SHMEM_RESPONSEMPK_FLINK: &str = "/response_mpk";

// Both
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

// Request Calculator
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

// Request Manager
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

// Request Calculator
fn recv_request(protected_region: Arc<Mutex<Arc<ProtectedRegion<&str>>>>, mpkshmem: &Shmem) -> Result<String, std::io::Error> {
    let mut ready = false;
    let mut request = String::new();

    while !ready {
        ready = check_request_mpk(&mpkshmem).unwrap();
    }

    let mut read_region = protected_region.lock().unwrap();
    {
        // Lock the region
        let locked_region = read_region.lock();
        // Read from the locked region
        request = locked_region.clone().to_string();
    }

    Ok(request)
}

//Request Manager
fn recv_response(protected_region: Arc<Mutex<Arc<ProtectedRegion<&str>>>>, mpkshmem: &Shmem) -> Result<String, std::io::Error> {
    let mut ready = false;
    let mut response = String::new();
    
    while !ready {
        ready = check_response_mpk(&mpkshmem).unwrap();
    }

    let mut read_region = protected_region.lock().unwrap();
    {
        // Lock the region
        let locked_region = read_region.lock();
        // Read from the locked region
        response = locked_region.clone().to_string();
    }
    Ok(response)
}

//Request Calculator
fn send_response(s_calc_write_region: Arc<Mutex<Arc<ProtectedRegion<&str>>>>, s: &str, mpkshmem: &Shmem) -> Result<(), std::io::Error> {
    // write "N" to mpkshmem to indicate not ready
    let mpk_raw_ptr = mpkshmem.as_ptr();
    let mpk_writer = unsafe { std::slice::from_raw_parts_mut(mpk_raw_ptr, 1) };
    let metadata = [78] as [u8; 1];
    mpk_writer.copy_from_slice(&metadata);

    // Write data
    let mut write_region = s_calc_write_region.lock().unwrap();
    write_region.modify(s);

    // write "D" to mpkshmem to indicate ready.
    let new_metadata = [68] as [u8; 1];
    mpk_writer.copy_from_slice(&new_metadata);

    
    println!("Sent response: {:?}", s);
    Ok(())
}

// Request Manager
fn send_data(s_man_write_region: Arc<Mutex<Arc<ProtectedRegion<&str>>>>, s: &str, mpkshmem: &Shmem) -> Result<(), std::io::Error> {
    // write "N" to mpkshmem to indicate not ready
    let mpk_raw_ptr = mpkshmem.as_ptr();
    let mpk_writer = unsafe { std::slice::from_raw_parts_mut(mpk_raw_ptr, 1) };
    let metadata = [78] as [u8; 1];
    mpk_writer.copy_from_slice(&metadata);

    // Write data
    let mut write_region = s_man_write_region.lock().unwrap();
    write_region.modify(s);

    // write "D" to mpkshmem to indicate ready.
    let new_metadata = [68] as [u8; 1];
    mpk_writer.copy_from_slice(&new_metadata);

    
    println!("Sent request: {:?}", s);
    Ok(())
}

// Request Calculator
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

fn request_manager(s_calc_write_region: Arc<Mutex<Arc<ProtectedRegion<&str>>>>, s_man_write_region: Arc<Mutex<Arc<ProtectedRegion<&str>>>>) -> Result<(), std::io::Error> {
    println!("Starting request-manager...");
    let shmem_request_mpk = create_shared_memory(SHMEM_REQUESTMPK_FLINK, 1)?;
    let shmem_response_mpk = create_shared_memory(SHMEM_RESPONSEMPK_FLINK, 1)?;

    let args = env::args().collect::<Vec<String>>();
    if args.len() != 2 {
        eprintln!("Usage: {} <file>", args[0]);
        std::process::exit(1);
    }

    let file_path = &args[1];
    if !Path::new(file_path).exists() {
        eprintln!("File does not exist: {}", file_path);
        std::process::exit(1);
    }

    // Read file contents
    let mut contents = fs::read_to_string(file_path)?;
    contents = contents.replace('\n', " ");

    // Create the request in the format {"type": "total", "string": "<file contents>"}
    let request = r#"{"type": "total", "string": ""#.to_string() + &contents + r#""}"#;

    send_data(s_man_write_region, &request, &shmem_request_mpk)?;

    let mut response = recv_response(s_calc_write_region, &shmem_response_mpk)?;
    if let Some(pos) = response.find("   ") {
        response = response[pos + 3..].to_string();
    }
    response = response.trim_start().to_string();
    
    println!("Received response: {}", response);

    Ok(())
}

fn request_calculator(s_calc_write_region: Arc<Mutex<Arc<ProtectedRegion<&str>>>>, s_man_write_region: Arc<Mutex<Arc<ProtectedRegion<&str>>>>) -> Result<(), std::io::Error> {
    println!("Starting request-calculator...");
    let shmem_request_mpk = create_shared_memory(SHMEM_REQUESTMPK_FLINK, 1)?;
    let shmem_response_mpk = create_shared_memory(SHMEM_RESPONSEMPK_FLINK, 1)?;
    
    let request = recv_request(s_man_write_region, &shmem_request_mpk)?;
    println!("Received request: {}", request);
    let mut response = process_request(request);

    response = format!("{:20}{}", "", response);  // the front is corrupted so if i add some spaces, we are good

    send_response(s_calc_write_region, &response, &shmem_response_mpk)?;

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let testa = Box::new("test string".to_string());
    let request = Box::new(r#"{"type": "total", "string": "hello world hello"}"#.to_string());
    let testa_static: &'static str = Box::leak(testa);
    let request_static: &'static str = Box::leak(request);


    let calc_write_pkey = ProtectionKeys::new(false).unwrap();
    let calc_write_region = calc_write_pkey.make_region(testa_static).unwrap(); // where the response goes

    let man_write_pkey = ProtectionKeys::new(false).unwrap();
    let man_write_region = man_write_pkey.make_region(request_static).unwrap(); // where the request goes.

    let s_calc_write_region = Arc::new(Mutex::new(calc_write_region));
    let s_man_write_region = Arc::new(Mutex::new(man_write_region));

    // Spawn threads
    let manager_handle = {
        let s_calc_write_region = s_calc_write_region.clone();
        let s_man_write_region = s_man_write_region.clone();
        std::thread::spawn(move || {
            request_manager(s_calc_write_region, s_man_write_region);
        })
    };

    let calculator_handle = {
        let s_calc_write_region = s_calc_write_region.clone();
        let s_man_write_region = s_man_write_region.clone();
        std::thread::spawn(move || {
            request_calculator(s_calc_write_region, s_man_write_region);
        })
    };

    // Wait for threads to finish
    manager_handle.join().unwrap();
    calculator_handle.join().unwrap();

    Ok(())
}
