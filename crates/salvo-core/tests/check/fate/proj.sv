// Projections: borrowed values, derived returns, and what a combinator's
// result is a view of.

//--- l7c_derived_returns
// [proj-readonly-return] L7c: `-> proj(p) T?` marks a derived
// return — the callee may return projections/elements of the kept
// parameter `p` without `copy`, and the caller's result fate-links to
// the argument. Violations: returning an independent value, annotating
// a moved or unknown parameter; and the caller-side discipline holds —
// mutating the argument poisons the result, and the borrowed result
// can never be moved (move-mode refuses borrowed links).
struct Person {
    name: Str,
    age: Int
}

fn take(p: Person) -> None => !p {
}

fn find_adult(persons: List<Person>) -> proj(persons) Person? => persons {
    for person in persons {
        if person.age >= 18 {
            return person
        }
    }
    return None
}

fn forwarded(persons: List<Person>, tag: Str) -> proj(persons) Person? => persons, tag {
    return first(persons)
}

fn bad_independent(persons: List<Person>) -> proj(persons) Person? => persons {
    //~ error[proj-readonly-return:not-derived]
    return Person {name: "made up", age: 1}
}

//~ error[proj-infer:moved-source]
fn bad_moved(persons: List<Person>) -> proj(persons) Person? => !persons {
    return None
}

//~ error[proj-infer:not-a-param]
fn bad_param(persons: List<Person>) -> proj(nobody) Person? => persons {
    return None
}

// The borrowed result can never be moved; `copy` is the remedy
// (ok_copy_escape is clean). [proj-type] names the projection's type.
fn escape_borrowed() {
    let people = List(Person {name: "Ada", age: 36})
    let head = first(people)
    if head is Person h {
        //~ error[proj-type:projection-consumed]
        take(h)
    }
}

fn ok_use() -> Int {
    let people = List(Person {name: "Ada", age: 36})
    let head = first(people)
    if head is Person h {
        return h.age
    }
    return 0
}

fn ok_copy_escape() {
    let people = List(Person {name: "Ada", age: 36})
    let head = first(people)
    if head is Person h {
        take(copy(h))
    }
}

//--- l7c_mutating_the_argument_poisons_the_derived_result
// Mutating the argument poisons the derived result: the IR flow pass's
// diagnostic since R8 piece 3c.
struct Person {
    name: Str,
    age: Int
}

fn poison_after_mutation() -> Int {
    let people = Mut List(Person {name: "Ada", age: 36})
    let head = first(people)
    add(people, Person {name: "Grace", age: 45})
    //~ error[fate-poison:shared-root]
    return head.age
}

//--- a_kept_parameter_takes_a_borrowed_union_arm
// [proj-opt-slot] A union arm that borrows (`Emitted (proj Str) | Finished`)
// passes whole into a kept owned union parameter: the parameter reads it, arm
// by arm (user decision 2026-10-10; refused from 2026-09-12 until then). A
// parameter that consumes its argument still refuses it.
fn show(step: Emitted Str | Finished) [Console] -> None {
    when step {
        is Emitted { println(step) }
        is Finished { println("done") }
    }
}

fn keep(step: Emitted Str | Finished) [] -> None => !step {
}

fn main() [use] {
    use StdConsole()
    let words = List("ann", "bo")
    let p = iter(words)
    show(next(p))
    //~ error[proj-type:nested-borrow]
    keep(next(p))
}

//--- a_combinator_result_is_a_view_of_its_container
// [proj-type] What a pass combinator's result *is*: `map`/`keep_all` over a
// container's pass yield a `Mut List<proj Str>` — a view, fate-linked to
// the container — so consuming either the view or the container is
// refused; an instantiation over a *generator* pass has no container and
// stays free. And `copy` un-projects exactly one level: `copy(e)` on a
// `proj Mut Str` is a `Mut Str` (observed via the mismatch against `Int`).
// The phase-2b regression set, closed 2026-09-12.
fn eat(xs: List<Str>) -> None => !xs {}

fn keep(w: proj Str) -> proj(w) Str => w {
    return w
}

struct Chars {
    n: Int
}

iter fn iter(c: Chars) -> Emitted Str | Finished {
    state CharsIter {
        at: Int = 0
    }
    if at >= c.n {
        return finished()
    }
    at = at + 1
    return emitted("x")
}

fn keep_all<It, T>(it: Mut It, ?Next<It, T>) -> Mut List<T> => it: Mut {
    let out = Mut List<T>()
    for x in it {
        add(out, x)
    }
    return out
}

fn main() [use] {
    use StdConsole()
    let words = List("ann", "bo")
    let p = iter(words)
    let kept = map(p, keep)
    // The combinator's type, named whole by the refusal to consume it.
    //~ error[proj-type:nested-borrow]
    eat(kept)
    // The generator instantiation is unlinked (`free` draws no error).
    let free = keep_all(iter(Chars {n: 2}))
    println("${size(free)}")
    let parts = Mut List(Mut Str("a"))
    let e = get(parts, 0) ?: unreachable!()
    // `copy(e)` on a `proj Mut Str` is a `Mut Str`: one level un-projected,
    // the `Mut` kept.
    //~ error[var-let-infer:annotation-mismatch]
    let _n: Int = copy(e)
}

//--- a_combinator_view_is_linked_through_the_pass_to_its_container
// The view is fate-linked *through* the temporary pass to the container:
// consuming the container poisons it. The IR flow pass's diagnostic
// since R8 piece 3c.
fn eat(xs: List<Str>) -> None => !xs {}

fn keep_all<It, T>(it: Mut It, ?Next<It, T>) -> Mut List<T> => it: Mut {
    let out = Mut List<T>()
    for x in it {
        add(out, x)
    }
    return out
}

fn main() [use] {
    use StdConsole()
    let words = List("ann", "bo")
    let more = keep_all(iter(words))
    eat(words)
    //~ error[fate-poison:shared-root]
    println("${size(more)}")
}

//--- a_written_proj_entry_narrows_the_instantiation_link
// [proj-infer] A written opaque projection annotation (`-> … holds proj(it)`)
// names the lends exactly and takes precedence over the instantiation
// fallback that links a `proj`-holding result to every kept argument —
// even where the *written* return type shows no projection (`Mut List<T>`
// with `T = proj Str`). The link still flows through the named source.
// (Closed 2026-09-12; before, the entry was consulted only behind a
// written-return-type gate the instantiation case never passed.)
fn eat(xs: List<Str>) -> None => !xs {}

fn keep_all<It, T>(it: Mut It, labels: List<Str>, ?Next<It, T>) -> Mut List<T> holds proj(it) => it: Mut, labels {
    let out = Mut List<T>()
    for x in it {
        add(out, x)
    }
    return out
}

fn main() [use] {
    use StdConsole()
    // Consuming the co-argument the entry does *not* name is free…
    let words = List("ann", "bo")
    let tags = List("x")
    let kept = keep_all(iter(words), tags)
    eat(tags)
    println("${size(kept)}")
    // …while the named source still links through the temporary pass.
    let words2 = List("ann", "bo")
    let tags2 = List("x")
    let kept2 = keep_all(iter(words2), tags2)
    eat(words2)
    //~ error[fate-poison:shared-root]
    println("${size(kept2)}")
}
