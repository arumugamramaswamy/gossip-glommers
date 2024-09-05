use std::collections::{HashMap, HashSet};

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
    let mut next_gen_id;
    let n_nodes;
    let msg = comms_client.msg_channel.recv().await.unwrap();

    match msg.body {
        MessageBody::Init {
            msg_id,
            node_id,
            node_ids,
        } => {
            my_id = node_id;
            n_nodes = node_ids.len();
            next_gen_id = node_ids.iter().position(|s| *s == my_id).unwrap() + 1;

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

    let msg = comms_client.msg_channel.recv().await.unwrap();
    let topology;
    match msg.body {
        MessageBody::Topology {
            msg_id,
            topology: t,
        } => {
            topology = t;
            let resp = Message {
                dest: msg.src,
                src: my_id.clone(),
                body: MessageBody::TopologyOk {
                    in_reply_to: msg_id,
                },
            };
            let _ = comms_client.response_channel.send(resp).await;
        }
        _ => panic!("init message expected"),
    }

    let mut messages = HashSet::new();

    // Read each line asynchronously
    while let Some(msg) = comms_client.msg_channel.recv().await {
        match msg.body {
            MessageBody::Gossip { message, .. } => {
                if messages.insert(message) {
                    for neighbour in topology.get(&my_id).unwrap().iter() {
                        let gossip = Message {
                            dest: neighbour.to_string(),
                            src: my_id.clone(),
                            body: MessageBody::Gossip { message },
                        };
                        let _ = comms_client.response_channel.send(gossip).await;
                    }
                }
            }
            MessageBody::Broadcast { msg_id, message } => {
                if messages.insert(message) {
                    for neighbour in topology.get(&my_id).unwrap().iter() {
                        let gossip = Message {
                            dest: neighbour.to_string(),
                            src: my_id.clone(),
                            body: MessageBody::Gossip { message },
                        };
                        let _ = comms_client.response_channel.send(gossip).await;
                    }
                }
                let resp = Message {
                    dest: msg.src,
                    src: my_id.clone(),
                    body: MessageBody::BroadcastOk {
                        in_reply_to: msg_id,
                    },
                };
                let _ = comms_client.response_channel.send(resp).await;
            }
            MessageBody::Read { msg_id } => {
                let resp = Message {
                    dest: msg.src,
                    src: my_id.clone(),
                    body: MessageBody::ReadOk {
                        in_reply_to: msg_id,
                        messages: messages.clone(),
                    },
                };
                let _ = comms_client.response_channel.send(resp).await;
            }
            _ => panic!("Unknown message type"),
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
    Read {
        msg_id: u32,
    },
    ReadOk {
        in_reply_to: u32,
        messages: HashSet<u32>,
    },
    Gossip {
        message: u32,
    },
    Broadcast {
        msg_id: u32,
        message: u32,
    },
    BroadcastOk {
        in_reply_to: u32,
    },
    Topology {
        msg_id: u32,
        topology: HashMap<String, Vec<String>>,
    },
    TopologyOk {
        in_reply_to: u32,
    },
}
