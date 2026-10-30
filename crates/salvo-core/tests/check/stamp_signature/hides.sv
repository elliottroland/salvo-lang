// [fn-by] [comptime-hygiene] `std.core.auto`'s `cmp` calls `cmp@[site](…)`
// per field, and a `name@[site]` does not see the stamped fn's declared
// parameters (user decision 2026-10-30, P23 item 6): their names are the
// user's, so a parameter named `cmp` captures nothing and the member stamps.
// Until then this was `fn-by:param-hides-name`.
struct P : Ordered<self> {
    x: Int
    fn cmp(cmp: P, b: P) -> Int by auto
}

struct P2 : Ordered<self> {
    x: Int
    fn cmp(first: P2, b: P2) -> Int by auto
}

// A bare name the template's file does not see is the site's, but a declared
// parameter of that name still captures it once the parameters are renamed
// (ROADMAP 0u): one error at the parameter, and nothing stamped. (Binding
// the parameters by position, PIPELINE.md C3.3, is what lets this go too.)
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
