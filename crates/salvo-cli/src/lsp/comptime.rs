//! Language-server help inside `comptime fn` bodies [lsp-comptime-help].
//!
//! The checker never sees a comptime fn's body as itself (it is stamped, not
//! checked), so there are no types to ask for. What the server knows instead
//! is read from the open file's text and from `std.core.comptime`, which
//! declares the compile-time model (`Struct`, `Field`, `Kind`…) with its docs:
//! hover, completion and go-to-definition below use those declarations
//! rather than words of their own.

use lsp_types::{
    CompletionItem, CompletionItemKind, Documentation, InsertTextFormat, MarkupContent, MarkupKind,
};
use salvo_syntax::ast::{CompKind, FnDecl, Item};
use salvo_syntax::{known, Span};

use super::{docs, span_text, DocView, Server};
use crate::analysis::Analysis;

/// The comptime fn whose text holds `offset`, read from the buffer.
pub(super) fn compfn_at(content: &str, offset: u32) -> Option<FnDecl> {
    let (mut module, _) = salvo_syntax::parse_module(content);
    // [mod-namespace] A namespace's declarations, at top level.
    salvo_syntax::desugar::flatten_namespaces(&mut module);
    module.items.into_iter().find_map(|item| match item {
        Item::Fn(f) if f.compfn.is_some() && f.span.start <= offset && offset <= f.span.end => Some(f),
        _ => None,
    })
}

fn word_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// The index of the `}` matching the `{` at `open` in `text`, ignoring
/// braces in strings and `//` comments; `None` when it is not closed.
fn matching_brace(text: &str, open: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let (mut depth, mut i) = (0i32, open);
    while i < bytes.len() {
        match bytes[i] {
            b'"' => {
                i += 1;
                while i < bytes.len() && bytes[i] != b'"' && bytes[i] != b'\n' {
                    i += if bytes[i] == b'\\' { 2 } else { 1 };
                }
            }
            b'/' if bytes.get(i + 1) == Some(&b'/') => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
                continue;
            }
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// A `[for name in seq]` whose block holds the cursor.
pub(super) struct Binder {
    pub name: String,
    /// The `std.core.comptime` struct it is: `Arm` over `T.arms`, `Index`
    /// over a tuple's `T.indices` [comptime-tuple], `Field` otherwise.
    pub owner: &'static str,
}

/// The binders of `[for …]` blocks enclosing `cursor` in `f`'s body.
pub(super) fn binders_at(content: &str, f: &FnDecl, cursor: usize) -> Vec<Binder> {
    let Some(body) = &f.body else { return Vec::new() };
    let (start, end) = (body.span.start as usize, cursor.min(content.len()));
    if start >= end {
        return Vec::new();
    }
    let region = &content[start..end];
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(p) = region[from..].find("[for ") {
        let at = from + p;
        from = at + 5;
        let Some(close) = region[at + 5..].find(']') else { break };
        let head = &region[at + 5..at + 5 + close];
        let Some((name, seq)) = head.split_once(" in ") else { continue };
        let after_head = at + 5 + close + 1;
        let Some(open) = region[after_head..].find('{').map(|i| after_head + i) else { continue };
        // Still open at the cursor: no matching brace within the region.
        if matching_brace(region, open).is_none() {
            let seq = seq.trim_end();
            let owner = if seq.ends_with(".arms") {
                known::COMPTIME_ARM
            } else if seq.ends_with(".indices") {
                known::COMPTIME_INDEX
            } else {
                known::COMPTIME_FIELD
            };
            out.push(Binder { name: name.trim().to_string(), owner });
        }
    }
    out
}

/// The base type name a `let name … = init` gives a local, as far as the text
/// says: an annotation, a constructor call (`Mut Str("…")`, `List<Int>()`), or
/// a literal.
pub(super) fn local_type(content: &str, f: &FnDecl, cursor: usize, name: &str) -> Option<String> {
    let body = f.body.as_ref()?;
    let (start, end) = (body.span.start as usize, cursor.min(content.len()));
    if start >= end {
        return None;
    }
    let mut found = None;
    for line in content[start..end].lines() {
        let t = line.trim();
        let Some(rest) = t.strip_prefix("let ") else { continue };
        let rest = rest.strip_prefix("mut ").unwrap_or(rest);
        let ident: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
        if ident != name {
            continue;
        }
        let after = rest[ident.len()..].trim_start();
        let (annotation, init) = match after.split_once('=') {
            Some((a, i)) => (a.trim().strip_prefix(':').map(str::trim), i.trim()),
            None => continue,
        };
        found = annotation.and_then(base_type_name).or_else(|| init_type(init));
    }
    found
}

/// The last capitalised word of a type text, without its generics:
/// `Mut List<Int>` is `List`.
fn base_type_name(text: &str) -> Option<String> {
    let head = text.split('<').next()?;
    let word = head.split_whitespace().rfind(|w| w.starts_with(|c: char| c.is_uppercase()))?;
    Some(word.trim_end_matches('?').to_string())
}

fn init_type(init: &str) -> Option<String> {
    if init.starts_with('"') {
        return Some("Str".to_string());
    }
    if init.starts_with(|c: char| c.is_ascii_digit()) {
        let digits: String = init.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_').collect();
        return Some(if digits.ends_with('L') { "Long" } else { "Int" }.to_string());
    }
    if init.starts_with("true") || init.starts_with("false") {
        return Some("Bool".to_string());
    }
    // `Mut Str("…")`, `List<Int>()`: capitalised words, then `(` or `<`.
    let upto = init.find(['(', '<'])?;
    let head = &init[..upto];
    if !head.starts_with(|c: char| c.is_uppercase()) {
        return None;
    }
    head.split_whitespace().rfind(|w| w.starts_with(|c: char| c.is_uppercase())).map(str::to_string)
}

/// The keys a call written at `[start, end)` could be declared under: a
/// type-named constructor is declared under its qualifiers and base joined
/// (`Mut Str(…)` is `Mut_Str`), a plain fn under its name.
pub(super) fn call_keys(content: &str, start: usize, end: usize) -> Vec<String> {
    let word = &content[start..end];
    let mut keys = Vec::new();
    // Qualifiers written before a capitalised base: `Mut NonEmpty List(`.
    if word.starts_with(|c: char| c.is_uppercase()) {
        let mut parts = vec![word.to_string()];
        let mut at = start;
        loop {
            let before = content[..at].trim_end_matches([' ', '\t']);
            if before.len() == at {
                break;
            }
            let wstart = before.rfind(|c: char| !(c.is_alphanumeric() || c == '_')).map(|i| i + 1).unwrap_or(0);
            let w = &before[wstart..];
            if w.is_empty() || !w.starts_with(|c: char| c.is_uppercase()) {
                break;
            }
            parts.insert(0, w.to_string());
            at = wstart;
        }
        for i in 0..parts.len() {
            keys.push(parts[i..].join("_"));
        }
        // The longest first; the bare name is the last.
    } else {
        keys.push(word.to_string());
    }
    keys
}

/// The text after a call's name up to its `(`: an optional `@module` or
/// `@[site]` selector. `Some(selector)` when a call follows.
pub(super) fn call_selector(content: &str, end: usize) -> Option<Option<String>> {
    let rest = content[end..].trim_start_matches([' ', '\t']);
    if rest.starts_with('(') || rest.starts_with('<') {
        return Some(None);
    }
    let rest = rest.strip_prefix('@')?;
    let sel_len = if let Some(inner) = rest.strip_prefix('[') {
        1 + inner.find(']')? + 1
    } else {
        rest.find(|c: char| !(c.is_alphanumeric() || c == '_' || c == '.')).unwrap_or(rest.len())
    };
    let sel = &rest[..sel_len];
    let after = rest[sel_len..].trim_start_matches([' ', '\t']);
    after.starts_with('(').then(|| Some(sel.to_string()))
}

impl Server<'_> {
    /// The file declaring the compile-time model, `std.core.comptime`.
    fn model_file(&self, analysis: &Analysis) -> Option<usize> {
        analysis.program.files.iter().position(|f| f.name.ends_with("core/comptime.sv"))
    }

    /// The struct or `type` of the model named `name`, with its file.
    fn model_decl<'a>(&self, analysis: &'a Analysis, name: &str) -> Option<(usize, &'a Item)> {
        let idx = self.model_file(analysis)?;
        let item = analysis.program.modules.get(idx)?.items.iter().find(|i| match i {
            Item::Struct(s) => s.name.name == name,
            Item::Type(t) => t.name.name == name,
            _ => false,
        })?;
        Some((idx, item))
    }

    /// The doc of `owner` (a model struct) or, with a `member`, of its field,
    /// rendered: the single place the words about a compile-time name live.
    pub(super) fn model_docs(
        &self,
        analysis: &Analysis,
        owner: &str,
        member: &str,
        link: &dyn Fn(usize, Span) -> Option<String>,
    ) -> Option<String> {
        let (idx, item) = self.model_decl(analysis, owner)?;
        let source = &analysis.program.files.get(idx)?.content;
        match item {
            Item::Struct(s) => {
                let scope = docs::struct_scope(s, idx, source, &analysis.program.modules, link);
                if member.is_empty() {
                    docs::render(&s.docs, &scope)
                } else {
                    let field = s.fields.iter().find(|f| f.name.name == member)?;
                    docs::render(&field.docs, &scope)
                }
            }
            _ => None,
        }
    }

    /// Hover for a name of the compile-time model written in a comptime fn
    /// (`Struct`, `Kind`, `Field`…): its declaration and docs.
    pub(super) fn model_word_hover(
        &self,
        analysis: &Analysis,
        word: &str,
        link: &dyn Fn(usize, Span) -> Option<String>,
    ) -> Option<String> {
        let (idx, item) = self.model_decl(analysis, word)?;
        let source = &analysis.program.files.get(idx)?.content;
        let (signature, docs_text) = match item {
            Item::Struct(s) => {
                let scope = docs::struct_scope(s, idx, source, &analysis.program.modules, link);
                (format!("comptime struct {}", s.name.name), docs::render(&s.docs, &scope))
            }
            Item::Type(t) => {
                let scope = docs::DocScope {
                    locals: Vec::new(),
                    file: idx,
                    source,
                    modules: &analysis.program.modules,
                    link,
                };
                (super::type_signature(t), docs::render(&t.docs, &scope))
            }
            _ => return None,
        };
        Some(docs::hover_markdown(&signature, &[docs_text]))
    }

    /// Completion inside a comptime fn's body, when the cursor is in one:
    /// `[` offers the compile-time constructs and the binders; a dot after a
    /// binder, the bound type or a `.type` offers the model's members. `None`
    /// when this is not the place (the ordinary completion goes on).
    pub(super) fn comptime_completion(
        &self,
        analysis: &Analysis,
        text: &str,
        word: usize,
        dot: bool,
    ) -> Option<Vec<CompletionItem>> {
        let f = compfn_at(text, word as u32)?;
        let body = f.body.as_ref()?;
        if !(body.span.start as usize <= word && word <= body.span.end as usize) {
            return None;
        }
        let bytes = text.as_bytes();
        // `[` just typed, at the start of a statement.
        if !dot && word > 0 && bytes[word - 1] == b'[' {
            let line_start = text[..word - 1].rfind('\n').map(|i| i + 1).unwrap_or(0);
            if !text[line_start..word - 1].trim().is_empty() {
                return None;
            }
            let bound = f.compfn.as_ref().and_then(|c| c.bound.as_ref()).map(|(_, id)| id.name.clone()).unwrap_or_else(|| "T".to_string());
            // [comptime-tuple] What the bound kind is walked by.
            let (binder, seq) = match f.compfn.as_ref().and_then(|c| c.bound.as_ref()).map(|(k, _)| *k) {
                Some(CompKind::Tuple) => ("index", "indices"),
                Some(CompKind::Union) => ("arm", "arms"),
                _ => ("field", "fields"),
            };
            let mut items = Vec::new();
            let mut construct = |label: &str, snippet: &str, detail: &str| {
                items.push(CompletionItem {
                    label: label.to_string(),
                    kind: Some(CompletionItemKind::KEYWORD),
                    detail: Some(detail.to_string()),
                    insert_text: Some(if self.snippets { snippet.to_string() } else { snippet.replace("${1:", "").replace("${2:", "").replace("${3:", "").replace('}', "") }),
                    insert_text_format: Some(if self.snippets { InsertTextFormat::SNIPPET } else { InsertTextFormat::PLAIN_TEXT }),
                    sort_text: Some(format!("0{label}")),
                    ..Default::default()
                });
            };
            construct("for", &format!("for ${{1:{binder}}} in ${{2:{bound}.{seq}}}"), "unroll the block once for each field, arm or element");
            construct("when", "when ${1:field.type.kind}", "dispatch at compile time on a kind, a type, a name, or a union value");
            construct("if", "if ${1:condition}", "keep or drop the block at compile time");
            construct("let", "let ${1:name} = ${2:expression}", "a compile-time value for the rest of the block");
            for b in binders_at(text, &f, word) {
                items.push(CompletionItem {
                    label: b.name.clone(),
                    kind: Some(CompletionItemKind::VARIABLE),
                    detail: Some(format!(
                        "splice the {} `{}` is at",
                        match b.owner {
                            known::COMPTIME_ARM => "arm",
                            known::COMPTIME_INDEX => "element",
                            _ => "field",
                        },
                        b.name
                    )),
                    sort_text: Some(format!("1{}", b.name)),
                    ..Default::default()
                });
            }
            return Some(items);
        }
        if !dot {
            return None;
        }
        // The receiver before the dot: `field`, `T`, `field.type`.
        let end = word - 1;
        let mut start = end;
        while start > 0 && (word_char(bytes[start - 1]) || bytes[start - 1] == b'.') {
            start -= 1;
        }
        let chain: Vec<&str> = text[start..end].split('.').collect();
        let owner = match chain.as_slice() {
            [root] => {
                if let Some(b) = binders_at(text, &f, word).iter().rev().find(|b| b.name == *root) {
                    Some(b.owner.to_string())
                } else {
                    f.compfn.as_ref().and_then(|c| c.bound.as_ref()).filter(|(_, id)| id.name == *root).map(|(k, _)| k.word().to_string())
                }
            }
            [root, "type"] if binders_at(text, &f, word).iter().any(|b| b.name == *root) => Some("Type".to_string()),
            _ => None,
        };
        let owner = owner?;
        let (idx, item) = self.model_decl(analysis, &owner)?;
        let Item::Struct(s) = item else { return None };
        let source = &analysis.program.files.get(idx)?.content;
        let link = self.doc_linker(analysis);
        let scope = docs::struct_scope(s, idx, source, &analysis.program.modules, &link);
        Some(
            s.fields
                .iter()
                .enumerate()
                .map(|(i, field)| CompletionItem {
                    label: field.name.name.clone(),
                    kind: Some(CompletionItemKind::FIELD),
                    detail: field.ty.as_ref().map(|t| t.to_string()),
                    documentation: docs::render(&field.docs, &scope)
                        .map(|value| Documentation::MarkupContent(MarkupContent { kind: MarkupKind::Markdown, value })),
                    sort_text: Some(format!("0{i:03}")),
                    ..Default::default()
                })
                .collect(),
        )
    }

    /// The type name of the receiver before a dot in a comptime body, from
    /// the text of the local's `let`: how `json.` finds the `Str` functions.
    pub(super) fn comptime_receiver(&self, text: &str, word: usize) -> Option<String> {
        let f = compfn_at(text, word as u32)?;
        let bytes = text.as_bytes();
        let end = word.checked_sub(1)?;
        let mut start = end;
        while start > 0 && word_char(bytes[start - 1]) {
            start -= 1;
        }
        let name = &text[start..end];
        if name.is_empty() || (start > 0 && bytes[start - 1] == b'.') {
            return None;
        }
        if let Some(t) = local_type(text, &f, word, name) {
            return Some(t);
        }
        // A parameter: its written type's base.
        let param = f.params.iter().find(|p| p.name.name == name)?;
        base_type_name(span_text(text, param.span).split_once(':')?.1)
    }

    /// Where `std.core.comptime` declares `owner` (a model struct or `type`),
    /// or with a `member` its field.
    fn model_location(&self, analysis: &Analysis, owner: &str, member: &str) -> Option<lsp_types::Location> {
        let (idx, item) = self.model_decl(analysis, owner)?;
        let span = match item {
            Item::Struct(s) if !member.is_empty() => s.fields.iter().find(|f| f.name.name == member)?.name.span,
            Item::Struct(s) => s.name.span,
            Item::Type(t) => t.name.span,
            _ => return None,
        };
        let (uri, range) = self.site_location(analysis, idx, span)?;
        Some(lsp_types::Location { uri, range })
    }

    /// Go-to-definition for a call in a comptime body: the fns of that name
    /// in scope (`@module` narrowing them), a type-named constructor included.
    pub(super) fn comptime_definition(
        &self,
        analysis: &Analysis,
        view: &DocView<'_>,
        offset: u32,
    ) -> Vec<lsp_types::Location> {
        let text = view.content;
        let Some(f) = compfn_at(text, offset) else { return Vec::new() };
        let bytes = text.as_bytes();
        let at = offset as usize;
        if at >= bytes.len() || !word_char(bytes[at]) {
            return Vec::new();
        }
        let (mut start, mut end) = (at, at);
        while start > 0 && word_char(bytes[start - 1]) {
            start -= 1;
        }
        while end < bytes.len() && word_char(bytes[end]) {
            end += 1;
        }
        // The compile-time model: a projection (`field.type`, `T.fields`)
        // goes to the member's declaration in `std.core.comptime`, a model
        // type (`Struct`, `Kind`, `Field`…) to its own.
        let member = analysis
            .comptime_hovers
            .iter()
            .filter(|h| h.file == view.file && h.span.holds(offset))
            .min_by_key(|h| h.span.len())
            .and_then(|h| h.doc_of.clone());
        let word = &text[start..end];
        let own_struct = {
            let (mut written, _) = salvo_syntax::parse_module(text);
            // [mod-namespace] A namespace's declarations, at top level.
            salvo_syntax::desugar::flatten_namespaces(&mut written);
            written.items.iter().any(|i| matches!(i, Item::Struct(s) if s.name.name == word))
        };
        let model_target = match member {
            Some((owner, member)) => Some((owner, member)),
            None if word.starts_with(|c: char| c.is_uppercase()) && !own_struct => Some((word.to_string(), String::new())),
            None => None,
        };
        if let Some((owner, member)) = model_target {
            if let Some(location) = self.model_location(analysis, &owner, &member) {
                return vec![location];
            }
        }
        let _ = f;
        let Some(selector) = call_selector(text, end) else { return Vec::new() };
        let Some(by_name) = analysis.overloads.get(view.file) else { return Vec::new() };
        let sites_for = |keys: &[String]| -> Vec<salvo_core::DefSite> { sites_by_keys(analysis, view.file, keys) };
        let _ = by_name;
        let mut sites = sites_for(&call_keys(text, start, end));
        if let Some(sel) = selector.filter(|s| !s.starts_with('[')) {
            let narrowed: Vec<_> = sites
                .iter()
                .filter(|s| {
                    analysis
                        .program
                        .files
                        .get(s.file)
                        .is_some_and(|f| f.module.0.join(".").ends_with(&sel))
                })
                .cloned()
                .collect();
            if !narrowed.is_empty() {
                sites = narrowed;
            }
        }
        sites
            .iter()
            .filter_map(|s| self.site_location(analysis, s.file, s.span))
            .map(|(uri, range)| lsp_types::Location { uri, range })
            .collect()
    }
}

/// The declarations a call's candidate keys name: those in scope in `file`,
/// else any exported one in the program (a std module not imported, whose
/// functions a template reaches by `@module` or by constructor).
pub(super) fn sites_by_keys(analysis: &Analysis, file: usize, keys: &[String]) -> Vec<salvo_core::DefSite> {
    if let Some(by_name) = analysis.overloads.get(file) {
        for key in keys {
            if let Some(found) = by_name.get(key) {
                return found.clone();
            }
        }
    }
    for key in keys {
        let mut found = Vec::new();
        for (idx, module) in analysis.program.modules.iter().enumerate() {
            for item in &module.items {
                if let Item::Fn(f) = item {
                    if f.exported && f.name.name == *key {
                        found.push(salvo_core::DefSite { file: idx, span: f.name.span });
                    }
                }
            }
        }
        if !found.is_empty() {
            return found;
        }
    }
    Vec::new()
}
