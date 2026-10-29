// Implicit parameters [implicit-param] [implicit-group]
// [implicit-resolve] [implicit-forward] [implicit-override].
//
// `?cmp: (T, T) -> Int` is a parameter the caller need not pass: the call
// site fills it by resolving the parameter's *name* at the parameter's
// *type*. That is the overload query the language already runs, only
// against a type instead of an argument list — which is why Salvo needs no
// qualified-name syntax for the defaults (Koka's `Str/cmp`): the `cmp`
// whose parameters accept `Str` *is* the default for `Str`.
//
// `?Additive<T>` spreads a `fns` group. It has no binder (user decision
// 2026-09-05): the members become implicit parameters in their own right,
// so resolution, forwarding and override all key on a member's own name
// and type, and a callee can reach the same parameter through a different
// grouping — or none.
//
// Over the real std, whose `cmp(Int, Int)` and `times(Int, Int)` are the
// scalar operators'. (The group was `Field` under the old prelude;
// `std.core.comptime` has a `Field` of its own.)

fns Additive<T> {
    fn add(a: T, b: T) -> T
    fn zero() -> T
}

fn add(a: Int, b: Int) -> Int { return 0 }
fn zero() -> Int { return 0 }

fn total<T>(a: T, ?Additive<T>) -> T => !a {
    return add(a, zero())
}

fn pick<T>(a: T, b: T, ?cmp: (T, T) -> Int) -> Int => !a, !b {
    return cmp(a, b)
}

// ===== [implicit-resolve] the default is an ordinary overload =====

// The `add`/`zero` whose parameters accept `Int` are what a call with
// `T = Int` gets — no declaration ties them to the type.
fn group_resolves() -> Int {
    return total(1)
}

// An individually written implicit needs no group at all.
fn written_resolves() -> Int {
    return pick(1, 2)
}

// Nothing resolves for a type with no matching declaration, and the error
// names both remedies — declare one, or pass one here. A `cmp` *is* declared
// — for `Int`, among others — so the diagnostic says which ones it looked at
// and what the position needed, rather than claiming nothing of that name
// exists. (`Str` under the old prelude, which had no `cmp(Str, Str)`.)
struct Sealed { n: Int }

fn unresolvable(s: Sealed) -> Int => !s {
    //~ error[implicit-resolve:no-fit]
    return pick(s, s)
}

// ===== [implicit-override] `name = value` at the call site =====

// The caller can replace one member of a group by its own name — the point
// of dropping the binder.
fn override_one_member() -> Int {
    return total(1, add = times)
}

// A lambda works as well as a fn name.
fn override_with_a_lambda() -> Int {
    return pick(1, 2, cmp = (x: Int, y: Int) -> x)
}

// A name that matches no implicit parameter is a mistake, not a no-op.
fn override_matching_nothing() -> Int {
    //~ error[implicit-override:no-implicit]
    return pick(1, 2, order = times)
}

// The value has to fit the parameter's type.
fn override_of_the_wrong_type() -> Int {
    //~ error[implicit-override:does-not-fit]
    return pick(1, 2, cmp = (x: Int) -> x)
}

// It fits as it would by name: a fn reading a borrow of its parameter fits a
// position passing that parameter, so `copy = copy@std.core.basic` is what
// resolution would have filled (found by the round trip, PIPELINE B4: the
// override was held to plain subtyping, and refused `(proj Str) -> Str`).
fn duplicate<T>(v: T, ?copy: (v: T) -> T) -> T => v {
    return copy(v)
}

fn override_reading_a_borrow() -> Str {
    return duplicate("a", copy = copy@std.core.basic)
}

// ===== [implicit-forward] generic code passes its own implicits on =====

// Inside a generic fn nothing about `T` is knowable [call-resolve], so the
// only thing that can fill an inner call's implicit is the enclosing fn's
// own — matched by name and type, and passed on without being named.
fn total_twice<T>(a: T, b: T, ?Additive<T>) -> T => !a, !b {
    return add(total(a), total(b))
}

// Forwarding matches on name and type, not on how the parameters were
// declared: a group here fills an individually-declared implicit there.
fn sum_pair<T>(a: T, b: T, ?add: (T, T) -> T) -> T => !a, !b {
    return add(a, b)
}

fn total_by_pair<T>(a: T, ?Additive<T>) -> T => !a {
    return sum_pair(a, zero())
}

// A generic fn that declares *nothing* cannot call one that needs an
// implicit: there is no default for an opaque `T` and nothing to forward.
// This is the colouring the feature costs, and the error says what to add
// ("no `add` fits … for `total`") — once per member of the group, `add` and
// `zero`.
fn careless<T>(a: T) -> T => !a {
    //~ error[implicit-resolve:no-fit]
    //~ error[implicit-resolve:no-fit]
    return total(a)
}

// ===== declaration-site rules =====

// [implicit-param] Only a function can be resolved by name and type.
//~ error[implicit-param:not-fn-type]
fn with_limit(a: Int, ?limit: Int) -> Int {
    return a
}

// [implicit-group] A spread of an unknown group names the mistake.
//~ error[implicit-group:unknown-group]
fn with_ring<T>(a: T, ?Ring<T>) -> T => !a {
    return a
}

// [implicit-same-name] Two implicits of one name at different types are two
// parameters (user decision 2026-09-28): `?Additive<A>, ?Additive<B>` brings
// one `add` per type, and nothing is refused.
fn two_groups<A, B>(a: A, b: B, ?Additive<A>, ?Additive<B>) -> A => !a, !b {
    return a
}

// A group's type arguments have to match its generics.
//~ error[implicit-group:type-arg-count]
fn wrong_arity<A, B>(a: A, ?Additive<A, B>) -> A => !a {
    return a
}

// A `fns` group member is a signature for a parameter; the default comes
// from a matching top-level fn, so a body here means nothing.
fns Bodied<T> {
    //~ error[implicit-group:member-body]
    fn mul(a: T, b: T) -> T { return a }
}

// ===== selection: an implicit is chosen as a call is =====

// [implicit-resolve] [fn-overload-rank] An implicit is chosen as a call is
// (user decision 2026-10-14): of a generic `order<T>` and a concrete
// `order(Int, Int)`, both fitting `(Int, Int) -> Int`, the more specific wins.
// Under the old rule — the most specific *scope*, then "two hits are
// ambiguous" — this was an ambiguity. (`order`, not `cmp`: a `cmp` on a type
// parameter or on `Int` outside `Int`'s file is refused [op-unique-shape].)
fn order<T>(a: T, b: T) -> Int { return 0 }
fn order(a: Int, b: Int) -> Int { return 0 }

fn pick_order<T>(a: T, b: T, ?order: (T, T) -> Int) -> Int => !a, !b {
    return order(a, b)
}

fn more_specific_wins() -> Int {
    return pick_order(1, 2)
}

// [implicit-resolve] [fn-overload-ambiguous] No single most specific
// candidate is an ambiguity naming the candidates and the remedies an
// implicit has — the override naming the place, or `rename fn` — rather than
// a guess. (`tie` was a second `order`.)
fn tie(a: Int | Bool, b: Int) -> Int { return 0 }
fn tie(a: Int, b: Int | Bool) -> Int { return 0 }

fn pick_tie<T>(a: T, b: T, ?tie: (T, T) -> Int) -> Int => !a, !b {
    return tie(a, b)
}

fn ambiguous() -> Int {
    //~ error[fn-overload-ambiguous:implicit]
    return pick_tie(1, 2)
}

// ===== B3 piece 1: implicits selected as a call selects (R10) =====

// [implicit-infer] Inference from an implicit runs selection and reads the
// variable off the winner (R10 D22): of `f(x: Int) -> Str` and `f<T>(x: T)
// -> Str`, both fitting `?f: (x: Int) -> B`, the concrete one is more specific
// and teaches `B = Str`. Before, the matcher wanted exactly one candidate to
// fit, so an un-annotated `conv(1)` could not infer `B`.
fn f(x: Int) [] -> Str => x { return "int" }
fn f<T>(x: T) [] -> Str => x { return "any" }
fn conv<B>(x: Int, ?f: (x: Int) -> B) [] -> B => x { return f(x) }
fn takes(s: Str) [] -> Str => !s { return s }

fn infers_from_the_winner() [] -> Str {
    let s = conv(1)
    return takes(s)
}

// [implicit-infer] Two candidates neither of which is more specific teach
// nothing: the ambiguity is the fill's to report, as one diagnostic.
// (`g` was a second `f`.)
fn g(x: Int | Bool) [] -> Str => x { return "a" }
fn g(x: Int | Str) [] -> Int => x { return 1 }
fn conv_g<B>(x: Int, ?g: (x: Int) -> B) [] -> B => x { return g(x) }

fn infers_nothing_from_a_tie() [] -> Str {
    //~ error[call-type-args:cannot-infer]
    let _s = conv_g(1)
    return ""
}

// [implicit-same-name] Two implicits of one name are ranked like overloads
// (R10 D21): `tag(n)` with `n: Int` fits both `?tag: (x: Int) -> Str` and
// `?tag: (x: Int | Str) -> Str`, and the `Int` one is more specific. Before,
// "could go through more than one implicit of that name".
fn tagged(n: Int, ?tag: (x: Int) -> Str, ?tag: (x: Int | Str) -> Str) [] -> Str => n {
    return tag(n)
}

// [implicit-same-name] [fn-overload-ambiguous] …and a tie is still the
// ordinary ambiguity ("none is more specific").
fn tagged_tie(n: Int, ?tag: (x: Int, y: Int | Str) -> Str, ?tag: (x: Int | Str, y: Int) -> Str) [] -> Str => n {
    //~ error[implicit-same-name:ambiguous]
    return tag(n, n)
}
