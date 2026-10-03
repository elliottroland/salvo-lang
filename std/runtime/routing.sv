// `runtime.routing`: addrs and answers across nodes, a service on the
// runtime's core [runtime-layers] (RUNTIME.md §11.5 step 11). It keeps who is
// where — the node every pool belongs to, the capability bits of every actor
// whose addr has left its node, the proxies standing for actors elsewhere and
// the credits each holds — and turns a send to a proxy, an answer to a token
// minted elsewhere, a credit and a handshake into **frames**: Salvo values in
// the canonical encoding [wire-format] (D4), the same bytes on both backends.
//
// What stays with the host is what only the generated code knows: how to
// encode a message of a given protocol, how to decode one for a given actor
// or waiting frame, and where a node's frames leave the process. Those are
// the platform fns below.
//
// Lock order: the core's scheduler, then this module's table — never the
// reverse. No member of the table calls the core; the exported functions do,
// outside it. The core calls [credit_back] with its own lock held, which is
// why staging a frame and sending it are separate: frames are sent by
// [flush], with no lock held, through the outbound hook.

import runtime

// [wire-format] The codec intrinsics, as `net` declares them: private here,
// since this module sits below `net`.
intrinsic fn encode<T>(value: T) [] -> Bytes => !value
intrinsic fn decode<T>(data: Bytes) [] -> T? => data

// ---- what only the host knows

// [wire-format] Decodes a message of protocol [proto] for hosted actor
// [addr], with the decoder its spawn registered: `None` when it has none or
// the bytes are malformed.
platform fn decode_message(addr: Int, proto: Str, payload: Bytes) [] -> Dyn? => addr, proto, payload

// [wire-format] Decodes an answer for waiting frame [wid] with the decoder
// its `waitfor` registered.
platform fn decode_waiter_answer(wid: Int, payload: Bytes) [] -> Dyn? => wid, payload

// [wire-format] Decodes an answer for the task exported under [key].
platform fn decode_task_answer(key: Long, payload: Bytes) [] -> Dyn? => key, payload

// [wire-format] An answer for an actor's continuation, still as bytes: the
// actor decodes it in its activation, since only it knows the type.
platform fn raw_answer(payload: Bytes) [] -> Dyn => !payload

// [node-group] [actor-group] The message a control frame's payload arrives
// as at [sink]: its handler's private `control(from, data)` member.
platform fn control_message(sink: Int, from: Long, payload: Bytes) [] -> Dyn? => sink, from, payload

// [addr-routable] Hands [frame] to node [from]'s outbound hook, for endpoint
// [to]: the transport moves it.
platform fn wire_out(from: Long, to: Bytes, frame: Bytes) [] -> None => from, !to, !frame

// ---- the frames [wire-format] (D4)

// 0: a message for a hosted actor, encoded with its protocol's hash.
struct RtMsgFrame { to: Long, actor: Long, bits: Long, from: Long, proto: Str, payload: Bytes }
// 1: an answer for a token minted on the node it goes to: an actor's
// continuation (kind 0), a waiting frame (1) or an exported task (2).
struct RtAnswerFrame { to: Long, kind: Int, id: Long, slot: Long, bits: Long, payload: Bytes }
// 2: credits for a proxy of actor (host, actor, bits) on node [to].
struct RtGrantFrame { to: Long, host: Long, actor: Long, bits: Long, n: Int }
// 3: a new proxy asks for its first credits.
struct RtOpenFrame { to: Long, actor: Long, bits: Long, from: Long }
// 4: a payload for the actor listening on [channel] at node [to] (0: the
// receiving node itself, a HELLO's).
struct RtControlFrame { to: Long, from: Long, channel: Str, payload: Bytes }
type RtFrame = RtMsgFrame | RtAnswerFrame | RtGrantFrame | RtOpenFrame | RtControlFrame

// [addr-routable] The routable identity of an actor.
export struct RtRemoteRef : Hashed<self> by auto {
    node: Long,
    actor: Long,
    bits: Long
}

// [addr-routable] A reply token's wire form.
export struct RtReplyParts {
    node: Long,
    kind: Int,
    id: Long,
    slot: Long,
    bits: Long
}

struct RtControlKey : Hashed<self> by auto {
    node: Long,
    channel: Str
}

// A frame waiting for its route or its node's outbound hook.
struct RtParked { from: Long, to: Long, frame: Bytes }
// A frame ready to go, sent by [flush].
struct RtStaged { from: Long, to: Bytes, frame: Bytes }

// [addr-routable] A task whose token crossed the wire: its body waits here
// for the answer.
linear struct RtExportedTask { pool: Int, body: RtBody }

fn drop_exported_task(t: RtExportedTask) [] -> None => !t {
    let {pool, body} = t
    drop_body(body)
}

// What importing an identity needs done outside the table.
export struct RtFound { idx: Int }
struct RtMakeProxy {}
struct RtMakeDead {}
type RtImport = RtFound | RtMakeProxy | RtMakeDead

effect RtRouteTable {
    fn node_of_pool(pool: Int) -> Long => pool
    fn adopt_pool(pool: Int, node: Long) -> None => !pool, !node
    fn add_node() -> Long
    fn hosts(node: Long) -> Bool => node
    fn identity_of(addr: Int, pool: Int) -> RtRemoteRef => addr, pool
    fn find_import(r: RtRemoteRef, here: Long) -> RtImport => r, here
    fn register_proxy(r: RtRemoteRef, idx: Int, here: Long) -> Int => !r, !idx, !here
    fn register_dead(idx: Int) -> Int => !idx
    fn is_proxy(addr: Int) -> Bool => addr
    fn proxy_ref(addr: Int) -> RtRemoteRef? => addr
    // 1: a credit taken; 0: none, [me] recorded to be woken; -1: not a proxy.
    fn take_credit(addr: Int, me: Parker) -> Int => addr, !me
    fn stage(from: Long, to: Long, frame: Bytes) -> None => !from, !to, !frame
    fn grant(addr: Int, pool: Int, from: Long, n: Int) -> None => !addr, !pool, !from, !n
    fn take_outbox() -> Mut List<RtStaged>
    fn add_route(node: Long, at: Bytes) -> None => !node, !at
    fn set_outbound(node: Long) -> None => !node
    fn has_outbound(node: Long) -> Bool => node
    fn accepts(to: Long, actor: Long, claimed: Long) -> Bool => to, actor, claimed
    fn received(idx: Int) -> None => !idx
    fn credited(r: RtRemoteRef, to: Long, n: Int) -> Bool => !r, !to, !n
    fn held(idx: Int) -> Int => idx
    fn put_task(key: Long, t: RtExportedTask) -> None => !key, !t
    fn take_task(key: Long) -> RtExportedTask? => key
    fn watch_channel(node: Long, channel: Str, sink: Int) -> None => !node, !channel, !sink
    fn channel_sink(node: Long, channel: Str) -> Int => node, channel
    fn set_protocols(table: List<(Str, Str)>) -> None => !table
    fn protocols() -> List<(Str, Str)>
    fn set_peer(node: Long, table: List<(Str, Str)>) -> None => !node, !table
    fn peer_hash(node: Long, protocol: Str) -> Str? => node, protocol
    fn forget_node(node: Long) -> List<Int> => node
    fn credits_of(addr: Int) -> Int? => addr
}

handler RtRoutes() of RtRouteTable {
    node_id: Long = 0
    hosted: Mut Set<Long> = mut_set_of()
    pool_node: Mut Map<Int, Long> = mut_map_of()
    bits: Mut Map<Int, Long> = mut_map_of()
    remote: Mut Map<Int, RtRemoteRef> = mut_map_of()
    proxies: Mut Map<RtRemoteRef, Int> = mut_map_of()
    credits: Mut Map<Int, Int> = mut_map_of()
    // For a hosted actor, the credits remote senders still hold; for a
    // proxy, the messages in flight to it.
    held_n: Mut Map<Int, Int> = mut_map_of()
    routes: Mut Map<Long, Bytes> = mut_map_of()
    outbound: Mut Set<Long> = mut_set_of()
    parked: Mut List<RtParked> = mut_list_of()
    outbox: Mut List<RtStaged> = mut_list_of()
    // Exported tasks and their keys, index-aligned: a map of linear values
    // does not build on Rust yet (ROADMAP 0c).
    task_keys: Mut List<Long> = mut_list_of()
    tasks: Mut List<RtExportedTask> = mut_list_of()
    controls: Mut Map<RtControlKey, Int> = mut_map_of()
    local: List<(Str, Str)> = []
    peers: Mut Map<Long, List<(Str, Str)>> = mut_map_of()
    dead_entry: Int = -1
    credit_waiters: Mut List<Parker> = mut_list_of()

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

    fn identity_of(addr: Int, pool: Int) -> RtRemoteRef => addr, pool {
        return identity_in(remote, bits, pool_node, node_id, addr, pool)
    }

    fn find_import(r: RtRemoteRef, here: Long) -> RtImport => r, here {
        if r.node == here {
            let idx = to_int(r.actor)
            let b = get(bits, idx)
            if !contains_key(remote, idx) && b is Long known {
                if known == r.bits {
                    return RtFound { idx: idx }
                }
            }
            if dead_entry >= 0 {
                return RtFound { idx: copy(dead_entry) }
            }
            return RtMakeDead {}
        }
        let p = get(proxies, r)
        if p is Int idx {
            return RtFound { idx: copy(idx) }
        }
        return RtMakeProxy {}
    }

    fn register_proxy(r: RtRemoteRef, idx: Int, here: Long) -> Int => !r, !idx, !here {
        let existing = get(proxies, r)
        if existing is Int e {
            // Another thread imported it meanwhile; the spare entry is unused.
            return copy(e)
        }
        put(remote, copy(idx), copy(r))
        put(proxies, copy(r), copy(idx))
        put(credits, copy(idx), 0)
        let frame = encode<RtFrame>(RtOpenFrame { to: copy(r.node), actor: copy(r.actor), bits: copy(r.bits), from: copy(here) })
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

    fn proxy_ref(addr: Int) -> RtRemoteRef? => addr {
        let r = get(remote, addr)
        if r is RtRemoteRef found {
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
        let frame = encode<RtFrame>(RtGrantFrame { to: copy(from), host: copy(me.node), actor: copy(me.actor), bits: copy(me.bits), n: n })
        stage_in(routes, outbound, outbox, parked, copy(me.node), from, frame)
    }

    fn take_outbox() -> Mut List<RtStaged> {
        let out: Mut List<RtStaged> = mut_list_of()
        while size(outbox) > 0 {
            add(out, remove_at(outbox, 0)!)
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
        if contains_key(remote, idx) {
            return false
        }
        let b = get(bits, idx)
        if b is Long known {
            return known == claimed
        }
        return false
    }

    fn received(idx: Int) -> None => !idx {
        let h = held_in(held_n, copy(idx))
        if h > 0 {
            put(held_n, idx, h - 1)
        }
    }

    fn credited(r: RtRemoteRef, to: Long, n: Int) -> Bool => !r, !to, !n {
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

    fn put_task(key: Long, t: RtExportedTask) -> None => !key, !t {
        add(task_keys, key)
        add(tasks, t)
    }

    fn take_task(key: Long) -> RtExportedTask? => key {
        let i = 0
        while i < size(task_keys) {
            if get(task_keys, i)! == key {
                let _k = remove_at(task_keys, copy(i))
                return remove_at(tasks, i)
            }
            i = i + 1
        }
        return None
    }

    fn watch_channel(node: Long, channel: Str, sink: Int) -> None => !node, !channel, !sink {
        put(controls, RtControlKey { node: node, channel: channel }, sink)
    }

    fn channel_sink(node: Long, channel: Str) -> Int => node, channel {
        let s = get(controls, RtControlKey { node: copy(node), channel: copy(channel) })
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
        let gone: Mut List<Int> = mut_list_of()
        for idx in keys(remote) {
            let r = get(remote, idx)
            if r is RtRemoteRef found {
                if found.node == node {
                    add(gone, copy(idx))
                }
            }
        }
        wake_senders(credit_waiters)
        return gone
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

// [addr-routable] The identity of local index [addr] on [pool]: its bits
// minted the first time it is asked for, which is the first time the addr
// leaves its node.
fn identity_in(remote: Map<Int, RtRemoteRef>, bits: Mut Map<Int, Long>, pool_node: Map<Int, Long>, node_id: Long, addr: Int, pool: Int) [] -> RtRemoteRef
=> remote, bits: Mut, pool_node, node_id, addr, pool {
    let r = get(remote, addr)
    if r is RtRemoteRef found {
        return copy(found)
    }
    let n = node_in(pool_node, node_id, pool)
    let b = get(bits, addr)
    if b is Long known {
        return RtRemoteRef { node: n, actor: to_long(addr), bits: copy(known) }
    }
    let minted = identity_bits()
    put(bits, copy(addr), copy(minted))
    return RtRemoteRef { node: n, actor: to_long(addr), bits: minted }
}

// Senders waiting for credit look again: a grant arrived, or a node left.
fn wake_senders(waiters: Mut List<Parker>) [] -> None => waiters: Mut {
    while size(waiters) > 0 {
        unpark(remove_at(waiters, 0)!)
    }
}

// Stages [frame] from node [from] for node [to]: ready to send when [to] has
// a route and [from] an outbound hook, parked until then otherwise.
fn stage_in(routes: Map<Long, Bytes>, outbound: Set<Long>, outbox: Mut List<RtStaged>, parked: Mut List<RtParked>, from: Long, to: Long, frame: Bytes) [] -> None
=> routes, outbound, outbox: Mut, parked: Mut, !from, !to, !frame {
    let ep = get(routes, to)
    if ep is Bytes at {
        if contains(outbound, from) {
            add(outbox, RtStaged { from: from, to: copy(at), frame: frame })
            return
        }
    }
    add(parked, RtParked { from: from, to: to, frame: frame })
}

// Frames parked for a route or an outbound hook try again.
fn restage(routes: Map<Long, Bytes>, outbound: Set<Long>, outbox: Mut List<RtStaged>, parked: Mut List<RtParked>) [] -> None
=> routes, outbound, outbox: Mut, parked: Mut {
    let waiting: Mut List<RtParked> = mut_list_of()
    while size(parked) > 0 {
        add(waiting, remove_at(parked, 0)!)
    }
    for p in waiting {
        stage_in(routes, outbound, outbox, parked, copy(p.from), copy(p.to), copy(p.frame))
    }
}

use RtRoutes()

// [addr-routable] A fresh node identity: random and non-negative, since the
// core marks a local sender as -1.
fn fresh_node() [] -> Long {
    let b = identity_bits()
    if b < 0 {
        return -(b + 1)
    }
    return b
}

// ---- for the host's shims

// [addr-routable] The node the calling thread's pool belongs to.
export fn here_node() [] -> Long {
    return node_of_pool(current_pool())
}

// [addr-routable] A pool made by the calling thread belongs to its node.
export fn adopt(pool: Int) [] -> None => !pool {
    adopt_pool(pool, here_node())
}

// [addr-routable] Hosts a fresh virtual node in this process.
export fn new_node() [] -> Long {
    return add_node()
}

// [addr-routable] A pool of [n] workers belonging to [node].
export fn pool_at(node: Long, n: Int) [] -> Int => !node, n {
    let p = new_pool_of(n, -1)
    adopt_pool(copy(p), node)
    return p
}

// [addr-routable] The routable identity of local index [addr].
export fn identity(addr: Int) [] -> RtRemoteRef => addr {
    return identity_of(copy(addr), actor_pool(copy(addr)))
}

// [addr-routable] Whether two addrs name one actor: the same index, or the
// same identity (an actor of a virtual node and its proxy on another).
export fn same_actor(a: Int, b: Int) [] -> Bool => a, b {
    if a == b {
        return true
    }
    if !is_proxy(copy(a)) && !is_proxy(copy(b)) {
        return false
    }
    return identity(a) == identity(b)
}

// [addr-routable] [addr-capability] A decoded identity as a local index:
// the actor itself when it is this node's and the bits match (a forged one
// answers the one shared dead entry), a proxy otherwise — made once, with an
// OPEN frame asking the host for credits.
export fn import_addr(node: Long, actor: Long, bits: Long) [] -> Int => !node, !actor, !bits {
    let r = RtRemoteRef { node: node, actor: actor, bits: bits }
    let here = here_node()
    let found = find_import(copy(r), copy(here))
    if found is RtFound f {
        return copy(f.idx)
    }
    let idx = spawn_inert()
    if found is RtMakeDead {
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

// [addr-routable] Whether [addr] is a proxy of an actor elsewhere.
export fn remote(addr: Int) [] -> Bool => addr {
    return is_proxy(addr)
}

// [addr-routable] [remote-backpressure] Sends a message already encoded
// under protocol [proto] to proxy [addr], waiting for a credit first; to a
// proxy that died (its node left), the silent no-op.
export fn send_remote(addr: Int, proto: Str, payload: Bytes) [] -> None => !addr, !proto, !payload {
    let r = proxy_ref(copy(addr))
    if r is None {
        return
    }
    while true {
        let got = take_credit(copy(addr), this_parker())
        if got == 1 {
            let from = here_node()
            let frame = encode<RtFrame>(RtMsgFrame { to: copy(r.node), actor: copy(r.actor), bits: copy(r.bits), from: copy(from), proto: proto, payload: payload })
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

// [addr-routable] [wire-format] Answers a token minted elsewhere: a REPLY
// frame back to its node.
export fn answer_remote(t: RtReplyParts, payload: Bytes) [] -> None => !t, !payload {
    let from = here_node()
    let frame = encode<RtFrame>(RtAnswerFrame { to: copy(t.node), kind: copy(t.kind), id: copy(t.id), slot: copy(t.slot), bits: copy(t.bits), payload: payload })
    stage(from, copy(t.node), frame)
    flush()
}

// [addr-routable] A reply token's wire form. Exporting a task moves its body
// here until its answer returns, keyed by its slot.
export fn export_reply(e: RtExported) [] -> RtReplyParts => !e {
    let {kind, id, slot, body} = e
    if kind == 2 {
        if body is RtBody b {
            put_task(copy(slot), RtExportedTask { pool: copy(id), body: b })
        }
        return RtReplyParts { node: node_of_pool(copy(id)), kind: 2, id: copy(slot), slot: slot, bits: 0 }
    }
    if body is RtBody b {
        drop_body(b)
    }
    if kind == 1 {
        return RtReplyParts { node: node_of_pool(waiter_pool(copy(id))), kind: 1, id: to_long(id), slot: slot, bits: 0 }
    }
    let me = identity(id)
    return RtReplyParts { node: copy(me.node), kind: 0, id: copy(me.actor), slot: slot, bits: copy(me.bits) }
}

// [remote-backpressure] The core's call when a message that came from node
// [from] leaves actor [addr]'s mailbox: a credit goes back. Staged, since
// the core's lock is held; [flush] sends it.
export fn credit_back(addr: Int, pool: Int, from: Long) [] -> None => !addr, !pool, !from {
    grant(addr, pool, from, 1)
}

// Sends every staged frame, with no lock held.
export fn flush() [] -> None {
    let out = take_outbox()
    for s in out {
        wire_out(copy(s.from), copy(s.to), copy(s.frame))
    }
}

// [addr-routable] Records where node [node]'s frames go: its encoded
// endpoint.
export fn route(node: Long, at: Bytes) [] -> None => !node, !at {
    add_route(node, at)
    flush()
}

// [addr-routable] The current node's outbound hook is bound.
export fn outbound_bound() [] -> None {
    set_outbound(here_node())
    flush()
}

// [net-connect] Whether the current node's outbound hook is bound.
export fn connected() [] -> Bool {
    return has_outbound(here_node())
}

// [remote-backpressure] A proxy's credit balance, or `None` for a local
// actor.
export fn credits(addr: Int) [] -> Int? => addr {
    return credits_of(addr)
}

// [actor-group] [remote-backpressure] `pending(addr)`: messages in flight to
// a proxy, the queue's depth for a local actor.
export fn pending(addr: Int) [] -> Int => addr {
    if is_proxy(copy(addr)) {
        return held(addr)
    }
    return mailbox_queued(addr)
}

// [node-group] [actor-group] The current node's control frames on
// [channel] go to [sink].
export fn watch_control(channel: Str, sink: Int) [] -> None => !channel, !sink {
    watch_channel(here_node(), channel, sink)
}

// [node-group] [actor-group] A CONTROL frame on [channel] from the current
// node to node [to].
export fn send_control(to: Long, channel: Str, payload: Bytes) [] -> None => !to, !channel, !payload {
    let from = here_node()
    let frame = encode<RtFrame>(RtControlFrame { to: copy(to), from: copy(from), channel: channel, payload: payload })
    stage(from, to, frame)
    flush()
}

// [node-group] A CONTROL frame for whichever node receives it: a HELLO's,
// handed to the transport directly, since the peer has no route yet.
export fn control_frame(channel: Str, payload: Bytes) [] -> Bytes => !channel, !payload {
    return encode<RtFrame>(RtControlFrame { to: 0, from: here_node(), channel: channel, payload: payload })
}

// [node-group] [node-exit] Node [node] has left: its route and protocol
// table are forgotten, and every proxy of an actor on it dies.
export fn node_left(node: Long) [] -> None => !node {
    for idx in forget_node(node) {
        kill_actor(copy(idx), "node left")
    }
}

// [protocol-hash] This program's protocol table, registered at start-up.
export fn register_protocols(table: List<(Str, Str)>) [] -> None => !table {
    set_protocols(table)
}

export fn local_protocols() [] -> List<(Str, Str)> {
    return protocols()
}

// [protocol-hash] Peer [node]'s table, as its handshake carried it.
export fn set_peer_protocols(node: Long, table: List<(Str, Str)>) [] -> None => !node, !table {
    set_peer(node, table)
}

// [protocol-hash] A peer's hash for a protocol: `None` for an unknown peer
// or a protocol it does not have.
export fn peer_protocol(node: Long, protocol: Str) [] -> Str? => node, protocol {
    return peer_hash(node, protocol)
}

// [addr-routable] [wire-format] Delivers one frame that arrived on the wire.
// A frame for a node not hosted here, a target that does not exist, a bits
// mismatch or a malformed payload is dropped, never delivered wrong
// [backend-never-wrong]. Answers whether it was delivered. Payloads are
// decoded with no lock held: an `Addr` or a `Reply` in one asks this module
// for its identity.
export fn deliver(data: Bytes) [] -> Bool => data {
    let f = decode<RtFrame>(data)
    if f is RtMsgFrame m {
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
    if f is RtAnswerFrame a {
        return deliver_answer(a)
    }
    if f is RtGrantFrame g {
        return credited(RtRemoteRef { node: copy(g.host), actor: copy(g.actor), bits: copy(g.bits) }, copy(g.to), copy(g.n))
    }
    if f is RtOpenFrame o {
        if !accepts(copy(o.to), copy(o.actor), copy(o.bits)) {
            return false
        }
        // [remote-backpressure] The first grant: the room left after what is
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
    if f is RtControlFrame c {
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

fn deliver_answer(a: RtAnswerFrame) [] -> Bool => a {
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
    if t is RtExportedTask task {
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
