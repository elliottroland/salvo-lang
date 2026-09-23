//! The pre-resolution expansions, run over a whole source set.
//!
//! Two desugarings live in `salvo-syntax` and must happen before anything
//! resolves: the `iter fn` expansion [iter-fn], which needs the *rest of the
//! program*'s struct declarations, and the `test` expansion [test-decl],
//! which needs to know whether a file is a test annex [test-file]. Every
//! driver — `salvo compile`, `salvo run`, `salvo analyze`, the language
//! server, `salvo test` — needs both, so they are one call here rather than a
//! step each pipeline can forget.

use salvo_syntax::ast::{Item, Module, StructDecl};

use crate::diag::FileDiagnostic;
use crate::source::{ModulePath, SourceFile};

/// [test-run] One test the source set declares.
#[derive(Clone, Debug, PartialEq)]
pub struct TestCase {
    /// Index into the source set's files: the annex declaring it.
    pub file: usize,
    /// The module **under test** — the annex's module without its trailing
    /// `test` segment, which is what the report calls the test and what a
    /// program would `import` [test-report].
    pub tested: ModulePath,
    /// The name as written in the `test "…"` declaration.
    pub name: String,
    /// The synthesized fn the harness calls [test-run].
    pub fn_name: String,
}

impl TestCase {
    /// [test-report] The test's full identity: `heap :: pushes come back in
    /// order`. What the report prints, what `--list` lists, and what a
    /// filter matches against.
    pub fn id(&self) -> String {
        format!("{} :: {}", self.tested, self.name)
    }
}

/// What [`expand`] found.
#[derive(Default)]
pub struct Expansion {
    /// Every test, in file then declaration order.
    pub tests: Vec<TestCase>,
    pub diagnostics: Vec<FileDiagnostic>,
}

/// Expands every `iter fn` [iter-fn] and every `test` block [test-decl] in
/// `modules`, which must be aligned with `files`.
///
/// A `test` block is expanded only in a test annex [test-file]; one in a
/// production file is refused here and dropped, so the rule is enforced once,
/// for every driver, before resolution.
pub fn expand(files: &[SourceFile], modules: &mut [Module]) -> Expansion {
    let mut out = Expansion::default();
    let all_structs: Vec<StructDecl> = modules
        .iter()
        .flat_map(salvo_syntax::desugar::struct_decls)
        .collect();
    for (file_idx, (file, module)) in files.iter().zip(modules.iter_mut()).enumerate() {
        let mut diags = salvo_syntax::desugar::expand_iter_fns_with(module, &all_structs);
        if file.is_test {
            let mangled = file.module.0.join("_");
            let (tests, test_diags) = salvo_syntax::desugar::expand_tests(module, &mangled);
            diags.extend(test_diags);
            let mut tested = file.module.0.clone();
            tested.pop();
            let tested = ModulePath(tested);
            out.tests.extend(tests.into_iter().map(|t| TestCase {
                file: file_idx,
                tested: tested.clone(),
                name: t.name,
                fn_name: t.fn_name,
            }));
        } else {
            // [test-file] A `test` block in a production file: refused here,
            // and dropped, so no driver has to decide what to do with one and
            // nothing downstream ever sees the form outside an annex. The
            // diagnostic names the file that would hold it, which is the fix.
            let annex = file
                .name
                .strip_suffix(".sv")
                .map(|stem| format!("{stem}.test.sv"))
                .unwrap_or_else(|| format!("{}.test.sv", file.module));
            let mut kept = Vec::with_capacity(module.items.len());
            for item in std::mem::take(&mut module.items) {
                match item {
                    Item::Test(t) => diags.push(salvo_syntax::Diagnostic::error(
                        format!(
                            "a `test` block belongs in a test annex, not in a \
                             production source file: move it to `{annex}`, which is \
                             part of this module and sees its private declarations \
                             [test-file]"
                        ),
                        t.name_span,
                    )),
                    other => kept.push(other),
                }
            }
            module.items = kept;
        }
        out.diagnostics
            .extend(diags.into_iter().map(|d| FileDiagnostic {
                file: file_idx,
                severity: d.severity,
                message: d.message,
                span: d.span,
                suggested_imports: Vec::new(),
            }));
    }
    out
}

/// Whether a module declares a `main` with a body: an entry-point candidate.
pub fn declares_main(module: &Module) -> bool {
    module.items.iter().any(|item| {
        matches!(item, Item::Fn(f) if f.name.name == "main" && f.body.is_some())
    })
}
