// The *oracle* for gnarly.sv: the same body in push style, with the deferred
// blocks expressed as real Rust scope guards.
//
// Why this exists. A hand-written state machine is only trustworthy against
// something that decides the answer independently, and hand-tracing the
// interleaving of a suspended body's `defer`s is exactly the reasoning most
// likely to be wrong. So the body is written here in the one style where no
// bookkeeping is needed at all — `yield` becomes a callback — and the
// deferred code becomes a `Drop` impl, which makes *Rust's own scope
// discipline* the authority on when it runs and in what order. Salvo's
// [defer] rule (at the end of the enclosing block, latest-registered first,
// on every exit path) is precisely Rust's drop order for locals.
//
// Whatever this prints is what gnarly.rs and gnarly.kt must print.

struct Guard(String);

impl Drop for Guard {
    fn drop(&mut self) {
        println!("{}", self.0);
    }
}

/// The hand-written pass from gnarly.sv, as the pull protocol renders it.
struct Countdown {
    at: i32,
}

impl Countdown {
    fn next(&mut self) -> Option<i32> {
        if self.at <= 0 {
            return None;
        }
        let v = self.at;
        self.at -= 1;
        Some(v)
    }
}

/// `walk`, in push style: `yield v` is `out(v)`, and `out` returns false when
/// the consumer wants no more — the `break` in gnarly.sv's `for`.
///
/// Note what is *not* here: no state, no resume points, no release
/// bookkeeping. Returning early drops the guards in scope, latest first.
fn walk_push(limit: i32, out: &mut impl FnMut(i32) -> bool) {
    println!("open");
    let _close = Guard("close".to_string());
    let mut row = 0;
    while row < limit {
        let r = row;
        let _row_end = Guard(format!("row {r} end"));
        let mut inner = Countdown { at: 2 };
        while let Some(col) = inner.next() {
            if col == 1 {
                continue;
            }
            if !out(r * 10 + col) {
                return;
            }
        }
        if !out(r * 100) {
            return;
        }
        row += 1;
    }
    if !out(999) {
        // The `while` ended, so no `_row_end` is live here — only `_close`.
        return;
    }
}

fn main() {
    let mut seen = 0;
    walk_push(2, &mut |v| {
        println!("got {v}");
        seen += 1;
        seen != 4
    });
    println!("done");
}
