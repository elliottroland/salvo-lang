// [linear-generics] Conditional containers and the instantiation ban.

linear struct Lines {
    name: Str
}

fn close(l: Lines) -> None => !l { discard(l) }

fn open_lines(n: Str) -> Lines => !n {
    return Lines { name: n }
}

// The conditional-container matrix (user decision 2026-09-12):
// `struct Box<T canbe linear>` with `T` reaching a field is linear exactly
// when the instantiation is.
struct Box<T canbe linear> {
    item: T
}

// [linear-group] Decomposition settles the container: `unbox` returns the
// linear field and owes nothing further.
fn unbox<T canbe linear>(box: Box<T>) -> T => !box {
    return box.item
}

fn a_conditional_container_owes_when_instantiated_linear(n: Str) -> None
=> !n {
    //~ error[ir-linear-check:leak-at-exit]
    let _b = Box<Lines> { item: open_lines(n) }
}

fn a_conditional_container_forwards_through_its_discharger(n: Str) -> None
=> !n {
    let b = Box<Lines> { item: open_lines(n) }
    let l = unbox(b)
    close(l)
}

fn a_plain_instantiation_owes_nothing() -> None {
    let _b = Box<Int> { item: 42 }
}

// The negative: a conditional container with no same-file discharger errors
// at the struct.
//~ error[linear-obligation:no-discharger]
struct Bare<T canbe linear> {
    item: T
}

// [linear-composite] A concrete linear field requires the marker, and with it
// the container is an ordinary linear struct: stored, forwarded, discharged
// as a unit.
linear struct Session {
    file: Lines
}

fn end_session(s: Session) -> None => !s {
    close(s.file)
}

fn a_concrete_linear_field_needs_the_marker(n: Str) -> None
=> !n {
    let s = Session { file: open_lines(n) }
    end_session(s)
}

// [linear-generics] The instantiation ban reaches effect members' own
// generics (the hole closed 2026-09-12): binding a member's `T` to a linear
// type would hand the obligation to handlers that never promised to honor
// it.
effect Sink {
    fn swallow<T>(x: T) -> None => !x
}

fn an_effect_member_generic_cannot_smuggle_a_linear_value(l: Lines) [Sink] -> None => !l {
    //~ error[linear-generics:effect-member-not-opted]
    swallow(l)
}
