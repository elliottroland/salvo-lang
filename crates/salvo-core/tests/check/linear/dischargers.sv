// [linear-group] The same-file discharge model: a linear type's dischargers
// are the fns — and the members of effects — declared in its own file that
// consume it, and a discharger's body ending ends the value (user decision
// 2026-10-30, which retired `discard`).

linear struct Lines {
    name: Str
}

fn close(l: Lines) -> None => !l {}

fn open_lines(n: Str) -> Lines => !n {
    return Lines { name: n }
}

// [linear-group] `drop` (std's uniform consuming callback) deliberately
// lacks `canbe linear`: handing it a linear value is refused by the
// instantiation ban — `drop` never discharges an obligation.
fn drop_refuses_a_linear_value() -> None {
    let l = open_lines("a")
    //~ error[linear-generics:not-opted]
    drop(l)
}

// [linear-group] A discharger may *forward* instead: the obligation
// terminates by moving into another same-file consumer.
fn shutdown(l: Lines) -> None => !l {
    close(l)
}

// [linear-group] A same-file consuming fn that does nothing with it ends it:
// consuming the value in its own file is the explicit act (user decision
// 2026-10-30, reversing 2026-09-12's "being a discharger is not an
// exemption", which `discard` was the escape from).
fn vanish(l: Lines) -> None => !l {
}

// [linear-group] Alternative dischargers: two same-file consuming fns are
// both legal terminals, and the leak diagnostic names them all.
linear struct Thread {
    id: Int
}

fn stop(t: Thread) -> None => !t {}

fn join(t: Thread) -> None => !t {}

fn start() -> Thread {
    return Thread { id: 1 }
}

fn alternative_dischargers_are_all_named() -> None {
    //~ error[ir-linear-check:leak-at-exit]
    let _t = start()
}

// [linear-group] The cache shape: a discharge with a context parameter is an
// ordinary same-file consuming fn — nothing extra to declare.
struct Cache canbe Mut {
    size: Int
}

linear struct Entry {
    id: Int
}

fn remove(cache: Mut Cache, e: Entry) -> None => cache: Mut, !e {}

fn mint() -> Entry {
    return Entry { id: 1 }
}

fn a_context_carrying_discharge_works(cache: Mut Cache) -> None => cache: Mut {
    let e = mint()
    remove(cache, e)
}

// ===== effect members as dischargers [linear-group] =====

// The amendment the bare-members decision entailed (user decision
// 2026-09-14, FILE_SYSTEM.md §5.8): a **consuming effect member** declared in
// the linear type's own file joins the discharge set, so a token whose only
// `close` is a `Sink` member has a legal death — and every handler's
// implementation of that member is a discharge context: its body ending ends
// the obligation.
export linear struct Token {
    handle: Int
}

export effect Sink {
    fn close(t: Token) -> Str => !t
    fn peek(t: Token) -> Int => t
}

handler Direct of Sink {
    fn close(t: Token) -> Str => !t {
        return "closed"
    }
    fn peek(t: Token) -> Int => t {
        return t.handle
    }
}


// [linear-group] A leak still leaks, and the hint names the *member* as the
// terminal.
fn a_leak_names_the_member_as_the_discharge() [Sink] -> Int {
    //~ error[ir-linear-check:leak-at-exit]
    let t = Token { handle: 1 }
    return peek(t)
}

// [linear-group] An **overloaded** consuming member gives each of its bodies
// its *own* contract: `close(InStream)` ends an `InStream`, not an
// `OutStream` — the fs shape exactly [effect-member-overload].
linear struct InStream {
    handle: Int
}

linear struct OutStream {
    handle: Int
}

effect Fs {
    fn close(s: InStream) -> Str => !s
    fn close(s: OutStream) -> Str => !s
}

handler MemFs of Fs {
    fn close(s: InStream) -> Str => !s {
        //~ error[ir-linear-check:leak-at-exit]
        let _other = OutStream { handle: 9 }
        return "in"
    }
    fn close(s: OutStream) -> Str => !s {
        return "out"
    }
}

// FILE: fake.sv
// [linear-group] The handler may live anywhere — a test double in another
// module discharges too, because discharger status is the *member's*.
import main.Sink
import main.Token

handler Fake of Sink {
    fn close(t: Token) -> Str => !t {
        return "faked"
    }
    fn peek(t: Token) -> Int => t {
        return 0
    }
}

// FILE: ticket.sv
// [linear-group] The same-file rule still binds: a member of an effect
// declared *elsewhere* than the type does not grant discharge, or any module
// could dispose of another's linear values.
export linear struct Ticket {
    handle: Int
}

export fn close(t: Ticket) -> None => !t {
}

// FILE: office.sv
import ticket.Ticket

effect Desk {
    fn take(t: Ticket) -> None => !t
}

handler Clerk of Desk {
    //~ error[ir-linear-check:leak-at-exit]
    fn take(t: Ticket) -> None => !t {
    }
}

// FILE: first.sv
// [linear-group] [type-identity] Two modules each declaring a linear
// `Handle`: the second is keyed by its module, and its own `close` is still
// its discharger — the discharge set is built of keys, not of the written
// name.
export linear struct Handle {
    fd: Int
}

export fn close(h: Handle) -> None => !h {
}

export fn open(fd: Int) -> Handle {
    return Handle { fd: fd }
}

// FILE: second.sv
export linear struct Handle {
    fd: Int
}

export fn close(h: Handle) -> None => !h {
}

export fn open(fd: Int) -> Handle {
    return Handle { fd: fd }
}

fn use_it() -> None {
    let h = open(1)
    close(h)
}
