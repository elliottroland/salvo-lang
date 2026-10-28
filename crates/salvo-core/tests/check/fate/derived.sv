// ===== Shared fate [fate-link] [fate-poison] [fate-derived-readonly] =====
// A binding from another variable shares its fate: what the root does
// reaches the derived variable, and what the derived variable may do is
// bounded by its root.

//--- fate_derived_variables_are_read_only
// [fate-derived-readonly] A fate-linked (derived) variable is read-only:
// moving it (consuming call, `return`) or mutating it (`Mut` argument)
// errors at the site with `copy` as the remedy. Reads stay legal.
//
// [fate-move-mode] S2: moving or mutating a derived variable whose
// roots are owned is legal — the binding takes ownership and the
// *root* is consumed at the binding; its later use names the binding
// (the IR flow pass's diagnostic since R8 piece 3c).
fn consume(v: List<Int>) -> None => !v {
}

fn move_derived() -> Int {
    let xs = List(1, 2)
    let ys = xs
    consume(ys)
    //~ error[fate-move-mode:bound-away]
    return size(xs)
}

fn mutate_derived() -> Int {
    let xs = Mut List(1, 2)
    let ys = xs
    add(ys, 3)
    //~ error[fate-move-mode:bound-away]
    return size(xs)
}

//--- fate_derived_from_a_kept_parameter_is_read_only
// A *written-kept* parameter root keeps the S1 error at the move
// site [fate-derived-readonly]: you cannot move out of a borrow.
// Exactly the one violation: reading a derived variable is fine.
fn return_derived(v: Str) -> Str => v {
    let w = v
    //~ error[fate-derived-readonly:shares-fate]
    return w
}

fn read_derived(v: Str) -> Int => v {
    let w = v
    return size(w)
}

//--- fate_root_events_poison_derived_variables
// [fate-poison] Mutating, moving, or reassigning a root poisons every
// variable derived from it: the later *use* errors, naming the link and
// the event; a poison never observed never fires.
// The IR flow pass's diagnostic since R8 piece 3c, naming the event's
// line.
fn consume(v: List<Int>) -> None => !v {
}

fn mutated() -> Int {
    let xs = Mut List(1)
    let ys = xs
    add(xs, 2)
    //~ error[fate-poison:shared-root]
    return size(ys)
}

fn moved() -> Int {
    let xs = List(1)
    let ys = xs
    consume(xs)
    //~ error[fate-poison:shared-root]
    return size(ys)
}

fn reassigned() -> Int {
    let xs = List(1)
    let ys = xs
    xs = List(2, 3)
    //~ error[fate-poison:shared-root]
    return size(ys)
}

fn unused_poison_is_fine() {
    let xs = Mut List(1)
    let _ys = xs
    add(xs, 2)
}

//--- fate_links_flow_through_projections_loops_and_bindings
// [fate-link] Links flow through projections (transitively to the root),
// loop bindings, and `is` bindings; reassignment from a fresh value
// severs them (revival).
struct Person {
    name: Str
}

// The loop assignment links `longest` through `person` to `persons`.
fn longest_name(persons: Person[]) -> Str => persons {
    let longest = ""
    for person in persons {
        if size(longest) < size(person.name) {
            longest = person.name
        }
    }
    //~ error[fate-derived-readonly:shares-fate]
    return longest
}

fn is_binding(v: Str | Int) -> Str => v {
    if v is Str s {
        //~ error[fate-derived-readonly:shares-fate]
        return s
    }
    return "other"
}

// `revived` is clean: reassignment severed the link.
fn revived(p: Person) -> Str => p {
    let n = p.name
    n = "fresh"
    return n
}

//--- fate_copy_produces_independent_values
// [copy-fn] `copy` keeps its argument (with all qualifiers) and returns
// an independent value: every fate error above has a one-word remedy,
// and copies are unaffected by later root events.
struct Person {
    name: Str
}

fn longest_name(persons: Person[]) -> Str => persons {
    let longest = ""
    for person in persons {
        if size(longest) < size(person.name) {
            longest = person.name
        }
    }
    return copy(longest)
}

fn independent() -> Int {
    let xs = Mut List(1)
    let ys = copy(xs)
    add(xs, 2)
    add(ys, 3)
    return size(ys) + size(xs)
}

fn main() [use] {
    use StdConsole
    println(longest_name(array_of(Person {name: "a"})))
    println("${independent()}")
}

//--- fate_links_merge_across_branches
// [fate-link] Links union across branch merges: a variable linked on any
// fall-through path is linked after the join.
fn pick(a: List<Int>, cond: Bool) -> List<Int> => a {
    let out: List<Int> = List(0)
    if cond {
        out = a
    }
    //~ error[fate-derived-readonly:shares-fate]
    return out
}

//--- fate_mut_projection_arguments_keep_a_handle_but_refuse_derived_mutation
// [deduce-field] Rung ⑤ v2 (2026-09-25): a contents mutation through a
// projection leaves a *handle to the same field* standing — `t` names the
// very list `add` grew, and both backends see it (Rust re-materializes the
// place, Kotlin holds the object), which is what the historical `diverged`
// case needed. What still refuses: mutating through a *derived* variable,
// which is an error like any mutation of one.
struct Holder {
    tags: Mut List<Int>
}

// `kept` is clean.
fn kept() -> Int {
    let h = Holder {tags: Mut List(1)}
    let t = h.tags
    add(h.tags, 2)
    return size(t)
}

fn mutate_through_derived(other: Holder) -> None => other {
    let h = other
    //~ error[fate-derived-readonly:shares-fate]
    add(h.tags, 2)
}

//--- s2_move_mode_pipelines_stay_clean
// [fate-move-mode] S2 positive matrix: move-mode bindings make consuming
// pipelines legal with zero copies — moving/mutating a derived variable
// whose ancestors are owned (locals, or parameters the fn's inferred
// contract moves) consumes the ancestors at the binding. Loop bindings
// are fresh per iteration (consuming one is legal), projections of
// *immutable* data in moved positions stay untracked (clone-vs-alias is
// unobservable), and `copy` still severs where the source must stay
// usable.
struct Person {
    name: Str,
    age: Int
}

struct Label {
    text: Str
}

fn consume(text: Str) -> None => !text {
}

fn longest_name(persons: List<Person>) -> Str
=> !persons {
    let longest = ""
    for person in persons {
        let name = person.name
        if size(name) > size(longest) {
            longest = name
        }
    }
    return longest
}

fn per_iteration() {
    for s in List("a", "b") {
        consume(s)
    }
}

fn immutable_projection_store(person: Person) -> Label => person {
    return Label {text: person.name}
}

fn copy_keeps_source() -> Int {
    let xs = List(1, 2)
    let ys = copy(xs)
    consume_list(ys)
    return size(xs)
}

fn consume_list(v: List<Int>) -> None => !v {
}

fn main() [use] {
    use StdConsole()
    let people = List(Person {name: "Ada", age: 36}, Person {name: "Grace", age: 45})
    println(longest_name(people))
}

//--- s2_move_mode_ancestors_are_consumed
// [fate-move-mode] [fate-partial-move] S2 negative matrix: the poison lands
// at the binding — using an ancestor after a move-mode binding errors naming
// the binding; a written-kept parameter ancestor keeps the S1 move-site
// error; and a projection of *mutable* data in a moved position leaves the
// root usable but records the moved place, so reading *that* projection back
// is the error (closing the 2026-09-02 parity divergence — L5 made the
// diagnostic name the field rather than the whole variable). The parity probe
// is its own program since R8 piece 3b, when its error became the IR flow
// pass's and a checker error anywhere hid those (per declaration since R8
// piece 9).
//
// A kept parameter's mutable data cannot be moved out; `copy` is the
// remedy (kept_remedy is clean).
struct Holder {
    tags: Mut List<Int>
}

struct Wrapper {
    item: Mut List<Int>
}

fn wrap(list: Mut List<Int>) -> Wrapper => !list {
    return Wrapper {item: list}
}

fn kept_leak(h: Holder) -> Wrapper => h {
    //~ error[fate-move-mode:kept-mutable]
    return wrap(h.tags)
}

fn kept_remedy(h: Holder) -> Wrapper => h {
    return wrap(copy(h.tags))
}

//--- s2_written_kept_binding_keeps_the_move_site_error
// A written-kept parameter binding keeps the S1 error at the move
// site [fate-derived-readonly] — the IR flow pass's since R8 piece 8.
fn kept_binding(v: Str) -> Str => v {
    let w = v
    //~ error[fate-derived-readonly:shares-fate]
    return w
}

//--- s2_chain_consumes_the_root_at_the_first_binding
// Chain: `xs` was consumed at `a`'s binding (the whole chain is
// move-mode); the use-site error names the binding. The IR flow pass's
// diagnostic since R8 piece 3c.
fn consume_list(v: List<Int>) -> None => !v {
}

fn chain() -> Int {
    let xs = List(1, 2)
    let a = xs
    let b = a
    consume_list(b)
    //~ error[fate-move-mode:bound-away]
    return size(xs)
}

//--- s2_parity_probe_is_rejected
// The parity probe is rejected: `wrap(h.tags)` moved mutable data out
// of `h`, so the later `add(h.tags, 9)` cannot observe an alias on one
// backend and a clone on the other. Since L5 the diagnostic names the
// *field* that left rather than the whole variable — `h` itself stays
// usable, and reading `h.tags` back is what fails [fate-partial-move] —
// and since R8 piece 3b the line it left at.
struct Holder {
    tags: Mut List<Int>
}

struct Wrapper {
    item: Mut List<Int>
}

fn wrap(list: Mut List<Int>) -> Wrapper => !list {
    return Wrapper {item: list}
}

fn parity_probe() -> Int {
    let h = Holder {tags: Mut List(1)}
    let w = wrap(h.tags)
    //~ error[fate-partial-move:overlapping-use]
    add(h.tags, 9)
    return size(w.item)
}
