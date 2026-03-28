use std::collections::HashMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json;
use tokio::io::{self, AsyncBufReadExt, BufReader, Lines, Stdin};
use tokio::sync::mpsc;
use tokio::time::interval;

#[tokio::main]
async fn main() {
    let mut receiver = Receiver::new();
    let my_id = perform_init(&mut receiver).await;
    let mut state = GCounterState::new(my_id);

    let mut interval = interval(Duration::from_millis(200));

    loop {
        tokio::select! {
            msg = receiver.recv() => {
                state.handle_message(msg).await
            },
            _ = interval.tick() => {
                state.trigger_reconciliation().await
            }
        }
    }
}

struct GCounterState {
    remaining: u32,
    current_state: u32,
    node_name: String,
    request_id_to_delta: HashMap<u32, u32>,
    message_id: u32,
}

impl GCounterState {
    fn new(my_id: String) -> Self {
        Self {
            remaining: 0,
            current_state: 0,
            node_name: my_id,
            request_id_to_delta: HashMap::new(),
            message_id: 0,
        }
    }

    async fn trigger_reconciliation(&mut self) {
        let msg_id = {
            self.message_id += 1;
            self.message_id
        };

        self.request_id_to_delta.insert(msg_id, self.remaining);

        send(
            "seq-kv".to_string(),
            self.node_name.clone(),
            MessageBody::Cas {
                msg_id,
                key: "k".to_string(),
                from: self.current_state,
                to: self.current_state + self.remaining,
                create_if_not_exists: true,
            },
        )
        .await;
    }

    async fn handle_message(&mut self, msg: Message) {
        match msg.body {
            MessageBody::Read { msg_id, .. } => {
                send(
                    msg.src,
                    self.node_name.clone(),
                    MessageBody::ReadOk {
                        in_reply_to: msg_id,
                        value: self.current_state,
                    },
                )
                .await
            }
            MessageBody::ReadOk { value, .. } => {
                if value > self.current_state {
                    self.current_state = value;
                }
            }
            MessageBody::Add { delta, msg_id } => {
                self.remaining += delta;
                send(
                    msg.src,
                    self.node_name.clone(),
                    MessageBody::AddOk {
                        in_reply_to: msg_id,
                    },
                )
                .await;
            }
            MessageBody::CasOk { in_reply_to } => {
                self.remaining -= self.request_id_to_delta.remove(&in_reply_to).unwrap();
            }
            MessageBody::Error { in_reply_to } => {
                let msg_id = {
                    self.message_id += 1;
                    self.message_id
                };
                self.request_id_to_delta.remove(&in_reply_to).unwrap();
                send(
                    "seq-kv".to_string(),
                    self.node_name.clone(),
                    MessageBody::Read {
                        msg_id,
                        key: Some("k".to_string()),
                    },
                )
                .await
            }
            _ => unreachable!(),
        }
    }
}

async fn perform_init(receiver: &mut Receiver) -> String {
    let msg = receiver.recv().await;
    match msg.body {
        MessageBody::Init {
            msg_id, node_id, ..
        } => {
            let _ = send(
                msg.src,
                node_id.clone(),
                MessageBody::InitOk {
                    in_reply_to: msg_id,
                },
            )
            .await;
            node_id
        }
        _ => panic!("init message expected"),
    }
}

struct Receiver(tokio::io::Lines<BufReader<Stdin>>);

impl Receiver {
    fn new() -> Self {
        Self(BufReader::new(tokio::io::stdin()).lines())
    }

    async fn recv(&mut self) -> Message {
        let line = self.0.next_line().await.unwrap().unwrap();
        eprintln!("Recved Message: {line:?}");
        let output = serde_json::from_str(&line).unwrap();
        output
    }
}

async fn send(dest: String, src: String, body: MessageBody) {
    let msg = Message { dest, src, body };
    eprintln!("Sending Message: {msg:?}");
    println!("{}", serde_json::to_string(&msg).unwrap());
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
        key: Option<String>,
    },
    ReadOk {
        in_reply_to: u32,
        value: u32,
    },
    Add {
        delta: u32,
        msg_id: u32,
    },
    AddOk {
        in_reply_to: u32,
    },
    Cas {
        msg_id: u32,
        key: String,
        from: u32,
        to: u32,
        create_if_not_exists: bool,
    },
    CasOk {
        in_reply_to: u32,
    },
    Error {
        in_reply_to: u32,
    },
}
