// R0 prototype, Rust: `passes.sv` as the emitters would produce it once
// `Iter<T>` is gone and `next` is the only protocol.
//
// Note what is *absent*, compared with the code the compiler emits today:
// no `SalvoIter`, no `Box<dyn SalvoPass<T>>`, no `Rc<dyn Fn() -> …>` factory,
// no per-effect-set pass trait, no variance adapter. A pass is a struct; a
// `for` calls `next` on a mutable place; getting a fresh pass is calling the
// function that constructs one.
//
// What is *new*: `SalvoYield<T>` — the obligation group rendered as a trait,
// needed by exactly one thing, a composed pass generic in its source
// (`MapA`). It is a **bound**, never a value type: no `dyn SalvoYield`
// appears anywhere in this file.
//
//   rustc --edition 2021 passes.rs -o ../../tmp/next/passes_rs && ../../tmp/next/passes_rs
//
// Hand-written, so it is narrower than emitter output on purpose: the real
// header is `#![allow(non_snake_case, unused_mut, …)]` and the real `Union2`
// carries `u1()`/`u2()` accessors and a `Display` impl. Naming and scaffolding
// only — the shapes are the ones the emitter would have to produce.

// `non_snake_case` is in the emitter's own allow header: `next__2` is how an
// overload is disambiguated on this backend. `dead_code` covers the parts of
// the generated scaffolding this one program does not reach.
#![allow(dead_code, non_snake_case)]

use std::rc::Rc;

// =====================================================================
// generated: unions.rs
// =====================================================================

#[derive(Clone, Debug)]
pub enum Union2<T1, T2> {
    U1(T1),
    U2(T2),
}

// =====================================================================
// std/core/console.sv (unchanged)
// =====================================================================

pub trait Console {
    fn print(&mut self, message: &String);
}

pub struct StdOutConsole {}

impl StdOutConsole {
    pub fn new() -> Self {
        Self {}
    }
}

impl Console for StdOutConsole {
    fn print(&mut self, message: &String) {
        print!("{}", message)
    }
}

pub fn println(console: &mut dyn Console, message: &String) {
    console.print(message);
    console.print(&("\n".to_string()));
}

// =====================================================================
// std/core/iterator.sv, as the reduction leaves it
// =====================================================================

#[derive(Clone, Debug)]
pub struct Finished {}

pub fn emitted<T>(value: T) -> T {
    return value;
}

pub fn finished() -> Finished {
    return Finished {};
}

/// [yield-group] `params Yield<T>` with `Self` bound to the declaring type.
///
/// Rendered as a trait because a generic composed pass has to call `next` on
/// a type it does not know — with no bound, nothing about a `P` is knowable,
/// and a free `next__N` cannot be selected inside a generic body. It is used
/// only in bound position (`P: SalvoYield<T>`); [group-not-a-value] is what
/// keeps `dyn SalvoYield<T>` from ever appearing.
pub trait SalvoYield<T> {
    fn __next(&mut self) -> Union2<T, Finished>;
}

// =====================================================================
// 1. A hand-written pass
// =====================================================================

#[derive(Clone, Debug)]
pub struct Countdown {
    pub at: i32,
}

pub fn next(c: &mut Countdown) -> Union2<i32, Finished> {
    if c.at <= 0 {
        return Union2::<i32, Finished>::U2(finished());
    }
    let v = c.at;
    c.at = c.at - 1;
    return Union2::<i32, Finished>::U1(emitted(v));
}

/// Generated from `struct Countdown : Yield<Int>`: the declaration is the
/// tie, so the impl is mechanical and forwards to the free function that
/// ordinary call sites already use.
impl SalvoYield<i32> for Countdown {
    fn __next(&mut self) -> Union2<i32, Finished> {
        next(self)
    }
}

pub fn countdown(from: i32) -> Countdown {
    return Countdown { at: from };
}

// =====================================================================
// 2. A `yield`-generated pass, with a `defer` and an effect
// =====================================================================

/// Generated for `fn chatty(limit: Int) [Console] -> Chatty : Yield<Int>`.
/// The parameters and the body's locals are fields; `__d0` is the `defer`
/// site's flag. This is `generator.rs`'s plan, unchanged — what moved is
/// that the struct is the *value* the function returns, rather than
/// something a factory mints.
pub struct Chatty {
    limit: i32,
    i: i32,
    __state: u32,
    __d0: bool,
}

impl Chatty {
    fn __run_d0(&mut self, console: &mut dyn Console) {
        if self.__d0 {
            self.__d0 = false;
            println(console, &("close".to_string()));
        }
    }
}

pub fn chatty(limit: i32) -> Chatty {
    return Chatty {
        limit,
        i: 0,
        __state: 0,
        __d0: false,
    };
}

/// The plan's `__advance`, now spelled as the group member. Handlers are
/// parameters, per resume — so nothing is captured and there is no `'static`
/// bound anywhere.
pub fn next__2(c: &mut Chatty, console: &mut dyn Console) -> Union2<i32, Finished> {
    loop {
        match c.__state {
            0 => {
                println(console, &("open".to_string()));
                c.__d0 = true;
                c.i = 0;
                c.__state = 1;
            }
            // `while i < limit {`
            1 => {
                if !(c.i < c.limit) {
                    c.__state = 3;
                    continue;
                }
                println(console, &(format!("make {}", c.i)));
                let v = c.i;
                // The yield sits at the end of the loop body, so its resume
                // point *is* the loop head: state 2 is the tail after it.
                c.__state = 2;
                return Union2::<i32, Finished>::U1(emitted(v));
            }
            2 => {
                c.i = c.i + 1;
                c.__state = 1;
            }
            // the end of the fn block: the release path runs
            3 => {
                c.__run_d0(console);
                c.__state = 4;
                return Union2::<i32, Finished>::U2(finished());
            }
            _ => return Union2::<i32, Finished>::U2(finished()),
        }
    }
}

/// The release path, as the `Linear` group's member would be spelled:
/// **consuming**. That is a simplification over the injected `close` — it
/// cannot be called twice, so nothing has to be idempotent at the call site.
/// The flag guard inside is still needed, because the body's own exhaustion
/// path may already have discharged the `defer`.
pub fn close(mut c: Chatty, console: &mut dyn Console) {
    c.__run_d0(console);
}

// =====================================================================
// 3. A composed pass — rendering (A), the group as a generic bound
// =====================================================================

/// Generated for `fn map_a<P: Yield<T>, T, U>(…) -> MapA<P, T, U> : Yield<U>`.
/// The source pass is a field, because it must survive the outer body's
/// suspensions; the `for`'s element binding is a field too (`__x`), as a slot
/// because `T` has no zero value.
pub struct MapA<P, T, U> {
    src: P,
    f: Rc<dyn Fn(&T) -> U>,
    __x: Option<T>,
    __state: u32,
}

pub fn map_a<P: 'static, T: 'static, U: 'static>(
    src: P,
    f: impl Fn(&T) -> U + 'static,
) -> MapA<P, T, U> {
    return MapA {
        src,
        f: Rc::new(f),
        __x: None,
        __state: 0,
    };
}

/// The bound is what justifies `m.src.__next()`. Static dispatch, one
/// monomorphization per source type, no erasure.
pub fn next__3<P: SalvoYield<T>, T, U>(m: &mut MapA<P, T, U>) -> Union2<U, Finished> {
    loop {
        match m.__state {
            // `for x in src {`
            0 => match m.src.__next() {
                Union2::U1(x) => {
                    m.__x = Some(x);
                    let f = m.f.clone();
                    let v = f(m.__x.as_ref().unwrap());
                    // yield at the end of the loop body: back to the head
                    m.__state = 0;
                    return Union2::<U, Finished>::U1(emitted(v));
                }
                Union2::U2(_) => {
                    m.__state = 1;
                }
            },
            1 => {
                m.__state = 2;
                return Union2::<U, Finished>::U2(finished());
            }
            _ => return Union2::<U, Finished>::U2(finished()),
        }
    }
}

impl<P: SalvoYield<T>, T, U> SalvoYield<U> for MapA<P, T, U> {
    fn __next(&mut self) -> Union2<U, Finished> {
        next__3(self)
    }
}

// =====================================================================
// 4. A composed pass — rendering (B), the member as an implicit parameter
// =====================================================================

/// Generated for the `?next`/`?close` form. The difference from `MapA` is
/// entirely in how the source's protocol arrives: as stored closures rather
/// than as a bound — which is exactly how the *existing* generated
/// `map_lazy` already carries its `?Iterable` member
/// (`iter: Rc<dyn Fn(It) -> SalvoIter<T>>`), so this rendering needs no new
/// machinery on either backend.
///
/// Two things it buys, and (A) cannot:
///   * the stored `next`'s type carries the source's **effects**, so an
///     effectful source composes;
///   * `close` is **optional at the call site** rather than a bound the
///     source type has to satisfy.
///
/// The source sits in a slot because closing it *consumes* it, and the
/// release path only ever holds `&mut self`.
pub struct MapB<P, T, U> {
    src: Option<P>,
    f: Rc<dyn Fn(&T) -> U>,
    next_src: Rc<dyn Fn(&mut P, &mut dyn Console) -> Union2<T, Finished>>,
    close_src: Rc<dyn Fn(P, &mut dyn Console)>,
    __x: Option<T>,
    __state: u32,
}

pub fn map_b<P: 'static, T: 'static, U: 'static>(
    src: P,
    f: impl Fn(&T) -> U + 'static,
    next_src: Rc<dyn Fn(&mut P, &mut dyn Console) -> Union2<T, Finished>>,
    close_src: Rc<dyn Fn(P, &mut dyn Console)>,
) -> MapB<P, T, U> {
    return MapB {
        src: Some(src),
        f: Rc::new(f),
        next_src,
        close_src,
        __x: None,
        __state: 0,
    };
}

impl<P, T, U> MapB<P, T, U> {
    /// The plan's `ClosePass` step: close the nested pass, once.
    fn __close_src(&mut self, console: &mut dyn Console) {
        if let Some(p) = self.src.take() {
            let cf = self.close_src.clone();
            cf(p, console);
        }
    }
}

pub fn next__4<P, T, U>(m: &mut MapB<P, T, U>, console: &mut dyn Console) -> Union2<U, Finished> {
    loop {
        match m.__state {
            0 => {
                let nf = m.next_src.clone();
                let step = nf(m.src.as_mut().unwrap(), console);
                match step {
                    Union2::U1(x) => {
                        m.__x = Some(x);
                        let f = m.f.clone();
                        let v = f(m.__x.as_ref().unwrap());
                        m.__state = 0;
                        return Union2::<U, Finished>::U1(emitted(v));
                    }
                    Union2::U2(_) => {
                        m.__state = 1;
                    }
                }
            }
            1 => {
                m.__close_src(console);
                m.__state = 2;
                return Union2::<U, Finished>::U2(finished());
            }
            _ => return Union2::<U, Finished>::U2(finished()),
        }
    }
}

pub fn close__2<P, T, U>(mut m: MapB<P, T, U>, console: &mut dyn Console) {
    m.__close_src(console);
}

// =====================================================================
// 5 & 6. A linear pass
// =====================================================================

#[derive(Clone, Debug)]
pub struct Lines {
    pub name: String,
    pub count: i32,
    pub at: i32,
}

pub fn next__5(l: &mut Lines) -> Union2<String, Finished> {
    if l.at >= l.count {
        return Union2::<String, Finished>::U2(finished());
    }
    l.at = l.at + 1;
    let row: String = format!("line {} of {}", l.at, l.name);
    return Union2::<String, Finished>::U1(emitted(row));
}

impl SalvoYield<String> for Lines {
    fn __next(&mut self) -> Union2<String, Finished> {
        next__5(self)
    }
}

/// `: Linear`'s member. It consumes, so on this backend it simply takes the
/// value — the ownership contract needs no deduction machinery to express.
pub fn close__3(l: Lines, console: &mut dyn Console) {
    println(console, &(format!("closing {}", l.name)));
}

pub fn open_lines(name: &String, count: i32, console: &mut dyn Console) -> Lines {
    println(console, &(format!("opening {}", name)));
    return Lines {
        name: name.clone(),
        count,
        at: 0,
    };
}

// =====================================================================
// The program
// =====================================================================

pub fn main() {
    let mut console = StdOutConsole::new();

    // --- 1: a pass beside a List ---
    let mut __loop1_pass = countdown(3);
    while let Union2::U1(n) = next(&mut __loop1_pass) {
        println(&mut console, &(format!("n {}", n)));
    }
    let names = vec!["ada".to_string(), "grace".to_string()];
    for s in &names {
        println(&mut console, &(format!("s {}", s.clone())));
    }

    // --- 2: a generated pass, abandoned after two elements ---
    let mut seen = 0;
    let mut __loop3_pass = chatty(3);
    while let Union2::U1(v) = next__2(&mut __loop3_pass, &mut console) {
        println(&mut console, &(format!("got {}", v)));
        seen = seen + 1;
        if seen == 2 {
            break;
        }
    }
    close(__loop3_pass, &mut console);

    // --- 3: composition by bound, over a pure source ---
    let mut __loop4_pass = map_a(countdown(3), |n: &i32| n * 2);
    while let Union2::U1(v) = next__3(&mut __loop4_pass) {
        println(&mut console, &(format!("A {}", v)));
    }

    // --- 4: composition by implicit, over an *effectful* source ---
    let mut __loop5_pass = map_b(
        chatty(3),
        |n: &i32| n * 2,
        Rc::new(|p: &mut Chatty, c: &mut dyn Console| next__2(p, c)),
        Rc::new(|p: Chatty, c: &mut dyn Console| close(p, c)),
    );
    while let Union2::U1(v) = next__4(&mut __loop5_pass, &mut console) {
        println(&mut console, &(format!("B {}", v)));
    }
    close__2(__loop5_pass, &mut console);

    // --- 5: a linear pass, drained ---
    let mut __loop6_pass = open_lines(&("a.txt".to_string()), 2, &mut console);
    while let Union2::U1(row) = next__5(&mut __loop6_pass) {
        println(&mut console, &row);
    }
    close__3(__loop6_pass, &mut console);

    // --- 6: a linear pass, abandoned ---
    let mut __loop7_pass = open_lines(&("b.txt".to_string()), 2, &mut console);
    while let Union2::U1(row) = next__5(&mut __loop7_pass) {
        println(&mut console, &row);
        break;
    }
    close__3(__loop7_pass, &mut console);

    println(&mut console, &("done".to_string()));
}
