// Handles: what a `ref` handle's identity tests lower to.

//--- notsame_handles
// [ref-notsame] [fate-elem-distinct] Two handles proven apart by `NotSame`
// through a custom container's accessor (map slots, one slot's roster, the
// bench) and through nested structs with no accessor of their own; two
// handles to one element make the test false at run time (`===`). Source
// and expected stdout are verbatim the Rust backend's
// `rustc_compiles_and_runs_notsame_handles` — the parity is the assertion.
//~ emits(std/core/ref.kt): ===
// A player lives on a team's roster (a map slot's list) or on the bench.
struct Player canbe Mut { name: Str, goals: Int }
struct Team canbe Mut { coach: Str, roster: List<Mut Player> }
struct League canbe Mut {
    teams: Map<Str, Mut Team>,
    bench: List<Mut Player>
}

// A custom container's own accessor: the handle's path runs through a map
// slot, or through a field.
fn at(l: League, team: Str, n: Int) [] -> ref(l) Mut Player? => l, team, n {
    if n < 0 {
        return at(l.bench, -n - 1)
    }
    let t = at(l.teams, team)
    if t is None {
        return None
    }
    return at(t.roster, n)
}

fn trade(l: League, a: ref(l) Mut Player, b: ref(l) Mut Player) -> None {
    let g = copy(a.goals)
    a.goals = b.goals
    b.goals = g
}

fn score(a: Mut Player, b: Mut Player) -> None => a: Mut, b: Mut {
    a.goals = a.goals + 1
    b.goals = b.goals + 1
}

fn league() [] -> Mut League {
    let teams: Mut Map<Str, Mut Team> = Mut Map()
    put(teams, "red", Mut Team { coach: "Ada", roster: List(Mut Player { name: "Bo", goals: 0 }, Mut Player { name: "Cy", goals: 5 }) })
    put(teams, "blue", Mut Team { coach: "Dee", roster: List(Mut Player { name: "Ed", goals: 2 }) })
    return Mut League { teams: teams, bench: List(Mut Player { name: "Flo", goals: 9 }) }
}

// Structs with no accessor of their own, nested: the handle's path is the
// field chain `a.b.c` plus a position.
struct Cell canbe Mut { v: Int }
struct Inner canbe Mut { c: List<Mut Cell> }
struct Outer canbe Mut { tag: Str, b: Inner }

fn at(a: Outer, index: Int) [] -> ref(a) Mut Cell? => a, index {
    return at(a.b.c, index)
}

fn bump(x: Mut Cell, y: Mut Cell) -> None => x: Mut, y: Mut {
    x.v = x.v + 1
    y.v = y.v + 10
}

fn shared_container() [Console] -> None {
    let l = league()
    let star = at(l, "red", 1) ?: unreachable!()
    star.goals = star.goals + 1
    println("bench ${size(l.bench)}")
    star.goals = star.goals + 1
    trade(l, at(l, "red", 0) ?: unreachable!(), at(l, "blue", 0) ?: unreachable!())
    trade(l, star, star)
    println("${star.name} ${star.goals}")
    // The bench and a roster: the paths diverge at the first field.
    let sub = at(l, "", -1) ?: unreachable!()
    if sub is NotSame(star) {
        score(star, sub)
        star.goals = star.goals + 10
        sub.goals = sub.goals + 100
    }
    // Two handles to one element: the test answers false at run time.
    let same_one = at(l, "red", 1) ?: unreachable!()
    if same_one is NotSame(star) {
        println("wrongly distinct")
    } else {
        println("same element")
    }
    println("${star.goals} ${(get(l.bench, 0) ?: unreachable!()).goals}")
}

fn map_slots() [Console] -> None {
    let l = league()
    // Two map slots: the paths diverge at the slot.
    let a = at(l, "red", 0) ?: unreachable!()
    let b = at(l, "blue", 0) ?: unreachable!()
    if b is NotSame(a) {
        score(a, b)
        println("${a.goals} ${b.goals}")
    }
    // One slot, two positions in its roster.
    let c = at(l, "red", 1) ?: unreachable!()
    if c is NotSame(a) {
        score(a, c)
    }
    let d = at(l, "", -1) ?: unreachable!()
    if d is NotSame(c) {
        score(c, d)
    }
    println("${c.goals} ${d.goals}")
}

fn nested() [Console] -> None {
    let a = Mut Outer { tag: "t", b: Inner { c: List(Mut Cell { v: 1 }, Mut Cell { v: 2 }) } }
    let p = at(a, 0) ?: unreachable!()
    let q = at(a, 1) ?: unreachable!()
    if q is NotSame(p) {
        bump(p, q)
        p.v = p.v + 100
        q.v = q.v + 200
    }
    let r = at(a, 0) ?: unreachable!()
    if r is NotSame(p) {
        println("wrongly distinct")
    } else {
        println("same cell")
    }
    println("${(get(a.b.c, 0) ?: unreachable!()).v} ${(get(a.b.c, 1) ?: unreachable!()).v}")
}

fn main() [use] {
    use StdConsole()
    shared_container()
    map_slots()
    nested()
}
