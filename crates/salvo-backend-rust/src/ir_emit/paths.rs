//! [rs-path] A `ref(c)` handle on Rust is a **storage path** from its
//! container: field steps, tuple steps, list indices and map slots, computed
//! once when the handle is minted and walked at every use (GROUP_BORROWING.md
//! Part 9). The path's type belongs to the (container type, element type)
//! pair and is read off the type definitions, so handles from any accessor of
//! one container compare and split:
//! - one way from the container to the element: the tuple of its dynamic
//!   positions (`usize` for one, `()` for none) — a list handle is a `usize`,
//!   a `Grid`'s is the index into its `cells`;
//! - several ways: a generated enum, one variant per way, each holding that
//!   way's positions, walked by a `match`.
//!
//! Kotlin needs none of it: a handle there is the element itself.

use std::collections::HashMap;

use salvo_core::types::Ty;
use salvo_core::ModulePath;
use salvo_ir::build::refs::{ref_container, storage};

use super::body::Kind;
use super::{rs_ident, ModuleEmitter, Shared};

/// One step of a path *shape*: what the type definitions say, positions not
/// filled in.
#[derive(Clone, Debug, PartialEq)]
pub enum PStep {
    Field(String),
    Tuple(usize),
    /// A list, deque or array position.
    Index,
    /// A map entry's slot.
    Slot,
}

/// The path type of one (container type, element type) pair.
#[derive(Clone, Debug)]
pub struct PathTy {
    pub shapes: Vec<Vec<PStep>>,
    /// The generated enum, when there is more than one shape.
    pub enum_name: Option<String>,
    pub home: Option<ModulePath>,
}

fn dyn_count(shape: &[PStep]) -> usize {
    shape.iter().filter(|s| matches!(s, PStep::Index | PStep::Slot)).count()
}

/// A step of a path whose positions are known as code.
#[derive(Clone, Debug)]
pub enum SStep {
    Field(String),
    Tuple(usize),
    /// A position (`PStep::Index` or `PStep::Slot`), as code.
    Dyn(PStep, String),
    /// A value of an enum path type: its shape is only known at run time.
    Opaque(PathTy, String),
}

/// A handle: the container's root place, and the path from it.
#[derive(Clone, Debug)]
pub struct Handle {
    pub root: String,
    pub steps: Vec<SStep>,
}

/// How a handle local is bound.
#[derive(Clone, Debug)]
pub enum HandleBinding {
    Total(Handle),
    /// An optional handle not yet tested: `var: Option<P>`, a position of
    /// `prefix` once present.
    Optional { var: String, prefix: Handle, pt: PathTy },
}

/// The key a (container, element) pair is cached under.
pub fn path_key(c: &Ty, e: &Ty) -> (String, String) {
    (format!("{}", storage(c)), format!("{}", storage(e)))
}

impl<'p> Shared<'p> {
    /// Every way from `c` to `e` through the type definitions.
    fn shapes(&self, c: &Ty, e: &Ty, prefix: &mut Vec<PStep>, stack: &mut Vec<String>, out: &mut Vec<Vec<PStep>>) -> Result<(), String> {
        let c = storage(&self.unalias(c));
        let e = storage(e);
        if c == e {
            out.push(prefix.clone());
            // The element holding more of itself would make the paths
            // unbounded (a recursive type).
            let key = format!("{c}");
            if stack.contains(&key) {
                return Err(format!("`{c}` contains itself, so its handles have no finite path yet [rs-path]"));
            }
            stack.push(key);
            let mut deeper = Vec::new();
            let r = self.shapes_inner(&c, &e, prefix, stack, &mut deeper);
            stack.pop();
            r?;
            if !deeper.is_empty() {
                return Err(format!("`{c}` contains itself, so its handles have no finite path yet [rs-path]"));
            }
            return Ok(());
        }
        let key = format!("{c}");
        if stack.contains(&key) {
            return Ok(());
        }
        stack.push(key);
        let r = self.shapes_inner(&c, &e, prefix, stack, out);
        stack.pop();
        r
    }

    fn shapes_inner(&self, c: &Ty, e: &Ty, prefix: &mut Vec<PStep>, stack: &mut Vec<String>, out: &mut Vec<Vec<PStep>>) -> Result<(), String> {
        let via = |step: PStep, t: &Ty, prefix: &mut Vec<PStep>, stack: &mut Vec<String>, out: &mut Vec<Vec<PStep>>| -> Result<(), String> {
            prefix.push(step);
            let r = self.shapes(t, e, prefix, stack, out);
            prefix.pop();
            r
        };
        match c {
            Ty::Named { name, args } if (name == "List" || name == "Deque") && args.len() == 1 => via(PStep::Index, &args[0], prefix, stack, out),
            Ty::Array(x) => via(PStep::Index, x, prefix, stack, out),
            Ty::Named { name, args } if name == "Map" && args.iter().filter(|a| !matches!(a, Ty::FnName(_))).count() == 2 => {
                let v = args.iter().filter(|a| !matches!(a, Ty::FnName(_))).nth(1).unwrap().clone();
                via(PStep::Slot, &v, prefix, stack, out)
            }
            Ty::Tuple(es) => {
                for (i, t) in es.iter().enumerate() {
                    via(PStep::Tuple(i), t, prefix, stack, out)?;
                }
                Ok(())
            }
            Ty::Named { name, args } => {
                let Some(s) = self.struct_decl(name) else { return Ok(()) };
                let subst: HashMap<String, Ty> = s.type_params.iter().map(|t| t.name.clone()).zip(args.iter().cloned()).collect();
                for f in &s.fields {
                    let ft = super::decls::subst_ty(&f.ty, &subst);
                    via(PStep::Field(f.name.clone()), &ft, prefix, stack, out)?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// The path type of handles of element type `e` into `c`.
    pub fn path_ty(&mut self, c: &Ty, e: &Ty) -> Result<PathTy, String> {
        let key = path_key(c, e);
        if let Some(p) = self.paths.get(&key) {
            return p.clone();
        }
        let r = self.compute_path_ty(c, e, false);
        self.paths.insert(key, r.clone());
        r
    }

    fn compute_path_ty(&self, c: &Ty, e: &Ty, may_enum: bool) -> Result<PathTy, String> {
        let cs = storage(c);
        if matches!(cs, Ty::Var(_)) {
            return Err(format!("a handle into the generic container `{cs}` has no path on the Rust backend yet [rs-path]"));
        }
        let mut out = Vec::new();
        self.shapes(c, e, &mut Vec::new(), &mut Vec::new(), &mut out)?;
        if out.is_empty() {
            return Err(format!("no storage of `{cs}` holds a `{}` [rs-path]", storage(e)));
        }
        if out.len() == 1 {
            return Ok(PathTy { shapes: out, enum_name: None, home: None });
        }
        if !may_enum {
            return Err(format!("internal: the path type of `{cs}` to `{}` was not declared ahead [rs-path]", storage(e)));
        }
        let san = |t: &Ty| -> String { format!("{t}").chars().map(|ch| if ch.is_alphanumeric() { ch } else { '_' }).collect() };
        let name = format!("__Path_{}__{}", san(&cs), san(&storage(e)));
        // Declared with the container's type, else the element's.
        let home = [&cs, &storage(e)].into_iter().find_map(|t| match t {
            Ty::Named { name, .. } => self.symbols.key_modules.get(name.as_str()).map(|m| (*m).clone()),
            _ => None,
        });
        Ok(PathTy { shapes: out, enum_name: Some(name), home })
    }

    /// [rs-path] Every handle type the program mentions, its path type
    /// computed ahead so an enum is declared before any module names it.
    pub fn compute_paths(&mut self) {
        let mut pairs: Vec<(Ty, Ty)> = Vec::new();
        let seen_ty = |t: &Ty, pairs: &mut Vec<(Ty, Ty)>| {
            collect_handles(t, pairs);
        };
        for m in &self.ir.modules {
            for d in &m.decls {
                let fns: Vec<&salvo_ir::FnDecl> = match d {
                    salvo_ir::Decl::Fn(f) => vec![f],
                    salvo_ir::Decl::Impl(h) => h.members.iter().chain(h.init.iter()).collect(),
                    salvo_ir::Decl::Interface(i) => {
                        for mem in &i.members {
                            seen_ty(&mem.ret, &mut pairs);
                        }
                        Vec::new()
                    }
                    _ => Vec::new(),
                };
                for f in fns {
                    seen_ty(&f.ret, &mut pairs);
                    for p in &f.params {
                        seen_ty(&p.ty, &mut pairs);
                    }
                    if let Some(b) = &f.body {
                        let mut from_stmts: Vec<(Ty, Ty)> = Vec::new();
                        let mut from_exprs: Vec<(Ty, Ty)> = Vec::new();
                        super::body::walk_block(
                            b,
                            &mut |s| match s {
                                salvo_ir::Stmt::Let { ty, .. } | salvo_ir::Stmt::ForEach { ty, .. } | salvo_ir::Stmt::Narrow { ty, .. } => collect_handles(ty, &mut from_stmts),
                                _ => {}
                            },
                            &mut |e| collect_handles(&e.ty, &mut from_exprs),
                        );
                        pairs.extend(from_stmts);
                        pairs.extend(from_exprs);
                    }
                }
            }
        }
        for (c, e) in pairs {
            let key = path_key(&c, &e);
            if self.paths.contains_key(&key) {
                continue;
            }
            let r = self.compute_path_ty(&c, &e, true);
            if let Ok(p) = &r {
                if p.enum_name.is_some() {
                    self.path_enums.push(p.clone());
                }
            }
            self.paths.insert(key, r);
        }
    }
}

/// Every (container, element) pair of the handle types inside `t`.
fn collect_handles(t: &Ty, out: &mut Vec<(Ty, Ty)>) {
    match t {
        Ty::Qualified { quals, base } => {
            if let Some(c) = quals.iter().find(|q| q.name == "ref").and_then(|q| q.args.first()) {
                out.push((c.clone(), storage(base)));
            }
            collect_handles(base, out);
        }
        Ty::Union(arms) | Ty::Tuple(arms) => arms.iter().for_each(|a| collect_handles(a, out)),
        Ty::Named { args, .. } => args.iter().for_each(|a| collect_handles(a, out)),
        Ty::Array(e) => collect_handles(e, out),
        Ty::Fn { params, ret, effects, .. } => {
            params.iter().chain(effects).for_each(|a| collect_handles(a, out));
            collect_handles(ret, out);
        }
        _ => {}
    }
}

impl<'a, 'p> ModuleEmitter<'a, 'p> {
    /// The path type a locator variant of `f` answers: its result's, a
    /// handle's or a reader's (a path in the lent parameter).
    pub fn loc_path_ty(&mut self, f: &salvo_ir::FnDecl, anchor: usize) -> Option<PathTy> {
        if salvo_ir::build::refs::has_ref(&f.ret) {
            return self.handle_path_ty(&f.ret);
        }
        let c = f.params.get(anchor)?.ty.clone();
        match self.s.path_ty(&c, &storage(&f.ret)) {
            Ok(p) => Some(p),
            Err(msg) => {
                self.error(msg);
                None
            }
        }
    }

    /// The path type of a handle type (its `ref` names the container).
    pub fn handle_path_ty(&mut self, handle: &Ty) -> Option<PathTy> {
        let Some(c) = ref_container(handle).cloned() else {
            self.error(format!("internal: the handle type `{handle}` names no container [rs-path]"));
            return None;
        };
        let e = storage(handle);
        match self.s.path_ty(&c, &e) {
            Ok(p) => Some(p),
            Err(msg) => {
                self.error(msg);
                None
            }
        }
    }

    /// A path type's Rust spelling.
    pub fn path_rust(&self, pt: &PathTy) -> String {
        if let Some(n) = &pt.enum_name {
            let prefix = pt.home.as_ref().map(|m| self.s.prefix(m)).unwrap_or_else(|| "crate::".to_string());
            return format!("{prefix}{n}");
        }
        match dyn_count(&pt.shapes[0]) {
            0 => "()".to_string(),
            1 => "usize".to_string(),
            n => format!("({})", vec!["usize"; n].join(", ")),
        }
    }

    /// The steps a value `code` of path type `pt` stands for.
    pub fn steps_of_value(&self, pt: &PathTy, code: &str) -> Vec<SStep> {
        if pt.enum_name.is_some() {
            return vec![SStep::Opaque(pt.clone(), code.to_string())];
        }
        let shape = &pt.shapes[0];
        let n = dyn_count(shape);
        let mut k = 0;
        shape
            .iter()
            .map(|s| match s {
                PStep::Field(f) => SStep::Field(f.clone()),
                PStep::Tuple(i) => SStep::Tuple(*i),
                d => {
                    let c = if n == 1 { code.to_string() } else { format!("{code}.{k}") };
                    k += 1;
                    SStep::Dyn(d.clone(), c)
                }
            })
            .collect()
    }

    /// The variant `v` of an enum path, its positions bound to fresh names:
    /// (pattern, steps).
    fn variant_steps(&mut self, pt: &PathTy, v: usize) -> (String, Vec<SStep>) {
        let name = self.path_rust(pt);
        let mut binders = Vec::new();
        let steps: Vec<SStep> = pt.shapes[v]
            .iter()
            .map(|s| match s {
                PStep::Field(f) => SStep::Field(f.clone()),
                PStep::Tuple(i) => SStep::Tuple(*i),
                d => {
                    let b = self.fresh("p");
                    binders.push(b.clone());
                    SStep::Dyn(d.clone(), b)
                }
            })
            .collect();
        let pat = if binders.is_empty() { format!("{name}::V{v}") } else { format!("{name}::V{v}({})", binders.join(", ")) };
        (pat, steps)
    }

    /// The place a handle names, as Rust text: `root.f[i]…`. An enum path
    /// is walked by a `match`, borrowing `&mut` when `mutable` (a write) and
    /// `&` otherwise.
    pub fn render_path(&mut self, root: &str, steps: &[SStep], mutable: bool) -> String {
        let mut out = root.to_string();
        for (i, s) in steps.iter().enumerate() {
            match s {
                SStep::Field(f) => {
                    out.push('.');
                    out.push_str(&rs_ident(f));
                }
                SStep::Tuple(t) => out.push_str(&format!(".{t}")),
                SStep::Dyn(_, c) => out.push_str(&format!("[{c}]")),
                SStep::Opaque(pt, code) => {
                    let pt = pt.clone();
                    let code = code.clone();
                    let rest = steps[i + 1..].to_vec();
                    let m = if mutable { "&mut " } else { "&" };
                    let mut arms = Vec::new();
                    for v in 0..pt.shapes.len() {
                        let (pat, mut vs) = self.variant_steps(&pt, v);
                        vs.extend(rest.iter().cloned());
                        let place = self.render_path(&out, &vs, mutable);
                        arms.push(format!("{pat} => {m}{place}"));
                    }
                    return format!("(*match {code} {{ {} }})", arms.join(", "));
                }
            }
        }
        out
    }

    /// The value of path type `pt` the steps stand for, as code.
    pub fn path_value(&mut self, steps: &[SStep], pt: &PathTy) -> Option<String> {
        // The value itself, when it already is one of this type.
        if let [SStep::Opaque(inner, code)] = steps {
            if inner.enum_name.is_some() && inner.enum_name == pt.enum_name {
                return Some(code.clone());
            }
        }
        if let Some(i) = steps.iter().position(|s| matches!(s, SStep::Opaque(..))) {
            let SStep::Opaque(inner, code) = &steps[i] else { unreachable!() };
            let (inner, code) = (inner.clone(), code.clone());
            let mut arms = Vec::new();
            for v in 0..inner.shapes.len() {
                let (pat, vs) = self.variant_steps(&inner, v);
                let mut all: Vec<SStep> = steps[..i].to_vec();
                all.extend(vs);
                all.extend(steps[i + 1..].iter().cloned());
                let val = self.path_value(&all, pt)?;
                arms.push(format!("{pat} => {val}"));
            }
            return Some(format!("match {code} {{ {} }}", arms.join(", ")));
        }
        let shape: Vec<PStep> = steps
            .iter()
            .map(|s| match s {
                SStep::Field(f) => PStep::Field(f.clone()),
                SStep::Tuple(t) => PStep::Tuple(*t),
                SStep::Dyn(d, _) => d.clone(),
                SStep::Opaque(..) => unreachable!(),
            })
            .collect();
        let codes: Vec<String> = steps.iter().filter_map(|s| if let SStep::Dyn(_, c) = s { Some(c.clone()) } else { None }).collect();
        let Some(v) = pt.shapes.iter().position(|s| *s == shape) else {
            self.error("a handle whose path is not one its container's type allows [rs-path] [backend-never-wrong]");
            return None;
        };
        if pt.enum_name.is_some() {
            let name = self.path_rust(pt);
            return Some(if codes.is_empty() { format!("{name}::V{v}") } else { format!("{name}::V{v}({})", codes.join(", ")) });
        }
        Some(match codes.len() {
            0 => "()".to_string(),
            1 => codes[0].clone(),
            _ => format!("({})", codes.join(", ")),
        })
    }

    /// A handle's steps with the container's own steps taken off the
    /// front (a `ref(c)` argument is a path in the argument passed as `c`).
    pub fn strip_prefix(&mut self, h: &Handle, container: &Handle) -> Option<Vec<SStep>> {
        if h.root != container.root || h.steps.len() < container.steps.len() {
            return None;
        }
        for (a, b) in h.steps.iter().zip(&container.steps) {
            let same = match (a, b) {
                (SStep::Field(x), SStep::Field(y)) => x == y,
                (SStep::Tuple(x), SStep::Tuple(y)) => x == y,
                (SStep::Dyn(k, x), SStep::Dyn(l, y)) => k == l && x == y,
                _ => false,
            };
            if !same {
                return None;
            }
        }
        Some(h.steps[container.steps.len()..].to_vec())
    }

    /// Two live `&mut` into one container, split where the paths diverge:
    /// an expression of type `(&mut E, &mut E)`. Paths that do not diverge
    /// name one element, which the checker's proof ruled out — a trap.
    pub fn split_pair(&mut self, root: &str, a: &[SStep], b: &[SStep]) -> String {
        if let Some(i) = a.iter().position(|s| matches!(s, SStep::Opaque(..))).filter(|i| *i == 0) {
            let SStep::Opaque(pt, code) = &a[i] else { unreachable!() };
            let (pt, code) = (pt.clone(), code.clone());
            let mut arms = Vec::new();
            for v in 0..pt.shapes.len() {
                let (pat, mut vs) = self.variant_steps(&pt, v);
                vs.extend(a[1..].iter().cloned());
                let inner = self.split_pair(root, &vs, b);
                arms.push(format!("{pat} => {inner}"));
            }
            return format!("match {code} {{ {} }}", arms.join(", "));
        }
        if matches!(b.first(), Some(SStep::Opaque(..))) {
            let SStep::Opaque(pt, code) = &b[0] else { unreachable!() };
            let (pt, code) = (pt.clone(), code.clone());
            let mut arms = Vec::new();
            for v in 0..pt.shapes.len() {
                let (pat, mut vs) = self.variant_steps(&pt, v);
                vs.extend(b[1..].iter().cloned());
                let inner = self.split_pair(root, a, &vs);
                arms.push(format!("{pat} => {inner}"));
            }
            return format!("match {code} {{ {} }}", arms.join(", "));
        }
        match (a.first(), b.first()) {
            (None, None) => "panic!(\"salvo: two handles to one element\")".to_string(),
            (None, _) | (_, None) => {
                self.error("two live handles where one contains the other [rs-path] [backend-never-wrong]");
                "panic!()".to_string()
            }
            (Some(x), Some(y)) => {
                let same_static = match (x, y) {
                    (SStep::Field(p), SStep::Field(q)) => Some(p == q),
                    (SStep::Tuple(p), SStep::Tuple(q)) => Some(p == q),
                    _ => None,
                };
                match same_static {
                    Some(true) => {
                        let next = self.render_path(root, &a[..1], true);
                        self.split_pair(&next, &a[1..], &b[1..])
                    }
                    Some(false) => {
                        // Different fields: disjoint by construction.
                        let pa = self.render_path(root, a, true);
                        let pb = self.render_path(root, b, true);
                        format!("(&mut {pa}, &mut {pb})")
                    }
                    None => {
                        let (SStep::Dyn(k, cx), SStep::Dyn(_, cy)) = (x, y) else {
                            self.error("two handles whose paths differ in kind [rs-path] [backend-never-wrong]");
                            return "panic!()".to_string();
                        };
                        let (k, cx, cy) = (k.clone(), cx.clone(), cy.clone());
                        let (u, w) = (self.fresh("u"), self.fresh("w"));
                        let split = match k {
                            PStep::Slot => format!("crate::platform_core_map::pair_mut(&mut {root}, {cx}, {cy})"),
                            _ => {
                                self.s.needs_seq = true;
                                format!("crate::seq::salvo_pair_mut(&mut {root}[..], {cx}, {cy})")
                            }
                        };
                        let pa = self.render_path(&format!("(*{u})"), &a[1..], true);
                        let pb = self.render_path(&format!("(*{w})"), &b[1..], true);
                        let next = self.render_path(root, &a[..1], true);
                        let same = self.split_pair(&next, &a[1..], &b[1..]);
                        format!("if {cx} != {cy} {{ let ({u}, {w}) = {split}.expect(\"salvo: value is absent\"); (&mut {pa}, &mut {pb}) }} else {{ {same} }}")
                    }
                }
            }
        }
    }

    /// A handle local's binding, if `name` is one.
    pub fn handle_of(&self, name: &str) -> Option<HandleBinding> {
        self.f.ref_handles.get(name).cloned()
    }

    /// Binds `local` as a handle.
    pub fn bind_handle(&mut self, local: &str, b: HandleBinding) {
        self.f.kinds.insert(local.to_string(), Kind::Elem);
        self.f.ref_handles.insert(local.to_string(), b);
    }

    /// The path type declarations this module carries.
    pub fn path_enums(&self) -> String {
        let mut out = String::new();
        for pt in &self.s.path_enums {
            if pt.home.as_ref() != Some(&self.module.path) && !(pt.home.is_none() && self.s.prefix(&self.module.path) == "crate::") {
                continue;
            }
            let n = pt.enum_name.as_ref().unwrap();
            let variants: Vec<String> = pt
                .shapes
                .iter()
                .enumerate()
                .map(|(v, s)| {
                    let k = dyn_count(s);
                    if k == 0 { format!("V{v}") } else { format!("V{v}({})", vec!["usize"; k].join(", ")) }
                })
                .collect();
            out.push_str(&format!("\n/// [rs-path] The paths to a handle's element.\n#[derive(Clone, Copy, PartialEq, Eq, Debug)]\npub enum {n} {{ {} }}\n", variants.join(", ")));
        }
        out
    }
}

/// A mint seen at its use: what it is a handle into, and the code that
/// answers its path.
#[derive(Clone, Debug)]
pub struct Mint {
    /// Statements to run first (a nested mint's path, bound).
    pub pre: Vec<String>,
    /// The handle of the container the callee is lent.
    pub prefix: Handle,
    /// The callee's path type, relative to `prefix`.
    pub pt: PathTy,
    /// The path, or an optional one.
    pub code: String,
    pub opt: bool,
}

impl<'a, 'p> ModuleEmitter<'a, 'p> {
    /// The handle a container argument is: a place (through a handle local
    /// or not), or a nested mint, its path bound in `pre`.
    pub fn anchor_handle(&mut self, a: &salvo_ir::Expr, indent: usize, pre: &mut Vec<String>) -> Option<Handle> {
        use salvo_ir::{ExprKind, Place, Step};
        match &a.kind {
            ExprKind::DropMut { value } => self.anchor_handle(value, indent, pre),
            ExprKind::Read { place, .. } => {
                let mut extra: Vec<SStep> = Vec::new();
                for st in &place.steps {
                    extra.push(match st {
                        Step::Field(f) => SStep::Field(f.clone()),
                        Step::Tuple(i) => SStep::Tuple(*i),
                        Step::Index(e) => {
                            let i = self.value(e, indent);
                            SStep::Dyn(PStep::Index, format!("(({i}) as usize)"))
                        }
                    });
                }
                match self.handle_of(&place.root.0) {
                    Some(HandleBinding::Total(h)) => {
                        let mut steps = h.steps.clone();
                        steps.extend(extra);
                        Some(Handle { root: h.root, steps })
                    }
                    Some(HandleBinding::Optional { .. }) => None,
                    None => {
                        let (root, _) = self.place_text(&Place { root: place.root.clone(), steps: Vec::new() }, indent);
                        Some(Handle { root, steps: extra })
                    }
                }
            }
            _ => {
                let m = self.mint(a, indent)?;
                pre.extend(m.pre.iter().cloned());
                let q = self.fresh("q");
                let code = if m.opt { format!("{}.expect(\"salvo: value is absent\")", m.code) } else { m.code.clone() };
                pre.push(format!("let {q} = {code};"));
                let mut steps = m.prefix.steps.clone();
                steps.extend(self.steps_of_value(&m.pt, &q));
                Some(Handle { root: m.prefix.root, steps })
            }
        }
    }

    /// The mint `e` is, when it is one: a call to a fn answering a handle
    /// (a path fn), a reader used as a handle (its locator), a lending
    /// member or callback, or `x!` of one.
    pub fn mint(&mut self, e: &salvo_ir::Expr, indent: usize) -> Option<Mint> {
        use salvo_ir::{ExprKind, FnRef, PassMode, Stmt};
        match &e.kind {
            // Inside a locator, a forwarded call answers its own locator's
            // path whatever its type says (a reader forwarding a reader).
            ExprKind::Call { target: FnRef::Decl(id), args, .. }
                if self.lends_loc(e) || (self.f.loc.is_some() && self.s.locs.contains(id) && self.s.ast_fn(id).is_none_or(|af| !af.intrinsic)) =>
            {
                let f = self.s.fn_decl(id)?;
                let k = match self.s.mut_lend(e) {
                    Some((_, k)) => k,
                    None => self.s.lend_param(f)?,
                };
                let mut pre = Vec::new();
                let prefix = self.anchor_handle(args.get(k)?, indent, &mut pre)?;
                let pt = if salvo_ir::build::refs::has_ref(&e.ty) {
                    self.handle_path_ty(&e.ty)?
                } else {
                    match self.s.path_ty(&args[k].ty, &storage(&e.ty)) {
                        Ok(p) => p,
                        Err(msg) => {
                            self.error(msg);
                            return None;
                        }
                    }
                };
                let code = self.loc_call(id, f, args, indent);
                Some(Mint { pre, prefix, pt, code, opt: e.ty.strip_quals().has_none_arm() })
            }
            ExprKind::Call { target: FnRef::Decl(id), args, .. } => {
                // An intrinsic with a locator form (a list's `get`).
                let af = self.s.ast_fn(id).filter(|af| af.intrinsic)?;
                if !salvo_ir::build::refs::has_ref(&e.ty) && self.f.loc.is_none() {
                    return None;
                }
                let name = af.name.name.clone();
                let recv = af.params.first().and_then(|p| salvo_backend::emit_util::type_base_name(&p.ty));
                let raw: Vec<String> = args.iter().map(|x| self.raw(x, indent)).collect();
                let code = crate::intrinsics::fn_call(&name, recv, &raw, crate::intrinsics::Spread::None, true)?;
                let mut pre = Vec::new();
                let prefix = self.anchor_handle(args.first()?, indent, &mut pre)?;
                let pt = match self.s.path_ty(&args[0].ty, &storage(&e.ty)) {
                    Ok(p) => p,
                    Err(msg) => {
                        self.error(msg);
                        return None;
                    }
                };
                Some(Mint { pre, prefix, pt, code, opt: e.ty.strip_quals().has_none_arm() })
            }
            ExprKind::Call { target: target @ FnRef::Local(l), type_args, args } => {
                let ty = self.s.unalias(&self.f.tys.get(&l.0).cloned().unwrap_or(Ty::Unknown));
                if !self.s.fn_ty_lends_mut(&ty) {
                    return None;
                }
                let mut pre = Vec::new();
                let prefix = args.iter().find(|a| !super::is_copy_ty(&a.ty)).and_then(|a| self.anchor_handle(a, indent, &mut pre))?;
                let pt = self.handle_path_ty(&e.ty)?;
                let saved = std::mem::replace(&mut self.f.want_path, true);
                let code = self.call(e, target, type_args, args, indent);
                self.f.want_path = saved;
                Some(Mint { pre, prefix, pt, code, opt: e.ty.strip_quals().has_none_arm() })
            }
            ExprKind::MemberCall { instance, member, args, .. } => {
                let m = self.member_decl(member)?;
                let iface = self.s.interface_by_id(&member.interface)?;
                if !self.s.member_lends_mut(iface, m) {
                    return None;
                }
                let k = self.s.member_lend_param(m)?;
                let mut pre = Vec::new();
                let prefix = self.anchor_handle(args.get(k)?, indent, &mut pre)?;
                let pt = self.handle_path_ty(&e.ty)?;
                let inst = self.raw(instance, indent);
                let mut ps = self.s.unalias_params(&m.params);
                ps[k].mode = PassMode::Lent;
                let a = self.args(&ps, args, super::decls::FnPos::DynParam, indent);
                let code = format!("{inst}.{}__loc({})", rs_ident(&m.emitted_name), a.join(", "));
                Some(Mint { pre, prefix, pt, code, opt: e.ty.strip_quals().has_none_arm() })
            }
            // `x!`: the path, or the trap.
            ExprKind::Branch { arms, otherwise: None, .. } if arms.len() == 1 && matches!(arms[0].0.kind, ExprKind::Bool(true)) => {
                let b = &arms[0].1;
                let [Stmt::Let { local, value, .. }] = b.stmts.as_slice() else { return None };
                let v = b.value.as_ref()?;
                let ExprKind::Switch { subject, arms: sarms, .. } = &v.kind else { return None };
                if super::body::bare(subject) != Some(local) {
                    return None;
                }
                let at = sarms.iter().find_map(|a| {
                    let from_stmt = a.body.stmts.iter().find_map(|s| match s {
                        Stmt::Expr(salvo_ir::Expr { kind: ExprKind::Unreachable { at, .. }, .. }) => Some(at.clone()),
                        _ => None,
                    });
                    match a.body.value.as_deref().map(|x| &x.kind) {
                        Some(ExprKind::Unreachable { at, .. }) => Some(at.clone()),
                        _ => from_stmt,
                    }
                })?;
                let mut m = self.mint(value, indent)?;
                if m.opt {
                    m.code = format!("{}.expect(\"salvo: value is absent at {at}\")", m.code);
                    m.opt = false;
                }
                Some(m)
            }
            ExprKind::Present { value } => {
                let mut m = self.mint(value, indent)?;
                if m.opt {
                    m.code = format!("{}.expect(\"salvo: value is absent\")", m.code);
                    m.opt = false;
                }
                Some(m)
            }
            _ => None,
        }
    }

    /// A mint used as a value: its element borrowed `&mut` (or the
    /// optional of it), the path walked once.
    pub fn materialize(&mut self, m: Mint) -> String {
        let x = self.fresh("l");
        let mut steps = m.prefix.steps.clone();
        steps.extend(self.steps_of_value(&m.pt, &x));
        let place = self.render_path(&m.prefix.root, &steps, true);
        let pre = if m.pre.is_empty() { String::new() } else { format!("{} ", m.pre.join(" ")) };
        if m.opt {
            format!("{{ {pre}match {} {{ Some({x}) => Some(&mut {place}), None => None }} }}", m.code)
        } else {
            format!("{{ {pre}let {x} = {}; &mut {place} }}", m.code)
        }
    }
}
