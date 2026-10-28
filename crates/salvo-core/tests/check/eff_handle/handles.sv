// [effect-eff-handle] First-class effect handles: `eff[E]` as a value type,
// `use H()` as an expression that registers a handler and yields its handle,
// a fn that returns/accepts a handle, and set-subsumption (`eff[A, B]` where
// `eff[A]` is wanted). User decisions 2026-10-18.

effect Clock {
    fn now() -> Int
}
handler Ticking() of Clock {
    state { tick: Int = 0 }
    fn now() -> Int {
        tick = tick + 1
        return tick
    }
}

// [effect-eff-handle] `use H()` is an expression: `let c = use H()` registers
// the handler and binds its handle. A fn may return the handle
// (`-> eff[Clock]`) and accept one (`clock: eff[Clock]`), and `use clock`
// binds a handed-in handle.
fn make() [use] -> eff[Clock] {
    let c = use Ticking()
    return c
}

fn read(clock: eff[Clock]) [use] -> Int => clock {
    use clock
    return now()
}

fn main() [use] {
    let c = make()
    let _n = read(c)
}

// [effect-eff-handle] A bare statement `use H()` still binds the effect in
// scope and yields no binding — the discarded-value case.
fn as_a_statement() [use] {
    use Ticking()
    let _n = now()
}

// [effect-eff-handle] A tuple handle's elements are ordinary single handles:
// you project one out by destructuring and pass it where that single handle
// is wanted (the narrow-from-wide direction, done by projection rather than
// subsumption).
effect Beep { fn beep() -> Int }
handler Both() of Clock, Beep {
    state { n: Int = 0 }
    fn now() -> Int {
        n = n + 1
        return n
    }
    fn beep() -> Int { return 2 }
}

fn projected_by_destructuring() [use] {
    let (c, _b) = use Both()
    let _n = read(c)
}

//--- handles of several effects
effect A { fn a() -> Int }
effect B { fn b() -> Int }
effect C { fn c() -> Int }

// [effect-eff-handle] A multi-effect handle is a *tuple* of single handles. A
// multi-face `use H()` as a value yields `(eff[A], eff[B])`, which
// destructures into the individual handles; `use e` over the whole tuple
// binds every effect.
handler Both() of A, B {
    state { n: Int = 0 }
    fn a() -> Int {
        n = n + 1
        return n
    }
    fn b() -> Int { return 2 }
}
fn make() [use] -> eff[A, B] {
    return use Both()
}
fn main() [use] {
    let both = make()
    use both
    let _x = a()
    let _y = b()
}

// [effect-eff-handle] Projection `e[A]` / `e[A, B]` / `e[alias]`: a
// non-consuming copy of the sub-handle carrying the named effects, a subset
// of what `e` carries. Yields a single handle for one effect, a tuple for
// several; an alias expands.
handler All() of A, B, C {
    fn a() -> Int { return 1 }
    fn b() -> Int { return 2 }
    fn c() -> Int { return 3 }
}
effects AB = [A, B]
fn use_a(h: eff[A]) [use] -> Int => h {
    use h
    return a()
}
fn projects() [use] {
    let all = use All()
    let _n = use_a(all[A])     // project one, pass it on
    let ab = all[AB]           // project via alias -> (eff[A], eff[B])
    use ab
    let _x = a()
    let _y = b()
    let (_hc, _hb) = all[C, B] // reordered subset
}

// [effect-eff-handle] Projecting an effect the handle does not carry is
// refused ("does not carry effect `B`").
handler OnlyA() of A {
    fn a() -> Int { return 1 }
}
fn projects_an_absent_effect() [use] {
    let h = use OnlyA()
    //~ error[effect-eff-handle:effect-not-carried]
    let _bad = h[B]
}

// [effect-eff-handle] Merge `eff[a, b]`: combine already-bound handles into
// one handle carrying the union of their effects, which `use` then binds
// whole.
handler HA() of A { fn a() -> Int { return 1 } }
handler HB() of B { fn b() -> Int { return 2 } }
fn run(caps: eff[A, B]) [use] -> Int => caps {
    use caps
    return a() + b()
}
fn merges() [use] {
    let ha = use HA()
    let hb = use HB()
    let _n = run(eff[ha, hb])
}

// [effect-eff-handle] `eff[*]` bundles every effect handle in scope, ordered
// to the expected type so it fits a declared return.
fn boot() [use] -> eff[A, B] {
    let _ha = use HA()
    let _hb = use HB()
    return eff[*]
}
fn boots() [use] {
    let _caps = boot()
}

// [effect-eff-handle] Merging a non-handle is refused ("merges effect
// handles").
fn merges_a_non_handle() [use] {
    let ha = use HA()
    let n = 3
    //~ error[effect-eff-handle:merge-not-handle]
    let _bad = eff[ha, n]
}
