// [effect-member-unique] [effect-member-overload] The member rules every
// effect shares (platform effects were removed 2026-10-01).

// [effect-member-unique] [effect-member-overload] Two members of one effect
// may share a name as an *overload* (user decision 2026-09-14) — what stays
// an error is the same name taking the same types, which no call could tell
// apart.
effect Store {
    fn put(value: Int) [] -> None
    //~ error[effect-member-unique:duplicate]
    fn put(value: Int) [] -> Int
}

// [effect-member-overload] The overload itself is legal: different parameter
// types, one name.
effect Shelf {
    fn put(value: Int) [] -> None
    fn put(value: Str) [] -> None => value
}

// [effect-member-overload] Two *different* effects may share a member
// name (user decision 2026-09-14, lifting the 2026-09-05 program-wide
// ban): a call disambiguates by which effect has a handler in scope, or
// explicitly with `member@Effect(…)` [effect-at]. The declaration itself
// is legal.
effect A {
    fn ping() [] -> None
}

effect B {
    fn ping() [] -> None
}

// [effect-member-unique] The *within-one-effect* duplicate stays an
// error, reported exactly once, at the second declaration.
effect C {
    fn ping() [] -> None
    //~ error[effect-member-unique:duplicate]
    fn ping() [] -> Int
}

effect D {
    fn ping() [] -> None
}

// [effect-member-unique] The rule is about *collisions*, not about effects
// having members: distinct names across effects stay legal.
effect E {
    fn tick() [] -> None
}

effect F {
    fn tock() [] -> None
}
