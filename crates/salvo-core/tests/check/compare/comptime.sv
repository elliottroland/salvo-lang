// [comptime-inline] [comptime-instantiate] [obligation-return] The comptime
// forms a `by` clause stamps. (Comptime syntax outside a `comptime fn`, a
// `refuse` at the `by` site and how a `by` path names its `comptime fn` are
// `compare_tests.rs`'s: each reports a second diagnostic for its one mistake.)

// [comptime-inline] A `[when field.kind]` is exhaustive over `Kind`'s arms,
// with no `else` [when-exhaustive], so a kind a comptime fn did not consider is
// an error — the evolution guard (comptime round 3). With every kind, it
// stamps.
comptime fn partial<T is Struct>(v: T) [] -> Int => v {
    let n = 0
    [for field in T.fields] {
        //~ error[when-exhaustive:kind-arms-missing]
        [when field.kind] {
            is Struct { n = n + 1 }
        }
    }
    return n
}

fns Partial<T> {
    fn partial(v: T) -> Int
}

struct P : Partial<self> by partial {
    x: Int
}

comptime fn full<T is Struct>(v: T) [] -> Int => v {
    let n = 0
    [for field in T.fields] {
        [when field.kind] {
            is Struct { n = n + 1 }
            is Union { n = n + 2 }
            is Tuple { n = n + 3 }
            is FnType { n = n + 4 }
            is Opaque { n = n + 5 }
        }
    }
    return n
}

fns Full<T> {
    fn full(v: T) -> Int
}

struct Q : Full<self> by full {
    x: Int
}

// [comptime-instantiate] A concrete `comptime fn` (no bound) is its own single
// instantiation, declared where a fn is — the one-field-by-hand case
// (comptime round 2), with `field.name == "…"` selecting it.
struct Reading : Ordered<self> {
    sensor: Str,
    value: Int

    comptime fn cmp(a: Reading, b: Reading) [] -> Int => a, b {
        [for field in Reading.fields] {
            [if field.name == "value"] {
                let c = cmp(b.value, a.value)
                if c != 0 { return c }
            } else {
                let c = cmp(a.[field], b.[field])
                if c != 0 { return c }
            }
        }
        return 0
    }
}

fn probe(a: Reading, b: Reading) [] -> Bool => a, b {
    return a < b
}

// A field no struct has is caught at the declaration.
struct Misspelled : Ordered<self> {
    sensor: Str

    comptime fn cmp(a: Misspelled, b: Misspelled) [] -> Int => a, b {
        [for field in Misspelled.fields] {
            //~ error[comptime-name:unknown-name]
            [if field.name == "valeu"] {
                return 0
            }
        }
        return 0
    }
}

//--- a member returning self is stamped at the clause
// FILE: decode.sv
// [obligation-return] `struct S : FromStrs<self> by decode` stamps a member
// whose parameters do not mention `S` — a decoder — at the struct the clause
// is on (the CLI design record X6, Q7 (a)). Two structs stamping it in one
// module are return-type overloads [fn-overload-return], chosen by where the
// result goes.
export fns FromStrs<T> {
    fn from_strs(n: Int) -> T
}

export comptime fn from_strs<T is Struct>(n: Int) [] -> T => n {
    return T { [for f in T.fields] { [f]: n } }
}
// FILE: main.sv
import decode

struct A : FromStrs<self> by decode {
    x: Int
}

struct B : FromStrs<self> by decode {
    y: Int,
    z: Int
}

fn probe() [] -> Int {
    let a: A = from_strs(1)
    let b: B = from_strs(2)
    return a.x + b.y + b.z
}

//--- a member returning self, with no place to say which
// FILE: decode.sv
// Without a place that says which, the call names the two.
export fns FromStrs<T> {
    fn from_strs(n: Int) -> T
}

export comptime fn from_strs<T is Struct>(n: Int) [] -> T => n {
    return T { [for f in T.fields] { [f]: n } }
}
// FILE: main.sv
import decode

struct A : FromStrs<self> by decode {
    x: Int
}

struct B : FromStrs<self> by decode {
    y: Int,
    z: Int
}

fn probe() [] -> Int {
    //~ error[fn-overload-return:ambiguous]
    let _a = from_strs(1)
    let b: B = from_strs(2)
    return b.y + b.z
}
