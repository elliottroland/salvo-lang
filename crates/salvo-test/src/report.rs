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

/// [test-output] Whose output the report shows: every test's, or only a
/// failing (or dying) test's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShowFor {
    All,
    Failed,
}

/// [test-output] Which of a test's output streams the report shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Streams {
    /// Only what the test wrote to stderr.
    Stderr,
    /// stderr and stdout.
    Both,
}

/// [test-output] What of a test's own output the report shows (`[test]
/// output` / `streams` in the manifest, `--output` / `--streams` on the
/// command line). The default is the failures' output, both streams.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OutputPolicy {
    pub show_for: ShowFor,
    pub streams: Streams,
}

impl Default for OutputPolicy {
    fn default() -> Self {
        OutputPolicy { show_for: ShowFor::Failed, streams: Streams::Both }
    }
}

impl OutputPolicy {
    /// From the written values (`all`/`failed`, `stderr`/`both`), each
    /// defaulted when absent. The values are checked where they are written.
    pub fn from_names(show_for: Option<&str>, streams: Option<&str>) -> Self {
        let default = OutputPolicy::default();
        OutputPolicy {
            show_for: match show_for {
                Some("all") => ShowFor::All,
                Some("failed") => ShowFor::Failed,
                _ => default.show_for,
            },
            streams: match streams {
                Some("stderr") => Streams::Stderr,
                Some("both") => Streams::Both,
                _ => default.streams,
            },
        }
    }
}

/// What one pass of the harness came to: its counts, and the stderr lines
/// that arrived while no test was running (the runner prints those after the
/// pass, as before).
pub struct Rendered {
    pub summary: Summary,
    pub loose_stderr: Vec<String>,
}

/// Renders a harness's output stream as the report, onto `out`, closing with
/// the summary.
pub fn render(
    lines: impl BufRead,
    out: &mut impl Write,
    color: Color,
    policy: OutputPolicy,
) -> std::io::Result<Summary> {
    let rendered = render_stream(lines, out, color, policy, &mut Vec::new)?;
    writeln!(out)?;
    summary_line(&rendered.summary, out, color)?;
    Ok(rendered.summary)
}

/// The prefix every protocol line starts with.
const MARK: &str = "##salvo-test ";

/// The test between its `begin` and its verdict.
struct Running {
    id: String,
    start: Instant,
    /// Its output, held until the verdict says whether it is shown
    /// ([`ShowFor::Failed`]): `(from stderr, line)`.
    held: Vec<(bool, String)>,
    /// Whether any of its output is already on the page (shown live), so the
    /// verdict re-states which test it was.
    shown: bool,
}

/// [test-report] [test-output] `render` without the closing summary: one
/// *pass* of the harness, which is not always the whole run — a test that
/// dies takes the process with it, so the runner re-runs the remainder and
/// the counts are merged before the summary is printed once [test-recover].
///
/// A test's output goes under its header, indented — stderr marked as such —
/// and the verdict re-states the test once it has: shown as it arrives under
/// [`ShowFor::All`], held until the verdict and shown only for a failure (or a
/// death) under [`ShowFor::Failed`]. `stderr` hands over the stderr lines
/// that have arrived since it was last asked, which go to the test running
/// when they are read (the two streams are separate pipes, so a line written
/// just before a verdict can land after it).
///
/// Output the test left unterminated (`print("…")` with no newline) shares a
/// line with the next protocol line; it is split off rather than mistaken for
/// part of it (found 2026-10-20: the verdict was lost, the test reported
/// `DIED` and the rest of the suite re-run).
pub fn render_stream(
    lines: impl BufRead,
    out: &mut impl Write,
    color: Color,
    policy: OutputPolicy,
    stderr: &mut dyn FnMut() -> Vec<String>,
) -> std::io::Result<Rendered> {
    let mut summary = Summary::default();
    let mut running: Option<Running> = None;
    let mut loose_stderr: Vec<String> = Vec::new();
    let live = policy.show_for == ShowFor::All;

    // One line of the test's output, from stdout or stderr.
    let output = |text: &str,
                  from_stderr: bool,
                  running: &mut Option<Running>,
                  loose_stderr: &mut Vec<String>,
                  out: &mut dyn Write|
     -> std::io::Result<()> {
        if !from_stderr && policy.streams == Streams::Stderr {
            return Ok(());
        }
        match running {
            Some(r) if live => {
                if !r.shown {
                    writeln!(out)?;
                    r.shown = true;
                }
                write_output_line(out, from_stderr, text)?;
                out.flush()
            }
            Some(r) => {
                r.held.push((from_stderr, text.to_string()));
                Ok(())
            }
            None if from_stderr => {
                loose_stderr.push(text.to_string());
                Ok(())
            }
            None => writeln!(out, "{text}"),
        }
    };
    // Before a verdict: the held output, when this verdict shows it, and the
    // header again when anything went under it.
    let close = |r: &mut Running, failed: bool, out: &mut dyn Write| -> std::io::Result<()> {
        if !live && failed && !r.held.is_empty() {
            writeln!(out)?;
            for (from_stderr, line) in r.held.drain(..) {
                write_output_line(out, from_stderr, &line)?;
            }
            r.shown = true;
        }
        if r.shown {
            write!(out, "test {} ... ", r.id)?;
        }
        Ok(())
    };

    for line in lines.lines() {
        let line = line?;
        for err in stderr() {
            output(&err, true, &mut running, &mut loose_stderr, out)?;
        }
        // Text the test did not end with a newline, glued to a protocol line.
        let protocol = match line.find(MARK) {
            Some(0) => line.as_str(),
            Some(at) => {
                output(&line[..at], false, &mut running, &mut loose_stderr, out)?;
                &line[at..]
            }
            None => {
                output(&line, false, &mut running, &mut loose_stderr, out)?;
                continue;
            }
        };
        if let Some(id) = protocol.strip_prefix(BEGIN) {
            if let Some(prior) = running.take() {
                // A `begin` with one still open cannot happen from the
                // generated harness; reported rather than assumed away.
                summary.unfinished.push(prior.id);
            }
            write!(out, "test {id} ... ")?;
            out.flush()?;
            running = Some(Running { id: id.to_string(), start: Instant::now(), held: Vec::new(), shown: false });
        } else if protocol == OK {
            let ms = match running.take() {
                Some(mut r) => {
                    close(&mut r, false, out)?;
                    r.start.elapsed().as_millis()
                }
                None => 0,
            };
            writeln!(out, "{} ({ms} ms)", color.green("ok"))?;
            summary.passed += 1;
        } else if let Some(message) = protocol.strip_prefix(FAIL) {
            if let Some(mut r) = running.take() {
                close(&mut r, true, out)?;
            }
            writeln!(out, "{}", color.red("FAILED"))?;
            for line in message.lines() {
                writeln!(out, "    {line}")?;
            }
            summary.failed += 1;
        } else {
            // A marker the harness never prints: the test's own text.
            output(protocol, false, &mut running, &mut loose_stderr, out)?;
        }
    }
    // A test still running here died: what it left on stderr is the runner's
    // to attribute ([`died_detail`]), so it is not drawn into the report too.
    if running.is_none() {
        for err in stderr() {
            output(&err, true, &mut running, &mut loose_stderr, out)?;
        }
    }
    if let Some(mut pending) = running.take() {
        close(&mut pending, true, out)?;
        writeln!(out, "{}", color.red("DIED"))?;
        summary.unfinished.push(pending.id);
    }
    Ok(Rendered { summary, loose_stderr })
}

/// One line of a test's output, under its header.
fn write_output_line(out: &mut dyn Write, from_stderr: bool, text: &str) -> std::io::Result<()> {
    if from_stderr {
        writeln!(out, "  [stderr] {text}")
    } else {
        writeln!(out, "  {text}")
    }
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
        run_with(input, OutputPolicy { show_for: ShowFor::All, streams: Streams::Both }, Vec::new())
    }

    /// With `stderr` handed over in one batch once the first test has begun
    /// (when the second stdout line is read), and the closing summary.
    fn run_with(input: &str, policy: OutputPolicy, stderr: Vec<&str>) -> (String, Summary) {
        let mut out = Vec::new();
        let mut pending: Vec<String> = stderr.into_iter().map(str::to_string).collect();
        let mut asked = 0;
        let mut feed = move || {
            asked += 1;
            if asked == 2 { std::mem::take(&mut pending) } else { Vec::new() }
        };
        let rendered = render_stream(input.as_bytes(), &mut out, Color::Never, policy, &mut feed).unwrap();
        writeln!(out).unwrap();
        summary_line(&rendered.summary, &mut out, Color::Never).unwrap();
        (String::from_utf8(out).unwrap(), rendered.summary)
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

    /// [test-output] Under `all`, a test's output goes under its header once,
    /// indented, and the verdict re-states the test.
    #[test]
    fn program_output_goes_under_its_test() {
        let (text, _) = run(&format!("{BEGIN}heap :: one\nfirst\nsecond\n{OK}\n"));
        assert!(
            text.starts_with("test heap :: one ... \n  first\n  second\ntest heap :: one ... ok ("),
            "{text}"
        );
    }

    /// [test-output] Under `failed`, a passing test's output is dropped and a
    /// failing (or dying) test's is shown before its verdict.
    #[test]
    fn failed_shows_only_the_failures_output() {
        let policy = OutputPolicy::default();
        let (text, _) = run_with(
            &format!("{BEGIN}m :: quiet\nhidden\n{OK}\n{BEGIN}m :: loud\nshown\n{FAIL}nope\n{BEGIN}m :: dies\nlast words\n"),
            policy,
            Vec::new(),
        );
        assert!(!text.contains("hidden"), "{text}");
        assert!(text.contains("test m :: quiet ... ok ("), "{text}");
        assert!(text.contains("test m :: loud ... \n  shown\ntest m :: loud ... FAILED\n    nope\n"), "{text}");
        assert!(text.contains("test m :: dies ... \n  last words\ntest m :: dies ... DIED"), "{text}");
    }

    /// [test-output] `stderr` shows a test's stderr and not its stdout; stderr
    /// lines are marked.
    #[test]
    fn stderr_streams_drop_stdout() {
        let policy = OutputPolicy { show_for: ShowFor::All, streams: Streams::Stderr };
        let (text, _) = run_with(&format!("{BEGIN}m :: t\nfrom stdout\n{OK}\n"), policy, vec!["from stderr"]);
        assert!(!text.contains("from stdout"), "{text}");
        assert!(text.contains("  [stderr] from stderr\ntest m :: t ... ok ("), "{text}");
    }

    /// [test-output] Output the test left without a newline is split off the
    /// protocol line it shares, so the verdict still counts.
    #[test]
    fn unterminated_output_keeps_the_verdict() {
        let (text, summary) = run(&format!("{BEGIN}m :: a\npartial{OK}\n{BEGIN}m :: b\n{OK}\n"));
        assert_eq!(summary.passed, 2, "{text}");
        assert!(summary.unfinished.is_empty(), "{text}");
        assert!(text.contains("  partial\ntest m :: a ... ok ("), "{text}");
    }

    #[test]
    fn an_empty_run_says_so() {
        let (text, summary) = run("");
        assert!(text.contains("0 tests: 0 passed"), "{text}");
        assert!(summary.ok());
    }
}
