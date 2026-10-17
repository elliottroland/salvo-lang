//! [effect-alias] Effect-group aliases: `effects IO = [Console, Fs]` is
//! shorthand for its member list anywhere an effect list appears. Expanded
//! away before resolution, the same way prerequisites are (user decision
//! 2026-10-14): the checker and both emitters see an ordinary program with no
//! aliases in it.

use std::path::Path;

use salvo_core::{check_program, resolve, FileDiagnostic, Program, SourceSet, Symbols};

/// The base types the checker needs, as a std file [intrinsic-std-only].
const STD_PRELUDE: &str = "export intrinsic type Int\nexport intrinsic type Str\nexport intrinsic type Bool\n\
                           export intrinsic type Long\nexport intrinsic type Any\nexport intrinsic type Never\n\
                           export intrinsic type Addr<E>\nexport struct Mailbox { capacity: Int }\n";

/// Parses, expands, resolves and checks; every diagnostic, both severities.
fn diags(files: &[(&str, &str)]) -> Vec<FileDiagnostic> {
    let mut sources = SourceSet::default();
    sources.add(
        "std/core/prelude.sv",
        SourceSet::classify(Path::new("core/prelude.sv")).unwrap(),
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
        modules,
        companions: Vec::new(),
    };
    let resolution = resolve(&program);
    let symbols = Symbols::collect(&program);
    out.extend(resolution.errors.clone());
    out.extend(check_program(&program, &resolution, &symbols).errors);
    out
}

fn errors(files: &[(&str, &str)]) -> Vec<String> {
    diags(files)
        .iter()
        .filter(|d| d.is_error())
        .map(|d| d.message.clone())
        .collect()
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

// [effect-alias] An alias in a fn's effect list is its member list: a fn
// declaring `[App]` may call members of every effect in `App`, and nothing
// else. A transitive alias (`Big = [App, Clock]`) closes to the same set,
// deduped.
#[test]
fn an_alias_is_its_member_list_in_a_fn() {
    let src = &format!(
        "{CLOCK}\
effects App = [Console, Clock]
effects Big = [App, Clock]

fn a() [App] -> None {{
    print(\"x\")
    let _ = now()
}}

fn b() [Big] -> None {{
    print(\"y\")
    let _ = now()
}}

fn main() [use] {{
    use Out()
    use Ticking()
    a()
    b()
}}
"
    );
    let errs = errors(&[("main.sv", src)]);
    assert!(errs.is_empty(), "{errs:?}");
}

// [effect-alias] A fn that declares `[App]` may not perform an effect App does
// not stand for — expansion adds nothing extra.
#[test]
fn an_alias_grants_only_its_members() {
    let src = &format!(
        "{CLOCK}\
effect Secret {{
    fn leak() -> None
}}
effects App = [Console]

fn a() [App] -> None {{
    leak()
}}
"
    );
    let errs = errors(&[("main.sv", src)]);
    assert!(!errs.is_empty(), "calling an effect outside the alias must fail");
}

// [effect-alias] An alias works in a handler's dependency list and in an
// effect's prerequisite list, not only in a fn's.
#[test]
fn an_alias_expands_in_handler_deps_and_prereqs() {
    let src = &format!(
        "{CLOCK}\
effects App = [Console, Clock]

effect Logger {{
    fn log(m: Str) -> None => m
}}
handler StampLog() [App] of Logger {{
    fn log(m: Str) -> None => m {{
        print(\"${{now()}} ${{m}}\")
    }}
}}

effect Reporter [App] {{
    fn report(w: Str) -> None => w
}}
handler Rep() of Reporter {{
    fn report(w: Str) -> None => w {{
        print(\"${{now()}} ${{w}}\")
    }}
}}

fn do_log(m: Str) [Logger] -> None => m {{ log(m) }}
fn do_rep(w: Str) [Reporter] -> None => w {{ report(w) }}

fn main() [use] {{
    use Out()
    use Ticking()
    use StampLog()
    do_log(\"a\")
    use Rep()
    do_rep(\"b\")
}}
"
    );
    let errs = errors(&[("main.sv", src)]);
    assert!(errs.is_empty(), "{errs:?}");
}

// [effect-alias] An exported alias is usable from another module: `import
// caps.App` brings the alias, and expansion synthesizes imports for whichever
// members the consuming file cannot already name (a `core.*` effect needs
// none; a local effect of the alias's module does).
#[test]
fn an_alias_crosses_modules() {
    let caps = "\
export effect Clock {
    fn now() -> Int
}
export handler Ticking() of Clock {
    state { tick: Int = 0 }
    fn now() -> Int {
        tick = tick + 1
        return tick
    }
}
export effect Console {
    fn print(message: Str) -> None => message
}
export handler Out() of Console {
    fn print(message: Str) -> None => message {}
}
export effects App = [Console, Clock]
";
    let main = "\
import caps.App
import caps.Ticking
import caps.Out

fn report(label: Str) [App] -> None => label {
    print(label)
    let _ = now()
}

fn main() [use] {
    use Out()
    use Ticking()
    report(\"x\")
}
";
    let errs = errors(&[("caps.sv", caps), ("main.sv", main)]);
    assert!(errs.is_empty(), "{errs:?}");
}

// [effect-alias] The declaration's own rules: no cycle, and only plain named
// effects as members — `use`, `spawn` and `any` are refused where the alias
// is declared.
#[test]
fn a_malformed_alias_is_refused() {
    let cycle = &format!("{CLOCK}effects A = [B, Console]\neffects B = [A]\nfn f() [A] -> None {{ print(\"x\") }}\n");
    let errs = errors(&[("main.sv", cycle)]);
    assert!(errs.iter().any(|e| e.contains("cyclic") && e.contains("[effect-alias]")), "{errs:?}");

    let kinds = &format!("{CLOCK}effects A = [Console, use]\n");
    let errs = errors(&[("main.sv", kinds)]);
    assert!(
        errs.iter().any(|e| e.contains("`use` and `spawn`") && e.contains("[effect-alias]")),
        "{errs:?}"
    );

    let any = &format!("{CLOCK}effects A = [any Console]\n");
    let errs = errors(&[("main.sv", any)]);
    assert!(
        errs.iter().any(|e| e.contains("`any` has no meaning in an effect alias")),
        "{errs:?}"
    );
}

// [effect-alias] `any Alias` in a use-site list is refused: an alias is several
// effects, and `any` is about one instance of one effect.
#[test]
fn any_on_an_alias_is_refused() {
    let src = &format!("{CLOCK}effects App = [Console, Clock]\nfn f() [any App] -> None {{ print(\"x\") }}\n");
    let errs = errors(&[("main.sv", src)]);
    assert!(
        errs.iter().any(|e| e.contains("is an effect alias") && e.contains("[effect-alias]")),
        "{errs:?}"
    );
}
