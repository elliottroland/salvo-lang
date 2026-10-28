// [effect-generic-decl] Two instances of an effect whose arguments are
// effects erase to one type, so one scope holds one: the second `use` is
// refused, naming the nested-block remedy; a nested scope is fine.
import std.net

actor effect Ping {
    send fn ping(out: Reply<Str>) => !out
}

actor effect Pong {
    send fn pong(out: Reply<Str>) => !out
}

fn inner() [use] {
    use LeastLoaded<Pong>(true)
}

fn main() [use] {
    use LeastLoaded<Ping>(true)
    inner()
    //~ error[effect-generic-decl:sibling-bound]
    use Sharded<Pong>()
}
