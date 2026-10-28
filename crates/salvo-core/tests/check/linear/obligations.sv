// The L6/L7a matrices: a linear value's obligation on every path, and the
// generic opt-in.

//--- l6_linear_obligations
// [linear-obligation] L6: values of a `: Linear<self>` type must be used —
// moved onward or `close`d — on every path. The negative matrix:
// scope-exit leak, consumed-on-some-paths-only, dropped expression
// result, overwriting a live value, returning while owing, `copy`
// refused, a lambda swallowing an obligation, and — since R4 part 2 —
// storing one in a struct field [linear-composite]. The positive matrix
// (pass onward, `close`, return with the caller obligated, kept-parameter
// borrow, derived alias) is clean.
//
// [ir-analyze] The path-dependent rules are the IR analysis's — the
// leaks (`leak`, `return_while_owing`), a value consumed on some paths
// only (`maybe_leak`), an assignment over a live one (`overwrite`, R8
// piece 7: the last two were the checker's) — reported, per declaration,
// beside the structural violations in the other fns.
linear struct FileHandle {
    fd: Int
}

fn close(x: FileHandle) -> None => !x { discard(x) }

// [linear-composite] [linear-group] `Box2` is a `linear struct` with a
// concrete linear field — legal since 2026-09-12: the container carries
// the obligation, `close_box` discharges it by forwarding, and the
// whole shape draws no error.
linear struct Box2 {
    item: FileHandle
}

fn close(x: Box2) -> None => !x { discard(x) }

linear struct Conn {
    tags: Mut List<Int>
}

fn close(x: Conn) -> None => !x { discard(x) }

fn open_file(path: Str) -> FileHandle => !path {
    return FileHandle {fd: size(path)}
}

fn close_file(h: FileHandle) -> None => !h {
    close(h)
}

fn close_box(b: Box2) -> None => !b {
    close(b)
}

fn inspect(h: FileHandle) -> Int => h {
    return h.fd
}

fn run(f: () -> None) {
    f()
}

fn leak() {
    //~ error[ir-linear-check:leak-at-exit]
    //~ warning[var-unused:never-used]
    let h = open_file("data.txt")
}

fn drop_result() {
    //~ error[linear-obligation:dropped-statement]
    open_file("data.txt")
}

fn return_while_owing(flag: Bool) -> Int {
    //~ error[ir-linear-check:leak-at-exit]
    let h = open_file("data.txt")
    if flag {
        return 0
    }
    close_file(h)
    return 1
}

fn copy_refused() {
    let h = open_file("data.txt")
    //~ error[linear-generics:copy-linear]
    let c = copy(h)
    close_file(h)
    close_file(c)
}

fn swallow() {
    let conn = Conn {tags: Mut List(1)}
    //~ error[linear-lambda:captures-linear]
    let g = () -> { add(conn.tags, 2) }
    run(g)
    close(conn)
}

fn ok_pass() {
    let h = open_file("data.txt")
    close_file(h)
}

fn ok_discard() {
    let h = open_file("data.txt")
    close(h)
}

fn ok_return() -> FileHandle {
    let h = open_file("data.txt")
    return h
}

fn ok_kept_borrow() {
    let h = open_file("data.txt")
    let _n = inspect(h)
    close_file(h)
}

fn ok_alias() {
    let h = open_file("data.txt")
    let alias = h
    let _n = inspect(alias)
    close_file(h)
}

fn composite_refused() {
    let h = open_file("data.txt")
    let b = Box2 {item: h}
    close_box(b)
}

fn caller_obligated() {
    let h = ok_return()
    close_file(h)
}

fn maybe_leak(flag: Bool) {
    let h = open_file("data.txt")
    //~ error[linear-obligation:some-paths]
    if flag {
        close_file(h)
    }
}

fn overwrite() {
    let h = open_file("a.txt")
    //~ error[linear-obligation:assign-drops]
    h = open_file("b.txt")
    close_file(h)
}

//--- l6_generics_refuse_linear_types
// [linear-generics] Generic instantiation with a linear type is refused
// unless the type parameter declares `<T canbe linear>`; variadic
// positions refuse linear values outright (they are untracked).
linear struct FileHandle {
    fd: Int
}

fn close(x: FileHandle) -> None => !x { discard(x) }

//~ error[deduce-default:kept-by-default-moved]
fn hold<T>(value: T) -> T {
    return value
}

fn generic_refused() {
    let h = FileHandle {fd: 1}
    //~ error[linear-generics:not-opted]
    let kept = hold(h)
    //~ error[linear-discard:linear]
    discard(kept)
}

fn variadic_refused() {
    let h = FileHandle {fd: 1}
    let h2 = FileHandle {fd: 2}
    //~ error[linear-container:position]
    //~ error[linear-generics:variadic]
    let _xs = List(h, h2)
}

//--- l7a_generic_linear_opt_in
// [linear-generics] L7a: `<T canbe linear>` opts a generic fn into
// linear instantiation — its body treats `T` values as linear (a
// written-moved parameter that the body drops is rejected), forwarding
// an opted `T` to an *unopted* generic is rejected, only `Linear` is
// accepted in the clause, and the opted std surface makes the
// `List<FileHandle>` workflow legal (construct empty, `add`
// individually — variadic positions refuse linear values — `size`,
// `discard`).
linear struct FileHandle {
    fd: Int
}

fn close(x: FileHandle) -> None => !x { discard(x) }

fn open_file(n: Int) -> FileHandle {
    return FileHandle {fd: n}
}

fn hold<T canbe linear>(value: T) -> T
=> !value {
    return value
}

// [ir-analyze] The opted body treats `T` as linear: `eat` drops its
// moved `value`, a path-exit leak the IR reports beside the structural
// errors in the other fns.
//~ error[ir-linear-check:leak-at-exit]
fn eat<T canbe linear>(value: T) -> None => !value {
}

// Forwarding to an unopted generic is rejected.
fn forward<T canbe linear>(value: T) -> T
=> !value {
    //~ error[linear-generics:not-opted]
    let kept = unopted(value)
    return kept
}

fn unopted<T>(value: T) -> T
=> !value {
    return value
}

// Only `Linear` is supported in the clause.
//~ error[linear-generics:with-clause]
fn bad_clause<T canbe Mut>(value: T) -> T
=> !value {
    return value
}

// Variadic positions refuse linear values; [linear-composite] a linear
// value cannot be stored in a variadic `T[]`.
fn variadic_refused() {
    let h = open_file(1)
    let h2 = open_file(2)
    //~ error[linear-container:position]
    //~ error[linear-generics:variadic]
    let _xs = Mut List(h, h2)
}

fn ok_hold() {
    let h = hold(open_file(9))
    close(h)
}
