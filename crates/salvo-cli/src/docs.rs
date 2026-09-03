//! Doc comments as LSP hover markdown [doc-comment] [doc-markdown].
//!
//! A declaration's docs are the `//` block directly above it, captured by
//! the parser. This module turns that block into markdown and resolves
//! `[symbol]` references in it [doc-symbol-ref].

use salvo_syntax::ast::{FieldDecl, FnDecl, Item, Module, StructDecl};
use salvo_syntax::Span;

/// Where a `[symbol]` reference can point.
pub struct DocScope<'a> {
    /// Names local to the documented declaration (parameters, generics,
    /// fields) with the span of their own declaration in `file`.
    pub locals: Vec<(String, Span)>,
    /// The file the docs live in, for local links.
    pub file: usize,
    /// That file's text, for rendering source fragments (field defaults).
    pub source: &'a str,
    /// Every module in the program, for type references.
    pub modules: &'a [Module],
    /// Renders a definition site as a markdown link target, or `None`
    /// when it has no addressable location (the embedded std).
    pub link: &'a dyn Fn(usize, Span) -> Option<String>,
}

impl DocScope<'_> {
    /// Resolves a `[symbol]` reference: a local name of the documented
    /// declaration first, then a type/qualifier/effect/handler declared
    /// in the program — the declaring file first, so a local name wins
    /// over a same-named one elsewhere.
    fn resolve(&self, name: &str) -> Option<(usize, Span)> {
        if let Some((_, span)) = self.locals.iter().find(|(n, _)| n == name) {
            return Some((self.file, *span));
        }
        let search = |file: usize| {
            self.modules
                .get(file)?
                .items
                .iter()
                .find_map(|item| decl_name_span(item, name))
                .map(|span| (file, span))
        };
        search(self.file).or_else(|| (0..self.modules.len()).find_map(search))
    }
}

/// The name span of `item` when it declares `name`.
fn decl_name_span(item: &Item, name: &str) -> Option<Span> {
    let ident = match item {
        Item::Struct(s) => &s.name,
        Item::Qualifier(q) => &q.name,
        Item::Effect(e) => &e.name,
        Item::Handler(h) => &h.name,
        Item::Type(t) => &t.name,
        Item::Fn(f) => &f.name,
        _ => return None,
    };
    (ident.name == name).then_some(ident.span)
}

/// Joins a doc block into markdown [doc-markdown]: the lines are already
/// stripped of `//`, so they pass through verbatim (markdown is the
/// format), with `[symbol]` references resolved [doc-symbol-ref].
pub fn render(docs: &[String], scope: &DocScope) -> Option<String> {
    if docs.iter().all(|line| line.trim().is_empty()) {
        return None;
    }
    let body: Vec<String> = docs.iter().map(|line| resolve_refs(line, scope)).collect();
    Some(body.join("\n").trim_end().to_string())
}

/// Rewrites `[symbol]` references in one line [doc-symbol-ref]. A name
/// that resolves becomes a link to its declaration (or inline code when
/// the declaration has no addressable location, as in the embedded std);
/// a name that does not resolve is left exactly as written, so prose
/// containing brackets is never mangled.
fn resolve_refs(line: &str, scope: &DocScope) -> String {
    let mut out = String::with_capacity(line.len());
    let mut rest = line;
    // Inline code spans are markdown, not references: skip over them so
    // `` `[a, b]` `` stays literal.
    while let Some(open) = rest.find(['[', '`']) {
        out.push_str(&rest[..open]);
        rest = &rest[open..];
        if rest.starts_with('`') {
            let end = rest[1..].find('`').map_or(rest.len(), |i| i + 2);
            out.push_str(&rest[..end]);
            rest = &rest[end..];
            continue;
        }
        let Some(close) = rest.find(']') else {
            out.push_str(rest);
            return out;
        };
        let name = &rest[1..close];
        // A markdown link (`[text](url)`) is already a link; leave it be.
        let is_link = rest[close + 1..].starts_with('(');
        match scope.resolve(name).filter(|_| !is_link) {
            Some((file, span)) => match (scope.link)(file, span) {
                Some(target) => out.push_str(&format!("[`{name}`]({target})")),
                None => out.push_str(&format!("`{name}`")),
            },
            None => out.push_str(&rest[..=close]),
        }
        rest = &rest[close + 1..];
    }
    out.push_str(rest);
    out
}

/// The `[symbol]`-resolution scope of a fn: its parameters and generics.
pub fn fn_scope<'a>(
    decl: &FnDecl,
    file: usize,
    source: &'a str,
    modules: &'a [Module],
    link: &'a dyn Fn(usize, Span) -> Option<String>,
) -> DocScope<'a> {
    let mut locals: Vec<(String, Span)> = decl
        .params
        .iter()
        .map(|p| (p.name.name.clone(), p.name.span))
        .collect();
    locals.extend(decl.generics.iter().map(|g| (g.name.clone(), g.span)));
    DocScope {
        locals,
        file,
        source,
        modules,
        link,
    }
}

/// The `[symbol]`-resolution scope of a struct: its fields and generics.
pub fn struct_scope<'a>(
    decl: &StructDecl,
    file: usize,
    source: &'a str,
    modules: &'a [Module],
    link: &'a dyn Fn(usize, Span) -> Option<String>,
) -> DocScope<'a> {
    let mut locals: Vec<(String, Span)> = decl
        .fields
        .iter()
        .map(|f| (f.name.name.clone(), f.name.span))
        .collect();
    locals.extend(decl.generics.iter().map(|g| (g.name.clone(), g.span)));
    DocScope {
        locals,
        file,
        source,
        modules,
        link,
    }
}

/// The **Fields** section of a struct's hover: every field with its type,
/// and its own doc comment when it has one [doc-struct-fields]. Fields
/// are listed in declaration order — including undocumented ones, so the
/// hover shows the whole shape of the struct, not just the annotated part.
pub fn field_section(fields: &[FieldDecl], scope: &DocScope) -> Option<String> {
    if fields.is_empty() {
        return None;
    }
    let mut out = String::from("**Fields**\n");
    for field in fields {
        // The default renders as written in the source [doc-struct-fields].
        let default = field
            .default
            .as_ref()
            .and_then(|e| {
                let span = e.span();
                scope
                    .source
                    .get(span.start as usize..span.end as usize)
                    .map(|text| format!(" = {text}"))
            })
            .unwrap_or_default();
        out.push_str(&format!(
            "\n- `{}: {}{}`",
            field.name.name, field.ty, default
        ));
        if let Some(doc) = render(&field.docs, scope) {
            // Continuation lines are indented into the list item so a
            // multi-line field doc stays part of its bullet.
            let mut lines = doc.lines();
            let first = lines.next().unwrap_or_default();
            out.push_str(&format!(" — {first}"));
            for line in lines {
                out.push('\n');
                if !line.trim().is_empty() {
                    out.push_str(&format!("  {line}"));
                }
            }
        }
    }
    Some(out)
}

/// Assembles a hover body: a `salvo` code block, then the markdown
/// sections, separated by rules.
pub fn hover_markdown(signature: &str, sections: &[Option<String>]) -> String {
    let mut out = format!("```salvo\n{signature}\n```");
    for section in sections.iter().flatten() {
        out.push_str("\n\n---\n\n");
        out.push_str(section);
    }
    out
}
