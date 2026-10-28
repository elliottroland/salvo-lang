// [platform-type] An opaque host handle in three kinds: a program uses one
// through platform fns, copies a plain one, mutates a linear `canbe Mut` one
// through a `Mut` parameter — and what the kinds forbid is refused at the
// declaration.
platform type Client
threadsafe platform type Parker
linear platform type Cursor canbe Mut
platform type Buffer canbe Mut

platform fn connect() [] -> Client
platform fn ping(c: Client) [] -> Int => c
platform fn open() [] -> Mut Cursor
platform fn step(c: Mut Cursor) [] -> Int => c: Mut
platform fn close(c: Cursor) [] -> None => !c

fn use_them() [] -> Int {
    let c = connect()
    let d = c
    let k = open()
    let n = step(k)
    close(k)
    return ping(d) + n
}

// [platform-value-type] A copyable `canbe Mut` one is a value type
// (`platform type Bytes canbe Mut`), which cannot be shared.
//~ error[platform-value-type:threadsafe-mut]
threadsafe platform type Shared canbe Mut

// (A platform type's `by` clauses stay in `platform_tests.rs`: refused, they
// also report diagnostics of their own, ROADMAP §0ar.)

// [platform-iterable] The host's loop needs the Salvo pass beside it.
//~ error[platform-iterable:no-iter]
iterable platform type Bag canbe Mut

// A linear handle cannot be copied, and is owed: forgetting one leaks.
linear platform type Lease
platform fn take() [] -> Lease
platform fn give(c: Lease) [] -> None => !c

fn leaks() [] -> None {
    //~ error[ir-linear-check:leak-at-exit]
    let _k = take()
}
