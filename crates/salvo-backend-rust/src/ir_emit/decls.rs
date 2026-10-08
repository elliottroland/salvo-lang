//! Declarations and types for the Rust IR emitter.

use std::collections::HashSet;

use salvo_core::types::Ty;
use salvo_ir::{Decl, ExprKind, FnDecl, FnKind, ImplDecl, InterfaceDecl, Param, PassMode, Stmt, StructDecl, TypeParam};

use super::{is_copy_ty, is_mut, is_proj, ModuleEmitter};
use crate::emit::{
    host_mod_name, platform_adapter_name, platform_trait_name, rs_ident, stateful_trait_name, stateless_trait_name,
};

/// Where a fn type is spelled: the shape of a callback differs by who holds it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum FnPos {
    /// A lent parameter of a top-level fn: `&mut impl FnMut(..)`.
    Param,
    /// A parameter of a trait member or an implicit: `&mut dyn FnMut(..)`.
    DynParam,
    /// Kept past the call (a struct field, an iterator fn's callback):
    /// `Arc<dyn Fn(..) + Send + Sync>` / `impl Fn + Send + Sync + 'static`.
    Stored,
    Owned,
    /// A local closure: annotated, borrowing what it captures.
    Local,
}

/// The mode a fn type's `i`th parameter is passed in [fn-contract].
pub(crate) fn fn_ty_param_mode(t: &Ty, i: usize) -> PassMode {
    let Ty::Fn { params, contract, .. } = t.strip_quals() else { return PassMode::Lent };
    let p = params.get(i);
    let (kept, mutable) = match contract.as_ref().and_then(|c| c.get(i)) {
        Some(c) => (c.kept, c.mutable || p.is_some_and(is_mut)),
        None => (true, p.is_some_and(is_mut)),
    };
    if kept && mutable {
        PassMode::LentMut
    } else if kept {
        PassMode::Lent
    } else {
        PassMode::Moved
    }
}

/// The names a `proj(...)` qualifier says a type borrows from.
pub(crate) fn proj_sources(t: &Ty, out: &mut Vec<String>) {
    match t {
        Ty::Qualified { quals, base } => {
            for q in quals.iter().filter(|q| q.name == "proj") {
                for a in &q.args {
                    if let Ty::ValueRef { path, .. } = a {
                        out.push(path.split('.').next().unwrap_or(path).to_string());
                    }
                }
            }
            proj_sources(base, out);
        }
        Ty::Named { args, .. } => args.iter().for_each(|a| proj_sources(a, out)),
        Ty::Union(arms) | Ty::Tuple(arms) => arms.iter().for_each(|a| proj_sources(a, out)),
        Ty::Array(e) => proj_sources(e, out),
        _ => {}
    }
}

impl<'a, 'p> ModuleEmitter<'a, 'p> {
    // ------------------------------------------------------------- types --

    pub fn ty(&mut self, t: &Ty) -> String {
        if let Ty::Named { name, args } = t {
            if args.is_empty() && self.f.render_subst.contains_key(name) {
                return self.ty(&Ty::Var(name.clone()));
            }
        }
        if !self.f.plain_vars.is_empty() {
            let s = super::strip_plain_proj(t, &self.f.plain_vars);
            if &s != t {
                let saved = std::mem::take(&mut self.f.plain_vars);
                let r = self.ty(&s);
                self.f.plain_vars = saved;
                return r;
            }
        }
        if salvo_core::literal::mentions_lit(t) {
            return self.ty(&salvo_core::literal::collapse_ty(t));
        }
        match t {
            Ty::Lit(l) => self.ty(&Ty::named(l.base())),
            Ty::ValueRef { .. } | Ty::ConstInt(_) => String::new(),
            Ty::Named { name, args } => {
                let args: Vec<Ty> = args.iter().filter(|a| !matches!(a, Ty::FnName(_))).map(|a| super::strip_nested_var_proj_in(a, true, &self.f.generics)).collect();
                if name == "Addr" && args.len() == 1 {
                    // [monitor-handler] a plain effect's addr is its handle.
                    if let Ty::Named { name: eff, .. } = args[0].strip_quals() {
                        if self.s.symbols.effects.get(eff.as_str()).is_some_and(|e| !e.is_actor) {
                            return self.ty(&args[0]);
                        }
                    }
                }
                if let Some(alias) = self.s.symbols.type_aliases.get(name.as_str()) {
                    if let Some(target) = alias.alias.as_ref() {
                        let subst: std::collections::HashMap<String, Ty> = alias.generics.iter().map(|g| g.name.clone()).zip(args.iter().cloned()).collect();
                        if let Some(expanded) = salvo_core::approx_ty(target, &subst) {
                            return self.ty(&salvo_ir::build::erase(&expanded));
                        }
                    }
                }
                if let Some(rs) = crate::intrinsics::type_name(name) {
                    if matches!(name.as_str(), "Addr" | "Pool" | "Reply" | "None" | "Never") || args.is_empty() {
                        return rs.to_string();
                    }
                    let a: Vec<String> = args.iter().map(|x| self.ty(x)).collect();
                    return format!("{rs}<{}>", a.join(", "));
                }
                let erased = self.s.erased.is_erased(salvo_core::typekey::plain(name));
                let mut a: Vec<String> = if erased { Vec::new() } else { args.iter().map(|x| self.ty(x)).collect() };
                if self.s.borrowing.contains(name) {
                    a.insert(0, self.f.lt.clone().unwrap_or_else(|| "'_".to_string()));
                }
                let base = if self.s.symbols.handlers.get(name.as_str()).is_some_and(|h| h.platform) {
                    let plain = salvo_core::typekey::plain(name);
                    match self.s.symbols.key_modules.get(name.as_str()) {
                        Some(m) => format!("{}{}", self.s.prefix(m), platform_adapter_name(&rs_ident(plain))),
                        None => platform_adapter_name(&rs_ident(plain)),
                    }
                } else {
                    self.type_path(name)
                };
                if a.is_empty() { base } else { format!("{base}<{}>", a.join(", ")) }
            }
            Ty::Qualified { quals, base } => {
                if quals.iter().any(|q| q.name == "proj") && !is_copy_ty(base) {
                    let lt = self.f.lt.clone().map(|l| format!("{l} ")).unwrap_or_default();
                    let m = if quals.iter().any(|q| q.name == "Mut") { "mut " } else { "" };
                    // The inner type's own borrows are elided.
                    let saved = self.f.lt.take();
                    let inner = self.ty(base);
                    self.f.lt = saved;
                    return format!("&{lt}{m}{inner}");
                }
                self.ty(base)
            }
            Ty::Union(_) => {
                let arms: Vec<Ty> = t.value_arms().into_iter().cloned().collect();
                let inner = match arms.len() {
                    0 => "()".to_string(),
                    1 => self.ty(&arms[0]),
                    n => {
                        self.s.union_sizes.insert(n);
                        let a: Vec<String> = arms.iter().map(|x| self.ty(x)).collect();
                        format!("crate::unions::Union{n}<{}>", a.join(", "))
                    }
                };
                if t.has_none_arm() { format!("Option<{inner}>") } else { inner }
            }
            Ty::Tuple(es) => {
                let a: Vec<String> = es.iter().map(|x| self.ty(x)).collect();
                if a.len() == 1 { format!("({},)", a[0]) } else { format!("({})", a.join(", ")) }
            }
            Ty::Array(e) => format!("Vec<{}>", self.ty(e)),
            Ty::Fn { .. } => self.fn_ty(t, FnPos::Stored),
            Ty::Var(v) => match self.f.render_subst.get(v).cloned() {
                Some(t) => {
                    let saved = std::mem::take(&mut self.f.render_subst);
                    let r = self.ty(&t);
                    self.f.render_subst = saved;
                    r
                }
                None => v.clone(),
            },
            Ty::Never => "()".to_string(),
            Ty::FnName(_) | Ty::Any | Ty::Unknown => {
                self.error(format!("a value of type `{t}` reached rust code generation"));
                "()".to_string()
            }
        }
    }

    /// A value slot's type, borrowed per `mode`: what a parameter is spelled.
    pub fn param_ty(&mut self, p: &Param, pos: FnPos) -> String {
        if matches!(p.ty.strip_quals(), Ty::Fn { .. }) {
            let pos = if p.mode == PassMode::Moved && pos == FnPos::Param { FnPos::Owned } else { pos };
            return self.fn_ty(&p.ty, pos);
        }
        let t = self.ty(&p.ty);
        if is_copy_ty(&p.ty) || is_proj(&p.ty) || p.variadic {
            return t;
        }
        // A borrowing struct carries the lifetime itself.
        let tie = self.f.tie.iter().find(|(t, _)| *t == super::body::strip_all_pub(&p.ty)).map(|(_, l)| l.clone());
        let lt = if self.s.holds_proj(&p.ty) { String::new() } else { tie.or_else(|| self.f.lt.clone()).map(|l| format!("{l} ")).unwrap_or_default() };
        match p.mode {
            PassMode::Moved => t,
            PassMode::Lent => format!("&{lt}{t}"),
            PassMode::LentMut => format!("&{lt}mut {t}"),
        }
    }

    /// A fn type at a position.
    pub fn fn_ty(&mut self, t: &Ty, pos: FnPos) -> String {
        let lends = self.s.fn_ty_lends_mut(t);
        let t = &self.s.unalias(t);
        let Ty::Fn { params, ret, effects, .. } = t.strip_quals() else { return self.ty(t) };
        let saved = self.f.lt.take();
        let mut ps: Vec<String> = Vec::new();
        for e in effects {
            let h = self.ty(e);
            ps.push(format!("&{h}"));
        }
        for (i, p) in params.iter().enumerate() {
            let mut mode = fn_ty_param_mode(t, i);
            if lends && mode == PassMode::LentMut {
                mode = PassMode::Lent;
            }
            let inner = if matches!(p.strip_quals(), Ty::Fn { .. }) { self.fn_ty(p, FnPos::DynParam) } else { self.ty(p) };
            if is_copy_ty(p) || is_proj(p) || matches!(p.strip_quals(), Ty::Fn { .. }) {
                ps.push(inner);
                continue;
            }
            let tie = self.f.tie.iter().find(|(t, _)| *t == super::body::strip_all_pub(p)).map(|(_, l)| format!("{l} ")).unwrap_or_default();
            ps.push(match mode {
                PassMode::Moved => inner,
                PassMode::Lent => format!("&{tie}{inner}"),
                PassMode::LentMut => format!("&{tie}mut {inner}"),
            });
        }
        let r = if lends {
            if ret.strip_quals().has_none_arm() { " -> Option<usize>".to_string() } else { " -> usize".to_string() }
        } else if ret.is_none_ty() {
            String::new()
        } else {
            format!(" -> {}", self.ty(ret))
        };
        self.f.lt = saved;
        let sig = format!("({}){r}", ps.join(", "));
        match pos {
            FnPos::Param | FnPos::DynParam | FnPos::Local => format!("&mut dyn FnMut{sig}"),
            FnPos::Stored => format!("std::sync::Arc<dyn Fn{sig} + Send + Sync>"),
            FnPos::Owned => format!("impl Fn{sig} + Send + Sync + 'static"),
        }
    }

    fn generics(&self, tps: &[TypeParam], bound: &str) -> Vec<String> {
        tps.iter()
            .map(|t| if (t.canbe_linear && bound == "Clone") || bound.is_empty() { t.name.clone() } else { format!("{}: {bound}", t.name) })
            .collect()
    }

    // ------------------------------------------------------------- decls --

    pub fn decl(&mut self, d: &Decl) {
        match d {
            Decl::Struct(s) => self.struct_decl(s),
            Decl::Union(u) => {
                // [platform-factory] a named union a host builds: the alias
                // the factories hang off.
                if let Some(f) = u.factories.clone() {
                    if f.nullable {
                        self.out.push_str(&format!("\n// No factories for `{}`: it admits `None`, and Rust has no inherent impl on an `Option` [platform-factory].\n", u.name));
                    } else {
                        let alias = rs_ident(&u.name);
                        let union = self.ty(&f.union);
                        self.out.push_str(&format!("\npub type {alias} = {union};\n"));
                        let code = self.factory_impl(&alias, "Self", false, &[f]);
                        self.out.push_str(&code);
                    }
                }
            }
            Decl::Interface(i) => self.interface_decl(i),
            Decl::Impl(h) => self.impl_decl(h),
            Decl::Fn(f) => {
                let code = self.fn_decl(f, None, 0);
                self.out.push_str(&code);
                if let Some(x) = f.factories.clone() {
                    let code = self.factory_impl(&salvo_core::abi::upper_camel(&f.name), "", true, &[x]);
                    self.out.push_str(&code);
                }
            }
            Decl::PlatformType(t) => {
                if t.platform {
                    let code = self.platform_type(t);
                    self.out.push_str(&code);
                }
            }
            Decl::Static(st) => self.static_decl(st),
        }
    }

    fn struct_decl(&mut self, s: &StructDecl) {
        let key = self.s.key_of(&s.id, &s.name);
        let borrowing = self.s.borrowing.contains(&key);
        let mut tps: Vec<String> = s.type_params.iter().map(|t| t.name.clone()).collect();
        if self.s.erased.is_erased(&s.name) {
            tps.clear();
        }
        let tp_names = tps.clone();
        if borrowing {
            tps.insert(0, "'s".to_string());
        }
        let generics = if tps.is_empty() { String::new() } else { format!("<{}>", tps.join(", ")) };
        let name = rs_ident(&s.name);
        let mut opaque_fields: Vec<String> = Vec::new();
        let mut no_clone = false;
        let mut fields = String::new();
        let saved = std::mem::replace(&mut self.f.lt, if borrowing { Some("'s".to_string()) } else { None });
        for f in &s.fields {
            let (opaque, uncloneable) = self.host_limits(&f.ty, 0);
            no_clone |= uncloneable;
            let fty = if matches!(f.ty.strip_quals(), Ty::Fn { .. }) {
                opaque_fields.push(f.name.clone());
                self.fn_ty(&f.ty, FnPos::Stored)
            } else {
                if opaque {
                    opaque_fields.push(f.name.clone());
                }
                self.ty(&f.ty)
            };
            fields.push_str(&format!("    pub {}: {fty},\n", rs_ident(&f.name)));
        }
        self.f.lt = saved;
        let derives = if opaque_fields.is_empty() {
            "#[derive(Clone, Debug, PartialEq)]\n"
        } else if no_clone {
            ""
        } else {
            "#[derive(Clone)]\n"
        };
        self.out.push_str(&format!("\n{derives}pub struct {name}{generics} {{\n{fields}}}\n"));
        if !opaque_fields.is_empty() {
            let bounded: Vec<String> = tps.iter().map(|t| if t.starts_with('\'') { t.clone() } else { format!("{t}: std::fmt::Debug") }).collect();
            let g = if tps.is_empty() { String::new() } else { format!("<{}>", bounded.join(", ")) };
            let mut body = format!("\nimpl{g} std::fmt::Debug for {name}{generics} {{\n    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {{\n        f.debug_struct(\"{}\")\n", s.name);
            for f in &s.fields {
                if opaque_fields.contains(&f.name) {
                    let shown = if matches!(f.ty.strip_quals(), Ty::Fn { .. }) { "<fn>" } else { "<opaque>" };
                    body.push_str(&format!("            .field(\"{}\", &\"{shown}\")\n", f.name));
                } else {
                    body.push_str(&format!("            .field(\"{}\", &self.{})\n", f.name, rs_ident(&f.name)));
                }
            }
            body.push_str("            .finish()\n    }\n}\n");
            self.out.push_str(&body);
        }
        // [wire-format] [rs-wire] The struct's codec.
        if s.has_wire_form && !borrowing {
            let args = if tp_names.is_empty() { String::new() } else { format!("<{}>", tp_names.join(", ")) };
            let g = if tp_names.is_empty() {
                String::new()
            } else {
                format!("<{}>", tp_names.iter().map(|t| format!("{t}: Clone + 'static + crate::wire::__Wire")).collect::<Vec<_>>().join(", "))
            };
            let mut enc = String::new();
            let mut dec = String::new();
            for f in &s.fields {
                let n = rs_ident(&f.name);
                enc.push_str(&format!("        crate::wire::__Wire::__enc(&self.{n}, out);\n"));
                dec.push_str(&format!("            {n}: crate::wire::__Wire::__dec(r)?,\n"));
            }
            self.out.push_str(&format!(
                "\nimpl{g} crate::wire::__Wire for {name}{args} {{\n    fn __enc(&self, out: &mut Vec<u8>) {{\n{enc}    }}\n    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {{\n        Some(Self {{\n{dec}        }})\n    }}\n}}\n"
            ));
        }
    }

    pub fn host_limits_pub(&self, t: &Ty) -> (bool, bool) {
        self.host_limits(t, 0)
    }

    /// [rs-host-fields] (no `Debug`/`PartialEq`, no `Clone`) of a field type.
    fn host_limits(&self, t: &Ty, depth: usize) -> (bool, bool) {
        if depth > 6 {
            return (false, false);
        }
        match t.strip_quals() {
            Ty::Named { name, args } => {
                let (mut o, mut u) = (false, false);
                if name == "Reply" {
                    o = true;
                    u = true;
                } else if let Some(d) = self.s.symbols.intrinsic_types.get(name.as_str()).filter(|d| d.platform) {
                    let value = !d.linear && d.auto_qualifiers.iter().any(|q| q.name.name == "Mut");
                    o = !value;
                    u = d.linear;
                } else if let Some(st) = self.s.struct_decl(name) {
                    for f in &st.fields {
                        let (a, b) = self.host_limits(&f.ty, depth + 1);
                        o |= a;
                        u |= b;
                    }
                }
                for a in args {
                    let (x, y) = self.host_limits(a, depth + 1);
                    o |= x;
                    u |= y;
                }
                (o, u)
            }
            Ty::Union(arms) | Ty::Tuple(arms) => arms.iter().fold((false, false), |(o, u), a| {
                let (x, y) = self.host_limits(a, depth + 1);
                (o || x, u || y)
            }),
            Ty::Array(e) => self.host_limits(e, depth + 1),
            Ty::Fn { .. } => (true, false),
            _ => (false, false),
        }
    }

    /// An interface member's signature, `RECV` standing for the receiver.
    /// [rs-loc] The signature of a lending member's locator face: its
    /// anchor lent, the answer a position.
    pub(super) fn member_loc_sig(&mut self, m: &salvo_ir::Member) -> Option<String> {
        let a = self.s.member_lend_param(m)?;
        let mut params = self.s.unalias_params(&m.params);
        params[a].mode = PassMode::Lent;
        let loc = salvo_ir::Member {
            name: m.name.clone(),
            emitted_name: format!("{}__loc", m.emitted_name),
            type_params: Vec::new(),
            params,
            ret: Ty::none(),
            send: false,
            result_check: None,
            factories: Vec::new(),
            span: m.span,
        };
        let sig = self.member_sig(&loc);
        let position = if m.ret.strip_quals().has_none_arm() { "Option<usize>" } else { "usize" };
        Some(format!("{sig} -> {position}"))
    }

    pub(super) fn member_sig(&mut self, m: &salvo_ir::Member) -> String {
        let mut ps: Vec<String> = Vec::new();
        let params = self.s.unalias_params(&m.params);
        let lt = self.ret_lifetime(&params, &m.ret, &[], true);
        for p in &params {
            let borrowed = lt.as_ref().is_some_and(|(_, b)| b.contains(&p.local.0));
            self.f.lt = if borrowed { lt.as_ref().map(|(l, _)| l.clone()) } else { None };
            let t = self.param_ty(p, FnPos::DynParam);
            ps.push(format!("{}: {t}", rs_ident(&p.local.0)));
        }
        self.f.lt = lt.as_ref().map(|(l, _)| l.clone());
        let ret = self.ret_ty(&m.ret);
        self.f.lt = None;
        let lg = if lt.is_some() { "<'a>" } else { "" };
        let ps = if ps.is_empty() { String::new() } else { format!(", {}", ps.join(", ")) };
        format!("fn {}{lg}(&RECV{ps}){ret}", rs_ident(&m.emitted_name))
    }

    pub fn ret_ty(&mut self, t: &Ty) -> String {
        if t.is_none_ty() || matches!(t, Ty::Never) {
            String::new()
        } else {
            format!(" -> {}", self.ty(t))
        }
    }

    /// The lifetime a signature names when its result borrows, and the
    /// parameters it ties: `None` when elision says it (one reference in).
    fn ret_lifetime(&self, params: &[Param], ret: &Ty, borrows: &[usize], receiver: bool) -> Option<(String, Vec<String>)> {
        if !self.s.holds_proj(ret) {
            return None;
        }
        let refs: Vec<&Param> = params
            .iter()
            .filter(|p| !is_copy_ty(&p.ty) && (p.mode != PassMode::Moved || is_proj(&p.ty) || self.s.holds_proj(&p.ty)) && !matches!(p.ty.strip_quals(), Ty::Fn { .. }))
            .collect();
        let mut names: Vec<String> = borrows.iter().filter_map(|i| params.get(*i)).map(|p| p.local.0.clone()).collect();
        proj_sources(ret, &mut names);
        names.retain(|n| refs.iter().any(|p| &p.local.0 == n));
        if names.is_empty() {
            // No source named: the first borrowed parameter.
            names = refs.iter().take(1).map(|p| p.local.0.clone()).collect();
        }
        let sources = refs.iter().map(|p| if p.mode != PassMode::Moved && !is_proj(&p.ty) && self.s.holds_proj(&p.ty) { 2 } else { 1 }).sum::<usize>();
        if !receiver && sources == 1 && params.iter().all(|p| !matches!(p.ty.strip_quals(), Ty::Fn { .. }) || p.mode == PassMode::Moved) {
            return None;
        }
        Some(("'a".to_string(), names))
    }

    // -------------------------------------------------------- interfaces --

    fn interface_decl(&mut self, i: &InterfaceDecl) {
        let erased = self.s.erased.is_erased(&i.name);
        let tps: Vec<String> = if erased { Vec::new() } else { i.type_params.iter().map(|t| t.name.clone()).collect() };
        let g = if tps.is_empty() { String::new() } else { format!("<{}>", tps.join(", ")) };
        let gs = if tps.is_empty() { String::new() } else { format!("<{}>", tps.iter().map(|t| format!("{t}: 'static")).collect::<Vec<_>>().join(", ")) };
        let name = rs_ident(&i.name);
        for m in &i.members {
            if !m.type_params.is_empty() {
                self.error(format!("effect member `{}` has its own generic parameters, which the rust backend cannot dispatch dynamically yet", m.name));
            }
        }
        let mut sigs: Vec<String> = i.members.iter().map(|m| self.member_sig(m)).collect();
        // [rs-loc] A lending member's locator face, beside its natural one.
        let mut loc_sigs: Vec<(usize, String)> = Vec::new();
        for (k, m) in i.members.iter().enumerate() {
            if self.s.member_lends_mut(i, m) {
                if let Some(sig) = self.member_loc_sig(m) {
                    loc_sigs.push((k, sig));
                }
            }
        }
        let plain_sigs = sigs.len();
        sigs.extend(loc_sigs.iter().map(|(_, s)| s.clone()));
        let stateless = stateless_trait_name(&i.name);
        let stateful = stateful_trait_name(&i.name);
        let mut out = format!("\npub trait {stateless}{g}: Send + Sync {{\n");
        for s in &sigs {
            out.push_str(&format!("    {};\n", s.replace("&RECV", "&self")));
        }
        out.push_str(&format!("}}\n\npub trait {stateful}{g}: Send {{\n"));
        for s in &sigs {
            out.push_str(&format!("    {};\n", s.replace("&RECV", "&mut self")));
        }
        out.push_str("}\n");
        // [effect-handle] [rs-handle] The handle.
        let inner = format!("__Inner_{name}");
        out.push_str(&format!(
            "\npub struct {name}{gs} {{\n    inner: {inner}{g},\n}}\n\n\
             pub enum {inner}{gs} {{\n    Shared(std::sync::Arc<dyn {stateless}{g}>),\n    Locked(std::sync::Arc<std::sync::Mutex<dyn {stateful}{g}>>),\n}}\n\n\
             impl{gs} Clone for {name}{g} {{\n    fn clone(&self) -> Self {{\n        Self {{ inner: match &self.inner {{\n            \
             {inner}::Shared(h) => {inner}::Shared(h.clone()),\n            {inner}::Locked(h) => {inner}::Locked(h.clone()),\n        }} }}\n    }}\n}}\n\n\
             impl{gs} {name}{g} {{\n    \
             pub fn shared<__H: {stateless}{g} + 'static>(inner: __H) -> Self {{\n        Self {{ inner: {inner}::Shared(std::sync::Arc::new(inner)) }}\n    }}\n    \
             pub fn share_shared(inner: std::sync::Arc<dyn {stateless}{g}>) -> Self {{\n        Self {{ inner: {inner}::Shared(inner) }}\n    }}\n    \
             pub fn locked<__H: {stateful}{g} + 'static>(inner: __H) -> Self {{\n        Self {{ inner: {inner}::Locked(std::sync::Arc::new(std::sync::Mutex::new(inner))) }}\n    }}\n    \
             pub fn share_locked(inner: std::sync::Arc<std::sync::Mutex<dyn {stateful}{g}>>) -> Self {{\n        Self {{ inner: {inner}::Locked(inner) }}\n    }}\n"
        ));
        let forwarded: Vec<(&salvo_ir::Member, String, String)> = i
            .members
            .iter()
            .zip(&sigs[..plain_sigs])
            .map(|(m, sig)| (m, sig.clone(), rs_ident(&m.emitted_name)))
            .chain(loc_sigs.iter().map(|(k, sig)| (&i.members[*k], sig.clone(), format!("{}__loc", rs_ident(&i.members[*k].emitted_name)))))
            .collect();
        for (m, sig, mname) in forwarded {
            let args: Vec<String> = m.params.iter().map(|p| rs_ident(&p.local.0)).collect();
            let a = args.join(", ");
            out.push_str(&format!(
                "    pub {} {{\n        match &self.inner {{\n            {inner}::Shared(h) => h.{mname}({a}),\n            {inner}::Locked(h) => h.lock().unwrap().{mname}({a}),\n        }}\n    }}\n",
                sig.replace("&RECV", "&self")
            ));
        }
        out.push_str("}\n");
        self.out.push_str(&out);
        if i.actor {
            self.actor_interface(i);
        }
        self.platform_interface(i, &sigs);
        // [platform-factory] [rs-platform-factory] a platform-handled member's
        // factories: the result's and the `Reply<T>` payloads' together.
        if self.s.platform_effects.contains_key(&i.name) {
            for m in &i.members {
                if !m.factories.is_empty() {
                    let code = self.factory_impl(&salvo_core::abi::upper_camel(&m.name), "", true, &m.factories);
                    self.out.push_str(&code);
                }
            }
        }
    }

    /// [platform-type] [rs-platform-type] A platform type is the host's struct
    /// of the same name, re-exported here, with a static assertion of the
    /// contract its kind promises — so a host type that is not `Send` (or
    /// `Clone`, or `Sync`) is the host compiler's error at this line.
    fn platform_type(&mut self, t: &salvo_ir::PlatformTypeDecl) -> String {
        let module = self.module.path.clone();
        self.s.platform_hosts.insert(module.clone());
        let name = rs_ident(&t.name);
        let sfx = salvo_core::naming::each_suffix(self.s.program, &module, &t.name);
        // [platform-generic] A generic one is asserted at a sample argument.
        let sample = if t.type_params.is_empty() {
            name.clone()
        } else {
            format!("{name}<{}>", t.type_params.iter().map(|_| "i32").collect::<Vec<_>>().join(", "))
        };
        let mut bounds = vec!["Send", "'static"];
        if !t.linear {
            bounds.push("Clone");
        }
        // [platform-value-type] A value type is printed, compared and hashed
        // as part of the structs holding it.
        if !t.linear && t.canbe_mut {
            bounds.extend(["std::fmt::Debug", "PartialEq", "Eq"]);
            if !t.slots {
                bounds.push("std::hash::Hash");
            }
        }
        if t.threadsafe {
            bounds.push("Sync");
        }
        let host = host_mod_name(&module);
        let mut out = format!(
            "\n/// [platform-type] The host's `{name}`.\npub use crate::{host}::{name};\n\
             const _: fn() = || {{ fn __contract<T: {}>() {{}} __contract::<{sample}>(); }};\n",
            bounds.join(" + ")
        );
        // [platform-iterable] The host's loop, asserted at the sample.
        if t.iterable {
            let subst: std::collections::HashMap<String, Ty> =
                t.type_params.iter().map(|g| (g.name.clone(), Ty::Named { name: "Int".to_string(), args: Vec::new() })).collect();
            let elem = match &t.iter_elem {
                Some(e) => {
                    let e = subst_ty(e, &subst);
                    self.ty(&e)
                }
                None => "()".to_string(),
            };
            out.push_str(&format!(
                "const _: fn() = || {{ fn __each(x: &{sample}) -> impl Iterator<Item = {elem}> + '_ \
                 {{ crate::{host}::each{sfx}(x).map(|e| e.clone()) }} let _ = __each; }};\n"
            ));
            if !t.type_params.is_empty() {
                out.push_str(&format!(
                    "const _: fn() = || {{ fn __each_ref(x: &{sample}) -> impl Iterator<Item = &{elem}> + '_ \
                     {{ crate::{host}::each{sfx}(x) }} fn __each_mut(x: &mut {sample}) -> impl Iterator<Item = &mut {elem}> + '_ \
                     {{ crate::{host}::each{sfx}_mut(x) }} fn __into_each(x: {sample}) -> impl Iterator<Item = {elem}> \
                     {{ crate::{host}::into_each{sfx}(x) }} let _ = (__each_ref, __each_mut, __into_each); }};\n"
                ));
            }
        }
        out
    }

    /// [platform-factory] [rs-platform-factory] `impl Target { pub fn arm(value) -> Ret }`,
    /// one per arm; `ret` empty means each set's own union type, `declare`
    /// declares the unit struct the factories hang off.
    fn factory_impl(&mut self, target: &str, ret: &str, declare: bool, sets: &[salvo_core::abi::Factories]) -> String {
        let mut out = String::from("\n/// Factories for the host: one per arm of the union [platform-factory].\n");
        if declare {
            let taken = self.module.decls.iter().any(|d| match d {
                Decl::Struct(s) => rs_ident(&s.name) == target,
                Decl::Interface(i) => rs_ident(&i.name) == target,
                Decl::Union(u) => rs_ident(&u.name) == target,
                Decl::PlatformType(t) => rs_ident(&t.name) == target,
                _ => false,
            });
            if taken {
                self.error(format!("the factories of `{target}` would be named like the type `{target}`: rename one [platform-factory]"));
                return String::new();
            }
            out.push_str(&format!("pub struct {target};\n\n"));
        }
        out.push_str(&format!("impl {target} {{\n"));
        let mut seen: Vec<String> = Vec::new();
        for set in sets {
            self.s.union_sizes.insert(set.arity);
            let ret = if ret.is_empty() { self.ty(&set.union) } else { ret.to_string() };
            for name in &set.dropped {
                out.push_str(&format!("    // no `{}`: two arms would share the name; build them as `Union{}::Uk(…)`\n", rs_ident(name), set.arity));
            }
            for f in &set.arms {
                if seen.contains(&f.name) {
                    continue;
                }
                seen.push(f.name.clone());
                let param = self.ty(&f.param);
                let mut build = format!("crate::unions::Union{}::U{}(value)", set.arity, f.arm + 1);
                if set.nullable {
                    build = format!("Some({build})");
                }
                let checks = match &f.check {
                    Some(check) => {
                        let what = format!("`{target}::{}`'s argument", rs_ident(&f.name));
                        let body = self.boundary_check(check, "__c", &what, 3);
                        format!("        {{\n            let __c = &value;\n{body}        }}\n")
                    }
                    None => String::new(),
                };
                out.push_str(&format!("    pub fn {}(value: {param}) -> {ret} {{\n{checks}        {build}\n    }}\n", rs_ident(&f.name)));
            }
        }
        out.push_str("}\n");
        out
    }

    /// [platform-abi] [rs-platform-handler] The host-facing traits and the
    /// `__Platform_E<T>` adapter of an effect some platform handler implements.
    fn platform_interface(&mut self, i: &InterfaceDecl, sigs: &[String]) {
        let Some(how) = self.s.platform_effects.get(&i.name).cloned() else { return };
        let name = rs_ident(&i.name);
        let adapter = platform_adapter_name(&name);
        let mut out = format!("\n/// The adapter a `use` of a platform handler of `{}` constructs [platform-abi].\npub struct {adapter}<T>(pub T);\n", i.name);
        let checking = !self.s.abi;
        let mut bodies: Vec<String> = Vec::new();
        for m in &i.members {
            let mut args: Vec<String> = Vec::new();
            for p in &m.params {
                let pname = rs_ident(&p.local.0);
                let payload = match p.ty.strip_quals() {
                    Ty::Named { name, args } if name == "Reply" && args.len() == 1 => Some(args[0].clone()),
                    _ => None,
                };
                match (p.check.clone().filter(|_| checking), payload) {
                    (Some(plan), Some(payload)) => {
                        let ty = self.ty(&payload);
                        let what = format!("what the host sent on `{}.{}`", i.name, m.name);
                        let body = self.boundary_check(&plan, "__c", &what, 4);
                        args.push(format!(
                            "{pname}.checked(std::sync::Arc::new(|__any: &dyn std::any::Any| {{\n            if let Some(__c) = __any.downcast_ref::<{ty}>() {{\n{body}            }}\n        }}))"
                        ));
                    }
                    _ => args.push(pname),
                }
            }
            let call = format!("self.0.{}({})", rs_ident(&m.emitted_name), args.join(", "));
            bodies.push(match m.result_check.clone().filter(|_| checking) {
                Some(plan) => {
                    let what = format!("`{}.{}`'s result", i.name, m.name);
                    let checks = self.boundary_check(&plan, "__c", &what, 3);
                    format!("let __r = {call};\n        {{\n            let __c = &__r;\n{checks}        }}\n        __r")
                }
                None => call,
            });
        }
        let faces = [
            (how.serialized, platform_trait_name(&name, false), stateful_trait_name(&i.name), "&mut self", "Send"),
            (how.threadsafe, platform_trait_name(&name, true), stateless_trait_name(&i.name), "&self", "Send + Sync"),
        ];
        for (wanted, host_trait, own_trait, recv, bounds) in faces {
            if !wanted {
                continue;
            }
            out.push_str(&format!(
                "\n/// What a `{}platform handler` of `{}` implements [platform-abi].\npub trait {host_trait}: {bounds} {{\n",
                if recv == "&self" { "threadsafe " } else { "" },
                i.name
            ));
            for (m, sig) in i.members.iter().zip(sigs) {
                let s = sig.replace("&RECV", recv);
                // [platform-never] The host's member answering `Never` is `-> !`.
                let s = if matches!(m.ret, Ty::Never) { format!("{s} -> !") } else { s };
                out.push_str(&format!("    {s};\n"));
            }
            out.push_str(&format!("}}\n\nimpl<T: {host_trait}> {own_trait} for {adapter}<T> {{\n"));
            for (sig, body) in sigs.iter().zip(&bodies) {
                out.push_str(&format!("    {} {{\n        {body}\n    }}\n", sig.replace("&RECV", recv)));
            }
            out.push_str("}\n");
        }
        self.out.push_str(&out);
    }

    // ------------------------------------------------------------- impls --

    fn impl_decl(&mut self, h: &ImplDecl) {
        if h.intrinsic {
            self.error(format!("intrinsic handler `{}` has no rust lowering", h.name));
            return;
        }
        if h.platform {
            // [platform-handler] the host's struct behind the effect's adapter.
            let Some(Ty::Named { name: face, .. }) = h.faces.first().map(|f| f.strip_quals().clone()) else { return };
            if !self.s.abi && salvo_core::host_file(&self.s.program.companions, &self.module.path).is_none() {
                return;
            }
            self.s.platform_hosts.insert(self.module.path.clone());
            let host = format!("crate::{}::{}", host_mod_name(&self.module.path), rs_ident(&h.name));
            let alias = platform_adapter_name(&rs_ident(&h.name));
            let adapter = format!("{}{}", self.type_prefix(&face), platform_adapter_name(&rs_ident(salvo_core::typekey::plain(&face))));
            let mut ps = Vec::new();
            for p in &h.ctor_params {
                let t = self.ty(&p.ty);
                ps.push(format!("{}: {t}", rs_ident(&p.local.0)));
            }
            let args: Vec<String> = h.ctor_params.iter().map(|p| rs_ident(&p.local.0)).collect();
            self.out.push_str(&format!(
                "\npub type {alias} = {adapter}<{host}>;\n\nimpl {alias} {{\n    pub fn new({}) -> Self {{\n        {adapter}({host}::new({}))\n    }}\n}}\n",
                ps.join(", "),
                args.join(", ")
            ));
            return;
        }
        let actor_faces = self.actor_faces(h);
        let is_actor = !actor_faces.is_empty();
        if is_actor && actor_faces.len() != h.faces.len() {
            self.error(format!("handler `{}` mixes actor and plain effects, which the rust backend does not render", h.name));
            return;
        }
        // [mixed-handler] [rs-mixed] The servant is an actor of its send
        // members; the faces are worn by the façade.
        let mixed = self.is_mixed(h);
        let is_actor = is_actor || mixed;
        let name = rs_ident(&h.name);
        let tps: Vec<String> = if self.s.erased.is_erased(&h.name) { Vec::new() } else { h.type_params.iter().map(|t| t.name.clone()).collect() };
        let g_args = if tps.is_empty() { String::new() } else { format!("<{}>", tps.join(", ")) };
        let g_bounded = if tps.is_empty() { String::new() } else { format!("<{}>", tps.iter().map(|t| format!("{t}: Clone + Send + Sync + 'static")).collect::<Vec<_>>().join(", ")) };
        let mut fields: Vec<(String, String)> = Vec::new();
        let mut ctor: Vec<String> = Vec::new();
        let mut inits: Vec<String> = Vec::new();
        for p in &h.ctor_params {
            let n = rs_ident(&p.local.0);
            if matches!(p.ty.strip_quals(), Ty::Fn { .. }) {
                let t = self.fn_ty(&p.ty, FnPos::Owned);
                let dynt = t.trim_start_matches("impl ").trim_end_matches(" + 'static").to_string();
                fields.push((n.clone(), format!("std::boxed::Box<dyn {dynt}>")));
                ctor.push(format!("{n}: {t}"));
                inits.push(format!("{n}: std::boxed::Box::new({n})"));
            } else {
                let t = self.ty(&p.ty);
                fields.push((n.clone(), t.clone()));
                ctor.push(format!("{n}: {t}"));
                inits.push(n);
            }
        }
        for (i, d) in h.deps.iter().enumerate() {
            let t = self.ty(d);
            fields.push((format!("__dep{i}"), t.clone()));
            ctor.push(format!("__dep{i}: {t}"));
            inits.push(format!("__dep{i}"));
        }
        // State initializers see the constructor parameters as locals.
        self.f = super::body::FnState::default();
        for p in &h.ctor_params {
            self.f.kinds.insert(p.local.0.clone(), super::body::Kind::Owned);
        }
        let mut state_inits = Vec::new();
        for f in &h.state {
            let t = self.ty(&f.ty);
            fields.push((rs_ident(&f.name), t));
            let init = match &f.default {
                Some(e) => self.value(e, 3),
                None => "Default::default()".to_string(),
            };
            state_inits.push(format!("{}: {init}", rs_ident(&f.name)));
        }
        let mut actor_inits = Vec::new();
        if is_actor {
            let (fs, is) = self.actor_fields(h, &actor_faces);
            fields.extend(fs);
            actor_inits = is;
        }
        let all: String = fields.iter().map(|(_, t)| t.as_str()).collect::<Vec<_>>().join(" ");
        let phantom: Vec<&String> = tps.iter().filter(|t| !mentions_word(&all, t)).collect();
        let shareable = !is_actor && h.state.is_empty() && !h.ctor_params.iter().any(|p| matches!(p.ty.strip_quals(), Ty::Fn { .. }));
        let mut out = format!("\n{}pub struct {name}{g_bounded} {{\n", if shareable { "#[derive(Clone)]\n" } else { "" });
        for (n, t) in &fields {
            out.push_str(&format!("    {n}: {t},\n"));
        }
        for t in &phantom {
            out.push_str(&format!("    __phantom_{t}: std::marker::PhantomData<{t}>,\n"));
        }
        out.push_str("}\n");
        // The constructor: the parameters bound first, so state initializers
        // can read them.
        let mut new_body = String::new();
        let ctor_names: Vec<String> = h.ctor_params.iter().map(|p| rs_ident(&p.local.0)).collect();
        let mut field_inits: Vec<String> = Vec::new();
        for (i, init) in inits.iter().enumerate() {
            if i < ctor_names.len() && !h.state.is_empty() && !init.contains("Box::new") {
                field_inits.push(format!("{0}: {0}.clone()", ctor_names[i]));
            } else {
                field_inits.push(init.clone());
            }
        }
        field_inits.extend(state_inits);
        field_inits.extend(actor_inits);
        for t in &phantom {
            field_inits.push(format!("__phantom_{t}: std::marker::PhantomData"));
        }
        let built = format!("Self {{\n            {}\n        }}", field_inits.join(",\n            "));
        // [handler-init] run once, right after construction.
        let mut init_fn = String::new();
        if let Some(init) = &h.init {
            // An actor's `init` is its first message [handler-init].
            if is_actor {
                new_body.push_str(&format!("        {built}\n"));
            } else {
                new_body.push_str(&format!("        let mut __s = {built};\n        __s.init();\n        __s\n"));
            }
            self.f = super::body::FnState::default();
            self.f.current_impl = Some(self.s.key_of(&h.id, &h.name));
            self.f.generics = h.type_params.iter().map(|t| t.name.clone()).collect();
            for p in &h.ctor_params {
                self.f.kinds.insert(p.local.0.clone(), super::body::Kind::SelfField);
            }
            for (i, _) in h.deps.iter().enumerate() {
                self.f.kinds.insert(format!("__dep{i}"), super::body::Kind::SelfField);
            }
            for s in &h.state {
                self.f.kinds.insert(s.name.clone(), super::body::Kind::SelfField);
            }
            self.f.ret = Some(Ty::none());
            let body = match &init.body {
                Some(b) => self.fn_body(b, 2),
                None => String::new(),
            };
            init_fn = format!("    fn init(&mut self) {{\n{body}    }}\n");
        } else {
            new_body.push_str(&format!("        {built}\n"));
        }
        // [actor-private-send] Private send members are inherent methods.
        let mut privs = String::new();
        for m in Self::private_sends(h, &actor_faces) {
            self.f = super::body::FnState::default();
            self.f.current_impl = Some(self.s.key_of(&h.id, &h.name));
            self.f.generics = h.type_params.iter().map(|t| t.name.clone()).collect();
            for p in &h.ctor_params {
                self.f.kinds.insert(p.local.0.clone(), super::body::Kind::SelfField);
            }
            for (i, _) in h.deps.iter().enumerate() {
                self.f.kinds.insert(format!("__dep{i}"), super::body::Kind::SelfField);
            }
            for s in &h.state {
                self.f.kinds.insert(s.name.clone(), super::body::Kind::SelfField);
            }
            let mut ps = Vec::new();
            for p in m.params.iter().skip(m.effect_params) {
                // A message's payload is owned.
                let p = &Param { mode: PassMode::Moved, ..p.clone() };
                self.f.bind_param(&p.local.0, p);
                let t = self.param_ty(p, FnPos::DynParam);
                ps.push(format!(", {}: {t}", rs_ident(&p.local.0)));
            }
            self.f.ret = Some(m.ret.clone());
            let body = match &m.body {
                Some(b) => self.fn_body(b, 2),
                None => String::new(),
            };
            privs.push_str(&format!("    fn {}(&mut self{}) {{\n{body}    }}\n", rs_ident(&m.name), ps.join("")));
        }
        out.push_str(&format!("\nimpl{g_bounded} {name}{g_args} {{\n    pub fn new({}) -> Self {{\n{new_body}    }}\n{init_fn}{privs}}}\n", ctor.join(", ")));
        let stateful = h.stateful;
        for face in &h.faces {
            if mixed {
                break;
            }
            let Ty::Named { name: fname, args } = face.strip_quals() else { continue };
            let Some(iface) = self.s.interface(fname) else {
                self.error(format!("handler `{}`: face `{fname}` not found", h.name));
                continue;
            };
            let prefix = self.type_prefix(fname);
            let tr = if stateful { stateful_trait_name(&iface.name) } else { stateless_trait_name(&iface.name) };
            let ta: Vec<String> = if self.s.erased.is_erased(&iface.name) { Vec::new() } else { args.iter().map(|a| self.ty(a)).collect() };
            let ta = if ta.is_empty() { String::new() } else { format!("<{}>", ta.join(", ")) };
            out.push_str(&format!("\nimpl{g_bounded} {prefix}{tr}{ta} for {name}{g_args} {{\n"));
            for m in &iface.members {
                // [effect-member-overload] the body whose parameters match.
                let subst: std::collections::HashMap<String, Ty> = iface.type_params.iter().map(|t| t.name.clone()).zip(args.iter().cloned()).collect();
                let cands: Vec<&FnDecl> = h.members.iter().filter(|f| f.name == m.name && self.member_matches(f, m)).collect();
                let pick = if cands.len() > 1 {
                    cands.iter().copied().find(|f| {
                        let own: Vec<&Param> = f.params.iter().skip(f.effect_params).collect();
                        own.iter().zip(&m.params).all(|(a, b)| super::body::strip_all_pub(&a.ty) == super::body::strip_all_pub(&subst_ty(&b.ty, &subst)))
                    })
                } else {
                    cands.first().copied()
                };
                let Some(body) = pick else {
                    self.error(format!("handler `{}` has no member `{}`", h.name, m.name));
                    continue;
                };
                let code = self.member_fn(h, body, m, stateful, args, iface);
                out.push_str(&code);
            }
            out.push_str("}\n");
        }
        if is_actor {
            out.push_str(&self.actor_body(h, &actor_faces));
        }
        if mixed {
            out.push_str(&self.facade(h));
        }
        self.out.push_str(&out);
    }

    /// [mixed-handler] [rs-mixed] `__Fac_H`: the servant's addr and the
    /// constructor parameters, wearing every face with the sync members.
    fn facade(&mut self, h: &ImplDecl) -> String {
        let hn = rs_ident(&h.name);
        let fac = format!("__Fac_{hn}");
        let mut out = format!("\n#[derive(Clone)]\npub struct {fac} {{\n    pub __addr: usize,\n");
        for p in &h.ctor_params {
            let t = self.ty(&p.ty);
            out.push_str(&format!("    pub {}: {t},\n", rs_ident(&p.local.0)));
        }
        out.push_str("}\n");
        for face in &h.faces {
            let Ty::Named { name: fname, args } = face.strip_quals() else { continue };
            let Some(iface) = self.s.interface(fname) else { continue };
            let prefix = self.type_prefix(fname);
            let tr = stateless_trait_name(&iface.name);
            let ta: Vec<String> = if self.s.erased.is_erased(&iface.name) { Vec::new() } else { args.iter().map(|a| self.ty(a)).collect() };
            let ta = if ta.is_empty() { String::new() } else { format!("<{}>", ta.join(", ")) };
            out.push_str(&format!("\nimpl {prefix}{tr}{ta} for {fac} {{\n"));
            for m in &iface.members {
                let Some(body) = h.members.iter().find(|f| f.name == m.name && !f.send && self.member_matches(f, m)) else { continue };
                self.f.in_facade = true;
                let code = self.member_fn(h, body, m, false, args, iface);
                out.push_str(&code);
            }
            out.push_str("}\n");
        }
        out
    }

    fn member_matches(&self, f: &FnDecl, m: &salvo_ir::Member) -> bool {
        let declared = f.params.len() - f.effect_params;
        declared == m.params.len() || f.params.len() == m.params.len()
    }

    /// The module prefix of a type key.
    pub fn type_prefix(&self, key: &str) -> String {
        match self.s.symbols.key_modules.get(key) {
            Some(m) => self.s.prefix(m),
            None => String::new(),
        }
    }

    /// A handler member implementing an interface member: the trait's
    /// signature (substituted with the face's arguments), the body's code.
    fn member_fn(&mut self, h: &ImplDecl, f: &FnDecl, m: &salvo_ir::Member, stateful: bool, face_args: &[Ty], iface: &InterfaceDecl) -> String {
        let mut out = self.member_fn_mode(h, f, m, stateful, face_args, iface, false);
        // [rs-loc] A lending member also answers where its result is.
        if self.s.member_lends_mut(iface, m) {
            out.push_str(&self.member_fn_mode(h, f, m, stateful, face_args, iface, true));
        }
        out
    }

    fn member_fn_mode(&mut self, h: &ImplDecl, f: &FnDecl, m: &salvo_ir::Member, stateful: bool, face_args: &[Ty], iface: &InterfaceDecl, loc: bool) -> String {
        let subst: std::collections::HashMap<String, Ty> = iface.type_params.iter().map(|t| t.name.clone()).zip(face_args.iter().cloned()).collect();
        let sig_member = salvo_ir::Member {
            params: m.params.clone(),
            ret: m.ret.clone(),
            name: m.name.clone(),
            emitted_name: m.emitted_name.clone(),
            type_params: Vec::new(),
            send: m.send,
            result_check: None,
            factories: Vec::new(),
            span: m.span,
        };
        self.f.render_subst = subst.clone();
        let anchor = if loc { self.s.member_lend_param(m) } else { None };
        let sig = if loc {
            self.member_loc_sig(&sig_member).unwrap_or_default()
        } else {
            self.member_sig(&sig_member)
        }
        .replace("&RECV", if stateful { "&mut self" } else { "&self" });
        let facade = self.f.in_facade;
        self.f = super::body::FnState::default();
        self.f.in_facade = facade;
        self.f.render_subst = subst.clone();
        self.f.current_impl = Some(self.s.key_of(&h.id, &h.name));
        self.f.generics = h.type_params.iter().chain(&f.type_params).map(|t| t.name.clone()).collect();
        for p in &h.ctor_params {
            self.f.kinds.insert(p.local.0.clone(), super::body::Kind::SelfField);
        }
        for (i, _) in h.deps.iter().enumerate() {
            self.f.kinds.insert(format!("__dep{i}"), super::body::Kind::SelfField);
        }
        for s in &h.state {
            self.f.kinds.insert(s.name.clone(), super::body::Kind::SelfField);
        }
        // The body's own parameters, under the trait's spelling.
        let own: Vec<&Param> = f.params.iter().skip(f.effect_params).collect();
        let mut tparams = self.s.unalias_params(&sig_member.params);
        if let Some(a) = anchor {
            // [rs-loc] The locator reads its anchor.
            tparams[a].mode = PassMode::Lent;
        }
        for (p, tp) in own.iter().zip(&tparams) {
            self.f.bind_param(&p.local.0, tp);
        }
        if let Some(a) = anchor {
            if let Some(p) = own.get(a) {
                self.f.loc = Some((p.local.0.clone(), sig_member.ret.strip_quals().has_none_arm()));
            }
        }
        self.f.ret = Some(sig_member.ret.clone());
        self.f.ret_lt = self.s.holds_proj(&sig_member.ret);
        let body = match &f.body {
            Some(b) => self.fn_body(b, 2),
            None => String::new(),
        };
        // A body parameter named differently from the trait's is rebound.
        let mut rebind = String::new();
        for (p, tp) in own.iter().zip(&sig_member.params) {
            if p.local.0 != tp.local.0 {
                rebind.push_str(&format!("        let {} = {};\n", rs_ident(&p.local.0), rs_ident(&tp.local.0)));
            }
        }
        format!("    {sig} {{\n{rebind}{body}    }}\n")
    }

    // --------------------------------------------------------------- fns --

    /// A fn declaration; `recv` is a member's receiver.
    pub fn fn_decl(&mut self, f: &FnDecl, recv: Option<&str>, indent: usize) -> String {
        let mut out = self.fn_decl_mode(f, recv, indent, false);
        if self.s.locs.contains(&f.id) && f.kind != FnKind::Intrinsic {
            out.push_str(&self.fn_decl_mode(f, recv, indent, true));
        }
        out
    }

    fn fn_decl_mode(&mut self, f: &FnDecl, recv: Option<&str>, indent: usize, loc: bool) -> String {
        if f.kind == FnKind::Intrinsic {
            return String::new();
        }
        let pad = "    ".repeat(indent);
        self.f = super::body::FnState::default();
        // [rs-fn-field] A fn whose result holds its callbacks takes them owned.
        let stores = self.stores_callbacks(f);
        let core_platform = f.kind == FnKind::Platform && f.id.module.0.first().is_some_and(|m| m == "core") && self.s.program.files.iter().any(|x| x.module == f.id.module && x.is_std);
        let bound = if f.kind == FnKind::Platform && !core_platform { "Send + 'static" } else { "Clone" };
        let erased_fn = self.s.ast_fn(&f.id).is_some_and(|af| {
            let fi = self.s.program.files.iter().position(|x| x.module == f.id.module).unwrap_or(0);
            self.s.erased.fns.contains(&(fi, af.name.span.start))
        });
        let mut tps = if erased_fn { Vec::new() } else { self.generics(&f.type_params, bound) };
        self.f.generics = f.type_params.iter().map(|t| t.name.clone()).collect();
        self.f.plain_vars = self.s.plain_proj_vars(&f.type_params, &f.params, &f.borrows);
        let anchor = if loc { self.s.lend_param(f) } else { None };
        let lt = if loc { None } else { self.ret_lifetime(&f.params, &f.ret, &f.borrows, recv.is_some()) };
        if lt.is_some() {
            tps.insert(0, "'a".to_string());
        }
        let mut ps: Vec<String> = recv.map(|r| vec![r.to_string()]).unwrap_or_default();
        let is_main = f.name == "main" && recv.is_none() && f.id.sub == 0;
        let mutated = super::body::assigned_roots(f.body.as_ref());
        let mut params = self.s.unalias_params(&f.params);
        // [rs-fn-lend] A callback handed a lent parameter's type, answering a
        // type variable, may answer a borrow of it: the two share a lifetime.
        if !loc {
            for p in params.iter().skip(f.effect_params) {
                if p.mode != PassMode::Lent || is_copy_ty(&p.ty) || matches!(p.ty.strip_quals(), Ty::Fn { .. }) {
                    continue;
                }
                let key = super::body::strip_all_pub(&p.ty);
                let tied = params.iter().any(|q| match q.ty.strip_quals() {
                    Ty::Fn { params: fps, ret, .. } => {
                        fps.iter().any(|x| super::body::strip_all_pub(x) == key) && !fps.iter().any(|x| super::body::strip_all_pub(x) == super::body::strip_all_pub(ret)) && (matches!(ret.strip_quals(), Ty::Var(_)) || matches!(ret.strip_quals(), Ty::Named { name, args } if args.is_empty() && self.f.generics.contains(name)))
                    }
                    _ => false,
                });
                if tied && !self.f.tie.iter().any(|(t, _)| *t == key) {
                    let l = format!("'{}", rs_ident(&p.local.0));
                    self.f.tie.push((key, l.clone()));
                    tps.insert(0, l);
                }
            }
        }
        if let Some(a) = anchor {
            // [rs-loc] The locator reads its anchor.
            params[a].mode = PassMode::Lent;
            let opt = f.ret.strip_quals().has_none_arm();
            self.f.loc = Some((params[a].local.0.clone(), opt));
        }
        let covered = if loc { std::collections::HashMap::new() } else { self.covered(f) };
        let mut anchor_done = false;
        for (i, p) in params.iter().enumerate() {
            let n = rs_ident(&p.local.0);
            // [canbe-entry] [rs-loc] A covered parameter is a position in the
            // anchor it may share.
            if let Some(a) = covered.get(&i) {
                if a == "__anchor" && !anchor_done {
                    anchor_done = true;
                    let elem = self.ty(&p.ty);
                    ps.push(format!("__anchor: &mut Vec<{elem}>"));
                    self.f.kinds.insert("__anchor".to_string(), super::body::Kind::RefMut);
                }
                let c = format!("__c{i}");
                ps.push(format!("{c}: usize"));
                self.f.kinds.insert(p.local.0.clone(), super::body::Kind::Elem);
                self.f.elem.insert(p.local.0.clone(), (a.clone(), Some(c)));
                self.f.tys.insert(p.local.0.clone(), p.ty.clone());
                continue;
            }
            if i < f.effect_params {
                let t = self.ty(&p.ty);
                ps.push(format!("{n}: &{t}"));
                self.f.kinds.insert(p.local.0.clone(), super::body::Kind::Ref);
                continue;
            }
            let implicit = i >= f.params.len() - f.implicit_params;
            let borrowed = lt.as_ref().is_some_and(|(_, b)| b.contains(&p.local.0));
            self.f.lt = if borrowed { lt.as_ref().map(|(l, _)| l.clone()) } else { None };
            // [deduce-field] a view stored into another parameter shares its lifetime.
            if !loc {
                if let Some(k) = f.holds.iter().position(|(a, b)| *a == i || *b == i) {
                    let l = format!("'h{k}");
                    if !tps.contains(&l) {
                        tps.insert(0, l.clone());
                    }
                    self.f.lt = Some(l);
                }
            }
            let pos = if stores { FnPos::Owned } else if implicit || recv.is_some() { FnPos::DynParam } else { FnPos::Param };
            let mut t = self.param_ty(p, pos);
            if self.s.once_param(f, i) && self.s.kept_param(f, i).is_none() {
                let sig = self.fn_ty(&p.ty, FnPos::DynParam);
                t = format!("impl FnOnce{}", sig.trim_start_matches("&mut dyn FnMut"));
            }
            if let Some((boxed, once)) = self.s.kept_param(f, i) {
                let sig = self.fn_ty(&p.ty, FnPos::DynParam);
                let sig = sig.trim_start_matches("&mut dyn FnMut").to_string();
                t = if boxed {
                    format!("std::boxed::Box<dyn {}{sig} + Send + 'static>", if once { "FnOnce" } else { "FnMut" })
                } else {
                    format!("fn{sig}")
                };
            }
            self.f.lt = None;
            let mut kind = self.f.bind_param(&p.local.0, p);
            if pos == FnPos::Owned && matches!(p.ty.strip_quals(), Ty::Fn { .. }) {
                kind = super::body::Kind::Owned;
                self.f.kinds.insert(p.local.0.clone(), kind);
            }
            let _ = &mutated;
            let mut_kw = if kind == super::body::Kind::Owned && !matches!(p.ty.strip_quals(), Ty::Fn { .. }) { "mut " } else { "" };
            ps.push(format!("{mut_kw}{n}: {t}"));
        }
        let body_tie = std::mem::take(&mut self.f.tie);
        let _ = body_tie;
        self.f.lt = lt.as_ref().map(|(l, _)| l.clone());
        let ret = match &self.f.loc {
            Some((_, true)) => " -> Option<usize>".to_string(),
            Some((_, false)) => " -> usize".to_string(),
            None => match &f.throws {
                // [rs-throw-controlflow] The throw is the return.
                Some(m) => {
                    let mt = self.ty(m);
                    let vt = if f.ret.is_none_ty() { "()".to_string() } else { self.ty(&f.ret) };
                    format!(" -> std::ops::ControlFlow<{mt}, {vt}>")
                }
                None => self.ret_ty(&f.ret),
            },
        };
        self.f.throws = if loc { None } else { f.throws.clone() };
        self.f.lt = None;
        self.f.ret = Some(f.ret.clone());
        self.f.ret_lt = self.s.holds_proj(&f.ret);
        let mut name = if recv.is_some() { rs_ident(&f.name) } else { self.fn_rust_name(&f.id) };
        if loc {
            name.push_str("__loc");
        }
        let g = if tps.is_empty() { String::new() } else { format!("<{}>", tps.join(", ")) };
        let vis = if recv.is_some() { "" } else { "pub " };
        let head = format!("{pad}{vis}fn {name}{g}({}){ret}", ps.join(", "));
        if f.kind == FnKind::Platform {
            let body = if loc { self.platform_loc_body(f, indent + 1) } else { self.platform_fn_body(f, indent + 1) };
            return format!("\n{head} {{\n{body}{pad}}}\n");
        }
        let Some(b) = &f.body else { return String::new() };
        let _ = is_main;
        let mut body = self.fn_body(b, indent + 1);
        if is_main && indent == 0 && !loc {
            body = format!("{}{body}", self.protocol_prelude());
        }
        if self.f.throws.is_some() && f.ret.is_none_ty() {
            body.push_str(&format!("{pad}    return std::ops::ControlFlow::Continue(());\n"));
        }
        format!("\n{head} {{\n{body}{pad}}}\n")
    }

    pub fn stores_callbacks_only(&self, f: &FnDecl) -> bool {
        self.stores_callbacks(f)
    }

    pub fn stores_callbacks_pub(&self, f: &FnDecl) -> bool {
        self.stores_callbacks(f) || f.params.iter().any(|p| p.mode == PassMode::Moved && matches!(self.s.unalias(&p.ty).strip_quals(), Ty::Fn { .. }))
    }

    /// [canbe-entry] The covered parameters of a fn and the anchor text each
    /// is a position in: the path a `canbe in` names, or `__anchor`.
    pub fn covered(&self, f: &FnDecl) -> std::collections::HashMap<usize, String> {
        let mut out = std::collections::HashMap::new();
        for m in &f.may_alias {
            match m {
                salvo_ir::MayAlias::Params(a, d) => {
                    out.insert(*a, "__anchor".to_string());
                    out.insert(*d, "__anchor".to_string());
                }
                salvo_ir::MayAlias::In { param, root, path } => {
                    let mut t = rs_ident(&f.params[*root].local.0);
                    for s in path {
                        match s {
                            salvo_ir::AnchorStep::Field(x) => {
                                t.push('.');
                                t.push_str(&rs_ident(x));
                            }
                            salvo_ir::AnchorStep::Tuple(i) => t.push_str(&format!(".{i}")),
                        }
                    }
                    out.insert(*param, t);
                }
            }
        }
        out
    }

    /// [rs-fn-field] Whether the fn hands back a struct holding a fn field
    /// (its callbacks are kept past the call).
    fn stores_callbacks(&self, f: &FnDecl) -> bool {
        if !f.params.iter().any(|p| matches!(p.ty.strip_quals(), Ty::Fn { .. })) {
            return false;
        }
        match f.ret.strip_quals() {
            Ty::Named { name, .. } => self.s.struct_decl(name).is_some_and(|s| s.fields.iter().any(|x| matches!(x.ty.strip_quals(), Ty::Fn { .. }))),
            _ => false,
        }
    }

    /// [platform-fn] A platform fn's wrapper: the host's fn, checked.
    fn platform_fn_body(&mut self, f: &FnDecl, indent: usize) -> String {
        let pad = "    ".repeat(indent);
        let module = f.id.module.clone();
        self.s.platform_hosts.insert(module.clone());
        // A lent callback is handed on as `&mut &mut dyn FnMut`, which fits
        // a host's `&mut impl FnMut` and coerces to its `&mut dyn FnMut`.
        // (rebound `mut` first: the parameter itself is not).
        let mut rebind = String::new();
        let args: Vec<String> = f
            .params
            .iter()
            .enumerate()
            .skip(f.effect_params)
            .map(|(i, p)| {
                let n = rs_ident(&p.local.0);
                let lent_fn = matches!(self.s.unalias(&p.ty).strip_quals(), Ty::Fn { .. }) && p.mode != PassMode::Moved && self.s.kept_param(f, i).is_none() && !self.s.once_param(f, i);
                if lent_fn {
                    rebind.push_str(&format!("{pad}let mut {n} = {n};\n"));
                    format!("&mut {n}")
                } else {
                    n
                }
            })
            .collect();
        let call = format!("crate::{}::{}({})", host_mod_name(&module), rs_ident(&f.name), args.join(", "));
        match f.result_check.clone().filter(|_| !self.s.abi) {
            Some(plan) => {
                let what = format!("`platform fn {}`'s result", f.name);
                let checks = self.boundary_check(&plan, "__c", &what, indent + 1);
                format!("{rebind}{pad}let __r = {call};\n{pad}{{\n{pad}    let __c = &__r;\n{checks}{pad}}}\n{pad}__r\n")
            }
            None => format!("{rebind}{pad}{call}\n"),
        }
    }

    /// [rs-loc] A platform fn's locator: the position of the borrow the
    /// host answers, in the anchor.
    fn platform_loc_body(&mut self, f: &FnDecl, indent: usize) -> String {
        let pad = "    ".repeat(indent);
        let module = f.id.module.clone();
        let args: Vec<String> = f.params.iter().skip(f.effect_params).map(|p| rs_ident(&p.local.0)).collect();
        let anchor = self.f.loc.as_ref().map(|(a, _)| rs_ident(a)).unwrap_or_default();
        let list = matches!(module.0.as_slice(), [a, b] if a == "core" && (b == "list" || b == "deque"));
        if list && f.name == "get" && args.len() == 2 {
            return format!("{pad}if {i} >= 0 && ({i} as usize) < {anchor}.len() {{ Some({i} as usize) }} else {{ None }}\n", i = args[1]);
        }
        if list && f.name == "get_at" && args.len() == 2 {
            return format!("{pad}{} as usize\n", args[1]);
        }
        let call = format!("crate::{}::{}({})", host_mod_name(&module), rs_ident(&f.name), args.join(", "));
        if f.ret.strip_quals().has_none_arm() {
            format!("{pad}{call}.map(|__x| {anchor}.iter().position(|__e| std::ptr::eq(__e, __x)).expect(\"salvo: a borrow outside its container\"))\n")
        } else {
            format!("{pad}{{ let __x = {call}; {anchor}.iter().position(|__e| std::ptr::eq(__e, __x)).expect(\"salvo: a borrow outside its container\") }}\n")
        }
    }

    /// [platform-check] Checks on the value behind the reference `v`.
    pub fn boundary_check(&mut self, plan: &salvo_core::abi::BoundaryCheck, v: &str, what: &str, indent: usize) -> String {
        use salvo_core::abi::BoundaryCheck as C;
        let pad = "    ".repeat(indent);
        let what_fmt = what.replace('{', "{{").replace('}', "}}");
        let d = indent;
        match plan {
            C::OneOf(lits) => {
                let pats: Vec<String> = lits.iter().map(rs_type_lit).collect();
                let subject = if lits.iter().any(|l| matches!(l, salvo_syntax::ast::TypeLit::Str(_))) { format!("{v}.as_str()") } else { format!("*{v}") };
                let listed = pats.join(", ").replace('{', "{{").replace('}', "}}").replace('"', "\\\"");
                format!("{pad}if !matches!({subject}, {}) {{\n{pad}    panic!(\"salvo: {what_fmt} was {{:?}}, which is not one of {listed} [platform-check]\", {v});\n{pad}}}\n", pats.join(" | "))
            }
            C::Qualifies { quals, ty, inner } => {
                let arg = if is_copy_ty(ty) { format!("*{v}") } else { v.to_string() };
                let mut out = String::new();
                for (q, m) in quals {
                    let decl = self.s.symbols.qualifiers.get(q.as_str()).and_then(|ds| ds.iter().copied().find(|d| self.s.symbols.qualifier_module(d) == Some(m)));
                    let Some(decl) = decl else {
                        self.error(format!("internal: qualifier `{q}` of module `{m}` not found [platform-check]"));
                        continue;
                    };
                    let base = match salvo_core::refine::of_base(&decl.of, &decl.generics) {
                        Some(subject) => format!("{}__{}", decl.name.name.replace('.', ""), subject.replace('.', "")),
                        None => decl.name.name.replace('.', ""),
                    };
                    let prefix = self.s.prefix(m);
                    out.push_str(&format!("{pad}if !{prefix}{base}_qualifies({arg}) {{\n{pad}    panic!(\"salvo: {what_fmt} was {{:?}}, which is not `{q}` [platform-check]\", {v});\n{pad}}}\n"));
                }
                if let Some(inner) = inner {
                    out.push_str(&self.boundary_check(inner, v, what, indent));
                }
                out
            }
            C::Elems(inner) => {
                let e = format!("__e{d}");
                let body = self.boundary_check(inner, &e, what, indent + 1);
                format!("{pad}for {e} in {v}.iter() {{\n{body}{pad}}}\n")
            }
            C::Entries(k, val) => {
                let (kn, vn) = (format!("__k{d}"), format!("__v{d}"));
                let mut body = String::new();
                if let Some(k) = k {
                    body.push_str(&self.boundary_check(k, &kn, what, indent + 1));
                }
                if let Some(val) = val {
                    body.push_str(&self.boundary_check(val, &vn, what, indent + 1));
                }
                format!("{pad}for ({kn}, {vn}) in {v}.iter() {{\n{body}{pad}}}\n")
            }
            C::Nullable(inner) => {
                let n = format!("__n{d}");
                let body = self.boundary_check(inner, &n, what, indent + 1);
                format!("{pad}if let Some({n}) = {v} {{\n{body}{pad}}}\n")
            }
            C::Union { arity, arms } => {
                self.s.union_sizes.insert(*arity);
                let mut out = String::new();
                for (i, _, inner) in arms {
                    let a = format!("__a{d}_{i}");
                    let body = self.boundary_check(inner, &a, what, indent + 1);
                    out.push_str(&format!("{pad}if let crate::unions::Union{arity}::U{}({a}) = {v} {{\n{body}{pad}}}\n", i + 1));
                }
                out
            }
            C::Struct { fields, .. } => {
                let mut out = String::new();
                for (field, inner) in fields {
                    let f = format!("__f{d}_{field}");
                    out.push_str(&format!("{pad}let {f} = &{v}.{};\n", rs_ident(field)));
                    out.push_str(&self.boundary_check(inner, &f, what, indent));
                }
                out
            }
            C::Tuple { elems, .. } => {
                let mut out = String::new();
                for (i, inner) in elems {
                    let t = format!("__t{d}_{i}");
                    out.push_str(&format!("{pad}let {t} = &{v}.{i};\n"));
                    out.push_str(&self.boundary_check(inner, &t, what, indent));
                }
                out
            }
        }
    }

    /// [mod-use] A module-level `use`: a lazily built `'static`, reached
    /// through an accessor fn. A handler instance is held shared (its faces'
    /// handles share it); a face is its handle.
    fn static_decl(&mut self, st: &salvo_ir::StaticDecl) {
        self.f = super::body::FnState::default();
        let name = super::body::rs_local(&st.local.0);
        let Some(Stmt::Let { value, .. }) = st.stmts.first() else {
            self.error("a module-level `use` with no binding");
            return;
        };
        let is_instance = matches!(st.ty.strip_quals(), Ty::Named { name, .. } if self.s.symbols.handlers.contains_key(name.as_str()));
        let (ty, init) = if is_instance {
            let h = self.ty(&st.ty);
            let stateful = match st.ty.strip_quals() {
                Ty::Named { name, .. } => match self.s.impl_decl(name) {
                    Some(h) if h.platform => !h.threadsafe,
                    Some(h) => h.stateful,
                    None => true,
                },
                _ => true,
            };
            let v = self.value(value, 2);
            if stateful {
                (format!("std::sync::Arc<std::sync::Mutex<{h}>>"), format!("std::sync::Arc::new(std::sync::Mutex::new({v}))"))
            } else {
                (format!("std::sync::Arc<{h}>"), format!("std::sync::Arc::new({v})"))
            }
        } else {
            let face = match st.ty.strip_quals() {
                Ty::Named { name, .. } => self.type_path(name),
                _ => self.ty(&st.ty),
            };
            let t = self.ty(&st.ty);
            let inst = match &value.kind {
                ExprKind::Handle { instance } => instance.as_ref(),
                ExprKind::Read { .. } => value,
                _ => {
                    // The face itself (an addr's instance).
                    let v = self.value(value, 2);
                    self.out.push_str(&format!(
                        "\npub fn {name}() -> &'static {t} {{\n    static CELL: std::sync::OnceLock<{t}> = std::sync::OnceLock::new();\n    CELL.get_or_init(|| {v})\n}}\n"
                    ));
                    return;
                }
            };
            let (src, _) = match &inst.kind {
                ExprKind::Read { place, .. } => (format!("{}{}()", self.s.prefix(&self.module.path), super::body::rs_local(&place.root.0)), ()),
                _ => (self.value(inst, 2), ()),
            };
            let inst_ty = match &inst.kind {
                ExprKind::Read { place, .. } => self.module.decls.iter().find_map(|d| match d {
                    Decl::Static(s) if s.local == place.root => Some(s.ty.clone()),
                    _ => None,
                }),
                _ => None,
            }
            .unwrap_or_else(|| inst.ty.clone());
            let stateful = match inst_ty.strip_quals() {
                Ty::Named { name, .. } => match self.s.impl_decl(name) {
                    Some(h) if h.platform => !h.threadsafe,
                    Some(h) => h.stateful,
                    None => true,
                },
                _ => true,
            };
            let mk = if stateful { "share_locked" } else { "share_shared" };
            (t, format!("{face}::{mk}({src}.clone())"))
        };
        self.out.push_str(&format!(
            "\npub fn {name}() -> &'static {ty} {{\n    static CELL: std::sync::OnceLock<{ty}> = std::sync::OnceLock::new();\n    CELL.get_or_init(|| {init})\n}}\n"
        ));
    }
}

/// A literal type's value as a Rust pattern.
fn rs_type_lit(l: &salvo_syntax::ast::TypeLit) -> String {
    use salvo_syntax::ast::TypeLit;
    match l {
        TypeLit::Str(s) => format!("{s:?}"),
        TypeLit::Int(i) => format!("{i}i32"),
        TypeLit::Long(i) => format!("{i}i64"),
        TypeLit::Bool(b) => b.to_string(),
    }
}

/// A type with type variables substituted.
/// Whether a type is, or holds, mutable data.
pub(crate) fn ty_carries_mut(ty: &Ty) -> bool {
    if ty.quals().iter().any(|q| q.name == "Mut") {
        return true;
    }
    match ty.strip_quals() {
        Ty::Named { args, .. } | Ty::Union(args) | Ty::Tuple(args) => args.iter().any(ty_carries_mut),
        Ty::Array(elem) => ty_carries_mut(elem),
        _ => false,
    }
}

pub(crate) fn subst_ty(t: &Ty, s: &std::collections::HashMap<String, Ty>) -> Ty {
    if s.is_empty() {
        return t.clone();
    }
    match t {
        Ty::Var(v) => s.get(v).cloned().unwrap_or_else(|| t.clone()),
        Ty::Named { name, args } if args.is_empty() && s.contains_key(name) => s[name].clone(),
        Ty::Named { name, args } => Ty::Named { name: name.clone(), args: args.iter().map(|a| subst_ty(a, s)).collect() },
        Ty::Qualified { quals, base } => subst_ty(base, s).qualify(quals.clone()),
        Ty::Union(a) => Ty::Union(a.iter().map(|x| subst_ty(x, s)).collect()),
        Ty::Tuple(a) => Ty::Tuple(a.iter().map(|x| subst_ty(x, s)).collect()),
        Ty::Array(e) => Ty::Array(Box::new(subst_ty(e, s))),
        Ty::Fn { params, ret, contract, effects } => Ty::Fn {
            params: params.iter().map(|x| subst_ty(x, s)).collect(),
            ret: Box::new(subst_ty(ret, s)),
            contract: contract.clone(),
            effects: effects.iter().map(|x| subst_ty(x, s)).collect(),
        },
        other => other.clone(),
    }
}

pub(crate) fn _unused(_: &HashSet<String>) {}

/// Whether `code` mentions `word` as a whole identifier.
fn mentions_word(code: &str, word: &str) -> bool {
    code.split(|c: char| !(c.is_alphanumeric() || c == '_')).any(|w| w == word)
}
