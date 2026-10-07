//! The IR builder [ir-build]: from the checked program to the IR, copying
//! every answer the checker recorded (IR.md §2). Nothing here infers; where
//! the checker left no answer the node is `Unsupported` and an error names it.

use std::collections::{HashMap, HashSet};

use salvo_core::check::{Checked, Coercion};
use salvo_core::program::{Program as CoreProgram, Symbols};
use salvo_core::resolve::Resolution;
use salvo_core::types::{Qual, Ty};
use salvo_core::FnKey;
use salvo_syntax::ast::{self, Expr as AExpr, Item};
use salvo_syntax::Span;

use crate::ir::*;

mod body;
mod decls;

pub use body::Lower;

/// Everything the builder reads, shared by every module.
pub struct Ctx<'p> {
    pub program: &'p CoreProgram,
    pub symbols: &'p Symbols<'p>,
    pub resolution: &'p Resolution<'p>,
    pub checked: &'p Checked,
    /// Every fn declaration's dump name, program-wide (references may reach
    /// modules that are not built).
    pub names: HashMap<DeclId, String>,
}

impl<'p> Ctx<'p> {
    pub fn decl_id(&self, key: FnKey) -> DeclId {
        DeclId { module: self.program.files[key.file].module.clone(), item: key.item, sub: 0 }
    }

    pub fn fn_by_key(&self, key: FnKey) -> Option<&'p ast::FnDecl> {
        match self.program.modules.get(key.file)?.items.get(key.item)? {
            Item::Fn(f) => Some(f),
            _ => None,
        }
    }

    /// The `DeclId` of a declaration (any item kind) by its address.
    pub fn item_id(&self, file_idx: usize, item: usize) -> DeclId {
        DeclId { module: self.program.files[file_idx].module.clone(), item, sub: 0 }
    }

    /// Where a struct/effect/handler declaration lives, by symbol key.
    pub fn decl_of_effect(&self, key: &str) -> Option<DeclId> {
        let decl: &ast::EffectDecl = self.symbols.effects.get(key).copied()?;
        let file = *self.resolution.decl_files.get(&(decl as *const ast::EffectDecl as usize))?;
        let item = self.program.modules[file]
            .items
            .iter()
            .position(|i| matches!(i, Item::Effect(e) if std::ptr::eq(e, decl)))?;
        Some(self.item_id(file, item))
    }

    pub fn written_ty(&self, file_idx: usize, ty: &ast::Type) -> Ty {
        if let Some(t) = self.checked.written_types.get(&(file_idx, ty.span())) {
            return erase(t);
        }
        match salvo_core::wire::approx_ty(ty, &HashMap::new()) {
            Some(t) => erase(&t),
            None => Ty::Unknown,
        }
    }

    pub fn ty_of(&self, file_idx: usize, span: Span) -> Option<Ty> {
        self.checked.ty_of(file_idx, span).map(erase)
    }

    fn compute_names(program: &CoreProgram) -> HashMap<DeclId, String> {
        let mut out = HashMap::new();
        for (file_idx, (file, module)) in program.files.iter().zip(&program.modules).enumerate() {
            let _ = file_idx;
            let mut by_name: HashMap<&str, usize> = HashMap::new();
            for item in &module.items {
                if let Item::Fn(f) = item {
                    *by_name.entry(f.name.name.as_str()).or_default() += 1;
                }
            }
            let prefix = file.module.0.join(".");
            for (item, decl) in module.items.iter().enumerate() {
                let name = match decl {
                    Item::Fn(f) => {
                        if by_name.get(f.name.name.as_str()).copied().unwrap_or(0) > 1 {
                            let params: Vec<String> = f
                                .params
                                .iter()
                                .filter(|p| !p.implicit)
                                .map(|p| p.ty.to_string())
                                .collect();
                            format!("{prefix}::{}({})", f.name.name, params.join(", "))
                        } else {
                            format!("{prefix}::{}", f.name.name)
                        }
                    }
                    Item::Struct(s) => format!("{prefix}::{}", s.name.name),
                    Item::Effect(e) => format!("{prefix}::{}", e.name.name),
                    Item::Handler(h) => format!("{prefix}::{}", h.name.name),
                    Item::Type(t) => format!("{prefix}::{}", t.name.name),
                    Item::Qualifier(q) => format!("{prefix}::{}", q.name.name),
                    _ => continue,
                };
                out.insert(DeclId { module: file.module.clone(), item, sub: 0 }, name);
                if let Item::Qualifier(q) = decl {
                    for (i, f) in q.fns.iter().enumerate() {
                        out.insert(DeclId { module: file.module.clone(), item, sub: i as u16 + 1 }, format!("{prefix}::{}.{}", q.name.name, f.name.name));
                    }
                }
            }
        }
        out
    }
}

impl Program {
    /// The dump spelling of a declaration reference.
    pub fn ref_name(&self, id: &DeclId) -> String {
        self.names.get(id).cloned().unwrap_or_else(|| format!("{}::#{}.{}", id.module.0.join("."), id.item, id.sub))
    }
}

/// [ir-types] Qualifiers are erased from every type the IR carries, except
/// `Mut`, the one a backend may represent as a different type.
pub fn erase(ty: &Ty) -> Ty {
    // [type-literal] literals collapse into their base first, so a union of
    // literals of one base is that base.
    if salvo_core::literal::mentions_lit(ty) {
        return erase(&salvo_core::literal::collapse_ty(ty));
    }
    match ty {
        Ty::Qualified { quals, base } => {
            let base = erase(base);
            if quals.iter().any(|q| q.name == "Mut") {
                base.qualify(vec![Qual::plain("Mut", Vec::new())])
            } else {
                base
            }
        }
        Ty::Named { name, args } => Ty::Named { name: name.clone(), args: args.iter().map(erase).collect() },
        Ty::Union(arms) => Ty::Union(arms.iter().map(erase).collect()),
        Ty::Tuple(es) => Ty::Tuple(es.iter().map(erase).collect()),
        Ty::Array(e) => Ty::Array(Box::new(erase(e))),
        Ty::Fn { params, ret, contract, effects } => Ty::Fn {
            params: params.iter().map(erase).collect(),
            ret: Box::new(erase(ret)),
            contract: contract.clone(),
            effects: effects.iter().map(erase).collect(),
        },
        // A literal type is its base at run time [type-literal].
        Ty::Lit(l) => Ty::named(l.base()),
        other => other.clone(),
    }
}

/// Builds the IR of every module in `modules` (all of them when `None`).
pub fn build_program<'p>(
    program: &'p CoreProgram,
    symbols: &'p Symbols<'p>,
    resolution: &'p Resolution<'p>,
    checked: &'p Checked,
    modules: Option<&HashSet<&salvo_core::ModulePath>>,
) -> (Program, Vec<String>) {
    let ctx = Ctx { program, symbols, resolution, checked, names: Ctx::compute_names(program) };
    let mut out = Program { modules: Vec::new(), names: HashMap::new(), facts: Facts::default() };
    let mut errors = Vec::new();
    for (file_idx, (file, module)) in program.files.iter().zip(&program.modules).enumerate() {
        if let Some(wanted) = modules {
            if !wanted.contains(&file.module) {
                continue;
            }
        }
        let (m, errs) = decls::build_module(&ctx, file_idx, file, module);
        out.modules.push(m);
        errors.extend(errs.into_iter().map(|e| format!("{}: {e}", file.name)));
    }
    out.names = ctx.names;
    out.facts.protocol_hashes = checked.protocol_hashes.clone();
    (out, errors)
}

/// The span-keyed coercion a slot recorded for an expression, applied as IR
/// nodes around it [ir-coerce].
pub fn apply_coercion(c: &Coercion, value: Expr, span: Span) -> Expr {
    match c {
        Coercion::WrapUnion { target, arm, inner } => {
            let value = match inner {
                Some(i) => apply_coercion(i, value, span),
                None => value,
            };
            Expr { ty: erase(target), span, kind: ExprKind::MakeUnion { arm: *arm, value: Box::new(value) } }
        }
        Coercion::Rewrap { from, to } => {
            Expr { ty: erase(to), span, kind: ExprKind::Rewrap { from: erase(from), value: Box::new(value) } }
        }
        Coercion::WrapOption { target } => {
            // The absent arm is `MakeNone`; a present value is itself, typed
            // at the optional.
            let mut v = value;
            v.ty = erase(target);
            v
        }
        Coercion::NoneUnit => Expr { ty: Ty::none(), span, kind: ExprKind::Unit },
        Coercion::DropMut { from, then } => {
            let dropped = Expr {
                ty: erase(from).strip_quals().clone(),
                span,
                kind: ExprKind::DropMut { value: Box::new(value) },
            };
            match then {
                Some(t) => apply_coercion(t, dropped, span),
                None => dropped,
            }
        }
    }
}

pub(crate) fn unsupported(ty: Ty, span: Span, what: impl Into<String>) -> Expr {
    Expr { ty, span, kind: ExprKind::Unsupported(what.into()) }
}

pub(crate) fn stmt_is_divergent(e: &AExpr) -> bool {
    matches!(e, AExpr::Return { .. } | AExpr::Break { .. } | AExpr::Continue { .. })
}

pub(crate) use salvo_syntax::ast::Module as AModule;
