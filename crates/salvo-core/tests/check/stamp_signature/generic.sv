// [comptime-generic] [obligation-by] At a generic target the member's
// signature carries the generic additions (the signature it stamps is
// `stamp_signature_tests.rs`'s).
// FILE: derive.sv
export comptime fn cmp<T is Struct>(a: T, b: T) [] -> Int => a, b {
    [for field in T.fields] {
        let c = cmp@[site](a.[field], b.[field])
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
