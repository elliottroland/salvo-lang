// [linear-container] [linear-state] Linearity in **collections** (LC-1…LC-5,
// user decisions 2026-09-15/16): a container is linear exactly when its
// element type is, obligations enter with `add`/`replace`, leave one at a
// time with `remove_first`/`remove`, and the container's own terminal is
// `drain`. The surface is std's (`lib/std/core/list.sv`, `map.sv`).
//
// * **LC-1/LC-5** conditional contagion: `List<T canbe linear>` makes
//   `List<Token>` a linear type and `List<Int>` a plain one, with no use-site
//   spelling anywhere.
// * **LC-2** the surface: take-by-move answering `T?`, the displacing
//   `replace`, `drain` as the terminal, and `get` answering a projection
//   that cannot be given away (opened 2026-10-02, user decision).
// * **LC-3** which containers: `List` and `Map` *values* yes; `Set` and map
//   **keys** refused, because dedup is dropping.
// * **LC-4** handler state: a process owns its obligations, and an
//   activation leaves every state field whole.

// A linear token with a discharger.
linear struct Token { id: Int }

fn spend(t: Token) [] -> None => !t {
    discard(t)
}

fn mint(id: Int) [] -> Token {
    return Token { id: id }
}

// ===== LC-1 / LC-5: the container is linear when its element is =====

// [linear-container] The whole of LC-1 in one fn: a list of obligations
// owes, and `drain` is what discharges it — each element landing in the
// callback, which consumes it.
fn a_list_of_obligations_owes_and_drain_discharges_it() [] -> None {
    let queue: Mut List<Token> = Mut List()
    add(queue, mint(1))
    add(queue, mint(2))
    drain(queue, spend)
}

// [linear-container] …and without the terminal it is an ordinary leak, whose
// diagnostic names **`drain`** rather than the element's own discharger: the
// container is what owes.
fn an_undrained_container_is_a_leak_naming_drain() [] -> None {
    //~ error[ir-linear-check:leak-at-exit]
    let queue: Mut List<Token> = Mut List()
    add(queue, mint(1))
}

// [linear-container] The conditional half: the *same* container type with a
// plain element owes nothing, and no annotation anywhere says which — the
// element's declaration is the only source of linearity.
fn a_plain_instantiation_of_the_same_container_owes_nothing() [] -> Int {
    let ns: Mut List<Int> = Mut List()
    add(ns, 1)
    return size(ns)
}

// [linear-container] Nesting is structural: a list of lists of obligations
// owes as a whole, since the judgment recurses through the opted position.
fn nesting_carries_the_obligation_through() [] -> None {
    //~ error[ir-linear-check:leak-at-exit]
    let outer: Mut List<Mut List<Token>> = Mut List()
    add(outer, Mut List())
}

// ===== LC-2: how obligations enter, move through, and leave =====

// [linear-container] Take-by-move answers `T?`, and the absence check *is*
// the union narrow [linear-union-arm]: the `None` arm owes nothing, the value
// arm owes and is discharged in the branch.
fn take_by_move_hands_over_one_obligation() [] -> None {
    let queue: Mut List<Token> = Mut List()
    add(queue, mint(1))
    let first = remove_first(queue)
    when first {
        is Token { spend(first) }
        is None {}
    }
    drain(queue, spend)
}

// [linear-container] …and dropping what was taken is the ordinary leak: the
// obligation left the container and landed in a local.
fn a_taken_obligation_still_has_to_be_discharged() [] -> None {
    let queue: Mut List<Token> = Mut List()
    add(queue, mint(1))
    //~ error[ir-linear-check:leak-at-exit]
    let _first = remove_first(queue)
    drain(queue, spend)
}

// [linear-container] [proj-linear] `get` answers a **projection** of a linear
// element (user decision 2026-10-02): it may be read without taking on the
// obligation, and the element still owes inside its container.
fn get_answers_a_projection_of_an_obligation() [] -> Int {
    let queue: Mut List<Token> = Mut List()
    add(queue, mint(1))
    let peek = get(queue, 0)
    let id = if peek is None { 0 } else { peek.id }
    drain(queue, spend)
    return id
}

// [proj-linear] A projection of an obligation cannot be given away or
// duplicated — so no obligation is discharged twice through one — and the
// container still owes its elements.
fn a_projection_of_an_obligation_cannot_be_consumed() [] -> None {
    let queue: Mut List<Token> = Mut List()
    add(queue, mint(1))
    let peek = get(queue, 0)
    if !(peek is None) {
        //~ error[proj-type:projection-consumed]
        spend(peek)
    }
    drain(queue, spend)
}

fn a_projection_of_an_obligation_cannot_be_copied() [] -> None {
    let queue: Mut List<Token> = Mut List()
    add(queue, mint(1))
    let peek = get(queue, 0)
    if !(peek is None) {
        //~ error[linear-generics:copy-linear]
        spend(copy(peek))
    }
    drain(queue, spend)
}

fn the_container_still_owes_under_a_projection() [] -> None {
    //~ error[ir-linear-check:leak-at-exit]
    let queue: Mut List<Token> = Mut List()
    add(queue, mint(1))
    let _peek = get(queue, 0)
}

// [linear-container] A map's values may be obligations: `put` is closed to
// them (it drops what it overwrites) and `replace` is the form that hands the
// displaced value back.
fn a_map_of_obligations_writes_with_replace() [] -> None {
    let table: Mut Map<Int, Token> = Mut Map()
    let displaced = replace(table, 1, mint(1))
    when displaced {
        is Token { spend(displaced) }
        is None {}
    }
    let taken = remove(table, 1)
    when taken {
        is Token { spend(taken) }
        is None {}
    }
    drain(table, spend)
}

// [linear-container] `put` is refused for a linear value, because it answers
// nothing: what it overwrote would be dropped in silence.
fn put_is_refused_for_obligations() [] -> None {
    let table: Mut Map<Int, Token> = Mut Map()
    //~ error[linear-generics:not-opted]
    put(table, 1, mint(1))
    drain(table, spend)
}

// ===== LC-3: which containers participate =====

// [linear-container] A set is refused, and the diagnostic says **why**:
// insertion deduplicates, and dedup is a silent drop no API reshaping can
// fix. The wording is the point — this is semantics, not a fence. `Ticket` is
// hashable, so linearity is the only thing wrong with it as an element.
linear struct Ticket : Hashed<self> by auto {
    id: Int
}

fn punch(t: Ticket) [] -> None => !t {
    discard(t)
}

//~ error[linear-composite:position]
fn a_set_of_obligations_is_refused_because_dedup_is_dropping(s: Mut Set<Ticket>) [] -> None => !s {}

// [linear-container] And a map **key** is refused for the same reason, with
// the values named as where obligations go.
//~ error[linear-composite:position]
fn a_map_key_cannot_be_an_obligation(m: Mut Map<Ticket, Int>) [] -> None => !m {}

// [linear-container] Arrays and tuples stay out of the first round, so they
// keep the general composite refusal — which names the containers that *do*
// hold obligations.
//~ error[linear-container:position]
fn an_array_is_still_refused(x: Token[]) [] -> None => !x {}

//~ error[linear-container:position]
fn a_tuple_is_still_refused(x: (Token, Int)) [] -> None => !x {}

// ===== LC-4: handler state =====

// [linear-state] The customer's shape: a handler parks obligations in state,
// takes them out one at a time, and drains what is left — putting a fresh
// container back, because an activation may not leave a hole.
effect Parking {
    fn park(t: Token) -> None => !t
    fn release() -> None
    fn shutdown() -> None
}

handler Parked() of Parking {
    state {
        waiting: Mut List<Token> = Mut List()
    }

    fn park(t: Token) -> None
    => !t {
        add(waiting, t)
    }

    fn release() -> None {
        let next = remove_first(waiting)
        when next {
            is Token { spend(next) }
            is None {}
        }
    }

    fn shutdown() -> None {
        drain(waiting, spend)
        waiting = Mut List()
    }
}

// [linear-state] The rule that makes it sound: a member that moves state out
// and returns without putting anything back would leave the process with a
// hole a later activation would read.
effect Closing {
    fn shutdown() -> None
}

handler Holey() of Closing {
    state {
        waiting: Mut List<Token> = Mut List()
    }

    //~ error[linear-state:hole]
    fn shutdown() -> None {
        drain(waiting, spend)
    }
}

// [linear-state] A **bare** obligation in state is refused instead: it could
// be stored and never taken out again (taking it is the hole above, and
// nothing could be put back), so the obligation would have no reachable
// discharge at all. The container is the remedy, and the diagnostic names
// it.
effect Holding {
    fn hold(t: Token) -> None => !t
}

handler Held() of Holding {
    state {
        //~ error[linear-state:field]
        held: Token = mint(0)
    }

    fn hold(t: Token) -> None
    => !t {
        spend(t)
    }
}

// [linear-state] The whole-state rule is about *obligations*, not about
// storage in general: a `Copy` scalar field is handed over by value on every
// read, which leaves no hole — the exemption [effect-state-store] already
// grants, and the reason this check reads the field's type.
effect Counting {
    fn bump(n: Int) -> None
    fn total() -> Int
}

handler Tally() of Counting {
    state {
        sum: Int = 0
    }

    fn bump(n: Int) -> None {
        sum = sum + n
    }

    fn total() -> Int {
        return sum
    }
}

// [col-replace] [linear-container] A qualifier's type parameter may `canbe
// linear` (2026-10-05, ROADMAP §0j step 6d), which is what lets std's `Idx`
// claim an index of a list of obligations and the total `replace` take one:
// the displaced element comes back, so nothing is dropped.
fn a_proven_index_replaces_in_a_list_of_obligations() [] -> None {
    let queue: Mut List<Token> = Mut List()
    add(queue, mint(1))
    let i = 0
    if i is Idx(queue) {
        let old = replace(queue, i, mint(2))
        spend(old)
    }
    drain(queue, spend)
}
