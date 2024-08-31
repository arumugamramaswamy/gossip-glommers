# Generate

It is about 6:30AM, and I'm not an early riser. Let's just blame it on one too many cups of coffee last night. After writhing in bed for the past couple of hours, I decided to just continue working on gossip glommers instead. So, here I am.

## Problem statement
<blockquote>

In this challenge, you’ll need to implement a globally-unique ID generation system that runs against Maelstrom’s unique-ids workload. Your service should be totally available, meaning that it can continue to operate even in the face of network partitions.

</blockquote>

To my sleep deprived brain, only a couple things stand out:
1. Globally unique
2. Totally available


In order for the generated IDs to be **globally unique**, all the nodes must start by sharing some state. But **total availability** imposes another restriction: all the nodes must be able to operate even if they cannot reach each other at all.

Off the top of my head, given N nodes, each node can start from offset o = 1..N and for a given node, the id can be given by o + N*(gen_number)

Now, we need to find out whether it is possible for us to easily assign these offsets to the nodes. Thinking back to the protocol spec, in the init message, we have the list of nodes and our current node. If all the nodes recieve the same list, we could just index into the list to get our offset.

<blockquote>

The node_ids field lists all nodes in the cluster, including the recipient. All nodes receive an identical list; you may use its order if you like.

</blockquote>

The protocol spec gives us the go ahead, so let's implement it!

## Implementation Details

I'm just copy pasting my code over because I don't have the mental bandwidth right now to refactor the code and make the apis look nice, maybe some other day.

We only need some minor changes to the init code and to the message handling code:

```rust
    ...
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
	    ...
	}
	...
    }
    ...
```

And for the message handling code:
```rust
    let resp = Message {
	dest: msg.src,
	src: my_id.clone(),
	body: MessageBody::GenerateOk {
	    in_reply_to: msg_id,
	    msg_id: {
		let tmp = my_msg_id;
		my_msg_id += 1;
		tmp
	    },
	    id: {
		next_gen_id += n_nodes;
		next_gen_id.try_into().unwrap()
	    },
	},
    };
```

Now, let's give it a run:
```
Everything looks good! ?(??`)?
```

Nice! Okay, let me try going to sleep again for a bit - see y'all next time!

P.S. I'm also pushing cleaning up the Makefile to future Aru, sucks to be him haha
