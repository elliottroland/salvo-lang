// Loops as values lower to block expressions.

//--- loops_lower_to_block_expressions
// [loop-while-value] [rs-loop-value]
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

    let i = 0
    // Value loops become block expressions with an Option result local,
    // unwrapped when the join type has no None arm.
    //~ emits: let mut __loop_3: Option<i32> = None;
    //~ emits: __loop_3 = Some(i32::wrapping_mul(i, 10i32));
    //~ emits: let mut __loop_value_6 = __loop_3.unwrap();
    // `else` runs only when the loop never did.
    //~ emits: let mut __ran_4: bool = false;
    //~ emits: if !(__ran_4) {
    let last = while i++ < 4 {
        i * 10
    } else {
        -1
    }
    println("last: ${last}")

    let j = 9
    let never = while j < 3 {
        j
    } else {
        -1
    }
    println("never: ${never}")

    // `break value` assigns before breaking.
    //~ emits: __loop_10 = Some(x);
    //~|                 break;
    // Optional joins keep the plain Option local (no unwrap).
    //~ emits: let mut __loop_value_11 = __loop_10;
    let found = for x in List(0, 1, 2, 3, 4, 5) {
        if x * x > 10 {
            break x
        }
        x
    }
    if found is Int f {
        println("found: ${f}")
    }

    for x in upto(0, 0) {
        println("unreachable")
    } else {
        println("empty range")
    }

    let k = 0
    let capped = while k < 5 {
        k++
        if k == 3 {
            break
        }
        k
    }
    println("capped: ${capped ?: unreachable!()}")

    let n = 0
    // A union-typed loop value re-wraps to the declared arm order.
    //~ emits: } { crate::unions::Union2::U1(__v) => crate::unions::Union2::U2(__v), crate::unions::Union2::U2(__v) => crate::unions::Union2::U1(__v),
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
