//! [std-cache] The one way a test builds and checks a program over the real
//! std (TESTS.md T3): std is read from `lib/`, parsed, expanded and checked
//! **once per test process** into a `salvo_core::StdFront`, and every program
//! a test builds holds that front's very modules, so only the test's own
//! files are parsed, expanded and checked.
//!
//! - [`std_front`]: the front, one per backend extension (`"kt"`, `"rs"`;
//!   they differ in std's companions only).
//! - [`program`]: a program of std and the given files, for a test that
//!   emits it (`emit_program_over`) or reads its IR.
//! - [`check`]: the program checked, with every diagnostic it produced —
//!   parse and expansion, checker, IR analysis — in that order.
//!
//! A test whose point is a different std (a std shadow, a `std = true`
//! tree, a fake std) builds its own `SourceSet`: the front is std as it is.

use std::path::Path;
use std::sync::OnceLock;

use salvo_core::source::{SourceFile, SourceSet};
use salvo_core::{FileDiagnostic, FrontProgram, Program, StdFront, Symbols};

/// The repository's std tree.
pub fn std_dir() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../lib"))
}

/// std's front for backend extension `ext` (`"kt"` or `"rs"`), built on
/// first use and shared by every thread of the process.
pub fn std_front(ext: &str) -> &'static StdFront {
    static KT: OnceLock<StdFront> = OnceLock::new();
    static RS: OnceLock<StdFront> = OnceLock::new();
    let cell = match ext {
        "kt" => &KT,
        "rs" => &RS,
        other => panic!("no std front for extension `{other}` (use \"kt\" or \"rs\")"),
    };
    cell.get_or_init(|| {
        let mut sources = SourceSet::default();
        let errors = sources.add_dir(std_dir(), ext, true, false);
        assert!(errors.is_empty(), "failed to read std: {errors:?}");
        StdFront::build(sources)
    })
}

/// The source files `files` name (`path`, `text`), none of them std: a
/// `.test.sv` path is a test annex.
pub fn source_files(files: &[(&str, &str)]) -> Vec<SourceFile> {
    let mut set = SourceSet::default();
    for (name, text) in files {
        let path = Path::new(name);
        let module = SourceSet::classify(path).unwrap_or_else(|e| panic!("{name}: {e}"));
        if SourceSet::is_test_path(path) {
            set.add_test(*name, module, text.to_string());
        } else {
            set.add(*name, module, text.to_string(), false);
        }
    }
    set.files
}

/// A program of std (for `ext`) and `files`, std's modules the front's own.
pub fn program(ext: &str, files: &[(&str, &str)]) -> FrontProgram {
    std_front(ext).program(source_files(files), Vec::new())
}

/// [`program`], asserting it parsed and expanded without an error.
pub fn clean_program(ext: &str, files: &[(&str, &str)]) -> Program {
    expect_clean(program(ext, files))
}

/// The program of `built`, asserting it parsed and expanded without an
/// error.
pub fn expect_clean(built: FrontProgram) -> Program {
    let errors: Vec<String> = built.diagnostics.iter().filter(|d| d.is_error()).map(|d| d.render(&built.program.files)).collect();
    assert!(errors.is_empty(), "parse or expansion errors: {errors:#?}");
    built.program
}

/// Checks `program` over std's front for `ext` (in full when the program
/// is not one of the front's: `StdFront::check`).
pub fn check_with(ext: &str, program: &Program, resolution: &salvo_core::Resolution<'_>, symbols: &Symbols<'_>) -> salvo_core::Checked {
    std_front(ext).check(program, resolution, symbols)
}

/// A program checked over the front, and everything it reported.
pub struct Checked {
    pub program: Program,
    pub checked: salvo_core::Checked,
    /// Parse and expansion diagnostics, std's included (there are none).
    pub parse: Vec<FileDiagnostic>,
    /// The IR analyses' diagnostics (`ir::analyze::analyze_full`).
    pub ir: Vec<FileDiagnostic>,
}

impl Checked {
    /// Every diagnostic: parse, checker, IR, in that order.
    pub fn diagnostics(&self) -> impl Iterator<Item = &FileDiagnostic> {
        self.parse.iter().chain(&self.checked.errors).chain(&self.ir)
    }

    /// Every error, rendered with its location (`file:line:col: message`).
    pub fn errors(&self) -> Vec<String> {
        self.diagnostics().filter(|d| d.is_error()).map(|d| d.render(&self.program.files)).collect()
    }

    /// Every error's message alone.
    pub fn error_messages(&self) -> Vec<String> {
        self.diagnostics().filter(|d| d.is_error()).map(|d| d.message.clone()).collect()
    }

    /// The checker's errors alone, rendered.
    pub fn checker_errors(&self) -> Vec<String> {
        self.checked.errors.iter().filter(|d| d.is_error()).map(|d| d.render(&self.program.files)).collect()
    }

    /// [diag-id] Every diagnostic's id, in order; `?` for one not yet given
    /// an id (TESTS.md T2).
    pub fn ids(&self) -> Vec<&'static str> {
        self.diagnostics().map(|d| d.id.unwrap_or("?")).collect()
    }

    /// Every warning, rendered.
    pub fn warnings(&self) -> Vec<String> {
        self.diagnostics().filter(|d| !d.is_error()).map(|d| d.render(&self.program.files)).collect()
    }
}

/// A project directory's program over std (for `ext`): the dependencies its
/// manifest names, then its source tree, as `salvo compile` loads them
/// [manifest-deps].
pub fn project(ext: &str, dir: &Path) -> FrontProgram {
    let project = salvo_core::Project::load(&dir.join(salvo_core::MANIFEST_FILE))
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()));
    let mut sources = SourceSet::default();
    for dep in project.dependencies().unwrap_or_else(|e| panic!("{e}")) {
        let errors = sources.add_dependency(&dep, ext);
        assert!(errors.is_empty(), "failed to load dependency `{}`: {errors:?}", dep.name);
    }
    let src = project.src();
    // [platform-root] The project's platform roots, as `salvo compile` sets them.
    sources.platform = project.platform_roots();
    let errors = sources.add_dir(&src, ext, false, false);
    assert!(errors.is_empty(), "failed to read {}: {errors:?}", src.display());
    std_front(ext).program(sources.files, sources.companions)
}

/// `files` checked over the `"kt"` front: parsed, expanded, resolved,
/// checked, and through the IR analyses, as the language server does.
pub fn check(files: &[(&str, &str)]) -> Checked {
    check_built(program("kt", files))
}

/// A program built over the `"kt"` front, checked as [`check`] does.
pub fn check_built(built: FrontProgram) -> Checked {
    let FrontProgram { program, diagnostics, .. } = built;
    let front = std_front("kt");
    let symbols = Symbols::collect(&program);
    let resolution = salvo_core::resolve(&program);
    let checked = front.check(&program, &resolution, &symbols);
    let (ir, _decision) = salvo_core::ir::analyze::analyze_full(&program, &symbols, &resolution, &checked, &diagnostics);
    drop((symbols, resolution));
    Checked { program, checked, parse: diagnostics, ir }
}

/// [`check`] of one file, `main.sv`.
pub fn check_main(src: &str) -> Checked {
    check(&[("main.sv", src)])
}

/// `salvo ir`'s dump of `program` (`ir::dump_program`), checked over std's
/// front for `ext`.
pub fn dump_ir(ext: &str, program: &Program, all: bool, module: Option<&str>) -> Result<(String, Vec<String>), Vec<String>> {
    salvo_core::ir::dump_program_over(program, all, module, Some(std_front(ext).cache()))
}
