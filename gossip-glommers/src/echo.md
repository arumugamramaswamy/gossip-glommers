# Echo
## Introduction
This is my attempt at solving the [Gossip Glomers](https://fly.io/dist-sys/) challenges.

This series is heavily inspired by [this guy](https://www.youtube.com/watch?v=gboGyccRVXI) and [this guy](https://fasterthanli.me/).

## Day 1
- My goal is to write the solutions to these challenges in rust using tokio
- In doing so, I aim to:
	1. Get better at rust
	2. Get more used to async rust
	3. Get better at writting and architecting distributed systems
	4. Also get better at writing articles:)
- My first goal is to integrate with Maelstrom


### Integration with Maelstrom
<blockquote>

Our first challenge is more of a “getting started” guide to get the hang of working with Maelstrom in Go. In Maelstrom, we create a node which is a binary that receives JSON messages from STDIN and sends JSON messages to STDOUT. You can find a full protocol specification on the Maelstrom project.

</blockquote>


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


## Day 2

I'm back!

We're gonna continue to try integrating with Maelstrom. Let's get Maelstrom setup:

They need us to install openjdk to complete the setup. I'm in half a mind to set it up using docker. I'm gonna use docker only to run the tests and I'm gonna carry out development outside the container.

```
FROM rust:latest

RUN apt-get update && \
      apt-get -y install graphviz gnuplot wget default-jdk

RUN wget https://github.com/jepsen-io/maelstrom/releases/download/v0.2.3/maelstrom.tar.bz2
RUN tar -xf maelstrom.tar.bz2

ENTRYPOINT ["./maelstrom/maelstrom"]

```

Also, in order to make usage easier, I'm writing a makefile!

I did some quick googling and found `cargo-workspaces` allowing you to have multiple rust projects in a single workspace directory, pretty cool!

We can now mount the build directory of the workspace as a volume and voila! All the binaries will be available inside the container!
```
docker-run:
	cargo build --manifest-path=tea/Cargo.toml && \
		docker run --name maelstrom --rm -v ./tea/target/:/builds/ maelstrom-container \
		test -w echo --bin /builds/debug/echo --node-count 1 --time-limit 10

```


## Day 3
I realized that there is no easy way to retrieve the logs for a maelstrom node - so before going any further, we're gonna retrieve the logs from the container.

From some quick googling:

<blockquote>

You can find the STDERR logs for each node under:

```
$MAELSTROM_DIR/store/$WORKLOAD/$TIMESTAMP/node-logs
```

</blockquote>

It looks like this maps to `/store` in the container. Let's mount a volume to this dir so that the files are persisted.

```
docker-run:
	cargo build --manifest-path=tea/Cargo.toml && \
		docker run --name maelstrom --rm -v ./tea/target/:/builds/ -v ./debug-logs:/store/ maelstrom-container \
		test -w echo --bin /builds/debug/echo --node-count 1 --time-limit 10

```

This seems to work! Onwards!


#### Handling node init


When I run the test, it tells me that init has failed. From the docs:

<blockquote>

At the start of a test, Maelstrom issues a single init message to each node, like so:

```
{
  "type":     "init",
  "msg_id":   1,
  "node_id":  "n3",
  "node_ids": ["n1", "n2", "n3"]
}
```

...

In response to the init message, each node must respond with a message of type init_ok.

```
{
  "type":        "init_ok",
  "in_reply_to": 1
}
```

</blockquote>

Let's quickly hack this together so that we can see other messages!


```rust
#[tokio::main]
async fn main() {
    ...
    let line = lines.next_line().await.unwrap().unwrap();
    eprintln!("{line}"); // This is how we debug while working with maelstrom
    println!(r#"{{"type":"init_ok","in_reply_to": 1}}"#); // Init response
    // Read each line asynchronously
    while let Some(line) = lines.next_line().await.unwrap() {
        eprintln!("{line}"); // More debugging
    }
}

```


When we run this:

```terminal
clojure.lang.ExceptionInfo: Malformed network message. Node n0 tried to send the following message via STDOUT:

{:type "init_ok", :in_reply_to 1, :body {}}

This is malformed because:

{:src missing-required-key,
 :dest missing-required-key,
 :type disallowed-key,
 :in_reply_to disallowed-key}
```


Hmm, back to the protocol.

<blockquote>

Messages

Both STDIN and STDOUT messages are JSON objects, separated by newlines (\n). Each message object is of the form:

```
{
  "src":  A string identifying the node this message came from
  "dest": A string identifying the node this message is to
  "body": An object: the payload of the message
}
```

Message Bodies

RPC messages exchanged with Maelstrom's clients have bodies with the following reserved keys:

```
{
  "type":        (mandatory) A string identifying the type of message this is
  "msg_id":      (optional)  A unique integer identifier
  "in_reply_to": (optional)  For req/response, the msg_id of the request
}
```

</blockquote>

Ah, we seem to have to only sent the Message body, let's wrap it further.

```rust
    println!(r#"{{"src":"n0","dest":"c0","body":{{"type":"init_ok","in_reply_to": 1}}}}"#);
```


```terminal
INFO [2024-08-31 08:30:22,611] jepsen worker 0 - jepsen.util 0:invoke	:echo	"Please echo 96"
INFO [2024-08-31 08:30:27,613] jepsen worker 0 - jepsen.util 0:info	:echo	"Please echo 96"	:net-timeout
INFO [2024-08-31 08:30:27,614] jepsen worker 0 - jepsen.util 1:invoke	:echo	"Please echo 3"
INFO [2024-08-31 08:30:32,614] jepsen worker 0 - jepsen.util 1:info	:echo	"Please echo 3"	:net-timeout
```

Ah, a new error! Nice, our init function seems to have worked, great!


#### Actually parsing the inputs

So far we've just been hacking our setup to try to see what messages the nodes are receiving, in order to actually solve this challenge, we need to parse the input in order to respond correctly.

To achieve this we are going to use `serde` and `serde-json`.

First we declare the messages that will be sent and received.
```rust
#[derive(Serialize, Deserialize, Debug)]
struct Message<'a, MessageBody> {
    src: &'a str,
    dest: &'a str,
    body: MessageBody,
}

#[derive(Serialize, Deserialize, Debug)]
struct InitBody<'a> {
    #[serde(rename = "type")]
    typ: &'a str,
    msg_id: u32,
    node_id: &'a str,
    node_ids: Vec<&'a str>,
}

#[derive(Serialize, Deserialize, Debug)]
struct InitResponseBody<'a> {
    #[serde(rename = "type")]
    typ: &'a str,
    in_reply_to: u32,
}
```


Next, we edit the setup code a bit to send the init message using serde_json and not our old hacks
```rust
    let msg: Message<InitBody> = serde_json::from_str(&line).unwrap();
    let my_id = msg.body.node_id;
    eprintln!("{msg:?}");

    let resp = Message::<InitResponseBody> {
        dest: msg.src,
        src: my_id,
        body: InitResponseBody {
            typ: "init_ok",
            in_reply_to: msg.body.msg_id,
        },
    };
    println!("{}", serde_json::to_string(&resp).unwrap());
```

Now that we're back to where we started, let's handle incoming echo requests

#### Handling incoming echo requests

<blockquote>

Echo spec:
```
{
  "src": "c1",
  "dest": "n1",
  "body": {
    "type": "echo",
    "msg_id": 1,
    "echo": "Please echo 35"
  }
}
```

Echo Response spec:
```
{
  "src": "n1",
  "dest": "c1",
  "body": {
    "type": "echo_ok",
    "msg_id": 1,
    "in_reply_to": 1,
    "echo": "Please echo 35"
  }
}
```

</blockquote>

Translating these to structs:
```rust
#[derive(Serialize, Deserialize, Debug)]
struct EchoBody<'a> {
    #[serde(rename = "type")]
    typ: &'a str,
    msg_id: u32,
    echo: &'a str,
}

#[derive(Serialize, Deserialize, Debug)]
struct EchoResponseBody<'a> {
    #[serde(rename = "type")]
    typ: &'a str,
    msg_id: u32,
    in_reply_to: u32,
    echo: &'a str,
}
```

Rewriting our while loop:
```rust
    while let Some(line) = lines.next_line().await.unwrap() {
        let msg: Message<EchoBody> = serde_json::from_str(&line).unwrap();
        let resp = Message::<EchoResponseBody> {
            dest: msg.src,
            src: my_id,
            body: EchoResponseBody {
                typ: "echo_ok",
                in_reply_to: msg.body.msg_id,
                msg_id: {
                    let tmp = my_msg_id;
                    my_msg_id += 1;
                    tmp
                },
                echo: msg.body.echo,
            },
        };
        println!("{}", serde_json::to_string(&resp).unwrap());
        eprintln!("{line}");
    }
```

```terminal
Everything looks good! ?(??`)?
```
Great, that seems to do it!


Now that the challenge is out of the way, let's try improving our code.

#### Polishing up the API

Currently there is no "Interface" that we use to send and recieve messages. In order to send a message, we simply call `println` and in order to recieve a message, we request for the next line from STDIN.

My idea is to have an IO loop that handles receiving messages and sending responses. This then interfaces with a state machine using a channel.

In order to be able to use a single channel for multiple different kinds of messages we need to ensure that the `Message` type is not generic over `MessageBody`.

##### Serde to the rescue!

There's this really cool feature in Serde which is internally tagged enums.

By changing our message body structs to the following single enum, we can make the entire `Message` type non generic.
```rust
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
enum MessageBody<'a> {
    Init {
        msg_id: u32,
        node_id: &'a str,
        node_ids: Vec<&'a str>,
    },
    InitOk {
        in_reply_to: u32,
    },
    Echo {
        msg_id: u32,
        echo: &'a str,
    },
    EchoOk {
        in_reply_to: u32,
        msg_id: u32,
        echo: &'a str,
    },
}
```

While implementing this code, I realized that if we use any lifetime other than static, the code can't be used across multiple threads. My current idea is to replace all instances of `&'a str` with `String` - the only issue is that this will cause performance degradation due to too many allocations.

We can come back to making performance improvements later if necessary.


##### Adding a comms using channels

I decided to create a communication handler to handle communication in a loop and a communication client that can be used by the main loop to recieve and send messages.
```rust
fn create_comms<'a>() -> (CommunicationHandler, CommunicationClient) {
    let (sender, recver) = mpsc::channel(100);
    let (sender2, recver2) = mpsc::channel(100);

    let stdin = io::stdin(); // Get the standard input handle
    let reader = BufReader::new(stdin); // Wrap it in a BufReader for efficiency
    let lines = reader.lines(); // Create an asynchronous line reader
    (
        CommunicationHandler {
            msg_channel: sender,
            response_channel: recver2,
            stdin_lines: lines,
        },
        CommunicationClient {
            msg_channel: recver,
            response_channel: sender2,
        },
    )
}

struct CommunicationHandler {
    msg_channel: mpsc::Sender<Message>,
    response_channel: mpsc::Receiver<Message>,
    stdin_lines: Lines<BufReader<Stdin>>,
}


struct CommunicationClient {
    msg_channel: mpsc::Receiver<Message>,
    response_channel: mpsc::Sender<Message>,
}
```

### Wrapping up
After some refactoring, I finally got it all to work together!

```rust
use serde::{Deserialize, Serialize};
use serde_json;
use tokio::io::{self, AsyncBufReadExt, BufReader, Lines, Stdin};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() {
    let (comms_handler, mut comms_client) = create_comms();
    tokio::spawn(comms_handler.communication_loop());

    let mut my_msg_id = 0;
    let my_id;
    let msg = comms_client.msg_channel.recv().await.unwrap();

    match msg.body {
        MessageBody::Init {
            msg_id, node_id, ..
        } => {
            my_id = node_id;
            let resp = Message {
                dest: msg.src,
                src: my_id.clone(),
                body: MessageBody::InitOk {
                    in_reply_to: msg_id,
                },
            };
            let _ = comms_client.response_channel.send(resp).await;
        }
        _ => panic!("init message expected"),
    }

    // Read each line asynchronously
    while let Some(msg) = comms_client.msg_channel.recv().await {
        match msg.body {
            MessageBody::Echo { msg_id, echo } => {
                let resp = Message {
                    dest: msg.src,
                    src: my_id.clone(),
                    body: MessageBody::EchoOk {
                        in_reply_to: msg_id,
                        msg_id: {
                            let tmp = my_msg_id;
                            my_msg_id += 1;
                            tmp
                        },
                        echo,
                    },
                };
                let _ = comms_client.response_channel.send(resp).await;
            }
            _ => panic!("Only Echo messages expected"),
        }
    }
}

fn create_comms<'a>() -> (CommunicationHandler, CommunicationClient) {
    let (sender, recver) = mpsc::channel(100);
    let (sender2, recver2) = mpsc::channel(100);

    let stdin = io::stdin(); // Get the standard input handle
    let reader = BufReader::new(stdin); // Wrap it in a BufReader for efficiency
    let lines = reader.lines(); // Create an asynchronous line reader
    (
        CommunicationHandler {
            msg_channel: sender,
            response_channel: recver2,
            stdin_lines: lines,
        },
        CommunicationClient {
            msg_channel: recver,
            response_channel: sender2,
        },
    )
}

struct CommunicationHandler {
    msg_channel: mpsc::Sender<Message>,
    response_channel: mpsc::Receiver<Message>,
    stdin_lines: Lines<BufReader<Stdin>>,
}

impl<'a> CommunicationHandler {
    async fn communication_loop(mut self) {
        loop {
            tokio::select! {
            Some(resp) = self.response_channel.recv() => {
                eprintln!("Sending Message: {resp:?}");
                println!("{}", serde_json::to_string(&resp).unwrap()); // TODO: replace with
                    // tokio esq way of handling this
            },
            Ok(line) = self.stdin_lines.next_line() => {
                let line = line.unwrap();
                let msg: Message = serde_json::from_str(&line).unwrap();
                eprintln!("Recved Message: {msg:?}");
                let _ = self.msg_channel.send(msg).await;
            },
                else => break,
            }
        }
    }
}

struct CommunicationClient {
    msg_channel: mpsc::Receiver<Message>,
    response_channel: mpsc::Sender<Message>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Message {
    src: String,
    dest: String,
    body: MessageBody,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
enum MessageBody {
    Init {
        msg_id: u32,
        node_id: String,
        node_ids: Vec<String>,
    },
    InitOk {
        in_reply_to: u32,
    },
    Echo {
        msg_id: u32,
        echo: String,
    },
    EchoOk {
        in_reply_to: u32,
        msg_id: u32,
        echo: String,
    },
}
```
