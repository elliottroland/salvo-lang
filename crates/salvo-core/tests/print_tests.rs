//! [ast-print] The AST printer (`salvo_syntax::print`) writes source that
//! parses back to a tree it prints identically: over every std module after
//! the expansions, and over every `.sv` file of the repository as written.
//! A printer that writes something the parser reads differently shows up as
//! a difference between the two printings, or as a parse error. The
//! formatter's results are data: `tests/print/` (see
//! [`every_print_fixture_formats_as_expected`]).

use std::path::Path;

use salvo_syntax::print::print_module;

/// Prints `module`, parses the text back and prints that: the first
/// difference, or the parse errors, or `None` when the printing is stable.
/// `module` is a program's, so the text is parsed back lowered
/// [syntax-lowered], as `module` was.
fn unstable_expanded(name: &str, module: &salvo_syntax::ast::Module) -> Option<String> {
    let first = print_module(module).text;
    let (mut again, diags) = salvo_testkit::lowered(&first);
    salvo_core::message::declare_arms(&mut again);
    let errors: Vec<String> = diags
        .iter()
        .filter(|d| d.is_error())
        .map(|d| {
            let (line, col) = salvo_syntax::span::line_col(&first, d.span.start);
            let text = first.lines().nth(line as usize - 1).unwrap_or("");
            format!("{line}:{col}: {}\n    {text}", d.message)
        })
        .collect();
    if !errors.is_empty() {
        return Some(format!("{name}: the printed module does not parse:\n  {}", errors.join("\n  ")));
    }
    let second = print_module(&again).text;
    if first == second {
        return None;
    }
    let (a, b): (Vec<&str>, Vec<&str>) = (first.lines().collect(), second.lines().collect());
    let at = a.iter().zip(&b).position(|(x, y)| x != y).unwrap_or(a.len().min(b.len()));
    Some(format!(
        "{name}: printed again, line {} differs:\n  first:  {}\n  second: {}",
        at + 1,
        a.get(at).unwrap_or(&"<end>"),
        b.get(at).unwrap_or(&"<end>")
    ))
}

/// [comptime-hygiene] `name@[site]` as the printer writes it, a bare name.
fn site_selectors_to_names(module: &mut salvo_syntax::ast::Module) {
    use salvo_syntax::ast::Expr;
    struct V;
    impl salvo_syntax::visit_mut::MutVisitor for V {
        fn visit_expr(&mut self, e: &mut Expr) {
            if let Expr::Scoped { base, name, module, span } = e {
                if matches!(module.as_slice(), [m] if m.name == "[site]") {
                    let name = name.clone();
                    let span = *span;
                    *e = match base.take() {
                        Some(b) => Expr::Field { base: b, field: name, span, grouped: false },
                        None => Expr::Ident(name),
                    };
                }
            }
        }
    }
    salvo_syntax::node_id::walk_module(&mut V, module);
}

/// [comptime-default] A field default an expansion placed, as the printing
/// for a parse writes it.
fn inline_field_defaults(program: &salvo_core::Program, module: &mut salvo_syntax::ast::Module) {
    use salvo_syntax::ast::Expr;
    struct V<'a>(&'a salvo_core::Program);
    impl salvo_syntax::visit_mut::MutVisitor for V<'_> {
        fn visit_expr(&mut self, e: &mut Expr) {
            if let Expr::FieldDefault { owner, field, .. } = e {
                if let Some(d) = salvo_core::printed::field_default(self.0, owner, field) {
                    *e = d;
                }
            }
        }
    }
    salvo_syntax::node_id::walk_module(&mut V(program), module);
}

/// A node's debug form without its spans.
fn shape_of(node: &impl std::fmt::Debug) -> String {
    let text = format!("{node:#?}");
    let mut out = String::with_capacity(text.len());
    for line in text.lines() {
        let t = line.trim_start();
        // A span's debug form, `12..15` or `12..15#3`, alone or after a key,
        // inside the parentheses written around its node, `((12..15))`
        // [expr-group], which the tree comparison leaves aside.
        let value = t.strip_prefix("span: ").or_else(|| t.split_once(": ").map(|(_, v)| v)).unwrap_or(t);
        let v = value.trim_end_matches(',');
        let depth = v.len() - v.trim_start_matches('(').len();
        let v = if depth > 0 && v.len() > 2 * depth && v[v.len() - depth..].bytes().all(|c| c == b')') { &v[depth..v.len() - depth] } else { v };
        let is_span = v.split_once("..").is_some_and(|(a, b)| {
            let b = b.split('#').next().unwrap_or("");
            !a.is_empty() && a.bytes().all(|c| c.is_ascii_digit()) && !b.is_empty() && b.bytes().all(|c| c.is_ascii_digit())
        });
        if is_span {
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// What a re-parse cannot know: the state name an `iter fn`'s expansion
/// keeps on the fns it writes, and the empty deduction list it gives the
/// `next` (which has no spelling: a written clause has an entry).
fn forget_expansion_marks(module: &mut salvo_syntax::ast::Module) {
    for item in &mut module.items {
        if let salvo_syntax::ast::Item::Fn(f) = item {
            f.iter_state_name = None;
            if f.deductions.as_ref().is_some_and(|d| d.is_empty()) {
                f.deductions = None;
            }
        }
    }
}

/// What differs between each wanted module of `program` (expanded) and its
/// printing parsed back, item by item in any order (the printer puts imports
/// first and a struct's fns in its body, which the parse hoists to the end),
/// but for what only an expansion records: where a stamped fn came from.
fn expanded_tree_differences(program: &salvo_core::Program, wanted: impl Fn(&salvo_core::source::SourceFile) -> bool) -> Vec<String> {
    let mut failures = Vec::new();
    for (i, f) in program.files.iter().enumerate() {
        if !wanted(f) {
            continue;
        }
        let mut module: salvo_syntax::ast::Module = (*program.modules[i]).clone();
        let printed = salvo_core::printed::print_file_expanded_source(program, i).text;
        inline_field_defaults(program, &mut module);
        let (mut again, diags) = salvo_testkit::lowered(&printed);
        if let Some(d) = diags.iter().find(|d| d.is_error()) {
            let (line, _) = salvo_syntax::span::line_col(&printed, d.span.start);
            failures.push(format!("{}: printed, it does not parse: {} at\n    {}", f.name, d.message, printed.lines().nth(line as usize - 1).unwrap_or("")));
            continue;
        }
        salvo_core::message::declare_arms(&mut again);
        for m in [&mut module, &mut again] {
            forget_expansion_marks(m);
            for item in &mut m.items {
                if let salvo_syntax::ast::Item::Fn(f) = item {
                    f.stamped = None;
                    // The printing's notes on what produced a fn read back
                    // as its docs.
                    f.docs.clear();
                }
            }
        }
        site_selectors_to_names(&mut module);
        let items = |m: &salvo_syntax::ast::Module| {
            let mut v: Vec<String> = m.items.iter().map(|it| shape_of(it)).collect();
            v.sort();
            v.join("\n")
        };
        let (a, b) = (items(&module), items(&again));
        if a != b {
            let (la, lb): (Vec<&str>, Vec<&str>) = (a.lines().collect(), b.lines().collect());
            let at = la.iter().zip(&lb).position(|(x, y)| x != y).unwrap_or(la.len().min(lb.len()));
            let from = at.saturating_sub(10);
            failures.push(format!(
                "{}: the tree differs at line {at}:\n--- expanded\n{}\n--- printed\n{}",
                f.name,
                la[from..(at + 3).min(la.len())].join("\n"),
                lb[from..(at + 3).min(lb.len())].join("\n")
            ));
        }
    }
    failures
}

/// [ast-print] Every module after the expansions: std's, as the program
/// sees it (expanded and stamped), prints to source that parses and prints
/// the same again; and std's and every example's own modules print to
/// source that parses to the same tree ([`expanded_tree_differences`]).
#[test]
fn every_expanded_module_prints_to_its_own_tree() {
    let built = salvo_testkit::front::std("kt").program(Vec::new(), Vec::new());
    let program = built.program;
    let mut failures = Vec::new();
    for (i, f) in program.files.iter().enumerate() {
        if let Some(why) = unstable_expanded(&f.name, &program.modules[i]) {
            failures.push(format!("[stable] {why}"));
        }
    }
    failures.extend(expanded_tree_differences(&program, |_| true).into_iter().map(|f| format!("[std tree] {f}")));
    let examples = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    let mut dirs: Vec<_> = std::fs::read_dir(&examples)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.join("salvo.toml").exists() && !std::fs::read_to_string(p.join("salvo.toml")).unwrap_or_default().contains("[dependencies]"))
        .collect();
    dirs.sort();
    assert!(dirs.len() > 5, "found only {} examples", dirs.len());
    for dir in dirs {
        let built = salvo_testkit::front::project("kt", &dir);
        failures.extend(expanded_tree_differences(&built.program, |f| !f.is_std).into_iter().map(|f| format!("[example tree] {f}")));
    }
    assert!(failures.is_empty(), "{} failures over {} std modules and the examples:\n{}", failures.len(), program.files.len(), failures.join("\n\n"));
}

/// Every `.sv` file under `dir` (recursively), sorted.
fn sv_files(dir: &Path) -> Vec<std::path::PathBuf> {
    fn walk(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().is_some_and(|x| x == "sv") {
                out.push(p);
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, &mut out);
    out.sort();
    out
}

/// `text` parsed as written, or its first parse error with its line.
fn parse_written(text: &str) -> Result<salvo_syntax::ast::Module, String> {
    let (m, diags) = salvo_syntax::parse_module(text);
    match diags.iter().find(|d| d.is_error()) {
        Some(d) => {
            let (line, _) = salvo_syntax::span::line_col(text, d.span.start);
            Err(format!("{} at\n    {}", d.message, text.lines().nth(line as usize - 1).unwrap_or("")))
        }
        None => Ok(m),
    }
}

/// What differs between `written` (a tree as parsed) and its printing
/// `printed` parsed as written, or between the printing and its own
/// printing; `None` when both hold.
fn tree_difference(written: &salvo_syntax::ast::Module, printed: &str) -> Option<String> {
    let again = match parse_written(printed) {
        Ok(m) => m,
        Err(why) => return Some(format!("printed, it does not parse: {why}")),
    };
    let (a, b) = (shape_of(written), shape_of(&again));
    if a != b {
        let (la, lb): (Vec<&str>, Vec<&str>) = (a.lines().collect(), b.lines().collect());
        let at = la.iter().zip(&lb).position(|(x, y)| x != y).unwrap_or(la.len().min(lb.len()));
        let from = at.saturating_sub(8);
        return Some(format!(
            "the tree differs at line {at}:\n--- written\n{}\n--- printed\n{}",
            la[from..(at + 3).min(la.len())].join("\n"),
            lb[from..(at + 3).min(lb.len())].join("\n")
        ));
    }
    let twice = print_module(&again).text;
    if twice != printed {
        let (pa, pb): (Vec<&str>, Vec<&str>) = (printed.lines().collect(), twice.lines().collect());
        let at = pa.iter().zip(&pb).position(|(x, y)| x != y).unwrap_or(pa.len().min(pb.len()));
        return Some(format!("printed again, line {} differs:\n  first:  {}\n  second: {}", at + 1, pa.get(at).unwrap_or(&"<end>"), pb.get(at).unwrap_or(&"<end>")));
    }
    None
}

/// Every `.sv` file of the repository: its sources, examples, benchmarks,
/// modules and test data, but not what a build or a tool wrote (`target/`,
/// `tmp/`, `wiki/`, `build/` and hidden directories).
fn every_sv_file() -> Vec<std::path::PathBuf> {
    fn walk(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        for e in entries.flatten() {
            let p = e.path();
            let name = e.file_name().to_string_lossy().to_string();
            if p.is_dir() {
                if !(name.starts_with('.') || ["target", "tmp", "wiki", "build", "node_modules"].contains(&name.as_str())) {
                    walk(&p, out);
                }
            } else if p.extension().is_some_and(|x| x == "sv") {
                out.push(p);
            }
        }
    }
    let mut out = Vec::new();
    walk(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."), &mut out);
    out.sort();
    out
}

/// Whether `name` (relative to the repository) is a file that must parse:
/// std's sources, the language-docs corpus, the print fixtures, and every
/// example's own sources. Elsewhere a file may be test data meant not to.
fn must_parse(name: &str) -> bool {
    ["lib/std/", "crates/salvo-syntax/tests/corpus/", "crates/salvo-core/tests/print/"].iter().any(|d| name.starts_with(d))
        || (name.starts_with("examples/") && name.split('/').any(|c| c == "salvo"))
}

/// [fmt-comments] Each comment's text, as written, counted.
fn comment_texts(source: &str) -> std::collections::BTreeMap<String, usize> {
    let mut out = std::collections::BTreeMap::new();
    for c in salvo_syntax::lexer::lex(source).comments {
        *out.entry(source[c.span.start as usize..c.span.end as usize].to_string()).or_insert(0) += 1;
    }
    out
}

/// [fmt-width] The lines of `printed` wider than the width where something
/// could make them fit: a line that would not fit even with each string
/// literal on it written `""` and its comment left out (neither is ever
/// broken, and a break around a string is made only where it lets the line
/// fit), or that is one token, is not one.
fn too_wide(printed: &str) -> Vec<String> {
    let lexed = salvo_syntax::lexer::lex(printed);
    let strings: Vec<(usize, usize)> = lexed
        .tokens
        .iter()
        .filter(|t| matches!(t.kind, salvo_syntax::token::TokenKind::Str(_)))
        .map(|t| (t.span.start as usize, t.span.end as usize))
        .collect();
    let comments: Vec<usize> = lexed.comments.iter().map(|c| c.span.start as usize).collect();
    let mut out = Vec::new();
    let mut start = 0;
    for line in printed.split_inclusive('\n') {
        let (ls, le) = (start, start + line.trim_end_matches('\n').len());
        start += line.len();
        let cut = comments.iter().copied().find(|&c| ls <= c && c < le).unwrap_or(le);
        let mut width = printed[ls..cut].trim_end().chars().count();
        for &(s, e) in strings.iter().filter(|&&(s, e)| s < cut && e > ls) {
            width -= printed[s.max(ls)..e.min(cut)].chars().count().saturating_sub(2);
        }
        let one_token = !printed[ls..le].trim().contains(' ');
        if printed[ls..le].chars().count() > salvo_syntax::print::WIDTH && width > salvo_syntax::print::WIDTH && !one_token {
            out.push(printed[ls..le].to_string());
        }
    }
    out
}

/// [ast-print] [syntax-lowered] [fmt-comments] [fmt-blank-lines] [fmt-width]
/// [fmt-author-breaks] The printer's guarantees over every `.sv` file of the
/// repository, each parsed once as written (the tree the parser returns, its
/// sugar kept, nothing expanded). A file that does not parse is skipped as
/// test data meant not to, but for std's, the corpus's, the print
/// fixtures' and the examples' own ([`must_parse`]), where it is a failure.
/// Each failure names its file and its check:
///
/// - `comments`: the printing keeps every comment, as written, as often as
///   it was written;
/// - `tree`: it parses to the same tree (spans aside) and prints the same
///   again: what a formatter needs;
/// - `width breaks`: printed with no trivia (so no break is the author's
///   and every one is the width's), it parses to the same tree and prints
///   the same again with its trivia, where each of those breaks is now
///   written: the breaks the width makes read back as the author's;
/// - `width`: no printed line is wider than the width, but where nothing
///   can make it fit ([`too_wide`]).
#[test]
fn every_file_prints_to_its_own_tree() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let paths = every_sv_file();
    assert!(paths.len() > 500, "found only {} files", paths.len());
    let names: Vec<String> = paths.iter().map(|p| p.strip_prefix(&root).unwrap_or(p).display().to_string()).collect();
    let required = names.iter().filter(|n| must_parse(n)).count();
    // Each file on a pool of threads; the failures in the files' order.
    let next = std::sync::atomic::AtomicUsize::new(0);
    let results = std::sync::Mutex::new(Vec::new());
    let workers = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| loop {
                let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let (Some(path), Some(name)) = (paths.get(i), names.get(i)) else { return };
                let text = std::fs::read_to_string(path).unwrap();
                let checked = file_failures(name, &text);
                results.lock().unwrap().push((i, checked));
            });
        }
    });
    let mut results = results.into_inner().unwrap();
    results.sort_by_key(|(i, _)| *i);
    let checked = results.iter().filter(|(_, r)| r.is_some()).count();
    let failures: Vec<String> = results.into_iter().flat_map(|(_, r)| r.unwrap_or_default()).collect();
    assert!(checked > 400, "only {checked} files parse");
    assert!(required > 80, "found only {required} files that must parse");
    assert!(failures.is_empty(), "{} failures over {checked} files:\n{}", failures.len(), failures.join("\n\n"));
}

/// The checks of [`every_file_prints_to_its_own_tree`] over the file `name`
/// holding `text`: its failures, or `None` when it is skipped.
fn file_failures(name: &str, text: &str) -> Option<Vec<String>> {
    let module = match parse_written(text) {
        Ok(m) => m,
        Err(why) if must_parse(name) => return Some(vec![format!("{name} [parse]: does not parse: {why}")]),
        Err(_) => return None,
    };
    let mut failures = Vec::new();
    let printed = print_module(&module).text;

    let (before, after) = (comment_texts(text), comment_texts(&printed));
    // A container's docs the parser also gives the member written on its
    // line (ROADMAP §0av, members on one line) print above both.
    let duplicated = DOCS_ON_A_SHARED_LINE.contains(&name);
    if duplicated && before == after {
        failures.push(format!("{name} [comments]: keeps its comments now: take it off `DOCS_ON_A_SHARED_LINE`"));
    }
    if before != after && !duplicated {
        let lost: Vec<String> = before.iter().filter(|(t, n)| after.get(*t) != Some(n)).map(|(t, n)| format!("{n} x {t}")).collect();
        let gained: Vec<String> = after.iter().filter(|(t, n)| before.get(*t) != Some(n)).map(|(t, n)| format!("{n} x {t}")).collect();
        failures.push(format!("{name} [comments]: comments differ\n  written: {}\n  printed: {}", lost.join("\n           "), gained.join("\n           ")));
    }

    // The files the printer cannot write yet are named, and must still fail:
    // one it can write is taken off the list.
    match (tree_difference(&module, &printed), NOT_YET_PRINTED.contains(&name)) {
        (Some(why), false) => failures.push(format!("{name} [tree]: {why}")),
        (None, true) => failures.push(format!("{name} [tree]: prints to its own tree now: take it off `NOT_YET_PRINTED`")),
        _ => {}
    }

    let mut bare = module;
    bare.trivia = Default::default();
    let unbroken = print_module(&bare).text;
    match parse_written(&unbroken) {
        Err(why) => failures.push(format!("{name} [width breaks]: printed with no trivia, it does not parse: {why}")),
        Ok(m) => {
            if let Some(why) = tree_difference(&m, &print_module(&m).text) {
                failures.push(format!("{name} [width breaks]: {why}"));
            }
        }
    }

    for line in too_wide(&printed) {
        failures.push(format!("{name} [width]: {line}"));
    }
    Some(failures)
}

/// The files holding what the printer cannot write as written yet: comptime
/// statements and heads (ROADMAP §0av).
const NOT_YET_PRINTED: &[&str] = &[];

/// The files whose first member is written on its container's line under the
/// container's docs, which the parser gives both (ROADMAP §0av): printed,
/// the docs are above each.
const DOCS_ON_A_SHARED_LINE: &[&str] = &[
    "crates/salvo-backend-rust/tests/emit/effects/effect_generics_erased.sv",
    "crates/salvo-cli/tests/run/network/the_network_sequence_step_actor_groups.sv",
    "crates/salvo-core/tests/check/actors/self_send.sv",
    "crates/salvo-core/tests/check/eff_handle/handles.sv",
];

/// [ast-print] [syntax-lowered] [expr-group] [lit-spelling] [fmt-comments]
/// [fmt-blank-lines] [fmt-width] [fmt-author-breaks] [fmt-imports] [cli-fmt]
/// The formatter (`salvo_syntax::fmt::format`: the printing, the import
/// sort, and the check that the result parses to the same tree with the
/// same comments) over its data, and its result formats as itself:
///
/// - each `tests/print/<name>.sv`, written in the printer's layout, formats
///   back byte for byte: `sugar.sv` each piece of sugar as it was written,
///   not as what it stands for; `spellings.sv` the parentheses and the
///   literals as written; `comments.sv` a comment in every slot and the
///   blank lines between statements, members and declarations; `breaks.sv`
///   each breakable construct short on one line, short and broken by its
///   author, and broken by the width, nested, and every chain's operator;
///   `imports_*.sv` the import runs the sort leaves as written;
/// - each `tests/print/canonical/<name>.in.sv` formats as `<name>.out.sv`:
///   a line too wide broken the canonical way, a layout's blank lines, a
///   moved comment, sorted imports;
/// - the example of `docs/language/Formatting.md` (its first two `salvo`
///   fences) formats as the page shows.
#[test]
fn every_print_fixture_formats_as_expected() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/print");
    let mut cases: Vec<(String, String, String)> = Vec::new();
    for path in sv_files(&dir) {
        let name = path.strip_prefix(&dir).unwrap().display().to_string();
        let text = std::fs::read_to_string(&path).unwrap();
        if path.parent() == Some(dir.as_path()) {
            cases.push((name, text.clone(), text));
        } else if name.ends_with(".in.sv") {
            let output = path.with_file_name(path.file_name().unwrap().to_string_lossy().replace(".in.sv", ".out.sv"));
            let want = std::fs::read_to_string(&output).unwrap_or_else(|e| panic!("{name}: no {}: {e}", output.display()));
            cases.push((name, text, want));
        } else if !name.ends_with(".out.sv") {
            panic!("{name}: a canonical fixture is `<name>.in.sv` with its `<name>.out.sv`");
        }
    }
    let page = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/language/Formatting.md")).unwrap();
    let fences: Vec<&str> = page.split("```salvo\n").skip(1).map(|f| f.split("```").next().unwrap()).collect();
    assert!(fences.len() >= 2, "Formatting.md: no example and its formatting");
    cases.push(("docs/language/Formatting.md".to_string(), fences[0].to_string(), fences[1].to_string()));
    assert!(cases.len() > 10, "found only {} fixtures", cases.len());
    let mut failures = Vec::new();
    for (name, input, want) in &cases {
        for (what, from) in [("formatted", input), ("formatted again", want)] {
            match salvo_syntax::fmt::format(from) {
                Err(e) => failures.push(format!("{name}: {what}, it is refused: {e:?}")),
                Ok(got) if &got != want => {
                    let (a, b): (Vec<&str>, Vec<&str>) = (want.lines().collect(), got.lines().collect());
                    let at = a.iter().zip(&b).position(|(x, y)| x != y).unwrap_or(a.len().min(b.len()));
                    failures.push(format!("{name}: {what}, line {}:\n  expected: {}\n  got:      {}\n\n{got}", at + 1, a.get(at).unwrap_or(&"<end>"), b.get(at).unwrap_or(&"<end>")));
                }
                Ok(_) => {}
            }
        }
    }
    assert!(failures.is_empty(), "{} of {} fixtures:\n{}", failures.len(), cases.len(), failures.join("\n\n"));
}

/// [ast-print] [syntax-lowered] [actor-use-addr] A dump marks the spawn a
/// `use … on` was lowered to with the `use` as written; a source printing
/// writes the spawn, which parses back to the same tree.
#[test]
fn a_lowered_use_on_is_marked_in_a_dump() {
    use salvo_syntax::print::{print_module_with, NoNotes, Reading};
    let (module, diags) = salvo_testkit::lowered("use Logging() on pool(1)\n\nfn f() [use, spawn] {\n    use Counting(0) with Db() on pool(2)\n}\n");
    assert!(!diags.iter().any(|d| d.is_error()), "{diags:?}");
    let dump = print_module_with(&module, &NoNotes, Reading::Dump).text;
    for want in ["use spawn Logging() on pool(1)#{desugared from `use Logging() on pool(1)`}", "use spawn Counting(0) with Db() on pool(2)#{desugared from `use Counting(0) with Db() on pool(2)`}"] {
        assert!(dump.contains(want), "missing `{want}` in:\n{dump}");
    }
    let source = print_module(&module).text;
    assert!(source.contains("    use spawn Counting(0) with Db() on pool(2)\n") && !source.contains("desugared"), "{source}");
}

/// [ast-print] [syntax-lowered] [expr-replyto-flat] A dump marks the call a
/// trailing `replyto` was lowered into with the call as written, once for a
/// chain.
#[test]
fn a_lowered_trailing_replyto_is_marked_in_a_dump() {
    use salvo_syntax::print::{print_module_with, NoNotes, Reading};
    let (module, diags) = salvo_testkit::lowered("send fn f(n: Int) [spawn] {\n    total(n) replyto got(n) replyto done()\n    total(n, replyto got(n))\n}\n");
    assert!(!diags.iter().any(|d| d.is_error()), "{diags:?}");
    let dump = print_module_with(&module, &NoNotes, Reading::Dump).text;
    let want = "    total(n, replyto got(n, replyto done()))#{desugared from `total(n) replyto got(n) replyto done()`}\n    total(n, replyto got(n))\n";
    assert!(dump.contains(want), "missing `{want}` in:\n{dump}");
    assert_eq!(dump.matches("desugared").count(), 1, "{dump}");
}
