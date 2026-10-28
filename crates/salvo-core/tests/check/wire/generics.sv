// [effect-generic-decl] An effect generic over an *effect*: `eff[E]` is the
// one position `E` may take, a generic instance is itself an effect inside
// `Addr` (`eff[Reg<Ping>]`), and a phantom struct parameter (`Protocol<E>`)
// types a value without occurring in it.
import std.net

actor effect Ping { send fn ping(n: Int) }

actor effect Reg<E> {
    send fn join(member: eff[E]) => !member
    send fn peers(out: Reply<List<eff[E]>>) => !out
    send fn twin(other: eff[Reg<E>]) => !other
}

handler Registering<E>() of Reg<E> {
    mailbox { capacity: 4 }
    state {
        all: Mut List<eff[E]> = Mut List()
    }
    send fn join(member: eff[E]) => !member { add(all, member) }
    send fn peers(out: Reply<List<eff[E]>>) => !out { out.send(copy(all)) }
    send fn twin(other: eff[Reg<E>]) => !other { }
}

fn main() [spawn] -> None {
    let _r = spawn Registering<Ping>() on pool(1)
    let _p = protocol<Ping>()
}
