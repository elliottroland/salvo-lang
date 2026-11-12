//! [cli-fmt] The formatter: a file parsed as written [syntax-lowered] and
//! printed back [ast-print], its imports sorted [fmt-imports]. `salvo fmt`
//! and the language server's document formatting both call [`format`].

use std::collections::BTreeMap;

use crate::diag::Diagnostic;

/// Why a file was not formatted.
#[derive(Debug)]
pub enum FormatError {
    /// The file does not parse: its errors, and nothing is printed.
    Parse(Vec<Diagnostic>),
    /// The printing does not parse, or does not hold the file's comments as
    /// written: a printer defect, so the file is left as it is.
    Unsafe(String),
}

/// [cli-fmt] `source` formatted: parsed (nothing checked or expanded),
/// printed, its imports sorted. The printing is refused, not returned, when
/// it does not parse back to the same tree (spans, parentheses and the
/// imports' order aside) or loses or repeats a comment: the guarantees the
/// printer's tests hold over every file, checked again on each.
pub fn format(source: &str) -> Result<String, FormatError> {
    let (module, diagnostics) = crate::parse_module(source);
    let errors: Vec<Diagnostic> = diagnostics.into_iter().filter(Diagnostic::is_error).collect();
    if !errors.is_empty() {
        return Err(FormatError::Parse(errors));
    }
    let printed = sort_imports(&crate::print::print_module(&module).text);
    verify(source, module, &printed)?;
    Ok(printed)
}

/// [cli-fmt] Whether `printed` may replace `source`, whose tree is
/// `module`: it parses, to the same tree, and holds the same comments.
fn verify(source: &str, module: crate::ast::Module, printed: &str) -> Result<(), FormatError> {
    let (again, diagnostics) = crate::parse_module(printed);
    if let Some(error) = diagnostics.iter().find(|d| d.is_error()) {
        return Err(FormatError::Unsafe(format!("its formatting does not parse: {}", error.message)));
    }
    if shape(module) != shape(again) {
        return Err(FormatError::Unsafe("its formatting would parse to a different tree".to_string()));
    }
    if comment_texts(source) != comment_texts(printed) {
        return Err(FormatError::Unsafe("its formatting would not keep its comments as written".to_string()));
    }
    Ok(())
}

/// A module's tree as its debug form with the imports in sorted order and
/// every span left out, with the parentheses counted on it [expr-group].
fn shape(mut module: crate::ast::Module) -> String {
    use crate::ast::Item;
    let at: Vec<usize> = (0..module.items.len()).filter(|&i| matches!(module.items[i], Item::Import(_))).collect();
    let mut imports: Vec<Item> = at.iter().map(|&i| module.items[i].clone()).collect();
    let key = |item: &Item| match item {
        Item::Import(i) => (i.path.iter().map(|p| p.name.to_string()).collect::<Vec<_>>(), i.alias.as_ref().map(|a| a.name.to_string())),
        _ => Default::default(),
    };
    imports.sort_by_key(key);
    for (&i, item) in at.iter().zip(imports) {
        module.items[i] = item;
    }
    without_spans(&format!("{module:?}"))
}

/// `text` with each span's debug form (`12..15`, `12..15#3`, inside the
/// parentheses counted on it) written `~`. Of the parentheses around one,
/// those matched on both sides go with it, so `Some(12..15)` and
/// `Some(((12..15)))` read alike.
fn without_spans(text: &str) -> String {
    let b = text.as_bytes();
    let digits = |mut i: usize| {
        let s = i;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        (i > s).then_some(i)
    };
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < b.len() {
        let mut j = i;
        while j < b.len() && b[j] == b'(' {
            j += 1;
        }
        let open = j - i;
        let span_end = digits(j).filter(|&k| b[k..].starts_with(b"..")).and_then(|k| digits(k + 2)).map(|k| match b.get(k) {
            Some(b'#') => digits(k + 1).unwrap_or(k),
            _ => k,
        });
        let preceded = i > 0 && (b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'.' || b[i - 1] == b'_');
        match span_end {
            Some(mut k) if !(open == 0 && preceded) => {
                let mut close = 0;
                while close < open && k < b.len() && b[k] == b')' {
                    close += 1;
                    k += 1;
                }
                out.push_str(&text[i..i + open - close]);
                out.push('~');
                i = k;
            }
            _ => {
                let c = text[i..].chars().next().unwrap_or(' ');
                out.push(c);
                i += c.len_utf8();
            }
        }
    }
    out
}

/// Each comment's text, as written, counted.
fn comment_texts(source: &str) -> BTreeMap<&str, usize> {
    let mut out = BTreeMap::new();
    for c in crate::lexer::lex(source).comments {
        *out.entry(&source[c.span.start as usize..c.span.end as usize]).or_insert(0) += 1;
    }
    out
}

/// [fmt-imports] [lsp-organize-imports] `text` with its top-level imports
/// sorted, each with the comments written directly above it, and the blank
/// lines between them removed; everything else as written. The run sorted
/// is from the first top-level import line to the last, with the comment
/// lines directly above the first unless they are the module's docs
/// [doc-module]. It is left as written when it holds anything but imports,
/// their comments and blank lines, or a comment with a blank line after it
/// (a heading over a group, which belongs to no one import).
///
/// The same holds when the sort would bring a comment to the top of a file
/// that has nothing above its imports, since there it would become the
/// module's docs.
///
/// Purely textual: an import is one line, `import a.b [as c]` from the
/// line's start, and imports sort by that line without its trailing
/// comment. Nothing is removed. Organize-imports calls it after removing
/// what it removes.
pub fn sort_imports(text: &str) -> String {
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let is_import = |l: &str| l.starts_with("import ");
    let is_comment = |l: &str| l.trim_start().starts_with("//");
    let is_blank = |l: &str| l.trim().is_empty();
    let (Some(first), Some(last)) = (lines.iter().position(|l| is_import(l)), lines.iter().rposition(|l| is_import(l))) else {
        return text.to_string();
    };
    // The comments directly above the first import are its own, unless only
    // blank lines precede them: then they are the module's docs.
    let mut start = first;
    while start > 0 && is_comment(lines[start - 1]) {
        start -= 1;
    }
    let docs_above = start < first && lines[..start].iter().all(|l| is_blank(l));
    if docs_above {
        start = first;
    }
    let mut entries: Vec<Vec<&str>> = Vec::new();
    let mut pending: Vec<&str> = Vec::new();
    for &line in &lines[start..=last] {
        if is_import(line) {
            pending.push(line);
            entries.push(std::mem::take(&mut pending));
        } else if is_comment(line) {
            pending.push(line);
        } else if !is_blank(line) || !pending.is_empty() {
            return text.to_string();
        }
    }
    let key = |entry: &Vec<&str>| entry.last().copied().unwrap_or("").split("//").next().unwrap_or("").trim().to_string();
    entries.sort_by_key(key);
    let mut out: String = lines[..start].concat();
    if entries.first().is_some_and(|e| e.len() > 1) {
        if docs_above {
            // A comment now first in the run would join the module's docs.
            out.push('\n');
        } else if lines[..start].iter().all(|l| is_blank(l)) {
            // It would become the module's docs [doc-module].
            return text.to_string();
        }
    }
    for line in entries.iter().flatten() {
        out.push_str(line);
        if !line.ends_with('\n') {
            out.push('\n');
        }
    }
    out.push_str(&lines[last + 1..].concat());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // [cli-fmt] The check a printing passes before it may replace a file:
    // one that only moves the layout, the parentheses or the imports'
    // order passes; one that does not parse, parses to another tree (a
    // literal respelled, the docs changed: the docs are part of the tree),
    // or loses or repeats a comment is refused, each for its reason. A span
    // reads alike with any parentheses counted on it. What the formatter
    // writes is the print fixtures' (`salvo-core/tests/print/`).
    #[test]
    fn a_printing_that_changes_the_file_is_refused() {
        let source = "import b\nimport a\n\n// f.\nfn f() -> Int {\n    return 1_000 // one thousand\n}\n";
        let cases: &[(&str, &str, Option<&str>)] = &[
            ("layout, parentheses and imports moved", "import a\nimport b\n\n// f.\nfn f() -> Int {\n  return (1_000) // one thousand\n}\n", None),
            ("blank lines between the imports", "import a\n\nimport b\n\n// f.\nfn f( ) -> Int {\n    return 1_000 // one thousand\n}\n", None),
            ("does not parse", "import a\nimport b\n\n// f.\nfn f() -> Int {\n    return 1_000 // one thousand\n", Some("does not parse")),
            ("a literal respelled", "import a\nimport b\n\n// f.\nfn f() -> Int {\n    return 1000 // one thousand\n}\n", Some("different tree")),
            ("another value", "import a\nimport b\n\n// f.\nfn f() -> Int {\n    return 2_000 // one thousand\n}\n", Some("different tree")),
            ("the docs changed", "import a\nimport b\n\n// f.\n\nfn f() -> Int {\n    return 1_000 // one thousand\n}\n", Some("different tree")),
            ("a comment lost", "import a\nimport b\n\n// f.\nfn f() -> Int {\n    return 1_000\n}\n", Some("comments")),
            ("a comment repeated", "import a\nimport b\n\n// f.\nfn f() -> Int {\n    // one thousand\n    // one thousand\n    return 1_000\n}\n", Some("comments")),
        ];
        let mut wrong = Vec::new();
        for (case, printed, refused) in cases {
            match (verify(source, crate::parse_module(source).0, printed), refused) {
                (Ok(()), None) => {}
                (Err(FormatError::Unsafe(why)), Some(want)) if why.contains(want) => {}
                (got, _) => wrong.push(format!("{case}: expected {}, got {got:?}", refused.map_or("a pass".to_string(), |w| format!("a refusal for `{w}`")))),
            }
        }
        for (written, without) in [("Some(12..15), (3..4#2, x), ((7..9)), a1..2", "Some~, (~, x), ~, a1..2"), ("Some(((12..15))), (((3..4#2)), x)", "Some~, (~, x)")] {
            if without_spans(written) != without {
                wrong.push(format!("spans of `{written}`: `{}`, expected `{without}`", without_spans(written)));
            }
        }
        assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    }
}
