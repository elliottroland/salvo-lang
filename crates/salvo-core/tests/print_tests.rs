//! [ast-print] The AST printer (`salvo_syntax::print`) writes source that
//! parses back to a tree it prints identically: over every std module after
//! the expansions, and over the parser corpus. A printer that writes
//! something the parser reads differently shows up as a difference between
//! the two printings, or as a parse error.

use std::path::Path;

use salvo_syntax::print::print_module;

/// Prints `module`, parses the text back and prints that: the first
/// difference, or the parse errors, or `None` when the printing is stable.
/// `expanded`: `module` is a program's, so the text is parsed back lowered
/// [syntax-lowered], as `module` was; otherwise it is the tree as written.
fn unstable(name: &str, module: &salvo_syntax::ast::Module, expanded: bool) -> Option<String> {
    let first = print_module(module).text;
    let (mut again, diags) = if expanded { salvo_testkit::lowered(&first) } else { salvo_syntax::parse_module(&first) };
    if expanded {
        salvo_core::message::declare_arms(&mut again);
    }
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

/// [ast-print] Every std module, as the program sees it (expanded and
/// stamped), prints to source that parses and prints the same again.
#[test]
fn every_std_module_prints_stably() {
    let built = salvo_testkit::front::std("kt").program(Vec::new(), Vec::new());
    let program = built.program;
    let mut failures = Vec::new();
    for (i, f) in program.files.iter().enumerate() {
        if let Some(why) = unstable(&f.name, &program.modules[i], true) {
            failures.push(why);
        }
    }
    assert!(failures.is_empty(), "{} of {} std modules:\n{}", failures.len(), program.files.len(), failures.join("\n\n"));
}

/// [ast-print] Every corpus file that parses, as parsed, prints stably.
#[test]
fn every_corpus_file_prints_stably() {
    let corpus = Path::new(env!("CARGO_MANIFEST_DIR")).join("../salvo-syntax/tests/corpus");
    let mut paths: Vec<_> = std::fs::read_dir(&corpus).unwrap().filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "sv")).collect();
    paths.sort();
    let mut failures = Vec::new();
    for path in paths {
        let text = std::fs::read_to_string(&path).unwrap();
        let (module, diags) = salvo_syntax::parse_module(&text);
        if diags.iter().any(|d| d.is_error()) {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        if let Some(why) = unstable(&name, &module, false) {
            failures.push(why);
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

/// The tree's debug form without its spans.
fn shape(module: &salvo_syntax::ast::Module) -> String {
    shape_of(module)
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
        // A span's debug form, `12..15` or `12..15#3`, alone or after a key.
        let value = t.strip_prefix("span: ").or_else(|| t.split_once(": ").map(|(_, v)| v)).unwrap_or(t);
        let v = value.trim_end_matches(',');
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

/// The source files a tree comparison runs over: the corpus, std's own
/// files and the examples'.
fn source_files() -> Vec<std::path::PathBuf> {
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
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut out = Vec::new();
    for dir in ["crates/salvo-syntax/tests/corpus", "lib/std", "examples"] {
        walk(&root.join(dir), &mut out);
    }
    out.sort();
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

/// [ast-print] Every source file that parses prints to source that parses
/// to the same tree (spans aside): the corpus, std's files and the
/// examples'.
#[test]
fn every_source_file_prints_to_its_own_tree() {
    let mut failures = Vec::new();
    let paths = source_files();
    for path in &paths {
        let text = std::fs::read_to_string(path).unwrap();
        let (mut module, diags) = salvo_syntax::parse_module(&text);
        if diags.iter().any(|d| d.is_error()) {
            continue;
        }
        let printed = print_module(&module).text;
        let (mut again, diags) = salvo_syntax::parse_module(&printed);
        if let Some(d) = diags.iter().find(|d| d.is_error()) {
            let (line, _) = salvo_syntax::span::line_col(&printed, d.span.start);
            failures.push(format!("{}: printed, it does not parse: {} at\n    {}", path.display(), d.message, printed.lines().nth(line as usize - 1).unwrap_or("")));
            continue;
        }
        forget_expansion_marks(&mut module);
        forget_expansion_marks(&mut again);
        let (a, b) = (shape(&module), shape(&again));
        if a != b {
            let (la, lb): (Vec<&str>, Vec<&str>) = (a.lines().collect(), b.lines().collect());
            let at = la.iter().zip(&lb).position(|(x, y)| x != y).unwrap_or(la.len().min(lb.len()));
            let from = at.saturating_sub(8);
            failures.push(format!(
                "{}: the tree differs at line {at}:\n--- written\n{}\n--- printed\n{}",
                path.display(),
                la[from..(at + 3).min(la.len())].join("\n"),
                lb[from..(at + 3).min(lb.len())].join("\n")
            ));
        }
    }
    assert!(failures.is_empty(), "{} of {} files:\n{}", failures.len(), paths.len(), failures.join("\n\n"));
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

/// [ast-print] Every std module after the expansions prints to source that
/// parses to the same tree ([`expanded_tree_differences`]).
#[test]
fn every_expanded_std_module_prints_to_its_own_tree() {
    let built = salvo_testkit::front::std("kt").program(Vec::new(), Vec::new());
    let failures = expanded_tree_differences(&built.program, |_| true);
    assert!(failures.is_empty(), "{} files:\n{}", failures.len(), failures.join("\n\n"));
}

/// [ast-print] And every example's own modules.
#[test]
fn every_expanded_example_prints_to_its_own_tree() {
    let examples = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    let mut dirs: Vec<_> = std::fs::read_dir(&examples)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.join("salvo.toml").exists() && !std::fs::read_to_string(p.join("salvo.toml")).unwrap_or_default().contains("[dependencies]"))
        .collect();
    dirs.sort();
    let mut failures = Vec::new();
    for dir in dirs {
        let built = salvo_testkit::front::project("kt", &dir);
        failures.extend(expanded_tree_differences(&built.program, |f| !f.is_std));
    }
    assert!(failures.is_empty(), "{} files:\n{}", failures.len(), failures.join("\n\n"));
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

/// What differs between `text` parsed as written and its printing parsed
/// as written, or between the printing and its own printing; `None` when
/// both hold.
fn written_difference(name: &str, text: &str) -> Option<String> {
    let parse = |t: &str| {
        let (m, diags) = salvo_syntax::parse_module(t);
        match diags.iter().find(|d| d.is_error()) {
            Some(d) => {
                let (line, _) = salvo_syntax::span::line_col(t, d.span.start);
                Err(format!("{} at\n    {}", d.message, t.lines().nth(line as usize - 1).unwrap_or("")))
            }
            None => Ok(m),
        }
    };
    let written = match parse(text) {
        Ok(m) => m,
        Err(why) => return Some(format!("{name}: does not parse: {why}")),
    };
    let printed = print_module(&written).text;
    let again = match parse(&printed) {
        Ok(m) => m,
        Err(why) => return Some(format!("{name}: printed, it does not parse: {why}")),
    };
    let (a, b) = (shape(&written), shape(&again));
    if a != b {
        let (la, lb): (Vec<&str>, Vec<&str>) = (a.lines().collect(), b.lines().collect());
        let at = la.iter().zip(&lb).position(|(x, y)| x != y).unwrap_or(la.len().min(lb.len()));
        let from = at.saturating_sub(8);
        return Some(format!(
            "{name}: the tree differs at line {at}:\n--- written\n{}\n--- printed\n{}",
            la[from..(at + 3).min(la.len())].join("\n"),
            lb[from..(at + 3).min(lb.len())].join("\n")
        ));
    }
    let twice = print_module(&again).text;
    if twice != printed {
        let (pa, pb): (Vec<&str>, Vec<&str>) = (printed.lines().collect(), twice.lines().collect());
        let at = pa.iter().zip(&pb).position(|(x, y)| x != y).unwrap_or(pa.len().min(pb.len()));
        return Some(format!("{name}: printed again, line {} differs:\n  first:  {}\n  second: {}", at + 1, pa.get(at).unwrap_or(&"<end>"), pb.get(at).unwrap_or(&"<end>")));
    }
    None
}

/// [ast-print] [syntax-lowered] Every source file **as written** — the tree
/// the parser returns, its sugar kept, nothing expanded — prints to source
/// that parses to the same tree (spans aside) and prints the same again:
/// what a formatter needs. Over std's sources, every example's, the
/// language-docs corpus and a fixture holding each piece of sugar in each
/// position. A file that does not parse is a failure, not a skip.
#[test]
fn every_source_file_as_written_prints_to_its_own_tree() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut paths = Vec::new();
    for dir in ["lib/std", "crates/salvo-syntax/tests/corpus", "crates/salvo-core/tests/print"] {
        paths.extend(sv_files(&root.join(dir)));
    }
    for example in sv_files(&root.join("examples")) {
        if example.components().any(|c| c.as_os_str() == "salvo") {
            paths.push(example);
        }
    }
    assert!(paths.len() > 80, "found only {} files", paths.len());
    let mut failures = Vec::new();
    for path in &paths {
        let text = std::fs::read_to_string(path).unwrap();
        let name = path.strip_prefix(&root).unwrap_or(path).display().to_string();
        let why = written_difference(&name, &text);
        // The files the printer cannot write yet are named, and must still
        // fail: one it can write is taken off the list.
        match (why, NOT_YET_PRINTED.contains(&name.as_str())) {
            (Some(why), false) => failures.push(why),
            (None, true) => failures.push(format!("{name}: prints to its own tree now: take it off `NOT_YET_PRINTED`")),
            _ => {}
        }
    }
    assert!(failures.is_empty(), "{} of {} files:\n{}", failures.len(), paths.len(), failures.join("\n\n"));
}

/// The files holding what the printer cannot write as written yet: comptime
/// statements and heads (ROADMAP §0av).
const NOT_YET_PRINTED: &[&str] = &[];

/// [ast-print] [syntax-lowered] The sugar fixture, written in the printer's
/// layout, prints back to its own text: each piece of sugar as it was
/// written, not as what it stands for.
#[test]
fn the_sugar_fixture_prints_as_written() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/print/sugar.sv");
    let text = std::fs::read_to_string(&path).unwrap();
    let code = &text;
    let (module, diags) = salvo_syntax::parse_module(&text);
    assert!(!diags.iter().any(|d| d.is_error()), "{diags:?}");
    let printed = print_module(&module).text;
    let (a, b): (Vec<&str>, Vec<&str>) = (code.lines().collect(), printed.lines().collect());
    let at = a.iter().zip(&b).position(|(x, y)| x != y);
    assert!(at.is_none() && a.len() == b.len(), "line {}:\n  written: {}\n  printed: {}\n\n{printed}", at.unwrap_or(a.len().min(b.len())) + 1, a.get(at.unwrap_or(0)).unwrap_or(&"<end>"), b.get(at.unwrap_or(0)).unwrap_or(&"<end>"));
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
