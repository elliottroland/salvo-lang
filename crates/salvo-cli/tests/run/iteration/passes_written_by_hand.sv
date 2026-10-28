// passes written by hand
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- a_fallible_pass_yields_a_result

import std.throw

struct Reader : Next<self, Ok Str | Err Str> canbe Mut {
    lines: List<Str>,
    at: Int
}

fn next(r: Mut Reader) -> Emitted (Ok Str | Err Str) | Finished => r: Mut {
    let line = get(r.lines, r.at)
    if line is Str {
        r.at = r.at + 1
        if line == "boom" {
            return emitted(err("bad line at ${r.at}"))
        }
        return emitted(ok(copy(line)))
    }
    return finished()
}

fn reader(lines: List<Str>) -> Reader => !lines {
    return Reader { lines: lines, at: 0 }
}

fn read_all(lines: List<Str>) [Throw<Str>, Console] -> Int
=> !lines {
    let count = 0
    for outcome in reader(lines) {
        when outcome {
            is Ok {
                println("line ${outcome}")
                count = count + 1
            }
            is Err {
                return throw("stopped: ${outcome}")
            }
        }
    }
    return count
}

fn main() [use] {
    use StdConsole()
    let good = try { read_all(List("alpha", "beta")) }
    when good {
        is Ok { println("read ${good}") }
        is Thrown { println("failed: ${good}") }
    }
    let bad = try { read_all(List("alpha", "boom", "gamma")) }
    when bad {
        is Ok { println("read ${bad}") }
        is Thrown { println("failed: ${bad}") }
    }
}
//= stdout
//= line alpha
//= line beta
//= read 2
//= line alpha
//= failed: stopped: bad line at 2

//--- a_hand_written_pass

struct Zip : Next<self, (Str, Int)> canbe Mut {
    left: List<Str>,
    right: List<Int>,
    at: Int
}

fn next(z: Mut Zip) -> Emitted (Str, Int) | Finished => z: Mut {
    let l = get(z.left, z.at)
    let r = get(z.right, z.at)
    if l is Str && r is Int {
        z.at = z.at + 1
        // `get` hands back borrows [copy-opt-in]: building a tuple *stores*
        // the element, so the copy is written where it happens.
        return emitted((copy(l), r))
    }
    return finished()
}

fn zip(left: List<Str>, right: List<Int>) -> Zip => !left, !right {
    return Zip { left: left, right: right, at: 0 }
}

struct Countdown : Next<self, Int> canbe Mut {
    at: Int
}

fn next(c: Mut Countdown) -> Emitted Int | Finished => c: Mut {
    if c.at <= 0 {
        return finished()
    }
    let v = copy(c.at)
    c.at = c.at - 1
    return emitted(v)
}

fn countdown(from: Int) -> Countdown {
    return Countdown { at: from }
}

fn main() [use] {
    use StdConsole()
    for n in countdown(3) {
        println("n ${n}")
    }
    let names = List("ada", "grace", "alan")
    let ages = List(36, 45)
    for pair in zip(names, ages) {
        println("${pair.0} is ${pair.1}")
    }
    println("done")
}
//= stdout
//= n 3
//= n 2
//= n 1
//= ada is 36
//= grace is 45
//= done
