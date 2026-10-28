// [linear-group] The opaque form: `linear intrinsic type` carries the
// obligation on a type whose representation belongs to the backend (user
// decision 2026-09-15 — `Reply<T>` is a scheduler handle, so there is nothing
// to make a `linear struct` of). With a consuming `intrinsic fn` beside it —
// std's `send` for `Reply<T>` — *values* of the type owe like any other
// linear value: dropping one is a leak, sending it discharges it. (The
// declaration-site rule, that the declaring file holds a discharger, needs a
// std module of the test's own: `linear_group_tests.rs`.)

fn answer(out: Reply<Int>) -> None => !out {
    send(out, 1)
}

//~ error[ir-linear-check:leak-at-exit]
fn drop_the_reply(out: Reply<Int>) -> Int => !out {
    return 1
}
