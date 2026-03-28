use std::collections::HashMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json;
use tokio::io::{AsyncBufReadExt, BufReader, Stdin};
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
    sender: Sender,
    request_id_to_delta: HashMap<u32, u32>,
    message_id: u32,
}

impl GCounterState {
    fn new(sender: Sender) -> Self {
        Self {
            remaining: 0,
            current_state: 0,
            sender,
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

        self.sender
            .send(
                "seq-kv".to_string(),
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
                self.sender
                    .send(
                        msg.src,
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
                self.sender
                    .send(
                        msg.src,
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
                self.sender
                    .send(
                        "seq-kv".to_string(),
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

async fn perform_init(receiver: &mut Receiver) -> Sender {
    let msg = receiver.recv().await;
    match msg.body {
        MessageBody::Init {
            msg_id, node_id, ..
        } => {
            let sender = Sender(node_id);
            let _ = sender
                .send(
                    msg.src,
                    MessageBody::InitOk {
                        in_reply_to: msg_id,
                    },
                )
                .await;
            sender
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

struct Sender(String);

impl Sender {
    async fn send(&self, dest: String, body: MessageBody) {
        let msg = Message {
            dest,
            src: self.0.clone(),
            body,
        };
        eprintln!("Sending Message: {msg:?}");
        println!("{}", serde_json::to_string(&msg).unwrap());
    }
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
