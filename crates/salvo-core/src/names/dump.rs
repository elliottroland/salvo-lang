//! [dump-resolve] The resolve phase's dump (`salvo dump --phase resolve`,
//! the pipeline record C2, P7, P22; user decision 2026-11-09): each file's
//! tree as resolution reads it — expanded and desugared, printed back as
//! Salvo [ast-print] — with what resolution bound every name to spelled in
//! place, placed by the printer's node map.
//!
//! Salvo where Salvo can spell it, `#{…}` where it cannot [dump-spelling]:
//! - a name meaning one declaration takes its module, `Point@app.shapes` —
//!   the declaration's own name where an import renamed it — and a callee
//!   or fn value its selector, `show@app.svc` or `greet@Greeter`; a name
//!   whose every declaration is std's (a type's key plain) stays bare;
//! - a name meaning several (a qualifier over several subjects, an
//!   overloaded fn across modules) lists them: `#{@std.core.list,
//!   @std.core.sorted}`, the first three unless every one is asked for;
//! - a variable names the line and column of its binding: `p#{32:9}`;
//! - a selector written as a type gets its module, `cmp@Person#{app.main}`;
//! - a leading-dot name is the type phase's, `#{deferred}`; a name nothing
//!   answers, `#{???}`.
//!
//! Generics and built-ins are left as they are; a name an expansion wrote
//! (a stamped body's, a hidden fn's) is spelled like any other.

use std::collections::HashSet;

use salvo_syntax::print::{node_key, NodeKey, Placed};
use salvo_syntax::Span;

use super::{Meaning, ValueRole};
use crate::program::{Program, Symbols};
use crate::resolve::Resolution;

/// How many of a name's declarations are listed before a count.
const SHOWN: usize = 3;

/// One edit of the file's text: `text` in place of `start..end`.
struct Edit {
    start: u32,
    end: u32,
    text: String,
}

/// One declaration a name may mean: its name, its module, and whether the
/// dump leaves it bare.
struct Decl {
    name: String,
    module: String,
    bare: bool,
}

/// [dump-resolve] File `file` of `program` — its expanded tree, printed
/// [ast-print] — with its names' resolutions spelled in (every candidate
/// listed when `all`). A variable's binding is placed by its line and
/// column in the printed text.
pub fn dump_file(program: &Program, resolution: &Resolution<'_>, symbols: &Symbols<'_>, file: usize, all: bool) -> String {
    let printed = crate::printed::print_file(program, file);
    let names = &resolution.names;
    // A name the printing wrote as it is spelled (not a hole's `_`, not a
    // type-named call's key).
    let placed = |span: Span, name: &str| -> Option<Placed> { printed.at(span).filter(|p| printed.text.get(p.start as usize..p.end as usize) == Some(name)) };
    // A name that declares (an `iter fn`'s `state Name`, which names the
    // struct the expansion writes) is no reference.
    let declared: HashSet<NodeKey> = declared_names(&program.modules[file]);
    // [qual-depend] A value block's names are values (places, fn
    // identities), spelled by the value pass.
    let as_values: HashSet<NodeKey> = names.values().iter().filter(|w| w.file as usize == file).filter_map(|w| node_key(w.span)).collect();
    let mut edits: Vec<Edit> = Vec::new();
    for w in names.refs().iter().filter(|w| w.file as usize == file) {
        let name = names.name(w);
        let Some(at) = placed(w.span, name) else { continue };
        let key = node_key(w.span);
        if w.generic
            || builtin(name)
            || key.is_some_and(|k| declared.contains(&k) || as_values.contains(&k))
            || crate::types::Const::parse(name).is_some()
        {
            continue;
        }
        if name.starts_with('.') {
            edits.push(Edit { start: at.end, end: at.end, text: "#{deferred}".into() });
            continue;
        }
        let decls = type_decls(program, resolution, symbols, names.meaning(w));
        if let Some(e) = spell(&decls, name, at, all) {
            edits.push(e);
        }
    }
    for w in names.values().iter().filter(|w| w.file as usize == file) {
        let name = names.value_name(w);
        let Some(at) = placed(w.span, name) else { continue };
        let at_end = |text: String| Edit { start: at.end, end: at.end, text };
        if let Some(b) = w.local {
            // Where the binding was printed; a binding the printing has no
            // place for (one only an expansion names) by `?`.
            let pos = printed.at(b).map(|p| salvo_syntax::span::line_col(&printed.text, p.start));
            edits.push(at_end(match pos {
                Some((line, col)) => format!("#{{{line}:{col}}}"),
                None => "#{?}".to_string(),
            }));
            continue;
        }
        let m = names.value_meaning(w);
        match w.role {
            // As written: the selector says which.
            ValueRole::Selected => {}
            ValueRole::Selector => {
                let decls = type_decls(program, resolution, symbols, m);
                if let [d] = decls.as_slice() {
                    if !d.bare && (m.structure.is_some() || m.opaque.is_some() || m.alias.is_some()) {
                        edits.push(at_end(format!("#{{{}}}", d.module)));
                    }
                }
            }
            ValueRole::Bare => {
                // [mod-namespace] A namespace's segment names no declaration.
                let namespaces = &program.modules[file].namespaces;
                if namespaces.iter().any(|n| n == name || n.starts_with(&format!("{name}."))) {
                    continue;
                }
                // [effect-available] With the members its body's written
                // effects add.
                let mut decls = value_decls(program, resolution, symbols, m, names.available_members(w));
                // A type-like name in value position (`InRange.qualifies(…)`,
                // a struct named as a value) means its declaration.
                if decls.is_empty() {
                    decls = type_decls(program, resolution, symbols, m);
                }
                // A body some of whose effects only types say (a `use` of a
                // handle, a lambda): the type phase may find a member.
                if decls.is_empty() && w.dynamic {
                    edits.push(at_end("#{deferred}".into()));
                    continue;
                }
                if let Some(e) = spell(&decls, name, at, all) {
                    edits.push(e);
                }
            }
        }
    }
    edits.sort_by_key(|e| (e.start, e.end));
    let source = printed.text.as_str();
    let mut out = String::with_capacity(source.len() + edits.len() * 16);
    let mut at = 0usize;
    for e in edits {
        let (start, end) = (e.start as usize, e.end as usize);
        if start < at {
            continue;
        }
        out.push_str(&source[at..start]);
        out.push_str(&e.text);
        at = end;
    }
    out.push_str(&source[at..]);
    out
}

/// The spelling of a name meaning `decls`: `None` when it stays as written.
fn spell(decls: &[Decl], name: &str, span: Placed, all: bool) -> Option<Edit> {
    match decls {
        [] => Some(Edit { start: span.end, end: span.end, text: "#{???}".into() }),
        [d] if d.bare && d.name == name => None,
        [d] if d.bare => Some(Edit { start: span.start, end: span.end, text: d.name.clone() }),
        // An import alias reads as the declaration's own name.
        [d] => Some(Edit { start: span.start, end: span.end, text: format!("{}@{}", d.name, d.module) }),
        many if many.iter().all(|d| d.bare) => None,
        many => {
            // The program's own first: std's are the common case.
            let mut sorted: Vec<&Decl> = many.iter().collect();
            sorted.sort_by_key(|d| d.bare);
            let mut sel: Vec<String> = Vec::new();
            for d in sorted {
                let s = format!("@{}", d.module);
                if !sel.contains(&s) {
                    sel.push(s);
                }
            }
            if let [one] = sel.as_slice() {
                // Overloads of one module: the selector says it all.
                return Some(Edit { start: span.end, end: span.end, text: one.clone() });
            }
            let shown = if all || sel.len() <= SHOWN { sel.join(", ") } else { format!("{}, … {} more", sel[..SHOWN].join(", "), sel.len() - SHOWN) };
            Some(Edit { start: span.end, end: span.end, text: format!("#{{{shown}}}") })
        }
    }
}

/// The node of every declaration's name.
fn declared_names(module: &salvo_syntax::ast::Module) -> HashSet<NodeKey> {
    use salvo_syntax::ast::Item;
    fn items(list: &[Item], out: &mut HashSet<NodeKey>) {
        for item in list {
            let name = match item {
                Item::Fn(f) => &f.name,
                Item::Struct(s) => &s.name,
                Item::Type(t) => &t.name,
                Item::Effect(e) => &e.name,
                Item::Handler(h) => &h.name,
                Item::Qualifier(q) => &q.name,
                Item::Fns(g) => &g.name,
                Item::EffectAlias(a) => &a.name,
                Item::Namespace(n) => {
                    items(&n.items, out);
                    continue;
                }
                _ => continue,
            };
            out.extend(node_key(name.span));
        }
    }
    let mut out = HashSet::new();
    items(&module.items, &mut out);
    out
}

/// The compiler's own names, which no declaration holds.
fn builtin(name: &str) -> bool {
    matches!(name, "None" | "Never" | "Any" | "Mut" | "Linear" | "linear" | "once" | "proj" | "ref" | "self")
        || salvo_syntax::ast::TypeLit::from_ref_name(name).is_some()
}

/// What a type-position name may mean: its type (or, failing one, its
/// handler), else its qualifiers, group and effect alias.
fn type_decls(program: &Program, resolution: &Resolution<'_>, symbols: &Symbols<'_>, m: &Meaning<'_>) -> Vec<Decl> {
    let decl = |name: &salvo_syntax::ast::Ident, keyed: Option<bool>| -> Decl {
        let file = resolution.decls.file_of(name.span);
        let module = file.map(|f| program.files[f].module.to_string()).unwrap_or_default();
        let std = file.is_some_and(|f| program.files[f].is_std);
        // [type-identity] A type is bare when its key is plain (std's first
        // declaration of the name); anything else, when it is std's.
        Decl { name: name.name.clone(), module, bare: std && keyed != Some(true) }
    };
    let key_keyed = |k: Option<&str>| Some(k.is_some_and(crate::typekey::is_keyed));
    if let Some(d) = m.structure {
        return vec![decl(&d.name, key_keyed(symbols.key_of(d)))];
    }
    if let Some(d) = m.effect {
        return vec![decl(&d.name, key_keyed(symbols.key_of(d)))];
    }
    if let Some(d) = m.opaque.or(m.alias) {
        return vec![decl(&d.name, key_keyed(symbols.key_of(d)))];
    }
    let mut out = Vec::new();
    if let Some(d) = m.handler {
        out.push(decl(&d.name, key_keyed(symbols.key_of(d))));
    }
    out.extend(m.qualifiers.iter().map(|q| decl(&q.name, None)));
    if let Some(g) = m.group {
        out.push(decl(&g.name, None));
    }
    if let Some((a, _)) = m.effect_alias {
        out.push(decl(&a.name, None));
    }
    out
}

/// What a bare value-position name may mean: its handler, or its fns and
/// effect members (each by its selector: a fn's module, a member's effect).
fn value_decls(
    program: &Program,
    resolution: &Resolution<'_>,
    symbols: &Symbols<'_>,
    m: &Meaning<'_>,
    available: &[(&salvo_syntax::ast::EffectDecl, &salvo_syntax::ast::FnDecl)],
) -> Vec<Decl> {
    if let Some(h) = m.handler {
        let file = resolution.decls.file_of(h.name.span);
        let keyed = symbols.key_of(h).is_some_and(crate::typekey::is_keyed);
        return vec![Decl {
            name: h.name.name.clone(),
            module: file.map(|f| program.files[f].module.to_string()).unwrap_or_default(),
            bare: file.is_some_and(|f| program.files[f].is_std) && !keyed,
        }];
    }
    let mut out: Vec<Decl> = m
        .fns
        .iter()
        .map(|e| Decl { name: e.decl.name.name.clone(), module: e.module.to_string(), bare: program.files[e.file].is_std })
        .collect();
    for (effect, f) in m.members.iter().chain(available) {
        let std = resolution.decls.file_of(effect.name.span).is_some_and(|i| program.files[i].is_std);
        out.push(Decl { name: f.name.name.clone(), module: effect.name.name.clone(), bare: std });
    }
    out
}
