# Broadcast

Let's jump right into the third challenge!

<blockquote>

In this challenge, you’ll need to implement a broadcast system that gossips messages between all nodes in the cluster. Gossiping is a common way to propagate information across a cluster when you don’t need strong consistency guarantees.

</blockquote>

Okay, it looks like the challenge is broken up into many small parts - the first one seem pretty straightforward.

There are 3 types of messages:
- topology -> indicates the neighbours of each node
- broadcast -> send a value to be broadcasted to all nodes in the cluster
- read -> get a (unordered) list of all values seen by the node

## Challenge 3A

```rust
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
```

Now that the messages are out of the way, we modify the init code to handle storing the topology value that we receive at the begining of the test.


```rust
... init complete

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
... handle incomming messages
```

And then finally, we handle the incoming messages:

```rust
    while let Some(msg) = comms_client.msg_channel.recv().await {
        match msg.body {
            MessageBody::Broadcast { msg_id, message } => {
                messages.insert(message);
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
```

Let's give that a run...

```terminal
Everything looks good! ?(??`)?
```

Sweet!

## Challenge 3B

<blockquote>

Your node should propagate values it sees from broadcast messages to the other nodes in the cluster. It can use the topology passed to your node in the topology message or you can build your own topology.

The simplest approach is to simply send a node’s entire data set on every message, however, this is not practical in a real-world system. Instead, try to send data more efficiently as if you were building a real broadcast system.

Values should propagate to all other nodes within a few seconds.

</blockquote>

Easy enough, let's introduce a new type of message - `Gossip`

When a node receives a new ID it fires of a `Gossip` message to all its neighbours.


```rust
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

```

That seems to do it!


```terminal
Everything looks good! ?(??`)?
```
