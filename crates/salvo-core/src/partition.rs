//! [decl-rows] The checker's rows, partitioned by the declaration that owns
//! them (PIPELINE.md §II.9 A4).
//!
//! Every row of `Checked` belongs to one declaration: a node-keyed row to
//! its key's declaration (`Key::decl` [node-id]), a declaration-keyed row
//! (`fn_effects`, `implicit_params`, `deductions`, …) to that declaration, a
//! row keyed by a handler's or an effect's symbol key to that handler or
//! effect, a row placed by its node's span (a local, a body's effects, a
//! diagnostic, a written type's lowering) to the declaration that node is
//! in, and a file's own rows (its carried identities, its scope's keys) to
//! the file's `DeclKind::File` id. The program-wide indexes resolution
//! builds (`type_ref_keys`, `type_ref_span_keys`, `visible_keys`,
//! `visible_handler_keys`, `platform_closure_all`, `refinements`) belong to
//! no declaration.
//!
//! One router does the partitioning, an exhaustive destructure of `Checked`
//! (a new table is a compile error here until it says where its rows go),
//! into a [`Sink`]: by declaration (`Checked::split_decls`, what `salvo
//! facts` prints and the typed form will be built from) or by the file a
//! declaration is in (`Checked::split_files`, the check cache's per-file
//! rows [lsp-check-cache]). `Checked::insert_rows` puts a part back.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use salvo_syntax::ast::{self, Item, Module};
use salvo_syntax::visit::Visitor;
use salvo_syntax::Span;

use crate::check::Checked;
use crate::decl_id::{DeclId, DeclIndex};
use crate::diag::FileDiagnostic;
use crate::program::{Program, Symbols};

/// [decl-rows] Which rows a partition takes.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Scope {
    /// The keyed rows the check cache stores per file: everything but what it
    /// keeps per walk step (diagnostics, the tables kept in order,
    /// `written_types`) and what every check recomputes (the deductions, the
    /// walk's `param_mutations`, the program-wide indexes, the parking set,
    /// the protocol hashes).
    Keyed,
    /// Every row.
    All,
}

/// [decl-rows] Where the router puts a declaration's rows.
pub(crate) trait Sink {
    /// The part declaration `decl`'s rows go to; `None` drops them.
    fn part(&mut self, decl: DeclId) -> Option<&mut Checked>;
    /// The part the rows of no declaration go to (`Scope::All` only).
    fn rest(&mut self) -> Option<&mut Checked>;
    /// Whether any declaration of file `file` has a part: a table placed by
    /// file and span skips the files that answer no.
    fn wants_file(&self, _file: usize) -> bool {
        true
    }
}

/// [decl-rows] What places the rows not keyed by a declaration or a node:
/// the program's declarations, its handlers' and effects' symbol keys, and
/// (for `Scope::All`) which declaration each `ast::Type` node is in.
pub(crate) struct Owners {
    decls: Arc<DeclIndex>,
    named: HashMap<String, DeclId>,
    types: HashMap<usize, DeclId>,
}

impl Owners {
    /// The owners in `files` of `program`: with `types`, also where each of
    /// their written types is (what `written_types` rows are placed by).
    pub(crate) fn new(program: &Program, symbols: &Symbols<'_>, decls: &Arc<DeclIndex>, files: &[usize], types: bool) -> Owners {
        let mut named = HashMap::new();
        for &f in files {
            for (i, item) in program.modules[f].items.iter().enumerate() {
                let key = match item {
                    Item::Handler(h) => symbols.key_or(h, &h.name.name),
                    Item::Effect(e) => symbols.key_or(e, &e.name.name),
                    _ => continue,
                };
                if let Some(id) = decls.item(f, i) {
                    named.insert(key.to_string(), id);
                }
            }
        }
        let mut owners = Owners { decls: decls.clone(), named, types: HashMap::new() };
        if types {
            for &f in files {
                owners.collect_types(f, &program.modules[f]);
            }
        }
        owners
    }

    fn collect_types(&mut self, file: usize, module: &Module) {
        struct Collect<'o> {
            file: usize,
            decls: &'o DeclIndex,
            hint: std::cell::Cell<usize>,
            out: &'o mut HashMap<usize, DeclId>,
        }
        impl Visitor for Collect<'_> {
            fn visit_type(&mut self, t: &ast::Type) {
                // A qualified type's span covers its qualifiers too, which is
                // no node's: its base's is.
                let span = match t {
                    ast::Type::Named { base, .. } => base.span,
                    _ => t.span(),
                };
                let decl = self
                    .decls
                    .node_key(self.file, span, &self.hint)
                    .map(|(d, _)| d)
                    .or_else(|| self.decls.file_id(self.file));
                if let Some(d) = decl {
                    self.out.insert(crate::check::written_type_key(t), d);
                }
            }
        }
        let mut c = Collect { file, decls: &self.decls, hint: std::cell::Cell::new(0), out: &mut self.types };
        crate::check_cache::walk_module(&mut c, module);
    }

    /// The declaration the node at `span` in `file` is in; a span no
    /// numbered node holds (a covering one) is the file's own.
    fn at(&self, file: usize, span: Span, hint: &std::cell::Cell<usize>) -> Option<DeclId> {
        self.decls.node_key(file, span, hint).map(|(d, _)| d).or_else(|| self.decls.file_id(file))
    }

    /// The declaration a diagnostic was raised in: the stamped fn it names,
    /// or the one its span is in.
    fn diag(&self, d: &FileDiagnostic, hint: &std::cell::Cell<usize>) -> Option<DeclId> {
        d.decl.or_else(|| self.at(d.file, d.span, hint))
    }
}

/// [decl-rows] Parts by the file each declaration is in, for the files
/// given. Each declaration's file is looked up in the index once.
struct ByFile<'d> {
    decls: &'d DeclIndex,
    parts: Vec<Option<Checked>>,
    files: HashMap<DeclId, Option<usize>, std::hash::BuildHasherDefault<PtrHasher>>,
}

impl Sink for ByFile<'_> {
    fn part(&mut self, decl: DeclId) -> Option<&mut Checked> {
        let decls = self.decls;
        let f = (*self.files.entry(decl).or_insert_with(|| decls.loc(decl).map(|l| l.file)))?;
        self.parts.get_mut(f).and_then(Option::as_mut)
    }
    fn rest(&mut self) -> Option<&mut Checked> {
        None
    }
    fn wants_file(&self, file: usize) -> bool {
        self.parts.get(file).is_some_and(Option::is_some)
    }
}

/// A hasher for a `DeclId`, which hashes as its pointer: one multiply.
#[derive(Default)]
struct PtrHasher(u64);

impl std::hash::Hasher for PtrHasher {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, bytes: &[u8]) {
        for b in bytes {
            self.0 = (self.0 ^ *b as u64).wrapping_mul(0x100000001b3);
        }
    }
    fn write_usize(&mut self, n: usize) {
        self.0 = (n as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15).rotate_left(29);
    }
}

/// [decl-rows] Parts by declaration.
#[derive(Default)]
struct ByDecl {
    parts: BTreeMap<DeclId, Checked>,
    rest: Checked,
}

impl Sink for ByDecl {
    fn part(&mut self, decl: DeclId) -> Option<&mut Checked> {
        Some(self.parts.entry(decl).or_default())
    }
    fn rest(&mut self) -> Option<&mut Checked> {
        Some(&mut self.rest)
    }
}

/// [decl-rows] A `Checked` split by declaration (`Checked::split_decls`).
pub struct DeclRows {
    /// Each declaration's rows, in id order.
    pub parts: BTreeMap<DeclId, Checked>,
    /// The program-wide indexes, which belong to no declaration.
    pub rest: Checked,
}

impl Checked {
    /// [decl-rows] Clones every row into a `Checked` of its own per
    /// declaration (`DeclRows`), the program-wide indexes into `rest`.
    /// `program` and `symbols` are what the rows were checked from.
    pub fn split_decls(&self, program: &Program, symbols: &Symbols<'_>) -> DeclRows {
        let files: Vec<usize> = (0..program.files.len()).collect();
        let owners = Owners::new(program, symbols, &self.decls, &files, true);
        let mut sink = ByDecl::default();
        self.route(&owners, Scope::All, &mut sink);
        for part in sink.parts.values_mut() {
            part.decls = self.decls.clone();
        }
        sink.rest.decls = self.decls.clone();
        DeclRows { parts: sink.parts, rest: sink.rest }
    }

    /// [lsp-check-cache] [decl-rows] Clones the keyed rows of the
    /// declarations in [files] into a `Checked` of its own per file: a
    /// declaration's rows go to the file the index places it in (`DeclIndex::
    /// loc`), not the file a key's origin names.
    pub(crate) fn split_files(&self, n_files: usize, files: &[usize], owners: &Owners) -> Vec<Option<Checked>> {
        let mut sink = ByFile { decls: &self.decls, parts: (0..n_files).map(|_| None).collect(), files: HashMap::default() };
        for &f in files {
            sink.parts[f] = Some(Checked::default());
        }
        self.route(owners, Scope::Keyed, &mut sink);
        sink.parts
    }

    /// [decl-rows] Routes every row `scope` takes to its declaration's part
    /// of `sink`. Exhaustive by construction: a new table is a compile error
    /// here until it says where its rows go.
    pub(crate) fn route(&self, owners: &Owners, scope: Scope, sink: &mut dyn Sink) {
        let all = scope == Scope::All;
        let hint = std::cell::Cell::new(0);
        let Checked { decls: _, expr_ty, comparisons, arm_compares, promotions, repr_ty, coerce, is_tests, safe_navs, when_exhaustive, when_arms, predicate_tests, qual_member_calls, field_casts, call_fn, for_drivers, refinements, use_effects, route_uses, route_keys, parking_handlers, protocol_hashes, use_deps, use_with_items, spawn_effects, spawn_deps, spawn_dep_items, replyto_members, facade_sends, waitfor_sites, handler_constructs, replyto_tasks, task_mint_effects, use_addrs, use_addr_tuples, project_effects, merge_effects, addr_calls, self_sends, linear_moves, moved_reads, flow_moves, fate_mutations, bind_facts, derived_uses, virtual_candidates, by_value_loops, state_takes, actor_gates, actor_sends, task_sends, task_mints, effect_calls, effect_member_calls, member_calls, dot_calls, written_types, local_calls, local_call_names, call_effects, fn_effects, decl_effects, handler_deps, effect_prereqs, fn_body_effects, call_type_args, lambda_effects, fn_value_implicits, widen_targets, pick_left, elvis_picks, elvis_tests, elvis_widen, may_throw, deductions, fn_refs, def_refs, local_decls, type_ref_keys, type_ref_span_keys, visible_keys, visible_handler_keys, field_refs, module_refs, fate_reads, interp_to_str, interp_union, interp_implicit, temp_views, lending_calls, fn_lends, handle_muts, param_mutations, moved_projections, boundary_checks, factories, platform_closure_all, lambda_captures, derived_calls, distinct_pairs, mut_lend_calls, fn_value_calls, lambda_contracts, implicit_params, established_quals, carried_identities, implicit_args, use_handler_args, implicit_members, errors } = self;
        // A node-keyed map, by its key's declaration.
        macro_rules! by_key {
            ($($t:ident),* $(,)?) => {$(
                for (k, v) in $t {
                    if let Some(p) = sink.part(k.decl) {
                        p.$t.insert(*k, v.clone());
                    }
                }
            )*};
        }
        // A node-keyed set.
        macro_rules! set_by_key {
            ($($t:ident),* $(,)?) => {$(
                for k in $t {
                    if let Some(p) = sink.part(k.decl) {
                        p.$t.insert(*k);
                    }
                }
            )*};
        }
        // A declaration-keyed map.
        macro_rules! by_id {
            ($($t:ident),* $(,)?) => {$(
                for (k, v) in $t {
                    if let Some(p) = sink.part(*k) {
                        p.$t.insert(*k, v.clone());
                    }
                }
            )*};
        }
        // A map keyed by a handler's or an effect's symbol key.
        macro_rules! by_name {
            ($($t:ident),* $(,)?) => {$(
                for (k, v) in $t {
                    if let Some(p) = owners.named.get(k.as_str()).and_then(|d| sink.part(*d)) {
                        p.$t.insert(k.clone(), v.clone());
                    }
                }
            )*};
        }
        // A table kept in order, by the key at its end.
        macro_rules! in_order {
            ($($t:ident),* $(,)?) => {$(
                if all {
                    for row in $t {
                        if let Some(p) = sink.part(row_key(row).decl) {
                            p.$t.push(row.clone());
                        }
                    }
                }
            )*};
        }
        // A program-wide index: no declaration's.
        macro_rules! program_wide {
            ($($t:ident),* $(,)?) => {$(
                if all {
                    if let Some(p) = sink.rest() {
                        p.$t = $t.clone();
                    }
                }
            )*};
        }
        by_key!(
            expr_ty, comparisons, arm_compares, promotions, repr_ty, coerce, is_tests, safe_navs, when_arms, predicate_tests,
            qual_member_calls, field_casts, call_fn, for_drivers, use_effects, route_uses, use_deps, use_with_items, spawn_effects,
            spawn_deps, spawn_dep_items, replyto_members, facade_sends, replyto_tasks, task_mint_effects, use_addrs, use_addr_tuples,
            project_effects, merge_effects, addr_calls, self_sends, fate_mutations, bind_facts, derived_uses, virtual_candidates,
            effect_calls, effect_member_calls, member_calls, local_call_names, call_effects, decl_effects, call_type_args,
            lambda_effects, fn_value_implicits, widen_targets, pick_left, elvis_picks, elvis_tests, elvis_widen, may_throw, fn_refs,
            def_refs, field_refs, module_refs, fate_reads, interp_to_str, interp_union, interp_implicit, lending_calls,
            boundary_checks, factories, lambda_captures, derived_calls, distinct_pairs, fn_value_calls, lambda_contracts,
            implicit_args, use_handler_args, implicit_members,
        );
        set_by_key!(
            when_exhaustive, linear_moves, moved_reads, flow_moves, by_value_loops, state_takes, dot_calls, local_calls, temp_views,
            handle_muts, moved_projections, mut_lend_calls,
        );
        by_id!(fn_effects, fn_lends, implicit_params);
        for (k, v) in established_quals {
            if let Some(p) = sink.part(k.0) {
                p.established_quals.insert(k.clone(), v.clone());
            }
        }
        by_name!(route_keys, handler_deps, effect_prereqs);
        // [decl-rows] A local, by the node it is declared at.
        for (f, locals) in local_decls {
            if !sink.wants_file(*f) {
                continue;
            }
            for l in locals {
                if let Some(p) = owners.at(*f, l.span, &hint).and_then(|d| sink.part(d)) {
                    p.local_decls.entry(*f).or_default().push(l.clone());
                }
            }
        }
        // [cmp-carry] A file's carried identities (the first writer in the
        // file's scope): the file's own.
        for (k, v) in carried_identities {
            if !sink.wants_file(k.0) {
                continue;
            }
            if let Some(p) = owners.decls.file_id(k.0).and_then(|d| sink.part(d)) {
                p.carried_identities.insert(k.clone(), v.clone());
            }
        }
        in_order!(waitfor_sites, handler_constructs, actor_gates, actor_sends, task_sends, task_mints);
        if all {
            by_id!(deductions, param_mutations);
            by_name!(protocol_hashes);
            for h in parking_handlers {
                let p = match owners.named.get(h.as_str()) {
                    Some(d) => sink.part(*d),
                    None => sink.rest(),
                };
                if let Some(p) = p {
                    p.parking_handlers.insert(h.clone());
                }
            }
            for (at, ty) in written_types {
                // A row at no module's node (one the checker built and
                // dropped) has nothing to belong to.
                if let Some(p) = owners.types.get(at).and_then(|d| sink.part(*d)) {
                    p.written_types.insert(*at, ty.clone());
                }
            }
            for row in fn_body_effects {
                if let Some(p) = owners.at(row.0, row.1, &hint).and_then(|d| sink.part(d)) {
                    p.fn_body_effects.push(row.clone());
                }
            }
            for d in errors {
                if let Some(p) = owners.diag(d, &hint).and_then(|id| sink.part(id)) {
                    p.errors.push(d.clone());
                }
            }
        }
        program_wide!(refinements, type_ref_keys, type_ref_span_keys, visible_keys, visible_handler_keys, platform_closure_all);
    }

    /// [lsp-check-cache] [decl-rows] Inserts one part's rows (a file's,
    /// `split_files`, or a declaration's, `split_decls`): the keyed tables;
    /// what `Scope::Keyed` leaves out is the caller's.
    pub(crate) fn insert_rows(&mut self, rows: &Checked) {
        let Checked { decls: _, expr_ty, comparisons, arm_compares, promotions, repr_ty, coerce, is_tests, safe_navs, when_exhaustive, when_arms, predicate_tests, qual_member_calls, field_casts, call_fn, for_drivers, refinements, use_effects, route_uses, route_keys, parking_handlers, protocol_hashes, use_deps, use_with_items, spawn_effects, spawn_deps, spawn_dep_items, replyto_members, facade_sends, waitfor_sites, handler_constructs, replyto_tasks, task_mint_effects, use_addrs, use_addr_tuples, project_effects, merge_effects, addr_calls, self_sends, linear_moves, moved_reads, flow_moves, fate_mutations, bind_facts, derived_uses, virtual_candidates, by_value_loops, state_takes, actor_gates, actor_sends, task_sends, task_mints, effect_calls, effect_member_calls, member_calls, dot_calls, written_types, local_calls, local_call_names, call_effects, fn_effects, decl_effects, handler_deps, effect_prereqs, fn_body_effects, call_type_args, lambda_effects, fn_value_implicits, widen_targets, pick_left, elvis_picks, elvis_tests, elvis_widen, may_throw, deductions, fn_refs, def_refs, local_decls, type_ref_keys, type_ref_span_keys, visible_keys, visible_handler_keys, field_refs, module_refs, fate_reads, interp_to_str, interp_union, interp_implicit, temp_views, lending_calls, fn_lends, handle_muts, param_mutations, moved_projections, boundary_checks, factories, platform_closure_all, lambda_captures, derived_calls, distinct_pairs, mut_lend_calls, fn_value_calls, lambda_contracts, implicit_params, established_quals, carried_identities, implicit_args, use_handler_args, implicit_members, errors } = rows;
        macro_rules! map {
            ($($t:ident),* $(,)?) => {$(
                self.$t.extend($t.iter().map(|(k, v)| (k.clone(), v.clone())));
            )*};
        }
        macro_rules! set {
            ($($t:ident),* $(,)?) => {$(
                self.$t.extend($t.iter().copied());
            )*};
        }
        macro_rules! not_keyed {
            ($($t:ident),* $(,)?) => {$(
                let _ = $t;
            )*};
        }
        map!(
            expr_ty, comparisons, arm_compares, promotions, repr_ty, coerce, is_tests, safe_navs, when_arms, predicate_tests,
            qual_member_calls, field_casts, call_fn, for_drivers, use_effects, route_uses, route_keys, use_deps, use_with_items,
            spawn_effects, spawn_deps, spawn_dep_items, replyto_members, facade_sends, replyto_tasks, task_mint_effects, use_addrs,
            use_addr_tuples, project_effects, merge_effects, addr_calls, self_sends, fate_mutations, bind_facts, derived_uses,
            virtual_candidates, effect_calls, effect_member_calls, member_calls, local_call_names, call_effects, fn_effects,
            decl_effects, handler_deps, effect_prereqs, call_type_args, lambda_effects, fn_value_implicits, widen_targets, pick_left,
            elvis_picks, elvis_tests, elvis_widen, may_throw, fn_refs, def_refs, field_refs, module_refs, fate_reads,
            interp_to_str, interp_union, interp_implicit, lending_calls, fn_lends, boundary_checks, factories, lambda_captures,
            derived_calls, distinct_pairs, fn_value_calls, lambda_contracts, implicit_params, established_quals,
            carried_identities, implicit_args, use_handler_args, implicit_members,
        );
        set!(
            when_exhaustive, linear_moves, moved_reads, flow_moves, by_value_loops, state_takes, dot_calls, local_calls, temp_views,
            handle_muts, moved_projections, mut_lend_calls,
        );
        for (f, locals) in local_decls {
            self.local_decls.entry(*f).or_default().extend(locals.iter().cloned());
        }
        // Per walk step (`StepRows`), recomputed every check, or the walk's
        // own map (`param_mutations`, by the caller).
        not_keyed!(
            refinements, parking_handlers, protocol_hashes, waitfor_sites, handler_constructs, actor_gates, actor_sends, task_sends,
            task_mints, written_types, fn_body_effects, deductions, type_ref_keys, type_ref_span_keys, visible_keys,
            visible_handler_keys, param_mutations, platform_closure_all, errors,
        );
    }
}

/// The key a row kept in order ends with.
trait RowKey {
    fn key(&self) -> &crate::check::Key;
}
impl RowKey for (String, crate::check::Key) {
    fn key(&self) -> &crate::check::Key {
        &self.1
    }
}
impl RowKey for (String, String, crate::check::Key) {
    fn key(&self) -> &crate::check::Key {
        &self.2
    }
}
fn row_key<R: RowKey>(row: &R) -> &crate::check::Key {
    row.key()
}
