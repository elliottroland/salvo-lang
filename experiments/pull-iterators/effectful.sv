// I4 prototype — an **effectful** producer, in the planned language.
//
// Not compilable today: the checker accepts all of this ([iter-effects] landed
// 2026-09-07), and both backends *refuse* it, because threading handlers into a
// generated pass is what this prototype is fixing the shape of.
//
// What it is chosen to exercise, in the order it matters:
//   * a producer that performs effects while the consumer drives it, declared
//     on its **return type** (`Console Iter<Int>`) — so calling `chatty` needs
//     no handler and driving it does;
//   * returned as a **factory** (no `Once`), so it is replayable: two loops
//     over two separate calls each start from the beginning;
//   * a `defer` inside the producer that itself **performs an effect**, which
//     is what makes the injected `close` observable for the first time: the
//     consumer `break`s and the producer's `close` still prints;
//   * a fn that **inherits** the claim from a producer parameter (`total`
//     declares no effects of its own);
//   * and the **variance** case — a *pure* producer passed where a claiming
//     one is expected, which is where the two representations meet.

fn chatty(limit: Int) -> Console Iter<Int> {
    println("open")
    defer { println("close") }
    let i = 0
    while i < limit {
        println("make ${i}")
        yield copy(i)
        i = i + 1
    }
}

fn plain(limit: Int) -> Iter<Int> {
    let i = 0
    while i < limit {
        yield copy(i)
        i = i + 1
    }
}

// Inherits `Console` from its parameter [iter-effects]: the only reason to
// take a producer is to drive it, and driving this one performs `Console`.
fn total(xs: Console Iter<Int>) -> Int {
    let sum = 0
    for v in xs {
        sum = sum + v
    }
    return sum
}

fn main() [use] -> None {
    use StdOutConsole()
    for v in chatty(3) {
        println("got ${v}")
        if v == 1 {
            break
        }
    }
    println("sum ${total(chatty(2))}")
    // A pure producer fits a claiming position: fewer effects fit where more
    // are expected [iter-effects].
    println("plain ${total(plain(4))}")
}

// Expected stdout — the interleaving is the point: the producer's work happens
// while the consumer drives it, and the deferred `close` runs on the path the
// consumer abandoned as well as the one it drained.
//
//   open
//   make 0
//   got 0
//   make 1
//   got 1
//   close
//   open
//   make 0
//   make 1
//   close
//   sum 1
//   plain 6
