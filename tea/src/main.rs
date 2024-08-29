use tokio::io::{self, AsyncBufReadExt, BufReader};

#[tokio::main]
async fn main() {
    let stdin = io::stdin(); // Get the standard input handle
    let reader = BufReader::new(stdin); // Wrap it in a BufReader for efficiency
    let mut lines = reader.lines(); // Create an asynchronous line reader

    println!("Please enter some text:");

    // Read each line asynchronously
    while let Some(line) = lines.next_line().await.unwrap() {
        println!("You typed: {}", line);
    }
}
