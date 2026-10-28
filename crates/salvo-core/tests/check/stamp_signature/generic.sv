// [comptime-generic] [obligation-by] At a generic target the member's
// signature carries the generic additions, and a template whose names differ
// from the group's still stamps the group's names (the signature it stamps
// is `stamp_signature_tests.rs`'s).
// FILE: derive.sv
export comptime fn cmp<T is Struct>(x: T, y: T) [] -> Int => x, y {
    [for field in T.fields] {
        let c = cmp(x.[field], y.[field])
        if c != 0 {
            return c
        }
    }
    return 0
}

// FILE: main.sv
import derive

struct W<T> : Ordered<self> by derive { item: T, n: Int }

fn probe() [] -> Int {
    return cmp(W { item: "a", n: 1 }, W { item: "b", n: 1 })
}
