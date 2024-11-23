use std::{env, io, fs::File};
use std::io::{BufRead, BufReader};
use std::collections::HashMap;

// Service 1 Functions
fn send_data(s: &str) -> Result<(), io::Error> {
    // For now, simply print the data that would be sent.
    println!("Sending data: {}", s);
    Ok(())
}

fn recv_total_count() -> Result<i32, io::Error> {
    println!("Received total count.");
    Ok(42) //  placeholder value
}

fn recv_counts() -> Result<HashMap<String, i32>, io::Error> {
    // Placeholder: Print something and return an empty HashMap for now.
    println!("Received counts.");
    Ok(HashMap::new())
}

fn main() -> Result<(), io::Error> {
    // Collect command-line arguments
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <string or file path>", args[0]);
        std::process::exit(1);
    }

    let input = &args[1];
    let data = if let Ok(file) = File::open(input) { // take in file.
        let reader = BufReader::new(file);
        let mut content = String::new();
        for line in reader.lines() {
            content.push_str(&line?);
            content.push('\n');
        }
        content
    } else {
        input.clone() // take in string
    };

    // Call send_data with the data
    send_data(&data)?;

    // recv functions to be dealt with.
    // Simulate continuous polling for requests (placeholder loop)
    loop {
        recv_data()?;
        break;
    }

    Ok(())
}

