// [fn-by] A declared parameter named like a function the template's body
// calls would capture those calls once the parameters are renamed (ROADMAP
// 0u): `std.core.auto`'s `cmp` calls `cmp(…)` per field. The declared names
// are the contract and a local shadows a name [call-resolve], so it is one
// error at the parameter — not the stamped body's "`cmp` is not callable" —
// and nothing is stamped for the member.
struct P : Ordered<self> {
    x: Int
    //~ error[fn-by:param-hides-name]
    fn cmp(cmp: P, b: P) -> Int by auto
}

// Any other name stamps, the declared one included.
struct P2 : Ordered<self> {
    x: Int
    fn cmp(first: P2, b: P2) -> Int by auto
}

// (The same capture through a group's clause stays a Rust test: it also
// reports `group-obligation:no-member`, ROADMAP §0ar.)
