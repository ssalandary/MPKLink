use std::os::unix::net::{UnixStream,UnixListener};
use std::io::{Write, BufRead, BufReader};

const UNIX_SOCKET_REQUEST: &str = "/tmp/request.sock";
const UNIX_SOCKET_RESPONSE: &str = "/tmp/response.sock";

// Service 1 Functions
fn setup_sockets() -> Result<(), std::io::Error> {
    std::fs::remove_file(UNIX_SOCKET_REQUEST).ok();
    std::fs::remove_file(UNIX_SOCKET_RESPONSE).ok();
    Ok(())
}

fn send_data(s: &str) -> Result<(), std::io::Error> {
    // https://stackoverflow.com/questions/60558757/how-to-send-and-listen-to-data-via-unix-sockets-in-rust
    let listener = UnixListener::bind(UNIX_SOCKET_REQUEST)?;

    match listener.accept() {
        Ok((mut stream, _)) => {
            println!("Connection established.");
            stream.write_all(s.as_bytes())?;
        }
        Err(e) => eprintln!("Error accepting connection: {}", e),
    }

    Ok(())
}

fn recv_response() -> Result<String, std::io::Error> {
    let stream = UnixStream::connect(UNIX_SOCKET_RESPONSE)?;
    let mut reader = BufReader::new(stream);
    let mut response = String::new();
    reader.read_line(&mut response)?;
    Ok(response.trim().to_string())
}

fn main() -> Result<(), std::io::Error> {
    println!("Starting request-manager...");

    // Create sockets
    setup_sockets().expect("Failed to create sockets");

    // Send a request
    let request = r#"{"type": "total", "string": "hello world hello"}"#;
    send_data(request)?;

    // Receive and print the response
    let response = recv_response()?;
    println!("Received response: {}", response);

    Ok(())
}

