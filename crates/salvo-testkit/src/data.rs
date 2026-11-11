//! [test-data] Checker test data (TESTS.md T1): `.sv` files whose comments
//! say which diagnostics checking them must produce, checked over the real
//! std ([`crate::front`]).
//!
//! ```text
//! //~ error[op-arith:no-function]
//! let a = true * false
//! ```
//!
//! - An annotation, `//~ error[id]` or `//~ warning[id]`, is a line of its
//!   own and names the next line that is not an annotation; several stack on
//!   one line. The diagnostics a program produces must be exactly the
//!   annotated ones — same file, line, severity and id, as many of each — so
//!   a file with no annotations checks clean, warnings included.
//! - [type-query] `//~ type[T]` asserts a `type` diagnostic (a `type?`'s)
//!   whose type is `T`: the one annotation whose payload is a type, not an
//!   id. It is judged by the checker where the `type?` is
//!   (`CheckCache::check_annotated`): `T`'s names resolved in the scope
//!   there and compared as `type!(place: T)` compares, claims by the value
//!   they are about, qualifier and union arm order aside. So it is exactly
//!   as strong as the assertion, and an annotation whose names mean nothing
//!   there fails the file saying so. Exactly, like the rest: an
//!   unannotated `type?` fails the file.
//! - `// FILE: name.sv` starts another file of the same program; before the
//!   first, the file is `main.sv`. `//--- name` starts another program,
//!   checked on its own: for what must not share a program with the rest (a
//!   parse error, a program-wide analysis, a name another case declares).
//! - Tests assert ids, never message text: the text is `messages.rs`'s
//!   business ([diag-id]).
//! - [test-emit-data] A backend's emit data (`<backend crate>/tests/emit/`,
//!   [`emit_dir`]) uses the same files with two more annotations, placed by
//!   the construct they are about: `//~ emits: text` (the program's own
//!   emitted code holds `text`; `//~|` lines continue it, `emits(path)`
//!   names an emitted file, `emits 2` counts, `!emits` forbids) and
//!   `//~ refused[id]` (emission fails with exactly these ids).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use salvo_syntax::diag::Severity;

/// One annotation, or one diagnostic, in the terms they are compared in.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Expect {
    pub file: String,
    /// 1-based, within its file's text.
    pub line: usize,
    pub severity: &'static str,
    /// The diagnostic's id; for a `type` annotation, its type as written,
    /// and for a `type` report, the annotation's text it was judged to hold
    /// ([`TypeExpect`]), else the type it reported, marked so that it
    /// matches no annotation.
    pub id: String,
}

/// [test-data] [type-query] A `//~ type[T]` annotation, for the checker to
/// judge at the line it names (`salvo_core::TypeAnnotations`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeExpect {
    pub file: String,
    /// The 1-based line it names.
    pub line: usize,
    /// `T`, trimmed.
    pub text: String,
    /// Where `T` starts in its file's text.
    pub offset: u32,
}

impl std::fmt::Display for Expect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}: {}[{}]", self.file, self.line, self.severity, self.id)
    }
}

/// [test-emit-data] What a backend's emitted code must hold: `text`
/// (several lines joined by `\n`) in the program's own files, or in the one
/// at `path`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Emit {
    /// The data-file line of the annotation, for reports.
    pub at: usize,
    pub path: Option<String>,
    pub text: String,
    /// How often: `Some(n)` exactly n times, `None` at least once.
    pub count: Option<usize>,
    /// `!emits`: never.
    pub absent: bool,
}

/// One program of a data file: its files and what it must report.
#[derive(Debug, Default)]
pub struct Case {
    /// `//--- name`, or `main` for the part before the first marker.
    pub name: String,
    /// The line of the data file the case starts on, for reports.
    pub at: usize,
    pub files: Vec<(String, String)>,
    pub expected: Vec<Expect>,
    /// [type-query] The `//~ type[T]` annotations (each also in
    /// `expected`), with where their types are written.
    pub types: Vec<TypeExpect>,
    /// [test-emit-data] `//~ emits: …` and `//~ !emits: …`.
    pub emits: Vec<Emit>,
    /// [test-emit-data] `//~ refused[id]`: the ids emission fails with.
    pub refused: Vec<String>,
}

/// Parses `emits: text`, `emits(path): text`, `emits 2: text` and their
/// `!emits` forms (`rest` is what follows `//~ `).
fn parse_emit(rest: &str, at: usize) -> Option<Result<Emit, String>> {
    let (absent, rest) = match rest.strip_prefix('!') {
        Some(r) => (true, r),
        None => (false, rest),
    };
    let rest = rest.strip_prefix("emits")?;
    let Some((head, text)) = rest.split_once(':') else {
        return Some(Err(format!("line {at}: `emits` takes `: text`")));
    };
    let mut head = head.trim();
    let mut path = None;
    if let Some(r) = head.strip_prefix('(') {
        let Some((p, r)) = r.split_once(')') else {
            return Some(Err(format!("line {at}: unclosed `(` in `emits(…)`")));
        };
        path = Some(p.trim().to_string());
        head = r.trim();
    }
    let count = if head.is_empty() {
        None
    } else {
        match head.parse::<usize>() {
            Ok(n) => Some(n),
            Err(_) => return Some(Err(format!("line {at}: `emits {head}`: a count is a number"))),
        }
    };
    if absent && count.is_some() {
        return Some(Err(format!("line {at}: `!emits` takes no count")));
    }
    let text = text.strip_prefix(' ').unwrap_or(text).to_string();
    Some(Ok(Emit { at, path, text, count, absent }))
}

/// An annotation waiting for the line it names: its data-file line, and
/// what it asserts; a `type` annotation's `offset` is where its type starts
/// in its file.
struct Pending {
    at: usize,
    severity: &'static str,
    id: String,
    offset: Option<u32>,
}

/// Splits a data file into its programs and their annotations.
pub fn parse(text: &str) -> Result<Vec<Case>, String> {
    let mut cases: Vec<Case> = Vec::new();
    let mut case = Case { name: "main".into(), at: 1, ..Case::default() };
    let mut file = ("main.sv".to_string(), String::new());
    let mut pending: Vec<Pending> = Vec::new();
    let mut line_in_file = 0usize;
    let finish_file = |case: &mut Case, file: &mut (String, String), pending: &mut Vec<Pending>, at: usize| -> Result<(), String> {
        if let Some(p) = pending.first() {
            return Err(format!("line {}: annotation `{}` names no line (at {at})", p.at, p.id));
        }
        pending.clear();
        let (name, content) = std::mem::take(file);
        if !content.trim().is_empty() || !case.files.is_empty() {
            case.files.push((name, content));
        }
        Ok(())
    };
    for (i, raw) in text.lines().enumerate() {
        let n = i + 1;
        let t = raw.trim();
        if let Some(name) = t.strip_prefix("//---") {
            finish_file(&mut case, &mut file, &mut pending, n)?;
            if !case.files.is_empty() || !case.expected.is_empty() || !case.emits.is_empty() || !case.refused.is_empty() {
                cases.push(std::mem::take(&mut case));
            }
            case = Case { name: name.trim().to_string(), at: n, ..Case::default() };
            file = ("main.sv".to_string(), String::new());
            line_in_file = 0;
            continue;
        }
        if let Some(name) = t.strip_prefix("// FILE:") {
            finish_file(&mut case, &mut file, &mut pending, n)?;
            file = (name.trim().to_string(), String::new());
            line_in_file = 0;
            continue;
        }
        // Every other line, annotations included, is the file's text, so
        // the lines a diagnostic reports are the data file's own.
        let line_start = file.1.len();
        file.1.push_str(raw);
        file.1.push('\n');
        line_in_file += 1;
        if let Some(cont) = raw.trim_start().strip_prefix("//~|") {
            // [test-emit-data] A further line of the `emits` text above.
            let Some(last) = case.emits.last_mut() else {
                return Err(format!("line {n}: `//~|` continues no `emits`"));
            };
            last.text.push('\n');
            last.text.push_str(cont.strip_prefix(' ').unwrap_or(cont));
            continue;
        }
        if let Some(rest) = raw.trim_start().strip_prefix("//~ ").filter(|r| r.trim_start().starts_with("emits") || r.trim_start().starts_with("!emits")) {
            match parse_emit(rest.trim_start(), n) {
                Some(Ok(e)) => case.emits.push(e),
                Some(Err(e)) => return Err(e),
                None => unreachable!(),
            }
            continue;
        }
        if let Some(id) = t.strip_prefix("//~ refused[").and_then(|r| r.strip_suffix(']')) {
            if !salvo_syntax::messages::id_well_formed(id) {
                return Err(format!("line {n}: `{id}` is not an id (`label:name`)"));
            }
            case.refused.push(id.to_string());
            continue;
        }
        if let Some(rest) = t.strip_prefix("//~") {
            let rest = rest.trim();
            // [test-data] [type-query] `//~ type[T]`: a `type` diagnostic
            // whose type is `T`, judged by the checker at the `type?`.
            // Brackets balance, since a type may hold some (an effect list).
            if let Some(r) = rest.strip_prefix("type[") {
                let Some(close) = closing_bracket(r) else {
                    return Err(format!("line {n}: unclosed `[` in `{t}`"));
                };
                if !r[close + 1..].trim().is_empty() {
                    return Err(format!("line {n}: nothing follows an annotation's type: `{t}`"));
                }
                let written = &r[..close];
                let ty = written.trim();
                if salvo_syntax::parse_type(ty).is_none() {
                    return Err(format!("line {n}: `{ty}` is not a type"));
                }
                // Where `T` starts in the file: `r` is a slice of `raw`.
                let in_line = (r.as_ptr() as usize - raw.as_ptr() as usize) + (written.len() - written.trim_start().len());
                pending.push(Pending { at: n, severity: "type", id: ty.to_string(), offset: Some((line_start + in_line) as u32) });
                continue;
            }
            let (severity, rest) = if let Some(r) = rest.strip_prefix("error[") {
                ("error", r)
            } else if let Some(r) = rest.strip_prefix("warning[") {
                ("warning", r)
            } else {
                return Err(format!("line {n}: an annotation is `//~ error[id]`, `//~ warning[id]` or `//~ type[T]`, not `{t}`"));
            };
            let Some((id, tail)) = rest.split_once(']') else {
                return Err(format!("line {n}: unclosed `[` in `{t}`"));
            };
            if !salvo_syntax::messages::id_well_formed(id) {
                return Err(format!("line {n}: `{id}` is not an id (`label:name`)"));
            }
            if !tail.trim().is_empty() {
                return Err(format!("line {n}: nothing follows an annotation's id (tests assert ids, not text): `{t}`"));
            }
            pending.push(Pending { at: n, severity, id: id.to_string(), offset: None });
            continue;
        }
        for p in pending.drain(..) {
            if let Some(offset) = p.offset {
                case.types.push(TypeExpect { file: file.0.clone(), line: line_in_file, text: p.id.clone(), offset });
            }
            case.expected.push(Expect { file: file.0.clone(), line: line_in_file, severity: p.severity, id: p.id });
        }
    }
    finish_file(&mut case, &mut file, &mut pending, text.lines().count())?;
    if !case.files.is_empty() || !case.expected.is_empty() {
        cases.push(case);
    }
    Ok(cases)
}

/// [test-data] Where the `]` closing an annotation's `[` is, in the text
/// after the `[`: brackets nest.
fn closing_bracket(text: &str) -> Option<usize> {
    let mut depth = 0usize;
    for (i, c) in text.char_indices() {
        match c {
            '[' => depth += 1,
            ']' if depth == 0 => return Some(i),
            ']' => depth -= 1,
            _ => {}
        }
    }
    None
}

/// What checking a case reported, in the annotations' terms.
pub struct Reported {
    /// Each diagnostic, as an annotation would say it.
    pub got: Vec<Expect>,
    /// Each diagnostic with its message, in `got`'s order, for reports.
    pub shown: Vec<String>,
    /// [type-query] The `//~ type[T]` annotations that mean no type at their
    /// `type?` (a name that resolves to nothing there), with why.
    pub unresolved: Vec<String>,
}

/// [test-data] [type-query] The case's `//~ type[T]` annotations, by the
/// index of their file in `program` and the line they name.
fn type_annotations(case: &Case, program: &salvo_core::Program) -> salvo_core::TypeAnnotations {
    let mut out = salvo_core::TypeAnnotations::default();
    for t in &case.types {
        let file = program.files.iter().position(|f| !f.is_std && f.name == t.file).unwrap_or_else(|| panic!("no file `{}` in the program", t.file));
        out.at.entry((file, t.line)).or_default().push(salvo_core::TypeAnnotation { text: t.text.clone(), offset: t.offset });
    }
    out
}

/// What checking `case` reported. A `type` report is named by the first
/// annotation on its line the checker judged it to hold, not yet taken by
/// another report there, and by the type it shows when there is none.
pub fn diagnostics(case: &Case) -> Reported {
    let files: Vec<(&str, &str)> = case.files.iter().map(|(n, c)| (n.as_str(), c.as_str())).collect();
    let built = crate::front::program("kt", &files);
    let annotations = type_annotations(case, &built.program);
    let checked = crate::front::check_built_annotated(built, &annotations);
    let mut reported = Reported { got: Vec::new(), shown: Vec::new(), unresolved: Vec::new() };
    let mut taken: std::collections::HashSet<(usize, usize, usize)> = std::collections::HashSet::new();
    for d in checked.diagnostics() {
        let file = &checked.program.files[d.file];
        let line = if d.span.is_expanded() {
            0
        } else {
            salvo_syntax::span::line_col(&file.content, d.span.start).0 as usize
        };
        let name = if file.is_std { format!("<std>/{}", file.name) } else { file.name.clone() };
        let e = match d.severity {
            Severity::Type => {
                let written = annotations.at.get(&(d.file, line)).map(Vec::as_slice).unwrap_or_default();
                for (a, v) in written.iter().zip(&d.type_verdicts) {
                    if let salvo_core::TypeVerdict::Unresolved(why) = v {
                        let note = format!("  {name}:{line}: type[{}]: {why}", a.text);
                        if !reported.unresolved.contains(&note) {
                            reported.unresolved.push(note);
                        }
                    }
                }
                let held = d.type_verdicts.iter().enumerate().find(|(i, v)| **v == salvo_core::TypeVerdict::Holds && !taken.contains(&(d.file, line, *i)));
                let id = match held {
                    Some((i, _)) => {
                        taken.insert((d.file, line, i));
                        written[i].text.clone()
                    }
                    // The type as shown: it holds no backtick, so it
                    // follows the last `` `: `` (an origin prefix may hold
                    // others). Marked, so it never matches an annotation
                    // by its spelling: only a verdict names one.
                    None => format!("{} (held by no annotation here)", d.message.rsplit_once("`: ").map(|(_, t)| t).unwrap_or(&d.message)),
                };
                Expect { file: name, line, severity: "type", id }
            }
            Severity::Error => Expect { file: name, line, severity: "error", id: d.id.unwrap_or("?").to_string() },
            Severity::Warning => Expect { file: name, line, severity: "warning", id: d.id.unwrap_or("?").to_string() },
        };
        reported.shown.push(format!("{e}: {}", d.message));
        reported.got.push(e);
    }
    reported
}

/// The mismatch between what `case` must report and what it did, or `None`.
pub fn check_case(case: &Case) -> Option<String> {
    if !case.emits.is_empty() || !case.refused.is_empty() {
        return Some(format!("case `{}` (line {}): `emits` and `refused` belong to a backend's emit data, not the checker's", case.name, case.at));
    }
    let Reported { got, shown, unresolved } = diagnostics(case);
    let mut want: BTreeMap<&Expect, isize> = BTreeMap::new();
    for e in &case.expected {
        *want.entry(e).or_default() += 1;
    }
    for e in &got {
        *want.entry(e).or_default() -= 1;
    }
    let missing: Vec<String> = want.iter().filter(|(_, n)| **n > 0).map(|(e, n)| format!("  {e}{}", if *n > 1 { format!(" (×{n})") } else { String::new() })).collect();
    let unexpected: Vec<String> = want
        .iter()
        .filter(|(_, n)| **n < 0)
        .flat_map(|(e, _)| shown.iter().filter(move |s| s.starts_with(&e.to_string())).map(|s| format!("  {s}")))
        .collect();
    if missing.is_empty() && unexpected.is_empty() && unresolved.is_empty() {
        return None;
    }
    let mut out = format!("case `{}` (line {}):", case.name, case.at);
    if !unresolved.is_empty() {
        out.push_str(&format!("\n annotated types that mean nothing at their `type?`:\n{}", unresolved.join("\n")));
    }
    if !missing.is_empty() {
        out.push_str(&format!("\n expected, not reported:\n{}", missing.join("\n")));
    }
    if !unexpected.is_empty() {
        out.push_str(&format!("\n reported, not annotated:\n{}", unexpected.join("\n")));
    }
    Some(out)
}

/// Every program of the `.sv` files under `dir`, with its file's name, and
/// the files that did not parse.
fn read_dir_cases(dir: &Path) -> (Vec<(String, Case)>, Vec<String>) {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "sv"))
        .collect();
    paths.sort();
    assert!(!paths.is_empty(), "no `.sv` data files in {}", dir.display());
    let mut cases: Vec<(String, Case)> = Vec::new();
    let mut failures: Vec<String> = Vec::new();
    for path in &paths {
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let text = std::fs::read_to_string(path).unwrap();
        match parse(&text) {
            Ok(cs) => cases.extend(cs.into_iter().map(|c| (name.clone(), c))),
            Err(e) => failures.push(format!("{name}: {e}")),
        }
    }
    (cases, failures)
}

/// `check` over every case on a pool of threads: each mismatch, in order.
fn over_cases(cases: &[(String, Case)], check: impl Fn(&Case) -> Option<String> + Sync) -> Vec<String> {
    let next = std::sync::atomic::AtomicUsize::new(0);
    let found = std::sync::Mutex::new(Vec::new());
    let workers = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).min(cases.len().max(1));
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| loop {
                let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let Some((name, case)) = cases.get(i) else { return };
                if let Some(report) = check(case) {
                    found.lock().unwrap().push((i, format!("{name}: {report}")));
                }
            });
        }
    });
    let mut found = found.into_inner().unwrap();
    found.sort();
    found.into_iter().map(|(_, f)| f).collect()
}

/// Checks every `.sv` file under `dir`, each program on a pool of threads,
/// and panics listing every mismatch.
pub fn check_dir(dir: &Path) {
    let (cases, mut failures) = read_dir_cases(dir);
    crate::front::std("kt");
    failures.extend(over_cases(&cases, check_case));
    assert!(failures.is_empty(), "{} of {} programs in {} disagree with their annotations:\n\n{}", failures.len(), cases.len(), dir.display(), failures.join("\n\n"));
}

/// [test-emit-data] A backend, for [`emit_dir`]: its warm std's extension,
/// how it emits a program (the files, or the rendered errors), and which
/// emitted files are the program's own rather than std's or the runtime's.
pub struct Emitter<'a> {
    pub ext: &'a str,
    pub emit: &'a (dyn Fn(&salvo_core::Program) -> Result<Vec<(PathBuf, String)>, Vec<String>> + Sync),
    pub own: &'a (dyn Fn(&Path) -> bool + Sync),
}

/// The id a rendered error carries under `-v` (`error[id]:` or `[id] …`).
fn rendered_id(error: &str) -> Option<&str> {
    let mut rest = error;
    while let Some(i) = rest.find('[') {
        let after = &rest[i + 1..];
        let end = after.find(']')?;
        let id = &after[..end];
        if salvo_syntax::messages::id_well_formed(id) {
            return Some(id);
        }
        rest = after;
    }
    None
}

/// [test-emit-data] What emitting `case` got wrong, or `None`.
pub fn emit_case(case: &Case, backend: &Emitter) -> Option<String> {
    let head = format!("case `{}` (line {}):", case.name, case.at);
    if !case.expected.is_empty() {
        return Some(format!("{head} emit data takes `emits` and `refused`; a checker diagnostic is the checker's data"));
    }
    let files: Vec<(&str, &str)> = case.files.iter().map(|(n, c)| (n.as_str(), c.as_str())).collect();
    let built = crate::front::program(backend.ext, &files);
    let parse: Vec<String> = built.diagnostics.iter().filter(|d| d.is_error()).map(|d| d.render(&built.program.files)).collect();
    if !parse.is_empty() {
        return Some(format!("{head} parse or expansion errors:\n  {}", parse.join("\n  ")));
    }
    match ((backend.emit)(&built.program), case.refused.is_empty()) {
        (Ok(_), false) => Some(format!("{head} emitted, but should be refused with {:?}", case.refused)),
        (Err(errors), true) => Some(format!("{head} refused:\n  {}", errors.join("\n  "))),
        (Err(errors), false) => {
            let mut got: Vec<&str> = errors.iter().map(|e| rendered_id(e).unwrap_or("?")).collect();
            let mut want: Vec<&str> = case.refused.iter().map(String::as_str).collect();
            got.sort();
            want.sort();
            (got != want).then(|| format!("{head} refused with {got:?}, annotated {want:?}:\n  {}", errors.join("\n  ")))
        }
        (Ok(files), true) => {
            let own: String = files.iter().filter(|(p, _)| (backend.own)(p)).map(|(_, c)| c.as_str()).collect::<Vec<_>>().join("\n");
            let mut wrong = Vec::new();
            for e in &case.emits {
                let text = match &e.path {
                    None => own.as_str(),
                    Some(p) => match files.iter().find(|(f, _)| f.to_string_lossy() == p.as_str()) {
                        Some((_, c)) => c.as_str(),
                        None => {
                            let paths: Vec<String> = files.iter().map(|(f, _)| f.display().to_string()).collect();
                            wrong.push(format!("line {}: no emitted file `{p}` (there are {})", e.at, paths.join(", ")));
                            continue;
                        }
                    },
                };
                let n = text.matches(e.text.as_str()).count();
                let ok = match (e.absent, e.count) {
                    (true, _) => n == 0,
                    (false, Some(want)) => n == want,
                    (false, None) => n > 0,
                };
                if !ok {
                    let what = if e.absent { "must not appear".to_string() } else { format!("found {n} time(s)") };
                    wrong.push(format!("line {}: `{}` {what}", e.at, e.text));
                }
            }
            (!wrong.is_empty()).then(|| format!("{head}\n  {}\n--- the program's own emitted code:\n{own}", wrong.join("\n  ")))
        }
    }
}

/// [test-emit-data] Emits every program under `dir` with `backend`, on a
/// pool of threads, and panics listing every mismatch.
pub fn emit_dir(dir: &Path, backend: &Emitter) {
    salvo_syntax::diag::show_ids(true);
    let (cases, mut failures) = read_dir_cases(dir);
    crate::front::std(backend.ext);
    failures.extend(over_cases(&cases, |c| emit_case(c, backend)));
    assert!(failures.is_empty(), "{} of {} programs in {} disagree with their annotations:\n\n{}", failures.len(), cases.len(), dir.display(), failures.join("\n\n"));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// [test-data] Annotations name the next line that is not one, stack,
    /// and count; `// FILE:` and `//---` split files and programs.
    #[test]
    fn annotations_name_the_line_below_them() {
        let cases = parse(
            "fn a() {}\n//~ error[op-arith:x]\n//~ warning[var-unused:y]\nlet z = 1\n\
             // FILE: lib.sv\n//~ error[call-resolve:z]\nfn b() {}\n//--- second\nfn c() {}\n",
        )
        .unwrap();
        assert_eq!(cases.len(), 2);
        assert_eq!(cases[0].files.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>(), ["main.sv", "lib.sv"]);
        let want: Vec<String> = cases[0].expected.iter().map(|e| e.to_string()).collect();
        assert_eq!(want, ["main.sv:4: error[op-arith:x]", "main.sv:4: warning[var-unused:y]", "lib.sv:2: error[call-resolve:z]"]);
        assert_eq!(cases[1].name, "second");
        assert!(cases[1].expected.is_empty());
        assert!(parse("//~ error[op-arith:x] text\nfn a() {}\n").is_err());
        assert!(parse("fn a() {}\n//~ error[op-arith:x]\n").is_err());
        assert!(parse("//~ note[op-arith:x]\nfn a() {}\n").is_err());
        let emits = parse("fn a() {}\n//~ emits: let x = 1;\n//~|     next\n//~ !emits(main.rs): y\n//~ emits 2: z \n//~ refused[rs-x:y]\n").unwrap();
        let e = &emits[0].emits;
        assert_eq!((e[0].text.as_str(), e[0].count, e[0].absent), ("let x = 1;\n    next", None, false));
        assert_eq!((e[1].path.as_deref(), e[1].absent), (Some("main.rs"), true));
        assert_eq!((e[2].text.as_str(), e[2].count), ("z ", Some(2)));
        assert_eq!(emits[0].refused, ["rs-x:y"]);
    }

    /// [test-data] [type-query] `//~ type[T]` holds a type, brackets and
    /// all, kept as written with where it is written, for the checker to
    /// judge at the `type?`.
    #[test]
    fn a_type_annotation_is_kept_with_where_it_is() {
        let cases = parse("fn a() {}\n//~ type[ (Int) [Log, Console] -> Mut  Ok Str ]\ntype?(x)\n").unwrap();
        assert_eq!(cases[0].expected[0].severity, "type");
        assert_eq!(cases[0].expected[0].id, "(Int) [Log, Console] -> Mut  Ok Str");
        let t = &cases[0].types[0];
        assert_eq!((t.file.as_str(), t.line), ("main.sv", 3));
        let content = &cases[0].files[0].1;
        assert_eq!(&content[t.offset as usize..t.offset as usize + t.text.len()], t.text);
        assert!(parse("//~ type[Int\nfn a() {}\n").is_err(), "unclosed");
        assert!(parse("//~ type[Int] more\nfn a() {}\n").is_err(), "a tail");
        assert!(parse("//~ type[Int |]\nfn a() {}\n").is_err(), "not a type");
    }

    const CURSOR: &str = "struct Cursor canbe Mut {\n    str: proj Str\n    pos: Idx(str) Int\n}\n";

    fn checked(body: &str) -> Option<String> {
        let cases = parse(&format!("{CURSOR}{body}\nfn main() {{}}\n")).unwrap();
        crate::front::std("kt");
        check_case(&cases[0])
    }

    /// [test-data] [type-query] An annotation is judged as `type!(place: T)`
    /// is, at the `type?`: a destructured `str` and the read `cursor.str`
    /// are one value whichever spelling the annotation uses, and qualifier
    /// order does not matter.
    #[test]
    fn a_type_annotation_compares_claims_by_identity() {
        let body = "fn f(cursor: Mut Cursor) [] -> None => cursor {\n    let { str, pos } = cursor\n    \
                    //~ type[Idx(cursor.str) Int]\n    type?(pos)\n    \
                    //~ type[Idx(str) Int]\n    type?(cursor.pos)\n    \
                    //~ type[Mut Cursor]\n    type?(cursor)\n    \
                    let _n = str.char_at(pos)\n}\n";
        assert_eq!(checked(body), None);
    }

    /// [test-data] [type-query] Two cursors each destructured into a local
    /// `str`: the first's index prints `Idx(str)` where `str` is the
    /// second's. An annotation spelled as the report is printed is about
    /// the wrong value there, and fails; one naming a local that is not in
    /// scope at the `type?` fails saying so.
    #[test]
    fn a_type_annotation_spelled_alike_but_about_another_value_fails() {
        let head = "fn f(a: Cursor, b: Cursor) [] -> None => a, b {\n    \
                    let _p = if size(a.str) > 0 {\n        let { str, pos } = a\n        let _c = size(str)\n        pos\n    \
                    } else {\n        let { str, pos } = a\n        let _c = size(str)\n        pos\n    }\n";
        let shadowed = format!(
            "{head}    if size(b.str) > 0 {{\n        let {{ str, pos }} = b\n        \
             //~ type[Idx(str) Int]\n        type?(_p)\n        let _d = str.char_at(pos)\n    }}\n}}\n"
        );
        let failure = checked(&shadowed).expect("`Idx(str)` is about `b.str` at the `type?`");
        assert!(failure.contains("expected, not reported:\n  main.sv:18: type[Idx(str) Int]"), "{failure}");
        assert!(failure.contains("type[Idx(str) Int (held by no annotation here)]"), "{failure}");
        let out_of_scope = format!("{head}    //~ type[Idx(str) Int]\n    type?(_p)\n}}\n");
        let failure = checked(&out_of_scope).expect("no `str` is in scope at the `type?`");
        assert!(failure.contains("annotated types that mean nothing at their `type?`:\n  main.sv:16: type[Idx(str) Int]: `str` names no local here"), "{failure}");
        let unannotated = format!("{head}    type?(_p)\n}}\n");
        let failure = checked(&unannotated).expect("a `type?` with no annotation fails");
        assert!(failure.contains("reported, not annotated"), "{failure}");
    }
}
