// ===== [deduce-field] field-granular mutation entries (rung ⑤ v1) =====
// A struct with two fields, one of them a container, and the fns the cases
// call; std's `first` gives the optional and the `NonEmpty` overloads.

struct Ring { power: Int }

struct Entity canbe Mut {
    hp: Int,
    rings: Mut List<Ring>
}

fn damage(e: Mut Entity, n: Int) [] -> None => e.hp: Mut, n {
    e.hp = e.hp - n
}

fn shrink(e: Mut Entity) [] -> None => e.rings: Mut {
    add(e.rings, Ring { power: 1 })
}

fn wipe(e: Mut Entity) [] -> None => !e.rings {
    e.rings = Mut List()
}

// [deduce-field] `=> e.hp: Mut` narrows the event a call produces, so a
// derivation of a **disjoint** field survives it — by the ordinary overlap
// rule [fate-field-disjoint], with no new invalidation rule (user decision
// 2026-09-25).
fn spares_a_disjoint_field(e: Mut Entity) [] -> Int => e.hp: Mut {
    let rings = e.rings
    damage(e, 5)
    return size(rings)
}

// [deduce-field] Rung ⑤ v2: a **contents** mutation of the field spares a
// handle to the *container itself* — its storage is still there — while a
// derivation reaching **into** the contents falls, because an element may be
// gone. The `crosses` bit is what tells the two apart, and it cannot be read
// off the path: `first(e.rings)` records `[.rings]` exactly as `e.rings`
// does.
fn spares_the_container(e: Mut Entity) [] -> Int => e.rings: Mut {
    let rings = e.rings
    shrink(e)
    return size(rings)
}

fn not_its_elements(e: Mut Entity) [] -> Int => e.rings: Mut {
    let ring = first(e.rings) ?: unreachable!()
    shrink(e)
    //~ error[fate-poison:shared-root]
    return ring.power
}

// [deduce-field] `=> !e.rings` states a **replacement** — the field's storage
// identity is destroyed — so the derivation falls. This is what keeps the
// backends in step: a spared binding would re-read the new value on Rust
// while Kotlin held the old object.
fn a_declared_replacement(e: Mut Entity) [] -> Int => !e.rings {
    let rings = e.rings
    wipe(e)
    //~ error[fate-poison:shared-root]
    return size(rings)
}

// [deduce-field] A written field entry is a **promise about where**, and
// every caller's precision rests on it: a body that mutates outside the
// declared set is an error naming the remedy.
//~ error[deduce-field:unpromised-mutation]
fn liar(e: Mut Entity, n: Int) [] -> None => e.hp: Mut, n {
    e.hp = e.hp - n
    e.rings = Mut List()
}

// ===== [fn-param-const] a parameter is a constant binding =====

// [fn-param-const] Assigning a parameter is refused — for a scalar as much as
// for a struct, since the rule is about the *binding*, not the type (user
// decision 2026-09-25, pre-aligning with `const`). Before this the construct
// type-checked and was then refused by the target compilers: Kotlin because
// parameters are `val`, Rust for any borrowed one.
fn assigns_a_scalar(n: Int) [] -> Int => n {
    //~ error[fn-param-const:assigned]
    n = n + 1
    return n
}

fn assigns_a_struct(e: Mut Entity) [] -> None => e: Mut {
    //~ error[fn-param-const:assigned]
    e = Mut Entity { hp: 1, rings: Mut List() }
}

// [fn-param-const] `n++` is an assignment to `n`, so it is refused too.
fn steps(n: Int) [] -> Int => n {
    //~ error[fn-param-const:assigned]
    n++
    return n
}

// [fn-param-const] What stays legal: assigning a **field** of a parameter
// (the way to change what the caller holds), and binding a local.
fn assigns_a_field(e: Mut Entity, n: Int) [] -> Int => e.hp: Mut, n {
    e.hp = e.hp - n
    let next = n + 1
    return next
}

// [deduce-field] **No inference** (since 2026-10-20, PIPELINE.md 4.3): a
// callee that writes no field entry mutates its parameter *anywhere*, as a
// written `=> e: Mut` does — the field set is the callee's interface, never
// read off its body. (From 2026-09-25 until then it was inferred.)
fn quiet(e: Mut Entity, n: Int) [] -> None {
    e.hp = e.hp - n
}

fn after_quiet(e: Mut Entity) [] -> Int {
    let rings = e.rings
    quiet(e, 5)
    //~ error[fate-poison:shared-root]
    return size(rings)
}

fn loud(e: Mut Entity, n: Int) [] -> None => e: Mut, n {
    e.hp = e.hp - n
}

fn after_loud(e: Mut Entity) [] -> Int {
    let rings = e.rings
    loud(e, 5)
    //~ error[fate-poison:shared-root]
    return size(rings)
}

// [deduce-field] An unwritten field set narrows nothing, whatever the body
// does: replacing the field, or handing the whole value to a mutator (these
// were the cases where the former inference had to stay conservative).
fn quiet_replaces(e: Mut Entity, n: Int) [] -> None {
    e.hp = e.hp - n
    e.rings = Mut List()
}

fn after_quiet_replaces(e: Mut Entity) [] -> Int {
    let rings = e.rings
    quiet_replaces(e, 5)
    //~ error[fate-poison:shared-root]
    return size(rings)
}

fn quiet_forwards(e: Mut Entity, n: Int) [] -> None {
    e.hp = e.hp - n
    wipe(e)
}

fn after_quiet_forwards(e: Mut Entity) [] -> Int {
    let rings = e.rings
    quiet_forwards(e, 5)
    //~ error[fate-poison:shared-root]
    return size(rings)
}

// ===== [qual-field-place] qualifiers on struct fields (the ⑤ follow-on) =====

// [qual-field-place] A qualifier claim about a **field place** narrows, is
// consumed by overload resolution, and — now that events are field-granular
// [deduce-field] — **survives a mutation of a disjoint field**. That last
// part is what the rung's work bought: flow facts were always place-keyed
// [flow-place], so the feature was waiting on the precision of invalidation,
// not on new machinery.
fn bump_hp(e: Mut Entity) [] -> None => e.hp: Mut {
    e.hp = e.hp + 1
}

fn claim_survives(e: Mut Entity) [] -> Int {
    if e.rings is NonEmpty {
        bump_hp(e)
        return first(e.rings).power
    }
    return 0
}

// [qual-field-place] …and a mutation of the **claimed** field clears it: the
// total overload stops resolving, so the optional one answers and the
// program must narrow again.
fn claim_falls(e: Mut Entity) [] -> Int {
    if e.rings is NonEmpty {
        shrink(e)
        //~ error[struct-field-resolve:not-a-struct]
        return first(e.rings).power
    }
    return 0
}
