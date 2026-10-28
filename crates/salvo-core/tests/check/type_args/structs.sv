// [struct-literal-arg] [generic-arity] A generic struct's type arguments:
// what a bare literal infers from its fields or from the parameter it is
// passed to, and what a type position must write.

struct Box<T> { value: T }

// [struct-literal-arg] A **bare generic struct literal** determines the
// struct's own type arguments from its **field values**, which is the only
// evidence a literal written as a call argument has: there is no annotation,
// and the enclosing call's substitution is not solved yet. Before this,
// `unwrap(Box { value: 7 })` was "no matching overload for `unwrap(Box)`" —
// the literal's type had no arguments at all, so it could not match `Box<T>`
// (defect closed 2026-09-26; the collection-literal half was
// [col-literal-arg]).
fn unwrap<T>(b: Box<T>) -> T => !b {
    return b.value
}

fn unwrapped() [] -> Int {
    return unwrap(Box { value: 7 })
}

// [struct-literal-arg] Two parameters, and a field each: both are determined,
// and the *declared* field order does not have to match the generic order.
struct Pair<A, B> { second: B, first: A }

fn left<A, B>(p: Pair<A, B>) -> A => !p {
    return p.first
}

fn lefted() [] -> Str {
    return left(Pair { second: 1, first: "x" })
}

// [struct-literal-arg] The fields are evidence of **last resort**: a written
// type argument still decides, so a literal whose field disagrees with it is
// reported rather than silently re-typed.
fn written_decides() [] -> None {
    //~ error[struct-decl:field-mismatch]
    let _b: Box<Str> = Box<Str> { value: 7 }
}

// [generic-arity] A generic struct referenced in a bare type position
// (a field declaration here) must supply every type argument it declares.
// Before this check, an omitted argument silently bound to `Ty::Unknown`
// and rode [type-unknown-lenient] for every later read through the field —
// no checking at all (defect closed 2026-10-19 via `List<Arg>` for a
// generic `Arg<T>` in `lib/std/args/parse.sv`). The diagnostic names the
// struct and the missing count, once.
//~ error[qual-value-arg:type-arg-count]
struct Holder { b: Box }

// [generic-arity] The check only fires when a count is actually declared
// and wrong; a fully-argued one is unaffected.
struct FullHolder { b: Box<Int> }

// [generic-arity] A struct *literal*'s written type is exempt from this
// check — unlike a field declaration, a literal may infer `T` from its
// field values [struct-literal-arg], so `Box { ... }` (no `<...>` at all)
// must not be flagged by the arity check that a bare type position gets.
fn bare_literal() [] -> None {
    let _b = Box { value: 7 }
}

// [struct-literal-arg] A struct literal passed as a *call argument*, with
// its generic type argument omitted, infers it from the callee's
// substituted parameter type when the call is otherwise unambiguous — no
// need to write `Arg<Int>` when `add_arg(list: Mut List<Arg<Int>>, elem:
// Arg<Int>)` already pins `T = Int` (fixed 2026-10-19 alongside the
// [generic-arity] check, via `lib/std/args/parse.sv`'s
// `args.ints.add(Arg { ... })`).
struct Arg<T> { short_name: Str? = None, default: T? = None }

fn add_arg<T>(list: Mut List<Arg<T>>, elem: Arg<T>) [] -> None => list: Mut, !elem {}

fn from_the_parameter() [] -> None {
    let xs: Mut List<Arg<Int>> = Mut List()
    add_arg(xs, Arg { short_name: "-n" })
}

// [struct-literal-arg] A written type argument on the literal still wins
// over the inferred one when it disagrees — the call-argument inference is
// only a fallback for when nothing else decided it.
fn own_written_decides() [] -> None {
    let xs: Mut List<Arg<Int>> = Mut List()
    //~ error[call-resolve:no-overload]
    add_arg(xs, Arg<Str> { default: "x" })
}
