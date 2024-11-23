use std::{env, io, fs::File};
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::cmp::Reverse;

fn recv_data() -> Result<(), io::Error> {
    // Pretend we're checking for the request somewhere...
    
    // request format: { "type": "total", "string": "hello world" }
    let request = r#"{ "type": "counts", "string": "hello world hello", "n": 2 }"#;

    // Parse the request (placeholder logic using a simple manual match)
    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(request) {
        let req_type = parsed["type"].as_str().unwrap_or_default();
        let s = parsed["string"].as_str().unwrap_or_default().to_string();
        
        match req_type {
            "total" => {
                let total_count = calculate_total_count(s)?;
                send_total_count(total_count)?;
            }
            "counts" => {
                let n = parsed["n"].as_i64().unwrap_or(0) as i32;
                let counts = calculate_counts(s, n)?;
                send_counts(counts)?;
            }
            _ => eprintln!("Unknown request type: {}", req_type),
        }
    } else {
        eprintln!("Failed to parse request.");
    }
    
    Ok(())
}

fn send_total_count(count: i32) -> Result<(), io::Error> {
    println!("Sending total count: {}", count);
    // Pretend we're sending count
    Ok(())
}

fn send_counts(counts: HashMap<String, i32>) -> Result<(), io::Error> {
    println!("Sending counts:");
    for (word, count) in &counts {
        println!("{}: {}", word, count);
    }
    // Sendin
    Ok(())
}

fn calculate_total_count(s: String) -> Result<i32, io::Error> {
    // Count the number of words in the string
    let total_count = s.split_whitespace().count() as i32;
    Ok(total_count)
}

fn calculate_counts(s: String, n: i32) -> Result<HashMap<String, i32>, io::Error> {
    let mut word_freq: HashMap<String, i32> = HashMap::new();

    // Count the frequency of each word
    for word in s.split_whitespace() {
        *word_freq.entry(word.to_string()).or_insert(0) += 1;
    }

    // Sort the words by frequency and return the top `n`
    let mut freq_vec: Vec<_> = word_freq.into_iter().collect();
    freq_vec.sort_by_key(|&(_, count)| Reverse(count));
    let top_n: HashMap<String, i32> = freq_vec.into_iter().take(n as usize).collect();

    Ok(top_n)
}

fn main() -> Result<(), io::Error> {
    println!("Service 2 is running...");

    // Simulate continuous polling for requests (placeholder loop)
    loop {
        recv_data()?;
        break;
    }

    Ok(())
}