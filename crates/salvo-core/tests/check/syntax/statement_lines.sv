// [stmt-line] A statement, and a top-level declaration, starts on its own
// line: Salvo has no separator, so the end of a line is what ends one (user
// decision 2026-11-11). Each slip is one error, at the second statement's
// first token, and the statement is still parsed, so what follows it is
// checked as if it had been on its own line: a `type?` after a slip is
// reported, and a declaration after one is found.

//--- two_statements_on_one_line_are_refused
fn run() [Console] {
    //~ error[stmt-line:same-line]
    let a = 1 let b = 2
    //~ error[stmt-line:same-line]
    //~ type[Int]
    println("${a}") type?(b)
    //~ error[stmt-line:same-line]
    let c = a + b c = c + 1
    println("${c}")
}

//--- a_statement_after_a_blocks_brace_is_refused
// A construct's `}` ends it unless the grammar continues it (`else`,
// `elif`), so what follows on the line is a second statement…
fn run(n: Int) [Console] {
    //~ error[stmt-line:same-line]
    if n > 0 { println("pos") } println("after")
    //~ error[stmt-line:same-line]
    for i in List(1, 2) { println("${i}") } println("done")
    //~ error[stmt-line:same-line]
    //~ type[Int]
    if n > 0 { println("pos") } else { println("neg") } type?(n)
}

//--- inside_a_one_line_block_too
// …and a one-line block holds one statement: a second inside it is refused,
// in a fn body, an arm, a lambda and a handler's member alike.
effect Clock {
    fn now() -> Int
}

handler Ticking for Clock {
    state {
        t: Int = 0
    }
    //~ error[stmt-line:same-line]
    fn now() -> Int { t = t + 1 return t }
}

fn arms(v: Int | Str) [Console] -> Int {
    when v {
        //~ error[stmt-line:same-line]
        is Int { println("int") return 1 }
        is Str { return 0 }
    }
}

fn lambdas() [Console] -> Int {
    //~ error[stmt-line:same-line]
    let f = (x: Int) [Console] -> Int { println("x") return x }
    return f(1)
}

//--- three_on_one_line_is_two_slips
// One error per statement that shares a line, and the third one is refused
// at its own first token.
fn run() [Console] {
    //~ error[stmt-line:same-line]
    //~ error[stmt-line:same-line]
    println("a") println("b") println("c")
    //~ error[stmt-line:same-line]
    //~ error[stmt-line:same-line]
    //~ type[Int]
    let a = 1 let _b = a type?(_b)
}

//--- a_stray_token_reports_what_it_is_only
// A token that starts no statement is the expression error alone, not also
// a same-line one.
fn main() -> Int {
    //~ error[syntax-expected:expression]
    let a = 1 )
    return a
}

//--- a_terminal_heads_line_is_its_own
// A terminal `...` head ends its line's statement: the body it takes, the
// rest of the block, starts on the next line. So does the inline `else`
// after an `if … ...`, a `(x) -> ...` argument, and an `iter fn`'s `state`
// block.
struct Countdown { from: Int }

fn run(o: Int?, xs: List<Int>) [Console] -> None {
    //~ error[stmt-line:same-line]
    if o is None ... else { return } println("then")
    //~ error[stmt-line:same-line]
    for x in xs ... println("${x}")
}

fn doubled(xs: List<Int>) -> Mut List<Int> => xs {
    //~ error[stmt-line:same-line]
    return map(xs, (e) -> ...) return e * 2
}

iter fn iter(c: Countdown) -> Emitted Int | Finished {
    //~ error[stmt-line:same-line]
    state CountdownIter { at: Int = c.from } if at <= 0 { return finished() }
    at = at - 1
    return emitted(at + 1)
}

//--- a_semicolon_is_refused_once
// Salvo has no `;`. One error for it, naming the fix; the statement after it
// is read as on a line of its own, so it is not refused a second time.
fn run() [Console] {
    //~ error[stmt-line:semicolon]
    let a = 1; let b = 2
    println("${a} ${b}")
}

//--- two_declarations_on_one_line_are_refused
// Top-level declarations end at their line's end too.
//~ error[stmt-line:same-line]
struct A { x: Int } struct B { y: Int }

//~ error[stmt-line:same-line]
fn one() -> Int { return 1 } fn two() -> Int { return 2 }

namespace N {
    //~ error[stmt-line:same-line]
    fn three() -> Int { return 3 } fn four() -> Int { return 4 }
}

fn found(b: B) -> Int {
    return b.y + two() + N.four()
}

//--- what_shares_a_line_legitimately
// What the grammar puts on one line stays accepted: a one-line block holding
// one statement, a construct's continuation after its `}` (`else`, `elif`),
// `when` arms and their one-statement bodies, a lambda's one-expression
// body, a struct's field list and literal, `?:`, a one-line `waitfor`, a
// trailing `replyto`, a one-line `state` block, and comptime brackets.
struct P { x: Int, y: Int }

type U = Int | Str | List<Int>

fn one() -> Int { return 1 }

fn pick(c: Bool) -> Int {
    if c { return 1 } else { return 2 }
}

fn chain(n: Int) -> Int {
    if n == 0 { return 0 } elif n == 1 { return 1 } else { return 2 }
}

fn arms(v: Int | Str) -> Int {
    when v { is Int { return 1 } is Str { return 0 } }
}

fn guard(n: Int) -> Int {
    if n < 0 { return 0 }
    return n
}

fn elvis(o: Int?) -> Int {
    return o ?: 0
}

fn lambda() -> Int {
    let add = (a: Int, b: Int) -> Int { return a + b }
    let inc = (x: Int) -> x + 1
    return inc(add(1, 2))
}

fn literal() -> Int {
    let p = P { x: 1, y: 2 }
    return p.x + p.y
}

actor effect Counter {
    send fn total(out: Reply<Int>) => !out
}

handler Counting() for Counter {
    mailbox { capacity: 4 }

    send fn total(out: Reply<Int>) { out.send(1) }
}

fn waiting() [spawn] -> Int {
    let c = spawn Counting() on pool(1)
    return waitfor out: Reply<Int> { c.total(out) }
}

send fn report(n: Int) {}

fn answered() [use, spawn] {
    let c = spawn Counting() on pool(1)
    c.total() replyto report()
}

struct Countdown { from: Int }

iter fn iter(c: Countdown) -> Emitted Int | Finished {
    state CountdownIter { at: Int = c.from }
    if at <= 0 { return finished() }
    at = at - 1
    return emitted(at + 1)
}

comptime fn arms_covered(u: U) [] -> Int => u {
    [for a in U.arms] { [when a.type] { is Int {} is Str {} is List<*> {} } }
    return 0
}
