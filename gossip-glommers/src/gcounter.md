# GCounter

Challenge 4, we're more than halfway through!

<blockquote>

In this challenge, you’ll need to implement a stateless, grow-only counter which will run against Maelstrom’s g-counter workload. This challenge is different than before in that your nodes will rely on a sequentially-consistent key/value store service provided by Maelstrom.

</blockquote>

The wording of the challenge is vague: implement a **stateless** grow-only counter...

Thankfully after browsing through some forums, I was able to find out how they define stateless: when a node goes crashes and comes back up, the service should function as normal (each node stores no state).

The sequentially-consisten key/value store seems interesting too. The api is as follows:

<blockquote>

func (kv *KV) Read(ctx context.Context, key string) (any, error)
    Read returns the value for a given key in the key/value store. Returns an
    *RPCError error with a KeyDoesNotExist code if the key does not exist.

func (kv *KV) Write(ctx context.Context, key string, value any) error
    Write overwrites the value for a given key in the key/value store.

func (kv *KV) CompareAndSwap(ctx context.Context, key string, from, to any, createIfNotExists bool) error
    CompareAndSwap updates the value for a key if its current value matches the
    previous value. Creates the key if createIfNotExists is true.
    
</blockquote>

Let's read up on what it means to be sequentially consistent.

<blockquote>

A process in a sequentially consistent system may be far ahead, or behind, of other processes. For instance, they may read arbitrarily stale state. However, once a process A has observed some operation from process B, it can never observe a state prior to B. This, combined with the total ordering property, makes sequential consistency a surprisingly strong model for programmers.

</blockquote>

Okay let's try reasoning about this.

Process A:
read x = 0
compare x = 0 and swap x = 10

Process B:
read x = 0
compare x = 0 and swap x = 5

This means that the compare and swap operation at either A or B must fail as there is no total ordering where both CAS operations succeed. Let's work this out to make it clearer.

A: read x = 0
B: read x = 0
B: cas (0, 5) <---- success, x = 5
A: cas (0, 10) <--- fail because x != 0


<blockquote>

Your node will need to accept two RPC-style message types: add & read. Your service need only be eventually consistent: given a few seconds without writes, it should converge on the correct counter value.

</blockquote>


Okay, the service needs to be eventually consistent. That should be easy enough to achieve.

Idea 1: when a node receives an Add message, we try
