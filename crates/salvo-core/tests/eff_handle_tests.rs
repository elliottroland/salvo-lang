//! [eff-handle] First-class effect handles: `eff[E]` as a value type, `use H()`
//! as an expression that registers a handler and yields its handle, a fn that
//! returns/accepts a handle, and set-subsumption (`eff[A, B]` where `eff[A]` is
//! wanted). User decisions 2026-10-18.

use std::path::Path;

use salvo_core::{check_program, resolve, FileDiagnostic, Program, SourceSet, Symbols};

const STD_PRELUDE: &str = "export intrinsic type Int\nexport intrinsic type Str\nexport intrinsic type Bool\n\
                           export intrinsic type Long\nexport intrinsic type Any\nexport intrinsic type Never\n\
                           export intrinsic type Addr<E>\nexport struct Mailbox { capacity: Int }\n";

fn diags(src: &str) -> Vec<FileDiagnostic> {
    let mut sources = SourceSet::default();
    sources.add(
        "std/core/prelude.sv",
        SourceSet::classify(Path::new("core/prelude.sv")).unwrap(),
        STD_PRELUDE.to_string(),
        true,
    );
    sources.add("main.sv", SourceSet::classify(Path::new("main.sv")).unwrap(), src.to_string(), false);
    let mut modules = Vec::new();
    for file in &sources.files {
        let (module, d) = salvo_syntax::parse_module(&file.content);
        let errs: Vec<_> = d.iter().filter(|x| x.is_error()).collect();
        assert!(errs.is_empty(), "parse errors in {}: {errs:?}", file.name);
        modules.push(module);
    }
    let expansion = salvo_core::expand(&sources.files, &mut modules);
    let mut out: Vec<FileDiagnostic> = expansion.diagnostics;
    let program = Program { files: sources.files, modules, companions: Vec::new() };
    let resolution = resolve(&program);
    let symbols = Symbols::collect(&program);
    out.extend(resolution.errors.clone());
    out.extend(check_program(&program, &resolution, &symbols).errors);
    out
}

fn errors(src: &str) -> Vec<String> {
    diags(src).iter().filter(|d| d.is_error()).map(|d| d.message.clone()).collect()
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
";

// [eff-handle] `use H()` is an expression: `let c = use H()` registers the
// handler and binds its handle. A fn may return the handle (`-> eff[Clock]`)
// and accept one (`clock: eff[Clock]`), and `use clock` binds a handed-in
// handle.
#[test]
fn use_is_an_expression_yielding_a_handle() {
    let src = &format!(
        "{CLOCK}\
fn make() [use] -> eff[Clock] {{
    let c = use Ticking()
    return c
}}

fn read(clock: eff[Clock]) [use] -> Int => clock {{
    use clock
    return now()
}}

fn main() [use] {{
    let c = make()
    let _n = read(c)
}}
"
    );
    let errs = errors(src);
    assert!(errs.is_empty(), "{errs:?}");
}

// [eff-handle] A bare statement `use H()` still binds the effect in scope and
// yields no binding — the discarded-value case.
#[test]
fn use_as_a_statement_still_binds_the_effect() {
    let src = &format!(
        "{CLOCK}\
fn main() [use] {{
    use Ticking()
    let _n = now()
}}
"
    );
    let errs = errors(src);
    assert!(errs.is_empty(), "{errs:?}");
}

// [eff-handle] Set-subsumption: a wider handle satisfies a narrower parameter.
// `eff[Clock, Beep]` passes where `eff[Clock]` is wanted (checked at the type
// level — multi-face *binding* to a value is a separate, deferred case).
#[test]
fn a_wider_handle_satisfies_a_narrower_parameter() {
    let src = &format!(
        "{CLOCK}\
fn read(clock: eff[Clock]) [use] -> Int => clock {{
    use clock
    return now()
}}

fn forward(wide: eff[Clock, Clock]) [use] -> Int => wide {{
    return read(wide)
}}

fn main() [use] {{
    let c = use Ticking()
    let _n = read(c)
}}
"
    );
    // `eff[Clock, Clock]` normalises to `eff[Clock]`, and `read(wide)` must
    // type-check under subsumption without an overload error.
    let errs = errors(src);
    assert!(
        !errs.iter().any(|m| m.contains("no matching overload")),
        "subsumption should let a wider handle fit a narrower parameter: {errs:?}"
    );
}

// [eff-handle] A multi-face handler's handle as a single value is refused for
// now (its flat representation lands with projection), named clearly rather
// than emitted wrong [backend-never-wrong].
#[test]
fn a_multi_face_handle_value_is_refused_for_now() {
    let src = "\
effect A { fn a() -> Int }
effect B { fn b() -> Int }
handler Both() of A, B {
    fn a() -> Int { return 1 }
    fn b() -> Int { return 2 }
}
fn main() [use] {
    let _both = use Both()
}
";
    let errs = errors(src);
    assert!(
        errs.iter().any(|m| m.contains("multi-face") && m.contains("[eff-handle]")),
        "a multi-face handle value should be refused by name: {errs:?}"
    );
}
