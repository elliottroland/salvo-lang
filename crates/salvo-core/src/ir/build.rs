//! The IR builder [ir-build]: from the checked program to the IR, copying
//! every answer the checker recorded (IR record §2). Nothing here infers; where
//! the checker left no answer the node is `Unsupported` and an error names it.

use std::collections::{HashMap, HashSet};

use crate::check::Checked;
use crate::program::{Program as CoreProgram, Symbols};
use crate::resolve::Resolution;
use crate::types::Ty;
use crate::DeclId;
use salvo_syntax::ast::{self, Expr as AExpr, Item};
use salvo_syntax::Span;

use crate::ir::*;

mod body;
mod erased;
pub mod actors;
pub mod refs;
mod decls;

pub use body::Lower;
pub use erased::{apply_convert, erase};

/// Everything the builder reads, shared by every module.
pub struct Ctx<'p> {
    pub program: &'p CoreProgram,
    pub symbols: &'p Symbols<'p>,
    pub resolution: &'p Resolution<'p>,
    pub checked: &'p Checked,
    /// Every fn declaration's dump name, program-wide (references may reach
    /// modules that are not built).
    pub names: HashMap<DeclId, String>,
    /// [fate-move-mode] The IR flow pass's decision the bodies are built
    /// from (empty on the first build: every binding borrow-mode).
    pub decision: Decision,
    /// [fate-move-mode] Every file's binding facts, by stamp: what a body's
    /// decision reads about the bindings in it.
    pub facts_by_decl: HashMap<DeclId, Vec<&'p crate::check::BindFact>>,
    /// [ir-analyze] What the program's errors broke: those declarations are
    /// built without their code, a broken file not at all.
    pub gate: &'p super::gate::Gate,
}

impl<'p> Ctx<'p> {
    /// [fate-move-mode] Whether the binding statement (or the loop, by its
    /// iterable) at `key` takes ownership: lowered as a move. The flow
    /// pass's decision is the one answer (PIPELINE.md D3.5(a)): it starts
    /// from the loops the checker saw data moved out of through their
    /// binding (`Checked::by_value_loops`).
    pub fn mode(&self, key: crate::check::Key) -> bool {
        self.decision.modes.contains(&key)
    }

    /// [deduce-field] Whether the binding at `key` is a virtual place.
    pub fn virtual_bind(&self, key: crate::check::Key) -> bool {
        self.decision.virtual_binds.contains_key(&key)
    }

    /// [fate-move-mode] A binding's links as the flow pass reads them, and
    /// whether it took ownership: the checker's facts (`bind_facts`), less
    /// the copies of links an ancestor binding that took ownership made
    /// (its descendant links to the owner, not to what the owner took),
    /// one per root.
    pub fn bind_links(&self, file_idx: usize, at: Span) -> Option<(bool, Vec<crate::check::FlowLink>)> {
        let hint = std::cell::Cell::new(0);
        let fact = self.checked.bind_facts.get(&self.checked.decls.key(file_idx, at, &hint))?;
        let decided = |s: crate::check::Key| self.decision.decided.contains(&s);
        let mut links: Vec<crate::check::FlowLink> = Vec::new();
        for l in &fact.links {
            if l.stamp != fact.stamp && decided(l.stamp) {
                continue;
            }
            if !links.iter().any(|e| e.root == l.root) {
                links.push(l.clone());
            }
        }
        Some((decided(fact.stamp), links))
    }
    pub fn fn_by_key(&self, key: DeclId) -> Option<&'p ast::FnDecl> {
        self.resolution.decls.fn_decl(&self.program.modules, key)
    }

    /// [op-equality] [op-order] [op-arith] Whether the declaration `key` is
    /// a primitive operation, as the checker recorded (`Checked::primitives`).
    pub fn is_primitive_operator(&self, key: DeclId) -> bool {
        matches!(self.checked.primitives.get(&crate::check::Key::name_of(key)), Some(crate::check::Primitive::Operator))
    }

    /// [decl-id] The id of the declaration at (`file_idx`, `item`), if it
    /// declares something.
    pub fn item_id(&self, file_idx: usize, item: usize) -> Option<DeclId> {
        self.resolution.decls.item(file_idx, item)
    }

    /// [decl-id] The item index `id` is declared at in its file (a member's
    /// container's); `usize::MAX` for a generated declaration.
    pub fn item_of(&self, id: DeclId) -> usize {
        self.resolution.decls.loc(id).map_or(usize::MAX, |l| l.item)
    }

    /// [dump-spelling] Every declaration's dump name (PIPELINE.md §II.10
    /// P22): `name@module`, with its parameter types — `to_str(Int)@std.core.basic`
    /// — when its file declares the name more than once.
    fn compute_names(program: &CoreProgram, decls: &crate::decl_id::DeclIndex) -> HashMap<DeclId, String> {
        let mut out = HashMap::new();
        for (file_idx, (file, module)) in program.files.iter().zip(&program.modules).enumerate() {
            let mut by_name: HashMap<&str, usize> = HashMap::new();
            for item in &module.items {
                if let Item::Fn(f) = item {
                    *by_name.entry(f.name.name.as_str()).or_default() += 1;
                }
            }
            let at = file.module.0.join(".");
            for (item, decl) in module.items.iter().enumerate() {
                let name = match decl {
                    Item::Fn(f) if by_name.get(f.name.name.as_str()).copied().unwrap_or(0) > 1 => {
                        format!("{}({})@{at}", f.name.name, written_params(&f.params))
                    }
                    Item::Fn(f) => format!("{}@{at}", f.name.name),
                    Item::Struct(s) => format!("{}@{at}", s.name.name),
                    Item::Effect(e) => format!("{}@{at}", e.name.name),
                    Item::Handler(h) => format!("{}@{at}", h.name.name),
                    Item::Type(t) => format!("{}@{at}", t.name.name),
                    Item::Qualifier(q) => format!("{}@{at}", q.name.name),
                    _ => continue,
                };
                let Some(id) = decls.item(file_idx, item) else { continue };
                out.insert(id, name);
                if let Item::Qualifier(q) = decl {
                    for (i, f) in q.fns.iter().enumerate() {
                        if let Some(id) = decls.member(file_idx, item, i + 1) {
                            out.insert(id, format!("{}.{}@{at}", q.name.name, f.name.name));
                        }
                    }
                }
            }
        }
        out
    }

    /// [dump-spelling] Every effect member's dump name: `greet@Greeter`, the
    /// selector Salvo writes, with its parameter types when the effect
    /// declares the name more than once.
    fn compute_member_names(program: &CoreProgram, decls: &crate::decl_id::DeclIndex) -> HashMap<(DeclId, usize), String> {
        let mut out = HashMap::new();
        for (file_idx, module) in program.modules.iter().enumerate() {
            for (item, decl) in module.items.iter().enumerate() {
                let Item::Effect(e) = decl else { continue };
                let Some(id) = decls.item(file_idx, item) else { continue };
                for (i, f) in e.fns.iter().enumerate() {
                    let overloaded = e.fns.iter().filter(|g| g.name.name == f.name.name).count() > 1;
                    let name = if overloaded {
                        format!("{}({})@{}", f.name.name, written_params(&f.params), e.name.name)
                    } else {
                        format!("{}@{}", f.name.name, e.name.name)
                    };
                    out.insert((id, i), name);
                }
            }
        }
        out
    }
}

/// [dump-spelling] A fn's written parameter types, the implicit ones left out.
fn written_params(params: &[ast::Param]) -> String {
    params.iter().filter(|p| !p.implicit).map(|p| p.ty.to_string()).collect::<Vec<_>>().join(", ")
}

impl Program {
    /// [dump-spelling] The dump spelling of a declaration reference:
    /// `name@module` (a generated declaration's own name).
    pub fn ref_name(&self, id: &DeclId) -> String {
        self.names.get(id).cloned().unwrap_or_else(|| format!("{}@{}", id.path().join("."), id.module().0.join(".")))
    }

    /// [dump-spelling] The dump spelling of an effect member, `greet@Greeter`
    /// (`Effect#index` for one the program does not declare).
    pub fn member_name(&self, m: &MemberRef) -> String {
        self.member_names.get(&(m.interface, m.index)).cloned().unwrap_or_else(|| format!("{}#{}", self.ref_name(&m.interface), m.index))
    }
}

/// Builds the IR of every module in `modules` (all of them when `None`), of
/// a program with no errors.
pub fn build_program<'p>(
    program: &'p CoreProgram,
    symbols: &'p Symbols<'p>,
    resolution: &'p Resolution<'p>,
    checked: &'p Checked,
    modules: Option<&HashSet<&crate::ModulePath>>,
) -> (Program, Vec<String>) {
    let gate = super::gate::Gate::default();
    let (ir, errors, _) = build_program_gated(program, symbols, resolution, checked, modules, &gate);
    (ir, errors)
}

/// [ir-analyze] `build_program` past a program's errors: a declaration
/// `gate` says broke is built without its code, a broken file not at all.
/// Also answers the items (by file and item index) whose build reported an
/// error.
pub fn build_program_gated<'p>(
    program: &'p CoreProgram,
    symbols: &'p Symbols<'p>,
    resolution: &'p Resolution<'p>,
    checked: &'p Checked,
    modules: Option<&HashSet<&crate::ModulePath>>,
    gate: &'p super::gate::Gate,
) -> (Program, Vec<String>, HashSet<(usize, usize)>) {
    let _t = crate::timing::phase("ir build");
    let mut facts_by_decl: HashMap<DeclId, Vec<&crate::check::BindFact>> = HashMap::new();
    for (k, fact) in &checked.bind_facts {
        facts_by_decl.entry(k.decl).or_default().push(fact);
    }
    for v in facts_by_decl.values_mut() {
        // In source order, as the bindings are written.
        v.sort_by_key(|f| f.stamp.span);
    }
    // [fate-move-mode] The decision starts from the loops the checker saw
    // iterate by value; the flow pass adds the bindings that take ownership.
    let decision = Decision { modes: checked.by_value_loops.clone(), ..Decision::default() };
    let mut ctx = Ctx { program, symbols, resolution, checked, names: Ctx::compute_names(program, &resolution.decls), decision, facts_by_decl, gate };
    let mut out = Program { modules: Vec::new(), names: HashMap::new(), member_names: Ctx::compute_member_names(program, &resolution.decls), decision: Decision::default() };
    let mut errors = Vec::new();
    let mut failed: HashSet<(usize, usize)> = HashSet::new();
    let mut built: Vec<(usize, decls::ModuleBuild)> = Vec::new();
    let t = crate::timing::phase("  ir build: bodies");
    for (file_idx, (file, module)) in program.files.iter().zip(&program.modules).enumerate() {
        if let Some(wanted) = modules {
            if !wanted.contains(&file.module) {
                continue;
            }
        }
        if gate.files.contains(&file_idx) {
            continue;
        }
        built.push((file_idx, decls::build_module(&ctx, file_idx, file, module)));
    }
    // [fate-move-mode] The flow pass decides, over the bodies built with
    // every binding borrow-mode, which bindings take ownership (and which
    // stay virtual places [deduce-field]); a declaration it decided anything
    // in is built again from the decision (PIPELINE.md D3.6: the item, not
    // its module), which is what the emitters and the flow pass's own check
    // read.
    drop(t);
    {
        let t = crate::timing::phase("  ir build: decide");
        let mut redo: Vec<(usize, usize)> = Vec::new();
        for (i, (_, mb)) in built.iter().enumerate() {
            for (d, decl) in mb.module.decls.iter().enumerate() {
                if super::flow::decide(decl, &mut ctx.decision) {
                    if let Some(item) = mb.item_of(d) {
                        if !redo.contains(&(i, item)) {
                            redo.push((i, item));
                        }
                    }
                }
            }
        }
        drop(t);
        let _t = crate::timing::phase("  ir build: again");
        for (i, item) in redo {
            let (file_idx, mb) = &mut built[i];
            mb.rebuild_item(&ctx, *file_idx, &program.modules[*file_idx], item);
        }
    }
    for (file_idx, mb) in built {
        let name = &program.files[file_idx].name;
        errors.extend(mb.errors().into_iter().map(|e| format!("{name}: {e}")));
        failed.extend(mb.failed().into_iter().map(|i| (file_idx, i)));
        out.modules.push(mb.module);
    }
    let generated = actors::generate(&ctx, &mut out);
    // [ref-handle] Every `ref` names its container's type.
    refs::annotate(&mut out);
    out.names = ctx.names;
    out.names.extend(generated);
    out.decision = ctx.decision;
    (out, errors, failed)
}

pub(crate) fn unsupported(ty: Ty, span: Span, what: impl Into<String>) -> Expr {
    Expr { ty, span, kind: ExprKind::Unsupported(what.into()) }
}

pub(crate) fn stmt_is_divergent(e: &AExpr) -> bool {
    matches!(e, AExpr::Return { .. } | AExpr::Break { .. } | AExpr::Continue { .. })
}

pub(crate) use salvo_syntax::ast::Module as AModule;
