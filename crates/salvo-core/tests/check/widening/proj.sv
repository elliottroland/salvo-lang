// ===== `proj` placement [proj-anywhere] [proj-field] =====
// `first`, `get`, `at` and `List(…)` are std's (`std.core.list`).

struct Person canbe Mut {
    name: Str
}

// [proj-anywhere] `proj(p)` is an ordinary qualifier now, writable
// wherever a type appears: the old return prefix and the union-arm spelling
// mean the same thing, and a parameter may be a bare `proj`.
fn head_a(list: List<Person>) [] -> proj(list) Person? => list {
    return first(list)
}

fn head_b(list: List<Person>) [] -> (proj(list) Person)? => list {
    return first(list)
}

fn hold(p: proj Person) [] -> Int => p {
    return 1
}

// [proj-anywhere] A `proj` in a *return* type must say what it borrows from.
//~ error[proj-infer:return-no-source]
fn head_c(list: List<Person>) [] -> proj Person? => list {
    return None
}

// [proj-anywhere] The source must be a parameter — checked for every
// occurrence, so a tuple borrowing from two parameters names each.
//~ error[proj-infer:not-a-param]
fn head_d(list: List<Person>) [] -> (proj(nope) Person)? => list {
    return first(list)
}

// [proj-anywhere] A `proj` parameter is a kept parameter: the body may read
// it and may not move it. [proj-type] Caught at the type level: a projection
// into a consuming position.
fn eat_person(p: Person) [] -> None => !p {}

fn hold_and_move(p: proj Person) [] -> Int => p {
    //~ error[proj-type:projection-consumed]
    eat_person(p)
    return 1
}

// ---- [proj-field] [proj-infer] [proj-readonly] (user decisions 2026-09-11) ----

// [proj-field] Any struct may hold a `proj` field; it is written without a
// source — each literal decides what it projects.
struct View<T> canbe Mut {
    items: proj List<T>,
    at: Int
}

fn view<T>(items: List<T>) -> Mut View<T> => items {
    return Mut View<T> { items: items, at: 0 }
}

fn advance<T>(v: Mut View<T>) -> Int => v: Mut {
    v.at = v.at + 1
    return v.at
}

struct Bad<T> {
    //~ error[proj-field:source-named]
    items: proj(x) List<T>
}

// [proj-infer] A constructed view is an owned object: it may be advanced
// (its `Mut` is real) while it keeps the source alive. Since [ref-handle]
// (P-3's lift, user decisions 2026-09-24) a wholesale projection that
// *carries `Mut`* — an element handle out of `List<Mut T>` — may be
// mutated too; only the read-only projection stays read-only
// [proj-readonly].
fn a_held_view_may_be_advanced() {
    let xs = List(1, 2)
    let v = view(xs)
    advance(v)
    advance(v)
}

// [ref-handle] A mutable element handle, minted by `at`, carries `Mut`, and
// the `Mut` position accepts it.
fn a_mut_element_handle_may_be_advanced(vs: List<Mut View<Int>>) {
    let v = at(vs, 0) ?: unreachable!()
    advance(v)
}

// [ref-handle] A projection out of `List<Mut T>` by `first` is a plain
// `proj`, which is read-only whatever its `Mut` says: a handle is minted with
// `at`.
fn a_plain_projection_of_a_mut_element_may_not(vs: List<Mut View<Int>>) {
    let v = first(vs) ?: unreachable!()
    //~ error[ref-handle:plain-projection-mutated]
    advance(v)
}

// A read-only projection still may not.
fn a_read_only_projection_may_not(vs: List<View<Int>>) {
    let v = first(vs) ?: unreachable!()
    //~ error[call-resolve:no-overload]
    advance(v)
}

// [proj-readonly] A parameter written with a top-level `proj Mut` is
// refused at the *declaration* (user decision 2026-09-12): the `proj`
// promises to accept borrows, the `Mut` refuses every one of them. The
// nested spelling (`List<proj Mut Int>`) stays legal — there the pair is
// the element type a view really holds — and so does a local annotation.
//~ error[proj-readonly:proj-mut-param]
fn read_proj_mut(s: proj Mut Str) -> Int => s {
    return 0
}

fn read_nested(xs: List<proj Mut Str>) -> Int {
    return 0
}

// [proj-infer] Moving the source while a view of it lives is refused —
// the link is inferred from `view`'s body, with nothing written.
fn eat<T>(xs: List<T>) -> Int => !xs { return 0 }

fn a_view_keeps_its_source_alive(xs: List<Int>) => !xs {
    let v = view(xs)
    eat(xs)
    //~ error[fate-poison:shared-root]
    advance(v)
}

// [proj-infer] A written `holds proj(a)` must match the body exactly; a
// returned view rooted in a local is refused.
//~ error[proj-infer:lends-disagree]
fn mk<T>(a: List<T>, b: List<T>) -> Mut View<T> holds proj(a) => a, b {
    return Mut View<T> { items: b, at: 0 }
}

// `local` aliases `a` (a parameter), so this is fine: the root is `a`.
fn mk_local<T>(a: List<T>) -> Mut View<T> => a {
    let local = a
    return Mut View<T> { items: local, at: 0 }
}

// [proj-infer] `holds proj(xs)` alone keeps everything (it is not an
// afterwards-qualifier), and a bodiless declaration may carry it. The first
// `advance` is fine (the entry keeps `xs` and lends it); the second sees the
// view poisoned by `eat(xs)` — the link came from the written `proj`, there
// being no body to infer it from.
effect Src {
    fn borrowed(xs: List<Int>, tag: Str) -> Mut View<Int> holds proj(xs) => xs, tag
}

fn a_written_proj_entry_keeps_and_declares(xs: List<Int>, tag: Str) [Src] => !xs, tag {
    let v = borrowed(xs, tag)
    advance(v)
    eat(xs)
    //~ error[fate-poison:shared-root]
    advance(v)
}

// [ref-handle] The mutable-element-handle discipline (P-3's lifts + P-9,
// user decisions 2026-09-24): two *read* handles coexist; mutating through
// one handle poisons a sibling derivation of the container; the acting
// handle itself survives its own mutation (its storage did not move). std's
// `get` is a derived optional read-only projection (`proj(list) T`), and its
// `at` the mint of a mutable handle (`ref(list) T`) [ref-handle].
struct Counter canbe Mut {
    n: Int
}

// Two read handles, then reads: fine (P-9: mode is inferred, and nothing
// here mutates).
fn two_read_handles_coexist(xs: List<Mut Counter>) -> Int {
    let a = get(xs, 0) ?: unreachable!()
    let b = get(xs, 1) ?: unreachable!()
    return a.n + b.n
}

// The handle survives its own mutation.
fn a_handle_survives_its_own_mutation(xs: List<Mut Counter>) -> Int {
    let h = at(xs, 0) ?: unreachable!()
    h.n = 5
    return h.n
}

// A sibling derivation of the container dies at the mutation.
fn the_sibling_is_poisoned(xs: List<Mut Counter>) -> Int {
    let a = get(xs, 0) ?: unreachable!()
    let h = at(xs, 0) ?: unreachable!()
    h.n = 5
    //~ error[fate-poison:shared-root]
    return a.n
}
