//! [round-trip] The round-trip check for calls (PIPELINE.md B4, §II.7; user
//! decision P5 (b): the corpus and the examples). A checked file is printed
//! back as Salvo with every choice its calls made written out — the callee
//! as `name@module` (`name@Effect` for an effect's member), the inferred type
//! arguments, the implicit arguments passed by name (`cmp = cmp@main`) — and
//! the written-out program is checked again, which must make the same
//! choices. A rule that decides differently on the implicit spelling than on
//! the explicit one fails it, and so does a printer that writes the wrong
//! thing.
//!
//! What has no surface spelling is skipped, as §II.7 marks it: operators,
//! interpolation parts and `for` (their calls are not written), a call
//! through a local or an implicit of the enclosing fn, sends, a renamed
//! name, a type argument with no source form, and anything a desugaring or a
//! stamp made.

use std::collections::BTreeMap;

use salvo_syntax::ast::{Expr, FnDecl, Item, Module, NamedArg};
use salvo_syntax::visit::{self, Visitor};
use salvo_syntax::Span;

use crate::check::{Checked, ImplicitArg};
use crate::program::Symbols;
use crate::types::Ty;
use crate::Program;

/// One file written back with its calls' choices spelled out, and what the
/// re-check must find there.
pub struct Rewrite {
    /// The written-out source.
    pub text: String,
    /// Every call whose choice was written out, by its start in the
    /// rewritten text: the callee it must resolve to, and the type arguments
    /// it must infer (now written).
    pub calls: BTreeMap<u32, Vec<String>>,
    /// Every implicit argument written out: the call's start in the
    /// rewritten text, the argument's name, and the declaration (or member)
    /// its value must name.
    pub implicits: Vec<(u32, String, String)>,
}

/// An edit: insert `text` at byte `at` of the original source.
struct Insert {
    at: u32,
    text: String,
}

/// The calls of one module's declarations, in walk order.
fn calls_of(m: &Module) -> Vec<&Expr> {
    let mut fns: Vec<&FnDecl> = Vec::new();
    collect_fns(&m.items, &mut fns);
    let mut v = Collect { found: Vec::new() };
    for f in fns {
        visit::walk_fn(&mut v, f);
    }
    // The visitor sees each node behind a reference as long-lived as `m`,
    // which owns every node it reached.
    v.found.into_iter().map(|e| unsafe { &*e }).collect()
}

fn collect_fns<'a>(items: &'a [Item], out: &mut Vec<&'a FnDecl>) {
    for item in items {
        match item {
            Item::Fn(f) => out.push(f),
            Item::Handler(h) => {
                out.extend(h.fns.iter());
                out.extend(h.init.iter());
            }
            Item::Qualifier(q) => out.extend(q.fns.iter()),
            Item::Struct(s) => out.extend(s.fns.iter()),
            Item::Namespace(n) => collect_fns(&n.items, out),
            _ => {}
        }
    }
}

struct Collect {
    found: Vec<*const Expr>,
}

impl Visitor for Collect {
    fn visit_expr(&mut self, e: &Expr) {
        if matches!(e, Expr::Call { .. }) {
            self.found.push(e as *const Expr);
        }
    }
}

/// Whether a type has a source spelling the round trip can write: named
/// types, unions, tuples and arrays of them, with no qualifier, borrow,
/// identity or unknown part.
fn spellable(t: &Ty) -> bool {
    match t {
        Ty::Named { name, args } => !name.contains('§') && !name.contains('_') && args.iter().all(spellable),
        Ty::Union(arms) => arms.iter().all(spellable),
        Ty::Tuple(elems) => !elems.is_empty() && elems.iter().all(spellable),
        Ty::Array(e) => spellable(e),
        _ => false,
    }
}

fn ident_at(source: &str, span: Span, name: &str) -> bool {
    source.get(span.start as usize..span.end as usize) == Some(name)
}

/// What a call's choice is, as the check compares it: the callee (a
/// declaration's id, or an effect member as `Effect#index`) and the type
/// arguments, when there are any. `None` for a call the round trip skips.
fn choice(checked: &Checked, file: usize, span: Span) -> Option<String> {
    let info = checked.call_info(checked.key_at(file, span));
    if info.local_call || info.addr_call.is_some() || info.self_send.is_some() || info.facade_send.is_some() || info.field_call {
        return None;
    }
    let callee = match (info.call_fn, info.member_call) {
        (Some(k), _) => k.to_string(),
        (None, Some(mc)) => format!("{}#{}", mc.effect, mc.index),
        _ => return None,
    };
    let targs = info.call_type_args.map(|ts| ts.iter().map(|t| t.to_string()).collect::<Vec<_>>().join(", ")).unwrap_or_default();
    Some(format!("{callee}<{targs}>"))
}

/// Writes `file` of a checked `program` back with its calls' choices
/// spelled out.
pub fn write_out(program: &Program, symbols: &Symbols<'_>, checked: &Checked, file: usize) -> Rewrite {
    let source = program.files[file].content.as_str();
    let module = &program.modules[file];
    let mut inserts: Vec<Insert> = Vec::new();
    // What to expect, by the call's start in the *original* text.
    let mut calls: BTreeMap<u32, Vec<String>> = BTreeMap::new();
    let mut implicits: Vec<(u32, String, String)> = Vec::new();
    for e in calls_of(module) {
        let Expr::Call { callee, type_args, args, named, span } = e else { continue };
        if span.is_expanded() || !span.is_numbered() {
            continue;
        }
        // The callee as written: a bare name, or a dot call's field.
        let (name, name_span) = match callee.as_ref() {
            Expr::Ident(id) => (id.name.as_str(), id.span),
            Expr::Field { field, grouped: false, .. } => (field.name.as_str(), field.span),
            _ => continue,
        };
        if name_span.is_expanded() || !ident_at(source, name_span, name) {
            continue;
        }
        let key = checked.key_at(file, *span);
        let info = checked.call_info(key);
        if let Expr::Field { .. } = callee.as_ref() {
            if !info.dot_call {
                continue;
            }
        }
        let Some(want) = choice(checked, file, *span) else { continue };
        // The selector: a declaration's module, or a member's effect.
        let selector = match (info.call_fn, info.member_call) {
            (Some(k), _) => {
                // A renamed name answers to its new name only, and `@` on it
                // is refused [fn-rename].
                if k.name() != name {
                    continue;
                }
                k.module().to_string()
            }
            (None, Some(mc)) => {
                let Some(effect) = symbols.effects.get(mc.effect.as_str()) else { continue };
                if effect.name.name == crate::THROW_EFFECT {
                    continue;
                }
                effect.name.name.clone()
            }
            _ => continue,
        };
        let mut text = format!("@{selector}");
        // [call-type-args] The inferred type arguments, where the call wrote
        // none and every one has a spelling. A member's pin its instance, so
        // they are left to it.
        if type_args.is_empty() && info.call_fn.is_some() {
            if let Some(ts) = info.call_type_args.filter(|ts| !ts.is_empty()) {
                if ts.iter().all(spellable) {
                    text.push_str(&format!("<{}>", ts.iter().map(|t| t.to_string()).collect::<Vec<_>>().join(", ")));
                }
            }
        }
        inserts.push(Insert { at: name_span.end, text });
        calls.entry(span.start).or_default().push(want);
        // [implicit-override] The implicit arguments, by name, before the
        // closing parenthesis — where the source has one.
        let Some(filled) = info.implicit_args else { continue };
        let close = span.end.saturating_sub(1);
        if source.as_bytes().get(close as usize) != Some(&b')') || args.last().is_some_and(|a| matches!(a, Expr::ReplyTo { .. })) {
            continue;
        }
        let mut names: Vec<&str> = Vec::new();
        let mut parts: Vec<(String, String, String)> = Vec::new();
        for arg in filled {
            let (name, value, expect) = match arg {
                ImplicitArg::Resolved { name, key, .. } => (name.clone(), format!("{}@{}", key.name(), key.module()), key.to_string()),
                ImplicitArg::Member { name, effect, index, .. } => {
                    let Some(decl) = symbols.effects.get(effect.as_str()) else { continue };
                    let Some(member) = decl.fns.get(*index) else { continue };
                    (name.clone(), format!("{}@{}", member.name.name, decl.name.name), format!("{effect}#{index}"))
                }
                _ => continue,
            };
            names.push(match arg {
                ImplicitArg::Resolved { name, .. } | ImplicitArg::Member { name, .. } => name.as_str(),
                _ => "",
            });
            parts.push((name, value, expect));
        }
        // Two implicits of one name are told apart by type, which a name
        // cannot say; and one already written is the user's.
        let mut sorted = names.clone();
        sorted.sort();
        sorted.dedup();
        if sorted.len() != names.len() || parts.iter().any(|(n, _, _)| named.iter().any(|a: &NamedArg| a.name.name == *n)) {
            continue;
        }
        let mut first = args.is_empty() && named.is_empty();
        let mut text = String::new();
        for (n, value, expect) in parts {
            if !first {
                text.push_str(", ");
            }
            first = false;
            text.push_str(&format!("{n} = {value}"));
            implicits.push((span.start, n, expect));
        }
        if !text.is_empty() {
            inserts.push(Insert { at: close, text });
        }
    }
    inserts.sort_by_key(|i| i.at);
    // The rewritten text, and where each original offset lands in it: an
    // insertion at `at` moves everything from `at` on.
    let mut text = String::with_capacity(source.len() + inserts.iter().map(|i| i.text.len()).sum::<usize>());
    let mut last = 0usize;
    for i in &inserts {
        text.push_str(&source[last..i.at as usize]);
        text.push_str(&i.text);
        last = i.at as usize;
    }
    text.push_str(&source[last..]);
    let moved = |at: u32| -> u32 { at + inserts.iter().filter(|i| i.at < at).map(|i| i.text.len() as u32).sum::<u32>() };
    Rewrite {
        text,
        calls: calls.into_iter().map(|(at, v)| (moved(at), v)).collect(),
        implicits: implicits.into_iter().map(|(at, n, e)| (moved(at), n, e)).collect(),
    }
}

/// What the re-check of a [`Rewrite`] found different: each call whose
/// choice changed, and each implicit argument whose written value names
/// something else. Empty when the round trip holds.
pub fn compare(program: &Program, checked: &Checked, file: usize, rewrite: &Rewrite) -> Vec<String> {
    let module = &program.modules[file];
    let source = program.files[file].content.as_str();
    let line = |at: u32| source[..at as usize].matches('\n').count() + 1;
    let mut found: BTreeMap<u32, Vec<String>> = BTreeMap::new();
    let mut values: BTreeMap<(u32, String), String> = BTreeMap::new();
    for e in calls_of(module) {
        let Expr::Call { named, span, .. } = e else { continue };
        if span.is_expanded() || !rewrite.calls.contains_key(&span.start) {
            continue;
        }
        if let Some(c) = choice(checked, file, *span) {
            found.entry(span.start).or_default().push(c);
        }
        for a in named {
            let id = match &a.value {
                Expr::Scoped { name, .. } | Expr::EffectScoped { name, .. } => name.span,
                _ => continue,
            };
            let key = checked.key_at(file, id);
            let got = match (checked.fn_refs.get(&key), checked.member_values.get(&key)) {
                (Some(k), _) => k.to_string(),
                (None, Some(ImplicitArg::Member { effect, index, .. })) => format!("{effect}#{index}"),
                _ => "nothing".to_string(),
            };
            values.insert((span.start, a.name.name.clone()), got);
        }
    }
    let mut out = Vec::new();
    for (at, want) in &rewrite.calls {
        let mut want = want.clone();
        want.sort();
        let mut got = found.get(at).cloned().unwrap_or_default();
        got.sort();
        if want != got {
            out.push(format!("line {}: the call chose {:?}, its written-out form {:?}", line(*at), want, got));
        }
    }
    for (at, name, want) in &rewrite.implicits {
        let got = values.get(&(*at, name.clone())).cloned().unwrap_or_else(|| "nothing".to_string());
        if *want != got {
            out.push(format!("line {}: `{name}` was filled by {want}, its written-out value names {got}", line(*at)));
        }
    }
    out
}
