//! [route-stub] The generated `route(group)` stub.
//!
//! `use route(group)` binds a protocol `E` to **whichever member a policy
//! picks per send**: a handler `of any E` whose every member asks `Pick<E>`
//! for a member (`route_to`, in std `net`) and forwards to it. Nobody writes
//! that handler — it is the same for every protocol but for the member list,
//! so the compiler writes it: one `__Route_E` per actor effect `E` a module
//! routes, appended to the module before resolution, and `use route(g)` is
//! checked as `use __Route_E(g)` once `g`'s type names `E`
//! ([`crate::check`]).
//!
//! **Syntactic, like every expansion here.** A module gets stubs when it
//! contains a `use route(…)` statement, for every actor effect `X` it names
//! in an `actor_group<X>(…)` or `protocol<X>()` call or an `ActorGroup<X>`
//! type — which is where a group handle comes from or is written down
//! (`g: Addr<ActorGroup<X>>`). A `use route(g)` whose `X` was never
//! spelled in the module is refused by the checker, naming that fix.
//!
//! **Spans.** Every synthesized node takes a fresh one-byte span **past the
//! end of the file**: the checker's tables are keyed by span, so nothing may
//! share one (see `Spans` in `salvo_syntax::desugar`), and a diagnostic on a
//! synthesized node — which the generator should make impossible — renders
//! clamped to the file's last line rather than pointing into a declaration
//! that did not write it.
//!
//! **Imports.** The stub is appended to the *routing* module, so the names
//! it uses have to be visible there: the kit's (`ActorGroup`, `Pick`,
//! `route_to`, `key_hash` from `net`) and every type the protocol's members
//! mention. Those the module has not imported are imported for it, from the
//! module that declares them — which is the only place the generated code
//! reaches past what the program wrote, and it reaches only for names the
//! program's own protocol put in its signatures.

use std::collections::{BTreeMap, BTreeSet, HashSet};

use salvo_syntax::ast::{
    Block, Deduction, DeductionKind, DeductionTarget, EffectDecl, EffectRef, Expr, FnDecl,
    HandlerDecl, Ident, ImportDecl, Item, Module, Param, Pattern, Stmt, StructLitField,
    StructLitFieldKind, Type, TypeRef,
};
use salvo_syntax::{Diagnostic, Span};

use crate::source::SourceFile;

/// The contextual name in `use route(group)`.
pub const ROUTE: &str = "route";
/// The generated handler's name for effect `E`.
pub fn stub_name(effect: &str) -> String {
    format!("__Route_{effect}")
}
/// [route-stub] The std qualifier marking a member's key parameter.
pub const KEY_QUALIFIER: &str = "Key";

const NET: &str = "net";
const KIT: &[&str] = &["ActorGroup", "Pick", "route_to", "key_hash"];

/// Appends a `__Route_X` handler to every module that routes `X`.
pub fn expand_route_stubs(files: &[SourceFile], modules: &mut [Module]) -> Vec<(usize, Diagnostic)> {
    let mut diags: Vec<(usize, Diagnostic)> = Vec::new();
    // The routable protocols: non-generic actor effects, by name. Cloned, since
    // the modules are mutated below.
    let mut effects: BTreeMap<String, (usize, EffectDecl)> = BTreeMap::new();
    // Every exported declaration name, by module — for the imports.
    let mut exported: Vec<BTreeSet<String>> = Vec::with_capacity(modules.len());
    for (idx, module) in modules.iter().enumerate() {
        let mut names = BTreeSet::new();
        for item in &module.items {
            match item {
                Item::Effect(e) => {
                    if e.is_actor && e.generics.is_empty() && e.fns.iter().any(|f| f.is_send) {
                        effects.entry(e.name.name.clone()).or_insert((idx, e.clone()));
                    }
                    if e.exported {
                        names.insert(e.name.name.clone());
                    }
                }
                Item::Struct(s) if s.exported => {
                    names.insert(s.name.name.clone());
                }
                Item::Type(t) if t.exported => {
                    names.insert(t.name.name.clone());
                }
                Item::Qualifier(q) if q.exported => {
                    names.insert(q.name.name.clone());
                }
                _ => {}
            }
        }
        exported.push(names);
    }
    for (idx, module) in modules.iter_mut().enumerate() {
        let mut routes = false;
        let mut named: BTreeSet<String> = BTreeSet::new();
        salvo_syntax::desugar::walk_module(
            module,
            &mut |stmt| {
                if let Stmt::Use { handler, .. } = stmt {
                    if is_route_call(handler) {
                        routes = true;
                    }
                }
            },
            &mut |expr| {
                if let Expr::Call { callee, type_args, .. } = expr {
                    if let Expr::Ident(id) = callee.as_ref() {
                        if id.name == "protocol" || id.name == "actor_group" {
                            for t in type_args {
                                if let Type::Named { base, .. } = t {
                                    named.insert(base.name.name.clone());
                                }
                            }
                        }
                    }
                    for t in type_args {
                        group_args(t, &mut named);
                    }
                }
            },
        );
        if !routes {
            continue;
        }
        // Types written in declarations and `let`s: where a group handle's
        // type is spelled when it did not come from `attach` in this module.
        for item in &module.items {
            match item {
                Item::Fn(f) => fn_types(f, &mut named),
                Item::Handler(h) => {
                    for p in &h.params {
                        group_args(&p.ty, &mut named);
                    }
                    for s in &h.state {
                        group_args(&s.ty, &mut named);
                    }
                    for f in &h.fns {
                        fn_types(f, &mut named);
                    }
                }
                Item::Struct(s) => {
                    for f in &s.fields {
                        group_args(&f.ty, &mut named);
                    }
                }
                _ => {}
            }
        }
        salvo_syntax::desugar::walk_module(module, &mut |stmt| {
            if let Stmt::Let { ty: Some(ty), .. } = stmt {
                group_args(ty, &mut named);
            }
        }, &mut |_| {});
        let existing: HashSet<String> = module
            .items
            .iter()
            .filter_map(|i| match i {
                Item::Handler(h) => Some(h.name.name.clone()),
                _ => None,
            })
            .collect();
        let mut spans = Spans::new(files[idx].content.len() as u32 + 1);
        let here = &files[idx].module;
        let mut imports: BTreeSet<(String, String)> = BTreeSet::new();
        for x in &named {
            let Some((decl_idx, effect)) = effects.get(x) else { continue };
            if existing.contains(&stub_name(x)) {
                continue;
            }
            let (handler, mut errs) = build_stub(effect, &mut spans);
            diags.extend(errs.drain(..).map(|d| (idx, d)));
            // Imports the stub needs: the kit, and the protocol's own types.
            if here.0.first().map(|s| s.as_str()) != Some(NET) {
                for name in KIT {
                    imports.insert((NET.to_string(), name.to_string()));
                }
            }
            let mut mentioned: BTreeSet<String> = BTreeSet::new();
            for f in &effect.fns {
                for p in &f.params {
                    type_names(&p.ty, &mut mentioned);
                }
            }
            mentioned.insert(x.clone());
            for name in mentioned {
                // Declared here, or in `core` (visible everywhere): no import.
                let from = if exported[*decl_idx].contains(&name) {
                    Some(*decl_idx)
                } else {
                    exported.iter().position(|names| names.contains(&name))
                };
                let Some(from) = from else { continue };
                if from == idx || files[from].module.0.first().map(|s| s.as_str()) == Some("core") {
                    continue;
                }
                imports.insert((files[from].module.to_string(), name));
            }
            module.items.push(Item::Handler(handler));
        }
        // Add what is not already imported — by name, or as a whole module.
        let mut have_modules: HashSet<String> = HashSet::new();
        let mut have_names: HashSet<(String, String)> = HashSet::new();
        for item in &module.items {
            let Item::Import(imp) = item else { continue };
            let path: Vec<&str> = imp.path.iter().map(|i| i.name.as_str()).collect();
            have_modules.insert(path.join("."));
            if path.len() > 1 {
                have_names.insert((
                    path[..path.len() - 1].join("."),
                    path[path.len() - 1].to_string(),
                ));
            }
        }
        let mut new_items: Vec<Item> = Vec::new();
        for (module_path, name) in imports {
            if have_modules.contains(&module_path) || have_names.contains(&(module_path.clone(), name.clone())) {
                continue;
            }
            let span = spans.take();
            let mut path: Vec<Ident> = module_path
                .split('.')
                .map(|seg| Ident { name: seg.to_string(), span })
                .collect();
            path.push(Ident { name, span });
            new_items.push(Item::Import(ImportDecl { path, alias: None, span }));
        }
        if !new_items.is_empty() {
            new_items.append(&mut module.items);
            module.items = new_items;
        }
    }
    diags
}

/// `route(expr)` as the handler of a `use`.
pub fn is_route_call(handler: &Expr) -> bool {
    matches!(handler, Expr::Call { callee, args, .. }
        if matches!(callee.as_ref(), Expr::Ident(id) if id.name == ROUTE) && args.len() == 1)
}

fn fn_types(f: &FnDecl, out: &mut BTreeSet<String>) {
    for p in &f.params {
        group_args(&p.ty, out);
    }
    if let Some(r) = &f.return_type {
        group_args(r, out);
    }
}

/// Every `X` of an `ActorGroup<X>` inside `ty`.
fn group_args(ty: &Type, out: &mut BTreeSet<String>) {
    fn in_ref(r: &TypeRef, out: &mut BTreeSet<String>) {
        if r.name.name == "ActorGroup" {
            if let Some(Type::Named { base, .. }) = r.args.first() {
                out.insert(base.name.name.clone());
            }
        }
        for a in &r.args {
            group_args(a, out);
        }
    }
    match ty {
        Type::Named { base, qualifiers } => {
            in_ref(base, out);
            for q in qualifiers {
                in_ref(q, out);
            }
        }
        Type::QualifiedGroup { base, .. } | Type::Array { elem: base, .. } | Type::Nullable { inner: base, .. } => {
            group_args(base, out)
        }
        Type::Union { arms, .. } | Type::Tuple { elems: arms, .. } => {
            for a in arms {
                group_args(a, out);
            }
        }
        Type::Fn { params, ret, .. } => {
            for p in params {
                group_args(p, out);
            }
            group_args(ret, out);
        }
    }
}

/// Every type name `ty` mentions.
fn type_names(ty: &Type, out: &mut BTreeSet<String>) {
    fn in_ref(r: &TypeRef, out: &mut BTreeSet<String>) {
        out.insert(r.name.name.clone());
        for a in &r.args {
            type_names(a, out);
        }
    }
    match ty {
        Type::Named { base, qualifiers } => {
            in_ref(base, out);
            for q in qualifiers {
                in_ref(q, out);
            }
        }
        Type::QualifiedGroup { base, qualifiers, .. } => {
            for q in qualifiers {
                in_ref(q, out);
            }
            type_names(base, out);
        }
        Type::Array { elem: base, .. } | Type::Nullable { inner: base, .. } => type_names(base, out),
        Type::Union { arms, .. } | Type::Tuple { elems: arms, .. } => {
            for a in arms {
                type_names(a, out);
            }
        }
        Type::Fn { params, ret, .. } => {
            for p in params {
                type_names(p, out);
            }
            type_names(ret, out);
        }
    }
}

/// Fresh one-byte spans past the end of the file.
struct Spans {
    next: u32,
}

impl Spans {
    fn new(start: u32) -> Self {
        Spans { next: start }
    }
    fn take(&mut self) -> Span {
        let at = self.next;
        self.next = self.next.saturating_add(1);
        Span::new(at, at + 1)
    }
}

fn ident(name: &str, spans: &mut Spans) -> Ident {
    Ident { name: name.to_string(), span: spans.take() }
}

fn type_ref(name: &str, args: Vec<Type>, spans: &mut Spans) -> TypeRef {
    TypeRef {
        name: ident(name, spans),
        args,
        value_args: Vec::new(),
        from: Vec::new(),
        at: None,
        binder: false,
        established: false,
        alias: None,
        span: spans.take(),
    }
}

fn named(name: &str, args: Vec<Type>, spans: &mut Spans) -> Type {
    Type::Named { qualifiers: Vec::new(), base: type_ref(name, args, spans) }
}

fn call(callee: &str, args: Vec<Expr>, spans: &mut Spans) -> Expr {
    Expr::Call {
        callee: Box::new(Expr::Ident(ident(callee, spans))),
        type_args: Vec::new(),
        args,
        named: Vec::new(),
        span: spans.take(),
    }
}

/// Whether a parameter type carries the `Key` claim.
fn is_key(ty: &Type) -> bool {
    match ty {
        Type::Named { qualifiers, .. } | Type::QualifiedGroup { qualifiers, .. } => {
            qualifiers.iter().any(|q| q.name.name == KEY_QUALIFIER)
        }
        _ => false,
    }
}

/// The handler `__Route_E(group: Addr<ActorGroup<E>>) [Pick<E>] of any E`.
fn build_stub(effect: &EffectDecl, spans: &mut Spans) -> (HandlerDecl, Vec<Diagnostic>) {
    let mut diags = Vec::new();
    let e = effect.name.name.as_str();
    let group_ty = {
        let inner = named(e, Vec::new(), spans);
        let group = named("ActorGroup", vec![inner], spans);
        named("Addr", vec![group], spans)
    };
    let mut fns: Vec<FnDecl> = Vec::new();
    for member in &effect.fns {
        if !member.is_send {
            continue;
        }
        let keys: Vec<&Param> = member.params.iter().filter(|p| is_key(&p.ty)).collect();
        if keys.len() > 1 {
            diags.push(Diagnostic::error(
                format!(
                    "`{}.{}` marks {} parameters `Key`; a member routes on at most one \
                     [route-stub]",
                    e,
                    member.name.name,
                    keys.len()
                ),
                member.name.span,
            ));
        }
        let mut f = member.clone();
        f.docs.clear();
        f.exported = false;
        f.intrinsic = false;
        f.name = ident(&member.name.name, spans);
        f.span = spans.take();
        for p in &mut f.params {
            p.name = ident(&p.name.name, spans);
            respan_type(&mut p.ty, spans);
            p.span = spans.take();
        }
        if let Some(ds) = &mut f.deductions {
            for d in ds {
                respan_deduction(d, spans);
            }
        }
        if let Some(r) = &mut f.return_type {
            respan_type(r, spans);
        }
        f.effects = None;
        // let __target = route_to(group[, key_hash(k)])
        let mut route_args = vec![Expr::Ident(ident("group", spans))];
        if let Some(k) = keys.first() {
            let k = Expr::Ident(ident(&k.name.name, spans));
            route_args.push(call("key_hash", vec![k], spans));
        }
        let target_let = Stmt::Let {
            pattern: Pattern::Ident(ident("__target", spans)),
            ty: None,
            value: call("route_to", route_args, spans),
            span: spans.take(),
        };
        // __target.m(p1, …, pn)
        let forward = Expr::Call {
            callee: Box::new(Expr::Field {
                base: Box::new(Expr::Ident(ident("__target", spans))),
                field: ident(&member.name.name, spans),
                span: spans.take(),
            }),
            type_args: Vec::new(),
            args: member
                .params
                .iter()
                .filter(|p| !p.implicit)
                .map(|p| Expr::Ident(ident(&p.name.name, spans)))
                .collect(),
            named: Vec::new(),
            span: spans.take(),
        };
        f.body = Some(Block { stmts: vec![target_let, Stmt::Expr(forward)], span: spans.take() });
        fns.push(f);
    }
    let mailbox = Expr::StructLit {
        ty: Some(named("Mailbox", Vec::new(), spans)),
        fields: vec![StructLitField {
            kind: StructLitFieldKind::Named {
                name: ident("capacity", spans),
                value: Expr::Int { value: 1, long: false, span: spans.take() },
            },
            span: spans.take(),
        }],
        span: spans.take(),
    };
    let pick = EffectRef::Effect(type_ref("Pick", vec![named(e, Vec::new(), spans)], spans));
    let handler = HandlerDecl {
        docs: Vec::new(),
        exported: false,
        intrinsic: false,
        platform: false,
        threadsafe: false,
        name: ident(&stub_name(e), spans),
        generics: Vec::new(),
        params: vec![Param {
            name: ident("group", spans),
            ty: group_ty,
            variadic: false,
            implicit: false,
            span: spans.take(),
        }],
        effects: Some(vec![pick]),
        of: vec![named(e, Vec::new(), spans)],
        of_any: vec![true],
        init: None,
        mailbox: Some(mailbox),
        state: Vec::new(),
        fns,
        span: spans.take(),
    };
    (handler, diags)
}

fn respan_type(ty: &mut Type, spans: &mut Spans) {
    match ty {
        Type::Named { qualifiers, base } => {
            for q in qualifiers {
                respan_ref(q, spans);
            }
            respan_ref(base, spans);
        }
        Type::QualifiedGroup { qualifiers, base, span } => {
            for q in qualifiers {
                respan_ref(q, spans);
            }
            respan_type(base, spans);
            *span = spans.take();
        }
        Type::Union { arms, span } | Type::Tuple { elems: arms, span } => {
            for a in arms {
                respan_type(a, spans);
            }
            *span = spans.take();
        }
        Type::Array { elem: inner, span } | Type::Nullable { inner, span } => {
            respan_type(inner, spans);
            *span = spans.take();
        }
        Type::Fn { params, param_names, effects, deductions, ret, span } => {
            for p in params {
                respan_type(p, spans);
            }
            for n in param_names.iter_mut().flatten() {
                n.span = spans.take();
            }
            for eff in effects.iter_mut().flatten() {
                match eff {
                    EffectRef::Use(s) | EffectRef::Spawn(s) => *s = spans.take(),
                    EffectRef::Effect(r) | EffectRef::AnyEffect(r) => {
                        respan_ref(r, spans)
                    }
                }
            }
            for d in deductions.iter_mut().flatten() {
                respan_deduction(d, spans);
            }
            respan_type(ret, spans);
            *span = spans.take();
        }
    }
}

fn respan_ref(r: &mut TypeRef, spans: &mut Spans) {
    r.name.span = spans.take();
    for a in &mut r.args {
        respan_type(a, spans);
    }
    for a in &mut r.value_args {
        respan_type(a, spans);
    }
    for f in &mut r.from {
        f.span = spans.take();
    }
    if let Some(at) = &mut r.at {
        at.span = spans.take();
    }
    if let Some(alias) = &mut r.alias {
        alias.span = spans.take();
    }
    r.span = spans.take();
}

fn respan_deduction(d: &mut Deduction, spans: &mut Spans) {
    match &mut d.target {
        DeductionTarget::Param { name, path } => {
            name.span = spans.take();
            for p in path {
                p.span = spans.take();
            }
        }
        DeductionTarget::Result { path } => {
            for p in path {
                p.span = spans.take();
            }
        }
        DeductionTarget::Opaque => {}
    }
    match &mut d.kind {
        DeductionKind::Exhaustive { quals, reapplied } => {
            for q in quals.iter_mut().chain(reapplied.iter_mut()) {
                respan_ref(q, spans);
            }
        }
        DeductionKind::Remove(quals) => {
            for q in quals {
                respan_ref(q, spans);
            }
        }
        DeductionKind::Proj(sources) | DeductionKind::With { others: sources } => {
            for s in sources {
                s.span = spans.take();
            }
        }
        DeductionKind::Preserve(quals) => {
            for q in quals {
                respan_ref(q, spans);
            }
        }
        DeductionKind::CanBe { others, .. } => {
            for path in others {
                for s in path {
                    s.span = spans.take();
                }
            }
        }
        DeductionKind::KeepAll | DeductionKind::Moved | DeductionKind::Deferred => {}
    }
    d.span = spans.take();
}
