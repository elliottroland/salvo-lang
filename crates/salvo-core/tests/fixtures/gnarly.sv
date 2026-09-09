// The generator planner's acceptance fixture: everything that has to be
// resumable at once, in one body.
//
//   * a `defer` at fn-block level (released on every exit path);
//   * a `defer` *inside a loop body*, registered anew each iteration,
//     capturing a per-iteration local (`r`) and discharged at the end of
//     that iteration — the case a single flag-per-site cannot represent;
//   * a nested loop over *another pass*, which has to stay alive across the
//     outer body's suspensions (so it is a field, not a local);
//   * `continue` inside the inner loop;
//   * two yields per outer iteration, i.e. several resume points;
//   * a yield after the loop;
//   * effects performed by the producer, and by a deferred block.
//
// Driven by a consumer that stops early, so the release path runs while the
// body is suspended mid-nest. This body is the one the I3 prototype's machine
// was hand-written for and checked against a push-style oracle; it is kept
// here, in the current language, as what the planner is asserted against.
//
// It *plans* but does not *emit* today: a pass as the subject of a suspending
// loop is a recorded codegen cut, which is exactly why the planner has to get
// the nested-pass field right in advance.

// The nested pass: a hand-written one, so the outer body drives a `next` that
// is not its own.
struct Countdown : Yield<self, Int> canbe Mut {
    at: Int
}

fn countdown(from: Int) -> [] Mut Countdown {
    return Mut Countdown { at: from }
}

fn next(c: Mut Countdown) -> [c: Mut] Emitted Int | Finished {
    if c.at <= 0 {
        return finished()
    }
    let v = copy(c.at)
    c.at = c.at - 1
    return emitted(v)
}

// The origin whose machine the planner has to write.
struct Grid : Yield<self, Int> {
    limit: Int
}

fn grid(limit: Int) -> [] Grid {
    return Grid { limit: limit }
}

yield fn next(g: Grid) [Console] -> Int {
    println("open")
    defer { println("close") }
    let row = 0
    while row < g.limit {
        let r = copy(row)
        defer { println("row ${r} end") }
        for col in countdown(2) {
            if col == 1 {
                continue
            }
            yield r * 10 + col
        }
        yield r * 100
        row = row + 1
    }
    yield 999
}

fn main() [use] -> None {
    use StdOutConsole()
    let seen = 0
    for v in grid(2) {
        println("got ${v}")
        seen = seen + 1
        if seen == 4 {
            break
        }
    }
    println("done")
}

// Expected stdout — derived by the I3 *oracle*, which expressed the same body
// in push style and let Rust's own scope guards decide when the deferred code
// runs:
//
//   open
//   got 2
//   got 0
//   row 0 end
//   got 12
//   got 100
//   row 1 end
//   close
//   done
