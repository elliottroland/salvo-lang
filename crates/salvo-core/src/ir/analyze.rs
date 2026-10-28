//! [ir-analyze] The IR-analysis stage: build the IR of a type-checked program,
//! run every IR-level analysis over it, and return the diagnostics as
//! `FileDiagnostic`s the gates (emission, `analyze`, the LSP) merge with the
//! checker's own.
//!
//! This is the home for analyses that are about *what happens as the code runs*
//! — flow, consumption, control-leaves — as opposed to the typing/resolution/
//! declaration-shape checks that stay in `check.rs` (IR_CHECKER.md Phase 2).
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
    let mut gate = super::gate::Gate::new(program, &resolution.decls, &checked.errors);
    for d in parse.iter().filter(|d| d.is_error()) {
        gate.break_file(d.file);
    }
    // Analyze only the user's own modules: std (and a dependency) is checked
    // elsewhere. (Under the std-cache path std's rows are the std-only
    // check's, seeded whole, so its IR would build — emission builds it,
    // and the TESTS.md C2 differential found it identical to a full check's —
    // but its analyses are not the project's to report.) The user modules
    // are always fully checked, so their tables are complete.
    let wanted: std::collections::HashSet<&crate::ModulePath> = program
        .units()
        .filter(|u| !u.file.is_std)
        .map(|u| &u.file.module)
        .collect();
    let (mut ir, _build_errors, failed) = super::build::build_program_gated(program, symbols, resolution, checked, Some(&wanted), &gate);
    let checkable = super::gate::Checkable::new(program, symbols, &resolution.decls, &gate, &failed);
    super::obligations::annotate(&mut ir, symbols);

    // Map each IR module's source file name to its index into `program.files`,
    // so a file-local span becomes a file-attributed diagnostic.
    let file_of: HashMap<&str, usize> = program.files.iter().enumerate().map(|(i, f)| (f.name.as_str(), i)).collect();

    let mut out = Vec::new();
    let throws = super::check::throws_map(&ir);
    for module in &ir.modules {
        let file = file_of.get(module.file_name.as_str()).copied().unwrap_or(0);
        let scope = resolution.scopes.get(file);
        let content = program.files.get(file).map(|f| f.content.as_str()).unwrap_or("");
        // The linear check runs on the user's own modules; a test file's
        // path-exit leaks are the harness's business, its flow rules (a value
        // consumed on some paths only, an assignment dropping one) are not.
        let linear = !program.files.get(file).is_some_and(|f| f.is_test);
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
                    FileDiagnostic::error(file, span, Message { text, ..message }).in_decl(Some(id))
                }
                _ => FileDiagnostic::error(file, span, message),
            };
            // [flow-use-after-move] A use of a moved value, naming the move.
            for mut d in super::flow::check_one(decl) {
                // [node-origin] A move in copied code is named where it lands.
                if let Some(m) = program.modules.get(file) {
                    d.moved_at = crate::origin::landing(m, d.moved_at);
                }
                out.push(report(d.span, d.render(content)));
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
    }
    // A leak reachable by more than one walk path reports once.
    out.sort_by(|a, b| (a.file, a.span.expn, a.span.start, &a.message).cmp(&(b.file, b.span.expn, b.span.start, &b.message)));
    out.dedup_by(|a, b| a.file == b.file && (a.span.expn, a.span.start) == (b.span.expn, b.span.start) && a.message == b.message);
    (out, ir.decision)
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
