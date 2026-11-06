// Addresses and answers across nodes: the service that makes an `eff[E]`
// or a `Reply<T>` that crossed the wire work on the other node exactly as
// it does here. It keeps who is where (the node every pool belongs to, the
// identity of every actor whose address has left its node, the proxies
// standing for actors elsewhere and the credits each holds), and turns a
// send to a proxy, an answer to a reply minted elsewhere, a credit and a
// handshake into **frames**: Salvo values in the canonical wire encoding,
// the same bytes on both backends.
//
// Like [std.runtime], whose service it is, this module is std's own: a
// program reaches it through [std.net] and [std.codec], and through
// addresses and replies that arrive in messages. A process is one node
// and may host more ([new_node]), each with pools of its own ([pool_at]),
// which is how two nodes run in one process over an in-memory transport.
//
// **How an address crosses.** An address leaves its node as its identity,
// a [RemoteRef]: the node, the actor's index there, and random bits minted
// the first time the address leaves. Arriving ([import_addr]), it becomes
// one of three things. An identity of this node whose bits match is the
// actor itself. An identity of this node with the wrong bits is the one
// shared dead entry, so a forged or stale address is a send to the dead:
// the bits are what make an address a capability on the wire. Any other
// identity becomes a **proxy**, made once per identity: a record in the
// scheduler's actor table whose sends are handed back to this module,
// encoded and sent as frames to the node the actor lives on.
//
// **Credits.** A mailbox's capacity holds across nodes. A new proxy asks
// the actor's node for credits, and is granted the room its mailbox has
// left beyond what other senders already hold, at least one. Each send
// through the proxy spends a credit; with none left, the sender parks
// until a grant arrives, exactly as a local send waits on a full mailbox.
// As each message from that node leaves the actor's mailbox, one credit is
// granted back.
//
// **Answers.** A reply minted here travels as [ReplyParts]: the node, what
// it answers (an actor's continuation, a waiting frame, a task) and its
// slot. A task's body stays here, keyed by the slot, until the answer comes
// back. Answering such a reply on another node ([answer_remote]) sends an
// answer frame back to the node that minted it, where it is decoded and
// delivered as an ordinary answer.
//
// **Nothing is delivered wrong.** A frame for a node not hosted here, for
// an actor that does not exist, with bits that do not match, or with a
// payload that does not decode is dropped ([deliver]). Nothing is resent
// either: a frame the transport loses is lost, so delivery is at most once.
//
// **A node that leaves** ([node_left]) is forgotten: its route and its
// protocol table go, and every proxy for an actor on it dies, as if it had
// faulted, with the reason "node left". Watchers of those proxies are sent
// an `Exit`, senders waiting for credit stop, and later sends do nothing.
//
// ```no-check
// import std.net
// import std.codec
// import std.time
//
// actor effect Counter {
//     send fn bump(n: Int)
//     send fn total(out: Reply<Int>) => !out
// }
//
// handler Counting() for Ordered Counter {
//     mailbox { capacity: 2 }
//
//     state {
//         sum: Int = 0
//     }
//
//     send fn bump(n: Int) {
//         sum = sum + n
//     }
//
//     send fn total(out: Reply<Int>) {
//         out.send(sum)
//     }
// }
//
// // Brings up the node the calling thread belongs to: its wire actors on a
// // pool of that node, and its route.
// fn bring_up(at: NodeEndpoint) [Transport, spawn] -> None => at {
//     let _connected = connect(copy(at), pool(1))
// }
//
// actor effect Boot {
//     send fn boot(at: NodeEndpoint, done: Reply<Bool>) => !at, !done
// }
//
// // Run on node B's pool, so the node it brings up is B.
// handler Booting() [Transport, spawn] for Ordered Boot {
//     mailbox { capacity: 1 }
//
//     send fn boot(at: NodeEndpoint, done: Reply<Bool>) {
//         bring_up(at)
//         done.send(true)
//     }
// }
//
// fn main() [use, spawn] {
//     use StdConsole()
//     let a = NodeEndpoint { host: "a", port: 1 }
//     let b = NodeEndpoint { host: "b", port: 1 }
//     let net = spawn MemNetwork() on pool(1)        // the in-memory wire
//
//     // Node A is this one; node B is hosted beside it.
//     use MemTransport(copy(a), copy(net))
//     bring_up(copy(a))
//     let pb = pool_at(new_node(), 1)
//     let booter = spawn Booting() with MemTransport(copy(b), copy(net)) on pb
//     let _up = waitfor done: Reply<Bool> { booter.boot(copy(b), done) }
//
//     // A counter on node B. Its address crosses as bytes, and comes back
//     // on node A as a proxy.
//     let there = spawn Counting() on pb
//     let far = decode<eff[Counter]>(encode(copy(there)))
//     if far is None {
//         return
//     }
//     let timer = spawn DefaultTimer() on pool(1)
//     let _t = waitfor f: Reply<Fired> { timer.after(millis(300), f) }
//     println("credits: ${credits(far) ?: -1}")      // credits: 2
//     for i in range(0, 5) {
//         far.bump(i)                                // through a 2-message mailbox
//     }
//     let sum = waitfor out: Reply<Int> { far.total(out) }
//     println("total over the wire: ${sum}")         // total over the wire: 10
// }
// ```
//
// **What stays with the host** is what only the generated code knows: how
// to encode a message of a given protocol, how to decode one for a given
// actor or waiting frame, and where a node's frames leave the process.
// Those are the platform functions below.
//
// **Locks and costs.** This module's table has a lock of its own. The
// order is the scheduler's lock, then this one, never the reverse: no
// member of the table calls the scheduler, and the scheduler calls
// [credit_back] with its own lock held, which is why a frame is first
// staged and later sent by [flush], with no lock held, through the node's
// outbound hook. A frame for a node with no route yet, or from a node
// whose outbound hook is not bound, waits and goes once both are known.
// Every remote send encodes its message and its frame. The first send
// through a new proxy waits one round trip for its first credits. Every
// identity imported adds a proxy record to the scheduler's table for the
// life of the process.
// @compilerrules [runtime-layers, net-wire-format, net-addr-routable, net-addr-capability, net-remote-backpressure, node-exit]

import std.runtime

// The codec intrinsics, as `net` declares them: private here,
// since this module sits below `net`.
// @compilerrules [net-wire-format]
intrinsic fn encode<T>(value: T) [] -> Bytes => !value
intrinsic fn decode<T>(data: Bytes) [] -> T? => data

// ---- what only the host knows

// Decodes a message of protocol [proto] for hosted actor
// [addr], with the decoder its spawn registered: `None` when it has none or
// the bytes are malformed.
// @compilerrules [net-wire-format]
platform fn decode_message(addr: Int, proto: Str, payload: Bytes) [] -> Dyn? => addr, proto, payload

// Decodes an answer for waiting frame [wid] with the decoder
// its `waitfor` registered.
// @compilerrules [net-wire-format]
platform fn decode_waiter_answer(wid: Int, payload: Bytes) [] -> Dyn? => wid, payload

// Decodes an answer for the task exported under [key].
// @compilerrules [net-wire-format]
platform fn decode_task_answer(key: Long, payload: Bytes) [] -> Dyn? => key, payload

// An answer for an actor's continuation, still as bytes: the
// actor decodes it in its activation, since only it knows the type.
// @compilerrules [net-wire-format]
platform fn raw_answer(payload: Bytes) [] -> Dyn => !payload

// The message a control frame's payload arrives
// as at [sink]: its handler's private `control(from, data)` member.
// @compilerrules [node-group, actor-group]
platform fn control_message(sink: Int, from: Long, payload: Bytes) [] -> Dyn? => sink, from, payload

// Hands [frame] to node [from]'s outbound hook, for endpoint
// [to]: the transport moves it.
// @compilerrules [net-addr-routable]
platform fn wire_out(from: Long, to: Bytes, frame: Bytes) [] -> None => from, !to, !frame

// ---- the frames
// @compilerrules [net-wire-format]

// 0: a message for a hosted actor, encoded with its protocol's hash.
struct MsgFrame { to: Long, actor: Long, bits: Long, from: Long, proto: Str, payload: Bytes }
// 1: an answer for a token minted on the node it goes to: an actor's
// continuation (kind 0), a waiting frame (1) or an exported task (2).
struct AnswerFrame { to: Long, kind: Int, id: Long, slot: Long, bits: Long, payload: Bytes }
// 2: credits for a proxy of actor (host, actor, bits) on node [to].
struct GrantFrame { to: Long, host: Long, actor: Long, bits: Long, n: Int }
// 3: a new proxy asks for its first credits.
struct OpenFrame { to: Long, actor: Long, bits: Long, from: Long }
// 4: a payload for the actor listening on [channel] at node [to] (0: the
// receiving node itself, a HELLO's).
struct ControlFrame { to: Long, from: Long, channel: Str, payload: Bytes }
type Frame = MsgFrame | AnswerFrame | GrantFrame | OpenFrame | ControlFrame

// The identity an actor's address has on the wire: the [node] it lives on,
// its index [actor] there, and the random [bits] that make the address hard
// to guess. Equal identities name the same actor, wherever they are read.
// @compilerrules [net-addr-routable, net-addr-capability]
export struct RemoteRef : Hashed<self> by auto {
    node: Long,
    actor: Long,
    bits: Long
}

// A reply's wire form.
//
// [node] is the node that minted it; [kind] is 0 for an actor's
// continuation, 1 for a waiting frame and 2 for a task; [id] is the actor,
// the waiter or the exported task; [slot] is the reply's slot; [bits], for
// an actor's continuation, are the actor's.
// @compilerrules [net-addr-routable]
export struct ReplyParts {
    node: Long,
    kind: Int,
    id: Long,
    slot: Long,
    bits: Long
}

struct ControlKey : Hashed<self> by auto {
    node: Long,
    channel: Str
}

// A frame waiting for its route or its node's outbound hook.
struct Parked { from: Long, to: Long, frame: Bytes }
// A frame ready to go, sent by [flush].
struct Staged { from: Long, to: Bytes, frame: Bytes }

// A task whose token crossed the wire: its body waits here
// for the answer.
// @compilerrules [net-addr-routable]
linear struct ExportedTask { pool: Int, body: Body }

fn drop_exported_task(t: ExportedTask) [] -> None => !t {
    let {pool, body} = t
    drop_body(body)
}

// What looking up an identity found: the index [idx] of the actor or proxy
// that stands for it. The other two outcomes are internal: a proxy, or the
// shared dead entry, to be made outside the table's lock.
export struct Found { idx: Int }
struct MakeProxy {}
struct MakeDead {}
type Import = Found | MakeProxy | MakeDead

effect RouteTable {
    fn node_of_pool(pool: Int) -> Long => pool
    fn adopt_pool(pool: Int, node: Long) -> None => !pool, !node
    fn add_node() -> Long
    fn hosts(node: Long) -> Bool => node
    fn identity_of(addr: Int, pool: Int) -> RemoteRef => addr, pool
    fn find_import(r: RemoteRef, here: Long) -> Import => r, here
    fn register_proxy(r: RemoteRef, idx: Int, here: Long) -> Int => !r, !idx, !here
    fn register_dead(idx: Int) -> Int => !idx
    fn is_proxy(addr: Int) -> Bool => addr
    fn proxy_ref(addr: Int) -> RemoteRef? => addr
    // 1: a credit taken; 0: none, [me] recorded to be woken; -1: not a proxy.
    fn take_credit(addr: Int, me: Parker) -> Int => addr, !me
    fn stage(from: Long, to: Long, frame: Bytes) -> None => !from, !to, !frame
    fn grant(addr: Int, pool: Int, from: Long, n: Int) -> None => !addr, !pool, !from, !n
    fn take_outbox() -> Mut List<Staged>
    fn add_route(node: Long, at: Bytes) -> None => !node, !at
    fn set_outbound(node: Long) -> None => !node
    fn has_outbound(node: Long) -> Bool => node
    fn accepts(to: Long, actor: Long, claimed: Long) -> Bool => to, actor, claimed
    fn received(idx: Int) -> None => !idx
    fn credited(r: RemoteRef, to: Long, n: Int) -> Bool => !r, !to, !n
    fn held(idx: Int) -> Int => idx
    fn put_task(key: Long, t: ExportedTask) -> None => !key, !t
    fn take_task(key: Long) -> ExportedTask? => key
    fn watch_channel(node: Long, channel: Str, sink: Int) -> None => !node, !channel, !sink
    fn channel_sink(node: Long, channel: Str) -> Int => node, channel
    fn set_protocols(table: List<(Str, Str)>) -> None => !table
    fn protocols() -> List<(Str, Str)>
    fn set_peer(node: Long, table: List<(Str, Str)>) -> None => !node, !table
    fn peer_hash(node: Long, protocol: Str) -> Str? => node, protocol
    fn forget_node(node: Long) -> List<Int> => node
    fn credits_of(addr: Int) -> Int? => addr
    // The views routers read.
    // @compilerrules [actor-router]
    fn set_view(group: Int, members: List<Int>) -> None => !group, !members
    fn view_of(group: Int) -> List<Int> => group
    fn version_of(group: Int) -> Long => group
    fn bump(group: Int) -> None => !group
    // Records [me] to be woken when [group]'s version moves past [seen];
    // answers its id, or -1 when it already has.
    fn add_view_waiter(group: Int, seen: Long, me: Parker) -> Long => !group, !seen, !me
    fn drop_view_waiter(id: Long) -> None => !id
}

// [group]'s view version. (A member calling a sibling goes
// through the module's binding, which would take the table's lock again: the
// members share these instead.)
// @compilerrules [actor-router]
fn version_in(versions: Map<Int, Long>, group: Int) [] -> Long => versions, group {
    let v = get(versions, group)
    if v is Long n {
        return copy(n)
    }
    return 0
}

// Moves [group]'s version and wakes its waiters.
// @compilerrules [actor-router]
fn bump_in(versions: Mut Map<Int, Long>, waiters: Mut List<ViewWaiter>, group: Int) [] -> None
=> versions: Mut, waiters: Mut, !group {
    put(versions, copy(group), version_in(versions, copy(group)) + 1)
    let i = 0
    while i < size(waiters) {
        if (get(waiters, i) ?: unreachable!()).group == group {
            let w = remove_at(waiters, copy(i)) ?: unreachable!()
            unpark(copy(w.parker))
        } else {
            i = i + 1
        }
    }
}

// A sender waiting for a view to change.
// @compilerrules [actor-router]
struct ViewWaiter {
    group: Int,
    id: Long,
    parker: Parker
}

handler Routes() for RouteTable {
    state {
        node_id: Long = 0
        hosted: Mut Set<Long> = Mut Set()
        pool_node: Mut Map<Int, Long> = Mut Map()
        bits: Mut Map<Int, Long> = Mut Map()
        remote: Mut Map<Int, RemoteRef> = Mut Map()
        proxies: Mut Map<RemoteRef, Int> = Mut Map()
        credits: Mut Map<Int, Int> = Mut Map()
        // For a hosted actor, the credits remote senders still hold; for a
        // proxy, the messages in flight to it.
        held_n: Mut Map<Int, Int> = Mut Map()
        routes: Mut Map<Long, Bytes> = Mut Map()
        outbound: Mut Set<Long> = Mut Set()
        parked: Mut List<Parked> = Mut List()
        outbox: Mut List<Staged> = Mut List()
        // Exported tasks and their keys, index-aligned: a map of linear values
        // does not build on Rust yet (ROADMAP 0c).
        task_keys: Mut List<Long> = Mut List()
        tasks: Mut List<ExportedTask> = Mut List()
        controls: Mut Map<ControlKey, Int> = Mut Map()
        local: List<(Str, Str)> = List<(Str, Str)>()
        peers: Mut Map<Long, List<(Str, Str)>> = Mut Map()
        dead_entry: Int = -1
        credit_waiters: Mut List<Parker> = Mut List()
        // Per group: the member list its replica mirrors, and a
        // version moved by every change and every `refresh`.
        // @compilerrules [actor-router]
        views: Mut Map<Int, List<Int>> = Mut Map()
        versions: Mut Map<Int, Long> = Mut Map()
        view_waiters: Mut List<ViewWaiter> = Mut List()
        next_waiter: Long = 0
    }

    init {
        node_id = fresh_node()
        add(hosted, copy(node_id))
        put(pool_node, 0, copy(node_id))
    }

    fn node_of_pool(pool: Int) -> Long => pool {
        return node_in(pool_node, node_id, pool)
    }

    fn adopt_pool(pool: Int, node: Long) -> None => !pool, !node {
        put(pool_node, pool, node)
    }

    fn add_node() -> Long {
        let n = fresh_node()
        add(hosted, copy(n))
        return n
    }

    fn hosts(node: Long) -> Bool => node {
        return contains(hosted, node)
    }

    fn identity_of(addr: Int, pool: Int) -> RemoteRef => addr, pool {
        return identity_in(remote, bits, pool_node, node_id, addr, pool)
    }

    fn find_import(r: RemoteRef, here: Long) -> Import => r, here {
        if r.node == here {
            let idx = to_int(r.actor)
            let b = get(bits, idx)
            if !contains_key(remote, idx) && b is Long known && known == r.bits {
                return Found { idx: idx }
            }
            if dead_entry >= 0 {
                return Found { idx: copy(dead_entry) }
            }
            return MakeDead {}
        }
        let p = get(proxies, r)
        if p is Int idx {
            return Found { idx: copy(idx) }
        }
        return MakeProxy {}
    }

    fn register_proxy(r: RemoteRef, idx: Int, here: Long) -> Int => !r, !idx, !here {
        let existing = get(proxies, r)
        if existing is Int e {
            // Another thread imported it meanwhile; the spare entry is unused.
            return copy(e)
        }
        put(remote, copy(idx), copy(r))
        put(proxies, copy(r), copy(idx))
        put(credits, copy(idx), 0)
        let frame = encode<Frame>(OpenFrame { to: copy(r.node), actor: copy(r.actor), bits: copy(r.bits), from: copy(here) })
        stage_in(routes, outbound, outbox, parked, here, copy(r.node), frame)
        return idx
    }

    fn register_dead(idx: Int) -> Int => !idx {
        if dead_entry >= 0 {
            return copy(dead_entry)
        }
        dead_entry = idx
        return copy(dead_entry)
    }

    fn is_proxy(addr: Int) -> Bool => addr {
        return contains_key(remote, addr)
    }

    fn proxy_ref(addr: Int) -> RemoteRef? => addr {
        let r = get(remote, addr)
        if r is RemoteRef found {
            return copy(found)
        }
        return None
    }

    fn take_credit(addr: Int, me: Parker) -> Int => addr, !me {
        let c = get(credits, addr)
        if c is Int n {
            if n > 0 {
                put(credits, copy(addr), n - 1)
                put(held_n, copy(addr), held_in(held_n, copy(addr)) + 1)
                return 1
            }
            add(credit_waiters, me)
            return 0
        }
        return -1
    }

    fn stage(from: Long, to: Long, frame: Bytes) -> None => !from, !to, !frame {
        stage_in(routes, outbound, outbox, parked, from, to, frame)
    }

    fn grant(addr: Int, pool: Int, from: Long, n: Int) -> None => !addr, !pool, !from, !n {
        put(held_n, copy(addr), held_in(held_n, copy(addr)) + n)
        let me = identity_in(remote, bits, pool_node, node_id, copy(addr), pool)
        let frame = encode<Frame>(GrantFrame { to: copy(from), host: copy(me.node), actor: copy(me.actor), bits: copy(me.bits), n: n })
        stage_in(routes, outbound, outbox, parked, copy(me.node), from, frame)
    }

    fn take_outbox() -> Mut List<Staged> {
        let out: Mut List<Staged> = Mut List()
        while size(outbox) > 0 {
            add(out, remove_at(outbox, 0) ?: unreachable!())
        }
        return out
    }

    fn add_route(node: Long, at: Bytes) -> None => !node, !at {
        put(routes, node, at)
        restage(routes, outbound, outbox, parked)
    }

    fn set_outbound(node: Long) -> None => !node {
        add(outbound, node)
        restage(routes, outbound, outbox, parked)
    }

    fn has_outbound(node: Long) -> Bool => node {
        return contains(outbound, node)
    }

    fn accepts(to: Long, actor: Long, claimed: Long) -> Bool => to, actor, claimed {
        if !contains(hosted, to) {
            return false
        }
        let idx = to_int(actor)
        let b = get(bits, idx)
        if !contains_key(remote, idx) && b is Long known && known == claimed {
            return true
        }
        return false
    }

    fn received(idx: Int) -> None => !idx {
        let h = held_in(held_n, copy(idx))
        if h > 0 {
            put(held_n, idx, h - 1)
        }
    }

    fn credited(r: RemoteRef, to: Long, n: Int) -> Bool => !r, !to, !n {
        if !contains(hosted, to) {
            return false
        }
        let p = get(proxies, r)
        if p is Int idx {
            let i = copy(idx)
            let c = get(credits, i)
            let now = 0
            if c is Int have {
                now = copy(have)
            }
            put(credits, copy(i), now + n)
            let h = held_in(held_n, copy(i)) - n
            if h < 0 {
                h = 0
            }
            put(held_n, i, h)
            wake_senders(credit_waiters)
            return true
        }
        return false
    }

    fn held(idx: Int) -> Int => idx {
        return held_in(held_n, idx)
    }

    fn put_task(key: Long, t: ExportedTask) -> None => !key, !t {
        add(task_keys, key)
        add(tasks, t)
    }

    fn take_task(key: Long) -> ExportedTask? => key {
        let i = 0
        while i < size(task_keys) {
            if (get(task_keys, i) ?: unreachable!()) == key {
                let _k = remove_at(task_keys, copy(i))
                return remove_at(tasks, i)
            }
            i = i + 1
        }
        return None
    }

    fn watch_channel(node: Long, channel: Str, sink: Int) -> None => !node, !channel, !sink {
        put(controls, ControlKey { node: node, channel: channel }, sink)
    }

    fn channel_sink(node: Long, channel: Str) -> Int => node, channel {
        let s = get(controls, ControlKey { node: copy(node), channel: copy(channel) })
        if s is Int sink {
            return copy(sink)
        }
        return -1
    }

    fn set_protocols(table: List<(Str, Str)>) -> None => !table {
        local = table
    }

    fn protocols() -> List<(Str, Str)> {
        return copy(local)
    }

    fn set_peer(node: Long, table: List<(Str, Str)>) -> None => !node, !table {
        put(peers, node, table)
    }

    fn peer_hash(node: Long, protocol: Str) -> Str? => node, protocol {
        let t = get(peers, node)
        if t is List<(Str, Str)> table {
            for entry in table {
                let (name, hash) = entry
                if name == protocol {
                    return copy(hash)
                }
            }
        }
        return None
    }

    fn forget_node(node: Long) -> List<Int> => node {
        let _route = remove(routes, node)
        let _peer = remove(peers, node)
        let gone: Mut List<Int> = Mut List()
        for idx in keys(remote) {
            let r = get(remote, idx)
            if r is RemoteRef found {
                if found.node == node {
                    add(gone, copy(idx))
                }
            }
        }
        wake_senders(credit_waiters)
        return gone
    }

    fn set_view(group: Int, members: List<Int>) -> None => !group, !members {
        put(views, copy(group), members)
        bump_in(versions, view_waiters, group)
    }

    fn view_of(group: Int) -> List<Int> => group {
        let v = get(views, group)
        if v is List<Int> found {
            return copy(found)
        }
        return List<Int>()
    }

    fn version_of(group: Int) -> Long => group {
        return version_in(versions, group)
    }

    fn bump(group: Int) -> None => !group {
        bump_in(versions, view_waiters, group)
    }

    fn add_view_waiter(group: Int, seen: Long, me: Parker) -> Long => !group, !seen, !me {
        if version_in(versions, copy(group)) != seen {
            return -1
        }
        next_waiter = next_waiter + 1
        add(view_waiters, ViewWaiter { group: group, id: copy(next_waiter), parker: me })
        return copy(next_waiter)
    }

    fn drop_view_waiter(id: Long) -> None => !id {
        let i = 0
        while i < size(view_waiters) {
            if (get(view_waiters, i) ?: unreachable!()).id == id {
                let _w = remove_at(view_waiters, copy(i))
                return
            }
            i = i + 1
        }
    }

    fn credits_of(addr: Int) -> Int? => addr {
        let c = get(credits, addr)
        if c is Int n {
            return copy(n)
        }
        return None
    }

}

// The node pool [pool] belongs to.
fn node_in(pool_node: Map<Int, Long>, node_id: Long, pool: Int) [] -> Long => pool_node, node_id, pool {
    let n = get(pool_node, pool)
    if n is Long v {
        return copy(v)
    }
    return copy(node_id)
}

// The count [held_n] keeps for [idx].
fn held_in(held_n: Map<Int, Int>, idx: Int) [] -> Int => held_n, idx {
    let h = get(held_n, idx)
    if h is Int n {
        return copy(n)
    }
    return 0
}

// The identity of local index [addr] on [pool]: its bits
// minted the first time it is asked for, which is the first time the addr
// leaves its node.
// @compilerrules [net-addr-routable]
fn identity_in(remote: Map<Int, RemoteRef>, bits: Mut Map<Int, Long>, pool_node: Map<Int, Long>, node_id: Long, addr: Int, pool: Int) [] -> RemoteRef
=> remote, bits: Mut, pool_node, node_id, addr, pool {
    let r = get(remote, addr)
    if r is RemoteRef found {
        return copy(found)
    }
    let n = node_in(pool_node, node_id, pool)
    let b = get(bits, addr)
    if b is Long known {
        return RemoteRef { node: n, actor: to_long(addr), bits: copy(known) }
    }
    let minted = identity_bits()
    put(bits, copy(addr), copy(minted))
    return RemoteRef { node: n, actor: to_long(addr), bits: minted }
}

// Senders waiting for credit look again: a grant arrived, or a node left.
fn wake_senders(waiters: Mut List<Parker>) [] -> None => waiters: Mut {
    while size(waiters) > 0 {
        unpark(remove_at(waiters, 0) ?: unreachable!())
    }
}

// Stages [frame] from node [from] for node [to]: ready to send when [to] has
// a route and [from] an outbound hook, parked until then otherwise.
fn stage_in(routes: Map<Long, Bytes>, outbound: Set<Long>, outbox: Mut List<Staged>, parked: Mut List<Parked>, from: Long, to: Long, frame: Bytes) [] -> None
=> routes, outbound, outbox: Mut, parked: Mut, !from, !to, !frame {
    let ep = get(routes, to)
    if ep is Bytes at {
        if contains(outbound, from) {
            add(outbox, Staged { from: from, to: copy(at), frame: frame })
            return
        }
    }
    add(parked, Parked { from: from, to: to, frame: frame })
}

// Frames parked for a route or an outbound hook try again.
fn restage(routes: Map<Long, Bytes>, outbound: Set<Long>, outbox: Mut List<Staged>, parked: Mut List<Parked>) [] -> None
=> routes, outbound, outbox: Mut, parked: Mut {
    let waiting: Mut List<Parked> = Mut List()
    while size(parked) > 0 {
        add(waiting, remove_at(parked, 0) ?: unreachable!())
    }
    for p in waiting {
        stage_in(routes, outbound, outbox, parked, copy(p.from), copy(p.to), copy(p.frame))
    }
}

use Routes()

// A fresh node identity: random and non-negative, since the
// core marks a local sender as -1.
// @compilerrules [net-addr-routable]
fn fresh_node() [] -> Long {
    let b = identity_bits()
    if b < 0 {
        return -(b + 1)
    }
    return b
}

// ---- for the host's shims

// The node the calling thread's pool belongs to: the node a send from here
// comes from, and the one a pool made here belongs to.
// @compilerrules [net-addr-routable]
export fn here_node() [] -> Long {
    return node_of_pool(current_pool())
}

// Records that [pool] belongs to the calling thread's node. [std.core.actor]'s
// `pool` calls it, so a pool made inside an actor on a virtual node belongs
// to that node.
// @compilerrules [net-addr-routable]
export fn adopt(pool: Int) [] -> None => !pool {
    adopt_pool(pool, here_node())
}

// Hosts a fresh virtual node in this process, and answers its id: random,
// and never negative.
// @compilerrules [net-addr-routable]
export fn new_node() [] -> Long {
    return add_node()
}

// Makes a pool of [n] worker threads belonging to [node], and answers its
// index: what runs there runs on that node.
// @compilerrules [net-addr-routable]
export fn pool_at(node: Long, n: Int) [] -> Int => !node, n {
    let p = new_pool_of(n, -1)
    adopt_pool(copy(p), node)
    return p
}

// The identity of the actor at local index [addr]. A proxy's is the remote
// actor's; a local actor's bits are minted the first time it is asked for.
// @compilerrules [net-addr-routable]
export fn identity(addr: Int) [] -> RemoteRef => addr {
    return identity_of(copy(addr), actor_pool(copy(addr)))
}

// Whether two addresses name one actor: the same index, or the same
// identity, as an actor of one node and its proxy on another have. What
// `==` on two `eff[E]` answers.
// @compilerrules [net-addr-routable]
export fn same_actor(a: Int, b: Int) [] -> Bool => a, b {
    if a == b {
        return true
    }
    if !is_proxy(copy(a)) && !is_proxy(copy(b)) {
        return false
    }
    return identity(a) == identity(b)
}

// The local index for an address decoded off the wire, as identity
// ([node], [actor], [bits]).
//
// The actor itself, when the identity is this node's and the bits match;
// the one shared dead entry, when it is this node's and they do not (a
// forged or unknown address); otherwise a proxy, made the first time the
// identity is seen, which asks the actor's node for credits.
// @compilerrules [net-addr-routable, net-addr-capability]
export fn import_addr(node: Long, actor: Long, bits: Long) [] -> Int => !node, !actor, !bits {
    let r = RemoteRef { node: node, actor: actor, bits: bits }
    let here = here_node()
    let found = find_import(copy(r), copy(here))
    if found is Found f {
        return copy(f.idx)
    }
    let idx = spawn_inert()
    if found is MakeDead {
        kill_actor(copy(idx), "unknown identity")
        return register_dead(idx)
    }
    let got = register_proxy(r, copy(idx), here)
    if got == idx {
        mark_proxy(idx)
    }
    flush()
    return got
}

// Whether the actor at [addr] is a proxy for an actor on another node.
// @compilerrules [net-addr-routable]
export fn remote(addr: Int) [] -> Bool => addr {
    return is_proxy(addr)
}

// Sends [payload], a message already encoded under protocol [proto], to
// the actor proxy [addr] stands for.
//
// It spends a credit, waiting for one first if there is none: that is how a
// remote mailbox's capacity slows a sender. A send through a proxy that
// has died (its node left) does nothing, and so does one through an
// address that is not a proxy.
// @compilerrules [net-addr-routable, net-remote-backpressure]
export fn send_remote(addr: Int, proto: Str, payload: Bytes) [] -> None => !addr, !proto, !payload {
    let r = proxy_ref(copy(addr))
    if r is None {
        return
    }
    while true {
        let got = take_credit(copy(addr), this_parker())
        if got == 1 {
            let from = here_node()
            let frame = encode<Frame>(MsgFrame { to: copy(r.node), actor: copy(r.actor), bits: copy(r.bits), from: copy(from), proto: proto, payload: payload })
            stage(from, copy(r.node), frame)
            flush()
            return
        }
        if got < 0 || mailbox_dead(copy(addr)) {
            return
        }
        park(this_parker())
        if mailbox_dead(copy(addr)) {
            return
        }
    }
}

// Answers a reply minted on another node with [payload], the answer
// already encoded: an answer frame back to the node in [t].
// @compilerrules [net-addr-routable, net-wire-format]
export fn answer_remote(t: ReplyParts, payload: Bytes) [] -> None => !t, !payload {
    let from = here_node()
    let frame = encode<Frame>(AnswerFrame { to: copy(t.node), kind: copy(t.kind), id: copy(t.id), slot: copy(t.slot), bits: copy(t.bits), payload: payload })
    stage(from, copy(t.node), frame)
    flush()
}

// The wire form of a reply about to leave this node. A task's body is kept
// here, keyed by the slot, until its answer comes back.
// @compilerrules [net-addr-routable]
export fn export_reply(e: Exported) [] -> ReplyParts => !e {
    let {kind, id, slot, body} = e
    if kind == 2 {
        if body is Body b {
            put_task(copy(slot), ExportedTask { pool: copy(id), body: b })
        }
        return ReplyParts { node: node_of_pool(copy(id)), kind: 2, id: copy(slot), slot: slot, bits: 0 }
    }
    if body is Body b {
        drop_body(b)
    }
    if kind == 1 {
        return ReplyParts { node: node_of_pool(waiter_pool(copy(id))), kind: 1, id: to_long(id), slot: slot, bits: 0 }
    }
    let me = identity(id)
    return ReplyParts { node: copy(me.node), kind: 0, id: copy(me.actor), slot: slot, bits: copy(me.bits) }
}

// Grants node [from] one credit back for the actor at [addr]: the
// scheduler's call when a message that came from [from] leaves the actor's
// mailbox. The scheduler's lock is held, so the grant is only staged;
// [flush] sends it.
// @compilerrules [net-remote-backpressure]
export fn credit_back(addr: Int, pool: Int, from: Long) [] -> None => !addr, !pool, !from {
    grant(addr, pool, from, 1)
}

// Sends every staged frame through its node's outbound hook. Called with
// no lock held.
export fn flush() [] -> None {
    let out = take_outbox()
    for s in out {
        wire_out(copy(s.from), copy(s.to), copy(s.frame))
    }
}

// Records where frames for [node] go: its endpoint, encoded in [at]. Frames
// that were waiting for the route are sent.
// @compilerrules [net-addr-routable]
export fn route(node: Long, at: Bytes) [] -> None => !node, !at {
    add_route(node, at)
    flush()
}

// Records that the current node's outbound hook is bound, so its frames
// can leave. Frames that were waiting for it are sent.
// @compilerrules [net-addr-routable]
export fn outbound_bound() [] -> None {
    set_outbound(here_node())
    flush()
}

// Whether the current node is on the wire: its outbound hook is bound.
// What [std.net]'s `connected` answers.
// @compilerrules [net-connect]
export fn connected() [] -> Bool {
    return has_outbound(here_node())
}

// How many credits the proxy at [addr] holds: how many messages it may
// send before it waits. `None` for an actor that is not a proxy.
// @compilerrules [net-remote-backpressure]
export fn credits(addr: Int) [] -> Int? => addr {
    return credits_of(addr)
}

// How much work is in front of the actor at [addr]: for a local actor, the
// messages in its mailbox; for a proxy, the messages sent through it whose
// credit has not come back. What [std.net]'s `pending` answers, and what a
// least-loaded route reads.
// @compilerrules [actor-group, net-remote-backpressure]
export fn pending(addr: Int) [] -> Int => addr {
    if is_proxy(copy(addr)) {
        return held(addr)
    }
    return mailbox_queued(addr)
}

// Sends the current node's control frames on [channel] to the actor at
// [sink].
//
// Each arrives as a message to the handler's private `control(from, data)`
// member: how a node group or an actor group hears from its peers.
// @compilerrules [node-group, actor-group]
export fn watch_control(channel: Str, sink: Int) [] -> None => !channel, !sink {
    watch_channel(here_node(), channel, sink)
}

// Sends [payload] on [channel] from the current node to node [to], as a
// control frame.
// @compilerrules [node-group, actor-group]
export fn send_control(to: Long, channel: Str, payload: Bytes) [] -> None => !to, !channel, !payload {
    let from = here_node()
    let frame = encode<Frame>(ControlFrame { to: copy(to), from: copy(from), channel: channel, payload: payload })
    stage(from, to, frame)
    flush()
}

// A control frame on [channel] for whichever node receives it, encoded:
// the first frame of a handshake, which the caller hands to the transport
// itself, since the peer has no route yet.
// @compilerrules [node-group]
export fn control_frame(channel: Str, payload: Bytes) [] -> Bytes => !channel, !payload {
    return encode<Frame>(ControlFrame { to: 0, from: here_node(), channel: channel, payload: payload })
}

// Forgets [node], which has left: its route and protocol table go, and
// every proxy for an actor on it dies with the reason "node left", so its
// watchers are sent an `Exit`.
// @compilerrules [node-group, node-exit]
export fn node_left(node: Long) [] -> None => !node {
    for idx in forget_node(node) {
        kill_actor(copy(idx), "node left")
    }
}

// Registers this program's protocol table, (name, hash) pairs, at
// start-up: what a handshake tells a peer.
// @compilerrules [actor-protocol-hash]
export fn register_protocols(table: List<(Str, Str)>) [] -> None => !table {
    set_protocols(table)
}

// This program's protocol table, as registered.
// @compilerrules [actor-protocol-hash]
export fn local_protocols() [] -> List<(Str, Str)> {
    return protocols()
}

// Records [node]'s protocol table, as its handshake carried it.
// @compilerrules [actor-protocol-hash]
export fn set_peer_protocols(node: Long, table: List<(Str, Str)>) [] -> None => !node, !table {
    set_peer(node, table)
}

// [node]'s hash for [protocol]: `None` for a peer not met or a protocol it
// does not have. Two nodes may talk on a protocol only when the hashes
// agree.
// @compilerrules [actor-protocol-hash]
export fn peer_protocol(node: Long, protocol: Str) [] -> Str? => node, protocol {
    return peer_hash(node, protocol)
}

// Delivers one frame that arrived on the wire, and answers whether it was
// delivered.
//
// A frame for a node not hosted here, for a target that does not exist,
// with bits that do not match, or whose payload does not decode is dropped,
// never delivered wrong. Payloads are decoded with no lock held, since an
// address or a reply inside one asks this module for its identity.
// @compilerrules [net-addr-routable, net-wire-format, backend-never-wrong]
export fn deliver(data: Bytes) [] -> Bool => data {
    let f = decode<Frame>(data)
    if f is MsgFrame m {
        if !accepts(copy(m.to), copy(m.actor), copy(m.bits)) {
            return false
        }
        let idx = to_int(m.actor)
        let msg = decode_message(copy(idx), copy(m.proto), copy(m.payload))
        if msg is Dyn v {
            received(copy(idx))
            let _queued = deliver_remote(idx, v, copy(m.from))
            return true
        }
        return false
    }
    if f is AnswerFrame a {
        return deliver_answer(a)
    }
    if f is GrantFrame g {
        return credited(RemoteRef { node: copy(g.host), actor: copy(g.actor), bits: copy(g.bits) }, copy(g.to), copy(g.n))
    }
    if f is OpenFrame o {
        if !accepts(copy(o.to), copy(o.actor), copy(o.bits)) {
            return false
        }
        // [net-remote-backpressure] The first grant: the room left after what is
        // queued and what other senders hold, and at least one, so a starved
        // sender is never stuck.
        let idx = to_int(o.actor)
        let room = mailbox_room(copy(idx)) - held(copy(idx))
        if room < 1 {
            room = 1
        }
        grant(copy(idx), actor_pool(copy(idx)), copy(o.from), room)
        flush()
        return true
    }
    if f is ControlFrame c {
        let node = copy(c.to)
        if node == 0 {
            node = here_node()
        }
        if !hosts(copy(node)) {
            return false
        }
        let sink = channel_sink(node, copy(c.channel))
        if sink < 0 {
            return false
        }
        let msg = control_message(copy(sink), copy(c.from), copy(c.payload))
        if msg is Dyn v {
            let queued = deliver_remote(sink, v, to_long(-1))
            return queued
        }
        return false
    }
    return false
}

fn deliver_answer(a: AnswerFrame) [] -> Bool => a {
    if !hosts(copy(a.to)) {
        return false
    }
    if a.kind == 0 {
        if !accepts(copy(a.to), copy(a.id), copy(a.bits)) {
            return false
        }
        answer(token_to_actor(to_int(a.id), copy(a.slot)), raw_answer(copy(a.payload)))
        return true
    }
    if a.kind == 1 {
        let wid = to_int(a.id)
        let v = decode_waiter_answer(copy(wid), copy(a.payload))
        if v is Dyn value {
            answer(token_to_waiter(wid, copy(a.slot)), value)
            return true
        }
        return false
    }
    let t = take_task(copy(a.id))
    if t is ExportedTask task {
        let v = decode_task_answer(copy(a.id), copy(a.payload))
        let {pool, body} = task
        if v is Dyn value {
            answer(mint_task_on(pool, body), value)
            return true
        }
        drop_body(body)
    }
    return false
}

// ---- the routers' views
//
// What a router reads on every send: each actor group's members,
// mirrored here by its replica, and a version that moves whenever they
// change or the group is refreshed.
// @compilerrules [actor-router]

// Sets the mirrored [members] of [group], by actor index: the group's
// replica calls it after every change. The view's version moves, and
// senders waiting for it wake.
// @compilerrules [actor-router]
export fn view_set(group: Int, members: List<Int>) [] -> None => !group, !members {
    set_view(group, members)
}

// The mirrored members of [group], sorted by (node, actor index): the same
// order on every node and both backends, so a selector that picks by
// position picks alike everywhere. It sorts on every call, in time
// quadratic in the number of members.
// @compilerrules [actor-router]
export fn view_members(group: Int) [] -> List<Int> => group {
    let left: Mut List<Int> = Mut List()
    for m in view_of(group) {
        add(left, copy(m))
    }
    let out: Mut List<Int> = Mut List()
    while size(left) > 0 {
        let best = 0
        let i = 1
        while i < size(left) {
            if before(identity(copy(get(left, i) ?: unreachable!())), identity(copy(get(left, best) ?: unreachable!()))) {
                best = copy(i)
            }
            i = i + 1
        }
        add(out, remove_at(left, best) ?: unreachable!())
    }
    return copy(out)
}

fn before(a: RemoteRef, b: RemoteRef) [] -> Bool => a, b {
    if a.node != b.node {
        return a.node < b.node
    }
    return a.actor <= b.actor
}

// [group]'s view version: moved by every change of its members and by
// every refresh. A route selector compares it to the last it saw.
// @compilerrules [actor-router]
export fn view_version(group: Int) [] -> Long => group {
    return version_of(group)
}

// Moves [group]'s view version without changing its members.
//
// For something a selector reads outside the view, such as an election's
// leader: whoever decides it calls this when the answer changes, and every
// route over the group looks again.
// @compilerrules [actor-router]
export fn view_refresh(group: Int) [] -> None => !group {
    bump(group)
}

// Parks the calling thread until [group]'s version moves past [seen], or
// [nanos] nanoseconds pass, whichever is first. Answers at once if the
// version has already moved. How a send waits when its selector finds no
// member.
// @compilerrules [actor-router]
export fn view_wait(group: Int, seen: Long, nanos: Long) [] -> None => !group, !seen, !nanos {
    let me = this_parker()
    let id = add_view_waiter(group, seen, copy(me))
    if id < 0 {
        return
    }
    park_nanos(me, nanos)
    drop_view_waiter(id)
}
