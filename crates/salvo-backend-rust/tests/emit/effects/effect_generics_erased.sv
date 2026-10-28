// An effect generic over an effect, erased to monomorphic items
// [effect-generic-decl].

//--- effect_only_generics_erase_to_monomorphic_items
// [effect-generic-decl] An effect generic over an *effect*, erased for the
// output: `Reg<E>` with `eff[E]` payloads and `Registering<E>` are emitted as
// monomorphic items, and `spawn Registering<Ping>()` needs no turbofish.
actor effect Ping { send fn ping(n: Int) => !n
    send fn count(out: Reply<Int>) => !out }
handler Pinging() [Console] of Ping {
    mailbox { capacity: 4 }
    state {
        seen: Int = 0
    }
    send fn ping(n: Int) => !n {
        seen = seen + 1
        println("ping ${n}")
    }
    send fn count(out: Reply<Int>) => !out { out.send(seen) }
}

//~ emits(main.rs): pub enum __Msg_Reg {
//~ emits(main.rs): pub trait __Stateless_Reg: Send + Sync {
// An erased declaration keeps no type arguments.
//~ !emits(main.rs): Reg<
actor effect Reg<E> {
    send fn join(member: eff[E]) => !member
    send fn members(out: Reply<List<eff[E]>>) => !out
}

//~ emits(main.rs): pub struct Registering {
//~ !emits(main.rs): Registering::<
handler Registering<E>() of Reg<E> {
    mailbox { capacity: 8 }
    state {
        all: Mut List<eff[E]> = Mut List()
    }
    send fn join(member: eff[E]) => !member { add(all, member) }
    send fn members(out: Reply<List<eff[E]>>) => !out { out.send(copy(all)) }
}

fn main() [use, spawn] {
    use StdConsole()
    let p = pool(1)
    let reg = spawn Registering<Ping>() on p
    let a = spawn Pinging() on p
    reg.join(copy(a))
    let ms = waitfor out: Reply<List<eff[Ping]>> { reg.members(out) }
    println("members: ${size(ms)}")
    for m in ms { m.ping(7) }
    let _seen = waitfor out: Reply<Int> { a.count(out) }
}
