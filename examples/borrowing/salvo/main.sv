// Borrowing, without references: a *projection* is a borrow, a projection
// carrying `Mut` is a **handle** on storage someone else owns, and an **alias
// group** is what a function says when two of its parameters may name the same
// object.
//
// Salvo has no `&`, no `&mut`, no lifetimes. What it has is one qualifier —
// `proj` — and three answers to the question Rust answers with a single rule
// ("a mutable borrow excludes all others"): say nothing and hold one handle at
// a time, *prove* two handles apart, or *declare* that they may coincide.
//
// Several shapes below are ones the natural Rust translation does not compile
// at all; they are marked `Rust: E0499` / `Rust: E0502` and the README has the
// rustc output for each. They run here because the Rust backend renders a
// mutable handle as a **position** (a container plus an index, re-materialized
// at each use) rather than as a reference.

// The subject: a fighter, mutable in place.
struct Fighter canbe Mut {
    // Who it is.
    name: Str,
    // What it has left.
    hp: Int,
    // What it can spend.
    energy: Int
}

// ===== 1. a read projection =====

// Searches a roster and lends back what it found. The result type says what
// the value *is*: `proj(roster) Fighter` — the element itself, borrowed from
// the parameter. Nothing is copied, and the caller's list keeps owning it.
//
// `return roster` would be a different sentence: returning a parameter *moves*
// it, because a return value is owned by the caller. A projection is how you
// hand back access without handing over ownership.
fn named(roster: List<Fighter>, name: Str) -> (proj(roster) Fighter)? => roster, name {
    for f in roster {
        if f.name == name {
            return f
        }
    }
    return None
}

// A **view**: an owned struct that holds a borrow. The struct declares *that*
// it projects (`roster: proj List<Fighter>`), each literal says *what*. It is
// an ordinary value — mutable in place, movable, storable — except that it may
// not outlive what it borrows, which the compiler tracks without anything
// being written on `window` below.
struct Window canbe Mut {
    // The roster this window looks into.
    roster: proj List<Fighter>,
    // Where it is looking.
    at: Int
}

fn window(roster: List<Fighter>) -> Mut Window => roster {
    return Mut Window { roster: roster, at: 0 }
}

// The window lends on: the element is a projection of `w`, which projects the
// roster. Two links, one borrow — and `peek` needs no `Mut`, since reading
// through a view is not a mutation of it.
fn peek(w: Window) -> (proj(w) Fighter)? => w {
    return get(w.roster, w.at)
}

// ===== 2. a mutable projection is a handle =====

// `Mut` on a parameter of a struct type is permission to change it in place.
// The call site supplies a handle; `heal` never learns where it came from.
fn heal(f: Mut Fighter) -> None => f: Mut {
    f.hp = f.hp + 10
    return None
}

// A lender of your own, and the shape to look at in the generated Rust: the
// loop becomes an indexed one and the found *position* travels out
// (`wounded__loc(squad: &Vec<Fighter>) -> Option<usize>`), so no reference
// outlives the search.
fn wounded(squad: List<Mut Fighter>) -> (proj(squad) Mut Fighter)? {
    for f in squad {
        if f.hp < 10 {
            return f
        }
    }
    return None
}

// A *generic* lender. `params Locate<C, L, T>` is std's bundle for
// position-based code — one function turning a container and a position into
// the element's handle — so this function is generic over what a position even
// is, while the caller, which knows the shape, fills `at` in.
//
// In the generated Rust the implicit is a **locator** closure
// (`&mut dyn FnMut(&Vec<Fighter>, &L) -> Option<usize>`): position data
// crosses the boundary and the handle is materialized on the other side, so
// nothing borrows across the call.
fn rally_at<L>(squad: List<Mut Fighter>, l: L, ?Locate<List<Mut Fighter>, L, Fighter>) -> None {
    heal(at(squad, l)!)
    return None
}

// ===== 3. two handles, proven apart =====

// An ordinary two-handle function: it says it mutates both, and says nothing
// about them coinciding — so every caller owes a proof that they do not.
fn duel(a: Mut Fighter, d: Mut Fighter) -> None => a: Mut, d: Mut {
    a.hp = a.hp - 1
    d.hp = d.hp - 2
    return None
}

// ===== 4. an alias group =====

// `=> a canbe d` is the other answer: this function *means* to accept two
// handles that might be one object, so no caller needs a proof and the
// self-strike case is legal. The relation is symmetric (saying it once says
// it) and non-transitive (it is a graph, not an equivalence).
//
// Rust: E0499. Two `&mut` into one container cannot coexist whether they
// alias or not, and no signature can say "these may be the same" — a Rust
// programmer writes the `i == j` case as a second function. The Rust backend
// renders a covered pair as one shared anchor plus a position each, so the
// aliasing is *exact*: both positions index the same storage.
fn strike(a: Mut Fighter, d: Mut Fighter) -> None
=> a canbe d, a: Mut, d: Mut {
    a.energy = a.energy - 1
    d.hp = d.hp - 2
    return None
}

// A squad, so the *anchored* form has a container to name.
struct Squad canbe Mut {
    // What it fights under.
    banner: Str,
    // Who is in it.
    members: List<Mut Fighter>
}

// `=> from|to canbe in squad.members` is the anchored form: each parameter may
// be an **element of a named container**. Two parameters anchored in the same
// path may therefore coincide, so the mutual aliasing falls out of the shared
// anchor — which is how the n-way case costs one entry per parameter instead
// of one per pair. It also licenses the shape a plain `canbe` cannot: handles
// passed *beside the container they came from*.
fn rotate(squad: Mut Squad, from: Mut Fighter, to: Mut Fighter) -> None
=> from|to canbe in squad.members, squad: Mut, from: Mut, to: Mut {
    from.energy = from.energy - 1
    to.energy = to.energy + 1
    return None
}

// ===== 5. which field a call touches =====

struct Camp canbe Mut {
    // What the camp has.
    supplies: Int,
    // What it flies.
    banners: Mut List<Str>
}

// `=> camp.supplies: Mut` narrows the mutation event to one field, so a handle
// into a *different* field survives the call. Written `=> camp: Mut` it would
// mean "mutated somewhere", and everything derived from `camp` would fall.
// Where nothing is written the field set is inferred from the body — the
// precision is the default, not a reward for annotating.
fn spend(camp: Mut Camp, n: Int) -> None => camp.supplies: Mut, n {
    camp.supplies = camp.supplies - n
    return None
}

// A **contents** mutation of the other field: the list grows, but it is still
// the same list, so a handle to it keeps working and sees the change. The
// other half of the distinction is `=> !camp.banners`, which says the field is
// *replaced* — storage identity destroyed, every handle to it gone.
fn hoist(camp: Mut Camp, banner: Str) -> None => camp.banners: Mut, !banner {
    add(camp.banners, banner)
    return None
}

fn main() [use] {
    use StdOutConsole()

    // ----- 1. read projections -----

    let roster: List<Fighter> = list_of(
        Fighter { name: "Ada", hp: 30, energy: 4 },
        Fighter { name: "Bo", hp: 8, energy: 9 })

    // A borrow of the element, read in place. Zero copies so far.
    let ada = named(roster, "Ada")!
    println("1. found ${ada.name}, hp ${ada.hp}")

    // A projection may be read anywhere, but *moving* one needs `copy` —
    // `add` takes ownership of its element, so `add(names, ada.name)` is an
    // error naming the remedy. The copy is written, which is the whole
    // principle: a copy never happens without the program asking for one.
    let names: Mut List<Str> = mut_list_of()
    add(names, copy(ada.name))
    println("1. copied out ${to_str(names)}")

    // A view holds its borrow across statements, and lends on from there.
    let w = window(roster)
    w.at = 1
    println("1. window at ${w.at}: ${peek(w)!.name}")

    // `filter` answers `Mut List<proj Fighter>` — a list *of borrows*,
    // holding a borrow of the pass it walked. Nothing was copied to build it.
    let pass = iter(roster)
    let standing = filter(pass, (f: Fighter) -> { return f.hp > 10 })
    println("1. ${size(standing)} of ${size(roster)} still standing")

    // ----- 2. handles -----

    // Two different permissions, and only the second lends handles:
    //   `Mut List<T>` — a mutable *container* (add, remove, swap)
    //   `List<Mut T>` — mutable *elements*, fixed shape
    let bench: Mut List<Fighter> = mut_list_of(Fighter { name: "Cy", hp: 12, energy: 2 })
    add(bench, Fighter { name: "Dee", hp: 6, energy: 7 })
    println("2. bench ${size(bench)}, front ${get(bench, 0)!.name} (a reading, not a handle)")

    let squad: List<Mut Fighter> = list_of(
        Mut Fighter { name: "Ada", hp: 30, energy: 4 },
        Mut Fighter { name: "Bo", hp: 8, energy: 9 })

    // A handle, bound and then held across a *read* of the container it
    // points into — a read cannot invalidate a handle, so both are live.
    //
    // Rust: E0502. A bound `&mut squad[0]` forbids the `size(squad)` between
    // the two writes; the position rendering is what lets rustc agree.
    let boss = get(squad, 0)!
    boss.hp = boss.hp + 1
    let n = size(squad)
    boss.hp = boss.hp + n
    println("2. ${get(squad, 0)!.name} at ${get(squad, 0)!.hp} after a read in the middle")

    // A search that lends what it found, mutated through where it is minted.
    heal(wounded(squad)!)

    // The same, through a caller-supplied accessor: `at` comes from the
    // `Locate` bundle, and `1` is what a position happens to be for a list.
    rally_at(squad, 1)
    println("2. after the searches: ${get(squad, 0)!.hp} ${get(squad, 1)!.hp}")

    // ----- 3. two handles at once -----

    let i = 0
    let j = 1

    // Say nothing and one handle lives at a time: `get(squad, i)` and
    // `get(squad, j)` may be the same element for all the compiler knows, so
    // the second is refused. `NotEq` is the proof — `j != i`, bound to `i`'s
    // identity — and with it the two handles coexist, a write through one
    // leaves the other standing, and one call may take both.
    //
    // Rust: E0499, and the remedy rustc suggests is `split_at_mut` — which is
    // exactly what this lowers to.
    if j is NotEq(i) {
        let a = get(squad, i)!
        let d = get(squad, j)!
        a.hp = a.hp + 1
        d.hp = d.hp + 1
        duel(a, d)
    }

    // std wraps the common shapes so a caller writes neither the handles nor
    // the claim. Both preserve `Idx`, so the reads after them stay total —
    // an in-place write moves no boundary.
    if i is Idx(squad) {
        if j is Idx(squad) {
            update(squad, i, (f: Mut Fighter) -> { f.energy = f.energy + 1 })
            if j is NotEq(i) {
                update2(squad, i, j, (a: Mut Fighter, b: Mut Fighter) -> {
                    a.energy = a.energy + 100
                    b.energy = b.energy + 200
                })
            }
            println("3. ${get(squad, i).energy} ${get(squad, j).energy} (total reads: `Idx` survived)")
        }
    }

    // ----- 4. the alias group -----

    // No proof anywhere, and none needed: `strike` covers the pair itself.
    if i is Idx(squad) {
        if j is Idx(squad) {
            strike(get(squad, i), get(squad, j))
            // …including the case the entry exists for: one fighter, both
            // roles, spending its own energy on itself.
            strike(get(squad, i), get(squad, i))
        }
    }
    println("4. ${get(squad, 0)!.hp} hp / ${get(squad, 0)!.energy} energy after striking itself")

    // The anchored form, where the container travels with its own elements:
    // one `&mut` of the squad in the generated Rust, and a position each.
    let team = Mut Squad { banner: "Red", members: list_of(
        Mut Fighter { name: "Cy", hp: 12, energy: 2 },
        Mut Fighter { name: "Dee", hp: 6, energy: 7 }) }
    rotate(team, get(team.members, i)!, get(team.members, j)!)
    println("4. ${team.banner}: ${get(team.members, 0)!.energy} ${get(team.members, 1)!.energy}")

    // ----- 5. field granularity -----

    let camp = Mut Camp { supplies: 10, banners: mut_list_of("red") }

    // A handle into one field, held across a call that mutates another.
    //
    // Rust: E0502. `spend` takes the whole struct `&mut`, so a live borrow of
    // `camp.banners` conflicts with it — field-level borrow splitting does not
    // cross a function boundary. Here the clause says where the write lands,
    // and the handle survives because the field it names was untouched.
    let banners = camp.banners
    spend(camp, 3)

    // …and it is still the same list, so a contents mutation through the
    // struct is visible through the handle.
    hoist(camp, "blue")
    println("5. supplies ${camp.supplies}, banners ${to_str(banners)}")
}
