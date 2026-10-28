// what crosses the platform boundary
//
// Compile-and-run data ([test-run-data]), moved from the CLI's run and
// platform tests. Each program's platform files are the skeletons
// `salvo platform generate` writes, their stubs implemented; that the
// skeletons say what they do stays with the CLI's platform tests.

// [platform-fn] [platform-handler] Implementation files: `platform fn`s (a
// field read, a struct built from host values, a union answered through its
// arm, a list of structs) and a platform handler with constructor
// parameters whose class keeps its own state. The program calls each fn's
// wrapper (`shoutPlatform` / `shout_platform`), which calls the
// implementation by its real name.
//--- platform_fns_and_handlers
// FILE: salvo.toml
[project]
name = "case"

[build]
platform = "platform"
// FILE: platform/main.rs
use crate::*;
use crate::unions::*;

pub fn shout(g: &Greeting) -> String {
    if g.loud { format!("{}!", g.text.to_uppercase()) } else { g.text.clone() }
}

pub fn make(text: &String) -> Greeting {
    Greeting { text: text.clone(), loud: true }
}

pub fn parse(s: &String) -> Union2<i32, String> {
    match s.parse::<i32>() {
        Ok(n) => Union2::U1(n),
        Err(_) => Union2::U2(format!("not a number: {s}")),
    }
}

pub fn describe_attrs(attrs: &Vec<Attr>) -> String {
    attrs
        .iter()
        .map(|a| format!("{}={}", a.data_type, a.string_value.clone().unwrap_or("-".to_string())))
        .collect::<Vec<_>>()
        .join(", ")
}

pub struct HostCounter {
    at: i32,
}

impl HostCounter {
    pub fn new(start: i32) -> Self {
        Self { at: start }
    }
}

impl crate::CounterPlatform for HostCounter {
    fn next(&mut self, step: i32) -> i32 {
        self.at += step;
        self.at
    }
}
// FILE: platform/main.kt
package salvo.platform.main

import salvo.*
import salvo.main.*

fun shout(g: Greeting): String = if (g.loud) g.text.uppercase() + "!" else g.text

fun make(text: String): Greeting = Greeting(text = text, loud = true)

fun parse(s: String): Union2<Int, String> {
    val n = s.toIntOrNull() ?: return Union2.U2("not a number: $s")
    return Union2.U1(n)
}

fun describeAttrs(attrs: List<Attr>): String =
    attrs.joinToString(", ") { "${it.dataType}=${it.stringValue ?: "-"}" }

class HostCounter(start: Int) : CounterPlatform {
    private var at = start
    override fun next(step: Int): Int {
        at += step
        return at
    }
}
// FILE: main.sv
struct Greeting { text: Str, loud: Bool }
struct Attr { data_type: Str, string_value: Str? = None }

platform fn shout(g: Greeting) [] -> Str => g
platform fn make(text: Str) [] -> Greeting => text
platform fn parse(s: Str) [] -> Ok Int | Err Str => s
platform fn describe_attrs(attrs: List<Attr>) [] -> Str => attrs

effect Counter {
    fn next(step: Int) -> Int => step
}

platform handler HostCounter(start: Int) of Counter

fn describe(r: Ok Int | Err Str) [] -> Str => r {
    when r {
        is Ok { return "ok ${r}" }
        is Err { return "err ${r}" }
    }
}

fn main() [use] {
    use StdConsole()
    use HostCounter(10)
    println(shout(make("hello")))
    println(shout(Greeting { text: "quiet", loud: false }))
    println(describe(parse("42")))
    println(describe(parse("x")))
    println("${next(5)} ${next(1)}")
    println(describe_attrs(List(Attr { data_type: "String", string_value: "hi" }, Attr { data_type: "Number" })))
}
//= stdout
//= HELLO!
//= quiet
//= ok 42
//= err not a number: x
//= 15 16
//= String=hi, Number=-

// [platform-never] A platform fn and a platform handler member answering
// `Never` are `-> !` / `: Nothing` on the host side, so a host body that
// returns is the host compiler's error, not a silent fall-through; a host
// that exits is the program's end.
//--- a_platform_member_answering_never_ends_the_program
// FILE: salvo.toml
[project]
name = "case"

[build]
platform = "platform"
// FILE: platform/main.rs
use crate::*;

pub fn bail(why: &String) -> ! { eprintln!("{why}"); std::process::exit(3) }

pub struct HostStop;
impl HostStop { pub fn new() -> Self { HostStop } }
impl crate::StopPlatform for HostStop {
    fn halt(&mut self, _why: &String) -> ! { std::process::exit(4) }
}
// FILE: platform/main.kt
package salvo.platform.main

import salvo.main.*

fun bail(why: String): Nothing { System.err.println(why); kotlin.system.exitProcess(3) }

class HostStop : StopPlatform {
    override fun halt(why: String): Nothing = kotlin.system.exitProcess(4)
}
// FILE: main.sv
effect Stop {
    fn halt(why: Str) [] -> Never => why
}

platform handler HostStop() of Stop

platform fn bail(why: Str) [] -> Never => why

fn main() [use] {
    use StdConsole()
    use HostStop()
    println("before")
    if size("x") > 5 {
        halt("never")
    }
    bail("stop")
}
//= exit 3
//= stderr
//= stop
//= stdout
//= before

// [platform-type] Platform types, declared in a module of their own: a copy
// of a plain handle shares the host object, a linear `canbe Mut` one is
// mutated through a `Mut` parameter and consumed by its closer, and a
// handle travels to an actor.
//--- platform_types
// FILE: salvo.toml
[project]
name = "case"

[build]
platform = "platform"
// FILE: platform/host.rs
use crate::*;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct Tally {
    n: Arc<Mutex<i32>>,
}

#[derive(Clone)]
pub struct Shared {}

pub struct Cursor {
    at: i32,
}

pub fn new_tally(start: i32) -> Tally {
    Tally { n: Arc::new(Mutex::new(start)) }
}

pub fn bump(t: &Tally) -> i32 {
    let mut n = t.n.lock().unwrap();
    *n += 1;
    *n
}

pub fn new_shared() -> Shared {
    Shared {}
}

pub fn label(_s: &Shared) -> String {
    "shared".to_string()
}

pub fn open_cursor(n: i32) -> Cursor {
    Cursor { at: n }
}

pub fn step(c: &mut Cursor) -> i32 {
    c.at += 1;
    c.at
}

pub fn close(c: Cursor) {
    let _ = c;
}
// FILE: platform/host.kt
package salvo.platform.host

import salvo.host.*

class Tally(var n: Int)
class Shared
class Cursor(var at: Int)

fun newTally(start: Int): Tally = Tally(start)
fun bump(t: Tally): Int = synchronized(t) { t.n += 1; t.n }
fun newShared(): Shared = Shared()
fun label(s: Shared): String = "shared"
fun openCursor(n: Int): Cursor = Cursor(n)
fun step(c: Cursor): Int { c.at += 1; return c.at }
fun close(c: Cursor) {}
// FILE: host.sv
// A counter living in the host.
export platform type Tally
export threadsafe platform type Shared
export linear platform type Cursor canbe Mut

export platform fn new_tally(start: Int) [] -> Tally => start
export platform fn bump(t: Tally) [] -> Int => t
export platform fn new_shared() [] -> Shared
export platform fn label(s: Shared) [] -> Str => s
export platform fn open_cursor(n: Int) [] -> Mut Cursor => n
export platform fn step(c: Mut Cursor) [] -> Int => c: Mut
export platform fn close(c: Cursor) [] -> None => !c
// FILE: main.sv
import host

actor effect Holder {
    send fn hold(t: Tally, done: Reply<Int>) => !t, !done
}

handler Holding() of Holder {
    mailbox { capacity: 4 }
    send fn hold(t: Tally, done: Reply<Int>) {
        done.send(bump(t))
    }
}

fn main() [use, spawn] {
    use StdConsole()
    let t = new_tally(10)
    let u = copy(t)
    println("${bump(t)} ${bump(u)}")
    println(label(new_shared()))
    let c = open_cursor(3)
    println("${step(c)} ${step(c)}")
    close(c)
    let h = spawn Holding() on pool(1)
    let got = waitfor done: Reply<Int> { h.hold(t, done) }
    println("held ${got}")
}
//= stdout
//= 11 12
//= shared
//= 4 5
//= held 13

// [platform-fn-value] Function values at the boundary: a platform fn taking
// an effect-free callback (a plain one, called twice, and a `once` one,
// consumed), one catching a fault the callback raises, and a platform
// handler member taking a callback — which on Rust is `&mut dyn FnMut`,
// since an effect trait is used as `dyn`.
//--- function_values_cross_the_platform_boundary
// FILE: salvo.toml
[project]
name = "case"

[build]
platform = "platform"
// FILE: platform/host.rs
use crate::host::*;

pub fn call(body: impl FnOnce() -> String) -> String {
    body()
}

pub fn twice(f: &mut impl FnMut(i32) -> i32, x: i32) -> i32 {
    let y = f(x);
    f(y)
}

pub fn run_guarded(body: impl FnOnce()) -> Option<String> {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(body)) {
        Ok(()) => None,
        Err(e) => Some(e.downcast_ref::<String>().cloned().or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_else(|| "fault".into())),
    }
}

pub struct HostApply {
}

impl HostApply {
    pub fn new() -> Self {
        Self { }
    }
}

impl crate::host::ApplyPlatform for HostApply {
    fn apply(&mut self, f: &mut dyn FnMut(i32) -> i32, x: i32) -> i32 {
        f(x)
    }
}
// FILE: platform/host.kt
package salvo.platform.host

import salvo.host.*

fun call(body: () -> String): String = body()
fun twice(f: (Int) -> Int, x: Int): Int = f(f(x))
fun runGuarded(body: () -> Unit): String? = try { body(); null } catch (t: Throwable) { t.message ?: "fault" }

class HostApply : ApplyPlatform {
    override fun apply(f: (Int) -> Int, x: Int): Int {
        return f(x)
    }
}
// FILE: host.sv
export platform fn call(body: once () -> Str) [] -> Str => !body
export platform fn twice(f: (n: Int) -> Int, x: Int) [] -> Int => f, x
export platform fn run_guarded(body: once () -> None) [] -> Str? => !body

export effect Apply {
    fn apply(f: (n: Int) -> Int, x: Int) -> Int => f, x
}

export platform handler HostApply() of Apply
// FILE: main.sv
import host

fn main() [use] {
    use StdConsole()
    let k = 5
    println("${twice(n -> n + k, 1)}")
    let label = "late"
    println(call(() -> "${label} ran"))
    let why = run_guarded(() -> {
        let xs: List<Int> = List<Int>()
        let _x = get(xs, 3) ?: unreachable!()
    })
    println("faulted: ${!(why is None)}")
    println("${run_guarded(() -> {}) is None}")
    use HostApply()
    println("${apply(n -> n * k, 2)}")
}
//= stdout
//= 11
//= late ran
//= faulted: true
//= true
//= 10

// [platform-fn-value] A platform fn that **keeps** a fn value (`=> !hook`
// on a plain fn type) takes a named top-level fn: the host stores it and
// calls it later, from another call. Rust's host holds a `fn` pointer;
// Kotlin's its own fn type.
//--- a_platform_fn_keeps_a_named_fn
// FILE: salvo.toml
[project]
name = "case"

[build]
platform = "platform"
// FILE: platform/host.rs
use std::sync::Mutex;

static HOOK: Mutex<Option<fn(i32, &String) -> String>> = Mutex::new(None);

pub fn keep(hook: fn(i32, &String) -> String) {
    *HOOK.lock().unwrap() = Some(hook);
}

pub fn fire(n: i32) -> String {
    let hook = HOOK.lock().unwrap().expect("kept");
    std::thread::spawn(move || hook(n, &"fired".to_string())).join().unwrap()
}
// FILE: platform/host.kt
package salvo.platform.host

private var hook: ((Int, String) -> String)? = null

fun keep(hook: (Int, String) -> String) {
    salvo.platform.host.hook = hook
}

fun fire(n: Int): String = hook!!(n, "fired")
// FILE: host.sv
export platform fn keep(hook: (n: Int, label: Str) -> Str) [] -> None => !hook
export platform fn fire(n: Int) [] -> Str => n
// FILE: main.sv
import host

fn shout(n: Int, label: Str) [] -> Str => n, label {
    return "${label} ${n * 2}"
}

fn main() [use] {
    use StdConsole()
    keep(shout)
    println(fire(4))
    println(fire(5))
}
//= stdout
//= fired 8
//= fired 10

// [platform-iterable] A customer's `iterable platform type`: a `for` over a
// value loops over the host's `each`, and a generic fn walking it through
// `Iter<self, BagIter, Int>` gets the Salvo pass — the two agreeing on the
// elements.
//--- an_iterable_platform_type_loops_over_its_hosts_each
// FILE: salvo.toml
[project]
name = "case"

[build]
platform = "platform"
// FILE: platform/host.rs
use crate::host::*;

#[derive(Clone)]
pub struct Bag {
    items: Vec<i32>,
}

pub fn bag_of(a: i32, b: i32) -> Bag {
    Bag { items: vec![a, b] }
}

pub fn bag_size(b: &Bag) -> i32 {
    b.items.len() as i32
}

pub fn bag_at(b: &Bag, i: i32) -> i32 {
    b.items[i as usize]
}

pub fn each(x: &Bag) -> impl Iterator<Item = i32> + '_ {
    x.items.iter().copied()
}
// FILE: platform/host.kt
package salvo.platform.host

import salvo.host.*

class Bag(val items: List<Int>)

fun bagOf(a: Int, b: Int): Bag = Bag(listOf(a, b))
fun bagSize(b: Bag): Int = b.items.size
fun bagAt(b: Bag, i: Int): Int = b.items[i]

fun each(x: Bag): Iterable<Int> = x.items
// FILE: host.sv
export iterable platform type Bag : Iter<self, BagIter, Int>
export platform fn bag_of(a: Int, b: Int) [] -> Bag
export platform fn bag_size(b: Bag) [] -> Int => b
export platform fn bag_at(b: Bag, i: Int) [] -> Int => b

export iter fn iter(b: Bag) -> Emitted Int | Finished {
    state BagIter {
        at: Int = 0
    }
    if at >= bag_size(b) {
        return finished()
    }
    at = at + 1
    return emitted(bag_at(b, at - 1))
}
// FILE: main.sv
import host

fn main() [use] {
    use StdConsole()
    let b = bag_of(3, 4)
    for x in b {
        println("each ${x}")
    }
    println("pass ${to_list(iter(b))}")
}
//= stdout
//= each 3
//= each 4
//= pass [3, 4]

// [platform-generic] Generic platform types and fns: the host stores, moves
// and hands back an opaque `T` (Rust bounds it `Send + 'static`, never
// `Clone`), which is enough to write an erased payload (`Dyn` with
// `erase`/`unerase`) — and a linear `T` travels through both.
//--- generic_platform_types_and_fns
// FILE: salvo.toml
[project]
name = "case"

[build]
platform = "platform"
// FILE: platform/host.rs
use crate::*;

pub struct Cell<T: Send + 'static> {
    v: T,
}

pub struct Dyn {
    v: Box<dyn std::any::Any + Send>,
}

pub fn cell_of<T: Send + 'static>(v: T) -> Cell<T> {
    Cell { v }
}

pub fn take<T: Send + 'static>(c: Cell<T>) -> T {
    c.v
}

pub fn erase<T: Send + 'static>(v: T) -> Dyn {
    Dyn { v: Box::new(v) }
}

pub fn unerase<T: Send + 'static>(d: Dyn) -> T {
    *d.v.downcast::<T>().expect("salvo: an erased value read back as another type")
}
// FILE: platform/host.kt
package salvo.platform.host

import salvo.host.*

class Cell<T>(val v: T)
class Dyn(val v: Any?)

fun <T> cellOf(v: T): Cell<T> = Cell(v)
fun <T> take(c: Cell<T>): T = c.v
fun <T> erase(v: T): Dyn = Dyn(v)
@Suppress("UNCHECKED_CAST")
fun <T> unerase(d: Dyn): T = d.v as T
// FILE: host.sv
export linear platform type Cell<T canbe linear>
export linear platform type Dyn

export platform fn cell_of<T canbe linear>(v: T) [] -> Cell<T> => !v
export platform fn take<T canbe linear>(c: Cell<T>) [] -> T => !c
export platform fn erase<T canbe linear>(v: T) [] -> Dyn => !v
export platform fn unerase<T canbe linear>(d: Dyn) [] -> T => !d
// FILE: main.sv
import host

struct Point { x: Int, y: Int }

fn main() [use] {
    use StdConsole()
    let c = cell_of("hi")
    println(take(c))
    let d = erase(Point { x: 1, y: 2 })
    let p: Point = unerase(d)
    println("${p.x} ${p.y}")
    let n: Int = unerase(erase(7))
    println("${n}")
    tickets()
}

linear struct Ticket { id: Int }

fn redeem(t: Ticket) [Console] -> None => !t {
    println("redeemed ${t.id}")
    discard(t)
}

fn tickets() [Console] -> None {
    let back: Ticket = take(cell_of(unerase<Ticket>(erase(Ticket { id: 9 }))))
    redeem(back)
}
//= stdout
//= hi
//= 1 2
//= 7
//= redeemed 9

// [platform-reply] A platform member that takes a `Reply` returns at once
// and completes the reply later from a host thread: `.hosted()` takes the
// token, `send` completes it. Two continuations — a `waitfor` in `main`,
// and a free `send fn` whose own answer `main` waits on.
//--- a_host_thread_completes_a_reply
// FILE: salvo.toml
[project]
name = "case"

[build]
platform = "platform"
// FILE: platform/main.rs
use crate::*;

pub struct HostSlow {
}

impl HostSlow {
    pub fn new() -> Self {
        Self { }
    }
}

impl crate::SlowPlatformSync for HostSlow {
    fn later(&self, n: i32, done: crate::scheduler::SalvoReply) {
        let done = done.hosted();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(50));
            if n >= 0 { done.send(n + 1) } else { drop(done) }
        });
    }
}
// FILE: platform/main.kt
package salvo.platform.main

import salvo.main.*

class HostSlow : SlowPlatform {
    override fun later(n: Int, done: salvo.SalvoReply) {
        val host = done.hosted()
        Thread { Thread.sleep(50); host.send(n + 1) }.start()
    }
}
// FILE: main.sv
effect Slow {
    fn later(n: Int, done: Reply<Int>) [] -> None => !n, !done
}

threadsafe platform handler HostSlow of Slow

send fn labelled(label: Str, out: Reply<Str>, n: Int) => !label, !out, !n {
    out.send("${label} ${n}")
}

fn main() [use] {
    use StdConsole()
    use HostSlow()
    let n = waitfor done: Reply<Int> {
        later(41, done)
    }
    println("waited for ${n}")
    let line = waitfor out: Reply<Str> {
        later(1, replyto labelled("continued with", out))
    }
    println(line)
}
//= stdout
//= waited for 42
//= continued with 2

// [platform-reply] On Rust, a host reply dropped unsent reaches the fault
// sink and the waiter reports its deadlock instead of hanging. Rust only:
// the JVM cannot see a drop, so Kotlin's waiter would wait for ever.
//--- a_host_reply_dropped_unsent_is_reported
// BACKENDS: rust
// FILE: salvo.toml
[project]
name = "case"

[build]
platform = "platform"
// FILE: platform/main.rs
use crate::*;

pub struct HostSlow {
}

impl HostSlow {
    pub fn new() -> Self {
        Self { }
    }
}

impl crate::SlowPlatformSync for HostSlow {
    fn later(&self, n: i32, done: crate::scheduler::SalvoReply) {
        let done = done.hosted();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(50));
            if n >= 0 { done.send(n + 1) } else { drop(done) }
        });
    }
}
// FILE: main.sv
effect Slow {
    fn later(n: Int, done: Reply<Int>) [] -> None => !n, !done
}

threadsafe platform handler HostSlow of Slow

send fn labelled(label: Str, out: Reply<Str>, n: Int) => !label, !out, !n {
    out.send("${label} ${n}")
}

fn main() [use] {
    use StdConsole()
    use HostSlow()
    let n = waitfor done: Reply<Int> {
        later(-1, done)
    }
    println("waited for ${n}")
    let line = waitfor out: Reply<Str> {
        later(1, replyto labelled("continued with", out))
    }
    println(line)
}
//= trap deadlock: nothing can run while main waits
//= stderr
//= salvo: a host reply was dropped without being sent [platform-reply]
//= salvo: deadlock: nothing can run while main waits
