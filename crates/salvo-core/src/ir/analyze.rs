//! [ir-analyze] The IR-analysis stage: build the IR of a type-checked program,
//! run every IR-level analysis over it, and return the diagnostics as
//! `FileDiagnostic`s the gates (emission, `analyze`, the LSP) merge with the
//! checker's own.
//!
//! This is the home for analyses that are about *what happens as the code runs*
//! — flow, consumption, control-leaves — as opposed to the typing/resolution/
//! declaration-shape checks that stay in `check.rs` (the IR-checker record Phase 2).
//! Its tenants are the linear-obligation check (`ir::check`, on the user's own
//! modules) and the use-after-move pass (`ir::flow` [flow-use-after-move], on
//! those and their test files); the fate/release, loop-totality,
//! exhaustiveness, and deadlock analyses are to follow.
//!
//! Runs per declaration (`ir::gate`): every declaration whose own checking
//! produced no error is built and checked, whatever errors the rest of the
//! program has; a broken one reports its own errors only.

use std::collections::HashMap;

use salvo_syntax::messages::Message;
use salvo_syntax::Span;

use crate::check::Checked;
use crate::diag::FileDiagnostic;
use crate::program::{Program, Symbols};
use crate::resolve::Resolution;

/// Build the IR of `program` and run the IR-level analyses, returning their
/// diagnostics attributed to source files: on every declaration the
/// checker's errors (`checked.errors`) leave clean [ir-analyze].
pub fn analyze(program: &Program, symbols: &Symbols, resolution: &Resolution, checked: &Checked) -> Vec<FileDiagnostic> {
    analyze_full(program, symbols, resolution, checked, &[]).0
}

/// `analyze`, also answering the flow pass's decision of the bindings'
/// modes [fate-move-mode] (the user's modules), for a consumer that
/// presents them — the language server drops the fate links of a binding
/// that took ownership from its hover (`Checked::fate_reads`). A file with an
/// error among `parse` (the parser's and the expansions' diagnostics) is
/// broken wholesale.
pub fn analyze_full(
    program: &Program,
    symbols: &Symbols,
    resolution: &Resolution,
    checked: &Checked,
    parse: &[FileDiagnostic],
) -> (Vec<FileDiagnostic>, super::Decision) {
    analyze_with(program, symbols, resolution, checked, parse, None)
}

/// [lsp-ir-cache] `analyze_full` keeping what each file's analysis found in
/// `cache`, for the next analysis of the same program: a file the check did
/// not walk (`walked` is what it walked), whose module is the one analysed,
/// is not built again while what its analysis reads of the rest of the
/// program is as it was (`fingerprint`).
pub fn analyze_cached(
    program: &Program,
    symbols: &Symbols,
    resolution: &Resolution,
    checked: &Checked,
    parse: &[FileDiagnostic],
    walked: &[usize],
    cache: &mut IrCache,
) -> (Vec<FileDiagnostic>, super::Decision) {
    analyze_with(program, symbols, resolution, checked, parse, Some((walked, cache)))
}

/// [lsp-ir-cache] What one file's IR analyses found, kept across analyses.
struct IrFile {
    /// The module analysed: what was found stands for it only while the
    /// program holds this very `Arc`.
    module: std::sync::Arc<salvo_syntax::ast::Module>,
    diagnostics: Vec<FileDiagnostic>,
    /// The bindings the flow pass decided take ownership [fate-move-mode].
    decided: Vec<crate::check::Key>,
    /// Whether each of its fns throws, which the linear check of every
    /// other file reads.
    throws: Vec<(crate::DeclId, bool)>,
}

/// [lsp-ir-cache] The IR analyses' findings per file, kept by the language
/// server beside its check cache.
#[derive(Default)]
pub struct IrCache {
    files: Vec<String>,
    fingerprint: u64,
    entries: HashMap<usize, IrFile>,
    /// The files the last analysis built (the rest it kept).
    built: Vec<usize>,
}

impl IrCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// The files the last analysis built: every other file it analyses was
    /// kept.
    pub fn built(&self) -> &[usize] {
        &self.built
    }
}

/// [lsp-ir-cache] What a file's IR analyses read of the rest of the program
/// beyond the interfaces (whose changes walk the file in the check, and so
/// build it here): the gate the program's errors set, and every table the
/// checker fills from bodies that one declaration's IR reads about another
/// — deductions, pass modes, lends, dischargers, effects, established
/// claims, implicit parameters, handler shapes — or that is recomputed every
/// check. Positions take no part: a body edit moves the declarations after
/// it, which says nothing new about them.
fn fingerprint(gate: &super::gate::Gate, checked: &Checked) -> u64 {
    use std::hash::{Hash, Hasher};
    fn h<T: Hash>(t: &T) -> u64 {
        let mut s = std::collections::hash_map::DefaultHasher::new();
        t.hash(&mut s);
        s.finish()
    }
    // A table's rows in any order: the sum of each row's hash.
    fn table<V>(rows: &HashMap<crate::check::Key, V>, keep: impl Fn(&crate::check::Key) -> bool, show: impl Fn(&V) -> String) -> u64 {
        let mut sum: u64 = rows.len() as u64;
        for (k, v) in rows {
            if keep(k) {
                sum = sum.wrapping_add(h(&(k, show(v))));
            }
        }
        sum
    }
    let all = |_: &crate::check::Key| true;
    let name = |k: &crate::check::Key| k.node.0 == crate::check::NAME_NODE;
    let debug = |v: &dyn std::fmt::Debug| format!("{v:?}");
    let sorted = |set: &std::collections::HashSet<(usize, usize)>| {
        let mut v: Vec<_> = set.iter().copied().collect();
        v.sort_unstable();
        v
    };
    let mut files: Vec<usize> = gate.files.iter().copied().collect();
    files.sort_unstable();
    let parts: Vec<u64> = vec![
        h(&files),
        h(&sorted(&gate.interfaces)),
        h(&sorted(&gate.bodies)),
        table(&checked.deductions, all, |v| debug(v)),
        table(&checked.param_modes, all, |v| debug(v)),
        table(&checked.primitives, all, |v| debug(v)),
        table(&checked.fn_lends, all, |v| debug(v)),
        table(&checked.discharges, all, |v| debug(v)),
        table(&checked.effects_taken, name, |v| debug(v)),
        table(&checked.established_quals, all, |v| debug(v)),
        table(&checked.effect_prereqs, all, |v| debug(v)),
        table(&checked.handler_deps, all, |v| debug(&v.iter().map(|(t, _)| t).collect::<Vec<_>>())),
        table(&checked.handler_shapes, all, |v| debug(&crate::check::HandlerShape { any_deps: Vec::new(), ..v.clone() })),
        table(&checked.implicit_params, all, |v| {
            debug(&v.iter().map(|p| crate::check::ImplicitParam { span: Span::default(), ..p.clone() }).collect::<Vec<_>>())
        }),
        h(&format!("{:?}", checked.protocol_hashes)),
        {
            let mut p: Vec<&String> = checked.parking_handlers.iter().collect();
            p.sort();
            h(&p)
        },
        {
            let mut r: Vec<String> = checked.route_keys.iter().map(|(k, v)| {
                let mut v: Vec<_> = v.iter().collect();
                v.sort();
                format!("{k}: {v:?}")
            }).collect();
            r.sort();
            h(&r)
        },
    ];
    h(&parts)
}

fn analyze_with(
    program: &Program,
    symbols: &Symbols,
    resolution: &Resolution,
    checked: &Checked,
    parse: &[FileDiagnostic],
    mut cache: Option<(&[usize], &mut IrCache)>,
) -> (Vec<FileDiagnostic>, super::Decision) {
    let mut gate = super::gate::Gate::new(program, &resolution.decls, &checked.errors);
    for d in parse.iter().filter(|d| d.is_error()) {
        gate.break_file(d.file);
    }
    // Analyze only the user's own modules, and std's **on-disk** ones — the
    // files of std being developed [std-shadow], whose analyses are the
    // developer's to see (2026-10-30: a discharger with its `discard` taken
    // out analysed clean in `lib/`, because only the repository's tests ran
    // the IR checks over std). The embedded std is checked by those tests.
    // (Under the std-cache path std's rows are the std-only check's, seeded
    // whole, so its IR builds — emission builds it, and the TESTS.md C2
    // differential found it identical to a full check's.)
    let wanted = |f: &crate::source::SourceFile| !f.is_std || f.is_shadow;
    // [lsp-ir-cache] The files whose findings stand: kept, not built.
    let mut kept: std::collections::HashSet<usize> = std::collections::HashSet::new();
    if let Some((walked, cache)) = cache.as_mut() {
        let _t = crate::timing::phase("  ir analyze: plan");
        let fp = fingerprint(&gate, checked);
        let same = cache.fingerprint == fp && cache.files.len() == program.files.len() && cache.files.iter().zip(&program.files).all(|(a, b)| *a == b.name);
        if !same {
            cache.entries.clear();
            cache.fingerprint = fp;
            cache.files = program.files.iter().map(|f| f.name.clone()).collect();
        }
        let walked: std::collections::HashSet<usize> = walked.iter().copied().collect();
        cache.entries.retain(|f, e| !walked.contains(f) && program.modules.get(*f).is_some_and(|m| std::sync::Arc::ptr_eq(m, &e.module)));
        kept = cache.entries.keys().copied().collect();
    }
    let want = |i: usize, f: &crate::source::SourceFile| wanted(f) && !kept.contains(&i);
    let (mut ir, _build_errors, failed) = super::build::build_files_gated(program, symbols, resolution, checked, &want, &gate);
    let checkable = super::gate::Checkable::new(program, symbols, &resolution.decls, &gate, &failed);
    super::obligations::annotate(&mut ir, symbols);

    // Map each IR module's source file name to its index into `program.files`,
    // so a file-local span becomes a file-attributed diagnostic.
    let file_of: HashMap<&str, usize> = program.files.iter().enumerate().map(|(i, f)| (f.name.as_str(), i)).collect();

    let mut throws = super::check::throws_map(&ir);
    if let Some((_, cache)) = cache.as_ref() {
        for e in cache.entries.values() {
            throws.extend(e.throws.iter().cloned());
        }
    }
    // What each built file found, and whether all of it is the file's own:
    // a finding placed in or rendered from another file is never kept.
    let mut found: HashMap<usize, (Vec<FileDiagnostic>, bool)> = HashMap::new();
    for module in &ir.modules {
        let file = file_of.get(module.file_name.as_str()).copied().unwrap_or(0);
        let scope = resolution.scopes.get(file);
        let content = program.files.get(file).map(|f| f.content.as_str()).unwrap_or("");
        // The linear check runs on the user's own modules; a test file's
        // path-exit leaks are the harness's business, its flow rules (a value
        // consumed on some paths only, an assignment dropping one) are not.
        let linear = !program.files.get(file).is_some_and(|f| f.is_test);
        let (out, own) = found.entry(file).or_insert_with(|| (Vec::new(), true));
        for decl in &module.decls {
            if !checkable.check(decl) {
                continue;
            }
            // [node-origin] A diagnostic at a node a stamp copied lands where its
            // origin says, as the checker's do, attributed to the stamped fn.
            let stamped = stamp_of(program, resolution, decl);
            // [diag-id] The message keeps its id through the prefix.
            let report = |span: Span, message: Message| match stamped {
                Some((id, stamp)) if span.is_expanded() => {
                    let (span, text) = crate::origin::render(stamp, span, message.text);
                    let at = crate::origin::file_of(&program.modules, file, span);
                    FileDiagnostic::error(at, span, Message { text, ..message }).in_decl(Some(id))
                }
                _ => FileDiagnostic::error(file, span, message),
            };
            // [flow-use-after-move] A use of a moved value, naming the move.
            for mut d in super::flow::check_one(decl) {
                // [node-origin] A move in copied code is named where it lands,
                // its line counted in the file it lands in.
                let (moved_file, moved_at) = crate::origin::landing(&program.modules, file, d.moved_at);
                *own &= moved_file == file;
                d.moved_at = moved_at;
                let moved_content = program.files.get(moved_file).map_or(content, |f| f.content.as_str());
                out.push(report(d.span, d.render(moved_content)));
            }
            let linear_diags = if linear {
                super::check::check_one(decl, &throws)
            } else {
                super::check::check_one_flow_only(decl, &throws)
            };
            for d in linear_diags {
                // [linear-group] Name the dischargers in the remedy ("discharge
                // it with `drain` / `close`"), computed from the leaked type in
                // the file's resolution scope.
                let mut message = Message { id: d.id, text: d.message };
                if let (Some(ty), Some(scope)) = (&d.discharge_type, scope) {
                    let set = crate::linear::discharge_set(symbols, resolution, scope, &checked.deductions, ty);
                    if !set.is_empty() && message.text.ends_with("discharge it") {
                        let named: Vec<String> = set.iter().map(|f| format!("`{f}`")).collect();
                        message.text.push_str(&format!(" with {}", named.join(" / ")));
                    }
                }
                out.push(report(d.span, message));
            }
        }
        *own &= out.iter().all(|d| d.file == file);
    }
    let mut out: Vec<FileDiagnostic> = Vec::new();
    let mut decision = std::mem::take(&mut ir.decision);
    match cache {
        Some((_, cache)) => {
            // The built files' findings are kept for the next analysis; the
            // kept files' join them.
            let mut decided: HashMap<usize, Vec<crate::check::Key>> = HashMap::new();
            for k in &decision.decided {
                if let Some(loc) = resolution.decls.loc(k.decl) {
                    decided.entry(loc.file).or_default().push(*k);
                }
            }
            let mut fn_throws: HashMap<usize, Vec<(crate::DeclId, bool)>> = HashMap::new();
            for module in &ir.modules {
                let Some(&file) = file_of.get(module.file_name.as_str()) else { continue };
                let entry = fn_throws.entry(file).or_default();
                for d in &module.decls {
                    if let super::Decl::Fn(f) = d {
                        entry.push((f.id, f.throws.is_some()));
                    }
                }
            }
            cache.built = (0..program.files.len()).filter(|&i| want(i, &program.files[i])).collect();
            for &f in &cache.built {
                let (diagnostics, own) = found.remove(&f).unwrap_or((Vec::new(), true));
                out.extend(diagnostics.iter().cloned());
                // A file whose build failed somewhere is built again: its
                // gating is the program's.
                if own && !failed.iter().any(|(file, _)| *file == f) {
                    cache.entries.insert(f, IrFile {
                        module: program.modules[f].clone(),
                        diagnostics,
                        decided: decided.remove(&f).unwrap_or_default(),
                        throws: fn_throws.remove(&f).unwrap_or_default(),
                    });
                }
            }
            for f in &kept {
                if let Some(e) = cache.entries.get(f) {
                    out.extend(e.diagnostics.iter().cloned());
                    decision.decided.extend(e.decided.iter().copied());
                }
            }
            // Findings in no built file (a generated declaration's).
            for (_, (diagnostics, _)) in found {
                out.extend(diagnostics);
            }
        }
        None => {
            for (_, (diagnostics, _)) in found {
                out.extend(diagnostics);
            }
        }
    }
    // A leak reachable by more than one walk path reports once.
    out.sort_by(|a, b| (a.file, a.span.expn, a.span.start, &a.message).cmp(&(b.file, b.span.expn, b.span.start, &b.message)));
    out.dedup_by(|a, b| a.file == b.file && (a.span.expn, a.span.start) == (b.span.expn, b.span.start) && a.message == b.message);
    (out, decision)
}

/// [node-origin] The stamp of the fn an IR declaration was built from, when a
/// stamp copied its body: with it, a diagnostic at a copied node renders
/// where its origin says.
fn stamp_of<'p>(program: &'p Program, resolution: &Resolution, decl: &super::Decl) -> Option<(crate::DeclId, &'p salvo_syntax::ast::Stamp)> {
    let super::Decl::Fn(f) = decl else { return None };
    let loc = resolution.decls.loc(f.id)?;
    match program.modules.get(loc.file)?.items.get(loc.item)? {
        salvo_syntax::ast::Item::Fn(ast) if loc.sub == 0 => ast.stamped.as_ref().map(|s| (f.id, s)),
        _ => None,
    }
}

/// [fate-link] [fate-move-mode] The presentation table without the links a
/// binding that took ownership made: such a binding is the value's owner,
/// and nothing derived from it shares fate with what it took.
pub fn drop_owned_fate_reads(checked: &mut Checked, decision: &super::Decision) {
    checked.fate_reads.retain(|_, reads| {
        reads.retain(|r| !decision.decided.contains(&r.bind));
        !reads.is_empty()
    });
}
