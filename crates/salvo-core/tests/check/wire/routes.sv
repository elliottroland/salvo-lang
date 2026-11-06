// [effect-generic-decl] Two instances of an effect whose arguments are
// effects erase to one type, so one scope holds one: the second `use` is
// refused, naming the nested-block remedy; a nested scope is fine.
actor effect Ping {
    send fn ping(out: Reply<Str>) => !out
}

actor effect Pong {
    send fn pong(out: Reply<Str>) => !out
}

effect Pick<E> {
    fn pick() -> eff[E]?
}

handler Nobody<E>() for Pick<E> {
    fn pick() -> eff[E]? {
        return None
    }
}

fn inner() [use] {
    use Nobody<Pong>()
}

fn main() [use] {
    use Nobody<Ping>()
    inner()
    //~ error[effect-generic-decl:sibling-bound]
    use Nobody<Pong>()
}
