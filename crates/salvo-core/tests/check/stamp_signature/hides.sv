// [fn-by] [comptime-hygiene] A declared parameter named like a function the
// template calls at the site would capture those calls once the parameters
// are renamed (ROADMAP 0u): `std.core.auto`'s `cmp` calls `cmp@[site](…)`
// per field, which reads as the bare name there, parameters included. The
// declared names are the contract and a local shadows a name
// [call-resolve], so it is one error at the parameter — not the stamped
// body's "`cmp` is not callable" — and nothing is stamped for the member.
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

// A bare name the template's file does not see is the site's, and a declared
// parameter of that name would still capture it once the parameters are
// renamed (ROADMAP 0u): one error at the parameter, and nothing stamped.
//--- a_bare_site_name
// FILE: derive.sv
export comptime fn cmp<T is Struct>(a: T, b: T) [] -> Int => a, b {
    return rank(a) - rank(b)
}
// FILE: main.sv
import derive

fn rank(p: R) [] -> Int => p {
    return p.x
}

struct R : Ordered<self> {
    x: Int
    //~ error[fn-by:param-hides-name]
    fn cmp(rank: R, b: R) -> Int by derive
}
