// I3 prototype — the *gnarly* resumable body, in the planned language.
// Not compilable today: an effectful iterator fn and the state-machine
// lowering are what this is validating.
//
// Everything that has to be resumable at once, in one body:
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
// Driven by a consumer that stops early, so the release path runs while the
// body is suspended mid-nest.

struct Countdown canbe Mut, Once {
    at: Int
}

fn next(c: Mut Countdown) -> [c: Mut] Emitted Int | Finished {
    if c.at <= 0 {
        return finished()
    }
    let v = copy(c.at)
    c.at = c.at - 1
    return emitted(v)
}

fn countdown(from: Int) -> Once Countdown {
    return Countdown { at: from }
}

fn walk(limit: Int) [Console] -> Once Iter<Int> {
    println("open")
    defer { println("close") }
    let row = 0
    while row < limit {
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
    for v in walk(2) {
        println("got ${v}")
        seen = seen + 1
        if seen == 4 {
            break
        }
    }
    println("done")
}

// Expected stdout — derived by the *oracle*, not by hand: see
// gnarly_oracle.rs, which expresses the same body in push style and lets
// Rust's own scope guards decide when the deferred code runs.
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
