//! Doc comments as LSP hover markdown [doc-comment] [doc-markdown].
//!
//! A declaration's docs are the `//` block directly above it, captured by
//! the parser. This module turns that block into markdown and resolves
//! `[symbol]` references in it [doc-symbol-ref].

use salvo_core::refine::RefnGroup;
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
    pub modules: &'a [std::sync::Arc<Module>],
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
/// [doc-tags] The tags follow the prose as sections, in the order the
/// sections are listed here — **Parameters**, **Returns**, **Throws**,
/// **Why not**, **See also**, **To do** — and a tag this version does not
/// know under its own name.
pub fn render(docs: &[String], scope: &DocScope) -> Option<String> {
    if docs.iter().all(|line| line.trim().is_empty()) {
        return None;
    }
    let parts = salvo_syntax::doc::split(docs);
    let mut body: Vec<String> = parts.prose.iter().map(|line| resolve_refs(line, scope)).collect();
    while body.first().is_some_and(|l| l.trim().is_empty()) {
        body.remove(0);
    }
    let sections: [(&str, &str); 6] =
        [("param", "Parameters"), ("returns", "Returns"), ("throws", "Throws"), ("whynot", "Why not"), ("see", "See also"), ("todo", "To do")];
    let mut add = |title: &str, items: Vec<String>| {
        if items.is_empty() {
            return;
        }
        if !body.is_empty() {
            body.push(String::new());
        }
        body.push(format!("**{title}**"));
        body.extend(items);
    };
    for (tag, title) in sections {
        let items: Vec<String> = parts
            .tags
            .iter()
            .filter(|t| t.name == tag)
            .map(|t| match &t.target {
                Some(p) => format!("- `{p}` — {}", resolve_refs(&t.text, scope)),
                None => format!("- {}", resolve_refs(&t.text, scope)),
            })
            .collect();
        add(title, items);
    }
    for t in parts.tags.iter().filter(|t| !sections.iter().any(|(s, _)| *s == t.name)) {
        add(&format!("@{}", t.name), vec![format!("- {}", resolve_refs(&t.text, scope))]);
    }
    if body.iter().all(|l| l.trim().is_empty()) {
        return None;
    }
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
    modules: &'a [std::sync::Arc<Module>],
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
    modules: &'a [std::sync::Arc<Module>],
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
            "\n- `{}: {}{}{}`",
            field.name.name,
            // [state-infer] An inferred state field shows `_` where the type
            // would be — the type is whatever the initializer resolves to.
            match &field.ty {
                Some(t) => t.to_string(),
                None => "_".to_string(),
            },
            if field.canbe_mut { " canbe Mut" } else { "" },
            default
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

/// The **Refinements** section of a function's hover [qual-refn-docs]: what
/// the qualifiers in scope *here* additionally know about this call, with
/// each refinement's own documentation merged in.
///
/// This is the documentation half of the feature. A refinement is written
/// somewhere else entirely — in the qualifier that owns the claim, possibly
/// in another package — so a reader of `add` would otherwise have no way to
/// learn that, in this file, it also establishes `NonEmpty`. Suppressed
/// groups are shown too: a conflict is silent in the type system by design
/// [qual-refn], and this is where it stops being invisible.
pub fn refinement_section(groups: &[RefnGroup], scope: &DocScope) -> Option<String> {
    if groups.is_empty() {
        return None;
    }
    let mut out = String::from("**Refinements** — in scope here:\n");
    for group in groups {
        let mut effects: Vec<String> = group.add.iter().map(|q| format!("+{q}")).collect();
        effects.extend(group.remove.iter().map(|q| format!("-{q}")));
        // [qual-preserve] Part of what a reader needs: the call keeps other
        // values' dependent claims about this parameter alive.
        effects.extend(group.preserve.iter().map(|q| format!("preserve {q}")));
        out.push_str(&format!(
            "\n- `[{}: {}]`",
            group.param,
            effects.join(" ")
        ));
        let from: Vec<String> = group
            .sources
            .iter()
            .map(|s| match &s.qualifier {
                Some(q) => format!("`{q}`"),
                None => "this module".to_string(),
            })
            .collect();
        out.push_str(&format!(" — from {}", from.join(", ")));
        for source in &group.sources {
            if let Some(doc) = render(&source.docs, scope) {
                for line in doc.lines() {
                    out.push('\n');
                    if !line.trim().is_empty() {
                        out.push_str(&format!("  {line}"));
                    }
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

#[cfg(test)]
mod tests {
    use super::*;

    /// [doc-tags] The tags follow the prose as sections, `[name]`s resolved
    /// as anywhere in a doc block, and an unknown tag under its own name.
    #[test]
    fn tags_render_as_sections_after_the_prose() {
        let link = |_: usize, _: Span| -> Option<String> { None };
        let scope = DocScope { locals: vec![("line".to_string(), Span::default())], file: 0, source: "", modules: &[], link: &link };
        let docs: Vec<String> = [
            "The words of a command line.",
            "",
            "@param line  the command line; quotes group",
            "             words",
            "@returns the words of [line]",
            "@frob something",
            "@todo the empty case",
        ]
        .iter()
        .map(|l| l.to_string())
        .collect();
        let shown = render(&docs, &scope).unwrap();
        assert_eq!(
            shown,
            "The words of a command line.\n\n**Parameters**\n- `line` — the command line; quotes group words\n\n**Returns**\n- the words of `line`\n\n**To do**\n- the empty case\n\n**@frob**\n- something"
        );
    }
}
