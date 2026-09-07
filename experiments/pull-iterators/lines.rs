// I1b prototype, Rust: what the emitter would produce for source.sv under the
// pull-iterator design. Hand-written and compiled before teaching it to the
// emitter, per PROGRESS.md's gotcha.
//
// Conventions kept from the current backend: effects are traits, handlers are
// structs implementing them, and an effect is *threaded* as a `&mut dyn E`
// parameter — never captured. That last point is the whole design: the pass
// below holds no handler, so it needs no lifetime and no `'static` bound, and
// an iterator function may therefore perform effects.
//
// Note what is NOT here: no `Pin`, no `Future`, no `Waker`, no
// `Box<dyn Iterator>`, no `Rc`, no `async`. Iteration is a plain struct with
// a `next`.

// ---------------------------------------------------------------- runtime
// Would live in runtime/iter.rs. In the real emission this is the generated
// `Union2<T, Stopped>` for the Salvo union `Next T | Stopped`; spelled as a
// dedicated enum here so the prototype reads.

pub enum SalvoStep<T> {
    Next(T),
    Stopped,
}

// ---------------------------------------------------------------- effects

pub trait Console {
    fn println(&mut self, line: &str);
}

pub trait FileSystem {
    fn open(&mut self, path: &str) -> i32;
    fn read_line(&mut self, handle: i32) -> Option<String>;
    fn close(&mut self, handle: i32);
}

// --------------------------------------------------------------- handlers

pub struct StdOutConsole;

impl Console for StdOutConsole {
    fn println(&mut self, line: &str) {
        println!("{line}");
    }
}

/// Stands in for a platform implementation: four lines, so the consumer's
/// `break` really is early.
pub struct FakeFileSystem {
    lines: Vec<String>,
    at: usize,
}

impl FakeFileSystem {
    pub fn new() -> Self {
        FakeFileSystem {
            lines: vec![
                "alpha".to_string(),
                "beta".to_string(),
                "gamma".to_string(),
                "delta".to_string(),
            ],
            at: 0,
        }
    }
}

impl FileSystem for FakeFileSystem {
    fn open(&mut self, _path: &str) -> i32 {
        self.at = 0;
        7
    }

    fn read_line(&mut self, _handle: i32) -> Option<String> {
        let line = self.lines.get(self.at).cloned();
        if line.is_some() {
            self.at += 1;
        }
        line
    }

    fn close(&mut self, _handle: i32) {}
}

// ------------------------------------------------------- the pass: lines()
//
// `fn lines(path: Str) [FileSystem, Console] -> Once Iter<Str>` becomes a
// struct plus a `next`. The body's locals are fields; each `yield` is a
// resume point; the `defer` is a flag plus an unwind path.

pub struct LinesPass {
    /// Parameter, moved in at construction.
    path: String,
    /// A body local (`let f = open(path)`), live from state 1 onwards.
    f: i32,
    /// Which resume point to enter on the next call.
    /// 0 = entry, 1 = the `while true` head, 2 = leaving, 3 = finished.
    state: u32,
    /// One flag per `defer` site, set when the site is passed.
    defer_close_pending: bool,
}

impl LinesPass {
    fn new(path: String) -> Self {
        LinesPass {
            path,
            f: 0,
            state: 0,
            defer_close_pending: false,
        }
    }

    /// The effects the *declaration* lists arrive per resume, in declaration
    /// order, exactly as they do for any other Salvo function.
    fn next(&mut self, fs: &mut dyn FileSystem, console: &mut dyn Console) -> SalvoStep<String> {
        loop {
            match self.state {
                // Entry: everything up to the first `yield`.
                0 => {
                    console.println(&format!("opening {}", self.path));
                    self.f = fs.open(&self.path);
                    self.defer_close_pending = true;
                    self.state = 1;
                    return SalvoStep::Next(format!("-- {} --", self.path));
                }
                // The `while true` head, which is also where the second
                // `yield` resumes to.
                1 => {
                    let l = fs.read_line(self.f);
                    match l {
                        Some(l) => {
                            self.state = 1;
                            return SalvoStep::Next(l);
                        }
                        // The bare `return`: leave the body, running defers.
                        None => {
                            self.state = 2;
                        }
                    }
                }
                // Leaving normally: the block's deferred code runs here, on
                // the way out, exactly as it does in a non-iterator fn.
                2 => {
                    self.run_pending_defers(fs, console);
                    self.state = 3;
                    return SalvoStep::Stopped;
                }
                // Finished. Driving a finished pass keeps reporting Stopped.
                _ => return SalvoStep::Stopped,
            }
        }
    }

    /// The injected close path: the same deferred code, reached because the
    /// *consumer* stopped rather than because the body did. Idempotent, so
    /// one call after the loop covers exhaustion and `break` alike.
    fn close(&mut self, fs: &mut dyn FileSystem, console: &mut dyn Console) {
        self.run_pending_defers(fs, console);
        self.state = 3;
    }

    /// Deferred blocks run latest-registered first, and may perform effects —
    /// which is why this takes the handlers too, and why a Rust `Drop` could
    /// never have been the mechanism.
    fn run_pending_defers(&mut self, fs: &mut dyn FileSystem, console: &mut dyn Console) {
        if self.defer_close_pending {
            self.defer_close_pending = false;
            console.println(&format!("closing {}", self.path));
            fs.close(self.f);
        }
    }
}

fn lines(path: String) -> LinesPass {
    LinesPass::new(path)
}

// -------------------------------------------------------------------- main

fn main() {
    let mut console = StdOutConsole;
    let mut fs = FakeFileSystem::new();
    // Handlers are locals; `&mut dyn` borrows are what the calls thread.
    let console: &mut dyn Console = &mut console;
    let fs: &mut dyn FileSystem = &mut fs;

    let mut seen = 0;

    // `for l in lines("data.txt") { ... }`
    let mut pass = lines("data.txt".to_string());
    while let SalvoStep::Next(l) = pass.next(fs, console) {
        console.println(&format!("line {l}"));
        seen += 1;
        if seen == 3 {
            break;
        }
    }
    // Injected on the way out of the loop: covers exhaustion and every
    // `break`, because both land here. A `return` inside the body would get
    // its own splice, like `defer` already does.
    pass.close(fs, console);

    console.println("done");
}
