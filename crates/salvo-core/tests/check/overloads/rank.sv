// Overload resolution, finalized (user decisions 2026-09-06/07). One rule,
// three steps, and two ways for the caller to override it:
//
// * [fn-overload-scope] functions arrive from ever more specific scopes —
//   `std.core`, then this file's imports, then this module — and every scope
//   that *fits the arguments* competes on signature (`scope.sv`);
// * [fn-overload-rank] within them, the most specific signature wins: a
//   type variable says least, a broader union says less than a narrower one
//   than an arm, more qualifiers say more (the *kind* of qualifier never
//   ranking), and a fixed parameter list beats a variadic one. Per argument
//   slot, so one argument's gain never pays for another's loss;
// * [fn-overload-ambiguous] no single most specific candidate is an error
//   naming the remedies — never a pick;
// * [fn-overload-at] `add@core.list(x)` names the module whose overload is
//   meant, and [fn-rename] `rename fn add2 = add(...)` gives one overload a
//   name of its own, taking it *out* of the shared one (`rename.sv`).
//
// This file: [fn-overload-rank], the specificity ladder. Each old test's
// overload set has a name of its own here, so they share one program.

// A concrete parameter type beats a type variable, in either declaration
// order — the case the ranking was first written for. (`portray` is the
// flipped declaration order of `describe`.)
fn describe<T>(value: T) [] -> Bool => !value { return true }
fn describe(value: Int) [] -> Str { return "concrete" }

fn portray(value: Int) [] -> Str { return "concrete" }
fn portray<T>(value: T) [] -> Bool => !value { return true }

fn concrete_beats_a_variable() -> None {
    let _picked: Str = describe(3)
    let _flipped: Str = portray(3)
}

// A **narrower union** says more, and an arm says more than the union it
// belongs to. The argument's own type decides what is even viable, so this
// only ranks candidates the caller could have meant.
fn take(v: Int | Str | Bool) [] -> Bool => !v { return true }
fn take(v: Int | Str) [] -> Char => !v { return 'p' }
fn take(v: Int) [] -> Str { return "int" }

fn narrower_union_wins() -> None {
    // A plain `Int` reaches the arm.
    let _picked: Str = take(3)
}

// A value the caller only knows as `Int | Str` cannot: specificity never
// exceeds what the caller knows.
fn known_as_the_union() -> None {
    let v: Int | Str = 3
    let _picked: Char = take(v)
}

// ... and once narrowed, it does.
fn narrowed_to_the_arm() -> None {
    let v: Int | Str = 3
    if v is Int {
        let _picked: Str = take(v)
    }
}

// `T` beats `T?` for a non-`None` argument — the optional is just a union
// with a `None` arm, so this needs no rule of its own.
fn opt_take(v: Int?) [] -> Bool => !v { return true }
fn opt_take(v: Int) [] -> Str { return "int" }

fn plain_beats_optional() -> None {
    let _picked: Str = opt_take(3)
}

// `Any` is the broadest type there is, so it is the last resort.
fn any_take(v: Any) [] -> Bool => !v { return true }
fn any_take(v: Int) [] -> Str { return "int" }

fn any_is_least_specific() -> None {
    let _picked: Str = any_take(3)
}

// And it really does accept everything (it is the top type, not a
// nominal type that happens to be called `Any`).
fn any_only(v: Any) [] -> Bool => !v { return true }

fn any_accepts_everything() -> None {
    let _picked: Bool = any_only("text")
}

// **More qualifiers say more**, and the *kind* of qualifier never ranks:
// two candidates with one qualifier each are unrankable, which is an
// ambiguity the caller settles with a rename (`rename.sv`).
qualifier Even of Int {
    fn qualifies(n: Int) -> Bool { return true }
}

qualifier Small of Int {
    fn qualifies(n: Int) -> Bool { return true }
}

// A superset wins.
fn label(n: Even Int) [] -> Bool => n { return true }
fn label(n: Even Small Int) [] -> Str => n { return "both" }

fn superset_wins() -> None {
    let n = 4
    if n is Even && n is Small {
        let _picked: Str = label(n)
    }
}

// Different single qualifiers do not ("ambiguous call to
// `badge(Even Small Int)`", naming `rename fn`), once.
fn badge(n: Even Int) [] -> Bool => n { return true }
fn badge(n: Small Int) [] -> Str => n { return "small" }

fn single_qualifiers_tie() -> None {
    let n = 4
    if n is Even && n is Small {
        //~ error[fn-overload-ambiguous:call]
        let _picked = badge(n)
    }
}

// [fn-overload-rank] With the slots otherwise equal, a **fixed** parameter
// list beats a variadic one — which is what lets an "empty" case be an
// overload rather than a special form.
fn make() [] -> Str { return "empty" }
fn make(...rest: Int[]) [] -> Bool { return true }

fn fixed_beats_variadic() -> None {
    let _picked: Str = make()
    // With arguments, only the variadic one fits.
    let _variadic: Bool = make(1, 2)
}

// Ranking is **per argument slot**: two candidates that each win one slot
// are an ambiguity, not a sum to be totted up — once, naming
// `mix(T, Int) -> Bool`, `mix(Int, T) -> Bool` and `rename fn`.
fn mix<T>(a: T, b: Int) [] -> Bool => !a { return true }
fn mix<T>(a: Int, b: T) [] -> Bool => !b { return true }

fn one_slot_never_pays_for_another() -> None {
    //~ error[fn-overload-ambiguous:call]
    let _x = mix(1, 2)
}

// [type-unknown-lenient] An un-inferred argument fits every candidate, so
// it must not produce an ambiguity of its own: only the unresolved call.
fn un_inferred_argument() -> None {
    //~ error[call-resolve:unknown-fn]
    let _x = mix(nope(), 2)
}

// [fn-overload-rank] [type-unknown-lenient] A lambda with an unwritten
// parameter type, at a position where the overloads take different fn types
// and none is more specific, has nothing to type it: refused, once, naming
// the remedy and both types (found beside B1 census D16, 2026-10-27 — the
// lambda's parameter stayed `?`, the first overload was taken, and both
// target compilers refused the code). Where the overloads agree on the
// position, it is typed from it; a written parameter type settles the call.
fn twice(f: (x: Int) -> Int) [] -> Int => f { return f(f(1)) }
fn twice(f: (x: Str) -> Str) [] -> Str => f { return f(f("s")) }

fn lambda_the_overloads_disagree_on() -> None {
    //~ error[fn-overload-rank:lambda-untyped]
    let _n = twice(x -> x)
}

fn lambda_with_a_written_type() -> None {
    let _n: Int = twice((x: Int) -> x)
}

fn pick(f: (x: Int) -> Int, n: Int) [] -> Int => f { return f(n) }
fn pick(f: (x: Int) -> Int, s: Str) [] -> Int => f { return f(1) }

fn lambda_the_overloads_agree_on() -> None {
    let _a: Int = pick(x -> x, 2)
    let _b: Int = pick(x -> x, "s")
}
