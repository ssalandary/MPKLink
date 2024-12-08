use std::env;
use std::io;
use std::io::{Write, BufRead, BufReader};
use std::fs::OpenOptions;
use nix::unistd::mkfifo;
use nix::sys::stat::Mode;
use nix::errno::Errno;

const PIPE_REQUEST: &str = "/tmp/request-pipe-request";
const PIPE_RESPONSE: &str = "/tmp/request-pipe-response";

// Service 1 Functions
fn create_pipe(pipe_path: &str) -> Result<(), nix::Error> {
    match mkfifo(pipe_path, Mode::S_IRUSR | Mode::S_IWUSR | Mode::S_IRGRP | Mode::S_IWGRP) {
        Ok(_) => Ok(()), // Successfully created the pipe
        Err(e) if e == nix::Error::from(Errno::EEXIST) => {
            // The pipe already exists; proceed as normal
            // println!("Pipe already exists at {}", pipe_path);
            Ok(())
        }
        Err(e) => Err(e), // Propagate other errors
    }
}

fn setup_pipes() -> Result<(), nix::Error> {
    create_pipe(PIPE_REQUEST)?; // Pipe for sending requests
    create_pipe(PIPE_RESPONSE)?; // Pipe for receiving responses
    Ok(())
}

fn send_data(s: &str) -> Result<(), io::Error> {
    let mut request_pipe = OpenOptions::new()
        .write(true)
        .open(PIPE_REQUEST)?;
    writeln!(request_pipe, "{}", s)?;
    Ok(())
}

fn recv_response() -> Result<String, std::io::Error> {
    let response_pipe = OpenOptions::new()
        .read(true)
        .open(PIPE_RESPONSE)?;
    let mut reader = BufReader::new(response_pipe);
    let mut response = String::new();
    reader.read_line(&mut response)?;
    Ok(response.trim().to_string())
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

    // Create pipes
    setup_pipes().expect("Failed to create pipes");

    // Send a request
    let request = r#"{"type": "total", "string": ""#.to_string() + &contents + r#""}"#;
    // println!("Sending request: {}", request);
    send_data(request.as_str())?;

    // Receive and print the response
    let response = recv_response()?;
    println!("Received response: {}", response);

    Ok(())
}
