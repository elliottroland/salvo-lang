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
const STD_PRELUDE: &str = "export intrinsic type Int\nexport intrinsic type Str\nexport intrinsic type Bool\n";

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
         handler AuditLogger [Telemetry] of Logger {\n    \
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

/// [effect-member-unique] [effect-member-overload] Two members of one effect
/// may share a name as an *overload* (user decision 2026-09-14) — what stays
/// an error is the same name taking the same types, which no call could tell
/// apart. A platform effect is an ordinary effect here too.
#[test]
fn duplicate_member_signatures_in_one_effect_are_rejected() {
    let errs = messages(&src(
        "platform effect Store {\n    \
             fn put(value: Int) [] -> None\n    \
             fn put(value: Int) [] -> Int\n}\n",
    ));
    assert!(
        errs.iter().any(|m| m.contains(
            "effect `Store` already declares a member named `put` with these \
             parameter types"
        )),
        "got {errs:?}"
    );
}

/// [effect-member-overload] The overload itself is legal: different parameter
/// types, one name.
#[test]
fn member_overloads_in_one_effect_are_legal() {
    let errs = messages(&src(
        "platform effect Store {\n    \
             fn put(value: Int) [] -> None\n    \
             fn put(value: Str) [] -> None => value\n}\n",
    ));
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

/// [effect-member-overload] Two *different* effects may share a member
/// name (user decision 2026-09-14, lifting the 2026-09-05 program-wide
/// ban): a call disambiguates by which effect has a handler in scope, or
/// explicitly with `member@Effect(…)` [effect-at]. The declaration itself
/// is legal.
#[test]
fn a_member_name_shared_by_two_effects_is_legal() {
    let errs = messages(&src(
        "platform effect A {\n    fn ping() [] -> None\n}\n\n\
         effect B {\n    fn ping() [] -> None\n}\n",
    ));
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

/// [effect-member-unique] The *within-one-effect* duplicate stays an
/// error, reported exactly once, at the second declaration.
#[test]
fn a_duplicate_member_is_reported_once() {
    let errs: Vec<String> = messages(&src(
        "platform effect A {\n    fn ping() [] -> None\n    fn ping() [] -> Int\n}\n\n\
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

// ===== platform handlers [platform-handler] =====

/// [platform-handler] The shape: an *ordinary* effect, a handler the host
/// implements, registered with `use` like any handler. Nothing else about
/// either declaration changes — which is the point of the form (FS-1
/// resolved as O-M2, user decision 2026-09-14).
#[test]
fn a_platform_handler_is_registered_with_use_like_any_handler() {
    let errs = messages(&src(
        "effect RawFs {\n    fn raw_close(handle: Int) [] -> Bool => handle\n}\n\n\
         platform handler HostRawFs of RawFs\n\n\
         fn shut(handle: Int) [RawFs] -> Bool {\n    return raw_close(handle)\n}\n\n\
         fn main() [use] -> None {\n    use HostRawFs()\n    shut(1)\n}\n",
    ));
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

/// [platform-handler] Constructor parameters are the host class's, passed
/// through by the `use` site: `use HostS3("bucket")` is how a host
/// implementation is configured.
#[test]
fn a_platform_handler_takes_constructor_arguments() {
    let errs = messages(&src(
        "effect Store {\n    fn put(key: Str) [] -> None => key\n}\n\n\
         platform handler HostS3(bucket: Str) of Store\n\n\
         fn main() [use] -> None {\n    use HostS3(\"salvo\")\n    put(\"k\")\n}\n",
    ));
    assert!(errs.is_empty(), "expected no errors, got {errs:?}");
}

/// [platform-handler] [intrinsic-std-only] Unlike `intrinsic`, `platform` is
/// the *customer's* modifier: the implementation is a file in their source
/// tree, not a table inside the compiler, so a program may declare one.
#[test]
fn a_platform_handler_is_not_std_only() {
    let errs = messages(&src(
        "effect Clock {\n    fn now() [] -> Int\n}\n\n\
         platform handler HostClock of Clock\n",
    ));
    assert!(
        !errs.iter().any(|m| m.contains("is the compiler's to declare")),
        "got {errs:?}"
    );
}

/// [platform-handler] The members are the host's, in the target language:
/// there is nothing for a Salvo body to mean, and the diagnostic names both
/// ways out (write the host file, or drop `platform`).
#[test]
fn a_platform_handler_with_a_body_is_rejected() {
    let errs = messages(&src(
        "effect Clock {\n    fn now() [] -> Int\n}\n\n\
         platform handler HostClock of Clock {\n    \
             fn now() -> Int {\n        return 0\n    }\n}\n",
    ));
    assert!(
        errs.iter().any(|m| m
            .contains("`platform handler HostClock` has no body in Salvo")
            && m.contains("`platform/` companion")
            && m.contains("salvo platform generate")),
        "got {errs:?}"
    );
}

/// [platform-handler] [effect-handler] State is a body too: a host class
/// hangs its own state on itself, in its own language.
#[test]
fn a_platform_handler_with_state_is_rejected() {
    let errs = messages(&src(
        "effect Clock {\n    fn now() [] -> Int\n}\n\n\
         platform handler HostClock of Clock {\n    ticks: Int = 0\n}\n",
    ));
    assert!(
        errs.iter()
            .any(|m| m.contains("`platform handler HostClock` has no body in Salvo")),
        "got {errs:?}"
    );
}

/// [platform-handler] [effect-handler-deps] A dependency is supplied *to a
/// handler's members*, and these members are host code, which performs no
/// Salvo effect. The remedy is a Salvo handler in between — which is exactly
/// what phase 4's `DefaultFs [RawFs] of Fs` is.
#[test]
fn a_platform_handler_with_effect_dependencies_is_rejected() {
    let errs = messages(&src(
        "effect Logger {\n    fn log(message: Str) -> None => message\n}\n\n\
         effect Clock {\n    fn now() [] -> Int\n}\n\n\
         platform handler HostClock [Logger] of Clock\n",
    ));
    assert!(
        errs.iter().any(|m| m
            .contains("`platform handler HostClock` may not declare effect dependencies")
            && m.contains("performs no Salvo effect")),
        "got {errs:?}"
    );
}

/// [platform-handler] Generic-free for the reason a `platform effect` is:
/// the host writes one concrete class, and a `use` site has no instance per
/// type argument to construct [backend-never-wrong].
#[test]
fn a_generic_platform_handler_is_rejected() {
    let errs = messages(&src(
        "effect Store<T> {\n    fn put(value: T) [] -> None => !value\n}\n\n\
         platform handler HostStore<T> of Store<T>\n",
    ));
    assert!(
        errs.iter()
            .any(|m| m.contains("`platform handler HostStore` may not be generic")),
        "got {errs:?}"
    );
}

/// [platform-handler] [platform-effect] A platform handler of a *platform
/// effect* is the one combination that means nothing: the host already
/// implements the whole effect and hands the instance to the entry point, so
/// there is no handler to register. The existing diagnostic covers it.
#[test]
fn a_platform_handler_of_a_platform_effect_is_rejected() {
    let errs = messages(&src(
        "platform effect Telemetry {\n    fn record(name: Str) [] -> None => name\n}\n\n\
         platform handler HostTelemetry of Telemetry\n",
    ));
    assert!(
        errs.iter().any(|m| m.contains("is a platform effect")
            && m.contains("has no Salvo handler")),
        "got {errs:?}"
    );
}
