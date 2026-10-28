//! [effect-alias] Effect-group aliases: `effects IO = [Console, Fs]` is
//! shorthand for its member list anywhere an effect list appears. Expanded
//! away before resolution, the same way prerequisites are (user decision
//! 2026-10-14): the checker and both emitters see an ordinary program with no
//! aliases in it.
//!
//! The rest of these tests are test data, `tests/check/aliases/` (TESTS.md T1).

use std::path::Path;

use salvo_core::{check_program, resolve, FileDiagnostic, Program, SourceSet, Symbols};

/// The base types the checker needs, as a std file [platform-intrinsic-std-only].
const STD_PRELUDE: &str = concat!(
    "export intrinsic type Int\nexport intrinsic type Str\nexport intrinsic type Bool\n\
                           export intrinsic type Long\nexport intrinsic type Any\nexport intrinsic type Never\n\
                           export intrinsic type Addr<E>\nexport struct Mailbox { capacity: Int }\n",
    // [op-equality] [op-order] What the comparison operators call.
    "export intrinsic fn eq(a: Int, b: Int) [] -> Bool => a, b\n",
    "export intrinsic fn eq(a: Long, b: Long) [] -> Bool => a, b\n",
    "export intrinsic fn eq(a: Bool, b: Bool) [] -> Bool => a, b\n",
    "export intrinsic fn eq(a: Str, b: Str) [] -> Bool => a, b\n",
    "export intrinsic fn cmp(a: Int, b: Int) [] -> Int => a, b\n",
    "export intrinsic fn cmp(a: Long, b: Long) [] -> Int => a, b\n",
    // [op-arith] [op-pow] What the arithmetic operators call.
    "export intrinsic fn plus(a: Int, b: Int) [] -> Int => a, b\n",
    "export intrinsic fn plus(a: Long, b: Long) [] -> Long => a, b\n",
    "export intrinsic fn minus(a: Int, b: Int) [] -> Int => a, b\n",
    "export intrinsic fn minus(a: Long, b: Long) [] -> Long => a, b\n",
    "export intrinsic fn times(a: Int, b: Int) [] -> Int => a, b\n",
    "export intrinsic fn times(a: Long, b: Long) [] -> Long => a, b\n",
    "export intrinsic fn divide(a: Int, b: Int) [] -> Int => a, b\n",
    "export intrinsic fn divide(a: Long, b: Long) [] -> Long => a, b\n",
);

/// Parses, expands, resolves and checks; every diagnostic, both severities.
fn diags(files: &[(&str, &str)]) -> Vec<FileDiagnostic> {
    let mut sources = SourceSet::default();
    sources.add(
        "std/core/prelude.sv",
        SourceSet::classify(Path::new("std/core/prelude.sv")).unwrap(),
        STD_PRELUDE.to_string(),
        true,
    );
    for (name, src) in files {
        let module = SourceSet::classify(Path::new(name)).unwrap();
        sources.add(*name, module, src.to_string(), false);
    }
    let mut modules = Vec::new();
    for file in &sources.files {
        let (module, diagnostics) = salvo_syntax::parse_module(&file.content);
        let errors: Vec<_> = diagnostics.iter().filter(|d| d.is_error()).collect();
        assert!(errors.is_empty(), "parse errors in {}: {errors:?}", file.name);
        modules.push(module);
    }
    let expansion = salvo_core::expand(&sources.files, &mut modules);
    let mut out: Vec<FileDiagnostic> = expansion.diagnostics;
    let program = Program {
        files: sources.files,
        modules: Program::share(modules),
        companions: Vec::new(),
    };
    let resolution = resolve(&program);
    let symbols = Symbols::collect(&program);
    out.extend(resolution.errors.clone());
    out.extend(check_program(&program, &resolution, &symbols).errors);
    out
}

const CLOCK: &str = "\
effect Clock {
    fn now() -> Int
}
handler Ticking() of Clock {
    state { tick: Int = 0 }
    fn now() -> Int {
        tick = tick + 1
        return tick
    }
}
effect Console {
    fn print(message: Str) -> None => message
}
handler Out() of Console {
    fn print(message: Str) -> None => message {}
}
";

// [effect-alias] The declaration's own rules: no cycle, and only plain named
// effects as members — `use`, `spawn` and `any` are refused where the alias
// is declared.
//
// Kept over its prelude, not moved to `check/aliases/`: checked exactly, the
// cycle is also reported at the `[A]` that uses it (ROADMAP §0ar).
#[test]
fn a_malformed_alias_is_refused() {
    let cycle = &format!("{CLOCK}effects A = [B, Console]\neffects B = [A]\nfn f() [A] -> None {{ print(\"x\") }}\n");
    let errs = diags(&[("main.sv", cycle)]);
    assert!(
        errs.iter().any(|d| d.id == Some("effect-alias:cycle")),
        "{errs:?}"
    );

    let kinds = &format!("{CLOCK}effects A = [Console, use]\n");
    let errs = diags(&[("main.sv", kinds)]);
    assert!(
        errs.iter().any(|d| d.id == Some("effect-alias:capability")),
        "{errs:?}"
    );

    let any = &format!("{CLOCK}effects A = [any Console]\n");
    let errs = diags(&[("main.sv", any)]);
    assert!(
        errs.iter().any(|d| d.is_error() && d.id == Some("effect-alias:any")),
        "{errs:?}"
    );
}

// [effect-alias] `any Alias` in a use-site list is refused: an alias is several
// effects, and `any` is about one instance of one effect.
//
// Kept over its prelude, not moved to `check/aliases/`: checked exactly, the
// refused `any App` also reports `print` as having no `Console` handler.
#[test]
fn any_on_an_alias_is_refused() {
    let src = &format!("{CLOCK}effects App = [Console, Clock]\nfn f() [any App] -> None {{ print(\"x\") }}\n");
    let errs = diags(&[("main.sv", src)]);
    assert!(
        errs.iter().any(|d| d.id == Some("effect-alias:any-alias")),
        "{errs:?}"
    );
}
