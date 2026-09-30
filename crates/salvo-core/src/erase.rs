//! [effect-generic-decl] Effect-typed generic parameters on declarations, and
//! their **erasure** for the backends.
//!
//! `actor effect ActorGroup<E> { send fn join(member: Addr<E>) }` is generic
//! over an *effect*: `E` stands for a protocol, not a type of values, and the
//! only position it may take is inside `Addr<…>` — the one sanctioned
//! effect-in-a-type-position [effect-not-data], which `watch<E>` already had
//! for a function (user decision 2026-09-26, P-2). The checker keeps such
//! instances distinct (`ActorGroup<Counter>` is not `ActorGroup<Ledger>`,
//! which is what types `attach<E>`); the **backends erase them**, because an
//! `Addr<E>` lowers to a scheduler index whatever `E` is — so the message
//! enum, the stub, the trait and the handler of an effect-only-generic
//! declaration are all monomorphic, and nothing about `E` reaches the output.
//!
//! This module is the erasure: given a checked program, a copy in which every
//! effect, handler and struct whose generic parameters are *all* effect-only
//! (each occurrence is exactly `Addr<P>`, or an argument of another erased
//! declaration) has its parameter list emptied. Type *mentions* keep their
//! arguments in the AST; the emitters drop them where they render an erased
//! name (`erased_generics` says which). Spans are untouched, so every checker
//! table keyed by span still applies to the copy.

use std::collections::{HashMap, HashSet};

use salvo_syntax::ast::{EffectDecl, HandlerDecl, Item, Module, StructDecl, Type, TypeRef};

use crate::program::Program;

/// The declarations whose generics erase, by name: effects, handlers and
/// structs. Computed once from the original program and consulted by the
/// emitters wherever an instance of one is rendered.
#[derive(Default, Debug, Clone)]
pub struct Erased {
    pub effects: HashSet<String>,
    pub handlers: HashSet<String>,
    pub structs: HashSet<String>,
    /// Functions whose generics are all effect-only (`attach<E>`): emitted
    /// without them, and called without a turbofish. Keyed by (module index,
    /// name span start), since overloads share a name and only one of them
    /// may erase (`eq<E>(a: Addr<E>, b: Addr<E>)` beside the ordinary `eq`s).
    pub fns: HashSet<(usize, u32)>,
}

impl Erased {
    pub fn is_erased(&self, name: &str) -> bool {
        self.effects.contains(name) || self.handlers.contains(name) || self.structs.contains(name)
    }
}

/// Whether every occurrence of `param` in `ty` is effect-only: the sole
/// argument of `Addr`, or an argument of an erased declaration at a position
/// whose parameter is itself effect-only. `None` when the parameter does not
/// occur; `Some(false)` on a value-position occurrence.
fn occurrences_effect_only(
    ty: &Type,
    param: &str,
    erased: &Erased,
    decls: &HashMap<&str, Vec<String>>,
    current: &str,
) -> Option<bool> {
    fn walk(
        ty: &Type,
        param: &str,
        erased: &Erased,
        decls: &HashMap<&str, Vec<String>>,
        found: &mut bool,
        ok: &mut bool,
        effect_position: bool,
    ) {
        match ty {
            Type::Literal { .. } => {}
            Type::Named { base, qualifiers } => {
                for q in qualifiers {
                    for a in &q.args {
                        walk(a, param, erased, decls, found, ok, false);
                    }
                }
                walk_ref(base, param, erased, decls, found, ok, effect_position);
            }
            Type::QualifiedGroup { base, qualifiers, .. } => {
                for q in qualifiers {
                    for a in &q.args {
                        walk(a, param, erased, decls, found, ok, false);
                    }
                }
                walk(base, param, erased, decls, found, ok, effect_position);
            }
            Type::Union { arms, .. } => arms
                .iter()
                .for_each(|a| walk(a, param, erased, decls, found, ok, false)),
            Type::Tuple { elems, .. } => elems
                .iter()
                .for_each(|e| walk(e, param, erased, decls, found, ok, false)),
            Type::Array { elem, .. } => walk(elem, param, erased, decls, found, ok, false),
            Type::Nullable { inner, .. } => walk(inner, param, erased, decls, found, ok, false),
            Type::Fn { params, ret, .. } => {
                for p in params {
                    walk(p, param, erased, decls, found, ok, false);
                }
                walk(ret, param, erased, decls, found, ok, false);
            }
        }
    }
    fn walk_ref(
        r: &TypeRef,
        param: &str,
        erased: &Erased,
        decls: &HashMap<&str, Vec<String>>,
        found: &mut bool,
        ok: &mut bool,
        effect_position: bool,
    ) {
        let name = r.name.name.as_str();
        if name == param {
            *found = true;
            if !effect_position {
                *ok = false;
            }
            return;
        }
        if name == "Addr" && r.args.len() == 1 {
            walk(&r.args[0], param, erased, decls, found, ok, true);
            return;
        }
        // An argument of an erased declaration is in effect position exactly
        // where that declaration's own parameter is (all of them, once it is
        // erased).
        let arg_is_effect = erased.is_erased(name) && decls.contains_key(name);
        for a in &r.args {
            walk(a, param, erased, decls, found, ok, arg_is_effect);
        }
    }
    // The declaration being judged counts as erased while judging it — a
    // self-reference (`start(me: Addr<ActorGroup<E>>)` inside `ActorGroup`)
    // would otherwise refuse itself; the fixpoint confirms or retracts.
    let mut assumed = erased.clone();
    assumed.effects.insert(current.to_string());
    assumed.handlers.insert(current.to_string());
    assumed.structs.insert(current.to_string());
    let mut found = false;
    let mut ok = true;
    walk(ty, param, &assumed, decls, &mut found, &mut ok, false);
    if found {
        Some(ok)
    } else {
        None
    }
}

fn effect_types(e: &EffectDecl) -> Vec<&Type> {
    let mut out = Vec::new();
    for f in &e.fns {
        for p in &f.params {
            out.push(&p.ty);
        }
        if let Some(r) = &f.return_type {
            out.push(r);
        }
    }
    out
}

fn handler_types(h: &HandlerDecl) -> Vec<&Type> {
    let mut out: Vec<&Type> = h.of.iter().collect();
    for p in &h.params {
        out.push(&p.ty);
    }
    for f in &h.state {
        out.push(&f.ty);
    }
    for f in &h.fns {
        for p in &f.params {
            out.push(&p.ty);
        }
        if let Some(r) = &f.return_type {
            out.push(r);
        }
    }
    out
}

fn struct_types(s: &StructDecl) -> Vec<&Type> {
    s.fields.iter().map(|f| &f.ty).collect()
}

fn fn_types(f: &salvo_syntax::ast::FnDecl) -> Vec<&Type> {
    let mut out: Vec<&Type> = f.params.iter().map(|p| &p.ty).collect();
    if let Some(r) = &f.return_type {
        out.push(r);
    }
    out
}

/// Whether every generic parameter of a declaration is effect-only over the
/// given types. A declaration with no generics is not "erased" (nothing to
/// erase); one whose parameter never occurs is not either (it is an ordinary
/// unused parameter).
fn all_effect_only(
    current: &str,
    generics: &[salvo_syntax::ast::Ident],
    types: &[&Type],
    erased: &Erased,
    decls: &HashMap<&str, Vec<String>>,
    phantom_ok: bool,
) -> bool {
    if generics.is_empty() {
        return false;
    }
    generics.iter().all(|g| {
        let mut seen = false;
        for t in types {
            match occurrences_effect_only(t, &g.name, erased, decls, current) {
                Some(false) => return false,
                Some(true) => seen = true,
                None => {}
            }
        }
        // A parameter that never occurs is a **phantom**: on a struct it
        // exists to type the value (`Protocol<E>`), and a backend struct
        // with an unused parameter does not compile, so it erases too.
        seen || phantom_ok
    })
}

/// Computes the erased set: a fixpoint, since a handler's generics count as
/// effect-only when they are arguments of an effect already found erasable.
pub fn erased_generics(program: &Program) -> Erased {
    let mut decls: HashMap<&str, Vec<String>> = HashMap::new();
    for unit in program.units() {
        for item in &unit.ast.items {
            match item {
                Item::Effect(e) if !e.generics.is_empty() => {
                    decls.insert(&e.name.name, e.generics.iter().map(|g| g.name.clone()).collect());
                }
                Item::Handler(h) if !h.generics.is_empty() => {
                    decls.insert(&h.name.name, h.generics.iter().map(|g| g.name.clone()).collect());
                }
                Item::Struct(s) if !s.generics.is_empty() => {
                    decls.insert(&s.name.name, s.generics.iter().map(|g| g.name.clone()).collect());
                }
                _ => {}
            }
        }
    }
    let mut erased = Erased::default();
    loop {
        let before = erased.effects.len() + erased.handlers.len() + erased.structs.len() + erased.fns.len();
        for (module_idx, unit) in program.units().enumerate() {
            for item in &unit.ast.items {
                match item {
                    Item::Effect(e) if all_effect_only(&e.name.name, &e.generics, &effect_types(e), &erased, &decls, false) => {
                        erased.effects.insert(e.name.name.clone());
                    }
                    Item::Handler(h) if all_effect_only(&h.name.name, &h.generics, &handler_types(h), &erased, &decls, false) => {
                        erased.handlers.insert(h.name.name.clone());
                    }
                    Item::Struct(s) if all_effect_only(&s.name.name, &s.generics, &struct_types(s), &erased, &decls, true) => {
                        erased.structs.insert(s.name.name.clone());
                    }
                    // A function's own name is never a type, so `current` is
                    // a name no type mentions.
                    Item::Fn(f) if all_effect_only("", &f.generics, &fn_types(f), &erased, &decls, false) => {
                        erased.fns.insert((module_idx, f.name.span.start));
                    }
                    _ => {}
                }
            }
        }
        if erased.effects.len() + erased.handlers.len() + erased.structs.len() + erased.fns.len() == before {
            break;
        }
    }
    erased
}

/// The program with every erased declaration's generic parameter list
/// emptied — what a backend emits from, with the original's checker tables.
pub fn erase_effect_generics(program: &Program, erased: &Erased) -> Program {
    let modules: Vec<Module> = program
        .modules
        .iter()
        .enumerate()
        .map(|(module_idx, m)| {
            let mut m = m.clone();
            for item in &mut m.items {
                match item {
                    Item::Effect(e) if erased.effects.contains(&e.name.name) => e.generics.clear(),
                    Item::Handler(h) if erased.handlers.contains(&h.name.name) => h.generics.clear(),
                    Item::Struct(s) if erased.structs.contains(&s.name.name) => {
                        s.generics.clear();
                        s.generic_canbe.clear();
                    }
                    Item::Fn(f) if erased.fns.contains(&(module_idx, f.name.span.start)) => {
                        f.generics.clear();
                        f.generic_canbe.clear();
                    }
                    _ => {}
                }
            }
            m
        })
        .collect();
    Program {
        files: program.files.clone(),
        modules,
        companions: program.companions.clone(),
    }
}
