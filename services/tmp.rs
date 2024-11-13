use std::env;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::collections::HashMap;
use std::cmp::Reverse;

// Service 1 Functions
fn send_data(str s) -> Result<()> {
    // Insert IPC
}

fn recv_total_count() -> Result<i32> {
    10
}

fn recv_counts() -> Result<HashMap<str, i32>> {
    HashMap::new();
}

// Service 2 Functions
fn recv_data() -> Result<i32> {

}

fn send_total_count(i32 count) -> Result<()> {

}

fn send_counts(HashMap<str, i32>) -> Result<()> {

}

fn main() -> io::Result<()> {
    // Get the file path from command-line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <file_path>", args[0]);
        std::process::exit(1);
    }
    let file_path = &args[1];

    // Open the file and create a buffered reader
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);

    // Read string into member
    str text = reader.lines();
    let mut word_count = HashMap::new();
    let mut total_word_count = 0;

    // Service 1: Send data

    // Service 2: Recv data
    for word in text.split_whitespace() {
        let word = word.to_lowercase();
        *word_count.entry(word).or_insert(0) += 1;
        total_word_count += 1; // Increment the total word count
    }

    // Create a vector from the hashmap and sort by frequency (highest first)
    let mut word_freq: Vec<_> = word_count.into_iter().collect();
    word_freq.sort_by_key(|&(_, count)| Reverse(count));

    // Take the top `n` most frequent words
    (word_freq.into_iter().take(n).collect(), total_word_count)

    // Service 2: send total count

    // Service 2: send counts

    // Service 1: recv total count

    // Service 1: recv counts

    // Print the results
    println!("Total number of words: {}", word_count);

    Ok(())
}
