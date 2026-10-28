//! [test-run-data] Compile-and-run test data (TESTS.md C3): `.sv` files
//! whose comments say how the program must behave when compiled by each
//! backend and run. The runner that compiles and runs them lives with the
//! backends (`salvo-cli`'s `run_data.rs`); this module is the format alone,
//! so a test without the backends (the IR corpus) can read the same files.
//!
//! ```text
//! // ARGS: a b
//! fn main() [use] -> None { … }
//! //= stdout
//! //= 3
//! //= done
//! ```
//!
//! - `// BACKENDS: rust kotlin` limits a program to those backends (both,
//!   when absent); `// ARGS: a b` is its command line, split on spaces.
//! - Comments before the first `//---` are the file's preamble, not a
//!   program; comments may follow a program's outcome.
//! - The outcome: `//= exit N` (the exit code, 0 when absent) and
//!   `//= trap <text>` (the program stops at a trap whose message starts
//!   `salvo: <text>`, a nonzero exit on either backend) come first; then
//!   `//= stdout`, after which every `//=` line is one line of the exact
//!   stdout (`//= ` and one space stripped, `//=` alone an empty line).
//!   Without `//= stdout`, the program prints nothing.
//! - `// FILE: name` starts another file of the same program (a `.sv`
//!   module, or a backend's companion or platform file: the program is read
//!   as `salvo compile` reads a source tree); before the first, the file is
//!   `main.sv`. `//--- name` starts another program.
//! - Every line stays in its file, the directives included, so a trap's
//!   `main.sv:3:5` is the data file's own line.

/// A backend a program runs on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Backend {
    Rust,
    Kotlin,
}

impl Backend {
    pub const ALL: [Backend; 2] = [Backend::Rust, Backend::Kotlin];

    pub fn name(self) -> &'static str {
        match self {
            Backend::Rust => "rust",
            Backend::Kotlin => "kotlin",
        }
    }

    /// The extension of its companion files, and of its std front.
    pub fn ext(self) -> &'static str {
        match self {
            Backend::Rust => "rs",
            Backend::Kotlin => "kt",
        }
    }
}

/// How a program must end.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Outcome {
    pub stdout: String,
    /// The exit code, for a program that does not trap.
    pub exit: i32,
    /// `Some(text)`: the program traps with a message starting `salvo: text`.
    pub trap: Option<String>,
}

/// One program of a run data file.
#[derive(Clone, Debug, Default)]
pub struct Case {
    /// `//--- name`, or `main` for the part before the first marker.
    pub name: String,
    /// The line of the data file the case starts on, for reports.
    pub at: usize,
    pub backends: Vec<Backend>,
    pub args: Vec<String>,
    pub files: Vec<(String, String)>,
    pub outcome: Outcome,
}

impl Case {
    /// The program's `.sv` files alone (`name`, `text`).
    pub fn sources(&self) -> Vec<(&str, &str)> {
        self.files.iter().filter(|(n, _)| n.ends_with(".sv")).map(|(n, c)| (n.as_str(), c.as_str())).collect()
    }
}

/// Splits a run data file into its programs.
pub fn parse(text: &str) -> Result<Vec<Case>, String> {
    struct State {
        case: Case,
        file: (String, String),
        /// After `//= stdout`: every `//=` line is output.
        in_stdout: bool,
        saw_stdout: bool,
        saw_backends: bool,
    }
    let fresh = |name: String, at: usize| State {
        case: Case { name, at, backends: Backend::ALL.to_vec(), ..Case::default() },
        file: ("main.sv".to_string(), String::new()),
        in_stdout: false,
        saw_stdout: false,
        saw_backends: false,
    };
    fn finish_file(s: &mut State) {
        let (name, content) = std::mem::take(&mut s.file);
        if !content.trim().is_empty() || !s.case.files.is_empty() {
            s.case.files.push((name, content));
        }
    }
    let mut cases = Vec::new();
    let mut s = fresh("main".into(), 1);
    let finish_case = |s: &mut State, cases: &mut Vec<Case>| {
        finish_file(s);
        // Comments alone (a file's preamble, before its first `//---`) are
        // not a program.
        let only_comments = s.case.files.iter().all(|(_, c)| c.lines().all(|l| l.trim().is_empty() || l.trim().starts_with("//")));
        if !s.case.files.is_empty() && !only_comments {
            cases.push(std::mem::take(&mut s.case));
        }
    };
    for (i, raw) in text.lines().enumerate() {
        let n = i + 1;
        let t = raw.trim();
        if let Some(name) = t.strip_prefix("//---") {
            finish_case(&mut s, &mut cases);
            s = fresh(name.trim().to_string(), n);
            continue;
        }
        if let Some(name) = t.strip_prefix("// FILE:") {
            if s.saw_stdout {
                return Err(format!("line {n}: `// FILE:` after `//= stdout` (the outcome comes last)"));
            }
            finish_file(&mut s);
            s.file = (name.trim().to_string(), String::new());
            continue;
        }
        s.file.1.push_str(raw);
        s.file.1.push('\n');
        if let Some(rest) = raw.trim_start().strip_prefix("//=") {
            if s.in_stdout {
                // Trailing spaces are output too.
                let line = rest.strip_prefix(' ').unwrap_or(rest);
                s.case.outcome.stdout.push_str(line);
                s.case.outcome.stdout.push('\n');
                continue;
            }
            let rest = rest.trim();
            if rest == "stdout" {
                s.in_stdout = true;
                s.saw_stdout = true;
            } else if let Some(code) = rest.strip_prefix("exit ") {
                s.case.outcome.exit = code.trim().parse().map_err(|_| format!("line {n}: `{t}`: the exit code is a number"))?;
            } else if let Some(trap) = rest.strip_prefix("trap ") {
                s.case.outcome.trap = Some(trap.trim().to_string());
            } else {
                return Err(format!("line {n}: `{t}`: before `//= stdout`, a `//=` line is `exit N` or `trap <text>`"));
            }
            continue;
        }
        if s.in_stdout && !t.is_empty() && !t.starts_with("//") {
            return Err(format!("line {n}: `{t}` follows `//= stdout` (the expected output ends the program)"));
        }
        if let Some(list) = t.strip_prefix("// BACKENDS:") {
            if s.saw_backends {
                return Err(format!("line {n}: a second `// BACKENDS:`"));
            }
            s.saw_backends = true;
            s.case.backends = list
                .split_whitespace()
                .map(|b| match b {
                    "rust" => Ok(Backend::Rust),
                    "kotlin" => Ok(Backend::Kotlin),
                    other => Err(format!("line {n}: `{other}` is not a backend (`rust`, `kotlin`)")),
                })
                .collect::<Result<_, _>>()?;
            if s.case.backends.is_empty() {
                return Err(format!("line {n}: `// BACKENDS:` names no backend"));
            }
        } else if let Some(args) = t.strip_prefix("// ARGS:") {
            s.case.args = args.split_whitespace().map(str::to_string).collect();
        }
    }
    finish_case(&mut s, &mut cases);
    for case in &cases {
        if case.outcome.trap.is_some() && case.outcome.exit != 0 {
            return Err(format!("case `{}` (line {}): `//= trap` and `//= exit` together", case.name, case.at));
        }
    }
    Ok(cases)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// [test-run-data] Headers, files, programs and the outcome block.
    #[test]
    fn a_run_file_splits_into_programs_and_outcomes() {
        let cases = parse(
            "// ARGS: a b\nfn main() {}\n// FILE: lib.sv\nfn f() {}\n//= stdout\n//= one\n//=\n//=  two\n\
             //--- second\n// BACKENDS: rust\nfn main() {}\n//= trap unreachable\n",
        )
        .unwrap();
        assert_eq!(cases.len(), 2);
        assert_eq!(cases[0].args, ["a", "b"]);
        assert_eq!(cases[0].files.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>(), ["main.sv", "lib.sv"]);
        assert_eq!(cases[0].outcome.stdout, "one\n\n two\n");
        assert_eq!(cases[0].backends, Backend::ALL);
        assert_eq!(cases[1].name, "second");
        assert_eq!(cases[1].backends, [Backend::Rust]);
        assert_eq!(cases[1].outcome.trap.as_deref(), Some("unreachable"));
        assert_eq!(cases[1].outcome.stdout, "");
        assert!(parse("fn main() {}\n//= stdout\n//= x\nfn g() {}\n").is_err());
        assert!(parse("//= bogus\nfn main() {}\n").is_err());
        assert!(parse("// BACKENDS: java\nfn main() {}\n").is_err());
    }
}
