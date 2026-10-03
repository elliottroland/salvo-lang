import net
import time

// ============================================================ the fleet ====
//
// Two virtual nodes in one process — `a` is `main`'s own node, `b` is booted
// on a second node id — joined by a `MemNetwork`, the in-memory transport the
// std tests use. Everything below runs unchanged over `HostTcpTransport`;
// only the three lines that build the transport would differ.

// The protocols this cluster serves. Each is an ordinary `actor effect`; what
// makes them cluster protocols is that their addresses cross the wire — so
// every payload has a wire form, and `protocol<E>()` says so.

// 1. A singleton: one sequencer hands out ids, every node hosts a candidate.
actor effect Sequencer {
    send fn next(out: Reply<Str>) => !out
}

// 2. Shards: `Key` marks the argument that decides the shard.
actor effect Inventory {
    send fn reserve(sku: Key Str, qty: Int, out: Reply<Str>) => !sku, !qty, !out
}

// 3. Scatter (map/reduce): every index answers a count, the sum is the answer.
actor effect Search {
    send fn query(word: Str, out: Reply<Int>) => !word, !out
}

// 4. Hedge: ask two replicas, take the first answer.
actor effect Lookup {
    send fn lookup(key: Str, out: Reply<Str>) => !key, !out
}

// ------------------------------------------------------------- members ----

handler Sequencing(who: Str) of Sequencer {
    mailbox { capacity: 16 }
    n: Int = 0
    send fn next(out: Reply<Str>) => !out {
        n = n + 1
        out.send("${who}#${n}")
    }
}

handler Stocking(shard: Str) of Inventory {
    mailbox { capacity: 32 }
    served: Int = 0
    send fn reserve(sku: Key Str, qty: Int, out: Reply<Str>) => !sku, !qty, !out {
        served = served + qty
        out.send("${shard}:${served}")
    }
}

// One index holds a slice of the corpus and answers how often a word occurs.
handler Indexing(words: List<Str>) of Search {
    mailbox { capacity: 16 }
    send fn query(word: Str, out: Reply<Int>) => !word, !out {
        let n = 0
        for w in words {
            if eq(w, word) {
                n = n + 1
            }
        }
        out.send(n)
    }
}

// A replica that answers at once…
handler Looking(who: Str) of Lookup {
    mailbox { capacity: 16 }
    send fn lookup(key: Str, out: Reply<Str>) => !key, !out {
        out.send("${key} from ${who}")
    }
}

// …and one that takes its time: the answer is parked on the timer and sent
// when it fires. `replyto` mints the continuation onto its own `answer`, a
// **private** send member — no face declares it, so nothing outside this
// handler can send to it, and its clause is written out like a free send fn's.
handler SlowLooking(who: Str, timer: Addr<Timer>) of Lookup {
    mailbox { capacity: 16 }
    send fn lookup(key: Str, out: Reply<Str>) => !key, !out {
        timer.after(millis(150), replyto answer(key, out))
    }
    send fn answer(key: Str, out: Reply<Str>, fired: Fired) => !key, !out, !fired {
        out.send("${key} from ${who}")
    }
}

// ------------------------------------------------ hand-written routers ----
//
// `route_any(group)` picks ONE member per send. A policy that must read the
// message — fan it out, race two copies — is a handler `of any E` written for
// that protocol: `any`, because it promises no order between two sends.

// Scatter: ask every member, sum the answers, reply once. The router itself
// is stateless and `use`-bound; the collecting is an actor's, since something
// has to receive the partial answers.
handler Scattering(group: Addr<ActorGroup<Search>>, gather: Addr<Gather>) of any Search {
    mailbox { capacity: 16 }
    send fn query(word: Str, out: Reply<Int>) => !word, !out {
        let members = waitfor ms: Reply<List<Addr<Search>>> { group.members(ms) }
        gather.scatter(word, members, out)
    }
}

actor effect Gather {
    send fn scatter(word: Str, members: List<Addr<Search>>, out: Reply<Int>) => !word, !members, !out
}

// One query at a time: the reply token waits in the actor's state until every
// partial answer is in. A linear value in state lives in a container — the
// queue owes, `remove_first` moves one obligation out — which is the shape
// `examples/actors/` explains.
handler Gathering() of Gather {
    mailbox { capacity: 16 }
    pending: Mut Deque<Reply<Int>> = mut_deque_of()
    left: Int = 0
    total: Int = 0
    send fn scatter(word: Str, members: List<Addr<Search>>, out: Reply<Int>) => !word, !members, !out {
        add_last(pending, out)
        left = size(members)
        total = 0
        for m in members {
            m.query(copy(word), replyto partial())
        }
    }
    // Private: one index's answer, the continuation `scatter` mints per member.
    send fn partial(n: Int) => !n {
        total = total + n
        left = left - 1
        if left == 0 {
            let out = remove_first(pending)
            when out {
                is Reply<Int> { out.send(copy(total)) }
                is None {}
            }
        }
    }
}

// Hedge: two members race, the first answer is the answer, the second is
// dropped — a `Reply` is discharged exactly once, so the second arrival has
// nothing to send to.
handler Hedging(group: Addr<ActorGroup<Lookup>>, racer: Addr<Race>) of any Lookup {
    mailbox { capacity: 16 }
    send fn lookup(key: Str, out: Reply<Str>) => !key, !out {
        let members = waitfor ms: Reply<List<Addr<Lookup>>> { group.members(ms) }
        racer.race(key, members, out)
    }
}

actor effect Race {
    send fn race(key: Str, members: List<Addr<Lookup>>, out: Reply<Str>) => !key, !members, !out
}

handler Racing() of Race {
    mailbox { capacity: 16 }
    pending: Mut Deque<Reply<Str>> = mut_deque_of()
    send fn race(key: Str, members: List<Addr<Lookup>>, out: Reply<Str>) => !key, !members, !out {
        add_last(pending, out)
        for m in members {
            m.lookup(copy(key), replyto first())
        }
    }
    send fn first(answer: Str) => !answer {
        let out = remove_first(pending)
        when out {
            is Reply<Str> { out.send(answer) }
            is None { discard(answer) }
        }
    }
}

// -------------------------------------------------------- an election ----
//
// `Leader` is a plain effect: whoever serves it decides who leads. This one
// is an election a child could run — the node whose host name sorts last
// wins — and it is still a real one: when that node leaves, the answer
// changes, and every `Elected` route follows it. `Elected` asks it when the
// route's view changes, not per send — here a node leaving changes the view
// anyway; an election whose answer moves on its own calls `refresh()` on the
// group. A Raft would serve the same effect, and nothing that routes through
// `Elected` would change.
handler LastHost(nodes: Addr<NodeGroup>, me: NodeEndpoint) of Leader {
    fn leader() -> NodeId? {
        let peers = waitfor out: Reply<List<Node>> { nodes.members(out) }
        let best_host = copy(me.host)
        let best = this_node()
        for n in peers {
            if n.at.host > best_host {
                best_host = copy(n.at.host)
                best = copy(n.id)
            }
        }
        return best
    }
}

// ---------------------------------------------------- the client side ----
//
// Every function below is written against the protocol alone. `any` says the
// function assumes nothing about which member answers — the honest claim for
// a fleet — and nothing here mentions a node, a group or a pick.

fn fresh_id() [any Sequencer] -> Str {
    return waitfor out: Reply<Str> { next(out) }
}

// Which shard a SKU lands on is the hash's business; that the *same* SKU
// lands on the *same* shard, and that a shard's count climbs with its own
// keys, is what the caller can see. (Which physical member is shard 0 depends
// on the members' order in the view, which this in-process fleet does not fix.)
fn checkout(skus: List<Str>) [any Inventory, Console] -> None {
    let shards: Mut List<Str> = mut_list_of()
    for sku in skus {
        let answer = waitfor out: Reply<Str> { reserve(copy(sku), 1, out) }
        let parts = split(answer, ":")
        add(shards, copy(get(parts, 0)!))
        println("  ${sku}: ${get(parts, 1)!} reserved on its shard so far")
    }
    println("  apple and apple on one shard: ${eq(get(shards, 0)!, get(shards, 2)!)}")
    println("  apple and fig on one shard: ${eq(get(shards, 0)!, get(shards, 3)!)}")
    println("  apple and pear on one shard: ${eq(get(shards, 0)!, get(shards, 1)!)}")
}

fn count(word: Str) [any Search] -> Int {
    return waitfor out: Reply<Int> { query(word, out) }
}

fn find(key: Str) [any Lookup] -> Str {
    return waitfor out: Reply<Str> { lookup(key, out) }
}

// A policy and its stub are bound for a scope, and a scope is a function:
// two policies for two protocols — `RouteSelector<Sequencer>`,
// `RouteSelector<Inventory>` — are
// one type in the generated code, so they take turns rather than share one.
// `Leader` arrives through the signature; `Elected` captures it from there.
fn two_ids(seq: Addr<ActorGroup<Sequencer>>) [use, Leader, Console] -> None => !seq {
    use Elected<Sequencer>()
    use route_any(seq)
    println("  ${fresh_id()} ${fresh_id()}")
}

fn shop(stock: Addr<ActorGroup<Inventory>>) [use, Console] -> None => !stock {
    use Sharded<Inventory>()
    use route_any(stock)
    checkout(["apple", "pear", "apple", "fig", "pear"])
}

// ----------------------------------------------------------- node `b` ----
//
// The second node, booted on its own pool: its own transport binding, its own
// node group, and its own replicas of every actor group — found by name
// through the node group, so the two nodes see one group each.
actor effect Boot {
    send fn boot(done: Reply<Addr<Sequencer>>) => !done
    send fn stop(done: Reply<Bool>) => !done
}

handler Booting(at: NodeEndpoint, all: List<NodeEndpoint>, net: Addr<MemNet>) [Transport, spawn] of Boot {
    mailbox { capacity: 2 }
    nodes: Addr<NodeGroup>? = None
    send fn boot(done: Reply<Addr<Sequencer>>) => !done {
        let p = pool(1)
        // A real second node is a second process; this in-process double runs
        // on another pool of the same process. The node group inherits this
        // actor's `Transport` (node b's, supplied `with` at the spawn below)
        // and connects the node itself, since nothing has yet.
        let group = spawn StaticNodeGroup("cluster", copy(all)) on p
        nodes = copy(group)

        let seq = actor_group<Sequencer>(copy(group))
        let mine = spawn Sequencing("b") on p in seq

        let stock = actor_group<Inventory>(copy(group))
        spawn Stocking("shard-b") on p in stock

        let index = actor_group<Search>(copy(group))
        spawn Indexing(["salvo", "actors", "salvo", "nodes"]) on p in index

        let looks = actor_group<Lookup>(group)
        let timer = spawn DefaultTimer() on p
        spawn SlowLooking("b (slow)", timer) on p in looks

        done.send(mine)
    }
    send fn stop(done: Reply<Bool>) => !done {
        let group = nodes
        if !(group is None) {
            group.leave()
        }
        done.send(true)
    }
}

// ----------------------------------------------------------- node `a` ----

fn settle(timer: Addr<Timer>) [] -> None => timer {
    let _f = waitfor f: Reply<Fired> { timer.after(millis(400), f) }
}

fn main() [use, spawn] {
    use StdOutConsole()
    let a = NodeEndpoint { host: "a", port: 1 }
    let b = NodeEndpoint { host: "b", port: 1 }
    let all = [copy(a), copy(b)]
    let network = spawn MemNetwork() on pool(1)

    // Node a: the transport, the node group — which connects the node to the
    // wire, since nothing has — and one member of everything.
    use MemTransport(copy(a), copy(network))
    let p = pool(2)
    let nodes = spawn StaticNodeGroup("cluster", copy(all)) on p

    let seq = actor_group<Sequencer>(copy(nodes))
    spawn Sequencing("a") on p in seq
    let stock = actor_group<Inventory>(copy(nodes))
    spawn Stocking("shard-a") on p in stock
    let index = actor_group<Search>(copy(nodes))
    spawn Indexing(["salvo", "is", "salvo"]) on p in index
    let looks = actor_group<Lookup>(copy(nodes))
    spawn Looking("a") on p in looks

    // Node b, and a moment for the two node groups to meet.
    let pb = pool_at(new_node(), 1)
    let booter = spawn Booting(copy(b), copy(all), copy(network)) with MemTransport(copy(b), copy(network)) on pb
    let remote_seq = waitfor done: Reply<Addr<Sequencer>> { booter.boot(done) }
    let timer = spawn DefaultTimer() on pool(1)
    settle(copy(timer))
    let members = waitfor out: Reply<List<Node>> { nodes.members(out) }
    println("nodes: ${size(members) + 1}, sequencers: ${size(waitfor out: Reply<List<Addr<Sequencer>>> { seq.members(out) })}")

    // 1. Singleton — ids come from the leader's sequencer. The leader is the
    //    node whose host sorts last: b.
    println("singleton (b's sequencer is remote: ${!eq(node_of(remote_seq), this_node())}):")
    use LastHost(copy(nodes), copy(a))
    two_ids(copy(seq))

    // 2. Sharded — the same SKU lands on the same shard, every time.
    println("sharded:")
    shop(stock)

    // 3. Scatter / map-reduce — every index counts, the router sums.
    println("scatter:")
    use Scattering(copy(index), spawn Gathering() on p)
    println("  salvo: ${count("salvo")}, actors: ${count("actors")}, none: ${count("none")}")

    // 4. Hedge — a fast and a slow replica race; the fast one answers.
    println("hedge:")
    use Hedging(copy(looks), spawn Racing() on p)
    println("  ${find("k1")}")

    // 5. Failover — b leaves: its members are withdrawn from every group, and
    //    the election answers a, so the singleton's ids now come from a.
    let _stopped = waitfor done: Reply<Bool> { booter.stop(done) }
    settle(copy(timer))
    println("after b left:")
    println("  sequencers: ${size(waitfor out: Reply<List<Addr<Sequencer>>> { seq.members(out) })}")
    two_ids(seq)
}
