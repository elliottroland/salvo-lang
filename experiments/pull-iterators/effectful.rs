// I4 prototype, Rust: effectful.sv as the emitters would produce it once a
// producer's handlers thread into its machine per resume.
//
// The three shapes being fixed here:
//
//  1. **A trait per effect set.** `Console Iter<T>`'s pass cannot be a
//     `std::iter::Iterator` — its `advance` takes a handler — so the emitter
//     generates one trait per effect set a program needs, the way it already
//     generates one `UnionN` per arity. The pure case keeps `Box<dyn
//     Iterator>` and is untouched.
//  2. **The factory stays a factory.** `Console Iter<T>` (no `Once`) is
//     replayable, so it is an `Rc<dyn Fn() -> Box<dyn SalvoPassConsole<T>>>`
//     exactly as the pure `SalvoIter<T>` is a factory of boxed iterators.
//     Note the boxing costs nothing new: the pure path already boxes.
//  3. **Variance needs an adapter.** A pure producer used where a claiming one
//     is expected has a *different representation*, so the emitter inserts a
//     wrapper whose `advance` ignores the handler. This is the finding that
//     would have derailed the emitter work if it had surfaced late.
//
// Plus the injected `close`, observable at last: the `for` that breaks early
// closes the pass, and the producer's deferred block prints.

use std::rc::Rc;

// ------------------------------------------------------- the effect (as today)

pub trait Console {
    fn println(&mut self, line: &String);
}

pub struct StdOutConsole;

impl Console for StdOutConsole {
    fn println(&mut self, line: &String) {
        println!("{line}");
    }
}

// ------------------------------------------- runtime: the pure pass (as today)

/// `Iter<T>`: a factory of passes. Unchanged from the current runtime.
pub struct SalvoIter<T>(Rc<dyn Fn() -> Box<dyn Iterator<Item = T>>>);

impl<T: Clone + 'static> SalvoIter<T> {
    pub fn from_factory(factory: Rc<dyn Fn() -> Box<dyn Iterator<Item = T>>>) -> Self {
        SalvoIter(factory)
    }
}

impl<T> Clone for SalvoIter<T> {
    fn clone(&self) -> Self {
        SalvoIter(self.0.clone())
    }
}

// ------------------------------- runtime: generated per effect set, here {Console}

/// One pass whose driving performs `Console`. Generated for each effect set a
/// program's producer types mention.
pub trait SalvoPassConsole<T> {
    fn advance(&mut self, console: &mut dyn Console) -> Option<T>;
    /// The release path: pending deferred blocks, latest first. Idempotent.
    fn close(&mut self, console: &mut dyn Console);
}

/// `Console Iter<T>`: a factory of those.
pub struct SalvoIterConsole<T>(Rc<dyn Fn() -> Box<dyn SalvoPassConsole<T>>>);

impl<T: 'static> SalvoIterConsole<T> {
    pub fn from_factory(factory: Rc<dyn Fn() -> Box<dyn SalvoPassConsole<T>>>) -> Self {
        SalvoIterConsole(factory)
    }

    pub fn mint(&self) -> Box<dyn SalvoPassConsole<T>> {
        (self.0)()
    }

    /// [iter-effects] The variance adapter: a producer performing *fewer*
    /// effects fits where more are expected, and on this backend that is a
    /// representation change, so the compiler inserts this at the boundary.
    pub fn from_pure(pure: SalvoIter<T>) -> Self
    where
        T: Clone,
    {
        SalvoIterConsole(Rc::new(move || Box::new(PureAsConsole((pure.0)()))))
    }
}

impl<T> Clone for SalvoIterConsole<T> {
    fn clone(&self) -> Self {
        SalvoIterConsole(self.0.clone())
    }
}

struct PureAsConsole<T>(Box<dyn Iterator<Item = T>>);

impl<T> SalvoPassConsole<T> for PureAsConsole<T> {
    fn advance(&mut self, _console: &mut dyn Console) -> Option<T> {
        self.0.next()
    }
    fn close(&mut self, _console: &mut dyn Console) {}
}

// ------------------------------------------------------ chatty, as a pass

struct PassChatty {
    /// Parameter, captured at creation and cloned per pass.
    limit: i32,
    /// Body local, hoisted.
    i: i32,
    state: u32,
    /// `defer { println("close") }`
    d0: bool,
}

impl PassChatty {
    fn new(limit: i32) -> Self {
        PassChatty {
            limit,
            i: 0,
            state: 0,
            d0: false,
        }
    }

    fn run_d0(&mut self, console: &mut dyn Console) {
        if self.d0 {
            self.d0 = false;
            console.println(&"close".to_string());
        }
    }
}

impl SalvoPassConsole<i32> for PassChatty {
    fn advance(&mut self, console: &mut dyn Console) -> Option<i32> {
        loop {
            match self.state {
                0 => {
                    console.println(&"open".to_string());
                    self.d0 = true;
                    self.i = 0;
                    self.state = 1;
                }
                // `while i < limit {`
                1 => {
                    if !(self.i < self.limit) {
                        self.state = 3;
                        continue;
                    }
                    console.println(&format!("make {}", self.i));
                    let v = self.i;
                    self.state = 2;
                    return Some(v);
                }
                // after the yield: the tail of the loop body
                2 => {
                    self.i = self.i + 1;
                    self.state = 1;
                }
                // the end of the fn block: its `defer` runs
                3 => {
                    self.run_d0(console);
                    self.state = 4;
                    return None;
                }
                _ => return None,
            }
        }
    }

    fn close(&mut self, console: &mut dyn Console) {
        self.run_d0(console);
        self.state = 4;
    }
}

fn chatty(limit: i32) -> SalvoIterConsole<i32> {
    let c_limit = limit;
    SalvoIterConsole::from_factory(Rc::new(move || Box::new(PassChatty::new(c_limit))))
}

// ------------------------------------------------------- plain, as today

struct PassPlain {
    limit: i32,
    i: i32,
    state: u32,
}

impl PassPlain {
    fn new(limit: i32) -> Self {
        PassPlain {
            limit,
            i: 0,
            state: 0,
        }
    }
}

impl Iterator for PassPlain {
    type Item = i32;
    fn next(&mut self) -> Option<i32> {
        loop {
            match self.state {
                0 => {
                    self.i = 0;
                    self.state = 1;
                }
                1 => {
                    if !(self.i < self.limit) {
                        self.state = 2;
                        continue;
                    }
                    let v = self.i;
                    self.state = 3;
                    return Some(v);
                }
                3 => {
                    self.i = self.i + 1;
                    self.state = 1;
                }
                _ => return None,
            }
        }
    }
}

fn plain(limit: i32) -> SalvoIter<i32> {
    let c_limit = limit;
    SalvoIter::from_factory(Rc::new(move || Box::new(PassPlain::new(c_limit))))
}

// --------------------------------------------------------------- total

/// Inherits `Console` from its parameter, so the handler is a leading
/// parameter exactly as a declared effect's would be [rs-effects].
fn total(xs: &SalvoIterConsole<i32>, console: &mut dyn Console) -> i32 {
    let mut sum = 0;
    let mut pass = xs.mint();
    while let Some(v) = pass.advance(console) {
        sum = sum + v;
    }
    // The injected close: one call after the loop covers exhaustion and every
    // early exit alike, because the flags make it idempotent.
    pass.close(console);
    sum
}

// ---------------------------------------------------------------- main

fn main() {
    let mut console_value = StdOutConsole;
    let console: &mut dyn Console = &mut console_value;

    let mut pass = chatty(3).mint();
    while let Some(v) = pass.advance(console) {
        console.println(&format!("got {v}"));
        if v == 1 {
            break;
        }
    }
    pass.close(console);

    let sum = total(&chatty(2), console);
    console.println(&format!("sum {sum}"));

    // [iter-effects] The variance boundary: `plain` is pure, the position
    // claims `Console`, so the adapter goes in here and nowhere else.
    let widened = SalvoIterConsole::from_pure(plain(4));
    let plain_sum = total(&widened, console);
    console.println(&format!("plain {plain_sum}"));
}
