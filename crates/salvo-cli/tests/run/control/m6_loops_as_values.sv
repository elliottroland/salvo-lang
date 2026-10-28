// M6: loops as values
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

// Full verification of the loops demo under kotlinc (skipped when
// kotlinc is not installed).
//--- loops_2

// Its own pass, named `Upto` rather than `Range`: `std.core.range` exports a
// `Range` of its own [mod-export], and two same-named structs in two modules
// confuse the pass-driving resolution (an open defect — ROADMAP.md).
struct Upto {
    start: Int,
    end: Int
}

fn upto(start: Int, end: Int) -> Upto {
    return Upto {start: start, end: end}
}

iter fn iter(r: Upto) -> Emitted Int | Finished {
    state UptoIter2 {
        at: Int = r.start
    }
    if at >= r.end {
        return finished()
    }
    let v = copy(at)
    at = at + 1
    return emitted(v)
}

fn main() [use] -> None {
    use StdConsole

    // Last evaluated body expression is the loop's value.
    let i = 0
    let last = while i++ < 4 {
        i * 10
    } else {
        -1
    }
    println("last: ${last}")

    // `else` provides the value when the loop never runs.
    let j = 9
    let never = while j < 3 {
        j
    } else {
        -1
    }
    println("never: ${never}")

    // `break value` short-circuits; no `else` makes the value optional.
    let found = for x in List(0, 1, 2, 3, 4, 5) {
        if x * x > 10 {
            break x
        }
        x
    }
    if found is Int f {
        println("found: ${f}")
    }

    // Statement-position loop with `else`, over a producer.
    for x in upto(0, 0) {
        println("unreachable")
    } else {
        println("empty range")
    }

    // A bare `break` keeps the previous iteration's value.
    let k = 0
    let capped = while k < 5 {
        k++
        if k == 3 {
            break
        }
        k
    }
    println("capped: ${capped ?: unreachable!()}")

    // Loop values join into unions and re-wrap to the declared type.
    let n = 0
    let verdict: Ok Int | Err Str = while n < 3 {
        if n == 2 {
            break ok(n)
        }
        n++
        err("not yet")
    } else {
        err("empty")
    }
    when verdict {
        is Ok {
            println("ok: ${verdict}")
        }
        is Err {
            println("err: ${verdict}")
        }
    }
}
//= stdout
//= last: 40
//= never: -1
//= found: 4
//= empty range
//= capped: 2
//= ok: 2
