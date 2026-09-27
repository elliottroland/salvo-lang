# cluster

Actors across machines, in one program: two virtual nodes joined by the
in-memory transport, and every pattern the network sequence was built for —
a **singleton** behind an election, **shards** by key, **scatter** (map/reduce),
a **hedge**, and **failover** when a node leaves. Nothing in it would change over
`HostTcpTransport` but the three lines that build the transport.

Run it:

```bash
cargo run -- run --backend rust   --src examples/cluster/salvo
cargo run -- run --backend kotlin --src examples/cluster/salvo
```

## What to look for

**The client side is written against the protocol alone.** `fresh_id`,
`checkout`, `count` and `find` declare `[any Sequencer]`, `[any Inventory]`,
`[any Search]`, `[any Lookup]` and nothing else: no node, no group, no policy.
`any` is the honest claim for a fleet — each send may go to a different member,
so the function assumes no order between its sends — and it is what lets a
router be bound where one actor used to be. A function that *did* rely on order
would write the bare `[Sequencer]`, and the compiler would refuse to bind a
router under it (`examples/actors/` has the single-actor shape).

**Two nodes, one group each.** Node `a` is `main`'s own; node `b` is booted on
a second node id by `Booting`. Each attaches a replica of every actor group —
`attach(protocol<Sequencer>(), nodes)` — and the replicas find each other by
name through the node group, so `seq.members(out)` on `a` lists `b`'s member
too. The `protocol<E>()` call is also where a protocol that could not cross a
node boundary would be refused.

**1 — singleton.** Every node hosts a `Sequencing`; ids must come from one.
`use LastHost(nodes, a)` binds `Leader` — an election a child could run, the
node whose host name sorts last — and `use Elected<Sequencer>()` plus
`use route(seq)` binds `Sequencer` to the member on the leader's node. A Raft
would serve the same `Leader` effect, and `two_ids` would not change.

**2 — sharded.** `Key` on `reserve`'s first parameter says the SKU decides the
shard. `Sharded<Inventory>` hashes it into a slot over the members, so the
same SKU lands on the same shard every time — the output shows apple and apple
together, and a shard's count climbing with its own keys — while *which*
physical member is shard 0 depends on the members' order, which this
in-process fleet does not pin down (so the program prints the invariant, not
the name). `Key` is erased from the type: callers pass a plain `Str`.

**3 — scatter.** `route` picks *one* member. A policy that must read the
message — fan it out — is a handler `of any Search` you write: `Scattering`
asks the replica for the members and hands the query to `Gathering`, an actor
that mints one continuation per member (`replyto partial()`), sums the
answers, and discharges the caller's reply once. The reply token waits in a
`Mut List<Reply<Int>>` in the actor's state: a linear value in state lives in
a container, and `remove_first` moves it out.

**4 — hedge.** The same shape with two members racing: `Racing` sends both,
the first answer is sent on, the second finds nothing to send to — a `Reply`
is discharged exactly once, so there is no double answer to guard against. The
slow replica parks its answer on the timer with `replyto answer(key, out)`,
where `answer` is a **private** send member — one no face declares, so nothing
outside the handler can send to it; `partial` and `first` are the same shape.

**5 — failover.** `b` leaves its node group. Every replica hears it through
`NodeChanges` and withdraws the members `b` hosted; the election now answers
`a`; and the singleton's ids come from `a`'s sequencer — with nothing rebound.

**Two policies take turns.** `Pick<Sequencer>` and `Pick<Inventory>` are one
type in the generated code (a type argument that is an effect is erased), so
one scope holds one of them; `two_ids` and `shop` are functions for that reason,
and `Leader` reaches `Elected` through `two_ids`'s signature.
