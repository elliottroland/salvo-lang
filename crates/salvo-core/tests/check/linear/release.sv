// Releasing a linear value **on every path** [linear-obligation].
//
// This was `defer_tests.rs` until 2026-09-10, when `defer` was removed from
// the language (user decision): a deferred block discharged an obligation on
// every exit, but linearity is what *checks* that the obligation is
// discharged, so `defer` was the partial solution to a problem the full one
// already covers. What is left to verify is that the checker demands the
// release on each path and accepts it when written — including the paths an
// author is most likely to forget: an early `return`, a `break`, and a call
// that may throw.

import std.throw
import std.process

linear struct Handle {
    fd: Int
}

fn open_handle(fd: Int) [] -> Handle {
    return Handle { fd: fd }
}

fn close_handle(h: Handle) [] -> None => !h {
    discard(h)
}

fn note(text: Str) [] -> None => text {}

// The motivating case, and the reason `defer` is not needed: a path that
// leaves without releasing is an error naming the value.
fn a_path_that_leaves_without_releasing_is_reported(flag: Bool) [] -> None {
    //~ error[ir-linear-check:leak-at-exit]
    let _h = open_handle(3)
    if flag {
        return
    }
    note("still open here")
}

// Written out on both paths, it is accepted — which is what "linearity is
// the full solution" means in practice: the compiler names the path you
// missed.
fn releasing_on_every_path_is_accepted(flag: Bool) [] -> None {
    let h = open_handle(3)
    if flag {
        close_handle(h)
        return
    }
    note("still open here")
    close_handle(h)
}

// A `break` is an exit like any other, and the value the *body* created dies
// with the iteration.
fn a_break_must_release_what_the_iteration_owns(flag: Bool) [] -> None {
    while flag {
        //~ error[ir-linear-check:leak-at-exit]
        let h = open_handle(1)
        if flag {
            break
        }
        close_handle(h)
    }
}

fn a_break_that_releases_is_accepted(flag: Bool) [] -> None {
    while flag {
        let h = open_handle(1)
        if flag {
            close_handle(h)
            break
        }
        close_handle(h)
    }
}

// The path `defer` was most useful for: a call that may throw. The code
// after it does not run on the throw path, so the release has to precede the
// call — and the diagnostic says so rather than naming a construct that no
// longer exists.
fn risky(flag: Bool) [Throw<Str>] -> None {
    if flag { return throw("no") }
}

fn a_call_that_may_throw_must_release_first(flag: Bool) [Throw<Str>] -> None {
    //~ error[ir-linear-check:leak-across-throw]
    let h = open_handle(3)
    risky(flag)
    close_handle(h)
}

// [cli-process] [type-any-never] A call to an effect member answering
// `Never` (`Process.exit`) ends the path the way a trap does: the process
// does not continue, so a linear value live at the call is not a leak —
// while the same value live at a `return` is.
fn a_linear_value_live_at_an_exit_is_not_a_leak(flag: Bool) [Process] -> None {
    let h = open_handle(3)
    if flag {
        exit(2)
    }
    close_handle(h)
}

fn a_linear_value_live_at_a_return_is(flag: Bool) [Process] -> None {
    //~ error[ir-linear-check:leak-at-exit]
    let h = open_handle(3)
    if flag {
        return
    }
    close_handle(h)
}
