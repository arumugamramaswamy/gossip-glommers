# Chapter 1
## Day 1
- My goal is to write the solutions to these challenges in rust using tokio
- In doinng so, I aim to:
	1. Get better at rust
	2. Get more used to async rust
	3. Get better at writting and architecting distributed systems
- My first goal is to integrate with Maelstrom


### Integration with Maelstrom
```
Our first challenge is more of a “getting started” guide" to get the hang of working with Maelstrom in Go. In Maelstrom, we create a node which is a binary that receives JSON messages from STDIN and sends JSON messages to STDOUT. You can find a full protocol specification on the Maelstrom project.
```

Blah blah blah, hang of working with Maelstrom, blah blah blah, **create a binary that receives JSON messages from STDIN and sends JSON messages to STDOUT**.


#### Reading from STDIN

Okay, first step to get a minimal example using tokio that can read from STDIN.

```rust
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
```

Its getting late now, and I have work tomorrow, so I will sign off for now - until next time!
