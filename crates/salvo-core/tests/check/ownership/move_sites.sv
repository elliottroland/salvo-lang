// The move events beyond a call's `!` parameter (L2), and the order of a
// call's arguments (L3).

//--- l2_move_sites_consume_roots
// [deduce-consume] L2: the remaining move events consume a bare root
// identifier exactly like a call-site move — storing it in a
// struct/array/tuple literal, spreading it (`...n`), `break n` (the code
// after the loop sees it moved, even when the `break` sits inside a
// branch), and the remedies that keep each of them clean
// the back edge), and passing it to a `use` handler constructor. The
// use-site diagnostic names the consuming event. Derived variables
// cannot be moved by any of these [fate-derived-readonly].
//
// [flow-use-after-move] The use is the IR flow pass's to report, and it
// names the line of the move.
struct Box {
    item: Str
}

effect Greeter {
    fn greet() -> Str
}

handler FixedGreeter(text: Str) of Greeter {
    fn greet() -> Str {
        return "hi"
    }
}

fn read(s: Str) -> None => s {
}

// Literal stores (tuple, struct) each consume `s`.
fn tuple_store() {
    let s = "x"
    let _t = (s, 1)
    //~ error[flow-use-after-move:consumed]
    read(s)
}

// [col-no-literal] A list is built by a constructor call now, and
// `List` consumes its elements like any `!` parameter.
fn list_store() {
    let s = "x"
    let _a = List(s)
    //~ error[flow-use-after-move:consumed]
    read(s)
}

fn struct_store() {
    let s = "x"
    let _b = Box {item: s}
    //~ error[flow-use-after-move:consumed]
    read(s)
}

// Spread consumes its base.
fn spread_store() {
    let b = Box {item: "x"}
    let _c = Box {...b}
    //~ error[flow-use-after-move:consumed]
    read(b.item)
}

// `break s` inside an always-exiting branch still reaches the code
// after the loop (the loop exit merges break-path states).
fn break_in_branch() {
    let s = "x"
    let _r = while true {
        if true {
            break s
        }
    }
    //~ error[flow-use-after-move:consumed]
    read(s)
}

// Handler-constructor arguments are stored in the handler.
fn use_ctor() [use] {
    let s = "hi"
    use FixedGreeter(s)
    //~ error[flow-use-after-move:consumed]
    read(s)
}

//--- l2_move_sites_of_a_derived_variable_consume_its_root
// [fate-move-mode] S2: storing/spreading a derived variable of owned
// roots is legal — the binding takes ownership, so the *root* is
// consumed at the binding and its later use names the binding. The IR
// flow pass's diagnostic since R8 piece 3c.
struct Box {
    item: Str
}

fn read(s: Str) -> None => s {
}

fn store_derived() {
    let xs = List(1, 2)
    let ys = xs
    let _t = (ys, 1)
    //~ error[fate-move-mode:bound-away]
    read_list(xs)
}

fn read_list(v: List<Int>) -> None => v {
}

fn spread_derived() {
    let b = Box {item: "x"}
    let d = b
    let _c = Box {...d}
    //~ error[fate-move-mode:bound-away]
    read(b.item)
}

//--- l2_move_sites_remedies_stay_clean
// [deduce-consume] L2 positive matrix: `copy` at each new move site keeps
// the source usable [copy-fn]; reassignment revives (also within a loop
// body ahead of the back edge); a `break value` consumes only its own
// operand; and a `return` inside a branch poisons derived variables only
// on the exiting path — the fall-through path never saw the move.
struct Box {
    item: Str
}

effect Greeter {
    fn greet() -> Str
}

handler FixedGreeter(text: Str) of Greeter {
    fn greet() -> Str {
        return "hi"
    }
}

fn read(s: Str) -> None => s {
}

fn copy_remedies() [use] {
    let s = "x"
    let _t = (copy(s), 1)
    let _a = List(copy(s))
    let b = Box {item: copy(s)}
    let _c = Box {...copy(b)}
    use FixedGreeter(copy(s))
    read(s)
    read(b.item)
}

fn revive() {
    let s = "x"
    let _t = (s, 1)
    s = "y"
    read(s)
}

fn break_consumes_only_its_operand() {
    let s = "x"
    let n = 0
    let _r = while n < 3 {
        if n == 2 {
            break n
        }
        n++
    }
    read(s)
}

fn return_poisons_only_exiting_path(flag: Bool) -> Str {
    let s = "x"
    let d = s
    if flag {
        return s
    }
    return copy(d)
}

fn main() {
    revive()
}

//--- l3_same_call_ordering
// [deduce-same-call] L3: arguments are evaluated left to right, so a
// value moved by an earlier argument of the same call cannot be
// mentioned by a later one (the IR flow pass's diagnostic since R8 piece 2) — double moves (`f(a, a)`), move-then-read
// (`f(a, size(a))`), and interpolated mentions are all rejected at the
// later argument. `copy` at the consuming argument is the remedy;
// kept-position reads are unaffected.
//
// [flow-use-after-move] The IR reads a call's arguments in order, so
// the flow pass reports the later argument, naming the call's line.
fn eat_two(a: List<Int>, b: List<Int>) -> None => !a, !b {
}

fn consume_first(a: List<Int>, n: Int) -> None => !a {
}

fn keep_first(a: List<Int>, n: Int) -> None => a {
}

fn double_move() {
    let xs = List(1, 2)
    //~ error[flow-use-after-move:consumed]
    eat_two(xs, xs)
}

fn move_then_read() {
    let xs = List(1, 2)
    //~ error[flow-use-after-move:consumed]
    consume_first(xs, size(xs))
}

// remedy and kept_then_read are clean.
fn remedy() {
    let xs = List(1, 2)
    eat_two(copy(xs), xs)
}

fn kept_then_read() {
    let ys = List(3)
    keep_first(ys, size(ys))
}
