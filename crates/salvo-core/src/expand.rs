//! The pre-resolution expansions.
//!
//! Per file ([`expand_file`]): the struct-body hoist, the `iter fn`
//! expansion [iter-fn] and the `test` expansion [test-decl], which needs to
//! know whether a file is a test annex [test-file]. Per program
//! ([`expand_program`]): comptime stamping. Effect aliases and
//! prerequisites are resolved, not expanded (PIPELINE.md R6), and routes are
//! derived in the IR (R7). Every driver needs both, so [`expand`] is the one call
//! that does both, and a driver caching parses calls the halves.

use std::sync::Arc;

use salvo_syntax::ast::{Item, Module};

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
    /// [test-kind] What the test runs on.
    pub kind: salvo_syntax::ast::TestKind,
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
    /// [comptime-fields] Hover text for the comptime names inside `comptime
    /// fn` bodies, which the checker never sees.
    pub comptime_hovers: Vec<crate::comptime::CompHover>,
    /// [lsp-completion] The arms each comptime `[when …]` has yet to cover.
    pub comptime_when_arms: Vec<crate::comptime::CompWhenArms>,
}

/// [test-run] A test as one file's expansion finds it: a [`TestCase`]
/// before the file has a place in a source set.
#[derive(Clone, Debug, PartialEq)]
pub struct FileTest {
    pub tested: ModulePath,
    pub name: String,
    pub fn_name: String,
    pub kind: salvo_syntax::ast::TestKind,
}

/// The expansions one file needs and nothing else (PIPELINE.md step 6): the
/// struct-body hoist [fn-attached], every `iter fn` [iter-fn], and the `test`
/// blocks [test-decl] — expanded in a test annex, refused and dropped in a
/// production file [test-file]. A function of the file's text, its module path
/// and whether it is an annex, so a driver may cache its result with the parse.
pub fn expand_file(file: &SourceFile, module: &mut Module) -> (Vec<FileTest>, Vec<salvo_syntax::Diagnostic>) {
    let mut diags = salvo_syntax::desugar::expand_iter_fns(module);
    let mut found = Vec::new();
    if file.is_test {
        let mangled = file.module.0.join("_");
        let (tests, test_diags) = salvo_syntax::desugar::expand_tests(module, &mangled);
        diags.extend(test_diags);
        let mut tested = file.module.0.clone();
        tested.pop();
        let tested = ModulePath(tested);
        found.extend(tests.into_iter().map(|t| FileTest {
            tested: tested.clone(),
            name: t.name,
            fn_name: t.fn_name,
            kind: t.kind,
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
    // [node-id] The file's tree is final until comptime stamps into it.
    salvo_syntax::node_id::number(module);
    (found, diags)
}

/// [lsp-std-front] A module an expansion may rewrite: an owned one, or one a
/// `Program` holds, which is rewritten in place when the program owns it
/// alone and copied first when it is shared (a language server's parse
/// cache keeps the parsed module; `Arc::make_mut`).
pub trait ModuleSlot: crate::decl_id::ModuleRef {
    fn module_mut(&mut self) -> &mut Module;
}

impl ModuleSlot for Module {
    fn module_mut(&mut self) -> &mut Module {
        self
    }
}

impl ModuleSlot for Arc<Module> {
    fn module_mut(&mut self) -> &mut Module {
        Arc::make_mut(self)
    }
}

/// Expands every file ([`expand_file`]) and then the program
/// ([`expand_program`]); `modules` must be aligned with `files`.
pub fn expand<M: ModuleSlot>(files: &[SourceFile], modules: &mut [M]) -> Expansion {
    let mut per_file = Vec::with_capacity(files.len());
    let mut diagnostics = Vec::new();
    for (file_idx, (file, module)) in files.iter().zip(modules.iter_mut()).enumerate() {
        let (tests, diags) = expand_file(file, module.module_mut());
        diagnostics.extend(diags.into_iter().map(|d| FileDiagnostic::from_parse(file_idx, d)));
        per_file.push(tests);
    }
    let mut out = expand_program(files, modules, per_file);
    diagnostics.append(&mut out.diagnostics);
    out.diagnostics = diagnostics;
    out
}

/// The expansions that read more than one file, over modules each already
/// through [`expand_file`] (whose tests come in [per_file], aligned with
/// `files`): comptime stamping.
pub fn expand_program<M: ModuleSlot>(files: &[SourceFile], modules: &mut [M], per_file: Vec<Vec<FileTest>>) -> Expansion {
    // [comptime-instantiate] After the per-module expansions (the struct-body
    // hoist has put every `by` declaration at module level): every `by` site
    // stamps its compfn, and no compfn survives.
    let comptime = crate::comptime::expand_comptime(files, modules);
    program_expansion(per_file, comptime)
}

/// [lsp-std-front] What [`expand_program`] does, over a program whose leading
/// files are a std that `prefix` recorded the expansion of
/// (`comptime::expand_comptime_prefix`), already expanded: only the files
/// after it are stamped. The std files' own expansion output is the
/// prefix's, which the caller adds; this one reports the rest.
pub fn expand_program_after(
    files: &[SourceFile],
    modules: &mut [Arc<Module>],
    per_file: Vec<Vec<FileTest>>,
    prefix: &crate::comptime::StdComptime,
) -> Expansion {
    let comptime = crate::comptime::expand_comptime_after(files, modules, prefix);
    program_expansion(per_file, comptime)
}

/// The program expansion's report: the tests of every file, aligned with the
/// files, and the comptime expansion's output.
pub fn program_expansion(per_file: Vec<Vec<FileTest>>, comptime: crate::comptime::ComptimeOutput) -> Expansion {
    let mut out = Expansion::default();
    for (file_idx, tests) in per_file.into_iter().enumerate() {
        out.tests.extend(tests.into_iter().map(|t| TestCase {
            file: file_idx,
            tested: t.tested,
            name: t.name,
            fn_name: t.fn_name,
            kind: t.kind,
        }));
    }
    for (file_idx, d) in comptime.diagnostics {
        out.diagnostics.push(FileDiagnostic::from_parse(file_idx, d));
    }
    out.comptime_hovers = comptime.hovers;
    out.comptime_when_arms = comptime.when_arms;
    // [actor-route-stub] Routes are not an expansion: the IR derives one per
    // routed protocol (PIPELINE.md R7).
    // [effect-alias] [effect-prereq] Aliases and prerequisites are not
    // expansions: aliases resolve in scope and both are expanded where an
    // effect list is lowered (PIPELINE.md R6).
    out
}

/// Whether a module declares a `main` with a body: an entry-point candidate.
pub fn declares_main(module: &Module) -> bool {
    module.items.iter().any(|item| {
        matches!(item, Item::Fn(f) if f.name.name == "main" && f.body.is_some())
    })
}
