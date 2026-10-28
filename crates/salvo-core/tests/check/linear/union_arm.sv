// [linear-union-arm] — O-C2. The fallible-open shape (user decision
// 2026-09-12): a written linear union arm is legal, the un-narrowed union
// owes, and narrowing decides — the `Ok` branch discharges with `close`, the
// `Err` branch owes nothing (an `Err Str` never held the handle). `Ok`, `Err`,
// `ok` and `err` are `std.core.result`'s.

linear struct Lines {
    name: Str
}

fn close(l: Lines) -> None => !l { discard(l) }

fn open_lines(n: Str) -> Lines => !n {
    return Lines { name: n }
}

fn open(flag: Bool, n: Str) -> Ok Lines | Err Str
=> !n {
    if flag {
        return ok(open_lines(n))
    }
    return err("empty")
}

fn a_linear_union_arm_settles_by_narrowing(flag: Bool, n: Str) -> None
=> !n {
    let r = open(flag, n)
    when r {
        is Ok { close(r) }
        is Err { }
    }
}

// The `Ok` arm keeps the handle, the `Err` arm owes nothing: settled on one
// path only, which the IR ledger reports at the `when` [linear-obligation].
fn an_unclosed_linear_arm_still_leaks(flag: Bool, n: Str) -> None
=> !n {
    let r = open(flag, n)
    //~ error[linear-obligation:some-paths]
    when r {
        is Ok { }
        is Err { }
    }
}

// `T?` falls out of the union rule: `None` owes nothing, so the untaken
// branch of `if h is Lines s { close(s) }` is settled by the else-narrow.
fn find(flag: Bool, n: Str) -> Lines?
=> !n {
    if flag {
        return open_lines(n)
    }
    return None
}

fn an_optional_linear_handle_owes_nothing_on_none(flag: Bool, n: Str) -> None
=> !n {
    let h = find(flag, n)
    if h is Lines s {
        close(s)
    }
}

// The un-narrowed union still owes: binding it and doing nothing leaks.
fn an_unnarrowed_linear_union_leaks(flag: Bool, n: Str) -> None
=> !n {
    //~ error[ir-linear-check:leak-at-exit]
    let _r = open(flag, n)
}
