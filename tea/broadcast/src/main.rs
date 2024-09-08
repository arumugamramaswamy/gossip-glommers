use std::collections::hash_set::Difference;
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json;
use tokio::io::{self, AsyncBufReadExt, BufReader, Lines, Stdin};
use tokio::sync::mpsc;
use tokio::time;

struct Gossipper {
    values: Vec<u32>,
    set: HashSet<u32>,
}

impl Gossipper {
    fn new() -> Self {
        Gossipper {
            values: Vec::new(),
            set: HashSet::new(),
        }
    }

    fn add(&mut self, v: u32) {
        if self.set.insert(v) {
            self.values.push(v);
        }
    }

    fn add_list(&mut self, l: Vec<u32>) {
        for v in l {
            self.add(v);
        }
    }

    fn get_seq_id(&self) -> usize {
        self.values.len()
    }

    fn get_gossip(&self, start_seq_id: usize) -> Vec<u32> {
        self.values[start_seq_id..].to_vec()
    }
}

mod test {
    use crate::Gossipper;

    #[test]
    fn test_gossiper() {
        let mut g = Gossipper::new();
        g.add(1);
        assert_eq!(g.get_gossip(g.get_seq_id()), Vec::<u32>::new());
        assert_eq!(g.get_gossip(0), vec![1]);
    }
}

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

    let mut neighbour_seq_ids_seen = HashMap::new();
    let mut seq_ids_neighbour_seen = HashMap::new();

    for neighbour in topology.get(&my_id).unwrap().iter() {
        neighbour_seq_ids_seen.insert(neighbour, 0);
        seq_ids_neighbour_seen.insert(neighbour, 0);
        for neighbour_of_neighbour in topology.get(neighbour).unwrap().iter() {
            if *neighbour_of_neighbour == my_id {
                continue;
            }

            neighbour_seq_ids_seen.insert(neighbour_of_neighbour, 0);
            seq_ids_neighbour_seen.insert(neighbour_of_neighbour, 0);
            for n3 in topology.get(neighbour_of_neighbour).unwrap().iter() {
                if *n3 == my_id {
                    continue;
                }
                neighbour_seq_ids_seen.insert(n3, 0);
                seq_ids_neighbour_seen.insert(n3, 0);
            }
        }
    }

    let mut gossiper = Gossipper::new();

    let mut interval = time::interval(Duration::from_millis(200));

    loop {
        tokio::select! {
            _ = interval.tick() => {
                let curr_seq_id = gossiper.get_seq_id();
                for (neighbour, seq_id_seen) in seq_ids_neighbour_seen.iter() {
                    if *seq_id_seen < curr_seq_id {
                        let gossip = Message {
                            dest: neighbour.to_string(),
                            src: my_id.clone(),
                            body: MessageBody::Gossip {
                                node_seq_id: curr_seq_id,
                                last_neighbour_seq_id: *neighbour_seq_ids_seen
                                    .get(neighbour)
                                    .unwrap(),
                                messages: gossiper.get_gossip(*seq_id_seen),
                            },
                        };
                        let _ = comms_client.response_channel.send(gossip).await;
                    }
                }
            },
            Some(msg) = comms_client.msg_channel.recv() => match msg.body {
                MessageBody::Gossip {
                    messages,
                    node_seq_id,
                    last_neighbour_seq_id,
                } => {
                    *neighbour_seq_ids_seen.get_mut(&msg.src).unwrap() = node_seq_id;
                    *seq_ids_neighbour_seen.get_mut(&msg.src).unwrap() = last_neighbour_seq_id;
                    gossiper.add_list(messages);
                }
                MessageBody::Broadcast { msg_id, message } => {
                    gossiper.add(message);
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
                            messages: gossiper.get_gossip(0)
                        },
                    };
                    let _ = comms_client.response_channel.send(resp).await;
                }
                _ => panic!("Unknown message type"),
            }
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
        messages: Vec<u32>,
    },
    Gossip {
        messages: Vec<u32>,
        node_seq_id: usize,
        last_neighbour_seq_id: usize,
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
