// [actor-mailbox] The actor settings slot (user decision 2026-09-16).

actor effect Counter {
    send fn bump(n: Int)
    send fn total(out: Reply<Int>) => !out
}

actor effect Log {
    send fn note(what: Str) => !what
}

handler Printing() of Log {
    mailbox { capacity: 4 }

    send fn note(what: Str) {}
}

handler Counting() [Log] of Counter {
    mailbox { capacity: 16 }

    state {
        sum: Int = 0
    }

    send fn bump(n: Int) {
        sum = sum + n
        note("bumped")
    }

    send fn total(out: Reply<Int>) {
        out.send(sum)
    }
}


// [actor-mailbox] The slot in its two shapes: a literal bound, and one taken
// as a constructor parameter, which is what "optionally exposed as a
// constructor argument" means, and needs no new grammar because the block's
// expressions are checked in the constructor's scope.
handler Fixed() of Log {
    mailbox { capacity: 16 }
    send fn note(what: Str) {}
}

handler Sized(room: Int) of Log {
    mailbox { capacity: room }
    send fn note(what: Str) {}
}

fn a_mailbox_may_be_literal_or_a_constructor_parameter() [use, spawn] {
    let _a = spawn Fixed() on pool(1)
    let _b = spawn Sized(4) on pool(1)
}

// [actor-mailbox] It is **required** on a handler of an actor effect: the
// queue bound has no default, since a bound the compiler picked would be a
// performance cliff nobody wrote.
//~ error[actor-mailbox:no-capacity]
handler Quiet() of Log {
    send fn note(what: Str) {}
}

// [actor-mailbox] …and refused on a handler of a **plain** effect, where
// there is no queue: its members run on the caller's thread.
effect Plain {
    fn ask() -> Int
}

handler Answering() of Plain {
    //~ error[actor-mailbox:nothing-to-enqueue]
    mailbox { capacity: 4 }
    fn ask() -> Int {
        return 1
    }
}

// [actor-mailbox] The scope is **constructor parameters only**: the bound is
// wanted before the actor exists (the scheduler needs it at spawn time, ahead
// of every state initialiser), so a state field is not in scope.
handler Loud() of Log {
    state {
        slots: Int = 8
    }
    //~ error[name-ident-resolve:unknown]
    mailbox { capacity: slots }
    send fn note(what: Str) {}
}

// [actor-mailbox] The slot only *reads*, and that needs no rule of its own:
// [effect-state-store] already refuses consuming what the handler was built
// with, and names `copy` as the remedy. A `Copy` scalar is exempt, which is
// why the ordinary `capacity: room` shape above is fine [copy-scalar-free].
fn measure(names: List<Str>) [] -> Int => !names {
    return 1
}

handler Wasteful(names: List<Str>) of Log {
    //~ error[effect-state-store:moved-out]
    mailbox { capacity: measure(names) }
    send fn note(what: Str) {}
}

// [actor-mailbox] The slot is a struct literal, so the field diagnostics are
// the ordinary ones: a future setting is a *field* on `Mailbox` rather than
// new grammar.
handler Odd() of Log {
    //~ error[struct-defaults:missing-field]
    //~ error[struct-field-resolve:literal-no-field]
    mailbox { depth: 4 }
    send fn note(what: Str) {}
}
