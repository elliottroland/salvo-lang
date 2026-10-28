// [fate-elem-distinct] Distinct awareness for mutable element handles: poison
// consults a live `NotSame` claim between two bound handles before killing the
// sibling [ref-notsame], the claim dies when either handle is rebound, and one
// call may take two handles only when they are proven apart — which
// statement-scoped mints never are.
//
// Over std's `std.core.list`: `at(es, i)` with a plain `Int` is the optional
// mint (`ref(es) T?`, picked with `?: unreachable!()`); with an `Idx(es) Int`
// it is the total one, and so is `get`.

struct Entity canbe Mut { hp: Int }

fn attack(a: Mut Entity, d: Mut Entity) [] -> None => a: Mut, d: Mut {
    a.hp = a.hp - 1
    d.hp = d.hp - 2
    return None
}

// [fate-elem-distinct] [ref-notsame] Two bound handles a live `NotSame` claim
// proves apart survive each other's mutations: the sibling names disjoint
// storage, so the poison spares it — and a write *through* a handle leaves the
// claim standing, so the second round of writes is accepted too.
fn two_proven_distinct_handles_coexist_across_mutation(es: List<Mut Entity>, i: Int, j: Int) [] -> None {
    let a = at(es, i) ?: unreachable!()
    let d = at(es, j) ?: unreachable!()
    if d is NotSame(a) {
        a.hp = a.hp + 1
        d.hp = d.hp + 1
        a.hp = a.hp + 1
        d.hp = d.hp + 1
    }
    return None
}

// [fate-elem-distinct] Without the claim, today's conservative rule stands: a
// computed index may alias every element [fate-field-disjoint], so the mutation
// through one handle poisons the sibling [fate-poison].
fn without_a_claim_the_sibling_handle_poisons(es: List<Mut Entity>, i: Int, j: Int) [] -> None => es, i, j {
    let a = at(es, i) ?: unreachable!()
    let d = at(es, j) ?: unreachable!()
    a.hp = a.hp + 1
    // Both uses of the poisoned `d` report: the target and the operand.
    //~ error[fate-poison:shared-root]
    //~ error[fate-poison:shared-root]
    d.hp = d.hp + 1
    return None
}

// [fate-elem-distinct] [ref-notsame] [qual-depend] The claim is about the
// handle a name holds *now*: rebinding `d` to another mint strips it, and the
// conservative poison returns — even though both names still exist.
fn rebinding_a_handle_strips_the_claim(es: List<Mut Entity>, i: Int, j: Int) [] -> None => es, i, j {
    let a = at(es, i) ?: unreachable!()
    let d = at(es, j) ?: unreachable!()
    if d is NotSame(a) {
        d = at(es, i) ?: unreachable!()
        a.hp = a.hp + 1
        //~ error[fate-poison:shared-root]
        //~ error[fate-poison:shared-root]
        d.hp = d.hp + 1
    }
    return None
}

// [fate-elem-distinct] The same index minted twice with no proof is the same
// element as far as the checker is concerned: the sibling poisons. (Proving
// such a pair apart is a run-time test that answers false — the
// compile-and-run cases on both backends cover it.)
fn the_same_index_without_a_proof_still_poisons(es: List<Mut Entity>, i: Int) [] -> None => es, i {
    let a = at(es, i) ?: unreachable!()
    let d = at(es, i) ?: unreachable!()
    a.hp = a.hp + 1
    //~ error[fate-poison:shared-root]
    //~ error[fate-poison:shared-root]
    d.hp = d.hp + 1
    return None
}

// [fate-elem-distinct] [ref-notsame] Statement-scoped mints cannot carry the
// proof — a claim is about two *bound* handles — so two of them in one call
// are refused even inside a `NotSame` block over other handles of the same
// elements, naming the remedy (`b is NotSame(a)`).
fn statement_scoped_mints_cannot_carry_the_proof(es: List<Mut Entity>, i: Int, j: Int) [] -> None => es, i, j {
    let a = at(es, i) ?: unreachable!()
    let d = at(es, j) ?: unreachable!()
    if d is NotSame(a) {
        //~ error[fate-elem-distinct:two-handles]
        attack(at(es, i) ?: unreachable!(), at(es, j) ?: unreachable!())
    }
    return None
}

// [fate-elem-distinct] Without the proof the pair is refused at the call,
// naming the remedy — the shape used to pass the checker and die at rustc
// (E0499), a checker/emitter disagreement closed with this rule.
fn an_unproven_pair_in_one_call_is_refused(es: List<Mut Entity>, i: Int, j: Int) [] -> None => es, i, j {
    //~ error[fate-elem-distinct:two-handles]
    attack(at(es, i) ?: unreachable!(), at(es, j) ?: unreachable!())
    return None
}

// [fate-elem-distinct] [ref-notsame] Two *bound* handles proven apart land in
// one call — the shape the Rust backend renders through `salvo_pair_mut` — and
// both survive it: writes through either afterwards are accepted.
fn a_proven_bound_pair_may_land_in_one_call(es: List<Mut Entity>, i: Int, j: Int) [] -> None {
    let a = at(es, i) ?: unreachable!()
    let d = at(es, j) ?: unreachable!()
    if d is NotSame(a) {
        attack(a, d)
        a.hp = a.hp + 1
        d.hp = d.hp + 1
        attack(d, a)
    }
    return None
}

// [fate-elem-distinct] …and the same bound pair *without* the proof is refused
// at the call.
fn an_unproven_bound_pair_in_one_call_is_refused(es: List<Mut Entity>, i: Int, j: Int) [] -> None => es, i, j {
    let a = at(es, i) ?: unreachable!()
    let d = at(es, j) ?: unreachable!()
    //~ error[fate-elem-distinct:two-handles]
    attack(a, d)
    return None
}

// [qual-depend] A parameter's **declared** dependent claim is live in the body
// — its roots fill from the sibling parameters at declaration — so the
// claim-demanding total overload resolves. (Before 2026-09-24 the declared
// claim stayed a root-free template and only `is`-established claims worked.)
fn a_declared_claim_serves_the_total_overload_in_the_body(es: List<Mut Entity>, i: Idx(es) Int) [] -> Int => es, i {
    return get(es, i).hp
}

// [fate-elem-distinct] The pair rule covers calls **through fn values** too
// ([fn-contract]'s keep-in-sync duty): two unproven handles into one container
// are refused at the second argument.
fn touch2_unproven(es: List<Mut Entity>, i: Idx(es) Int, j: Idx(es) Int,
                   f: (a: Mut Entity, b: Mut Entity) -> None) [] -> None {
    //~ error[fate-elem-distinct:two-handles]
    f(at(es, i), at(es, j))
    return None
}

// [fate-elem-distinct] [ref-notsame] …and a `NotSame` proof over two bound
// handles legalizes it, ordinary Salvo end to end.
fn touch2_proven(es: List<Mut Entity>, i: Idx(es) Int, j: Idx(es) Int,
                 f: (a: Mut Entity, b: Mut Entity) -> None) [] -> None {
    let a = at(es, i)
    let d = at(es, j)
    if d is NotSame(a) {
        f(a, d)
    }
    return None
}

// A callee taking two handles anchored at one container parameter
// [ref-anchor]: `ref(c)` on both says they may be the same element.
fn duel(c: List<Mut Entity>, a: ref(c) Mut Entity, d: ref(c) Mut Entity) [] -> None {
    a.hp = a.hp - 1
    d.hp = d.hp - 2
    return None
}

// [ref-anchor] ④b — a callee whose two handle parameters are both `ref(c)`
// takes two element handles of one container with **no disjointness proof**:
// the same-call rule stands down exactly for the pair anchored at one
// container parameter — including the self-strike.
fn a_covered_pair_needs_no_proof(es: List<Mut Entity>, i: Idx(es) Int, j: Idx(es) Int) [] -> None {
    duel(es, at(es, i), at(es, j))
    duel(es, at(es, i), at(es, i))
    return None
}

// [ref-anchor] …and an *uncovered* callee still refuses the pair: the
// exemption is exactly anchor-shaped (P-6).
fn duel_uncovered(a: Mut Entity, d: Mut Entity) [] -> None => a: Mut, d: Mut {
    a.hp = a.hp - 1
    d.hp = d.hp - 2
    return None
}

fn an_uncovered_callee_still_refuses_the_pair(es: List<Mut Entity>, i: Idx(es) Int, j: Idx(es) Int) [] -> None {
    //~ error[fate-elem-distinct:two-handles]
    duel_uncovered(at(es, i), at(es, j))
    return None
}

// [GB-fix-1b] [ref-anchor] A `ref(c)` pair drawn from **two different
// containers** is refused at the call: each handle must come from the very
// container passed for its `ref(…)` parameter. (The `canbe` shape this
// replaces used to compile and silently miscompile on Rust — one list's handle
// redirected onto the other — a backend-parity break.)
fn a_covered_pair_from_two_containers_is_refused(es: List<Mut Entity>, fs: List<Mut Entity>, i: Idx(es) Int, j: Idx(fs) Int) [] -> None {
    //~ error[ref-anchor:not-a-handle]
    duel(es, at(es, i), at(fs, j))
    return None
}

// [GB-fix-1b] [ref-anchor] …and the same pair drawn from **one container** is
// accepted — the refusal is only for the different-container case, nothing
// else.
fn a_covered_pair_from_one_container_is_accepted(es: List<Mut Entity>, i: Idx(es) Int, j: Idx(es) Int) [] -> None {
    duel(es, at(es, i), at(es, j))
    return None
}

// [ref-handle] A plain `get` is read-only even through a fn value: a `Mut`
// position of the fn type refuses it, as a named callee does.
fn a_read_only_projection_does_not_fit_a_fn_values_mut_position(es: List<Mut Entity>, i: Idx(es) Int, f: (a: Mut Entity) -> None) [] -> None {
    //~ error[ref-handle:plain-projection]
    f(get(es, i))
    return None
}

// [ref-anchor] A handle parameter's container must be another parameter.
//~ error[ref-anchor:not-a-param]
fn bad(c: List<Mut Entity>, a: ref(nosuch) Mut Entity) [] -> None {
    a.hp = a.hp - 1
    return None
}

// [proj-opt-slot] A borrowed optional fits a kept owned optional parameter — of
// a fn and of an effect member — whether it is a read-only `get` or an
// optional handle (user decision 2026-10-10).
fn hp_of(e: Entity?) [] -> Int => e {
    if e is Entity {
        return e.hp
    }
    return 0
}

effect Reader {
    fn read(e: Entity?) -> Int => e
}

fn a_borrowed_optional_fits_a_kept_owned_optional(es: List<Mut Entity>) [Reader] -> Int => es {
    let h = at(es, 0)
    return hp_of(get(es, 0)) + hp_of(at(es, 1)) + hp_of(h) + read(get(es, 2)) + read(h)
}

// [proj-opt-slot] A `Mut` optional position still needs the owned value: the
// lift is for reading positions only.
fn heal(e: Mut Entity?) [] -> None => e: Mut {
    if e is Entity {
        e.hp = e.hp + 1
    }
    return None
}

fn a_borrowed_optional_does_not_fit_a_mut_optional(es: List<Mut Entity>) [] -> None => es {
    //~ error[ref-handle:plain-projection-mutated]
    heal(get(es, 0))
    return None
}

// ===== handles into a chain of element containers [ref-anchor] =====

struct Track canbe Mut { plays: Int }

struct Album canbe Mut { tracks: List<Mut Track> }

struct Library canbe Mut { albums: List<Mut Album> }

fn swap_in(lib: Library, al: ref(lib) Mut Album, a: ref(al) Mut Track, b: ref(al) Mut Track) [] -> None {
    let p = a.plays
    a.plays = b.plays
    b.plays = p
    return None
}

// [ref-anchor] A handle anywhere inside the container fits, and a `ref(c)` may
// name another handle parameter: the handles of one chain are positions in one
// container, so they may alias with nothing to prove (user decisions
// 2026-10-10).
fn handles_into_an_element_container_chain_need_no_proof(lib: Library) [] -> None => lib: None {
    let al = at(lib.albums, 0) ?: unreachable!()
    swap_in(lib, al, at(al.tracks, 0) ?: unreachable!(), at(al.tracks, 0) ?: unreachable!())
    swap_in(lib, at(lib.albums, 1) ?: unreachable!(), at((at(lib.albums, 1) ?: unreachable!()).tracks, 0) ?: unreachable!(), at((at(lib.albums, 1) ?: unreachable!()).tracks, 1) ?: unreachable!())
    return None
}

// [ref-anchor] …but a handle into *another* album than the one passed for its
// anchor is refused: the callee would read it as a position in `al`.
fn a_handle_outside_its_anchor_is_refused(lib: Library, other: Album) [] -> None => lib, other {
    let al = at(lib.albums, 0) ?: unreachable!()
    //~ error[ref-anchor:not-a-handle]
    swap_in(lib, al, at(other.tracks, 0) ?: unreachable!(), at(al.tracks, 0) ?: unreachable!())
    return None
}
