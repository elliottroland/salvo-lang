//! [node-id] Node identity: every span-bearing node of a module gets its
//! ordinal in the file (`Span::node`), declaration by declaration, and the
//! module records each declaration's run of ordinals (`Module::node_ranges`).
//!
//! A fact the checker records is keyed by the declaration a node is in and
//! the node's ordinal within it (`salvo_core::check::Key`), so a key does not
//! move when code above the declaration changes, and two copies of one
//! template are two nodes whatever their spans. Every stage that rewrites a
//! module's tree numbers it last (the parse, each file's expansion, the
//! comptime stamping of a file), so the ordinals the checker and the IR
//! builder read are the same field of the same node and nothing recomputes
//! them. The walk is exhaustive over each declaration by destructuring: a
//! new field holding a span is a compile error here.
//!
//! The order: the file's own nodes first (imports, renames, refinements,
//! module-level `use`s — what no declaration holds), then each item in
//! order, a member (an effect's, handler's, qualifier's or group's fn, a
//! handler's `init` last) after its container's own nodes, as `DeclLoc`
//! numbers members.

use crate::ast::*;
use crate::span::Span;
use crate::visit_mut::{self as vm, MutVisitor};

struct Numberer {
    next: u32,
    /// [node-file] The file's tag (`Module::file_tag`).
    file: u32,
}

impl MutVisitor for Numberer {
    fn visit_span(&mut self, span: &mut Span) {
        span.node = self.next;
        span.file = self.file;
        self.next += 1;
    }
}

/// Numbers every node of `module` and records its declarations' ranges.
pub fn number(module: &mut Module) {
    let mut n = Numberer { next: 0, file: module.file_tag };
    let mut ranges = Vec::new();
    // The file's own nodes.
    for item in &mut module.items {
        match item {
            Item::Import(i) => import(&mut n, i),
            Item::Refn(r) => refn(&mut n, r),
            Item::Rename(r) => rename(&mut n, r),
            Item::Namespace(ns) => namespace(&mut n, ns),
            Item::Test(t) => test(&mut n, t),
            _ => {}
        }
    }
    for u in &mut module.uses {
        let ModuleUse { handler, span } = u;
        n.visit_span(span);
        vm::walk_expr(&mut n, handler);
    }
    ranges.push(NodeRange { first: 0, len: n.next, item: NodeRange::FILE, sub: 0 });
    for (idx, item) in module.items.iter_mut().enumerate() {
        let idx = idx as u32;
        let mut run = |n: &mut Numberer, sub: u32, f: &mut dyn FnMut(&mut Numberer)| {
            let first = n.next;
            f(n);
            ranges.push(NodeRange { first, len: n.next - first, item: idx, sub });
        };
        match item {
            Item::Fn(f) => run(&mut n, 0, &mut |n| fn_decl(n, f)),
            Item::Struct(s) => run(&mut n, 0, &mut |n| struct_decl(n, s)),
            Item::Type(t) => run(&mut n, 0, &mut |n| type_decl(n, t)),
            Item::EffectAlias(a) => run(&mut n, 0, &mut |n| {
                let EffectAliasDecl { docs: _, exported: _, name, effects, span } = a;
                n.visit_span(span);
                n.visit_ident(name);
                for e in effects {
                    vm::walk_effect_ref(n, e);
                }
            }),
            Item::Effect(e) => {
                let mut fns = std::mem::take(&mut e.fns);
                run(&mut n, 0, &mut |n| effect_decl(n, e));
                for (i, f) in fns.iter_mut().enumerate() {
                    run(&mut n, i as u32 + 1, &mut |n| fn_decl(n, f));
                }
                e.fns = fns;
            }
            Item::Handler(h) => {
                let mut fns = std::mem::take(&mut h.fns);
                let mut init = h.init.take();
                run(&mut n, 0, &mut |n| handler_decl(n, h));
                for (i, f) in fns.iter_mut().chain(init.iter_mut()).enumerate() {
                    run(&mut n, i as u32 + 1, &mut |n| fn_decl(n, f));
                }
                h.fns = fns;
                h.init = init;
            }
            Item::Qualifier(q) => {
                let mut fns = std::mem::take(&mut q.fns);
                run(&mut n, 0, &mut |n| qualifier_decl(n, q));
                for (i, f) in fns.iter_mut().enumerate() {
                    run(&mut n, i as u32 + 1, &mut |n| fn_decl(n, f));
                }
                q.fns = fns;
            }
            Item::Fns(g) => {
                let mut fns = std::mem::take(&mut g.fns);
                run(&mut n, 0, &mut |n| {
                    let FnsDecl { docs: _, exported: _, name, generics, fns: _, deductions, span } = g;
                    n.visit_span(span);
                    n.visit_ident(name);
                    for g in generics {
                        n.visit_ident(g);
                    }
                    for d in deductions {
                        vm::walk_deduction(n, d);
                    }
                });
                for (i, f) in fns.iter_mut().enumerate() {
                    run(&mut n, i as u32 + 1, &mut |n| fn_decl(n, f));
                }
                g.fns = fns;
            }
            Item::Import(_) | Item::Refn(_) | Item::Rename(_) | Item::Namespace(_) | Item::Test(_) => {}
        }
    }
    module.node_ranges = ranges;
}

/// Walks every node of `module` with `v`, in the order [`number`] numbers
/// them: what a pass that rewrites a whole file before numbering uses
/// (`desugar::lower_picks`).
pub fn walk_module<V: MutVisitor>(v: &mut V, module: &mut Module) {
    for item in &mut module.items {
        match item {
            Item::Import(i) => import(v, i),
            Item::Refn(r) => refn(v, r),
            Item::Rename(r) => rename(v, r),
            Item::Namespace(ns) => namespace(v, ns),
            Item::Test(t) => test(v, t),
            _ => {}
        }
    }
    for u in &mut module.uses {
        let ModuleUse { handler, span } = u;
        v.visit_span(span);
        vm::walk_expr(v, handler);
    }
    for item in module.items.iter_mut() {
        match item {
            Item::Fn(f) => fn_decl(v, f),
            Item::Struct(s) => struct_decl(v, s),
            Item::Type(t) => type_decl(v, t),
            Item::EffectAlias(a) => {
                let EffectAliasDecl { docs: _, exported: _, name, effects, span } = a;
                v.visit_span(span);
                v.visit_ident(name);
                for e in effects {
                    vm::walk_effect_ref(v, e);
                }
            }
            Item::Effect(e) => {
                let mut fns = std::mem::take(&mut e.fns);
                effect_decl(v, e);
                for f in fns.iter_mut() {
                    fn_decl(v, f);
                }
                e.fns = fns;
            }
            Item::Handler(h) => {
                let mut fns = std::mem::take(&mut h.fns);
                let mut init = h.init.take();
                handler_decl(v, h);
                for f in fns.iter_mut().chain(init.iter_mut()) {
                    fn_decl(v, f);
                }
                h.fns = fns;
                h.init = init;
            }
            Item::Qualifier(q) => {
                let mut fns = std::mem::take(&mut q.fns);
                qualifier_decl(v, q);
                for f in fns.iter_mut() {
                    fn_decl(v, f);
                }
                q.fns = fns;
            }
            Item::Fns(g) => {
                let mut fns = std::mem::take(&mut g.fns);
                let FnsDecl { docs: _, exported: _, name, generics, fns: _, deductions, span } = g;
                v.visit_span(span);
                v.visit_ident(name);
                for g in generics {
                    v.visit_ident(g);
                }
                for d in deductions {
                    vm::walk_deduction(v, d);
                }
                for f in fns.iter_mut() {
                    fn_decl(v, f);
                }
                g.fns = fns;
            }
            Item::Import(_) | Item::Refn(_) | Item::Rename(_) | Item::Namespace(_) | Item::Test(_) => {}
        }
    }
}

fn idents<V: MutVisitor>(n: &mut V, ids: &mut [Ident]) {
    for i in ids {
        n.visit_ident(i);
    }
}

fn canbe<V: MutVisitor>(n: &mut V, gs: &mut [(Ident, TypeRef)]) {
    for (g, q) in gs {
        n.visit_ident(g);
        vm::walk_type_ref(n, q);
    }
}

fn slots<V: MutVisitor>(n: &mut V, ss: &mut [SlotDecl]) {
    for s in ss {
        match s {
            SlotDecl::One(FnSlot { name, ty, span }) => {
                n.visit_span(span);
                n.visit_ident(name);
                vm::walk_type(n, ty);
            }
            SlotDecl::Group(t) => vm::walk_type_ref(n, t),
        }
    }
}

fn by_ref<V: MutVisitor>(n: &mut V, by: &mut ByRef) {
    let ByRef { path, at, span } = by;
    n.visit_span(span);
    idents(n, path);
    match at {
        Some(ByAt::Module(sp)) => n.visit_span(sp),
        Some(ByAt::Fn(sel)) => idents(n, sel),
        None => {}
    }
}

fn obligations<V: MutVisitor>(n: &mut V, os: &mut [Obligation]) {
    for o in os {
        let Obligation { by, group } = o;
        vm::walk_type_ref(n, group);
        if let Some(by) = by {
            by_ref(n, by);
        }
    }
}

fn field<V: MutVisitor>(n: &mut V, f: &mut FieldDecl) {
    let FieldDecl { docs: _, annotations, name, ty, canbe_mut: _, default, span } = f;
    n.visit_span(span);
    for a in annotations {
        vm::walk_type_ref(n, a);
    }
    n.visit_ident(name);
    if let Some(t) = ty {
        vm::walk_type(n, t);
    }
    if let Some(d) = default {
        vm::walk_expr(n, d);
    }
}

fn param<V: MutVisitor>(n: &mut V, p: &mut Param) {
    let Param { name, ty, variadic: _, implicit: _, default, span } = p;
    n.visit_span(span);
    n.visit_ident(name);
    vm::walk_type(n, ty);
    if let Some(d) = default {
        vm::walk_expr(n, d);
    }
}

fn responses<V: MutVisitor>(n: &mut V, rs: &mut [ConditionalResponse]) {
    for r in rs {
        let ConditionalResponse { subject, negated: _, qual, span } = r;
        n.visit_span(span);
        n.visit_ident(subject);
        vm::walk_type_ref(n, qual);
    }
}

fn fn_decl<V: MutVisitor>(n: &mut V, f: &mut FnDecl) {
    let FnDecl {
        docs: _,
        exported: _,
        intrinsic: _,
        platform: _,
        is_iter: _,
        iter_state,
        iter_state_name,
        is_send: _,
        name,
        type_name,
        scoped_to,
        compfn,
        by,
        stamped,
        generics,
        generic_canbe,
        derived_return,
        params,
        implicit_groups,
        effects,
        deductions,
        return_type,
        response,
        constructs,
        body,
        span,
    } = f;
    n.visit_span(span);
    n.visit_ident(name);
    if let Some(t) = type_name {
        vm::walk_type(n, t);
    }
    if let Some(s) = scoped_to {
        n.visit_ident(s);
    }
    if let Some(CompFn { bound, span }) = compfn {
        n.visit_span(span);
        if let Some((_, id)) = bound {
            n.visit_ident(id);
        }
    }
    idents(n, generics);
    canbe(n, generic_canbe);
    if let Some(d) = derived_return {
        n.visit_ident(d);
    }
    for p in params {
        param(n, p);
    }
    for g in implicit_groups {
        vm::walk_type_ref(n, g);
    }
    if let Some(effects) = effects {
        for e in effects {
            vm::walk_effect_ref(n, e);
        }
    }
    if let Some(ds) = deductions {
        for d in ds {
            vm::walk_deduction(n, d);
        }
    }
    if let Some(t) = return_type {
        vm::walk_type(n, t);
    }
    if let Some(rs) = response {
        responses(n, rs);
    }
    if let Some(c) = constructs {
        vm::walk_type_ref(n, c);
    }
    if let Some(s) = iter_state_name {
        n.visit_ident(s);
    }
    for st in iter_state {
        field(n, st);
    }
    if let Some(b) = body {
        vm::walk_block(n, b);
    }
    if let Some(by) = by {
        by_ref(n, by);
    }
    // A stamp's `site` and copy targets point at other declarations' nodes
    // (where a diagnostic lands): references, not nodes of this one. Its
    // roots are names this fn's comptime constructs wrote.
    if let Some(Stamp { compfn: _, from: _, at_type: _, site: _, expansions: _, roots, template_file: _, site_names: _, scopes: _ }) = stamped {
        idents(n, roots);
    }
}

fn struct_decl<V: MutVisitor>(n: &mut V, s: &mut StructDecl) {
    let StructDecl {
        docs: _,
        annotations,
        exported: _,
        name,
        generics,
        generic_canbe,
        fn_slots,
        obligations: obls,
        auto_qualifiers,
        fields,
        fns,
        refns,
        linear: _,
        opaque: _,
        noremote: _,
        comptime: _,
        span,
    } = s;
    n.visit_span(span);
    for a in annotations {
        vm::walk_type_ref(n, a);
    }
    n.visit_ident(name);
    idents(n, generics);
    canbe(n, generic_canbe);
    slots(n, fn_slots);
    obligations(n, obls);
    for q in auto_qualifiers {
        vm::walk_type_ref(n, q);
    }
    for f in fields {
        field(n, f);
    }
    // Hoisted to module level by the per-file expansion [fn-attached]; a
    // module numbered before it holds them here.
    for f in fns {
        fn_decl(n, f);
    }
    for r in refns {
        refn(n, r);
    }
}

fn type_decl<V: MutVisitor>(n: &mut V, t: &mut TypeDecl) {
    let TypeDecl {
        docs: _,
        exported: _,
        intrinsic: _,
        linear: _,
        noremote: _,
        comptime: _,
        platform: _,
        threadsafe: _,
        iterable: _,
        name,
        generics,
        generic_canbe,
        fn_slots,
        auto_qualifiers,
        alias,
        obligations: obls,
        refns,
        span,
    } = t;
    n.visit_span(span);
    n.visit_ident(name);
    idents(n, generics);
    canbe(n, generic_canbe);
    slots(n, fn_slots);
    for q in auto_qualifiers {
        vm::walk_type_ref(n, q);
    }
    if let Some(a) = alias {
        vm::walk_type(n, a);
    }
    obligations(n, obls);
    for r in refns {
        refn(n, r);
    }
}

fn effect_decl<V: MutVisitor>(n: &mut V, e: &mut EffectDecl) {
    let EffectDecl { docs: _, exported: _, is_actor: _, name, generics, prereqs, fns: _, span } = e;
    n.visit_span(span);
    n.visit_ident(name);
    idents(n, generics);
    for p in prereqs {
        vm::walk_effect_ref(n, p);
    }
}

fn handler_decl<V: MutVisitor>(n: &mut V, h: &mut HandlerDecl) {
    let HandlerDecl {
        docs: _,
        exported: _,
        intrinsic: _,
        platform: _,
        threadsafe: _,
        name,
        generics,
        params,
        effects,
        of,
        of_any: _,
        init: _,
        mailbox,
        state,
        state_name,
        fns: _,
        span,
    } = h;
    n.visit_span(span);
    n.visit_ident(name);
    idents(n, generics);
    for p in params {
        param(n, p);
    }
    if let Some(effects) = effects {
        for e in effects {
            vm::walk_effect_ref(n, e);
        }
    }
    for t in of {
        vm::walk_type(n, t);
    }
    if let Some(m) = mailbox {
        vm::walk_expr(n, m);
    }
    if let Some(s) = state_name {
        n.visit_ident(s);
    }
    for f in state {
        field(n, f);
    }
}

fn qualifier_decl<V: MutVisitor>(n: &mut V, q: &mut QualifierDecl) {
    let QualifierDecl {
        docs: _,
        exported: _,
        intrinsic: _,
        subject: _,
        name,
        generics,
        generic_canbe,
        fn_slots,
        value_slots,
        of,
        field_overrides,
        fns: _,
        refns,
        has_body: _,
        comptime: _,
        span,
    } = q;
    n.visit_span(span);
    n.visit_ident(name);
    idents(n, generics);
    canbe(n, generic_canbe);
    slots(n, fn_slots);
    for ValueSlot { name, ty, span } in value_slots {
        n.visit_span(span);
        n.visit_ident(name);
        vm::walk_type(n, ty);
    }
    vm::walk_type(n, of);
    for f in field_overrides {
        field(n, f);
    }
    for r in refns {
        refn(n, r);
    }
}

fn import<V: MutVisitor>(n: &mut V, i: &mut ImportDecl) {
    let ImportDecl { path, alias, span } = i;
    n.visit_span(span);
    idents(n, path);
    if let Some(a) = alias {
        n.visit_ident(a);
    }
}

fn refn<V: MutVisitor>(n: &mut V, r: &mut RefnDecl) {
    let RefnDecl { docs: _, name, at, generics, params, wildcard: _, deductions, response, span } = r;
    n.visit_span(span);
    n.visit_ident(name);
    if let Some(a) = at {
        n.visit_ident(a);
    }
    idents(n, generics);
    for p in params {
        param(n, p);
    }
    for d in deductions {
        let RefnDeduction { param, add, remove, preserve, span } = d;
        n.visit_span(span);
        n.visit_ident(param);
        for q in add.iter_mut().chain(remove.iter_mut()).chain(preserve.iter_mut()) {
            vm::walk_type_ref(n, q);
        }
    }
    if let Some(rs) = response {
        responses(n, rs);
    }
}

fn rename<V: MutVisitor>(n: &mut V, r: &mut RenameDecl) {
    let RenameDecl { docs: _, name, target, generics, params, span } = r;
    n.visit_span(span);
    n.visit_ident(name);
    n.visit_ident(target);
    idents(n, generics);
    for p in params {
        param(n, p);
    }
}

/// Flattened before resolution [mod-namespace]; a module numbered before that
/// holds its declarations here, among the file's own nodes.
fn namespace<V: MutVisitor>(n: &mut V, ns: &mut NamespaceDecl) {
    let NamespaceDecl { docs: _, exported: _, name, items, terminal: _, span } = ns;
    n.visit_span(span);
    n.visit_ident(name);
    for item in items {
        match item {
            Item::Import(i) => import(n, i),
            Item::Refn(r) => refn(n, r),
            Item::Rename(r) => rename(n, r),
            Item::Namespace(ns) => namespace(n, ns),
            Item::Test(t) => test(n, t),
            Item::Fn(f) => fn_decl(n, f),
            Item::Struct(s) => struct_decl(n, s),
            Item::Type(t) => type_decl(n, t),
            Item::Effect(e) => {
                effect_decl(n, e);
                for f in &mut e.fns {
                    fn_decl(n, f);
                }
            }
            Item::Handler(h) => {
                handler_decl(n, h);
                for f in h.fns.iter_mut().chain(h.init.iter_mut()) {
                    fn_decl(n, f);
                }
            }
            Item::Qualifier(q) => {
                qualifier_decl(n, q);
                for f in &mut q.fns {
                    fn_decl(n, f);
                }
            }
            Item::Fns(g) => {
                for f in &mut g.fns {
                    fn_decl(n, f);
                }
                for d in &mut g.deductions {
                    vm::walk_deduction(n, d);
                }
                n.visit_span(&mut g.span);
                n.visit_ident(&mut g.name);
                idents(n, &mut g.generics);
            }
            Item::EffectAlias(a) => {
                n.visit_span(&mut a.span);
                n.visit_ident(&mut a.name);
                for e in &mut a.effects {
                    vm::walk_effect_ref(n, e);
                }
            }
        }
    }
}

/// Expanded into a fn in a test annex, dropped elsewhere (`expand_file`); a
/// module numbered before that holds it among the file's own nodes.
fn test<V: MutVisitor>(n: &mut V, t: &mut TestDecl) {
    let TestDecl { docs: _, name: _, name_span, kind: _, body, span } = t;
    n.visit_span(span);
    n.visit_span(name_span);
    vm::walk_block(n, body);
}
