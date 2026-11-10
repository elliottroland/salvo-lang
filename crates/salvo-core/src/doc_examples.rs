//! [doc-example] Code examples in doc comments, checked with the program.
//!
//! A fenced block in a doc comment (a declaration's or the module's) whose
//! info string is empty or `salvo` becomes a file of its own: a hidden
//! module the analysis parses, resolves and checks with everything else,
//! whose diagnostics are reported, as warnings, at the comment lines the
//! code came from. ```` ```no-check ```` is Salvo that is not checked; any
//! other info string (`text`, `toml`, …) is not Salvo.
//!
//! * A fence of **statements** is wrapped in `fn example() [use, Console] {
//!   … }`, with its leading `import` lines kept above the wrapper. A fence
//!   with a declaration at its left edge is a file as written, and one with a
//!   `test` block is a test annex.
//! * The example **sees** `std.core.*`, its own imports and the module it
//!   documents: as a whole-module import (the exported names a reader can
//!   use) for the module doc and an exported declaration's, and whole, private
//!   names included, for a non-exported declaration's. It stays a module of
//!   its own either way, so what it declares never joins the documented one
//!   (`resolve.rs`).
//! * Its **module path** is the documented module's plus one segment no file
//!   can spell: `#<owner>.<n>`, the documented declaration's name path
//!   (`module` for the module doc) and the fence's place among that name's
//!   examples. It does not move when another example is added elsewhere, so
//!   an unedited example keeps its parse (the cache is keyed by text and
//!   module) and its declarations keep their ids [decl-id].

use std::sync::Arc;

use salvo_syntax::ast::{FnDecl, Item, Module};
use salvo_syntax::Span;

use crate::source::{ModulePath, SourceFile};

/// [doc-example] What a file made from a doc example came from.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DocExample {
    /// The file whose doc comment holds the example (an index into the
    /// program's files).
    pub host: usize,
    /// The module the example documents, which it sees.
    pub documents: ModulePath,
    /// Whether it sees the documented module's private names: an example on
    /// a non-exported declaration.
    pub own: bool,
    /// Per line of the example's text, the host byte offset its column 0
    /// stands for. A wrapper line (the `fn example` around statements)
    /// stands for the fence's opening line.
    pub lines: Vec<u32>,
    /// Where the fence's opening line starts in the host, for a position
    /// the line map cannot place.
    pub fence: u32,
}

/// The segment that opens an example module's own part (`#map.0`).
pub const SEGMENT_MARK: char = '#';

/// The words that make a fence a file of declarations when a line opens
/// with one at the fence's left edge.
const DECLARATION_WORDS: &[&str] = &[
    "fn", "struct", "linear", "effect", "actor", "handler", "type", "qualifier", "provenance", "fns", "iter",
    "namespace", "platform", "intrinsic", "send", "comptime", "threadsafe", "iterable", "opaque", "export",
    "effects", "refn", "rename", "noremote", "mixed",
];

impl DocExample {
    /// The host offset for an offset into the example's text `content`.
    pub fn host_offset(&self, content: &str, offset: u32) -> u32 {
        let before = &content[..(offset as usize).min(content.len())];
        let line = before.matches('\n').count();
        let line_start = before.rfind('\n').map_or(0, |i| i + 1) as u32;
        match self.lines.get(line) {
            Some(base) if *base != u32::MAX => base + (offset - line_start),
            _ => self.fence,
        }
    }

    /// The offset into the example's text `content` for a host offset, when
    /// the host offset is on one of the example's code lines.
    pub fn example_offset(&self, content: &str, host: u32) -> Option<u32> {
        let mut start = 0u32;
        for (line, text) in content.split('\n').enumerate() {
            let base = self.lines.get(line).copied().unwrap_or(u32::MAX);
            if base != u32::MAX && base <= host && host <= base + text.len() as u32 {
                return Some(start + (host - base));
            }
            start += text.len() as u32 + 1;
        }
        None
    }

    /// The span in the host for a span of the example's text.
    pub fn host_span(&self, content: &str, span: Span) -> Span {
        if !span.is_written() {
            return Span { start: self.fence, end: self.fence, ..span };
        }
        let start = self.host_offset(content, span.start);
        // An end on another line than the start is clamped to the start's
        // line: the host's text between two code lines is comment markers.
        let end = self.host_offset(content, span.end.max(span.start));
        let same_line = |a: u32, b: u32| self.lines.iter().any(|base| *base != u32::MAX && *base <= a)
            && line_of(&self.lines, a) == line_of(&self.lines, b);
        Span { start, end: if same_line(start, end) { end } else { start }, ..span }
    }
}

/// [doc-example] Reports what the examples' files found at the comments
/// they came from: an error becomes a warning there, keeping its id, and an
/// example's own warnings (a binding shown and never used, mostly) are
/// dropped. Every other diagnostic is left as it is.
pub fn report(diagnostics: &mut Vec<crate::FileDiagnostic>, files: &[SourceFile]) {
    diagnostics.retain(|d| files.get(d.file).is_none_or(|f| f.doc_example.is_none()) || d.is_error());
    for d in diagnostics.iter_mut() {
        let Some(file) = files.get(d.file) else { continue };
        let Some(doc) = &file.doc_example else { continue };
        d.severity = salvo_syntax::diag::Severity::Warning;
        d.doc_example = true;
        d.message = salvo_syntax::messages::in_doc_example(&d.message);
        d.span = doc.host_span(&file.content, d.span);
        d.file = doc.host;
        d.suggested_imports.clear();
    }
}

fn line_of(lines: &[u32], host: u32) -> Option<usize> {
    lines.iter().enumerate().filter(|(_, b)| **b != u32::MAX && **b <= host).map(|(i, _)| i).last()
}

/// [doc-example] The example files of `file` (index `host` in the program),
/// from its text and its parsed module.
pub fn extract(file: &SourceFile, host: usize, module: &Module) -> Vec<SourceFile> {
    let text = &file.content;
    let line_starts: Vec<u32> = std::iter::once(0)
        .chain(text.match_indices('\n').map(|(i, _)| i as u32 + 1))
        .collect();
    let line_of = |offset: u32| match line_starts.binary_search(&offset) {
        Ok(l) => l,
        Err(l) => l - 1,
    };
    // The declarations a doc block can sit above, by the line they open on.
    let mut owners: Vec<(usize, String, bool)> = Vec::new();
    {
        let mut add = |span: Span, name: String, exported: bool| {
            if span.is_written() {
                owners.push((line_of(span.start), name, exported));
            }
        };
        let fns = |container: &str, fs: &[FnDecl], exported: bool, add: &mut dyn FnMut(Span, String, bool)| {
            for f in fs {
                add(f.name.span, format!("{container}.{}", f.name.name), exported);
            }
        };
        for item in &module.items {
            match item {
                Item::Fn(f) => add(f.name.span, f.name.name.clone(), f.exported),
                Item::Struct(s) => {
                    add(s.name.span, s.name.name.clone(), s.exported);
                    for field in &s.fields {
                        add(field.name.span, format!("{}.{}", s.name.name, field.name.name), s.exported);
                    }
                    fns(&s.name.name, &s.fns, s.exported, &mut add);
                }
                Item::Type(t) => add(t.name.span, t.name.name.clone(), t.exported),
                Item::Qualifier(q) => {
                    add(q.name.span, q.name.name.clone(), q.exported);
                    fns(&q.name.name, &q.fns, q.exported, &mut add);
                }
                Item::Effect(e) => {
                    add(e.name.span, e.name.name.clone(), e.exported);
                    fns(&e.name.name, &e.fns, e.exported, &mut add);
                }
                Item::Handler(h) => {
                    add(h.name.span, h.name.name.clone(), h.exported);
                    fns(&h.name.name, &h.fns, h.exported, &mut add);
                }
                Item::Fns(g) => {
                    add(g.name.span, g.name.name.clone(), g.exported);
                    fns(&g.name.name, &g.fns, g.exported, &mut add);
                }
                Item::EffectAlias(a) => add(a.name.span, a.name.name.clone(), a.exported),
                _ => {}
            }
        }
    }
    let lines: Vec<&str> = text.split('\n').collect();
    let comment = |l: &str| l.trim_start().starts_with("//");
    let mut out = Vec::new();
    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut i = 0;
    let mut first_run = true;
    while i < lines.len() {
        if !comment(lines[i]) {
            i += 1;
            continue;
        }
        let start = i;
        while i < lines.len() && comment(lines[i]) {
            i += 1;
        }
        let end = i; // the line after the run
        let module_doc = first_run && !module.docs.is_empty();
        first_run = false;
        let owner = if module_doc {
            Some(("module".to_string(), true))
        } else {
            owners.iter().find(|(l, _, _)| *l == end).map(|(_, n, e)| (n.clone(), *e))
        };
        let Some((owner, exported)) = owner else { continue };
        for fence in fences(&lines, start, end) {
            let n = counts.entry(owner.clone()).or_insert(0);
            let ordinal = *n;
            *n += 1;
            let Some((content, map, is_test)) = program(&fence, &line_starts) else { continue };
            let mut path = file.module.0.clone();
            path.push(format!("{SEGMENT_MARK}{owner}.{ordinal}"));
            if is_test {
                path.push(crate::source::TEST_SUFFIX.trim_start_matches('.').to_string());
            }
            out.push(SourceFile {
                name: format!("{}{SEGMENT_MARK}{owner}.{ordinal}", file.name),
                module: ModulePath(path),
                content,
                is_std: false,
                is_shadow: false,
                is_test,
                dependency: None,
                doc_example: Some(Arc::new(DocExample {
                    host,
                    documents: file.module.clone(),
                    own: !exported,
                    lines: map,
                    fence: line_starts[fence.open] + fence.indent,
                })),
            });
        }
    }
    out
}

/// One fence of a comment run: its opening line, and each code line with
/// the host offset its code starts at.
struct Fence {
    open: usize,
    indent: u32,
    code: Vec<(String, u32)>,
}

/// The checked fences of the comment lines `start..end`: an info string of
/// nothing or `salvo`. A fence that never closes is not an example.
fn fences(lines: &[&str], start: usize, end: usize) -> Vec<Fence> {
    let mut out = Vec::new();
    let mut i = start;
    while i < end {
        let (indent, body) = strip(lines[i]);
        let ticks = body.chars().take_while(|c| *c == '`').count();
        if ticks < 3 {
            i += 1;
            continue;
        }
        let info = body[ticks..].trim();
        let open = i;
        let mut code = Vec::new();
        i += 1;
        let mut closed = false;
        while i < end {
            let (prefix, line) = strip(lines[i]);
            if line.trim_start().starts_with(&"`".repeat(ticks)) && line.trim().chars().all(|c| c == '`') {
                closed = true;
                i += 1;
                break;
            }
            code.push((line.to_string(), prefix));
            i += 1;
        }
        if closed && (info.is_empty() || info == "salvo") {
            out.push(Fence { open, indent, code });
        }
    }
    out
}

/// A comment line's text after `//` and one space, with the byte width of
/// what was cut (indentation, `//`, the space).
fn strip(line: &str) -> (u32, &str) {
    let lead = line.len() - line.trim_start().len();
    let rest = &line[lead + 2..];
    let rest_cut = if rest.starts_with(' ') { 1 } else { 0 };
    ((lead + 2 + rest_cut) as u32, &rest[rest_cut..])
}

/// The example file for `fence`: its text, its line map, and whether it is
/// a test annex. `None` for an empty fence.
fn program(fence: &Fence, line_starts: &[u32]) -> Option<(String, Vec<u32>, bool)> {
    if fence.code.iter().all(|(l, _)| l.trim().is_empty()) {
        return None;
    }
    let host = |k: usize| line_starts[fence.open + 1 + k] + fence.code[k].1;
    let opens = |l: &str, word: &str| l.strip_prefix(word).is_some_and(|r| r.is_empty() || r.starts_with(' ') || r.starts_with('<') || r.starts_with('('));
    let is_test = fence.code.iter().any(|(l, _)| opens(l, "test"));
    let declarations = is_test || fence.code.iter().any(|(l, _)| DECLARATION_WORDS.iter().any(|w| opens(l, w)));
    let mut text = String::new();
    let mut map = Vec::new();
    if declarations {
        for (k, (l, _)) in fence.code.iter().enumerate() {
            text.push_str(l);
            text.push('\n');
            map.push(host(k));
        }
        return Some((text, map, is_test));
    }
    // Statements: the leading imports (and the blank and comment lines among
    // them) stay above the wrapper, which stands for the fence's own line.
    let lead = fence
        .code
        .iter()
        .take_while(|(l, _)| { let t = l.trim(); t.is_empty() || t.starts_with("//") || opens(l, "import") })
        .count();
    for (k, (l, _)) in fence.code.iter().enumerate().take(lead) {
        text.push_str(l);
        text.push('\n');
        map.push(host(k));
    }
    text.push_str("fn example() [use, Console] {\n");
    map.push(u32::MAX);
    for (k, (l, _)) in fence.code.iter().enumerate().skip(lead) {
        text.push_str(l);
        text.push('\n');
        map.push(host(k));
    }
    text.push_str("}\n");
    map.push(u32::MAX);
    Some((text, map, false))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(content: &str) -> (SourceFile, Module) {
        let f = SourceFile {
            name: "shapes.sv".into(),
            module: ModulePath::parse("shapes"),
            content: content.into(),
            is_std: false,
            is_shadow: false,
            is_test: false,
            dependency: None,
            doc_example: None,
        };
        let (m, _) = salvo_syntax::parse_module(content);
        (f, m)
    }

    /// [doc-example] A statement fence is wrapped, keeps its imports above
    /// the wrapper, and maps every code line to its comment line.
    #[test]
    fn a_statement_fence_is_wrapped_and_mapped() {
        let src = "// Doubles.\n//\n// ```\n// import std.heap\n// let x = double(2)  // 4\n// ```\nexport fn double(n: Int) -> Int {\n    return n * 2\n}\n";
        let (f, m) = file(src);
        let ex = extract(&f, 7, &m);
        assert_eq!(ex.len(), 1);
        let e = &ex[0];
        assert_eq!(e.module.to_string(), "shapes.#double.0");
        assert_eq!(e.content, "import std.heap\nfn example() [use, Console] {\nlet x = double(2)  // 4\n}\n");
        let d = e.doc_example.as_ref().unwrap();
        assert!(!d.own && d.host == 7);
        let at = e.content.find("double(2)").unwrap() as u32;
        assert_eq!(&src[d.host_offset(&e.content, at) as usize..][..9], "double(2)");
        assert_eq!(d.example_offset(&e.content, src.find("double(2)").unwrap() as u32), Some(at));
    }

    /// [doc-example] Declarations are a file as written; a `test` block makes
    /// an annex; `no-check`, other languages, free comments and a private
    /// declaration's examples are told apart.
    #[test]
    fn fences_are_told_apart() {
        let src = "// The module.\n//\n// ```\n// let a = 1\n// ```\n\n// Free text.\n// ```\n// let b = 2\n// ```\n\n// ```no-check\n// nope\n// ```\n// ```text\n// prose\n// ```\n// ```\n// struct P {}\n// ```\n// ```\n// test \"t\" {\n// }\n// ```\nfn helper() -> Int {\n    return 1\n}\n";
        let (f, m) = file(src);
        let ex = extract(&f, 0, &m);
        let names: Vec<String> = ex.iter().map(|e| e.module.to_string()).collect();
        assert_eq!(names, ["shapes.#module.0", "shapes.#helper.0", "shapes.#helper.1.test"]);
        assert!(!ex[0].doc_example.as_ref().unwrap().own);
        assert!(ex[1].doc_example.as_ref().unwrap().own);
        assert_eq!(ex[1].content, "struct P {}\n");
        assert!(ex[2].is_test);
    }
}
