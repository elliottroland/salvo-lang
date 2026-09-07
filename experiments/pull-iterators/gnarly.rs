// I3 prototype, Rust: gnarly.sv as a hand-written state machine — what the
// emitter would produce once `yield` functions lower to a pass instead of an
// `async` block.
//
// Compare with gnarly_oracle.rs, which decides the expected output using
// Rust's own scope guards. This file must print exactly the same thing.
//
// The shape being tested: a *flat* dispatch loop over numbered resume points,
// with the body's locals — and the inner pass — as fields. Note that no
// `Pin`, `Future`, `Waker`, `Box<dyn Iterator>`, `Rc` or `async` appears.

pub enum Step<T> {
    Emitted(T),
    Finished,
}

pub trait Console {
    fn println(&mut self, line: &str);
}

pub struct StdOutConsole;

impl Console for StdOutConsole {
    fn println(&mut self, line: &str) {
        println!("{line}");
    }
}

// ------------------------------------------------ the hand-written pass

pub struct Countdown {
    at: i32,
}

impl Countdown {
    fn next(&mut self) -> Step<i32> {
        if self.at <= 0 {
            return Step::Finished;
        }
        let v = self.at;
        self.at -= 1;
        Step::Emitted(v)
    }
}

fn countdown(from: i32) -> Countdown {
    Countdown { at: from }
}

// ------------------------------------------------------ walk, as a pass
//
// Resume points, in the order the body reaches them:
//   0  entry: the prelude, up to the first suspension
//   1  the outer `while` head
//   2  the inner `for` head — also where `yield r * 10 + col` resumes
//   3  after the inner loop: `yield r * 100`
//   4  where that yield resumes: the tail of the outer loop body
//   5  after the outer loop: `yield 999`
//   6  where *that* resumes: the end of the fn block
//   7  finished
//
// Deferred blocks are a flag each, plus whatever local they read. `d1` is the
// interesting one: it sits *inside* the loop body, so it is registered anew
// every iteration and discharged at the end of that iteration — the flag is
// cleared as it runs, and `r` (which it reads) is a field that the next
// iteration overwrites. That is why one slot suffices and no stack is needed:
// a loop-body `defer` cannot outlive its iteration.

pub struct WalkPass {
    /// Parameter.
    limit: i32,
    /// Body locals, hoisted.
    row: i32,
    r: i32,
    /// The inner pass has to survive the outer body's suspensions, so it is a
    /// field rather than a local. (For a *recursive* producer this field would
    /// have the struct's own type, which is why recursion needs a `Box`.)
    inner: Option<Countdown>,
    state: u32,
    /// `defer { println("close") }` — fn-block level, no captures.
    d0_pending: bool,
    /// `defer { println("row ${r} end") }` — loop-body level, reads `r`.
    d1_pending: bool,
}

impl WalkPass {
    fn new(limit: i32) -> Self {
        WalkPass {
            limit,
            row: 0,
            r: 0,
            inner: None,
            state: 0,
            d0_pending: false,
            d1_pending: false,
        }
    }

    fn next(&mut self, console: &mut dyn Console) -> Step<i32> {
        loop {
            match self.state {
                0 => {
                    console.println("open");
                    self.d0_pending = true;
                    self.row = 0;
                    self.state = 1;
                }
                // `while row < limit {`
                1 => {
                    if self.row >= self.limit {
                        self.state = 5;
                        continue;
                    }
                    self.r = self.row;
                    self.d1_pending = true;
                    self.inner = Some(countdown(2));
                    self.state = 2;
                }
                // `for col in countdown(2) {` — and the resume point of the
                // yield inside it, so `continue` is just staying here.
                2 => {
                    let step = self.inner.as_mut().expect("the inner pass").next();
                    match step {
                        Step::Emitted(col) => {
                            if col == 1 {
                                continue;
                            }
                            let v = self.r * 10 + col;
                            self.state = 2;
                            return Step::Emitted(v);
                        }
                        Step::Finished => {
                            self.inner = None;
                            self.state = 3;
                        }
                    }
                }
                // `yield r * 100`
                3 => {
                    let v = self.r * 100;
                    self.state = 4;
                    return Step::Emitted(v);
                }
                // The tail of the outer loop body: the increment, then the
                // end of the block, which is where its `defer` runs.
                4 => {
                    self.row += 1;
                    self.run_d1(console);
                    self.state = 1;
                }
                // `yield 999`, after the loop.
                5 => {
                    self.state = 6;
                    return Step::Emitted(999);
                }
                // The end of the fn block: its `defer` runs, then the body is
                // over.
                6 => {
                    self.run_d0(console);
                    self.state = 7;
                    return Step::Finished;
                }
                _ => return Step::Finished,
            }
        }
    }

    /// The injected release path: the same deferred code, reached because the
    /// *consumer* stopped rather than because the body did. Latest-registered
    /// first, and idempotent — the flags are what make one call after the loop
    /// cover `break` and exhaustion alike.
    fn close(&mut self, console: &mut dyn Console) {
        self.run_d1(console);
        self.run_d0(console);
        self.state = 7;
    }

    fn run_d1(&mut self, console: &mut dyn Console) {
        if self.d1_pending {
            self.d1_pending = false;
            console.println(&format!("row {} end", self.r));
        }
    }

    fn run_d0(&mut self, console: &mut dyn Console) {
        if self.d0_pending {
            self.d0_pending = false;
            console.println("close");
        }
    }
}

fn walk(limit: i32) -> WalkPass {
    WalkPass::new(limit)
}

// -------------------------------------------------------------------- main

fn main() {
    let mut console = StdOutConsole;
    let console: &mut dyn Console = &mut console;

    let mut seen = 0;
    let mut pass = walk(2);
    while let Step::Emitted(v) = pass.next(console) {
        console.println(&format!("got {v}"));
        seen += 1;
        if seen == 4 {
            break;
        }
    }
    pass.close(console);

    console.println("done");
}
