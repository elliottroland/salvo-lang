//! [facts-dump] The pre-IR dumper (PIPELINE.md D1): the checker's facts,
//! declaration by declaration — each expression's type, the declaration a
//! call or a name resolved to, and the coercions, moves and dot-calls the
//! checker recorded. It is the readable form of what the walks hand the IR
//! builder, the way `salvo ir` is of what the builder hands the backends, so
//! a fact can be looked at (and snapshot-tested) before anything lowers it.
//!
//! [decl-rows] The rows are the checker's own partition by declaration
//! (`Checked::split_decls`): each declaration of a file, in declaration
//! order, under its id [decl-id], with one line per expression it owns, in
//! source order:
//!
//! ```text
//! total(C) -> Int@main
//!   3:9   `sum`                      Int
//!   4:5   `size(xs)`                 Int  -> std.core.list::size
//! ```

use salvo_syntax::ast::Item;
use salvo_syntax::Span;

use crate::check::{Checked, Key};
use crate::decl_id::{DeclId, DeclKind};
use crate::program::Symbols;
use crate::Program;

/// Dumps the facts of every declaration in the files [wanted] accepts.
pub fn dump(program: &Program, symbols: &Symbols<'_>, checked: &Checked, wanted: &dyn Fn(usize) -> bool) -> String {
    let rows = checked.split_decls(program, symbols);
    let mut by_file: Vec<Vec<(DeclId, &Checked)>> = (0..program.files.len()).map(|_| Vec::new()).collect();
    for (id, part) in &rows.parts {
        if let Some(loc) = checked.decls.loc(*id) {
            if loc.file < by_file.len() && !part.expr_ty.is_empty() {
                by_file[loc.file].push((*id, part));
            }
        }
    }
    let mut out = String::new();
    for (file_idx, decls) in by_file.iter_mut().enumerate() {
        if !wanted(file_idx) {
            continue;
        }
        let file = &program.files[file_idx];
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(&format!("module {} ({})\n", file.module, file.name));
        // In declaration order: the file's own nodes first, as they are
        // numbered, then each item, its members after it.
        decls.sort_by_key(|(id, _)| {
            let l = checked.decls.loc(*id).expect("placed above");
            (l.item != usize::MAX, l.item, l.sub)
        });
        for (id, part) in decls.iter() {
            decl(&mut out, program, checked, file_idx, *id, part);
        }
    }
    out
}

fn decl(out: &mut String, program: &Program, checked: &Checked, file: usize, id: DeclId, part: &Checked) {
    let source = program.files[file].content.as_str();
    if id.kind() == DeclKind::File {
        out.push_str(&format!("(file)@{}\n", id.module()));
    } else {
        out.push_str(&format!("{id}\n"));
    }
    let mut keys: Vec<&Key> = part.expr_ty.keys().collect();
    // [node-id] One row per expression, in source order and then walk
    // order: expressions sharing a span (what a desugaring made, at its
    // declaration's span) are each listed, a row repeating the one above
    // once.
    keys.sort_by_key(|k| (k.span.expn, k.span.start, std::cmp::Reverse(k.span.end), k.node));
    let mut last_row = String::new();
    for key in keys {
        let ty = &part.expr_ty[key];
        let span = key.span;
        // [origin] A copied node is listed where a diagnostic about it would
        // land (`origin::landing`): its field or arm, the `by` site, or the
        // written code an in-place copy stands for.
        let at = crate::origin::landing(&program.modules[file], span);
        let (line, col) = line_col(source, if at.is_expanded() { source.len() as u32 } else { at.start });
        let mut row = format!("  {:<8}{:<30} {ty}", format!("{line}:{col}"), snippet(source, at));
        if let Some(target) = part.call_fn.get(key) {
            row.push_str(&format!("  -> {}", fn_name(program, &checked.decls, *target)));
        } else if let Some(m) = part.member_calls.get(key) {
            row.push_str(&format!("  -> {}.#{}", crate::typekey::plain(&m.effect), m.index));
        }
        let mut marks: Vec<&str> = Vec::new();
        if part.coerce.contains_key(key) {
            marks.push("coerced");
        }
        if part.dot_calls.contains(key) {
            marks.push("dot");
        }
        // [fate-move-mode] A binding's mode is the IR's decision; the
        // checker knows only the loops a projection move makes iterate by
        // value.
        if part.by_value_loops.contains(key) {
            marks.push("by-value");
        }
        if part.linear_moves.contains(key) {
            marks.push("linear-move");
        }
        if part.repr_ty.get(key).is_some_and(|r| r != ty) {
            marks.push("narrowed");
        }
        if !marks.is_empty() {
            row.push_str(&format!("  [{}]", marks.join(", ")));
        }
        let row = row.trim_end().to_string();
        if row != last_row {
            out.push_str(&row);
            out.push('\n');
            last_row = row;
        }
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
        // A copied node (a stamp's): no text of its own here.
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
