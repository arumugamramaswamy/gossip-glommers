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

## Challenge 3C

<blockquote>

Your node should propagate values it sees from broadcast messages to the other nodes in the cluster—even in the face of network partitions! Values should propagate to all other nodes by the end of the test. Nodes should only return copies of their own local values.

</blockquote>

Uhoh **network partitions** :(

Let's discuss what this could mean for us. Consider the following topology:
   
   
   1     2
A --- B --- C
      |
      | 3
      |
      D

A network partition could mean that any/ all of links 1, 2 and 3 could be severed - making the nodes unable to communicate with each other.

Our current algo doesn't work because when a link goes down, the messages that we would send on a given link are not replayed when the link is re-established. Hence we must be able to track which values were received and which values were dropped.

In order for server A to know which values B has received, there must be some way for B to acknowledge a value. Until B acknowledges a value, A will include it in every gossip message.

One potential way to achieve this is by attaching a sequence id (Ni) to every gossip message. This sequence ID would indicate how many values A has received. The gossip message will also include all values received by A so far (Mij, j < Ni).

The first message sent from A has the following structure:
Gossip {
    seq_id: Na = 5,
    values: Maj, j < 5: list of 5 values
}

When B receives this message, it stores Na in the variable Rab. Rab is the latest sequence id from A that B has seen.
B includes Rab in its next gossip message to A:
Gossip {
    seq_id: Nb = 9,
    values: Mbj, j < 9: list of 9 values
    latest_seq_id_seen: Rab = 5, (recieved Na)
}

When A receives this message, it knows that B has received all values Maj, j < Rab. Hence these values have been acknowledged by B. A can now omit them in further gossip messages. A future gossip message could be as follows:
Gossip {
    seq_id: Na = 20,
    values: Maj, j < 20, j >= Rab (currently = 5): list of 15 values
    latest_seq_id_seen: Rba = 9 (recieved Nb)
}

Hence the final structure of a gossip message is:
1. A's sequence number (Na)
2. The latest sequence number from B that A has seen (Rba)
3. The values from A that B has not acked yet. (Maj, j < Na, j >= Rab)

We implement this by maintaining 2 dictionaries:
```rust
    let mut neighbour_seq_ids_seen = HashMap::new(); // This Represents Rna, N is any other neighbour of A
    let mut seq_ids_neighbour_seen = HashMap::new(); // This Represents Ran, N is any other neighbour of A 
```

We also create a utility struct that manages the values to be gossiped:
```rust
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
```


We also have a timer that triggers gossip messages every 100ms on each node:
```rust
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
```

Running this code:
```terminal
Everything looks good! ?(??`)?
```

Sweet!

## Challenge 3D
TODO: talk about the neighbour graph analysis
## Challenge 3E
