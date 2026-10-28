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
    pub id: String,
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

/// Splits a data file into its programs and their annotations.
pub fn parse(text: &str) -> Result<Vec<Case>, String> {
    let mut cases: Vec<Case> = Vec::new();
    let mut case = Case { name: "main".into(), at: 1, ..Case::default() };
    let mut file = ("main.sv".to_string(), String::new());
    let mut pending: Vec<(usize, &'static str, String)> = Vec::new();
    let mut line_in_file = 0usize;
    let finish_file = |case: &mut Case, file: &mut (String, String), pending: &mut Vec<(usize, &'static str, String)>, at: usize| -> Result<(), String> {
        if let Some((n, _, id)) = pending.first() {
            return Err(format!("line {n}: annotation `{id}` names no line (at {at})"));
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
            let (severity, rest) = if let Some(r) = rest.strip_prefix("error[") {
                ("error", r)
            } else if let Some(r) = rest.strip_prefix("warning[") {
                ("warning", r)
            } else {
                return Err(format!("line {n}: an annotation is `//~ error[id]` or `//~ warning[id]`, not `{t}`"));
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
            pending.push((n, severity, id.to_string()));
            continue;
        }
        for (_, severity, id) in pending.drain(..) {
            case.expected.push(Expect { file: file.0.clone(), line: line_in_file, severity, id });
        }
    }
    finish_file(&mut case, &mut file, &mut pending, text.lines().count())?;
    if !case.files.is_empty() || !case.expected.is_empty() {
        cases.push(case);
    }
    Ok(cases)
}

/// What checking `case` reported, in the annotations' terms.
pub fn diagnostics(case: &Case) -> (Vec<Expect>, Vec<String>) {
    let files: Vec<(&str, &str)> = case.files.iter().map(|(n, c)| (n.as_str(), c.as_str())).collect();
    let checked = crate::front::check(&files);
    let mut got = Vec::new();
    let mut shown = Vec::new();
    for d in checked.diagnostics() {
        let file = &checked.program.files[d.file];
        let line = if d.span.is_expanded() {
            0
        } else {
            salvo_syntax::span::line_col(&file.content, d.span.start).0 as usize
        };
        let e = Expect {
            file: if file.is_std { format!("<std>/{}", file.name) } else { file.name.clone() },
            line,
            severity: match d.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
            },
            id: d.id.unwrap_or("?").to_string(),
        };
        shown.push(format!("{e}: {}", d.message));
        got.push(e);
    }
    (got, shown)
}

/// The mismatch between what `case` must report and what it did, or `None`.
pub fn check_case(case: &Case) -> Option<String> {
    if !case.emits.is_empty() || !case.refused.is_empty() {
        return Some(format!("case `{}` (line {}): `emits` and `refused` belong to a backend's emit data, not the checker's", case.name, case.at));
    }
    let (got, shown) = diagnostics(case);
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
    if missing.is_empty() && unexpected.is_empty() {
        return None;
    }
    let mut out = format!("case `{}` (line {}):", case.name, case.at);
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
    crate::front::std_front("kt");
    failures.extend(over_cases(&cases, check_case));
    assert!(failures.is_empty(), "{} of {} programs in {} disagree with their annotations:\n\n{}", failures.len(), cases.len(), dir.display(), failures.join("\n\n"));
}

/// [test-emit-data] A backend, for [`emit_dir`]: its std front's extension,
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
    crate::front::std_front(backend.ext);
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
}
