//! Hygiene for the language documentation.
//!
//! `docs/language/` is the narrative specification and the source of truth for
//! behaviour; `wiki/` is a **generated** copy of it for reading on GitHub
//! (`salvo docs`, run in `lib/`). Both facts are only true if something checks them,
//! which is what these tests do — the same reasoning that makes
//! `every_examples_checked_in_*_is_current` a test rather than a habit.

use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    // `CARGO_MANIFEST_DIR` is crates/salvo-cli.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("repo root")
        .to_path_buf()
}

/// Every page in `docs/language/` is listed in the index, and every page the
/// index lists exists. Without this a new page is invisible — the index is
/// what gives the pages a reading order, and what the wiki sidebar is built
/// from.
#[test]
fn every_language_page_is_indexed() {
    let dir = repo_root().join("docs/language");
    let index = std::fs::read_to_string(dir.join("README.md")).expect("docs/language/README.md");

    let mut pages: Vec<String> = std::fs::read_dir(&dir)
        .expect("docs/language")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.ends_with(".md") && n != "README.md")
        .collect();
    pages.sort();

    for page in &pages {
        assert!(
            index.contains(&format!("]({page})")),
            "docs/language/{page} is not linked from README.md — add it to the \
             reading order, or the wiki sidebar will not carry it"
        );
    }
    // …and nothing is listed that does not exist.
    for line in index.lines() {
        let Some(start) = line.find("](") else { continue };
        let rest = &line[start + 2..];
        let Some(end) = rest.find(')') else { continue };
        let target = &rest[..end];
        if !target.ends_with(".md") || target.contains('/') {
            continue;
        }
        assert!(
            dir.join(target).exists(),
            "docs/language/README.md links {target}, which does not exist"
        );
    }
}

/// When a `wiki/` checkout is present it must be **current**: it is generated,
/// so a stale page is a page that lies about the language. Skips when the
/// checkout is absent — the wiki is a separate repository and a clone of this
/// one need not have it (the toolchain tests skip the same way).
#[test]
fn the_wiki_is_current_when_present() {
    let root = repo_root();
    if !root.join("wiki").is_dir() {
        eprintln!("skipping: no wiki/ checkout");
        return;
    }
    // [cli-docs] std's manifest names the wiki as its docs output, so the
    // check runs where that manifest is.
    let out = Command::new(env!("CARGO_BIN_EXE_salvo"))
        .args(["docs", "--check"])
        .current_dir(root.join("lib"))
        .output()
        .expect("run salvo docs --check");
    assert!(
        out.status.success(),
        "wiki/ is out of date — run `salvo docs` in lib/\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// The rule labels the specs define: LANGUAGE_SPEC.md and the backend specs,
/// one `* [label]` (or `- [label]`) opening a rule.
fn defined_labels() -> std::collections::HashSet<String> {
    ["LANGUAGE_SPEC.md", "BACKEND_SPEC.kotlin.md", "BACKEND_SPEC.rust.md"].into_iter().flat_map(labels_of).collect()
}

fn std_sources(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("lib/").filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_dir() {
            std_sources(&path, out);
        } else if path.extension().is_some_and(|e| e == "sv") {
            out.push(path);
        }
    }
}

/// [doc-compilerrules] std's comments cite compiler rules only in an
/// `@compilerrules` tag, which hover and `salvo docs` leave out, or inside a
/// fn body; and every rule `@compilerrules` cites exists. A cited rule is a
/// bracketed label the specs define (`[symbol]` references are not labels,
/// so they are left alone). A test annex is held to its doc blocks only
/// [doc-comment].
#[test]
fn std_docs_cite_rules_only_in_compilerrules() {
    let labels = defined_labels();
    assert!(labels.len() > 500, "found only {} rule labels", labels.len());
    let declaration = |line: &str| {
        let t = line.trim_start();
        let word: String = t.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_').collect();
        const STARTS: &[&str] = &[
            "export", "fn", "struct", "effect", "handler", "qualifier", "type", "platform", "intrinsic", "linear", "iter",
            "comptime", "send", "actor", "fns", "namespace", "rename", "opaque", "threadsafe", "state", "test", "init",
            "monitor", "mixed", "alias", "any",
        ];
        !word.is_empty()
            && (STARTS.contains(&word.as_str())
                || (word.starts_with(|c: char| c.is_ascii_lowercase()) && t[word.len()..].trim_start().starts_with(':')))
    };
    let mut files = Vec::new();
    std_sources(&repo_root().join("lib"), &mut files);
    files.sort();
    let mut inline = Vec::new();
    let mut unknown = Vec::new();
    for file in &files {
        let text = std::fs::read_to_string(file).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        // A module's comments cite rules only inside fn bodies, where they
        // are notes for the compiler's maintainers; a test annex's, only
        // outside its doc blocks.
        let annex = file.to_string_lossy().ends_with(".test.sv");
        let bodies = fn_body_lines(&text);
        let in_body = |line: usize| bodies.iter().any(|(a, b)| *a < line && line < *b);
        let mut i = 0;
        while i < lines.len() {
            if !lines[i].trim_start().starts_with("//") {
                i += 1;
                continue;
            }
            let start = i;
            while i < lines.len() && lines[i].trim_start().starts_with("//") {
                i += 1;
            }
            let documents = i < lines.len() && declaration(lines[i]);
            for (n, line) in lines[start..i].iter().enumerate() {
                let comment = line.trim_start().trim_start_matches('/').trim_start();
                let tag = comment.starts_with("@compilerrules");
                let mut rest = comment;
                while let Some(open) = rest.find('[') {
                    let Some(close) = rest[open..].find(']') else { break };
                    for label in rest[open + 1..open + close].split(',').map(str::trim) {
                        let at = format!("{}:{}", file.strip_prefix(repo_root()).unwrap().display(), start + n + 1);
                        if tag && !labels.contains(label) {
                            unknown.push(format!("{at}: `{label}`"));
                        } else if !tag && labels.contains(label) && if annex { documents } else { !in_body(start + n) } {
                            inline.push(format!("{at}: [{label}]"));
                        }
                    }
                    rest = &rest[open + close + 1..];
                }
            }
        }
    }
    assert!(inline.is_empty(), "std's comments cite rules in their prose, where hover and the wiki show them; move them to `@compilerrules` (inside a fn body a comment may cite them):\n{}", inline.join("\n"));
    assert!(unknown.is_empty(), "`@compilerrules` cites rules no spec defines:\n{}", unknown.join("\n"));
}

/// The 0-based lines that open and close each fn body of `src`, members
/// included: a line strictly between the two is inside the body.
fn fn_body_lines(src: &str) -> Vec<(usize, usize)> {
    use salvo_syntax::ast::{FnDecl, Item};
    let (module, _) = salvo_syntax::parse_module(src);
    let line = |offset: u32| src[..offset as usize].matches('\n').count();
    let mut out = Vec::new();
    let mut add = |fs: &[FnDecl]| {
        for f in fs {
            if let Some(b) = &f.body {
                out.push((line(b.span.start), line(b.span.end)));
            }
        }
    };
    for item in &module.items {
        match item {
            Item::Fn(f) => add(std::slice::from_ref(f)),
            Item::Effect(e) => add(&e.fns),
            Item::Handler(h) => add(&h.fns),
            Item::Fns(g) => add(&g.fns),
            Item::Qualifier(q) => add(&q.fns),
            _ => {}
        }
    }
    out
}

/// [doc-example] Every example in std's doc comments checks: `salvo analyze`
/// in `lib/`, which checks them with std, reports nothing at all.
#[test]
fn std_doc_examples_check() {
    let out = Command::new(env!("CARGO_BIN_EXE_salvo"))
        .args(["analyze", "--format", "json"])
        .current_dir(repo_root().join("lib"))
        .output()
        .expect("run salvo analyze");
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).expect("json");
    let found: Vec<String> = json
        .as_array()
        .unwrap()
        .iter()
        .map(|d| format!("{}:{}: {}", d["file"].as_str().unwrap_or(""), d["line"], d["message"].as_str().unwrap_or("")))
        .collect();
    assert!(found.is_empty() && out.status.success(), "std's doc examples report:\n{}", found.join("\n"));
}

/// [diag-id] Every diagnostic id names a rule a spec defines (its label,
/// before the `:`), and no two messages share an id.
#[test]
fn every_diagnostic_id_names_a_defined_rule() {
    let labels = defined_labels();
    let mut seen = std::collections::HashSet::new();
    let mut unknown = Vec::new();
    for (name, id) in salvo_syntax::messages::ALL {
        assert!(seen.insert(*id), "two messages are `{id}`");
        if !labels.contains(salvo_syntax::messages::label_of(id)) {
            unknown.push(format!("{name}: `{id}`"));
        }
    }
    assert!(unknown.is_empty(), "ids naming rules no spec defines:\n{}", unknown.join("\n"));
}

/// The rule labels one spec file defines, one `* [label]` (or `- [label]`)
/// opening a rule.
fn labels_of(spec: &str) -> std::collections::HashSet<String> {
    let text = std::fs::read_to_string(repo_root().join(spec)).expect(spec);
    let mut out = std::collections::HashSet::new();
    for line in text.lines() {
        let t = line.trim_start();
        let Some(rest) = t.strip_prefix("* [").or_else(|| t.strip_prefix("- [")) else { continue };
        let Some(end) = rest.find(']') else { continue };
        let label = &rest[..end];
        if !label.is_empty() && label.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
            out.insert(label.to_string());
        }
    }
    out
}

/// [diag-id] [backend-core-layers] A backend's diagnostic ids name rules
/// LANGUAGE_SPEC.md or that backend's own spec defines, never the other
/// backend's prefix; and no id is two diagnostics, across the core's list
/// and both backends'.
#[test]
fn every_backend_diagnostic_id_names_a_rule_of_its_backend() {
    let core = labels_of("LANGUAGE_SPEC.md");
    let backends: [(&str, &str, &str, &[(&str, &str)]); 2] = [
        ("kotlin", "BACKEND_SPEC.kotlin.md", "rs-", salvo_backend_kotlin::messages::ALL),
        ("rust", "BACKEND_SPEC.rust.md", "kt-", salvo_backend_rust::messages::ALL),
    ];
    let mut seen: std::collections::HashSet<&str> = salvo_syntax::messages::ALL.iter().map(|(_, id)| *id).collect();
    let mut problems = Vec::new();
    for (backend, spec, foreign, all) in backends {
        let own = labels_of(spec);
        for (name, id) in all {
            let label = salvo_syntax::messages::label_of(id);
            if !core.contains(label) && !own.contains(label) {
                problems.push(format!("{backend} {name}: `{id}` names a rule neither LANGUAGE_SPEC.md nor {spec} defines"));
            }
            if label.starts_with(foreign) {
                problems.push(format!("{backend} {name}: `{id}` names the other backend's rule"));
            }
            if !seen.insert(id) {
                problems.push(format!("{backend} {name}: `{id}` is already another message's id"));
            }
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

/// [doc-compilerknown] Every std declaration the compiler names
/// (`salvo_syntax::known::ALL`) carries `@compilerknown`, every
/// compiler-known module (`known::MODULES`) carries it in its own doc
/// comment, and nothing else in std does: renaming a tagged declaration is
/// the one std change the compiler has to follow.
#[test]
fn std_compiler_known_names_are_tagged() {
    use salvo_syntax::ast::Item;
    use salvo_syntax::known::{KnownKind, ALL, MODULES};
    let tagged = |docs: &[String]| salvo_syntax::doc::split(docs).tag("compilerknown").is_some();
    let lib = repo_root().join("lib");
    let mut files = Vec::new();
    std_sources(&lib, &mut files);
    files.sort();
    // (module, name, kind) → whether each declaration is tagged.
    let mut decls: Vec<(String, String, KnownKind, bool)> = Vec::new();
    let mut modules: Vec<(String, bool)> = Vec::new();
    for file in &files {
        let rel = file.strip_prefix(&lib).unwrap().to_string_lossy().to_string();
        if rel.ends_with(".test.sv") {
            continue;
        }
        let module = rel.trim_end_matches(".sv").replace('/', ".");
        let (ast, _) = salvo_syntax::parse_module(&std::fs::read_to_string(file).unwrap());
        modules.push((module.clone(), tagged(&ast.docs)));
        for item in &ast.items {
            let (name, kind, docs) = match item {
                Item::Fn(f) => (f.name.name.clone(), KnownKind::Fn, &f.docs),
                Item::Struct(s) => (s.name.name.clone(), KnownKind::Type, &s.docs),
                Item::Type(t) => (t.name.name.clone(), KnownKind::Type, &t.docs),
                Item::Effect(e) => (e.name.name.clone(), KnownKind::Type, &e.docs),
                Item::Handler(h) => (h.name.name.clone(), KnownKind::Type, &h.docs),
                Item::Qualifier(q) => (q.name.name.clone(), KnownKind::Type, &q.docs),
                Item::Fns(g) => (g.name.name.clone(), KnownKind::Type, &g.docs),
                _ => continue,
            };
            decls.push((module.clone(), name, kind, tagged(docs)));
        }
    }
    let mut problems = Vec::new();
    for k in ALL {
        let found: Vec<&(String, String, KnownKind, bool)> =
            decls.iter().filter(|(m, n, kind, _)| m == k.module && n == k.name && *kind == k.kind).collect();
        if found.is_empty() {
            problems.push(format!("`{}` in `{}` is listed but not declared there", k.name, k.module));
        }
        for (m, n, _, t) in found {
            if !t {
                problems.push(format!("`{n}` in `{m}` is compiler-known but has no `@compilerknown`"));
            }
        }
    }
    for (m, n, kind, t) in &decls {
        if *t && !ALL.iter().any(|k| k.module == m && k.name == n && k.kind == *kind) {
            problems.push(format!("`{n}` in `{m}` carries `@compilerknown` but `known::ALL` does not list it"));
        }
    }
    for (m, t) in &modules {
        let listed = MODULES.contains(&m.as_str());
        if listed && !t {
            problems.push(format!("module `{m}` is compiler-known but its doc comment has no `@compilerknown`"));
        } else if !listed && *t {
            problems.push(format!("module `{m}` carries `@compilerknown` but `known::MODULES` does not list it"));
        }
    }
    for m in MODULES {
        if !modules.iter().any(|(n, _)| n == m) {
            problems.push(format!("module `{m}` is listed but std has no such module"));
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

/// [cli-docs] `salvo docs` over a small project with a `[docs]` section: a
/// page per module with each exported declaration under its own heading,
/// `[symbol]` links to those headings, the home and topic pages with their
/// links rewritten, the index and the sidebar; `--check` passes on what it
/// wrote and fails once a page changes.
#[test]
fn salvo_docs_writes_wiki_pages() {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("docs_site");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::create_dir_all(root.join("guide")).unwrap();
    std::fs::write(
        root.join("salvo.toml"),
        "[project]\nname = \"shop\"\nversion = \"0.1.0\"\n\n[build]\nsrc = \"src\"\n\n[docs]\nout = \"wiki\"\nhome = \"guide/Start.md\"\ntopics = \"guide\"\n",
    )
    .unwrap();
    std::fs::write(
        root.join("src/shapes.sv"),
        "// Shapes and their areas.\n//\n// Start with [Shape].\n\n// A square, by its side.\n//\n// ```no-check\n// let s = Shape { side: 2 }\n// ```\nexport struct Shape {\n    // The length of a side.\n    side: Int\n}\n\n// The area of [s], a [Shape].\n//\n// ```\n// let x: Str = area(Shape { side: 2 })\n// ```\n//\n// @param s  the shape\n// @compilerrules [fn-overload] hidden\nexport fn area(s: Shape) -> Int {\n    return s.side * s.side\n}\n\nfn hidden() -> Int {\n    return 0\n}\n",
    )
    .unwrap();
    // [actor-message] A send member is rendered as a namespace's fn, with the
    // message struct it generates, which has no entry of its own.
    std::fs::write(
        root.join("src/orders.sv"),
        "// The orders desk. Send one with [Desk.order]; its message is [Desk.Order].\nexport actor effect Desk {\n    // Orders `qty` of `sku`.\n    send fn order(sku: Str, qty: Int) => !sku\n    // Clears the desk.\n    send fn clear()\n}\n",
    )
    .unwrap();
    std::fs::write(root.join("guide/Start.md"), "# Shop\n\nRead [the tour](Tour.md) first.\n").unwrap();
    std::fs::write(root.join("guide/Tour.md"), "# Tour\n\nSee [shapes](../src/shapes.sv#struct-shape).\n\n```salvo\nlet x = 1\n```\n").unwrap();
    let run = |args: &[&str]| Command::new(env!("CARGO_BIN_EXE_salvo")).args(args).current_dir(&root).output().expect("run salvo docs");
    let out = run(&["docs"]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    // [doc-example] What the published examples get wrong is printed.
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("in a doc example: expected `Str`, found `Int`") && stderr.contains("docs: 1 warning in doc examples"), "{stderr}");
    let read = |name: &str| std::fs::read_to_string(root.join("wiki").join(name)).unwrap_or_else(|e| panic!("{name}: {e}"));
    let module = read("shapes.md");
    assert!(module.contains("### struct Shape"), "{module}");
    assert!(module.contains("### fn area"), "{module}");
    assert!(!module.contains("hidden"), "a private fn and `@compilerrules` are left out: {module}");
    assert!(module.contains("Start with [`Shape`](#struct-shape)."), "{module}");
    assert!(module.contains("The area of `s`, a [`Shape`](#struct-shape)."), "{module}");
    assert!(module.contains("```rust\nlet s = Shape { side: 2 }\n```"), "a `no-check` fence is Salvo, fenced `rust` [doc-example]: {module}");
    assert!(module.contains("- `side: Int` — The length of a side."), "{module}");
    assert!(module.contains("**Parameters**"), "{module}");
    let orders = read("orders.md");
    assert!(orders.contains("### fn Desk.order") && orders.contains("send fn order(sku: Str, qty: Int)"), "{orders}");
    assert!(orders.contains("**Message** `Desk.Order`") && orders.contains("- `qty: Int`"), "{orders}");
    assert!(!orders.contains("### struct Desk.Order"), "a message struct has no entry of its own: {orders}");
    assert!(orders.contains("- [`Desk.order`](#fn-deskorder) — Orders `qty` of `sku`."), "the effect links its send members: {orders}");
    assert!(orders.contains("its message is [`Desk.Order`](#fn-deskorder)"), "a message struct's name links to its member: {orders}");
    let home = read("Home.md");
    assert!(home.contains("# Shop") && home.contains("[the tour](Tour)") && home.contains("[`shapes`](shapes)"), "{home}");
    let tour = read("Tour.md");
    assert!(!tour.contains("# Tour") && tour.contains("[shapes](shapes#struct-shape)") && tour.contains("```rust\nlet x = 1"), "{tour}");
    assert!(read("Index.md").contains("| [`area`](shapes#fn-area) | function | [`shapes`](shapes) | The area of `s`, a [`Shape`](shapes#struct-shape). |"));
    assert!(read("_Sidebar.md").contains("* [Tour](Tour)"));
    assert!(run(&["docs", "--check"]).status.success());
    std::fs::write(root.join("wiki/Tour.md"), "edited\n").unwrap();
    let stale = run(&["docs", "--check"]);
    assert!(!stale.status.success() && String::from_utf8_lossy(&stale.stderr).contains("out of date: Tour.md"));
}
