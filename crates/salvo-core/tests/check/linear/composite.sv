// [linear-composite] — R4 part 2. The interim rule (user decision
// 2026-09-08): a linear value may not be **stored in a composite** unless
// the composite carries the obligation too. Every way of putting one into a
// composite that cannot is refused *at the store*, which is also what keeps
// the diagnostics single: no follow-on leak for a container that was never
// built. [linear-container] Lists and maps of obligations can (LC-1…LC-5).

linear struct Lines {
    name: Str
}

fn close(l: Lines) -> None => !l { discard(l) }

fn open_lines(n: Str) -> Lines => !n {
    return Lines { name: n }
}

// A struct field is the first thing anyone tries; the error lands at the
// field's type and the remedy is the marker: a concrete linear field is
// legal exactly on a `linear struct` (user decision 2026-09-12).
struct Holder {
    //~ error[linear-composite:field]
    handle: Lines
}

// [linear-container] LC-3 (user decision 2026-09-15): a `List<Lines>` is
// itself linear, so a struct field of that type makes the *struct* a
// resource — and contagion stays **spelled**: the marker is the remedy,
// exactly as it is for a concrete linear field.
struct ListHolder {
    //~ error[linear-composite:field]
    handles: Mut List<Lines>
}

// [linear-state] LC-4 (user decision 2026-09-15): a handler's state is a
// composite too. A process owns its obligations, and the shapes the
// concurrency surface is for live in handler state — but a **bare**
// obligation in a field could never be taken back out (that would leave a
// hole, and nothing could be put back), so the container is the form and
// the diagnostic says so.
effect Note {
    fn note(m: Str) -> None => !m
}

handler Keeper of Note {
    state {
        //~ error[linear-state:field]
        held: Lines = open_lines("a")
    }

    fn note(m: Str) -> None {}
}

// A *container* field is legal, and an activation that drains it puts a
// fresh one back — which is the whole of LC-4's discipline.
handler ListKeeper of Note {
    state {
        held: Mut List<Lines> = Mut List()
    }

    fn note(m: Str) -> None {
        drain(held, close)
        held = Mut List()
    }
}

// Written type positions: an array element and a tuple component.
//~ error[linear-container:position]
fn take_array(x: Lines[]) -> None => !x {}

//~ error[linear-container:position]
fn take_tuple(x: (Lines, Int)) -> None => !x {}

// [linear-union-arm] Union arms are *not* composites (O-C2, user decision
// 2026-09-12): a union value is one handle, so a written linear arm is legal
// and the obligation is the union's — the fallible-open shape. `T?` follows.
// The body owes: consuming `x` satisfies it, so `=> !x` with an empty body
// leaks, and the *arm* legality is the point here.
//~ error[ir-linear-check:leak-at-exit]
fn take_union(x: Lines | Int) -> None => !x {}

//~ error[ir-linear-check:leak-at-exit]
fn take_optional(x: Lines?) -> None => !x {}

// One store, one error: a nested composite reports once rather than once per
// level — a list of lists of obligations is one linear value, owed whole.
//~ error[ir-linear-check:leak-at-exit]
fn take_nested(x: List<List<Lines>>) -> None => !x {}

// The inferred store: a tuple literal, which has no written type to refuse.
// (The array-literal half went with collection literals [col-no-literal]: a
// constructor call is checked against its callee's `canbe linear`.)
fn literal_composites_are_refused() -> None {
    //~ error[linear-container:position]
    let _t = (open_lines("a"), 1)
}

// A generic field type hides the store from the struct's declaration, so the
// literal is where it surfaces.
struct Box<T> {
    item: T
}

fn a_generic_struct_cannot_be_instantiated_with_a_linear_type() -> None {
    //~ error[linear-container:position]
    let _b = Box { item: open_lines("a") }
}

// [linear-container] LC-1/LC-2 (user decisions 2026-09-15/16): the generic
// *call* store `add(list, elem)` is not an error — the *container* carries
// the obligation, and the leak diagnostic names its terminal.
fn storing_into_a_container_makes_the_container_owe() -> None {
    //~ error[ir-linear-check:leak-at-exit]
    let xs: Mut List<Lines> = Mut List()
    add(xs, open_lines("a"))
}

// …and draining it discharges every element through the callback.
fn draining_the_container_discharges_it() -> None {
    let xs: Mut List<Lines> = Mut List()
    add(xs, open_lines("a"))
    drain(xs, close)
}

// But a signature that only *reads* a composite of `T` stores nothing:
// `size(list: List<T>) -> Int` stays callable from opted-in generic code,
// which is the shape the refinement demo relies on.
fn count_all<T canbe linear>(list: List<T>) -> Int => list {
    return size(list)
}

// And the composite refusal is about *linear* content only: an ordinary
// container of ordinary values is untouched.
struct Names {
    names: Mut List<Str>
}

fn a_composite_of_plain_values_is_unaffected() -> None {
    let l = open_lines("a")
    let _h = Names { names: Mut List("x") }
    close(l)
}
