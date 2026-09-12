//! [platform-effect] A `platform effect` is an effect whose members the
//! *host* implements in the target language. The compiler generates the
//! interface; the instance arrives from outside the Salvo program, handed
//! to the entry point by the host's own `main`. Everything else about it is
//! an ordinary effect — which is the point: the existing interface/trait
//! emission, `&mut dyn` threading and handler-dependency machinery all
//! apply unchanged.
//!
//! What it adds is a small set of restrictions, each because the
//! implementation is not Salvo's (user decisions 2026-09-05).

use std::path::Path;

use salvo_core::{check_program, resolve, FileDiagnostic, Program, SourceSet, Symbols};

/// [intrinsic-std-only] The intrinsic declarations these sources rely on.
/// `intrinsic` is the compiler's modifier and only std may write it, so this
/// is loaded as a *std* file rather than pasted into the source under test.
/// Module `core.prelude`: `core.*` is implicitly imported, so the test source
/// sees these names without an `import`.
const STD_PRELUDE: &str = "intrinsic type Int\nintrinsic type Str\nintrinsic type Bool\n";

fn check_errors(src: &str) -> Vec<FileDiagnostic> {
    let mut sources = SourceSet::default();
    sources.add(
        "std/core/prelude.sv",
        SourceSet::classify(Path::new("core/prelude.sv")).unwrap(),
        STD_PRELUDE.to_string(),
        true,
    );
    sources.add(
        "main.sv",
        SourceSet::classify(Path::new("main.sv")).unwrap(),
        src.to_string(),
        false,
    );
    let mut modules = Vec::with_capacity(sources.files.len());
    for file in &sources.files {
        let (ast, diagnostics) = salvo_syntax::parse_module(&file.content);
        let parse_errors: Vec<_> = diagnostics.iter().filter(|d| d.is_error()).collect();
        assert!(
            parse_errors.is_empty(),
            "parse errors in {}: {parse_errors:?}",
            file.name
        );
        modules.push(ast);
    }
    let program = Program {
        files: sources.files,
        modules,
        companions: Vec::new(),
    };
    let symbols = Symbols::collect(&program);
    let resolution = resolve(&program);
    let mut errors: Vec<FileDiagnostic> = resolution.errors.clone();
    errors.extend(check_program(&program, &resolution, &symbols).errors);
    errors
}

fn messages(src: &str) -> Vec<String> {
    check_errors(src).iter().map(|d| d.message.clone()).collect()
}

/// A std-less prelude: these tests never emit, so the base types only have
/// to exist.
const PRELUDE: &str = "\
";

fn src(body: &str) -> String {
    format!("{PRELUDE}\n{body}")
}

// ===== the shape works =====

/// [platform-effect] The baseline: a platform effect is performed like any
/// other effect, declared up the call chain, and needs no handler anywhere.
#[test]
fn a_platform_effect_is_performed_like_any_other() {
    let errs = messages(&src(
        "platform effect Telemetry {\n    \
             fn record(name: Str, value: Int) [] -> None => name, value\n}\n\n\
         fn work(n: Int) [Telemetry] -> Int {\n    \
             record(\"work\", n)\n    return n\n}\n\n\
         fn main() [Telemetry] {\n    work(1)\n}\n",
    ));
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

/// [platform-effect] [effect-handler-deps] A Salvo handler may *depend* on
/// a platform effect (user decision 2026-09-05): that is how a handler
/// written in Salvo reaches the host. The dependency is a constructor
/// parameter of effect type, exactly as for any other effect.
#[test]
fn a_handler_may_depend_on_a_platform_effect() {
    let errs = messages(&src(
        "platform effect Telemetry {\n    \
             fn record(name: Str) [] -> None => name\n}\n\n\
         effect Logger {\n    fn log(message: Str) -> None => message\n}\n\n\
         handler AuditLogger(telemetry: Telemetry) of Logger {\n    \
             fn log(message: Str) -> None => message {\n        \
                 record(message)\n    }\n}\n\n\
         fn main() [use, Telemetry] {\n    use AuditLogger()\n}\n",
    ));
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

// ===== restrictions =====

/// [platform-effect] The host's implementation *is* the handler, so a Salvo
/// one would be a second, unreachable implementation. The diagnostic names
/// the remedy: an ordinary `effect` is what you want if you meant to handle
/// it in Salvo.
#[test]
fn a_salvo_handler_for_a_platform_effect_is_rejected() {
    let errs = messages(&src(
        "platform effect Telemetry {\n    fn record(name: Str) [] -> None => name\n}\n\n\
         handler MyTelemetry of Telemetry {\n    \
             fn record(name: Str) [] -> None => name {\n    }\n}\n",
    ));
    assert!(
        errs.iter().any(|m| m.contains("is a platform effect")
            && m.contains("has no Salvo handler")
            && m.contains("ordinary `effect`")),
        "got {errs:?}"
    );
}

/// [platform-effect] A generic platform effect would need the host to
/// implement one interface per instantiation — Kotlin's facets exist for
/// that and Rust has no equivalent, so it is refused at the declaration
/// rather than at codegen [backend-never-wrong].
#[test]
fn a_generic_platform_effect_is_rejected() {
    let errs = messages(&src(
        "platform effect Store<T> {\n    fn put(value: T) [] -> None => !value\n}\n",
    ));
    assert!(
        errs.iter()
            .any(|m| m.contains("platform effect `Store` may not be generic")),
        "got {errs:?}"
    );
}

/// [platform-effect] [effect-member-generics] A generic *member* is already
/// a loud codegen error on the Rust backend; the host implements a concrete
/// signature, so it is rejected here too.
#[test]
fn a_generic_platform_member_is_rejected() {
    let errs = messages(&src(
        "platform effect Store {\n    fn put<T>(value: T) [] -> None => !value\n}\n",
    ));
    assert!(
        errs.iter().any(|m| m
            .contains("member `put` of platform effect `Store` may not be generic")),
        "got {errs:?}"
    );
}

/// [effect-member-unique] Two members of one effect cannot share a name:
/// overloading is not available through the interface, on either backend.
#[test]
fn duplicate_member_names_in_one_effect_are_rejected() {
    let errs = messages(&src(
        "platform effect Store {\n    \
             fn put(value: Int) [] -> None\n    \
             fn put(value: Str) [] -> None => !value\n}\n",
    ));
    assert!(
        errs.iter().any(|m| m
            == "effect `Store` already declares a member named `put`"),
        "got {errs:?}"
    );
}

/// [effect-member-unique] A member name identifies its effect program-wide,
/// so two *different* effects cannot share one — there is no syntax to say
/// which effect a call means. Previously this resolved to whichever effect
/// was collected last and surfaced as a baffling "no handler for effect"
/// (user decision 2026-09-05).
#[test]
fn a_member_name_shared_by_two_effects_is_rejected() {
    let errs = messages(&src(
        "platform effect A {\n    fn ping() [] -> None\n}\n\n\
         effect B {\n    fn ping() [] -> None\n}\n",
    ));
    assert!(
        errs.iter().any(|m| m.contains("effect `A` already declares a member named `ping`")
            && m.contains("no syntax to say which effect")),
        "got {errs:?}"
    );
}

/// [effect-member-unique] Reported exactly once per collision, at the
/// second declaration — the check walks source order rather than the
/// hash-ordered, last-wins `Symbols` maps.
#[test]
fn a_shared_member_name_is_reported_once() {
    let errs: Vec<String> = messages(&src(
        "platform effect A {\n    fn ping() [] -> None\n}\n\n\
         effect B {\n    fn ping() [] -> None\n}\n",
    ))
    .into_iter()
    .filter(|m| m.contains("already declares a member named `ping`"))
    .collect();
    assert_eq!(errs.len(), 1, "got {errs:?}");
}

/// [effect-member-unique] The rule is about *collisions*, not about effects
/// having members: distinct names across effects stay legal.
#[test]
fn distinct_member_names_across_effects_are_fine() {
    let errs = messages(&src(
        "platform effect A {\n    fn ping() [] -> None\n}\n\n\
         effect B {\n    fn pong() [] -> None\n}\n",
    ));
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}
