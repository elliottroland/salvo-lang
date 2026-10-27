//! [facts-dump] The pre-IR dumper (PIPELINE.md D1): every declaration of a
//! module with the checker's facts about its body filled in — each
//! expression's type, the declaration a call or a name resolved to, and the
//! coercions, moves and dot-calls the checker recorded. It is the readable
//! form of what the body walk hands the IR builder, the way `salvo ir` is of
//! what the builder hands the backends, so a fact can be looked at (and
//! snapshot-tested) before anything lowers it.
//!
//! One line per expression, in source order, indented under its declaration:
//!
//! ```text
//! fn total(c: C) -> Int
//!   3:9   `sum`                      Int
//!   4:5   `size(xs)`                 Int  -> std.core.list::size
//! ```

use salvo_syntax::ast::{Expr, FnDecl, Item};
use salvo_syntax::visit::{self as v, Visitor};
use salvo_syntax::Span;

use crate::check::Checked;
use crate::decl_id::DeclId;
use crate::Program;

/// Dumps the facts of every declaration in the files [wanted] accepts.
pub fn dump(program: &Program, checked: &Checked, wanted: &dyn Fn(usize) -> bool) -> String {
    let mut out = String::new();
    for (file_idx, (file, module)) in program.files.iter().zip(&program.modules).enumerate() {
        if !wanted(file_idx) {
            continue;
        }
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(&format!("module {} ({})\n", file.module, file.name));
        let source = file.content.as_str();
        for item in &module.items {
            match item {
                Item::Fn(f) => decl(&mut out, program, checked, file_idx, source, "", f),
                Item::Handler(h) => {
                    for f in h.init.iter().chain(&h.fns) {
                        decl(&mut out, program, checked, file_idx, source, &format!("{}.", h.name.name), f);
                    }
                }
                Item::Qualifier(q) => {
                    for f in &q.fns {
                        decl(&mut out, program, checked, file_idx, source, &format!("{}.", q.name.name), f);
                    }
                }
                _ => {}
            }
        }
    }
    out
}

fn decl(out: &mut String, program: &Program, checked: &Checked, file: usize, source: &str, owner: &str, f: &FnDecl) {
    let Some(body) = &f.body else { return };
    let params: Vec<String> = f.params.iter().map(|p| format!("{}: {}", p.name.name, p.ty)).collect();
    let ret = f.return_type.as_ref().map(|t| format!(" -> {t}")).unwrap_or_default();
    out.push_str(&format!("fn {owner}{}({}){ret}\n", f.name.name, params.join(", ")));
    struct Collect(Vec<Span>);
    impl Visitor for Collect {
        fn visit_expr(&mut self, e: &Expr) {
            self.0.push(e.span());
        }
    }
    let mut c = Collect(Vec::new());
    v::walk_block(&mut c, body);
    let mut spans = c.0;
    spans.sort_by_key(|s| (s.expn, s.start, std::cmp::Reverse(s.end)));
    spans.dedup();
    for span in spans {
        let key = (file, span);
        let Some(ty) = checked.expr_ty.get(&key) else { continue };
        // [origin] A copied node's range is its template's: listed at the
        // end of the file, with no text of its own.
        let (line, col) = line_col(source, if span.is_expanded() { source.len() as u32 } else { span.start });
        let mut row = format!("  {:<8}{:<30} {ty}", format!("{line}:{col}"), snippet(source, span));
        if let Some(target) = checked.call_fn.get(&key) {
            row.push_str(&format!("  -> {}", fn_name(program, &checked.decls, *target)));
        } else if let Some(m) = checked.member_calls.get(&key) {
            row.push_str(&format!("  -> {}.#{}", crate::typekey::plain(&m.effect), m.index));
        }
        let mut marks: Vec<&str> = Vec::new();
        if checked.coerce.contains_key(&key) {
            marks.push("coerced");
        }
        if checked.dot_calls.contains(&key) {
            marks.push("dot");
        }
        // [fate-move-mode] A binding's mode is the IR's decision; the
        // checker knows only the loops a projection move makes iterate by
        // value.
        if checked.by_value_loops.contains(&key) {
            marks.push("by-value");
        }
        if checked.linear_moves.contains(&key) {
            marks.push("linear-move");
        }
        if checked.repr_ty.get(&key).is_some_and(|r| r != ty) {
            marks.push("narrowed");
        }
        if !marks.is_empty() {
            row.push_str(&format!("  [{}]", marks.join(", ")));
        }
        out.push_str(row.trim_end());
        out.push('\n');
    }
}

/// `module::name`, with the parameter types when the module declares the
/// name more than once.
fn fn_name(program: &Program, decls: &crate::decl_id::DeclIndex, key: DeclId) -> String {
    let Some(loc) = decls.loc(key) else { return "?".to_string() };
    let (Some(file), Some(module)) = (program.files.get(loc.file), program.modules.get(loc.file)) else {
        return "?".to_string();
    };
    let Some(Item::Fn(f)) = module.items.get(loc.item) else { return "?".to_string() };
    let overloads = module.items.iter().filter(|i| matches!(i, Item::Fn(g) if g.name.name == f.name.name)).count();
    if overloads > 1 {
        let params: Vec<String> = f.params.iter().filter(|p| !p.implicit).map(|p| p.ty.to_string()).collect();
        format!("{}::{}({})", file.module, f.name.name, params.join(", "))
    } else {
        format!("{}::{}", file.module, f.name.name)
    }
}

fn line_col(source: &str, at: u32) -> (usize, usize) {
    let at = (at as usize).min(source.len());
    let before = &source[..at];
    let line = before.matches('\n').count() + 1;
    let col = before.rfind('\n').map(|i| at - i).unwrap_or(at + 1);
    (line, col)
}

/// The source text of a span on one line, shortened.
fn snippet(source: &str, span: Span) -> String {
    let (s, e) = (span.start as usize, (span.end as usize).min(source.len()));
    if span.is_expanded() || s >= e || s >= source.len() {
        // A synthesized node (a desugaring's or a stamp's): no text of its own.
        return "`…`".to_string();
    }
    // A desugaring's synthesized node borrows one byte of its declaration
    // (`desugar::Spans`), usually from inside a longer token.
    let word = |c: Option<char>| c.is_some_and(|c| c.is_alphanumeric() || c == '_');
    if e - s == 1 && (word(source[..s].chars().next_back()) || word(source[e..].chars().next())) {
        return "`…`".to_string();
    }
    let text = &source[s..e];
    let first = text.lines().next().unwrap_or("");
    let mut shown: String = first.chars().take(26).collect();
    if shown.len() < text.len() {
        shown.push('…');
    }
    format!("`{shown}`")
}
