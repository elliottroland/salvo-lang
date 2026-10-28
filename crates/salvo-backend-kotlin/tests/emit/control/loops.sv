// Loops as values: the `run {}` blocks, result locals and `else` flags a
// value loop lowers to.

//--- loops_lower_to_run_blocks
// [loop-while-value] [kt-loop-value]
// The M6 demo (`LOOPS` in codegen_tests.rs): loop values from body tails,
// `break value`, loop `else` in value and statement position, bare `break`
// (optional value), and a union-typed loop value re-wrapped to the declared
// type [loop-while-value].
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
    // [ir-loop] Value loops become `run {}` blocks with a nullable result
    // local, read narrowed after the loop when the join type has no `None`
    // arm (`Justification::LoopValue`).
    //~ emits: val last: Int = kotlin.run {
    //~ emits: var __loop_3: Int? = null
    //~ emits: __loop_3 = (i * 10)
    //~ emits: val __loop_value_6: Int = __loop_3!!
    let last = while i++ < 4 {
        i * 10
    } else {
        // The `else` runs (and assigns) only when the loop never did.
        //~ emits: var __ran_4: Boolean = false
        //~ emits: if (!(__ran_4)) {
        //~ emits: __loop_3 = (-1)
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
    // Without `else` (or with a bare `break`) the value stays nullable: no
    // `!!` on the `found`/`capped` loops.
    //~ emits: val __loop_value_11: Int? = __loop_10
    //~|
    let found = for x in List(0, 1, 2, 3, 4, 5) {
        if x * x > 10 {
            // `break value` assigns the result local before breaking.
            //~ emits: __loop_10 = x
            //~|                 break
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
    //~ emits: val __loop_value_18: Int? = __loop_16
    //~|
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
    // A union-typed loop value keeps the declared arm order.
    //~ emits: var __loop_22: Union2<String, Int>? = null
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
