// ===== [deduce-reapply] a claim the function re-establishes =====
//
// The whole point (user decision 2026-09-22, ROADMAP's D2): a mutator keeps a
// user qualifier across a `Mut` parameter by saying it **establishes** it.
// Plain `Q` stays what it always was — a claim the body must have preserved
// — and that is the distinction the spelling exists to draw.

// The shared shape: a qualifier whose file owns a mutator over it. `heap_add`
// mutates through `add`, whose own exhaustive clause strips the claim, and
// nothing but `heap_add` knows it is re-established.
qualifier H<T> of List<T>

fn heap_add<T>(heap: H<T> Mut List<T>, elem: T) [] -> None
=> heap: +H<T> Mut, !elem {
    add(heap, elem)
}

// …and the plain spelling of the same signature still fails, which is what
// makes `+` mean something.
fn heap_add_plain<T>(heap: H<T> Mut List<T>, elem: T) [] -> None
//~ error[deduce-gained:qualifier-removed]
=> heap: H Mut, !elem {
    add(heap, elem)
}

// A compiler qualifier is not a claim about the value and nothing could
// establish it, so `+Mut` is refused outright rather than ignored.
//~ error[deduce-reapply:compiler-qualifier]
fn plus_mut(xs: Mut List<Int>) [] -> None => xs: +Mut {
}

// Re-establishing is not *adding*: the qualifier has to be one the parameter
// declares, so the claim the caller ends up with is the parameter's own.
// [deduce-reapply] A claim the parameter does **not** hold can be established
// too (user decision 2026-09-22): `make_heap(list)` takes an ordinary list
// and hands back a heap, which is the `+Q` form's other half — the same
// trust, and the same same-file confinement. The plain spelling could never
// say this: it may only preserve what the parameter declares.
// [deduce-reapply] …and it reaches the caller **whatever the order of the
// declarations**: what a call needs to know about a callee comes out of a
// side table, and a table filled as the check walks makes the answer depend
// on where the callee sits. That had already cost the implicit parameters
// once (2026-09-22); this pins the same property for the claims a fn
// establishes, with the callee written *below* its caller.
fn only_heap<T>(xs: H<T> Mut List<T>) [] -> Int => xs {
    return 1
}

fn caller(xs: Mut List<Int>) [] -> Int => xs: Mut {
    make_heap(xs)
    return only_heap(xs)
}

fn make_heap<T>(xs: Mut List<T>) [] -> None
=> xs: +H<T> Mut {
}

// …and the claim has to make sense for the parameter's type, on the terms a
// constructor's `as Q` answers to [qual-ctor-fn].
fn not_a_list(n: Int) [] -> None
//~ error[deduce-reapply:does-not-apply]
=> n: +H {
}

// [cmp-carry] A qualifier that **holds a function** cannot be established
// without naming it: every operation that reads the claim binds the slot
// [cmp-binder], so a claim with no identity is one nothing can use. The
// remedy is in the message.
qualifier HC<T>(?cmp: (T, T) -> Int) of List<T>

fn make_heap_c<T>(xs: Mut List<T>, ?cmp: (T, T) -> Int) [] -> None
//~ error[deduce-reapply:no-identity]
=> xs: +HC Mut {
}

//--- only the qualifier's own file
// The trust is gated to the qualifier's **own file** [qual-ctor-same-file]:
// the same party already trusted to mint the claim, and nobody else.
import q.H

fn heap_add<T>(heap: H<T> Mut List<T>, elem: T) [] -> None
//~ error[deduce-reapply:foreign-qualifier]
=> heap: +H<T> Mut, !elem {
    add(heap, elem)
}

// FILE: q.sv
export qualifier H<T> of List<T>
