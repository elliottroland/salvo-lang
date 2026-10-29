// [actor-route-stub] The refusals, and what is no longer one (user decision R7):
// `use any` of something that is not a group handle; a protocol member
// marking two `Key` parameters, refused at the effect whether or not
// anything routes it; and a `use any h` in a module that never spells the
// protocol, which compiles now that the route is derived from `h`'s type.

//--- use_any_binds_a_group_handle
import std.net

fn main() [use] {
    let x = 3
    use LeastLoaded<NodeGroup>(true)
    //~ error[actor-route-stub:not-a-group]
    use any x
}

//--- a_protocol_member_marks_one_key
// Two `Key`s, and nothing routes `Two`.
import std.net

actor effect Two {
    //~ error[actor-route-stub:several-keys]
    send fn go(a: Key Str, b: Key Int) => !a, !b
}

fn main() {
}

//--- a_route_needs_no_spelling_of_the_protocol
// The group's type is spelled only in `lib`: `main` routes it all the same.
import std.net
import lib.Ping
import lib.some_group

fn main() [use, spawn] {
    use LeastLoaded<Ping>(true)
    let h = some_group()
    use any h
}
// FILE: lib.sv
import std.net

export actor effect Ping {
    send fn ping(out: Reply<Str>) => !out
}

export fn some_group() [spawn] -> eff[ActorGroup<Ping>] {
    return actor_group<Ping>(spawn StaticNodeGroup("x", List(NodeEndpoint { host: "a", port: 1 })) with MemTransport(NodeEndpoint { host: "a", port: 1 }, (spawn MemNetwork() on pool(1))) on pool(1))
}

//--- use_any_binds_the_weak_face_of_a_group
// [actor-route-stub] [effect-any] `use any g` binds `any Ping` through the
// policy in scope.
import std.net

actor effect Ping {
    send fn ping(k: Key Str, out: Reply<Str>) => !k, !out
}

fn one() [any Ping] -> Str {
    return waitfor out: Reply<Str> { ping("a", out) }
}

fn strong() [Ping] -> Str {
    return waitfor out: Reply<Str> { ping("a", out) }
}

fn main(g: eff[ActorGroup<Ping>]) [use]
=> !g {
    use Sharded<Ping>()
    use any g
    let _a = one()
}

//--- a_policy_may_override_its_waits
import std.net

actor effect Ping {
    send fn ping(k: Key Str, out: Reply<Str>) => !k, !out
}

fn one() [any Ping] -> Str {
    return waitfor out: Reply<Str> { ping("a", out) }
}

fn strong() [Ping] -> Str {
    return waitfor out: Reply<Str> { ping("a", out) }
}

fn main(g: eff[ActorGroup<Ping>]) [use]
=> !g {
    use Sharded<Ping>(config = RouteConfig {})
    use any g
    let _a = one()
}

//--- a_route_binds_no_strong_face
// The route binds `any Ping`: a bare `[Ping]` is refused [effect-any].
import std.net

actor effect Ping {
    send fn ping(k: Key Str, out: Reply<Str>) => !k, !out
}

fn one() [any Ping] -> Str {
    return waitfor out: Reply<Str> { ping("a", out) }
}

fn strong() [Ping] -> Str {
    return waitfor out: Reply<Str> { ping("a", out) }
}

fn main(g: eff[ActorGroup<Ping>]) [use]
=> !g {
    use Sharded<Ping>()
    use any g
    //~ error[effect-any:single-instance-callee]
    let _a = strong()
}

//--- a_route_needs_a_selector
// No policy in scope: the route's one dependency is missing — reported once,
// the calls after it reading the face it would have bound (until 2026-10-28
// `one()` was also told no handler was in scope).
import std.net

actor effect Ping {
    send fn ping(k: Key Str, out: Reply<Str>) => !k, !out
}

fn one() [any Ping] -> Str {
    return waitfor out: Reply<Str> { ping("a", out) }
}

fn strong() [Ping] -> Str {
    return waitfor out: Reply<Str> { ping("a", out) }
}

fn main(g: eff[ActorGroup<Ping>]) [use]
=> !g {
    //~ error[actor-route-stub:no-selector]
    use any g
    let _a = one()
}
