use std::env;
use std::io::{Write, BufRead, BufReader};
use std::os::unix::net::UnixStream;

const UNIX_SOCKET: &str = "/tmp/service.sock";

// Service 1 Functions

fn send_data(mut stream: &UnixStream, s: &str) -> Result<(), std::io::Error> {
    stream.write_all(s.as_bytes())?;
    stream.write_all(b"\n")?; // Add newline to signal end of message
    // println!("Sent request: {}", s);
    Ok(())
}

fn recv_response(stream: &UnixStream) -> Result<String, std::io::Error> {
    let mut reader = BufReader::new(stream);
    let mut response = String::new();
    reader.read_line(&mut response)?;
    Ok(response)
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

    // https://stackoverflow.com/questions/60558757/how-to-send-and-listen-to-data-via-unix-sockets-in-rust
    let stream = UnixStream::connect(UNIX_SOCKET)?;
    // println!("Connected to socket: {}", UNIX_SOCKET);

    // Send a request
    let request = r#"{"type": "total", "string": ""#.to_string() + &contents + r#""}"#;
    send_data(&stream, request.as_str())?;

    // Receive and print the response
    let response = recv_response(&stream)?;
    println!("Received response: {}", response);

    Ok(())
}

