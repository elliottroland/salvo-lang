//! [test-report] The report: what a person reads off `salvo test`.
//!
//! ```text
//! test heap :: an empty heap pops nothing ... ok (2 ms)
//! test heap :: pops come out in order ... FAILED
//!     expected 9, got 5
//!
//! 5 tests: 4 passed, 1 failed
//! ```
//!
//! The module name is the one a program would `import` (`heap`, not the
//! annex's `heap.test`), the name is printed without its quotes, `ok` is green
//! and `FAILED` red, and a passing test carries the milliseconds it took (user
//! decisions 2026-09-23).
//!
//! Rendering lives here rather than in the harness because the harness is
//! Salvo — which has no escape for the ESC byte — and because timing the
//! protocol lines as they stream needs no clock capability inside the test
//! (see `harness`).

use std::io::{BufRead, Write};
use std::time::Instant;

use crate::harness::{BEGIN, FAIL, OK};

/// ANSI colour, when the output is a terminal [test-report].
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Always,
    Never,
}

impl Color {
    fn paint(self, code: &str, text: &str) -> String {
        match self {
            Color::Always => format!("\x1b[{code}m{text}\x1b[0m"),
            Color::Never => text.to_string(),
        }
    }

    fn green(self, text: &str) -> String {
        self.paint("32", text)
    }

    fn red(self, text: &str) -> String {
        self.paint("31", text)
    }
}

/// What a run came to.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Summary {
    pub passed: usize,
    pub failed: usize,
    /// Tests that began and never reported: the program died mid-test (a
    /// panic, an `!` on `None`, a killed process). Counted as failures, and
    /// named, because a run that ends silently is the one case a report must
    /// not lose.
    pub unfinished: Vec<String>,
}

impl Summary {
    /// [test-recover] Folds another pass's counts in: a run is one pass per
    /// process, and a died test costs a process.
    pub fn merge(&mut self, other: Summary) {
        self.passed += other.passed;
        self.failed += other.failed;
        self.unfinished.extend(other.unfinished);
    }

    pub fn total(&self) -> usize {
        self.passed + self.failed
    }

    /// Whether the run should be reported as a success.
    pub fn ok(&self) -> bool {
        self.failed == 0 && self.unfinished.is_empty()
    }
}

/// Renders a harness's output stream as the report, onto `out`.
///
/// Every line that is not protocol is passed through: a test that prints is
/// printing for a reason, and swallowing it would make a `println` debug
/// session impossible.
pub fn render(
    lines: impl BufRead,
    out: &mut impl Write,
    color: Color,
) -> std::io::Result<Summary> {
    let summary = render_stream(lines, out, color)?;
    writeln!(out)?;
    summary_line(&summary, out, color)?;
    Ok(summary)
}

/// [test-report] `render` without the closing summary: one *pass* of the
/// harness, which is not always the whole run — a test that dies takes the
/// process with it, so the runner re-runs the remainder and the counts are
/// merged before the summary is printed once [test-recover].
pub fn render_stream(
    lines: impl BufRead,
    out: &mut impl Write,
    color: Color,
) -> std::io::Result<Summary> {
    let mut summary = Summary::default();
    // The test currently between its `begin` and its verdict, and when it
    // started — which is how a passing test gets its milliseconds.
    let mut running: Option<(String, Instant)> = None;
    for line in lines.lines() {
        let line = line?;
        if let Some(id) = line.strip_prefix(BEGIN) {
            if let Some((prior, _)) = running.take() {
                // A `begin` with one still open cannot happen from the
                // generated harness; reported rather than assumed away.
                summary.unfinished.push(prior);
            }
            write!(out, "test {id} ... ")?;
            out.flush()?;
            running = Some((id.to_string(), Instant::now()));
            continue;
        }
        if line == OK {
            let ms = running
                .take()
                .map(|(_, start)| start.elapsed().as_millis())
                .unwrap_or(0);
            writeln!(out, "{} ({ms} ms)", color.green("ok"))?;
            summary.passed += 1;
            continue;
        }
        if let Some(message) = line.strip_prefix(FAIL) {
            running = None;
            writeln!(out, "{}", color.red("FAILED"))?;
            for line in message.lines() {
                writeln!(out, "    {line}")?;
            }
            summary.failed += 1;
            continue;
        }
        // Output from the code under test.
        if running.is_some() {
            // Keep the pending `test … ` line readable: the test's own output
            // goes below it.
            writeln!(out)?;
            writeln!(out, "  {line}")?;
            // The verdict line that follows re-states which test it was.
            let (id, start) = running.take().expect("checked");
            write!(out, "test {id} ... ")?;
            out.flush()?;
            running = Some((id, start));
        } else {
            writeln!(out, "{line}")?;
        }
    }
    if let Some((pending, _)) = running.take() {
        writeln!(out, "{}", color.red("DIED"))?;
        summary.unfinished.push(pending);
    }
    Ok(summary)
}

/// [test-recover] What the runner prints under a test that died: the trap
/// message the program left on stderr, which is what tells a reader *why* —
/// a failed `assert!` says so in Salvo's own words [assert-trap].
///
/// Only the lines that look like they belong to the failure are shown: Salvo's
/// own trap lines first, and if there are none, the tail of whatever the
/// program said. A host stack trace is left out; it names generated code.
pub fn died_detail(stderr: &str) -> Vec<String> {
    let salvo: Vec<String> = stderr
        .lines()
        .filter(|l| l.contains(TRAP_PREFIX))
        .map(|l| {
            // Keep the message from `salvo:` on, dropping the host's framing
            // (`thread 'main' panicked at …:` / `Exception in thread "main" …`).
            match l.find(TRAP_PREFIX) {
                Some(at) => l[at..].to_string(),
                None => l.to_string(),
            }
        })
        .collect();
    if !salvo.is_empty() {
        return salvo;
    }
    stderr
        .lines()
        .filter(|l| !l.trim_start().starts_with("at "))
        .filter(|l| !l.trim().is_empty())
        .rev()
        .take(3)
        .map(str::to_string)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect()
}

/// [assert-trap] How a Salvo trap message starts, on either backend.
pub const TRAP_PREFIX: &str = "salvo: ";

/// The closing summary, printed once for a whole run [test-report].
pub fn print_summary(
    summary: &Summary,
    out: &mut impl Write,
    color: Color,
) -> std::io::Result<()> {
    writeln!(out)?;
    summary_line(summary, out, color)
}

/// The closing line: counts, and the tests that never finished.
fn summary_line(
    summary: &Summary,
    out: &mut impl Write,
    color: Color,
) -> std::io::Result<()> {
    let total = summary.total() + summary.unfinished.len();
    let mut parts = vec![color.green(&format!("{} passed", summary.passed))];
    if summary.failed > 0 {
        parts.push(color.red(&format!("{} failed", summary.failed)));
    }
    if !summary.unfinished.is_empty() {
        parts.push(color.red(&format!(
            "{} never finished",
            summary.unfinished.len()
        )));
    }
    writeln!(
        out,
        "{total} test{}: {}",
        if total == 1 { "" } else { "s" },
        parts.join(", ")
    )?;
    for pending in &summary.unfinished {
        writeln!(out, "    {pending} — the program stopped while it ran")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(input: &str) -> (String, Summary) {
        let mut out = Vec::new();
        let summary = render(input.as_bytes(), &mut out, Color::Never).unwrap();
        (String::from_utf8(out).unwrap(), summary)
    }

    /// [test-report] A pass carries its milliseconds; a failure carries its
    /// message, indented under the test.
    #[test]
    fn renders_passes_and_failures() {
        let (text, summary) = run(&format!(
            "{BEGIN}heap :: one\n{OK}\n{BEGIN}heap :: two\n{FAIL}expected 9, got 5\n"
        ));
        assert!(text.contains("test heap :: one ... ok ("), "{text}");
        assert!(text.contains("test heap :: two ... FAILED\n    expected 9, got 5\n"), "{text}");
        assert!(text.contains("2 tests: 1 passed, 1 failed"), "{text}");
        assert_eq!(summary.passed, 1);
        assert_eq!(summary.failed, 1);
        assert!(!summary.ok());
    }

    /// A test that begins and never reports is the one failure a report must
    /// not lose: the program died while it ran.
    #[test]
    fn a_test_that_never_finishes_is_named() {
        let (text, summary) = run(&format!("{BEGIN}heap :: one\n{OK}\n{BEGIN}heap :: two\n"));
        assert!(text.contains("test heap :: two ... DIED"), "{text}");
        assert_eq!(summary.unfinished, vec!["heap :: two".to_string()]);
        assert!(!summary.ok());
    }

    /// Output from the code under test is passed through, not swallowed.
    #[test]
    fn program_output_survives() {
        let (text, _) = run(&format!("{BEGIN}heap :: one\nhello from the test\n{OK}\n"));
        assert!(text.contains("  hello from the test"), "{text}");
        assert!(text.contains("test heap :: one ... ok ("), "{text}");
    }

    #[test]
    fn an_empty_run_says_so() {
        let (text, summary) = run("");
        assert!(text.contains("0 tests: 0 passed"), "{text}");
        assert!(summary.ok());
    }
}
