// [actor-group] [net-noremote] The **crossing site**: naming a protocol with a
// `noremote` payload for a group is refused at `protocol<E>()`, by member and
// type — the protocol itself stays a legal local one (user decision
// 2026-09-26: the check lands at the binding, not the declaration).
import std.net

noremote struct Canvas { n: Int }

actor effect Painter { send fn paint(c: Canvas) => !c }

actor effect Counter { send fn bump(n: Int) }

fn named() -> None {
    let _ok = protocol<Counter>()
    //~ error[net-noremote:group-across-nodes]
    let _bad = protocol<Painter>()
}

// [actor-group] [implicit-resolve] The same site one level up:
// `actor_group<E>(…)`, where a program actually writes the protocol. Now
// an ordinary fn over `?protocol: () -> Protocol<E>`, so the refusal fires
// where the intrinsic `protocol<E>` is resolved to fill it — and names
// the call the program wrote.
fn opened(nodes: eff[NodeGroup]) [spawn] -> None
=> !nodes {
    //~ error[net-noremote:group-across-nodes]
    let _bad = actor_group<Painter>(nodes)
}

// [implicit-forward] A generic fn of the program's own forwards its
// `?protocol` into `actor_group<E>` without resolving anything (nothing
// about an opaque `E` is knowable); the refusal lands at *its* concrete
// call, and a wire-form protocol passes through the same path.
fn open<E>(nodes: eff[NodeGroup], ?protocol: () -> Protocol<E>) [spawn] -> eff[ActorGroup<E>] => !nodes {
    return actor_group<E>(nodes)
}

fn forwarded(a: eff[NodeGroup], b: eff[NodeGroup]) [spawn] -> None
=> !a, !b {
    let _ok = open<Counter>(a)
    //~ error[net-noremote:group-across-nodes]
    let _bad = open<Painter>(b)
}
