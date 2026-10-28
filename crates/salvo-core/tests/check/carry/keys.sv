// ===== [col-key-eligible] where a key's own identity is required =====

// [col-key-eligible] [cmp-carry] A key needs its **own** `hash`/`eq` only
// where the identity is resolved by name. Written identities, binders and a
// `let` pattern the initializer fills all say where the identity comes from,
// so a struct with none of its own is a key there — the annotation and the
// call it types agree (found 2026-10-20: the call was accepted and the same
// value's annotation refused).
struct P { x: Int }

fn my_hash(v: P) [] -> Long => v { return hash(v.x) }
fn my_eq(a: P, b: P) [] -> Bool => a, b { return a.x == b.x }

fn count_keys(s: Set<P>(?hash, ?eq)) [] -> Int => s { return size(s) }

fn written_identities() [] -> Int {
    let s = Mut Set<P>(hash = my_hash, eq = my_eq)
    let t: Mut Set<P> = Mut Set(hash = my_hash, eq = my_eq)
    let u: Mut Set<P>(my_hash, my_eq) = Mut Set(hash = my_hash, eq = my_eq)
    return count_keys(s) + count_keys(t) + count_keys(u)
}

// Where nothing is written the key's own pair is still required, and the
// refusal names the clause that stamps one ("`P` cannot be a key",
// "Hashed<self> by auto").
//~ error[col-key-eligible:key]
fn count_plain(s: Set<P>) [] -> Int => s { return size(s) }

// [col-key-eligible] What no identity fixes is refused however the identity
// is given, and once: a float key with a hand-written pair, and `Set(1.5)`
// whose constructor would otherwise also report the `hash` it cannot find
// ("`Double` cannot be a key").
fn dh(d: Double) [] -> Long => d { return 1L }
fn de(a: Double, b: Double) [] -> Bool => a, b { return a == b }

fn float_key_written() [] -> Int {
    //~ error[col-key-eligible:key]
    let t = Mut Set<Double>(hash = dh, eq = de)
    return size(t)
}

//--- a float key resolved by name
// `Set(1.5)`: a program of its own, since an ineligible key type is refused
// once per program.
fn float_key_resolved() [] -> Int {
    //~ error[col-key-eligible:key]
    let s = Set(1.5)
    return size(s)
}

//--- an identity is a declaration
// FILE: p.sv
// ===== [cmp-carry] an identity is a declaration =====
//
// The program of ROADMAP §0n's aliasing defect: two modules each declare a
// `my_hash` (and `my_eq`) for one `P` and build sets with them.
export struct P : Hashed<self> by auto { x: Int }
// FILE: a.sv
import p.P
export fn my_hash(v: P) [] -> Long => v { return hash(v.x) }
export fn my_eq(l: P, r: P) [] -> Bool => l, r { return l.x == r.x }
export fn make_a() [] -> Mut Set<P>(my_hash, my_eq) {
    return Mut Set<P>(hash = my_hash, eq = my_eq)
}
// FILE: b.sv
import p.P
fn my_hash(v: P) [] -> Long => v { return 7L }
fn my_eq(l: P, r: P) [] -> Bool => l, r { return true }
export fn make_b() [] -> Mut Set<P>(my_hash, my_eq) {
    return Mut Set<P>(hash = my_hash, eq = my_eq)
}
// FILE: main.sv
import p.P
import a.make_a
import a.my_hash
import a.my_eq
import b.make_b

// [cmp-carry] [cmp-binder] Two modules' same-named fns are **two
// identities**: one binder cannot hold both, and the types name each by its
// module ("Mut Set<P>(my_hash@a, my_eq@a)", "Mut Set<P>(my_hash@b,
// my_eq@b)"; found 2026-10-20 — they compared equal by name, and the binder
// was filled with the reading module's `my_hash`, a silently wrong program).
fn same(x: Mut Set<P>(?hash, ?eq), y: Mut Set<P>(?hash, ?eq)) [] -> None => x, y {}

fn run() [] -> None {
    //~ error[call-resolve:no-overload]
    same(make_a(), make_b())
}

// [cmp-binder] A binder filled from a value's identity calls **that**
// declaration, wherever the value is read: `b`'s private `my_hash` reaches a
// file that imports `a`'s under the same name (here), and one that imports
// neither (`other.sv`).
fn has(x: Mut Set<P>(?hash, ?eq), v: P) [] -> Bool => x, v { return contains(x, v) }

fn read_here() [] -> Bool {
    return has(make_b(), P { x: 1 })
}
// FILE: other.sv
import p.P
import b.make_b

fn has(x: Mut Set<P>(?hash, ?eq), v: P) [] -> Bool => x, v { return contains(x, v) }

fn read_there() [] -> Bool {
    return has(make_b(), P { x: 1 })
}

//--- a written identity two modules fit
// FILE: lib.sv
// [cmp-carry] [fn-overload-ambiguous] A written identity has no rung
// fallback (R10 D23): `SortedSet<Str>(by_len)` beside an imported `by_len` of
// the same signature is ambiguous, an error naming `by_len@lib` or
// `by_len@main` — once, though the type is lowered at every use. Before, the
// most specific rung's (this module's) was taken silently, while the call
// `by_len(a, b)` was refused.
export fn by_len(a: Str, b: Str) [] -> Int => a, b { return size(b) - size(a) }
// FILE: main.sv
import lib.by_len

fn by_len(a: Str, b: Str) [] -> Int => a, b { return size(a) - size(b) }

fn run() [] -> Int {
    //~ error[cmp-carry:ambiguous]
    let m: Mut SortedSet<Str>(by_len) = Mut SortedSet("a", cmp = by_len@main)
    return size(m)
}

// `by_len@main` settles it.
fn run_settled() [] -> Int {
    let m: Mut SortedSet<Str>(by_len@main) = Mut SortedSet("a", cmp = by_len@main)
    return size(m)
}
