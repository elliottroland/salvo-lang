// [fn-by] [comptime-hygiene] A `name@[site]` does not see the stamped fn's
// parameters (user decision 2026-10-30, P23 item 6): a template whose
// parameter shares a name with the site's function it calls reaches the
// function. Here the parameter is `cmp`, and `cmp@[site](…)` per field is
// the site's `cmp` of the field's type.
// FILE: derive.sv
export comptime fn cmp<T is Struct>(cmp: T, b: T) [] -> Int => cmp, b {
    [for field in T.fields] {
        let c = cmp@[site](cmp.[field], b.[field])
        if c != 0 {
            return c
        }
    }
    return 0
}
// FILE: main.sv
import derive

struct P : Ordered<self> {
    x: Int
    fn cmp(cmp: P, b: P) -> Int by derive
}

fn probe() [] -> Int {
    return cmp(P { x: 1 }, P { x: 2 })
}
