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

// [eff-handle] A tuple handle's elements are ordinary single handles: you
// project one out by destructuring and pass it where that single handle is
// wanted (the narrow-from-wide direction, done by projection rather than
// subsumption).
#[test]
fn a_single_handle_projects_out_of_a_multi_effect_handle() {
    let src = &format!(
        "{CLOCK}\
effect Beep {{ fn beep() -> Int }}
handler Both() of Clock, Beep {{
    state {{ n: Int = 0 }}
    fn now() -> Int {{
        n = n + 1
        return n
    }}
    fn beep() -> Int {{ return 2 }}
}}

fn read(clock: eff[Clock]) [use] -> Int => clock {{
    use clock
    return now()
}}

fn main() [use] {{
    let (c, _b) = use Both()
    let _n = read(c)
}}
"
    );
    let errs = errors(src);
    assert!(
        !errs.iter().any(|m| m.contains("no matching overload")),
        "the projected single handle should fit the narrower parameter: {errs:?}"
    );
}

// [eff-handle] A multi-effect handle is a *tuple* of single handles. A
// multi-face `use H()` as a value yields `(eff[A], eff[B])`, which destructures
// into the individual handles; `use e` over the whole tuple binds every effect.
#[test]
fn a_multi_face_handle_is_a_tuple_of_single_handles() {
    let src = "\
effect A { fn a() -> Int }
effect B { fn b() -> Int }
handler Both() of A, B {
    state { n: Int = 0 }
    fn a() -> Int {
        n = n + 1
        return n
    }
    fn b() -> Int { return 2 }
}
fn make() [use] -> eff[A, B] {
    return use Both()
}
fn main() [use] {
    let both = make()
    use both
    let _x = a()
    let _y = b()
}
";
    let errs = errors(src);
    assert!(errs.is_empty(), "a multi-effect handle should be a usable tuple: {errs:?}");
}

// [eff-handle] Projection `e[A]` / `e[A, B]` / `e[alias]`: a non-consuming copy
// of the sub-handle carrying the named effects, a subset of what `e` carries.
// Yields a single handle for one effect, a tuple for several; an alias expands.
#[test]
fn a_handle_projects_a_subset() {
    let src = "\
effect A { fn a() -> Int }
effect B { fn b() -> Int }
effect C { fn c() -> Int }
handler All() of A, B, C {
    fn a() -> Int { return 1 }
    fn b() -> Int { return 2 }
    fn c() -> Int { return 3 }
}
effects AB = [A, B]
fn use_a(h: eff[A]) [use] -> Int => h {
    use h
    return a()
}
fn main() [use] {
    let all = use All()
    let _n = use_a(all[A])     // project one, pass it on
    let ab = all[AB]           // project via alias -> (eff[A], eff[B])
    use ab
    let _x = a()
    let _y = b()
    let (_hc, _hb) = all[C, B] // reordered subset
}
";
    let errs = errors(src);
    assert!(errs.is_empty(), "projection should type-check: {errs:?}");
}

// [eff-handle] Projecting an effect the handle does not carry is refused.
#[test]
fn projecting_an_absent_effect_is_refused() {
    let src = "\
effect A { fn a() -> Int }
effect B { fn b() -> Int }
handler OnlyA() of A {
    fn a() -> Int { return 1 }
}
fn main() [use] {
    let h = use OnlyA()
    let _bad = h[B]
}
";
    let errs = errors(src);
    assert!(
        errs.iter().any(|m| m.contains("does not carry effect `B`") && m.contains("[eff-handle]")),
        "projecting an absent effect should be refused: {errs:?}"
    );
}

// [eff-handle] Merge `eff[a, b]`: combine already-bound handles into one
// handle carrying the union of their effects, which `use` then binds whole.
#[test]
fn merge_combines_bound_handles() {
    let src = "\
effect A { fn a() -> Int }
effect B { fn b() -> Int }
handler HA() of A { fn a() -> Int { return 1 } }
handler HB() of B { fn b() -> Int { return 2 } }
fn run(caps: eff[A, B]) [use] -> Int => caps {
    use caps
    return a() + b()
}
fn main() [use] {
    let ha = use HA()
    let hb = use HB()
    let _n = run(eff[ha, hb])
}
";
    let errs = errors(src);
    assert!(errs.is_empty(), "merge should combine handles: {errs:?}");
}

// [eff-handle] `eff[*]` bundles every effect handle in scope, ordered to the
// expected type so it fits a declared return.
#[test]
fn star_merge_bundles_every_handle_in_scope() {
    let src = "\
effect A { fn a() -> Int }
effect B { fn b() -> Int }
handler HA() of A { fn a() -> Int { return 1 } }
handler HB() of B { fn b() -> Int { return 2 } }
fn boot() [use] -> eff[A, B] {
    let _ha = use HA()
    let _hb = use HB()
    return eff[*]
}
fn main() [use] {
    let _caps = boot()
}
";
    let errs = errors(src);
    assert!(errs.is_empty(), "eff[*] should bundle in-scope handles: {errs:?}");
}

// [eff-handle] Merging a non-handle is refused.
#[test]
fn merging_a_non_handle_is_refused() {
    let src = "\
effect A { fn a() -> Int }
handler HA() of A { fn a() -> Int { return 1 } }
fn main() [use] {
    let ha = use HA()
    let n = 3
    let _bad = eff[ha, n]
}
";
    let errs = errors(src);
    assert!(
        errs.iter().any(|m| m.contains("merges effect handles") && m.contains("[eff-handle]")),
        "merging a non-handle should be refused: {errs:?}"
    );
}
