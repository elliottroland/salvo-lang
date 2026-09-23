//! The Kotlin code emitter.
//!
//! M2 scope: functions (with effects as leading parameters), structs (data
//! classes), effects (interfaces), handlers (classes/objects, including
//! `intrinsic handler` lowerings), intrinsic fn/type lowerings,
//! string interpolation, `if`/`is` with bindings, iterator functions
//! (`yield` -> Kotlin `iterator {}` builder), `while`/`for` statements.
//! M6 adds loops as values: `break value` and loop `else` lower through a
//! `run {}` block with a result local [while-value] [kt-loop-value].
//!
//! Not yet supported (reported as codegen errors, never silently wrong
//! code [backend-never-wrong]): tuples beyond Pair/Triple, multi-spread
//! struct literals.

use std::collections::{BTreeSet, HashMap, HashSet};

use salvo_core::check::{Checked, Coercion, PredicateCheck, UnionTest};
use salvo_core::types::{FnId, Ty};
use salvo_core::{ModulePath, Program, Symbols};
use salvo_syntax::ast::*;
use salvo_syntax::Span;

pub struct EmittedFile {
    /// Path relative to the target dir, e.g. `core/console.kt`.
    pub rel_path: std::path::PathBuf,
    pub content: String,
}

/// Emits Kotlin for every *reachable* module that produces code
/// [mod-used-only]. Returns the files or the accumulated codegen/type
/// errors, **dropping** any warnings: this is the shape the golden tests
/// want. The driver calls [`emit_program_reporting`], which hands them back
/// [qual-refn-conflict].
pub fn emit_program(program: &Program) -> Result<Vec<EmittedFile>, Vec<String>> {
    emit_program_reporting(program).map(|(files, _warnings)| files)
}

/// [`emit_program`] with the non-fatal diagnostics: `(files, warnings)`,
/// each warning rendered with its own `warning:` prefix and location
/// [diag-structured].
///
/// Two returns rather than one, because they mean different things: a
/// warning must not stop emission (a suppressed refinement conflict leaves
/// a legal program [qual-refn-conflict]), so it cannot travel as an error —
/// and it must not be silently swallowed either, or the diagnostic exists
/// only in `salvo analyze`.
pub fn emit_program_reporting(
    program: &Program,
) -> Result<(Vec<EmittedFile>, Vec<String>), Vec<String>> {
    let symbols = Symbols::collect(program);
    let resolution = salvo_core::resolve(program);
    let checked = salvo_core::check_program(program, &resolution, &symbols);
    // Only *errors* stop emission: a warning reports something the author
    // probably did not intend without rejecting the program
    // [diag-structured], which is what a suppressed refinement conflict
    // needs [qual-refn-conflict]. Checker diagnostics are structured;
    // render them here at the backend boundary.
    if checked.errors.iter().any(|d| d.is_error()) {
        // Every diagnostic is rendered on the failure path, warnings
        // included: each carries its own severity prefix, so they read
        // correctly next to the errors and nothing is lost while the author
        // fixes the errors.
        return Err(checked
            .errors
            .iter()
            .map(|d| d.render(&program.files))
            .collect());
    }
    let warnings: Vec<String> = checked
        .errors
        .iter()
        .filter(|d| !d.is_error())
        .map(|d| d.render(&program.files))
        .collect();
    // [mod-used-only] Only modules the program uses are transpiled.
    let reachable = salvo_core::reachable_modules(program, &resolution);
    // The modules that will exist as Kotlin files: targets for generated
    // imports [kt-package].
    let emitted_modules: HashSet<&ModulePath> = program
        .units()
        .filter(|u| {
            reachable.contains(&u.file.module)
                && module_produces_code(u.ast)
        })
        .map(|u| &u.file.module)
        .collect();

    let mut errors: Vec<String> = Vec::new();

    // [fn-effects] Where each effect's *interface* lives, as a
    // fully-qualified package prefix: the generated pass-interface file lives
    // in the root `salvo` package and mentions interfaces from arbitrary
    // module packages, so it names them in full rather than importing them.
    let mut effect_paths: HashMap<String, String> = HashMap::new();
    for unit in program.units() {
        if !emitted_modules.contains(&unit.file.module) {
            continue;
        }
        let prefix = format!("{}.", kotlin_package(&unit.file.module));
        for item in &unit.ast.items {
            if let Item::Effect(e) = item {
                effect_paths.insert(e.name.name.clone(), prefix.clone());
            }
        }
    }

    let mut files = Vec::new();
    let mut union_sizes: BTreeSet<usize> = BTreeSet::new();
    // [kt-tuple-class] The tuple arities past `Pair`/`Triple` the program
    // names, which `tuples.kt` declares a class for.
    let mut tuple_sizes: BTreeSet<usize> = BTreeSet::new();
    // [kt-throw-signal] Generated once for the whole program, when
    // anything throws.
    let mut needs_throw = false;
    // [kt-ordered] And for the structural comparison an ordered
    // struct's `compareTo` uses [col-hashed-ordered].
    let mut needs_compare = false;
    let mut needs_keyed = false;
    // [kt-bytes] And for the byte buffer, whenever a `Bytes` is named
    // anywhere in the program [bytes-type].
    let mut needs_bytes = false;
    // [kt-actor] And for the scheduler, when a program spawns.
    let mut needs_scheduler = false;
    // [time-types] And for the two clock readings, when a program reads time.
    let mut needs_time = false;
    // [kt-effect-fusion] The fusion switch is program-wide: a fn's
    // signature cannot depend on which of its callers happens to hold a
    // fusion, so either every effect site fuses or none does. Same gate as
    // [rs-effect-fusion].
    let fusion = program_needs_fusion(&symbols, &reachable);
    // [kt-effect-fusion] The Has-accessor interfaces the program uses,
    // merged across files into `fx.kt` (interface name → rendered effect
    // instance).
    let mut has_ifaces: std::collections::BTreeMap<String, String> =
        std::collections::BTreeMap::new();
    // [platform-handler] [platform-tree] Modules whose host companion a
    // `use` of a platform handler needs; checked once every file is emitted.
    let mut platform_hosts: BTreeSet<ModulePath> = BTreeSet::new();
    for (file_idx, unit) in program.units().enumerate() {
        if !reachable.contains(&unit.file.module) || !module_produces_code(unit.ast) {
            continue;
        }
        // Generated Kotlin imports: a wildcard per foreign emitted module
        // this file references, plus alias imports for aliased Salvo
        // imports of Kotlin-visible items [kt-imports].
        let mut emitter = Emitter::new(&symbols, &checked, program, file_idx, &unit.file.name);
        emitter.effect_paths = effect_paths.clone();
        emitter.fusion = fusion;
        let generated = emitter.generate_imports(
            unit.ast,
            &unit.file.module,
            &resolution.scopes[file_idx],
            &emitted_modules,
        );
        emitter.generated_imports = generated;
        let content = emitter.emit_module(unit.ast);
        errors.extend(emitter.errors);
        union_sizes.extend(emitter.union_sizes);
        tuple_sizes.extend(emitter.tuple_sizes);
        needs_throw |= emitter.needs_throw;
        needs_compare |= emitter.needs_compare;
        needs_keyed |= emitter.needs_keyed;
        needs_bytes |= emitter.needs_bytes;
        needs_scheduler |= emitter.needs_scheduler;
        needs_time |= emitter.needs_time;
        has_ifaces.extend(emitter.has_ifaces);
        platform_hosts.extend(emitter.platform_hosts);
        let mut rel_path = std::path::PathBuf::new();
        for part in &unit.file.module.0 {
            rel_path.push(part);
        }
        rel_path.set_extension("kt");
        files.push(EmittedFile { rel_path, content });
    }
    if !union_sizes.is_empty() {
        files.push(EmittedFile {
            rel_path: std::path::PathBuf::from("unions.kt"),
            content: generate_unions_file(&union_sizes),
        });
    }
    // [kt-tuple-class] The tuples Kotlin does not have, declared once for the
    // program exactly as the union wrappers are.
    if !tuple_sizes.is_empty() {
        files.push(EmittedFile {
            rel_path: std::path::PathBuf::from("tuples.kt"),
            content: generate_tuples_file(&tuple_sizes),
        });
    }
    // [kt-effect-fusion] The Has-accessor interfaces, once per program:
    // `fx.kt` in the root `salvo` package, wildcard-importing every emitted
    // module so effect and argument type names resolve as they do at the
    // use sites that named them.
    if !has_ifaces.is_empty() {
        let mut content = String::from(
            "// Generated by the Salvo compiler: effect accessor interfaces \
             [kt-effect-fusion].\npackage salvo\n\n",
        );
        let mut mods: Vec<String> = emitted_modules
            .iter()
            .map(|m| kotlin_package(m))
            .collect();
        mods.sort();
        for m in mods {
            content.push_str(&format!("import {m}.*\n"));
        }
        for (iface, rendered) in &has_ifaces {
            let prop = format!("__fx_{}", sanitize_instance(rendered));
            content.push_str(&format!(
                "\ninterface {iface} {{\n    val {prop}: {rendered}\n}}\n"
            ));
            // [with-clause] The one-instance adapter: a `with` clause's
            // **private instance** satisfies a handle-dep constructor
            // parameter through this, since that parameter is typed as the
            // accessor interface (stable per-effect identity) rather than as
            // the effect. One tiny class per effect, inert where unused.
            content.push_str(&format!(
                "\nclass __One_{}(private val __e: {rendered}) : {iface} {{\n    \
                 override val {prop}: {rendered} get() = __e\n}}\n",
                sanitize_instance(rendered)
            ));
        }
        files.push(EmittedFile {
            rel_path: std::path::PathBuf::from("fx.kt"),
            content,
        });
    }
    // [time-timer] The scheduler's deadline thread reads the monotonic clock,
    // so the time runtime travels with it: a `Fired` must sit on the same
    // timeline `tick()` reports, which is what one shared reading buys.
    let needs_time = needs_time || needs_scheduler;
    if needs_throw {
        files.push(EmittedFile {
            rel_path: std::path::PathBuf::from("throw.kt"),
            content: generate_throw_file(),
        });
    }
    if needs_compare {
        files.push(EmittedFile {
            rel_path: std::path::PathBuf::from("compare.kt"),
            content: generate_compare_file(),
        });
    }
    // [cmp-carry] Only where a collection's type *names* the hash and equality its
    // keys are kept by: everything else is still a `LinkedHashMap`.
    if needs_keyed {
        files.push(EmittedFile {
            rel_path: std::path::PathBuf::from("keyed.kt"),
            content: generate_keyed_file(),
        });
    }
    if needs_bytes {
        files.push(EmittedFile {
            rel_path: std::path::PathBuf::from("bytes.kt"),
            content: generate_bytes_file(),
        });
    }
    if needs_scheduler {
        files.push(EmittedFile {
            rel_path: std::path::PathBuf::from("scheduler.kt"),
            content: generate_scheduler_file(),
        });
    }
    if needs_time {
        files.push(EmittedFile {
            rel_path: std::path::PathBuf::from("hosttime.kt"),
            content: generate_time_file(),
        });
    }
    // [backend-companion] Backend-native companion files are copied
    // verbatim whenever their module is needed. A companion must not
    // collide with a generated file.
    for comp in &program.companions {
        if !reachable.contains(&comp.module) {
            continue;
        }
        if files.iter().any(|f| f.rel_path == comp.rel_path) {
            errors.push(format!(
                "companion file `{}` collides with the generated file of module \
                 `{}`: a companion cannot replace a module Salvo emits",
                comp.rel_path.display(),
                comp.module
            ));
            continue;
        }
        files.push(EmittedFile {
            rel_path: comp.rel_path.clone(),
            content: comp.content.clone(),
        });
    }
    // [platform-tree] A `main` that needs a platform effect is not the
    // program's entry point any more, so the host file must exist — and
    // the error has to name the command that creates it, or the failure
    // only surfaces as `kotlinc` not finding a `main`.
    for unit in program.units() {
        if unit.file.is_std
            || !reachable.contains(&unit.file.module)
            || salvo_core::platform_entry(unit.ast, &symbols).is_none()
        {
            continue;
        }
        if salvo_core::host_file(&program.companions, &unit.file.module).is_none() {
            errors.push(salvo_core::missing_host_error(
                &unit.file.module,
                &salvo_core::host_rel_path(&unit.file.module, "kt"),
            ));
        }
    }
    // [platform-handler] [platform-tree] A `use` of a platform handler
    // constructs a host class, so the companion that defines it must exist —
    // in std as much as in customer code, since std ships its own
    // `platform/` files.
    for module in &platform_hosts {
        if salvo_core::host_file(&program.companions, module).is_some() {
            continue;
        }
        let handlers: Vec<&str> = program
            .units()
            .filter(|u| u.file.module == *module)
            .flat_map(|u| salvo_core::platform_handlers(u.ast))
            .map(|h| h.name.name.as_str())
            .collect();
        errors.push(salvo_core::missing_handler_host_error(
            &handlers.join("`, `"),
            module,
            &salvo_core::host_rel_path(module, "kt"),
        ));
    }

    if errors.is_empty() {
        Ok((files, warnings))
    } else {
        Err(errors)
    }
}

/// [kt-effect-fusion] Does any handler in the program declare an effect
/// dependency (a constructor parameter of *bare* effect type,
/// [effect-handler-deps])? If so the whole program switches to the fusion
/// emission; otherwise effects thread as one handler parameter each
/// [kt-effect-params] and nothing fusion-related is emitted. The predicate
/// matches [rs-effect-fusion]'s gate exactly, so the two backends fuse the
/// same programs.
///
/// **Reachable handlers only** (2026-09-14): std ships one, `DefaultFs
/// [RawFs]` in `core.hostfs`, and a program that never names it must not pay
/// the fused emission — which is also why that handler is not in `core.fs`,
/// a module every iterating program drags in [mod-used-only].
fn program_needs_fusion(symbols: &Symbols<'_>, reachable: &HashSet<&ModulePath>) -> bool {
    symbols.handlers.iter().any(|(name, h)| {
        h.effects.iter().flatten().count() > 0
            && symbols
                .handler_modules
                .get(name)
                .is_none_or(|m| reachable.contains(*m))
    })
}

/// Does `code` mention `name` as a whole identifier? Used to spot a type
/// parameter surviving into generated code that cannot declare one.
fn mentions_ident(code: &str, name: &str) -> bool {
    let is_ident_byte = |b: u8| b.is_ascii_alphanumeric() || b == b'_';
    let bytes = code.as_bytes();
    let mut from = 0;
    while let Some(at) = code[from..].find(name) {
        let start = from + at;
        let end = start + name.len();
        let before_ok = start == 0 || !is_ident_byte(bytes[start - 1]);
        let after_ok = end == bytes.len() || !is_ident_byte(bytes[end]);
        if before_ok && after_ok {
            return true;
        }
        from = start + 1;
    }
    false
}

/// [kt-effect-fusion] A rendered effect instance as an identifier fragment:/// `Random<Int>` → `Random_Int`. Not injective (an effect literally named
/// `Random_Int` collides); the registration guard reports the collision
/// rather than silently sharing an interface.
fn sanitize_instance(rendered: &str) -> String {
    let mut out = String::new();
    for c in rendered.chars() {
        if c.is_alphanumeric() || c == '_' {
            out.push(c);
        } else if !out.ends_with('_') {
            out.push('_');
        }
    }
    out.trim_end_matches('_').to_string()
}

fn kotlin_package(module: &ModulePath) -> String {
    let mut out = String::from("salvo");
    for part in &module.0 {
        out.push('.');
        out.push_str(&kt_ident(part));
    }
    out
}

/// [qual-lift] Visits every `^` check a condition applies, through `&&`
/// chains as well.
fn collect_widen_checks<'a>(cond: &'a Expr, f: &mut impl FnMut(&'a Expr, Span)) {
    match cond {
        // [qual-lift] A lift **with a binding** materializes the value into
        // that name instead, so there is no shadow to make: the binding is
        // emitted by the `is`-binding path, and a multi-arm lift narrows
        // nothing to shadow anyway.
        Expr::Widen {
            subject,
            binding: None,
            span,
            ..
        } => f(subject, *span),
        Expr::Binary {
            op: BinaryOp::And,
            lhs,
            rhs,
            ..
        } => {
            collect_widen_checks(lhs, f);
            collect_widen_checks(rhs, f);
        }
        _ => {}
    }
}

/// The generated throw signal [kt-throw-signal]: the JVM's unwinding *is*
/// the propagation, so `throw` throws and the innermost `try` catches. No
/// stack trace and no suppression bookkeeping — it is a control transfer,
/// not an error — and it is never visible in Salvo source, so nothing can
/// catch it by accident: `try` compiles to the only `catch` for it.
///
/// The `tag` is the Salvo type of the message as written at the throw site.
/// Rust wraps a message into the delimiter's arm at the *propagation* site,
/// which it has (`?` unwrapping a `ControlFlow`); the JVM has no such site
/// — the exception flies through untouched — so the arm is chosen at the
/// `catch`, and the tag is what tells it which one. Erasure-proof by
/// construction: it compares Salvo type names, not JVM classes.
/// Source in `runtime/throw.kt`, included verbatim and compiled directly
/// by `runtime_tests.rs`.
fn generate_throw_file() -> String {
    include_str!("../runtime/throw.kt").to_string()
}

/// [kt-ordered] The structural comparison an ordered struct's
/// `compareTo` uses for each field [col-hashed-ordered].
///
/// Source in `runtime/keyed.kt`, included verbatim [backend-companion].
fn generate_keyed_file() -> String {
    include_str!("../runtime/keyed.kt").to_string()
}

/// Source in `runtime/compare.kt`, included verbatim and compiled directly
/// by `runtime_tests.rs`.
fn generate_compare_file() -> String {
    include_str!("../runtime/compare.kt").to_string()
}

/// [kt-bytes] The `Bytes` buffer class, which is `Bytes` *and* `Mut Bytes`
/// [bytes-type]: neither `List<UByte>` (a box per element) nor `UByteArray`
/// (fixed-size, and not a `List<T>`, so generic code cannot take one) is a
/// growable byte buffer on this backend.
///
/// Source in `runtime/bytes.kt`, included verbatim and compiled directly by
/// `runtime_tests.rs`.
/// [kt-actor] The actor scheduler asynchronous effect handlers run on —
/// the mirror of the Rust backend's, with identical decided semantics and
/// identical observable behaviour (asserted by the runtime tests). Emitted
/// only when a program spawns.
/// Source in `runtime/scheduler.kt`.
fn generate_scheduler_file() -> String {
    include_str!("../runtime/scheduler.kt").to_string()
}

/// [time-types] [kt-time] The two clock readings `time`'s effects are built
/// on, mirroring the Rust backend's `time.rs` number for number. Emitted only
/// when a program reads time. Source in `runtime/hosttime.kt`.
fn generate_time_file() -> String {
    include_str!("../runtime/hosttime.kt").to_string()
}

/// [actor-use-addr] The forwarding stub of a protocol: `__Stub_Counter`.
fn stub_class_name(effect: &str) -> String {
    format!("__Stub_{effect}")
}

/// [monitor-handler] [kt-monitor] The lock wrapper of a **plain** effect:
/// `__Mon_Random`, the effect implemented by synchronizing on a shared
/// instance and delegating — what an `Addr<Random>` *is* in Kotlin, and what
/// a monitor spawn answers. Per effect, like the send stub: a holder knows
/// only the effect the handle serves.
fn monitor_class_name(effect: &str) -> String {
    format!("__Mon_{effect}")
}

/// [mixed-handler] [kt-mixed] The façade of a mixed handler:
/// `__Fac_CyclicRandom` — the servant's addr plus the constructor parameters
/// plus the sync member bodies. Per *handler*, since the bodies are its.
fn facade_class_name(handler: &str) -> String {
    format!("__Fac_{handler}")
}

/// The base name of a *rendered* effect instance (`Store<Int>` → `Store`) —
/// what a generated class named after the effect declaration needs. An actor
/// effect is never generic today ([actor-effect-kind] refuses one as a
/// protocol), so this only ever strips nothing; it is written for the day one
/// is allowed.
fn base_of_rendered(rendered: &str) -> &str {
    match rendered.find('<') {
        Some(at) => rendered[..at].trim(),
        None => rendered.trim(),
    }
}

/// [kt-actor] The message class of a protocol: `__Msg_Counter`, named after
/// the *effect*, since that is what a sender knows [actor-types].
fn msg_class_name(effect: &str) -> String {
    format!("__Msg_{effect}")
}

/// [kt-actor] [actor-replyto] The parked-continuation class of a protocol:
/// `__Cont_Counter`. Beside the message class and named the same way — a
/// continuation is a member invocation waiting for its last argument.
fn cont_class_name(effect: &str) -> String {
    format!("__Cont_{effect}")
}

/// [kt-actor] One nested class of it, named after the member in upper camel
/// so the generated Kotlin reads like Kotlin.
fn msg_variant_name(member: &str) -> String {
    let mut out = String::new();
    let mut upper = true;
    for c in member.chars() {
        if c == '_' {
            upper = true;
            continue;
        }
        if upper {
            out.extend(c.to_uppercase());
            upper = false;
        } else {
            out.push(c);
        }
    }
    out
}

/// [kt-actor] The generated body a spawn hands the scheduler:
/// `__Actor_Counting`, wrapping the handler instance and dispatching messages
/// onto its members.
fn actor_class_name(handler: &str) -> String {
    format!("__Actor_{handler}")
}

fn generate_bytes_file() -> String {
    include_str!("../runtime/bytes.kt").to_string()
}

/// Whether an effect instance is the throw effect [throw]: the JVM unwinds
/// to the delimiter, so it is never a handler parameter [kt-throw-signal].
fn is_throw_effect_ty(ty: &Ty) -> bool {
    matches!(ty, Ty::Named { name, .. } if name == salvo_core::THROW_EFFECT)
}

fn generate_unions_file(sizes: &BTreeSet<usize>) -> String {
    let mut out = String::from(
        "// Generated by the Salvo compiler: sealed wrappers for union types.\npackage salvo\n",
    );
    for &n in sizes {
        let params: Vec<String> = (1..=n).map(|i| format!("out T{i}")).collect();
        let args: Vec<String> = (1..=n).map(|i| format!("T{i}")).collect();
        out.push_str(&format!(
            "\nsealed interface Union{n}<{}> {{\n    val value: Any?\n}}\n",
            params.join(", ")
        ));
        for i in 1..=n {
            out.push_str(&format!(
                "data class U{n}_{i}<{}>(override val value: T{i}) : Union{n}<{}>\n",
                params.join(", "),
                args.join(", ")
            ));
        }
    }
    out
}

/// [kt-tuple-class] [type-tuple] The tuple classes Kotlin lacks: `Pair` and
/// `Triple` cover two and three, and everything past them is a **generated
/// data class** — one per arity the program actually names, in the `salvo`
/// package beside the union wrappers.
///
/// A `data class` is what makes them behave like tuples rather than like
/// objects: structural `equals`/`hashCode` (so a big tuple is a `Set` element
/// or a `Map` key on the same terms as a `Pair`) and `componentN` (so
/// `let (a, b, c, d) = t` destructures). Ordering goes through
/// `__salvoCompare`, which reaches them through `SalvoTuple` — the one thing a
/// generic helper cannot do by type test, since there is no common supertype
/// Kotlin already knows.
///
/// The property names are `first`, `second`, `third` and then `v3`, `v4`, …:
/// keeping `Pair`/`Triple`'s three names is what lets one index rule serve
/// every arity [kt-tuple-component].
fn generate_tuples_file(sizes: &BTreeSet<usize>) -> String {
    let mut out = String::from(
        "// Generated by the Salvo compiler: the tuple types Kotlin does not \
         have [kt-tuple-class].\npackage salvo\n",
    );
    for &n in sizes {
        let params: Vec<String> = (1..=n).map(|i| format!("out T{i}")).collect();
        let args: Vec<String> = (1..=n).map(|i| format!("T{i}")).collect();
        let fields: Vec<String> = (0..n)
            .map(|i| format!("val {}: T{}", tuple_field(i), i + 1))
            .collect();
        let parts: Vec<String> = (0..n).map(tuple_field).collect();
        out.push_str(&format!(
            "\ndata class SalvoTuple{n}<{}>(\n    {},\n) : SalvoTuple {{\n    \
             override val __parts: List<Any?> get() = listOf({})\n}}\n",
            params.join(", "),
            fields.join(",\n    "),
            parts.join(", ")
        ));
        let _ = &args;
    }
    out
}

/// [kt-tuple-component] The property a generated tuple gives position `i`.
fn tuple_field(i: usize) -> String {
    match i {
        0 => "first".to_string(),
        1 => "second".to_string(),
        2 => "third".to_string(),
        n => format!("v{n}"),
    }
}

/// Does this module contain anything that turns into Kotlin code?
fn module_produces_code(module: &Module) -> bool {
    module.items.iter().any(|item| match item {
        Item::Struct(_) | Item::Effect(_) => true,
        Item::Handler(_) => true,
        Item::Fn(f) => f.body.is_some() || f.structural,
        Item::Qualifier(q) => q.fns.iter().any(|f| f.body.is_some()),
        _ => false,
    })
}

/// [platform-tree] [kt-platform-host] The Kotlin package of a module's host
/// file: `salvo.platform.` plus the module path. A package of its own is
/// what keeps the host's facade class distinct from the generated module's
/// — both files are named after the module, so sharing a package would put
/// two `MainKt` classes on the classpath.
pub fn host_package(module: &ModulePath) -> String {
    let mut out = String::from("salvo.platform");
    for part in &module.0 {
        out.push('.');
        out.push_str(&kt_ident(part));
    }
    out
}

/// [platform-tree] The Kotlin class name a host implementation gets for
/// effect `E`: `EHost`. Named, not anonymous, because the file is the
/// customer's from the moment it is written — they need something to hang
/// state and constructor parameters on.
fn host_class(effect: &str) -> String {
    format!("{effect}Host")
}

/// [platform-tree] [kt-platform-host] Renders the host implementation
/// skeleton for every module that declares platform effects, plus the
/// module whose `main` needs one (which is where the program's real entry
/// point goes).
///
/// This is `salvo platform generate`'s whole output. It runs the front end
/// exactly as `emit_program` does — the skeleton has to match the
/// interfaces the emitter generates, member for member, so the two must be
/// rendered by the same code against the same checked program.
pub fn platform_skeletons(program: &Program) -> Result<Vec<EmittedFile>, Vec<String>> {
    let symbols = Symbols::collect(program);
    let resolution = salvo_core::resolve(program);
    let checked = salvo_core::check_program(program, &resolution, &symbols);
    // Only errors stop the skeleton renderer, and warnings are *not*
    // surfaced here: `salvo platform generate` writes host stubs once, and
    // nagging about the program's diagnostics is the compile path's job
    // (`emit_program_reporting`) and `salvo analyze`'s
    // [qual-refn-conflict].
    if checked.errors.iter().any(|d| d.is_error()) {
        return Err(checked
            .errors
            .iter()
            .filter(|d| d.is_error())
            .map(|d| d.render(&program.files))
            .collect());
    }

    // Which module declares each platform effect, so a host file can import
    // an effect that lives elsewhere.
    let mut effect_module: HashMap<&str, &ModulePath> = HashMap::new();
    for unit in program.units() {
        for e in salvo_core::platform_effects(unit.ast) {
            effect_module.insert(e.name.name.as_str(), &unit.file.module);
        }
    }
    // [platform-handler] And every effect, platform or not: a platform
    // handler implements an *ordinary* effect, whose interface may live in
    // another module's package (std's, typically).
    let mut all_effect_module: HashMap<&str, &ModulePath> = HashMap::new();
    for unit in program.units() {
        for item in &unit.ast.items {
            if let Item::Effect(e) = item {
                all_effect_module.insert(e.name.name.as_str(), &unit.file.module);
            }
        }
    }

    let mut files = Vec::new();
    let mut errors = Vec::new();
    for (file_idx, unit) in program.units().enumerate() {
        if unit.file.is_std {
            continue;
        }
        let effects = salvo_core::platform_effects(unit.ast);
        let handlers = salvo_core::platform_handlers(unit.ast);
        let entry = salvo_core::platform_entry(unit.ast, &symbols);
        if effects.is_empty() && handlers.is_empty() && entry.is_none() {
            continue;
        }
        let module = &unit.file.module;
        let mut emitter =
            Emitter::new(&symbols, &checked, program, file_idx, &unit.file.name);

        let mut body = String::new();
        for e in &effects {
            body.push_str(&emitter.host_impl(e));
        }
        // [platform-handler] One class per platform handler, named after the
        // handler itself: the `use` site constructs *this* class, so the
        // Salvo name and the Kotlin name are the same name.
        for h in &handlers {
            body.push_str(&emitter.host_handler_impl(h));
        }
        // Imports: the module's own generated package always (the
        // interfaces and the entry point live there), plus the packages of
        // any platform effect declared elsewhere that the entry needs, and
        // of the effect a platform handler implements [platform-handler] —
        // which is an ordinary effect and may be declared anywhere, std
        // included.
        let mut imports: BTreeSet<String> = BTreeSet::new();
        imports.insert(format!("import {}.*", kotlin_package(module)));
        for h in &handlers {
            // [platform-handler] [effect-handler-multi] One face: the host
            // writes one class implementing one generated interface.
            let Some(effect) = type_base_name(&h.of[0]) else {
                continue;
            };
            match all_effect_module.get(effect) {
                Some(other) if *other != module => {
                    imports.insert(format!("import {}.*", kotlin_package(other)));
                }
                Some(_) => {}
                None => errors.push(format!(
                    "{}: `platform handler {}` implements `{effect}`, whose \
                     declaration could not be located",
                    unit.file.name, h.name.name
                )),
            }
        }
        if let Some(f) = entry {
            let args = emitter.platform_entry_effects(f);
            let mut calls = Vec::new();
            for effect in &args {
                match effect_module.get(effect.as_str()) {
                    Some(other) if *other != module => {
                        imports.insert(format!("import {}.*", kotlin_package(other)));
                        imports.insert(format!("import {}.*", host_package(other)));
                    }
                    Some(_) => {}
                    None => errors.push(format!(
                        "{}: `main` needs the platform effect `{effect}`, whose \
                         declaration could not be located",
                        unit.file.name
                    )),
                }
                calls.push(format!("{}()", host_class(effect)));
            }
            body.push_str(&format!(
                "\n// The program's entry point [kt-platform-host]: Salvo's `main` \
                 needs a\n// platform effect, so it is emitted as `{SALVO_ENTRY}` \
                 and this is the\n// `main` the toolchain runs.\nfun main() {{\n    \
                 {SALVO_ENTRY}({})\n}}\n",
                calls.join(", ")
            ));
        }
        errors.extend(std::mem::take(&mut emitter.errors));
        // [kt-platform-host] The union wrappers a fallible member's result
        // lowers to live in the root `salvo` package, so a host file whose
        // signatures mention one has to import it. Same reason as the
        // module import above: the skeleton's job is to compile.
        if !emitter.union_sizes.is_empty() {
            imports.insert("import salvo.*".to_string());
        }

        let mut content = format!(
            "// Host implementation of the platform declarations of Salvo module \
             `{module}`.\n//\n// Generated once by `salvo platform generate`; the \
             compiler never writes\n// this file again — it is yours. Nothing here \
             is checked by Salvo: the\n// Kotlin compiler checks it, against the \
             interfaces the backend generates\n// from the `platform effect` and \
             `platform handler` declarations.\npackage {}\n\n",
            host_package(module)
        );
        for import in &imports {
            content.push_str(import);
            content.push('\n');
        }
        content.push_str(&body);
        files.push(EmittedFile {
            rel_path: salvo_core::host_rel_path(module, "kt"),
            content,
        });
    }
    if errors.is_empty() {
        Ok(files)
    } else {
        Err(errors)
    }
}

/// Kotlin reserved words that need backtick-escaping as identifiers.
/// [platform-effect] The name a `main` that needs platform effects is
/// emitted under. The host's own `main` constructs the implementations and
/// calls this; the two cannot both be called `main`, and the host's is the
/// one the toolchain must find.
pub const SALVO_ENTRY: &str = "salvoMain";

/// [effect-handler-deps] [kt-effect-fusion] The field a handler with
/// dependencies stores its fused environment in. One per handler *instance*,
/// built at the `use` site: a handler holds its dependencies for its
/// lifetime, so nothing is rebuilt per member call (user decision
/// 2026-09-14). Salvo never names the dependencies, so the name is the
/// emitter's and cannot collide with a written one.
const HANDLER_CARRIER: &str = "__fx";

/// [kt-effect-fusion] The type parameter a dependent handler's carrier is
/// bound to. A *parameter* rather than one of the generated `__Fx_N`
/// classes, which are per-file: a handler declared in one module is
/// constructed in another, and the two files' classes of the same shape are
/// different types with the same name.
const HANDLER_FX: &str = "__Fx";

const KOTLIN_KEYWORDS: &[&str] = &[
    "as", "break", "class", "continue", "do", "else", "false", "for", "fun", "if", "in",
    "interface", "is", "null", "object", "package", "return", "super", "this", "throw", "true",
    "try", "typealias", "typeof", "val", "var", "when", "while",
];

fn kt_ident(name: &str) -> String {
    if KOTLIN_KEYWORDS.contains(&name) {
        format!("`{name}`")
    } else {
        name.to_string()
    }
}

struct Emitter<'p> {
    /// [placeholder] The code `_` renders as: the unpicked arm's read, set while
    /// a qualifier pick's right-hand side is emitted [pick].
    placeholder_code: Option<String>,
    /// [pick] Counter for the temporaries a qualifier pick binds.
    pick_vars: usize,
    symbols: &'p Symbols<'p>,
    checked: &'p Checked,
    program: &'p Program,
    file_idx: usize,
    file_name: String,
    imports: BTreeSet<String>,
    errors: Vec<String>,
    /// Wrapper union sizes this emitter has rendered.
    union_sizes: BTreeSet<usize>,
    /// [kt-tuple-class] The tuple arities beyond `Pair`/`Triple` this file
    /// names, merged program-wide into `tuples.kt` the way `union_sizes`
    /// drives `unions.kt`.
    tuple_sizes: BTreeSet<usize>,
    /// [fn-effects] Every effect set whose generated pass interface this
    /// file mentions, mapped to the *Kotlin* effect types the interface's
    /// `advance` takes as handler parameters. Merged program-wide into
    /// `iter_effects.kt`, the way `union_sizes` drives `unions.kt`.
    /// [fn-effects] Fully-qualified package prefix per effect name, for the
    /// generated pass-interface file (which imports nothing).
    effect_paths: HashMap<String, String>,
    /// [kt-effect-fusion] The program-wide fusion gate: any handler
    /// declares an effect dependency. Mirrors [rs-effect-fusion]'s switch —
    /// a fn's signature must not depend on which of its callers holds a
    /// fusion, so either every effect site fuses or none does.
    fusion: bool,
    /// [kt-effect-fusion] The Has-accessor interfaces this file mentions:
    /// interface name → the rendered effect instance it accesses. Merged
    /// program-wide into `fx.kt`, the way `union_sizes` drives `unions.kt`
    /// — per *instance*, not per declaration, because erasure forbids one
    /// class implementing `__Has_Random<Int>` and `__Has_Random<Double>`.
    has_ifaces: std::collections::BTreeMap<String, String>,
    /// [effect-member-overload] The **handler** whose members are being emitted
    /// as overrides, if any: a member's *name* is the effect's, which differs
    /// from the written one when the member is overloaded (`close(InStream)` /
    /// `close(OutStream)`) — and with several faces [effect-handler-multi] the
    /// face that declares each member is the one that names it, so what is kept
    /// here is the handler rather than one effect.
    handler_member_of: Option<&'p HandlerDecl>,
    /// [actor-replyto] [actor-self-send] The **name** of the handler whose
    /// members are being emitted, when any are: a `replyto` mints against the
    /// enclosing handler's protocol and a self-send calls its member. `None`
    /// in ordinary fns, which is where the checker has already refused both.
    current_handler: Option<String>,
    /// [platform-handler] [platform-tree] The modules whose `platform/`
    /// companion this file's `use` sites depend on: registering a platform
    /// handler constructs a *host* class, so the companion defining it has
    /// to exist. Collected per file and checked once, program-wide, the way
    /// `has_ifaces` is merged.
    platform_hosts: BTreeSet<ModulePath>,
    /// [kt-effect-fusion] Counter for per-file fused class names
    /// (`__Fx_1`, `__Fx_2`, …; per-file is per-package, so no wider
    /// uniqueness is needed).
    fusion_id: usize,
    /// [kt-effect-fusion] The fused classes emitted in this file, keyed by
    /// their canonical body text → class name, so one effect set gets one
    /// class however many scopes need it (user decision 2026-09-14).
    fx_classes: HashMap<String, String>,
    /// [kt-effect-fusion] Generated fused classes, appended at the end of
    /// the module body.
    generated_items: Vec<String>,
    /// [kt-none-unit] The fn being emitted returns `None`, i.e. Kotlin `Unit`:
    /// `return None` must be a **bare** `return`, since `return null` against a
    /// `Unit` return type is a kotlinc error. Not decidable from the returned
    /// *value*'s type — `return None` in a `Str?`-returning fn is `return
    /// null` and correct.
    ret_is_unit: bool,
    /// [implicit-param] The implicit parameters of the fn being emitted, in
    /// the checker's order: trailing parameters of the signature, and the
    /// names a bare call inside the body reaches as *values* rather than
    /// resolving as overloads.
    implicits: Vec<salvo_core::ImplicitParam>,
    /// [iter-fn] Passes minted at the arguments of the call being
    /// rendered: (local name, construction, release). The mint is the
    /// compiler's value, so the compiler releases it after the call —
    /// whatever the callee did with it — which is the `for` lowering's
    /// discipline (construct, drive, close) at a call site.
    pending_mints: Vec<(String, String, String)>,
    /// [iter-fn] The machine types minted at the call being emitted, in
    /// argument order: the advance adapter writes one on its lambda parameter,
    /// which is what pins the callee's type argument.
    mint_machines: Vec<String>,
    /// [kt-throw-signal] This file throws (or delimits a throw), so the
    /// program needs the generated signal class.
    needs_throw: bool,
    /// [kt-ordered] Whether this module declared an ordered struct, so
    /// the comparison runtime is emitted.
    needs_compare: bool,
    /// [cmp-carry] Whether this module builds a collection keyed by a **named**
    /// hash and equality, which is the only thing `keyed.kt` is for.
    needs_keyed: bool,
    /// [kt-bytes] Whether this file named a `Bytes`, so the program needs
    /// the buffer runtime class.
    needs_bytes: bool,
    /// [kt-actor] This file spawns, sends to an addr, or bridges with
    /// `waitfor`, so the scheduler file is part of the program.
    needs_scheduler: bool,
    /// [time-types] Whether this module reads a clock, so the time runtime is
    /// part of the program.
    needs_time: bool,
    /// [is-bind-once] Hoisted `is` subjects: the span of a subject that is not
    /// a place, mapped to the `val` holding its single evaluation. The test and
    /// the binding both read it through `emit_place_storage`; they used to
    /// emit the subject independently, so a call ran twice per test.
    is_temps: HashMap<(usize, Span), String>,
    is_temp_id: usize,
    /// [iter-fn] Of those, the ones held in a nullable property
    /// because their type has no zero value: reads unwrap with `!!`.
    gen_slots: HashSet<String>,
    /// The indentation of the statement being emitted, so an
    /// expression-position `try` block reads like the rest of the output.
    expr_indent: usize,
    /// Effect environment: handlers in scope, keyed primarily by the
    /// *checker* effect type (`ty`), with the Kotlin rendering kept for
    /// AST-side fallbacks and diagnostics [effect-disambiguation].
    effect_env: Vec<EffectEntry>,
    /// Names that are reassigned (or `++`-incremented) in the current
    /// function; these become `var`.
    mutated: HashSet<String>,
    /// Generic parameters in scope (treated as opaque type names).
    generics: HashSet<String>,
    /// [copy-implicit] The implicit constructor parameters of the handler
    /// whose members are being emitted: a call to one goes through the
    /// property (`copy(v)`), shadowing any fn of that name — exactly as a
    /// fn's own implicit parameter does.
    ctor_implicits: HashSet<String>,
    /// [kt-suppress-cast] Whether the function body being emitted contains
    /// a cast kotlinc would flag as unchecked (an erased payload read cast
    /// to a generic parameter or a parameterized type). `emit_fn_inner`
    /// reads it after the body and prepends `@Suppress("UNCHECKED_CAST")` —
    /// generated code must stay warning-free, and the author cannot edit it.
    unchecked_cast: bool,
    /// Enclosing loops during emission [while-value]: the result variable
    /// a `break value` assigns before breaking (`None` for loops whose
    /// value is discarded). Innermost last.
    loop_results: Vec<Option<String>>,
    /// Counter for unique `__loopN` lowering locals.
    loop_id: usize,
    /// [let-destructure] Pending destructuring prologues, innermost last: a
    /// loop header whose pattern destructures pushes the statements its body
    /// must open with, and the body emission pops them.
    loop_destructures: Vec<String>,
    /// Counter for unique `__destructuredN` temps [let-destructure].
    destructure_id: usize,
    /// Names already bound in the current function (parameters, locals,
    /// bindings): effect parameters and `use` variables pick names that
    /// avoid them [kt-effect-params].
    taken_names: HashSet<String>,
    /// [effect-handler-deps] Effect dependencies of the handler whose
    /// members are being emitted: a constructor parameter of effect type
    /// becomes a `private val`, so member bodies reach the effect through
    /// that field instead of a leading parameter (their signatures must
    /// match the effect interface).
    handler_deps: Vec<EffectEntry>,
    /// Statement context: inside an `iterator {}` builder, bare `return`
    /// re-targets to `return@iterator` [iter-protocol] — carried as state
    /// so value-position lowerings (value blocks, loop lowering, `when`
    /// expressions) inherit it; lambda bodies reset it (a lambda's
    /// `return` never targets the enclosing iterator).
    stmt_ctx: StmtCtx,
    /// Generated Kotlin imports for this file [kt-imports] (wildcards for
    /// referenced foreign modules, aliases for aliased imports).
    generated_imports: BTreeSet<String>,
}

/// One handler in scope: the checker effect type when known (the primary
/// lookup key — immune to rendering drift), the Kotlin rendering of the
/// effect type, and the expression providing the handler. Under the fusion
/// [kt-effect-fusion], `fused` names the one value carrying this effect
/// across *fused-callee* calls, while `expr` stays the per-effect handler
/// object member dispatch and fn-value threading use.
#[derive(Clone)]
struct EffectEntry {
    ty: Option<Ty>,
    rendered: String,
    expr: String,
    fused: Option<String>,
}

impl<'p> Emitter<'p> {
    fn new(
        symbols: &'p Symbols<'p>,
        checked: &'p Checked,
        program: &'p Program,
        file_idx: usize,
        file_name: &str,
    ) -> Self {
        Emitter {
            placeholder_code: None,
            pick_vars: 0,
            symbols,
            checked,
            program,
            file_idx,
            file_name: file_name.to_string(),
            imports: BTreeSet::new(),
            errors: Vec::new(),
            union_sizes: BTreeSet::new(),
            tuple_sizes: BTreeSet::new(),
            effect_paths: HashMap::new(),
            fusion: false,
            has_ifaces: std::collections::BTreeMap::new(),
            handler_member_of: None,
            current_handler: None,
            platform_hosts: BTreeSet::new(),
            fusion_id: 0,
            fx_classes: HashMap::new(),
            generated_items: Vec::new(),
            ret_is_unit: false,
            needs_throw: false,
            needs_compare: false,
            needs_keyed: false,
            needs_bytes: false,
            needs_scheduler: false,
            needs_time: false,
            is_temps: HashMap::new(),
            is_temp_id: 0,
            gen_slots: HashSet::new(),
                            implicits: Vec::new(),
            pending_mints: Vec::new(),
                    mint_machines: Vec::new(),
            expr_indent: 0,
            effect_env: Vec::new(),
            mutated: HashSet::new(),
            generics: HashSet::new(),
            ctor_implicits: HashSet::new(),
            unchecked_cast: false,
            loop_results: Vec::new(),
            loop_id: 0,
            loop_destructures: Vec::new(),
            destructure_id: 0,
            taken_names: HashSet::new(),
            handler_deps: Vec::new(),
            stmt_ctx: StmtCtx::Normal,
            generated_imports: BTreeSet::new(),
        }
    }

    // ================= checker table access =================

    /// The checked (logical, post-narrowing) type of an expression.
    fn ty_of(&self, span: Span) -> Option<&'p Ty> {
        self.checked.expr_ty.get(&(self.file_idx, span))
    }

    /// [op-promote] [kt-op-promote] The explicit conversion a widened operand
    /// needs where Kotlin's own operator set does not cover the mix. Only
    /// equality needs it — `==`/`!=` are type-strict on the JVM, while the
    /// arithmetic and ordering operators accept mixed widths directly — and
    /// only `Long` and `Double` can be targets, since widening goes up within
    /// a class.
    fn promote_operand(&self, span: Span, code: String) -> String {
        match self.checked.promotions.get(&(self.file_idx, span)) {
            Some(Ty::Named { name, .. }) if name == "Long" => format!("({code}).toLong()"),
            Some(Ty::Named { name, .. }) if name == "Double" => format!("({code}).toDouble()"),
            _ => code,
        }
    }

    /// The declared (physical) type of a narrowed identifier use.
    fn repr_of(&self, span: Span) -> Option<&'p Ty> {
        self.checked.repr_ty.get(&(self.file_idx, span))
    }

    fn coercion_of(&self, span: Span) -> Option<&'p Coercion> {
        self.checked.coerce.get(&(self.file_idx, span))
    }

    fn is_test_of(&self, span: Span) -> Option<&'p UnionTest> {
        self.checked.is_tests.get(&(self.file_idx, span))
    }

    /// [iter-protocol] How a `for` drives a **pass**, if its subject is one.
    fn pass_driver_of(&self, iterable: &Expr) -> Option<salvo_core::PassDriver> {
        self.checked
            .for_drivers
            .get(&(self.file_idx, iterable.span()))
            .cloned()
    }

    fn emit_pass_loop_header(
        &mut self,
        driver: salvo_core::PassDriver,
        pattern: &Pattern,
        iterable: &Expr,
        indent: usize,
    ) -> String {
        let pad = "    ".repeat(indent);
        let inner_pad = "    ".repeat(indent + 1);
        // [iter-generic-drive] The `next` is either a declared overload or an
        // **implicit parameter** of this body, called by its own name — a
        // generic pass has no declaration to resolve against, and the parameter
        // shadows the fns of that name here anyway [implicit-param].
        let decl = match driver.next.key() {
            Some(key) => match self.fn_by_key(key) {
                Some(decl) => Some(decl),
                None => {
                    self.error("the `next` this `for` resolved to is not available");
                    return String::new();
                }
            },
            None => None,
        };
        // [fn-effects] An effectful `next` takes its handlers as leading
        // arguments, threaded into every turn of the loop from the scope the
        // `for` is written in. An *implicit* `next` is refused instead: there
        // the effects live on a fn *value*, whose caller supplies them through a
        // different path [backend-never-wrong].
        let mut handler_args: Vec<String> = Vec::new();
        match &driver.next {
            salvo_core::PassMember::Fn(key) => {
                let effects: Vec<Ty> = self
                    .checked
                    .fn_effects
                    .get(key)
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|t| !is_throw_effect_ty(t))
                    .collect();
                for ty in &effects {
                    handler_args.push(self.thread_effect_fused_by_ty(ty));
                }
                // [kt-effect-fusion] An effectful `next` is an ordinary
                // fused callee: one carrier for its whole effect set.
                if self.fusion {
                    handler_args.dedup();
                    handler_args.truncate(1);
                }
            }
            salvo_core::PassMember::Implicit(_) => {
                if driver
                    .next
                    .key()
                    .and_then(|k| self.checked.fn_effects.get(&k))
                    .is_some_and(|e| !e.is_empty())
                {
                    self.error(
                        "a generic `next` that performs effects is not supported \
                         yet: the handlers would have to reach a fn value the \
                         caller supplied",
                    );
                }
            }
        }
        let lead = if handler_args.is_empty() {
            String::new()
        } else {
            format!("{}, ", handler_args.join(", "))
        };
        if driver.arms < 2 {
            self.error("a `next` result must have both an `Emitted` and a `Finished` arm");
            return String::new();
        }
        let callee = match (&driver.next, decl) {
            (salvo_core::PassMember::Implicit(name), _) => kt_ident(name),
            (_, Some(decl)) => self.kotlin_fn_name(decl),
            (_, None) => {
                self.error("the `next` this `for` resolved to is not available");
                return String::new();
            }
        };
        // A *non-generic* `next` lets the arm be spelled with its real type
        // arguments, which is what keeps the element read cast-free: an
        // `is U2_1<*, *>` smart-cast leaves `value` at `Any?`, and casting
        // back would warn ("unchecked cast") in code the user cannot edit.
        // A generic `next` has type arguments this loop does not know — there
        // is no call node to read them from — so it falls back to stars.
        let arm_args: Option<Vec<String>> = match decl {
            Some(decl) if decl.generics.is_empty() => match &decl.return_type {
                Some(Type::Union { arms, .. }) if arms.len() == driver.arms => {
                    Some(arms.iter().map(|a| self.emit_type(a)).collect())
                }
                _ => None,
            },
            // A generic `next`, or an implicit one: the type arguments are not
            // knowable here (there is no call node), so the arm is spelled with
            // stars and the element read casts.
            _ => None,
        };
        let loop_id = self.fresh_loop_var();
        let (place, step) = (format!("{loop_id}_pass"), format!("{loop_id}_step"));
        // [iter-pass] The subject is a *container*, not a pass: its `iter` mints
        // one, called once before the loop.
        let subject = match driver.mint_iter_fn {
            Some(key) => match self.fn_by_key(key) {
                Some(decl) => {
                    let callee = self.kotlin_fn_name(decl);
                    let arg = self.emit_expr(iterable);
                    format!("{callee}({arg})")
                }
                None => {
                    self.error("the `iter` this `for` mints with is not available");
                    return String::new();
                }
            },
            None => self.emit_expr(iterable),
        };
        let var = self.for_pattern_var(pattern, indent + 1);
        self.union_sizes.insert(driver.arms);
        let (arm, read) = match arm_args {
            Some(args) => (
                format!(
                    "U{}_{}<{}>",
                    driver.arms,
                    driver.emitted_arm + 1,
                    args.join(", ")
                ),
                format!("{step}.value"),
            ),
            None => {
                let elem = self
                    .binding_ty_text(pattern)
                    .unwrap_or_else(|| "Any?".to_string());
                let stars = vec!["*"; driver.arms].join(", ");
                self.note_payload_cast();
                (
                    format!("U{}_{}<{stars}>", driver.arms, driver.emitted_arm + 1),
                    format!("{step}.value as {elem}"),
                )
            }
        };
        // [iter-drive-in-place] A pass the fn *keeps* is advanced where it
        // lives, so the caller sees the position the loop reached. Kotlin's
        // local would have aliased it anyway; naming the subject directly is
        // what makes the two backends say so identically [backend-parity].
        if driver.in_place {
            return format!(
                "{pad}while (true) {{\n\
                 {inner_pad}val {step} = {callee}({lead}{subject})\n\
                 {inner_pad}if ({step} !is {arm}) {{ break }}\n\
                 {inner_pad}val {var} = {read}\n"
            );
        }
        format!(
            "{pad}var {place} = {subject}\n\
             {pad}while (true) {{\n\
             {inner_pad}val {step} = {callee}({lead}{place})\n\
             {inner_pad}if ({step} !is {arm}) {{ break }}\n\
             {inner_pad}val {var} = {read}\n"
        )
    }

    /// The Kotlin rendering of a `for` binding's type, from the checker's
    /// record for the pattern's own span.
    fn binding_ty_text(&mut self, pattern: &Pattern) -> Option<String> {
        let span = match pattern {
            Pattern::Ident(id) => id.span,
            Pattern::Tuple { span, .. } => *span,
            Pattern::Struct { span, .. } => *span,
        };
        let ty = self.ty_of(span).cloned()?;
        Some(self.kotlin_ty(&ty))
    }

    fn fn_by_key(&self, key: salvo_core::FnKey) -> Option<&'p FnDecl> {
        match self.program.modules.get(key.file)?.items.get(key.item)? {
            Item::Fn(f) => Some(f),
            _ => None,
        }
    }


    fn error(&mut self, msg: impl Into<String>) {
        self.errors.push(format!("{}: {}", self.file_name, msg.into()));
    }

    // ================= module =================

    fn emit_module(&mut self, module: &'p Module) -> String {
        let mut body = String::new();
        for item in &module.items {
            match item {
                // [name-dot] Dot-named structs are emitted *inside* their
                // namespace class, not at the top level.
                Item::Struct(s) if s.name.name.contains('.') => {}
                Item::Struct(s) => body.push_str(&self.emit_struct_with_members(s, module)),
                // [throw] The throw effect has no handlers — `try` delimits
                // it — so there is nothing to implement: emitting an
                // interface for it would be dead, misleading code.
                Item::Effect(e) if e.name.name == salvo_core::THROW_EFFECT => {}
                Item::Effect(e) => body.push_str(&self.emit_effect(e)),
                Item::Handler(h) => body.push_str(&self.emit_handler(h)),
                // [iter-fn] A `yield fn` is not a function in the
                // output: it *is* the hidden state machine the `for` sugar
                // constructs, so nothing callable is emitted for it.
                // [cmp-auto] A structural member has no body and is still
                // emitted: its body is the host's own operation.
                Item::Fn(f) if f.body.is_some() || f.structural => {
                    body.push_str(&self.emit_fn(f))
                }
                Item::Qualifier(q) => body.push_str(&self.emit_qualifier(q)),
                _ => {}
            }
        }
        // [kt-effect-fusion] The fused classes generated for this file's
        // `use` sites and boundary combiners.
        for item in std::mem::take(&mut self.generated_items) {
            body.push_str(&item);
        }
        // [kt-package] Each module gets its own Kotlin package.
        let pkg = kotlin_package(&self.program.files[self.file_idx].module);
        let mut out = format!("package {pkg}\n");
        // [kt-imports] Generated module imports, template `imports:`
        // lines, and the sealed union wrappers when this file uses any.
        let mut imports = self.generated_imports.clone();
        imports.extend(self.imports.iter().cloned());
        if !self.union_sizes.is_empty() {
            imports.insert("import salvo.*".to_string());
        }
        // [kt-effect-fusion] The Has-accessor interfaces live in `fx.kt`,
        // in the root `salvo` package.
        if !self.has_ifaces.is_empty() {
            imports.insert("import salvo.*".to_string());
        }
        // [kt-throw] And so does the throw signal. A module that throws
        // without also using a union wrapper had no import for it — which
        // nothing hit until `std.test`'s assertions, the first throwing
        // module that is not an entry point (found 2026-09-23).
        if self.needs_throw {
            imports.insert("import salvo.*".to_string());
        }
        if !imports.is_empty() {
            out.push('\n');
            for import in &imports {
                out.push_str(import);
                out.push('\n');
            }
        }
        out.push_str(&body);
        out
    }

    // ================= declarations =================

    /// A struct plus the dot-named structs it namespaces, emitted as
    /// Kotlin *nested* classes [name-dot] [kt-nested-dot-name]. Nested,
    /// never `inner`: an `inner class` captures an outer instance and
    /// could not be constructed on its own.
    fn emit_struct_with_members(&mut self, s: &StructDecl, module: &Module) -> String {
        let prefix = format!("{}.", s.name.name);
        let members: Vec<&StructDecl> = module
            .items
            .iter()
            .filter_map(|item| match item {
                Item::Struct(m) if m.name.name.starts_with(&prefix) => Some(m),
                _ => None,
            })
            .collect();
        let outer = self.emit_struct(s);
        if members.is_empty() {
            return outer;
        }
        let mut nested = String::new();
        for m in members {
            for line in self.emit_struct(m).lines() {
                if line.is_empty() {
                    nested.push('\n');
                } else {
                    nested.push_str(&format!("    {line}\n"));
                }
            }
        }
        // `data class X(\n ... )\n` grows a body holding the members.
        format!("{} {{\n{nested}}}\n", outer.trim_end())
    }

    fn emit_struct(&mut self, s: &StructDecl) -> String {
        let is_mut = s
            .auto_qualifiers
            .iter()
            .any(|q| q.name.name == "Mut");
        let saved = self.enter_generics(&s.generics);
        let generics = self.emit_generic_params(&s.generics);
        // A dot-named struct is declared with its *member* segment: it is
        // nested inside its namespace class [name-dot].
        let declared_name = s.name.name.rsplit('.').next().unwrap_or(&s.name.name);
        // [kt-struct-empty] A *fieldless* struct cannot be a data class:
        // Kotlin requires at least one primary-constructor parameter
        // ("data class must have at least one primary constructor
        // parameter"). A plain class is the faithful rendering — with no
        // fields there is no state for `equals`/`copy` to compare or clone,
        // so nothing the data modifier would have provided is observable.
        // Such a struct is a tag: `struct Finished {}` in std's iterator
        // protocol [iter-protocol] is the first one, and `is Finished` is a
        // type test either way.
        if s.fields.is_empty() {
            self.generics = saved;
            return format!("\nclass {declared_name}{generics}\n");
        }
        // [col-hashed-ordered] [kt-ordered] An ordered struct needs a real
        // `Comparable`: a Kotlin data class gets `equals`/`hashCode` for free
        // but *not* comparison, so `p < q` would be an unresolved
        // `compareTo`. The order is lexicographic by field declaration order,
        // which is the language's rule and matches Rust's derived `Ord`.
        // [cmp-auto] An `auto fn cmp@T` is what asks for a real `Comparable`:
        // the generated `cmp@T` is defined in terms of this `compareTo`
        // [kt-cmp-groups]. Asked of the *declaration*, not of an obligation
        // clause — `auto` is a modifier on the function now (user decision
        // 2026-09-22), so a struct may generate a `cmp` with no clause at all.
        let ordered = self.program.has_auto_member(&s.name.name, "cmp");
        let self_ty = format!(
            "{declared_name}{}",
            if s.generics.is_empty() {
                String::new()
            } else {
                format!(
                    "<{}>",
                    s.generics
                        .iter()
                        .map(|g| g.name.clone())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
        );
        let mut out = format!("\ndata class {declared_name}{generics}(\n");
        for field in &s.fields {
            let kw = if is_mut { "var" } else { "val" };
            let ty = self.emit_type(&field.ty);
            let default = match &field.default {
                Some(expr) => format!(" = {}", self.emit_expr(expr)),
                None => String::new(),
            };
            out.push_str(&format!(
                "    {kw} {}: {ty}{default},\n",
                kt_ident(&field.name.name)
            ));
        }
        // [col-equality] [kt-float-eq] Salvo owns floating-point equality, so
        // a struct with a float field **overrides** the data class's
        // `equals`, which makes every `==` on it (and every `contains`, and
        // every collection lookup) use our semantics.
        //
        // The default would diverge from Rust: Kotlin's generated `equals`
        // calls `Double.equals`, for which `NaN` equals itself and `+0.0`
        // differs from `-0.0`, while Rust's derived `PartialEq` is IEEE —
        // the opposite on both counts. Verified before fixing: the same
        // program printed `struct nan == nan: false` on Rust and `true` on
        // Kotlin. Comparing the fields with `==`, whose operands are
        // statically `Double`/`Float`, is IEEE, so the two agree.
        //
        // `hashCode` is left to the data class: a float-bearing struct is
        // barred from `: auto Hashed<self>` [col-hashed-ordered], so it never
        // reaches a hash table where the (NaN-only) inconsistency could
        // matter.
        if ordered {
            // Comparison first, then the float-aware equality if it is also
            // needed — both live in the same class body.
            out.push_str(&format!(") : Comparable<{self_ty}> {{\n"));
            out.push_str(&format!(
                "    override fun compareTo(other: {self_ty}): Int {{\n"
            ));
            // [kt-ordered] Through the runtime helper rather than
            // `field.compareTo(...)`: Salvo says a `List` or a tuple is
            // orderable when its elements are, and neither is `Comparable`
            // on the JVM [col-hashed-ordered].
            self.needs_compare = true;
            for field in &s.fields {
                let name = kt_ident(&field.name.name);
                out.push_str(&format!(
                    "        run {{ val __c = salvo.__salvoCompare({name}, other.{name}); if (__c != 0) return __c }}\n"
                ));
            }
            out.push_str("        return 0\n    }\n");
            if self.struct_has_float_field(s) {
                let star_args = if s.generics.is_empty() {
                    String::new()
                } else {
                    format!(
                        "<{}>",
                        s.generics.iter().map(|_| "*").collect::<Vec<_>>().join(", ")
                    )
                };
                let comparisons: Vec<String> = s
                    .fields
                    .iter()
                    .map(|f| {
                        let name = kt_ident(&f.name.name);
                        format!("{name} == other.{name}")
                    })
                    .collect();
                out.push_str("    override fun equals(other: Any?): Boolean {\n");
                out.push_str("        if (this === other) return true\n");
                out.push_str(&format!(
                    "        if (other !is {declared_name}{star_args}) return false\n"
                ));
                out.push_str(&format!("        return {}\n", comparisons.join(" && ")));
                out.push_str("    }\n");
            }
            out.push_str("}\n");
        } else if self.struct_has_float_field(s) {
            let star_args = if s.generics.is_empty() {
                String::new()
            } else {
                format!(
                    "<{}>",
                    s.generics.iter().map(|_| "*").collect::<Vec<_>>().join(", ")
                )
            };
            let comparisons: Vec<String> = s
                .fields
                .iter()
                .map(|f| {
                    let name = kt_ident(&f.name.name);
                    format!("{name} == other.{name}")
                })
                .collect();
            out.push_str(") {\n");
            out.push_str("    override fun equals(other: Any?): Boolean {\n");
            out.push_str("        if (this === other) return true\n");
            out.push_str(&format!(
                "        if (other !is {declared_name}{star_args}) return false\n"
            ));
            out.push_str(&format!("        return {}\n", comparisons.join(" && ")));
            out.push_str("    }\n}\n");
        } else {
            out.push_str(")\n");
        }
        self.generics = saved;
        out
    }

    /// [col-equality] Whether a struct has a *direct* floating-point field,
    /// which is what makes the data class's `equals` disagree with Rust's
    /// derive. A nested struct needs no special case: if it holds a float it
    /// gets its own comparison, which this one then calls.
    fn struct_has_float_field(&self, s: &StructDecl) -> bool {
        s.fields.iter().any(|f| match &f.ty {
            Type::Named { base, .. } => matches!(base.name.name.as_str(), "Double" | "Float"),
            _ => false,
        })
    }

    fn emit_effect(&mut self, e: &EffectDecl) -> String {
        let saved = self.enter_generics(&e.generics);
        let generics = self.emit_generic_params(&e.generics);
        let mut out = format!("\ninterface {}{generics} {{\n", e.name.name);
        for (i, f) in e.fns.iter().enumerate() {
            // A member's own generics render on the member
            // [effect-member-generics].
            let member_saved = self.enter_generics(&f.generics);
            let member_generics = self.emit_generic_params(&f.generics);
            let params = self.emit_member_param_list_with_implicits(f);
            let ret = self.emit_return_type(f.return_type.as_ref());
            out.push_str(&format!(
                "    fun{member_generics} {}({params}){ret}\n",
                self.member_name(e, i)
            ));
            self.generics = member_saved;
        }
        out.push_str("}\n");
        // [kt-actor] [actor-use-addr] The forwarding stub: the effect,
        // implemented by sending to an addr.
        out.push_str(&self.emit_addr_stub(e));
        // [monitor-handler] [kt-monitor] The lock wrapper: the effect,
        // implemented by synchronizing on a shared instance and delegating —
        // what a monitor spawn answers and `Addr<E>` lowers to for a plain
        // effect.
        out.push_str(&self.emit_monitor_stub(e));
        // [kt-actor] [actor-send-fn] The protocol's **message type**: a
        // sealed class with one nested class per send member. It belongs to
        // the *effect*, because a sender holds an `Addr` and knows only the
        // effect it serves — the same reason the binding swap works.
        out.push_str(&self.emit_message_classes(e));
        self.generics = saved;
        out
    }

    /// [kt-actor] `sealed class __Msg_E { class Member(payload…) : __Msg_E() }`
    /// — the messages an actor serving `E` receives, and the whole of what
    /// crosses the seam at runtime (the scheduler is untyped: `Any?`).
    /// [actor-use-addr] [kt-actor] `__Stub_E`: the effect implemented by
    /// **sending to an addr** — what `use addr` binds, and what a spawned child
    /// receives for an addr-supplied dependency. Per *effect*, since that is
    /// all it depends on.
    fn emit_addr_stub(&mut self, e: &EffectDecl) -> String {
        if !e.is_actor || !e.generics.is_empty() {
            return String::new();
        }
        let sends: Vec<(usize, &FnDecl)> = e
            .fns
            .iter()
            .enumerate()
            .filter(|(_, f)| f.is_send)
            .collect();
        if sends.is_empty() {
            return String::new();
        }
        self.needs_scheduler = true;
        let name = stub_class_name(&e.name.name);
        let msg = msg_class_name(&e.name.name);
        let mut out = format!(
            "\nclass {name}(private val addr: Int) : {} {{\n",
            e.name.name
        );
        for (i, f) in &sends {
            let member = salvo_core::effect_member_name(e, *i);
            let params = self.emit_member_param_list_with_implicits(f);
            let args: Vec<String> = f
                .params
                .iter()
                .filter(|p| !p.implicit)
                .map(|p| p.name.name.clone())
                .collect();
            out.push_str(&format!(
                "    override fun {member}({params}) {{\n        \
                 salvo.SalvoSched.send(addr, {msg}.{}({}))\n    }}\n",
                msg_variant_name(&member),
                args.join(", ")
            ));
        }
        out.push_str("}\n");
        out
    }

    /// [monitor-handler] [kt-monitor] `__Mon_E`: a **plain** effect
    /// implemented by synchronizing on a shared handler instance and
    /// delegating — the monitor of SH-3 (user decision 2026-09-19), the
    /// Kotlin half of the Rust backend's `Arc<Mutex<dyn E + Send>>` wrapper.
    /// The lock is innermost by construction (the checker refused the handler
    /// any dependencies, so a member can perform no effect and no wait while
    /// it is held) — which is also why the JVM monitor's *re-entrancy* can
    /// never be observed against Rust's non-reentrant `Mutex`: no member can
    /// reach another handler, so no path routes back [backend-never-wrong].
    ///
    /// Per effect, like the send stub; emitted for every plain effect —
    /// generic exactly as the effect is (`__Mon_Random<T>` for `Random<T>`,
    /// [kt-monitor]), so a stateful handler of a generic effect instance
    /// shares like any other — and unused wrappers are inert classes kotlinc
    /// accepts quietly.
    fn emit_monitor_stub(&mut self, e: &EffectDecl) -> String {
        if e.is_actor {
            return String::new();
        }
        let members: Vec<(usize, &FnDecl)> = e
            .fns
            .iter()
            .enumerate()
            .filter(|(_, f)| !f.is_send)
            .collect();
        let name = monitor_class_name(&e.name.name);
        let generics = self.emit_generic_params(&e.generics);
        let g_args = if e.generics.is_empty() {
            String::new()
        } else {
            format!(
                "<{}>",
                e.generics
                    .iter()
                    .map(|g| g.name.clone())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        let mut out = format!(
            "\nclass {name}{generics}(private val inner: {eff}{g_args}) : {eff}{g_args} {{\n",
            eff = e.name.name
        );
        for (i, f) in &members {
            let member = self.member_name(e, *i);
            // A member's own generics render on the override, exactly as they
            // do on the interface [effect-member-generics].
            let member_saved = self.enter_generics(&f.generics);
            let member_generics = self.emit_generic_params(&f.generics);
            let params = self.emit_member_param_list_with_implicits(f);
            let ret = self.emit_return_type(f.return_type.as_ref());
            let mut args: Vec<String> = f
                .params
                .iter()
                .filter(|p| !p.implicit)
                .map(|p| kt_ident(&p.name.name))
                .collect();
            args.extend(self.implicits_of(f).iter().map(|imp| kt_ident(&imp.name)));
            out.push_str(&format!(
                "    override fun{member_generics} {member}({params}){ret} =\n        \
                 synchronized(inner) {{ inner.{member}({}) }}\n",
                args.join(", ")
            ));
            self.generics = member_saved;
        }
        out.push_str("}\n");
        out
    }

    fn emit_message_classes(&mut self, e: &EffectDecl) -> String {
        // [actor-effect-kind] The kind is the gate, as in the Rust backend.
        if !e.is_actor {
            return String::new();
        }
        let sends: Vec<(usize, &FnDecl)> = e
            .fns
            .iter()
            .enumerate()
            .filter(|(_, f)| f.is_send)
            .collect();
        if sends.is_empty() {
            return String::new();
        }
        if !e.generics.is_empty() {
            self.error(format!(
                "effect `{}` is generic, which the kotlin backend cannot make a \
                 actor protocol of yet",
                e.name.name
            ));
            return String::new();
        }
        self.needs_scheduler = true;
        let enum_name = msg_class_name(&e.name.name);
        let mut out = format!("\nsealed class {enum_name} {{\n");
        for (i, f) in &sends {
            let member = salvo_core::effect_member_name(e, *i);
            let variant = msg_variant_name(&member);
            let payload: Vec<String> = f
                .params
                .iter()
                .filter(|p| !p.implicit)
                .map(|p| format!("val {}: {}", p.name.name, self.emit_type(&p.ty)))
                .collect();
            out.push_str(&format!(
                "    class {variant}({}) : {enum_name}()\n",
                payload.join(", ")
            ));
        }
        out.push_str("}\n");
        out
    }

    /// [kt-actor] [actor-replyto] `__Cont_H`: the **parked continuation**,
    /// emitted beside the *handler* whose members it names
    /// [effect-handler-multi] — a mint is lexical, so the members are the
    /// handler's, and with several faces they come from several protocols.
    ///
    /// A `replyto k(captures)` stores one under the slot the runtime handed
    /// back; `resume` looks it up, and the subclass says which member to call
    /// and therefore what to cast the answer to. So a subclass carries the
    /// member's parameters **minus the trailing one**: that last parameter is
    /// the answer itself [actor-replyto], which arrives with the reply.
    ///
    /// A parameterless member can never be a target — there is no answer for
    /// the token to carry — so it gets no subclass.
    fn emit_cont_classes(&mut self, h: &HandlerDecl) -> String {
        // [mixed-handler] [kt-mixed] Handler-keyed for a mixed servant: the
        // subclasses are its own `send fn` members, named as `__Msg_H`'s
        // variants are.
        if self.handler_is_mixed(h) {
            let targets = self.mixed_cont_targets(h);
            if targets.is_empty() {
                return String::new();
            }
            let cont_name = cont_class_name(&h.name.name);
            let mut out = format!("\nsealed class {cont_name} {{\n");
            for f in targets {
                let variant = msg_variant_name(&f.name.name);
                let fixed: Vec<&Param> = f.params.iter().filter(|p| !p.implicit).collect();
                let captures: Vec<String> = fixed[..fixed.len() - 1]
                    .iter()
                    .map(|p| format!("val {}: {}", p.name.name, self.emit_type(&p.ty)))
                    .collect();
                if captures.is_empty() {
                    out.push_str(&format!("    object {variant} : {cont_name}()\n"));
                } else {
                    out.push_str(&format!(
                        "    class {variant}({}) : {cont_name}()\n",
                        captures.join(", ")
                    ));
                }
            }
            out.push_str("}\n");
            return out;
        }
        let targets = self.cont_targets(h);
        if targets.is_empty() {
            return String::new();
        }
        let cont_name = cont_class_name(&h.name.name);
        let mut out = format!("\nsealed class {cont_name} {{\n");
        for (e, i) in &targets {
            let Some(f) = e.fns.get(*i) else { continue };
            let member = salvo_core::effect_member_name(e, *i);
            let variant = msg_variant_name(&member);
            let fixed: Vec<&Param> = f.params.iter().filter(|p| !p.implicit).collect();
            let captures: Vec<String> = fixed[..fixed.len() - 1]
                .iter()
                .map(|p| format!("val {}: {}", p.name.name, self.emit_type(&p.ty)))
                .collect();
            out.push_str(&format!(
                "    class {variant}({}) : {cont_name}()\n",
                captures.join(", ")
            ));
        }
        out.push_str("}\n");
        out
    }

    /// [platform-tree] [kt-platform-host] One host implementation skeleton:
    /// a named class implementing the generated interface, every member
    /// stubbed with `TODO`. The signatures come from
    /// [`Emitter::emit_effect`]'s own renderers, so a skeleton that drifts
    /// from the interface is impossible by construction.
    fn host_impl(&mut self, e: &EffectDecl) -> String {
        let mut out = format!(
            "\nclass {} : {} {{\n",
            host_class(&e.name.name),
            e.name.name
        );
        for (i, f) in e.fns.iter().enumerate() {
            let member_saved = self.enter_generics(&f.generics);
            let params = self.emit_member_param_list_with_implicits(f);
            let ret = self.emit_return_type(f.return_type.as_ref());
            out.push_str(&format!(
                "    override fun {}({params}){ret} {{\n        \
                 TODO(\"implement {}.{}\")\n    }}\n",
                self.member_name(e, i),
                e.name.name,
                f.name.name
            ));
            self.generics = member_saved;
        }
        out.push_str("}\n");
        out
    }

    /// [platform-handler] [kt-platform-handler] One host *handler* skeleton: a
    /// class named after the handler, implementing the generated interface of
    /// the ordinary effect it handles, with the handler's constructor
    /// parameters as its own and every member stubbed with `TODO`. The `use`
    /// site constructs exactly this class, so the name is not the emitter's
    /// to choose.
    fn host_handler_impl(&mut self, h: &HandlerDecl) -> String {
        let of = self.emit_type(&h.of[0]);
        let Some(effect) = type_base_name(&h.of[0])
            .and_then(|n| self.symbols.effects.get(n))
            .copied()
        else {
            self.error(format!(
                "platform handler `{}` implements `{of}`, which is not a declared \
                 effect",
                h.name.name
            ));
            return String::new();
        };
        let ctor = if h.params.is_empty() {
            String::new()
        } else {
            let params: Vec<String> = h
                .params
                .iter()
                .map(|p| {
                    format!(
                        "private val {}: {}",
                        kt_ident(&p.name.name),
                        self.emit_type(&p.ty)
                    )
                })
                .collect();
            format!("({})", params.join(", "))
        };
        let mut out = format!("\nclass {}{ctor} : {of} {{\n", kt_ident(&h.name.name));
        for (i, f) in effect.fns.iter().enumerate() {
            let member_saved = self.enter_generics(&f.generics);
            let params = self.emit_member_param_list_with_implicits(f);
            let ret = self.emit_return_type(f.return_type.as_ref());
            out.push_str(&format!(
                "    override fun {}({params}){ret} {{\n        \
                 TODO(\"implement {}.{}\")\n    }}\n",
                self.member_name(effect, i),
                effect.name.name,
                f.name.name
            ));
            self.generics = member_saved;
        }
        out.push_str("}\n");
        out
    }

    /// [platform-tree] The platform effects `main` receives, as rendered
    /// Kotlin type names *in parameter order* — the arguments the host's
    /// `main` must pass to the generated entry point. Read from the same
    /// checker table the parameters themselves come from, so the two
    /// cannot disagree.
    fn platform_entry_effects(&mut self, f: &FnDecl) -> Vec<String> {
        let checked_effects: Option<Vec<Ty>> = self
            .checked
            .fn_refs
            .get(&(self.file_idx, f.name.span))
            .and_then(|key| self.checked.fn_effects.get(key))
            .cloned();
        let mut out = Vec::new();
        match checked_effects {
            Some(tys) => {
                for ty in tys {
                    if is_throw_effect_ty(&ty) {
                        continue;
                    }
                    let rendered = self.kotlin_ty(&ty);
                    if self.is_platform_effect(Some(&ty), &rendered) && !out.contains(&rendered)
                    {
                        out.push(rendered);
                    }
                }
            }
            None => {
                for eff in f.effects.iter().flatten() {
                    if let EffectRef::Effect(r) | EffectRef::LocalEffect(r) = eff {
                        let rendered = self.emit_type_ref(r);
                        if self.is_platform_effect(None, &rendered)
                            && !out.contains(&rendered)
                        {
                            out.push(rendered);
                        }
                    }
                }
            }
        }
        out
    }

    fn emit_handler(&mut self, h: &'p HandlerDecl) -> String {
        if h.intrinsic {
            return self.emit_intrinsic_handler(h);
        }
        // [platform-handler] [kt-platform-handler] Nothing is emitted for a
        // platform handler: its class is the host's, in the module's
        // `platform/` companion, and the `use` site constructs it by name
        // (`emit_use`). The generated *interface* is the effect's, emitted
        // as any effect's is — which is what the host class implements.
        if h.platform {
            return String::new();
        }
        let saved = self.enter_generics(&h.generics);
        let mut generics = self.emit_generic_params(&h.generics);
        // [effect-handler-multi] Every face is a supertype: one class, one
        // piece of state, one interface per effect it implements. A member that
        // implements a same-named member of two faces needs **one** override
        // here — Kotlin lets a single method satisfy both interfaces, which is
        // why this side needs no forwarding.
        let of = h
            .of
            .clone()
            .iter()
            .map(|of| self.emit_type(of))
            .collect::<Vec<String>>()
            .join(", ");
        // [effect-handler-deps] [kt-effect-fusion] The handler's dependencies
        // arrive as **one fused value, built at the `use` site and stored**:
        // a handler holds its environment for its lifetime, so there is
        // nothing to rebuild per call (user decision 2026-09-14). The
        // parameter is unnamed in Salvo, so the field name is the emitter's.
        let deps = self.handler_dep_effects(h);
        let (dep_entries, dep_param, dep_bounds) = if deps.is_empty() {
            (Vec::new(), None, Vec::new())
        } else if self.handler_is_handle_dep(h) {
            // [use-local] [effect-handler-deps] The handle-dep form (user
            // decision 2026-09-20): one trailing constructor parameter per
            // dep, typed as the dep's **Has-accessor interface** — a stable
            // per-effect identity (unlike the per-file `__Fx_N` classes),
            // and what the `use` site's own fused value already implements.
            // On the JVM the captured reference *is* the handle; member
            // bodies reach the dep through the accessor property, and a
            // member calling a `[Console]` fn threads the captured carrier.
            let entries: Vec<EffectEntry> = deps
                .iter()
                .map(|rendered| {
                    let (_, prop) = self.has_iface(rendered);
                    let field = format!("__dep_{}", sanitize_instance(rendered));
                    EffectEntry {
                        ty: None,
                        rendered: rendered.clone(),
                        expr: format!("{field}.{prop}"),
                        fused: Some(field),
                    }
                })
                .collect();
            let params: Vec<String> = deps
                .iter()
                .map(|rendered| {
                    let (iface, _) = self.has_iface(rendered);
                    format!(
                        "private val __dep_{}: {iface}",
                        sanitize_instance(rendered)
                    )
                })
                .collect();
            (entries, Some(params.join(", ")), Vec::new())
        } else {
            // [kt-effect-fusion] The carrier is a **type parameter** bounded
            // by the Has-accessor interfaces, not a concrete `__Fx_N` class:
            // those are minted per *file*, so a handler in one module could
            // not be constructed from another (the `use` site's carrier of
            // the same shape is a different class with the same name — found
            // 2026-09-14 by std's `DefaultFs [RawFs]`, whose `use` lives in
            // the program). A bound reads the accessors and erases to one
            // class, exactly as it does for a fn [kt-effect-fusion].
            self.check_fusable(&deps);
            let mut bounds: Vec<String> = Vec::new();
            let entries: Vec<EffectEntry> = deps
                .iter()
                .map(|rendered| {
                    let (iface, prop) = self.has_iface(rendered);
                    bounds.push(format!("{HANDLER_FX} : {iface}"));
                    EffectEntry {
                        ty: None,
                        rendered: rendered.clone(),
                        expr: format!("{HANDLER_CARRIER}.{prop}"),
                        fused: Some(HANDLER_CARRIER.to_string()),
                    }
                })
                .collect();
            (
                entries,
                Some(format!("private val {HANDLER_CARRIER}: {HANDLER_FX}")),
                bounds,
            )
        };
        let ctor = {
            let mut params: Vec<String> = h
                .params
                .iter()
                .map(|p| {
                    format!(
                        "private val {}: {}",
                        kt_ident(&p.name.name),
                        self.emit_type(&p.ty)
                    )
                })
                .collect();
            // The carrier goes last, so the data parameters keep the
            // positions the Salvo source wrote.
            params.extend(dep_param);
            if params.is_empty() {
                String::new()
            } else {
                format!("({})", params.join(", "))
            }
        };
        // The carrier's type parameter joins the handler's own, and its
        // bounds go in a `where` clause after the supertype.
        let where_clause = if dep_bounds.is_empty() {
            String::new()
        } else {
            generics = match generics.strip_suffix('>') {
                Some(rest) => format!("{rest}, {HANDLER_FX}>"),
                None => format!("<{HANDLER_FX}>"),
            };
            format!(" where {}", dep_bounds.join(", "))
        };
        // [mixed-handler] [kt-mixed] A mixed handler's class is the servant
        // alone: it implements no face (the sync members live on the façade),
        // so it has no supertype.
        let mixed = self.handler_is_mixed(h);
        let mut out = if mixed {
            format!("\nclass {}{generics}{ctor}{where_clause} {{\n", h.name.name)
        } else {
            format!(
                "\nclass {}{generics}{ctor} : {of}{where_clause} {{\n",
                h.name.name
            )
        };
        for field in &h.state {
            let ty = self.emit_type(&field.ty);
            let init = match &field.default {
                Some(expr) => format!(" = {}", self.emit_expr(expr)),
                None => String::new(),
            };
            out.push_str(&format!(
                "    private var {}: {ty}{init}\n",
                kt_ident(&field.name.name)
            ));
        }
        // [kt-actor] [actor-self-send] [actor-replyto] The two generated
        // fields a handler of an `actor effect` carries, whichever way it is
        // bound (a handler is compiled once):
        //
        // * `__addr` — the actor this instance *is*, written by `__Actor_H`
        //   from the activation's `SalvoCtx`, and `null` when the instance was
        //   bound with `use` instead. That absence is the self-send's
        //   discriminator (user decision 2026-09-15, D5-a).
        // * `__parked` — the parked-continuation table, slot → `__Cont_E`. On
        //   the handler rather than on `__Actor_H` because the *mint* happens in
        //   a member body, which cannot see the actor class (D5-b).
        //
        // Neither may be `private`: Kotlin's class-level `private` is visible
        // only inside the class itself, and `__Actor_H` — a different class —
        // has to write one and read the other. (Rust needs no such care: its
        // privacy is per module, and both live in one.)
        let actor_handler = self.handler_is_actor(h) || mixed;
        if actor_handler {
            // [kt-mailbox] [actor-mailbox] The mailbox bound, computed where a
            // state field's initialiser is computed — which is what it is: an
            // expression over constructor parameters, evaluated once when the
            // handler is built. The spawn site reads it off the instance, so the
            // spawn line never says it and the arguments are evaluated once.
            let cap = self.mailbox_capacity_code(h);
            out.push_str(&format!("    internal val __mailboxCapacity: Int = {cap}\n"));
            out.push_str("    internal var __addr: Int? = null\n");
            if let Some(cont) = self.handler_cont_type(h) {
                out.push_str(&format!(
                    "    internal val __parked: MutableMap<Long, {cont}> = mutableMapOf()\n"
                ));
            }
        }
        let saved_deps = std::mem::replace(&mut self.handler_deps, dep_entries);
        let ctor_implicits: HashSet<String> = h
            .params
            .iter()
            .filter(|p| p.implicit)
            .map(|p| p.name.name.clone())
            .collect();
        let saved_ctor = std::mem::replace(&mut self.ctor_implicits, ctor_implicits);
        // [effect-member-overload] Member names come from the effect, not
        // from the handler's own spelling.
        let saved_member_effect = std::mem::replace(&mut self.handler_member_of, Some(h));
        // [actor-replyto] [actor-self-send] Both forms resolve against the
        // handler whose members these are.
        let saved_handler =
            std::mem::replace(&mut self.current_handler, Some(h.name.name.clone()));
        for f in &h.fns {
            // [mixed-handler] [kt-mixed] The split: send members are the
            // servant's, plain `fun`s (no interface declares them); sync
            // members are the façade's and are emitted there.
            if mixed {
                if !f.is_send {
                    continue;
                }
                out.push_str(&self.emit_fn_inner(f, "fun", 1, false));
                continue;
            }
            out.push_str(&self.emit_fn_inner(f, "override fun", 1, false));
        }
        self.current_handler = saved_handler;
        self.handler_member_of = saved_member_effect;
        self.ctor_implicits = saved_ctor;
        self.handler_deps = saved_deps;
        out.push_str("}\n");
        if mixed {
            // [actor-replyto] [kt-mixed] The parked-continuation classes,
            // handler-keyed like the servant's protocol.
            out.push_str(&self.emit_cont_classes(h));
            out.push_str(&self.emit_facade(h));
            out.push_str(&self.emit_mixed_actor_parts(h));
            self.generics = saved;
            return out;
        }
        // [actor-replyto] The parked-continuation classes, beside the handler
        // whose members they name.
        out.push_str(&self.emit_cont_classes(h));
        // [kt-actor] The body a `spawn` hands the scheduler, for a handler
        // that can be one.
        out.push_str(&self.emit_actor_body(h, &deps));
        self.generics = saved;
        out
    }

    /// [kt-actor] `__Actor_H`: the `SalvoActor` a spawn hands the
    /// scheduler. It owns the handler instance — an actor's state *is* the
    /// handler's — and `handle` casts the protocol's message and calls the
    /// member the class names.
    ///
    /// A **dependent** handler *stores* its fused environment
    /// ([kt-effect-fusion]), as a constructor argument whose type is the
    /// handler's extra type parameter. So an actor over one is generic in the
    /// same carrier and passes it nowhere: the handler already holds it, and
    /// the spawn site builds it. That is the whole of the difference here —
    /// where Rust has to own a provider and rebuild the view per activation
    /// ([rs-actor]), Kotlin's objects alias.
    fn emit_actor_body(&mut self, h: &HandlerDecl, deps: &[String]) -> String {
        // [effect-handler-multi] One protocol per face, each with its own
        // message class and its own dispatcher; one mailbox and one state serve
        // all of them.
        let faces: Vec<&'p EffectDecl> = self
            .handler_faces(h)
            .into_iter()
            .filter(|e| e.is_actor)
            .collect();
        if faces.len() != h.of.len() || faces.is_empty() || !h.generics.is_empty() {
            return String::new();
        }
        let sends: Vec<(&'p EffectDecl, Vec<(usize, &'p FnDecl)>)> = faces
            .iter()
            .map(|e| {
                let list: Vec<(usize, &'p FnDecl)> = e
                    .fns
                    .iter()
                    .enumerate()
                    .filter(|(_, f)| f.is_send)
                    .collect();
                (*e, list)
            })
            .collect();
        // [actor-effect-kind] Only an actor protocol gets an actor body.
        if sends.iter().all(|(_, list)| list.is_empty()) {
            return String::new();
        }
        self.needs_scheduler = true;
        let proc_name = actor_class_name(&h.name.name);
        // Per face: its message class and the dispatcher that runs its members.
        // One face keeps the bare `__dispatch`, which is what a
        // single-protocol actor has always emitted.
        let single = faces.len() == 1;
        let dispatchers: Vec<(String, String)> = faces
            .iter()
            .map(|e| {
                let msg = msg_class_name(&e.name.name);
                let dispatch = if single {
                    "__dispatch".to_string()
                } else {
                    format!("__dispatch{}", kt_ident(&e.name.name))
                };
                (msg, dispatch)
            })
            .collect();
        // The carrier's type parameter and its bounds, repeated from the
        // handler's own declaration: a `Counting<__Fx>` field needs the same
        // `where` clause the class was declared with.
        let (proc_generics, handler_ty, where_clause) = if deps.is_empty() {
            (String::new(), h.name.name.clone(), String::new())
        } else {
            let bounds: Vec<String> = deps
                .iter()
                .map(|rendered| format!("{HANDLER_FX} : {}", self.has_iface(rendered).0))
                .collect();
            (
                format!("<{HANDLER_FX}>"),
                format!("{}<{HANDLER_FX}>", h.name.name),
                format!(" where {}", bounds.join(", ")),
            )
        };
        let mut out = format!(
            "\nclass {proc_name}{proc_generics}(private val handler: {handler_ty}) : \
             salvo.SalvoActor{where_clause} {{\n    \
             override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {{\n        \
             handler.__addr = ctx.addr\n"
        );
        if single {
            let (msg, dispatch) = &dispatchers[0];
            out.push_str(&format!("        {dispatch}(msg as {msg})\n    }}\n"));
        } else {
            // [effect-handler-multi] One mailbox carries every face's messages,
            // so the delivery asks which protocol this one belongs to. The
            // `else` is unreachable: the runtime only ever delivers what a
            // sender built from one of these protocols.
            out.push_str("        when (msg) {\n");
            for (msg, dispatch) in &dispatchers {
                out.push_str(&format!("            is {msg} -> {dispatch}(msg)\n"));
            }
            out.push_str(
                "            else -> error(\"a message of one of this actor's protocols\")\n        }\n    }\n",
            );
        }
        for ((effect, list), (msg, dispatch)) in sends.iter().zip(&dispatchers) {
            out.push_str(&format!(
                "\n    private fun {dispatch}(m: {msg}) {{\n        when (m) {{\n"
            ));
            for (i, f) in list {
                let member = salvo_core::effect_member_name(effect, *i);
                let variant = msg_variant_name(&member);
                let args: Vec<String> = f
                    .params
                    .iter()
                    .filter(|p| !p.implicit)
                    .map(|p| format!("m.{}", p.name.name))
                    .collect();
                out.push_str(&format!(
                    "            is {msg}.{variant} -> handler.{member}({})\n",
                    args.join(", ")
                ));
            }
            out.push_str("        }\n    }\n");
        }
        // [actor-replyto] The other half of the table: the slot names the
        // parked continuation, and its subclass says which member to resume
        // and therefore what the answer casts to — its *trailing* parameter's
        // type.
        match self.handler_cont_type(h) {
            None => out.push_str(
                "\n    override fun resume(ctx: salvo.SalvoCtx, slot: Long, value: Any?) {\n        \
                 error(\"this protocol has no continuation targets\")\n    }\n",
            ),
            Some(cont) => {
                out.push_str(
                    "\n    override fun resume(ctx: salvo.SalvoCtx, slot: Long, value: Any?) {\n        \
                     handler.__addr = ctx.addr\n        \
                     // A reply whose continuation is gone: nothing to run.\n        \
                     val c = handler.__parked.remove(slot) ?: return\n        \
                     when (c) {\n",
                );
                for (effect, list) in &sends {
                    for (i, f) in list {
                        let fixed: Vec<&Param> =
                            f.params.iter().filter(|p| !p.implicit).collect();
                        if fixed.is_empty() {
                            continue;
                        }
                        let member = salvo_core::effect_member_name(effect, *i);
                        let variant = msg_variant_name(&member);
                        let mut args: Vec<String> = fixed[..fixed.len() - 1]
                            .iter()
                            .map(|p| format!("c.{}", p.name.name))
                            .collect();
                        let answer_ty = self.emit_type(&fixed[fixed.len() - 1].ty);
                        args.push(format!("value as {answer_ty}"));
                        out.push_str(&format!(
                            "            is {cont}.{variant} -> handler.{member}({})\n",
                            args.join(", ")
                        ));
                    }
                }
                out.push_str("        }\n    }\n");
            }
        }
        out.push_str("}\n");
        out
    }

    /// [kt-actor] [actor-effect-kind] Does `h` implement an **`actor
    /// effect`**? The gate for the two generated fields and for the actor
    /// class: a plain effect is never actor-backed.
    fn handler_is_actor(&self, h: &HandlerDecl) -> bool {
        // [effect-handler-multi] Every face of a handler is of one kind, so any
        // of them answers.
        h.of.iter().any(|of| {
            type_base_name(of)
                .and_then(|n| self.symbols.effects.get(n))
                .is_some_and(|e| e.is_actor)
        })
    }

    /// [actor-replyto] [effect-handler-multi] The `__Cont_H` class of the
    /// handler `h` — named after the **handler**, not the effect, because a
    /// mint is lexical: `replyto k(…)` targets a member of the handler it is
    /// written in, and with several faces those members come from several
    /// protocols. [mixed-handler] [kt-mixed] A mixed servant parks too, keyed
    /// on its own `send fn` members — no face declares them. `None` when no
    /// member could be a continuation target (every send member is
    /// parameterless, so no answer could be carried), in which case no
    /// `__parked` table is emitted either.
    fn handler_cont_type(&self, h: &HandlerDecl) -> Option<String> {
        if self.handler_is_mixed(h) {
            if self.mixed_cont_targets(h).is_empty() {
                return None;
            }
            return Some(cont_class_name(&h.name.name));
        }
        if !self.handler_is_actor(h) {
            return None;
        }
        if self.cont_targets(h).is_empty() {
            return None;
        }
        Some(cont_class_name(&h.name.name))
    }

    /// [mixed-handler] [kt-mixed] The mixed servant's members a continuation
    /// could target: a handler-local `send fn` with at least one parameter
    /// (the trailing one is the answer) — the handler-keyed twin of
    /// [`Emitter::cont_targets`], named as `__Msg_H`'s variants are.
    fn mixed_cont_targets<'h>(&self, h: &'h HandlerDecl) -> Vec<&'h FnDecl> {
        h.fns
            .iter()
            .filter(|f| f.is_send && f.params.iter().any(|p| !p.implicit))
            .collect()
    }

    /// [effect-handler-multi] The effects a handler implements, as
    /// declarations and in declaration order.
    fn handler_faces(&self, h: &HandlerDecl) -> Vec<&'p EffectDecl> {
        h.of
            .iter()
            .filter_map(|of| type_base_name(of))
            .filter_map(|n| self.symbols.effects.get(n).copied())
            .collect()
    }

    /// [effect-handler-multi] The faces that declare the member `f`
    /// implements, each with the member's index in that face.
    fn member_faces(&self, h: &HandlerDecl, f: &FnDecl) -> Vec<(&'p EffectDecl, usize)> {
        salvo_core::handler_member_faces(&self.handler_faces(h), f)
    }

    /// [actor-replyto] The handler's members a continuation could target: a
    /// `send fn` with at least one parameter (the trailing one is the answer),
    /// paired with the face that declares it.
    fn cont_targets(&self, h: &HandlerDecl) -> Vec<(&'p EffectDecl, usize)> {
        let mut targets: Vec<(&'p EffectDecl, usize)> = Vec::new();
        for f in &h.fns {
            if !f.is_send || !f.params.iter().any(|p| !p.implicit) {
                continue;
            }
            if let Some(found) = self.member_faces(h, f).into_iter().next() {
                if found.0.is_actor {
                    targets.push(found);
                }
            }
        }
        targets
    }

    /// [effect-handler-deps] The effects `h` declares as dependencies,
    /// rendered, in declaration order — the handler's own effect list since
    /// 2026-09-14 (user decision). No filtering: every entry is a
    /// dependency, and the checker has already refused `use` and `Throw`.
    fn handler_dep_effects(&mut self, h: &HandlerDecl) -> Vec<String> {
        let refs: Vec<TypeRef> = h
            .effects
            .iter()
            .flatten()
            .filter_map(|e| match e {
                // [effect-local] Dep locality is the checker's business; at
                // the fusion layer a `local E` dep threads like any other.
                EffectRef::Effect(r) | EffectRef::LocalEffect(r) => Some(r.clone()),
                EffectRef::Use(_) => None,
                // [actor-spawn-effect] A capability, not an effect type: no
                // handler parameter is threaded for it.
                EffectRef::Spawn(_) => None,
            })
            .collect();
        refs.iter().map(|r| self.emit_type_ref(r)).collect()
    }

    /// An `intrinsic handler` [backend-intrinsic]: a std handler whose
    /// members this backend implements directly. The signatures come from
    /// the *effect* it implements (the handler declaration is bodyless),
    /// and the bodies from [`crate::intrinsics::handler_member`].
    ///
    /// Like every handler it emits as a class instantiated at its `use`
    /// site — `object` was an artifact of `StdOutConsole` being stateless.
    fn emit_intrinsic_handler(&mut self, h: &HandlerDecl) -> String {
        // [effect-handler-multi] One face, like a platform handler's: the
        // members are the backend's, and the checker refuses a second.
        let of = self.emit_type(&h.of[0]);
        let Some(effect) = type_base_name(&h.of[0])
            .and_then(|n| self.symbols.effects.get(n))
            .copied()
        else {
            self.error(format!(
                "intrinsic handler `{}` implements `{of}`, which is not a declared \
                 effect",
                h.name.name
            ));
            return String::new();
        };
        let ctor = if h.params.is_empty() {
            String::new()
        } else {
            let params: Vec<String> = h
                .params
                .iter()
                .map(|p| {
                    format!(
                        "private val {}: {}",
                        kt_ident(&p.name.name),
                        self.emit_type(&p.ty)
                    )
                })
                .collect();
            format!("({})", params.join(", "))
        };
        let mut out = format!("\nclass {}{ctor} : {of} {{\n", h.name.name);
        for (i, member) in effect.fns.iter().enumerate() {
            let params = self.emit_member_param_list_with_implicits(member);
            let ret = self.emit_return_type(member.return_type.as_ref());
            let arg_names: Vec<String> = member
                .params
                .iter()
                .map(|p| kt_ident(&p.name.name))
                .collect();
            let Some(body) = crate::intrinsics::handler_member(
                &h.name.name,
                &member.name.name,
                &arg_names,
            ) else {
                self.error(format!(
                    "intrinsic handler `{}` has no kotlin lowering for member `{}`",
                    h.name.name, member.name.name
                ));
                continue;
            };
            out.push_str(&format!(
                "    override fun {}({params}){ret} {{\n",
                self.member_name(effect, i)
            ));
            // A value-returning member returns its body's value; `run`
            // makes multi-line bodies (statements + final expression) work
            // unchanged [kt-handler-template-return].
            if ret.is_empty() {
                for line in body.lines() {
                    out.push_str(&format!("        {line}\n"));
                }
            } else {
                out.push_str("        return run {\n");
                for line in body.lines() {
                    out.push_str(&format!("            {line}\n"));
                }
                out.push_str("        }\n");
            }
            out.push_str("    }\n");
        }
        out.push_str("}\n");
        out
    }

    /// [effect-member-overload] The emitted name of an effect member: the
    /// declared name, unless the effect *overloads* it, in which case every
    /// occurrence after the first is suffixed. Kotlin *could* overload — and
    /// that is exactly the problem: it would resolve by Kotlin's type
    /// lattice, not Salvo's, the hazard [kt-fn-mangling] states for top-level
    /// fns. The rule is `salvo_core`'s, so the interface, every handler
    /// override, the skeletons and the call sites agree — and so the Rust
    /// backend picks the same names.
    fn member_name(&self, effect: &EffectDecl, idx: usize) -> String {
        kt_ident(&salvo_core::effect_member_name(effect, idx))
    }

    /// [effect-member-overload] The emitted name of a *handler's* member: the
    /// name of the effect member it implements, matched by name and written
    /// parameter types (`salvo_core::effect_member_index`).
    fn handler_member_name(&mut self, h: &HandlerDecl, f: &FnDecl) -> String {
        match self.member_faces(h, f).into_iter().next() {
            Some((e, i)) => self.member_name(e, i),
            None => kt_ident(&f.name.name),
        }
    }

    /// [effect-member-overload] The emitted name of the member a *call*
    /// resolved to: the checker records which overload
    /// (`Checked::effect_member_calls`), and where it did not the name is
    /// declared once, so the name itself answers.
    fn called_member_name(&mut self, effect: &str, name: &str, span: Span) -> String {
        let Some(decl) = self.symbols.effects.get(effect).copied() else {
            return kt_ident(name);
        };
        match self.checked.effect_member_calls.get(&(self.file_idx, span)) {
            Some(&idx) => self.member_name(decl, idx),
            None => match salvo_core::effect_members_named(decl, name).as_slice() {
                [_] | [] => kt_ident(name),
                _ => {
                    self.error(format!(
                        "internal: `{name}` is overloaded on effect `{effect}` and \
                         the checker recorded no resolution for this call"
                    ));
                    kt_ident(name)
                }
            },
        }
    }

    fn emit_fn(&mut self, f: &FnDecl) -> String {
        self.emit_fn_inner(f, "fun", 0, true)
    }

    /// [cmp-auto] [kt-cmp-groups] A canonical generated by a `default`
    /// obligation: the host's own operation, wrapped in the fn the rest of the
    /// compiler resolved, so nothing else in the backend learns that `default`
    /// exists.
    ///
    /// `cmp` goes through the runtime comparator rather than `compareTo`
    /// directly, for the reason every ordering on this backend does: a `List`
    /// or a tuple field is not `Comparable` on the JVM, and `String.compareTo`
    /// is the wrong order [kt-ordered]. The struct's own generated `compareTo`
    /// is what it lands on, which is what keeps this member and the type's
    /// ordering the same thing.
    fn emit_structural_fn(&mut self, f: &FnDecl) -> String {
        let member = f.name.name.clone();
        let param = f.params.first().cloned();
        let Some(param) = param else {
            self.error(format!("internal: structural `{member}` has no parameter"));
            return String::new();
        };
        let saved = self.enter_generics(&f.generics);
        let generics = self.emit_generic_params(&f.generics);
        let ty = self.emit_type(&param.ty);
        self.generics = saved;
        let name = self.kotlin_fn_name(f);
        match member.as_str() {
            "cmp" => {
                self.needs_compare = true;
                format!(
                    "\nfun {generics}{name}(a: {ty}, b: {ty}): Int {{\n    return salvo.__salvoCompare(a, b)\n}}\n"
                )
            }
            "eq" => format!(
                "\nfun {generics}{name}(a: {ty}, b: {ty}): Boolean {{\n    return a == b\n}}\n"
            ),
            // [cmp-hash-values] This host's digest; Rust's is its own.
            _ => format!(
                "\nfun {generics}{name}(value: {ty}): Long {{\n    return value.hashCode().toLong()\n}}\n"
            ),
        }
    }

    /// A predicate qualifier's functions (`qualifies`) become top-level
    /// Kotlin functions named `{Qualifier}_{fn}` (qualifiers themselves are
    /// erased; only the predicates survive as code).
    fn emit_qualifier(&mut self, q: &QualifierDecl) -> String {
        let mut out = String::new();
        for f in &q.fns {
            if f.body.is_none() {
                continue;
            }
            let mut renamed = f.clone();
            renamed.name.name = format!("{}_{}", q.name.name, f.name.name);
            renamed.generics = q
                .generics
                .iter()
                .cloned()
                .chain(f.generics.iter().cloned())
                .collect();
            out.push_str(&self.emit_fn_inner(&renamed, "fun", 0, true));
        }
        out
    }

    /// Emits a function declaration. `top_level` functions get effect
    /// parameters; handler methods (`override fun`) do not.
    fn emit_fn_inner(&mut self, f: &FnDecl, kw: &str, indent: usize, top_level: bool) -> String {
        // [cmp-auto] A generated structural member has no body to emit: its
        // body *is* the host's own operation.
        if f.structural {
            return self.emit_structural_fn(f);
        }
        let Some(body) = &f.body else {
            return String::new();
        };
        let saved_generics = self.enter_generics(&f.generics);
        let saved_env = std::mem::take(&mut self.effect_env);
        let saved_mutated = std::mem::take(&mut self.mutated);
        collect_mutated(body, &mut self.mutated);
        // Effect parameters and `use` variables must not collide with the
        // fn's own parameters or locals [kt-effect-params].
        let saved_taken = std::mem::take(&mut self.taken_names);
        for p in &f.params {
            self.taken_names.insert(p.name.name.clone());
        }
        collect_declared(body, &mut self.taken_names);

        let is_main = top_level && f.name.name == "main";
        let mut generics = self.emit_generic_params(&f.generics);
        // Effect dependencies become leading parameters [kt-effect-params],
        // sourced from the checker's lowered effect list when available
        // (checker-`Ty` keys; the AST rendering is the unchecked
        // fallback).
        let mut params: Vec<String> = Vec::new();
        let mut where_clause = String::new();
        let mut body_prelude = String::new();
        // [effect-handler-deps] A handler member reaches its handler's
        // dependencies through the **stored fused value** the `use` site
        // built ([kt-effect-fusion]): the entries already point into it, so a
        // member body needs no prelude and no per-call allocation.
        for entry in self.handler_deps.clone() {
            self.effect_env.push(entry);
        }
        let provided: Vec<String> =
            self.effect_env.iter().map(|e| e.rendered.clone()).collect();
        // [platform-effect] `main` takes only its *platform* effects: the host
        // supplies those by calling the entry point, and everything else it
        // needs is registered inside it with `use`.
        if !is_main || self.declares_platform_effect(f) {
            let keep = |emitter: &Self, ty: Option<&Ty>, rendered: &str| {
                !is_main || emitter.is_platform_effect(ty, rendered)
            };
            let checked_effects: Option<Vec<Ty>> = self
                .checked
                .fn_refs
                .get(&(self.file_idx, f.name.span))
                .and_then(|key| self.checked.fn_effects.get(key))
                .cloned();
            let mut effects: Vec<(Option<Ty>, String)> = Vec::new();
            match checked_effects {
                Some(tys) => {
                    for ty in tys {
                        // [kt-throw-signal] Throwing needs no handler: the
                        // JVM unwinds to the innermost `try`, so `Throw`
                        // never becomes a parameter.
                        if is_throw_effect_ty(&ty) {
                            continue;
                        }
                        let rendered = self.kotlin_ty(&ty);
                        if provided.contains(&rendered)
                            || effects.iter().any(|(_, r)| *r == rendered)
                        {
                            continue;
                        }
                        if !keep(self, Some(&ty), &rendered) {
                            continue;
                        }
                        effects.push((Some(ty), rendered));
                    }
                }
                None => {
                    for eff in f.effects.iter().flatten() {
                        if let EffectRef::Effect(r) | EffectRef::LocalEffect(r) = eff {
                            if r.name.name == salvo_core::THROW_EFFECT {
                                continue;
                            }
                            let rendered = self.emit_type_ref(r);
                            if provided.contains(&rendered)
                                || effects.iter().any(|(_, r)| *r == rendered)
                            {
                                continue;
                            }
                            if !keep(self, None, &rendered) {
                                continue;
                            }
                            effects.push((None, rendered));
                        }
                    }
                }
            }
            if self.fusion && is_main && self.declares_platform_effect(f) {
                // [kt-platform-host] The host constructs one implementation
                // per platform effect and calls the entry with them, so the
                // entry keeps per-effect parameters — the one ABI the fusion
                // cannot reshape — and opens by combining them into a fused
                // value [kt-effect-fusion].
                if !effects.is_empty() {
                    let rendered: Vec<String> =
                        effects.iter().map(|(_, r)| r.clone()).collect();
                    let (class, _props, order) = self.emit_fx_class(&rendered);
                    let var = self.unique_name("__fx".to_string());
                    let mut args: Vec<String> = Vec::new();
                    for (ty, rendered) in effects {
                        let param = self.unique_name(effect_param_name(&rendered));
                        params.push(format!("{param}: {rendered}"));
                        args.push(param.clone());
                        self.effect_env.push(EffectEntry {
                            ty,
                            rendered,
                            expr: param,
                            fused: Some(var.clone()),
                        });
                    }
                    // The fused class takes its effects in canonical order.
                    let args: Vec<String> =
                        order.iter().map(|i| args[*i].clone()).collect();
                    let pad_body = "    ".repeat(indent + 1);
                    body_prelude =
                        format!("{pad_body}val {var} = {class}({})\n", args.join(", "));
                }
            } else if self.fusion {
                // [kt-effect-fusion] One fused value for *any* number of
                // effects, single included (user decision 2026-09-14:
                // consistency over a special case): a `__Fx` type parameter
                // bounded by the Has-accessor interfaces, which erasure
                // makes a single emitted function.
                if !effects.is_empty() {
                    let var = self.unique_name("__fx".to_string());
                    let mut bounds: Vec<String> = Vec::new();
                    for (ty, rendered) in effects {
                        let (iface, prop) = self.has_iface(&rendered);
                        bounds.push(format!("__Fx : {iface}"));
                        self.effect_env.push(EffectEntry {
                            ty,
                            rendered,
                            expr: format!("{var}.{prop}"),
                            fused: Some(var.clone()),
                        });
                    }
                    generics = match generics.strip_suffix('>') {
                        Some(rest) => format!("{rest}, __Fx>"),
                        None => "<__Fx>".to_string(),
                    };
                    where_clause = format!(" where {}", bounds.join(", "));
                    params.push(format!("{var}: __Fx"));
                }
            } else {
                for (ty, rendered) in effects {
                    let param = self.unique_name(effect_param_name(&rendered));
                    self.effect_env.push(EffectEntry {
                        ty,
                        rendered: rendered.clone(),
                        expr: param.clone(),
                        fused: None,
                    });
                    params.push(format!("{param}: {rendered}"));
                }
            }
        }
        for p in &f.params {
            if p.implicit {
                continue; // appended below, in the checker's order
            }
            let ty = self.emit_type(&p.ty);
            // [kt-variadic] A variadic parameter is an ordinary `Array<T>`
            // parameter, not a `vararg`. Kotlin's `vararg` of a *primitive*
            // element type is an `IntArray`/`DoubleArray`/… rather than an
            // `Array<Int>`, and those are unrelated types on the JVM: nothing
            // generic accepts one, so `iter(ns)` inside the body failed and an
            // `Array<Int>` could not be spread into the position. Since both
            // sides of every call are generated, the `vararg` sugar bought
            // nothing and cost a representation split.
            params.push(format!("{}: {ty}", kt_ident(&p.name.name)));
        }
        // [implicit-param] Implicit parameters are ordinary trailing
        // parameters of fn type: the caller passes what resolution found, so
        // nothing about them survives into the target language.
        let own_implicits = self.implicits_of(f);
        let saved_implicits = std::mem::replace(&mut self.implicits, own_implicits);
        for imp in &self.implicits.clone() {
            let ty = self.kotlin_ty(&imp.ty);
            params.push(format!("{}: {ty}", kt_ident(&imp.name)));
        }

        let ret = if is_main {
            String::new()
        } else {
            self.emit_return_type(f.return_type.as_ref())
        };
        // [kt-none-unit] An empty rendered return type *is* Kotlin `Unit`.
        let saved_ret_unit = std::mem::replace(&mut self.ret_is_unit, ret.is_empty());

        let pad = "    ".repeat(indent);
        let name = if is_main {
            // [platform-effect] A `main` needing platform effects is not the
            // program's entry point any more — the *host's* `main` is, and
            // it calls this after constructing the implementations. Renaming
            // it is what makes the JVM pick the host's entry rather than
            // this one, whose signature it could not satisfy.
            if self.declares_platform_effect(f) {
                SALVO_ENTRY.to_string()
            } else {
                "main".to_string()
            }
        } else if top_level {
            self.kotlin_fn_name(f)
        } else {
            // [effect-member-overload] A handler member implements one
            // *overload* of its effect's member, and the interface names the
            // overloads apart, so the override has to use the same name.
            match self.handler_member_of {
                Some(h) => self.handler_member_name(h, f),
                None => kt_ident(&f.name.name),
            }
        };
        // [kt-suppress-cast] The body is emitted before the signature line is
        // assembled, so a noted unchecked cast (an erased payload read cast to
        // a generic or parameterized type) can put `@Suppress("UNCHECKED_CAST")`
        // on the function — kotlinc's warning would otherwise land in code the
        // author cannot edit. Saved and restored because handler members are
        // emitted inside an enclosing file walk, not because fns nest.
        let saved_cast = std::mem::replace(&mut self.unchecked_cast, false);
        let body_out = {
            let saved_ctx = self.stmt_ctx;
            self.stmt_ctx = StmtCtx::Normal;
            let rendered = self.emit_block_stmts(body, indent + 1);
            self.stmt_ctx = saved_ctx;
            rendered
        };
        let suppress = if self.unchecked_cast {
            format!("{pad}@Suppress(\"UNCHECKED_CAST\", \"USELESS_CAST\")\n")
        } else {
            String::new()
        };
        self.unchecked_cast = saved_cast;
        let mut out = format!(
            "\n{suppress}{pad}{kw}{generics} {name}({}){ret}{where_clause} {{\n",
            params.join(", ")
        );
        // [kt-effect-fusion] A dyn-boundary fn (platform `main`, a handler
        // member with dependencies) opens by combining its per-effect
        // values into the fused carrier the rest of the body threads.
        out.push_str(&body_prelude);
        out.push_str(&body_out);
        out.push_str(&format!("{pad}}}\n"));

        self.generics = saved_generics;
        self.effect_env = saved_env;
        self.mutated = saved_mutated;
        self.taken_names = saved_taken;
        self.implicits = saved_implicits;
        self.ret_is_unit = saved_ret_unit;
        out
    }


    /// [interp-to-str] Wraps an interpolated value in the `to_str` the
    /// checker resolved for it, or returns it unchanged when it renders
    /// natively.
    fn apply_interp_to_str(&mut self, expr: &Expr, code: String) -> String {
        let key = (self.file_idx, expr.span());
        // [interp-struct] A struct with no `to_str` of its own renders
        // field-wise, in the language's format rather than the JVM's
        // `toString` (a data class prints `Person(name=ann)`).
        if let Some(name) = self.checked.interp_struct.get(&key).cloned() {
            if let Some(fields) = self.struct_field_names(&name) {
                let inner: Vec<String> = fields
                    .iter()
                    .map(|f| format!("{f}: ${{{code}.{}}}", kt_ident(f)))
                    .collect();
                return format!("\"{name} {{ {} }}\"", inner.join(", "));
            }
        }
        let Some(fn_key) = self.checked.interp_to_str.get(&key).copied() else {
            return code;
        };
        let Some(decl) = self.fn_by_key(fn_key) else {
            self.error("the `to_str` this interpolation resolved to is not available");
            return code;
        };
        if decl.intrinsic {
            let recv = decl.params.first().and_then(|p| type_base_name(&p.ty));
            return match crate::intrinsics::fn_call(
                &decl.name.name,
                recv,
                &[code.clone()],
                &[],
                None,
                None,
            ) {
                Some(rendered) => rendered,
                None => {
                    self.error(
                        "`to_str` for this type has no lowering on the Kotlin backend"
                            .to_string(),
                    );
                    code
                }
            };
        }
        format!("{}({code})", self.kotlin_fn_name(decl))
    }


    /// [interp-struct] The field names of a declared struct, in order.
    fn struct_field_names(&self, name: &str) -> Option<Vec<String>> {
        for module in self.program.modules.iter() {
            for item in &module.items {
                if let Item::Struct(decl) = item {
                    if decl.name.name == name {
                        return Some(
                            decl.fields.iter().map(|f| f.name.name.clone()).collect(),
                        );
                    }
                }
            }
        }
        None
    }

    /// The generated Kotlin imports of one file [kt-imports]: a wildcard
    /// import per foreign emitted module whose names the file uses, plus
    /// Kotlin alias imports for every aliased Salvo import of an item
    /// that exists as a Kotlin symbol (an `intrinsic fn` has none: it is
    /// lowered inline).
    /// Aliased fns get one alias import per overload *symbol*: a mangled
    /// qualified overload [kt-qual-mangling] is its own Kotlin name, and
    /// its alias carries the same `__Qual` suffix so aliased call sites
    /// can address it.
    fn generate_imports(
        &mut self,
        ast: &Module,
        own: &ModulePath,
        scope: &salvo_core::ModuleScope<'_>,
        emitted_modules: &HashSet<&ModulePath>,
    ) -> BTreeSet<String> {
        let mut imports = BTreeSet::new();
        for name in salvo_core::reach::used_names(ast) {
            for module in scope.name_origins.get(name).into_iter().flatten() {
                if *module != own && emitted_modules.contains(*module) {
                    imports.insert(format!("import {}.*", kotlin_package(module)));
                }
            }
        }
        // [iter-protocol] A pass-driving loop is *synthesized*, and it spells
        // the protocol's own names — `Finished`, in the arm test of each step —
        // which the source need never mention: `for x in pass` is the whole of
        // it. So a file that drives a pass imports the protocol's module too.
        let drives_or_mints = self
            .checked
            .for_drivers
            .keys()
            .any(|(file, _)| *file == self.file_idx);
        if drives_or_mints {
            for module in scope.name_origins.get("Finished").into_iter().flatten() {
                if *module != own && emitted_modules.contains(*module) {
                    imports.insert(format!("import {}.*", kotlin_package(module)));
                }
            }
        }
        for item in &ast.items {
            let Item::Import(imp) = item else { continue };
            let (Some(alias), Some(item_name)) = (&imp.alias, imp.path.last()) else {
                continue;
            };
            // Only items with a Kotlin symbol can be alias-imported: fns
            // with bodies, structs, effects, handlers. Inlined intrinsics,
            // type aliases, and qualifiers resolve without one.
            let alias_name = alias.name.as_str();
            let bodied_fns: Vec<&FnDecl> = scope
                .fns
                .get(alias_name)
                .into_iter()
                .flatten()
                .filter(|e| e.decl.body.is_some())
                .map(|e| e.decl)
                .collect();
            let has_symbol = !bodied_fns.is_empty()
                || scope.structs.contains_key(alias_name)
                || scope.effects.contains_key(alias_name)
                || scope.handlers.contains_key(alias_name);
            if !has_symbol {
                continue;
            }
            let Some(module) = scope
                .name_origins
                .get(alias_name)
                .and_then(|ms| ms.first())
            else {
                continue;
            };
            if !emitted_modules.contains(*module) {
                continue;
            }
            let pkg = kotlin_package(module);
            if bodied_fns.is_empty() {
                imports.insert(format!(
                    "import {pkg}.{} as {}",
                    kt_ident(&item_name.name),
                    kt_ident(alias_name)
                ));
                continue;
            }
            for decl in bodied_fns {
                let kotlin_name = self.kotlin_fn_name(decl);
                let aliased = aliased_symbol(&kotlin_name, &decl.name.name, alias_name);
                imports.insert(format!("import {pkg}.{kotlin_name} as {aliased}"));
            }
        }
        imports
    }

    /// The Kotlin name for a top-level fn [kt-fn-mangling]. Salvo resolves
    /// overloads in the *checker*; Kotlin must not get a second opinion,
    /// and it will take one whenever several overloads share a name —
    /// because Kotlin's own resolution follows Kotlin's type lattice, not
    /// Salvo's. Two Salvo types with no subtype relation at all can map
    /// onto Kotlin types that have one (`Iter<T>` → `Iterable<T>` and
    /// `List<T>` → `List<T>`, and Kotlin's `List` *is* an `Iterable`), so
    /// a call the checker resolved to the `Iter` overload silently
    /// re-resolved to the `List` one — a `map(xs.iter(), f)` inside the
    /// `List` overload became an infinite recursion. Qualifier erasure
    /// [qual-erasure] is the same hazard by another route.
    ///
    /// So the name is made unique whenever a name has more than one
    /// emitted overload, by exactly the rule the Rust backend uses
    /// [rs-fn-mangling] (which has to do this because Rust has no
    /// overloading at all): the `__Qual` suffix where it disambiguates,
    /// then positional suffixes for whatever still collides. Both backends
    /// therefore pick the same names.
    fn kotlin_fn_name(&mut self, decl: &FnDecl) -> String {
        let name = decl.name.name.clone();
        let overloads: Vec<&FnDecl> = match self.symbols.fns.get(name.as_str()) {
            // [cmp-auto] A generated structural member has no body and is
            // still emitted, so it takes part in mangling like any overload —
            // without this, three `auto Eq` structs would all emit `eq`.
            Some(o) if o.len() > 1 => o
                .iter()
                .filter(|f| f.body.is_some() || f.structural)
                .copied()
                .collect(),
            _ => return kt_ident(&name),
        };
        if overloads.len() <= 1 {
            return kt_ident(&name);
        }
        // Qualifier-suffix pass: an overload whose erased signature
        // collides with another's gets its qualifier suffix
        // [kt-qual-mangling].
        let mangled: Vec<String> = overloads
            .iter()
            .map(|f| {
                let suffix = qual_suffix(f);
                if suffix.is_empty() {
                    return name.clone();
                }
                let mine = self.erased_sig(f);
                let collides = overloads.iter().any(|other| {
                    !std::ptr::eq(*other as *const FnDecl, *f as *const FnDecl)
                        && self.erased_sig(other) == mine
                });
                if collides {
                    format!("{name}__{suffix}")
                } else {
                    name.clone()
                }
            })
            .collect();
        // Positional pass over what still collides.
        let mut final_names = mangled.clone();
        for i in 0..final_names.len() {
            let dup_before = mangled[..i].iter().filter(|m| **m == mangled[i]).count();
            if dup_before > 0 {
                final_names[i] = format!("{}__{}", mangled[i], dup_before + 1);
            }
        }
        for (f, final_name) in overloads.iter().zip(&final_names) {
            if std::ptr::eq(*f as *const FnDecl, decl as *const FnDecl) {
                return kt_ident(final_name);
            }
        }
        kt_ident(&name)
    }

    /// The erased (Kotlin) parameter signature of a fn, for collision
    /// detection between qualifier-based overloads.
    fn erased_sig(&mut self, decl: &FnDecl) -> String {
        let saved = self.enter_generics(&decl.generics);
        let sig = decl
            .params
            .iter()
            .map(|p| self.emit_type(&p.ty))
            .collect::<Vec<_>>()
            .join(",");
        self.generics = saved;
        sig
    }

    fn emit_param_list(&mut self, params: &[Param]) -> String {
        params
            .iter()
            .filter(|p| !p.implicit)
            .map(|p| {
                // [kt-variadic] An ordinary array parameter — see `emit_fn`.
                format!("{}: {}", kt_ident(&p.name.name), self.emit_type(&p.ty))
            })
            .collect::<Vec<_>>()
            .join(", ")
    }


    fn emit_generic_params(&self, generics: &[Ident]) -> String {
        if generics.is_empty() {
            String::new()
        } else {
            format!(
                "<{}>",
                generics
                    .iter()
                    .map(|g| g.name.clone())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
    }

    fn enter_generics(&mut self, generics: &[Ident]) -> HashSet<String> {
        let saved = self.generics.clone();
        for g in generics {
            self.generics.insert(g.name.clone());
        }
        saved
    }

    /// [kt-suppress-cast] Notes a cast of an erased (`Any?`) payload, which
    /// kotlinc flags one of two ways: **unchecked** where the target's
    /// arguments are erased (a bare generic parameter, or any parameterized
    /// type), and **useless** where kotlinc's own smart cast already gave
    /// the payload that type — which it does for a concrete arm of a
    /// concretely-typed union. Both are cosmetic and both are the emitter's
    /// to silence: generated code must stay warning-free, and the author
    /// cannot edit it.
    fn note_payload_cast(&mut self) {
        self.unchecked_cast = true;
    }

    fn emit_return_type(&mut self, ty: Option<&Type>) -> String {
        match ty {
            None => String::new(),
            Some(Type::Named { base, .. }) if base.name.name == "None" => String::new(),
            Some(t) => format!(": {}", self.emit_type(t)),
        }
    }

    // ================= types =================

    fn emit_type(&mut self, ty: &Type) -> String {
        match ty {
            Type::Named { qualifiers, base } => self.emit_named_type(qualifiers, base),
            Type::Nullable { inner, .. } => format!("{}?", self.emit_type(inner)),
            Type::Array { elem, .. } => format!("Array<{}>", self.emit_type(elem)),
            Type::Union { arms, .. } => self.emit_union_type(arms),
            Type::Fn {
                params,
                ret,
                effects,
                ..
            } => {
                // [fn-effects] The effects are threaded in, so they are part
                // of the Kotlin function type's parameter list.
                let mut ps: Vec<String> = Vec::new();
                for eff in effects.iter().flatten() {
                    if let EffectRef::Effect(r) | EffectRef::LocalEffect(r) = eff {
                        ps.push(self.emit_type_ref(r));
                    }
                }
                ps.extend(params.iter().map(|p| self.emit_type(p)));
                format!("({}) -> {}", ps.join(", "), self.emit_type(ret))
            }
            Type::Tuple { elems, .. } => {
                let strs: Vec<String> =
                    elems.iter().map(|e| self.emit_type(e)).collect();
                self.tuple_type(&strs)
            }
            Type::QualifiedGroup { base, .. } => self.emit_type(base),
        }
    }

    fn emit_named_type(&mut self, qualifiers: &[TypeRef], base: &TypeRef) -> String {
        let name = base.name.name.as_str().to_string();
        let has_mut = qualifiers.iter().any(|q| q.name.name == "Mut");

        // A `Mut`-qualified intrinsic type maps through its own `Mut`
        // lowering [type-canbe-mut] (e.g. `Mut List<T>` ->
        // `MutableList<T>`).
        if has_mut {
            let arg_strs: Vec<String> = self.type_arg_strs(&base.args);
            if let Some(code) = self.expand_mut_type(&name, &arg_strs) {
                return code;
            }
        }
        self.emit_type_ref_named(&name, &base.args)
    }
    fn expand_mut_type(&mut self, name: &str, arg_strs: &[String]) -> Option<String> {
        let kt = crate::intrinsics::mut_type_name(name)?;
        let args = if arg_strs.is_empty() {
            String::new()
        } else {
            format!("<{}>", arg_strs.join(", "))
        };
        Some(format!("{kt}{args}"))
    }

    fn emit_type_ref(&mut self, r: &TypeRef) -> String {
        self.emit_type_ref_named(&r.name.name, &r.args)
    }

    /// [cmp-carry] The rendered type arguments, **identities dropped**: an
    /// identity is carried by the checker and by the container's own comparator,
    /// never by the emitted type. One helper, so every path that renders a written
    /// argument list drops the same thing.
    fn type_arg_strs(&mut self, args: &[Type]) -> Vec<String> {
        args.iter()
            .filter(|a| !salvo_syntax::ast::is_identity_arg(a))
            .map(|a| self.emit_type(a))
            .collect()
    }

    fn emit_type_ref_named(&mut self, name: &str, args: &[Type]) -> String {
        // Type aliases expand structurally (with generic substitution).
        if let Some(alias) = self.symbols.type_aliases.get(name) {
            let alias = *alias;
            if let Some(target) = &alias.alias {
                if alias.generics.is_empty() {
                    return self.emit_type(target);
                }
                let map: std::collections::HashMap<&str, &Type> = alias
                    .generics
                    .iter()
                    .map(|g| g.name.as_str())
                    .zip(args.iter())
                    .collect();
                let substituted = subst_ast_type(target, &map);
                return self.emit_type(&substituted);
            }
        }
        // [monitor-handler] [kt-monitor] A plain effect's addr is the
        // effect's lock wrapper, not a scheduler index — decided on the
        // *written* type here, before the argument is rendered away.
        if name == "Addr" && args.len() == 1 {
            if let Type::Named { base: eff, .. } = &args[0] {
                let effect = eff.name.name.clone();
                if self
                    .symbols
                    .effects
                    .get(effect.as_str())
                    .is_some_and(|e| !e.is_actor)
                {
                    let eff_args: Vec<String> =
                        eff.args.iter().map(|a| self.emit_type(a)).collect();
                    return self.plain_addr_rendering(&effect, &eff_args);
                }
            }
        }
        let arg_strs: Vec<String> = self.type_arg_strs(args);
        self.emit_named_parts(name, &arg_strs)
    }

    /// [monitor-handler] [kt-monitor] The rendering of `Addr<E>` for a plain
    /// effect `E`: the effect's own interface, at the instantiation the
    /// addr's type argument carries (`Random<Int>` for `Addr<Random<Int>>`).
    fn plain_addr_rendering(&mut self, effect: &str, args: &[String]) -> String {
        let arity = self
            .symbols
            .effects
            .get(effect)
            .map_or(0, |e| e.generics.len());
        if args.len() != arity {
            // An uninstantiated generic effect has no concrete handle type;
            // the checker resolves the instance everywhere it can, so this
            // is a leniency path, not a rule [type-unknown-lenient].
            self.error(format!(
                "the shared handle of effect `{effect}` needs its {arity} type \
                 argument(s), and this mention carries {}",
                args.len()
            ));
            return "Unit".to_string();
        }
        // [monitor-handler] [mixed-handler] [kt-monitor] The handle *is* the
        // interface: a monitor's `__Mon_E` and a mixed handler's `__Fac_H`
        // both implement it, and JVM references make the value freely
        // shareable — Kotlin needs no clone-box machinery where Rust does.
        if args.is_empty() {
            effect.to_string()
        } else {
            format!("{effect}<{}>", args.join(", "))
        }
    }

    /// Maps a named type (with already-emitted generic arguments) to Kotlin:
    /// intrinsic types [backend-intrinsic] [kt-none-unit], or a
    /// pass-through name [type-unknown-lenient].
    fn emit_named_parts(&mut self, name: &str, arg_strs: &[String]) -> String {
        let args = if arg_strs.is_empty() {
            String::new()
        } else {
            format!("<{}>", arg_strs.join(", "))
        };
        // Intrinsic (compiler-mapped) types [backend-intrinsic]. There is no
        // other mapping to try: a type the target language provides is
        // reached through a `platform effect`, not by naming it here.
        if let Some(kt) = crate::intrinsics::type_name(name) {
            // [kt-bytes] Naming the type is what pulls in its runtime class;
            // `Mut Bytes` renders through here too (one class serves both).
            if name == "Bytes" {
                self.needs_bytes = true;
            }
            // [kt-actor] The scheduler's handles are not generic here: an addr
            // is an `Int` and a token is one class, so the Salvo type argument
            // has no rendering — the generated message classes carry it.
            if matches!(name, "Addr" | "Pool" | "Reply") {
                self.needs_scheduler = true;
                return kt.to_string();
            }
            return format!("{kt}{args}");
        }
        // [backend-intrinsic] [backend-never-wrong] An `intrinsic type` this
        // backend has no mapping for cannot pass through: its Salvo name
        // means nothing in Kotlin, so emitting it would hand kotlinc a
        // dangling reference instead of reporting the gap here. (`Addr`,
        // `Reply` and `Pool` are exactly this until the actor classes
        // land.)
        if self.symbols.intrinsic_types.contains_key(name) {
            self.error(format!(
                "the `{name}` type is not supported by the kotlin backend yet"
            ));
            return format!("{name}{args}");
        }
        // Structs, generics, effects, and unknown names pass through.
        format!("{name}{args}")
    }

    fn emit_type_args(&mut self, args: &[Type]) -> String {
        // [cmp-carry] A written identity is not a type argument: the container's
        // own comparator holds the ordering, so it is dropped here — the one place
        // every written argument list passes through.
        let args: Vec<&Type> = args
            .iter()
            .filter(|a| !salvo_syntax::ast::is_identity_arg(a))
            .collect();
        if args.is_empty() {
            String::new()
        } else {
            format!(
                "<{}>",
                args.iter()
                    .map(|a| self.emit_type(a))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
    }

    /// Renders a checker `Ty` as Kotlin. Used for effect types resolved by
    /// the checker (`use_effects`/`effect_calls`/`call_effects`)
    /// [kt-effect-params]: the result keys effect-environment lookups, so
    /// it must agree with `emit_type` on the rendering of the same source
    /// type.
    fn kotlin_ty(&mut self, ty: &Ty) -> String {
        match ty {
            Ty::Named { name, args } => {
                // [monitor-handler] [kt-monitor] A plain effect's addr is the
                // effect's lock wrapper, not a scheduler index.
                if name == "Addr" && args.len() == 1 {
                    if let Ty::Named { name: effect, args: eff_args } = args[0].strip_quals() {
                        let effect = effect.clone();
                        let eff_args = eff_args.clone();
                        if self
                            .symbols
                            .effects
                            .get(effect.as_str())
                            .is_some_and(|e| !e.is_actor)
                        {
                            let rendered: Vec<String> =
                                eff_args.iter().map(|a| self.kotlin_ty(a)).collect();
                            return self.plain_addr_rendering(&effect, &rendered);
                        }
                    }
                }
                let arg_strs: Vec<String> = args.iter().map(|a| self.kotlin_ty(a)).collect();
                self.emit_named_parts(name, &arg_strs)
            }
            Ty::Qualified { quals, base } => {
                // [fn-effects] An effect claim on a producer names the
                // A `Mut`-qualified intrinsic type maps through its `Mut`
                // lowering [type-canbe-mut]; other qualifiers erase.
                if let Ty::Named { name, args } = base.as_ref() {
                    if quals.iter().any(|q| q.name == "Mut") {
                        let name = name.clone();
                        let arg_strs: Vec<String> =
                            args.iter().map(|a| self.kotlin_ty(a)).collect();
                        if let Some(code) = self.expand_mut_type(&name, &arg_strs) {
                            return code;
                        }
                    }
                }
                self.kotlin_ty(base)
            }
            Ty::Array(elem) => format!("Array<{}>", self.kotlin_ty(elem)),
            Ty::Tuple(elems) => {
                let strs: Vec<String> = elems.iter().map(|e| self.kotlin_ty(e)).collect();
                self.tuple_type(&strs)
            }
            Ty::Var(v) => v.clone(),
            Ty::Any => "Any".to_string(),
            Ty::Never => "Nothing".to_string(), // Kotlin's own bottom type keeps its name
            // [implicit-param] A fn type reaches here as an implicit
            // parameter's type, which is always one: `(A, B) -> R`, with a
            // `None` result spelled `Unit` as Kotlin wants it.
            Ty::Fn { params, ret, .. } => {
                let ps: Vec<String> = params.iter().map(|p| self.kotlin_ty(p)).collect();
                let r = if ret.is_none_ty() {
                    "Unit".to_string()
                } else {
                    self.kotlin_ty(ret)
                };
                format!("({}) -> {r}", ps.join(", "))
            }
            // [union-repr] A union reaches here inside an implicit
            // parameter's fn type — `params Yield`'s `next` returns
            // `Emitted T | Finished` — and it lowers to the wrapper
            // encoding like any other union. Before R5 no implicit member
            // returned one, and the fallback below printed the *Salvo*
            // text into the Kotlin source [backend-never-wrong].
            Ty::Union(_) => self.emit_ty(ty),
            // Unknowns do not occur as effect types; the Salvo-side
            // rendering keeps the lookup falling back to base-name matching
            // for anything unexpected.
            other => other.to_string(),
        }
    }

    /// Unions lower to the sealed wrapper encoding: `None` arms become
    /// Kotlin nullability, a single remaining arm is `T?`, two or more
    /// become `UnionN<...>`. Duplicate arms (same base and qualifiers) are
    /// collapsed like the checker does.
    fn emit_union_type(&mut self, arms: &[Type]) -> String {
        let mut nullable = false;
        // (dedup key: qualifier names + emitted base) -> emitted base
        let mut value_arms: Vec<(String, String)> = Vec::new();
        for arm in arms {
            if is_none_type(arm) {
                nullable = true;
                continue;
            }
            let quals: Vec<&str> = match arm {
                Type::Named { qualifiers, .. } | Type::QualifiedGroup { qualifiers, .. } => {
                    qualifiers.iter().map(|q| q.name.name.as_str()).collect()
                }
                _ => Vec::new(),
            };
            let emitted = self.emit_type(arm);
            let key = format!("{quals:?}|{emitted}");
            if !value_arms.iter().any(|(k, _)| *k == key) {
                value_arms.push((key, emitted));
            }
        }
        let suffix = if nullable { "?" } else { "" };
        match value_arms.len() {
            0 => "Any?".to_string(),
            1 => format!("{}{suffix}", value_arms[0].1),
            n => {
                self.union_sizes.insert(n);
                let args: Vec<String> = value_arms.into_iter().map(|(_, e)| e).collect();
                format!("Union{n}<{}>{suffix}", args.join(", "))
            }
        }
    }

    // ================= checker-type (Ty) rendering =================

    /// [cmp-carry] The `hash` and `eq` a hash container built here is kept by, as
    /// two Kotlin function references — `None` for the canonical path, which stays
    /// a `LinkedHashMap` and is what every Salvo program has always compiled to.
    ///
    /// On the JVM there is no zero-sized-type trick: the pair is passed to the
    /// runtime container as *values*. The container extends the JVM's abstract
    /// collections, so the emitted type is unchanged and only the construction
    /// differs — which is why this backend pays one call site and no lowerings.
    fn keyed_pair(&mut self, span: Span) -> Option<(String, String)> {
        let ty = self.checked.expr_ty.get(&(self.file_idx, span))?.clone();
        let Ty::Named { name, args } = ty.strip_quals() else {
            return None;
        };
        if !matches!(name.as_str(), "Set" | "Map") {
            return None;
        }
        let subject = args.first()?.clone();
        let ids: Vec<FnId> = args
            .iter()
            .filter_map(|a| match a {
                Ty::FnName(id) => Some(id.clone()),
                _ => None,
            })
            .collect();
        if ids.len() != 2 {
            return None;
        }
        self.needs_keyed = true;
        let hash = self.identity_fn_name(&ids[0], &subject)?;
        let eq = self.identity_fn_name(&ids[1], &subject)?;
        Some((hash, eq))
    }

    /// [cmp-carry] The Kotlin name of the declaration an identity resolves to —
    /// the *checker's* answer, recorded where the type was written.
    fn identity_fn_name(&mut self, id: &FnId, subject: &Ty) -> Option<String> {
        let decl = self
            .checked
            .carried_identities
            .get(&(id.clone(), subject.clone()))
            .copied()
            .and_then(|k| self.fn_by_key(k));
        match decl {
            // A **function reference**: the container takes the pair as values, so
            // what it needs is `::name` rather than a call.
            Some(decl) => Some(format!("::{}", self.kotlin_fn_name(decl))),
            None => {
                self.error(format!(
                    "the `{id}` this collection is keyed by has no resolved declaration"
                ));
                None
            }
        }
    }

    /// [cmp-carry] The Kotlin function a keyed container's named ordering is kept
    /// by, when this expression builds one — a `TreeSet`/`TreeMap` takes a
    /// comparator, so the identity needs no marker here, only a name.
    ///
    /// `None` means "the canonical path", which is `__salvoCompare` and what every
    /// sorted collection was built with before an ordering could be named. The
    /// **hash** pair is refused instead: `LinkedHashSet` keys off
    /// `hashCode`/`equals` with no slot for a function, so it needs a runtime
    /// container this backend does not have yet [backend-never-wrong].
    fn container_ordering(&mut self, span: Span) -> Option<String> {
        let ty = self.checked.expr_ty.get(&(self.file_idx, span))?.clone();
        let Ty::Named { name, args } = ty.strip_quals() else {
            return None;
        };
        if !matches!(name.as_str(), "Set" | "Map" | "SortedSet" | "SortedMap") {
            return None;
        }
        let Some(Ty::FnName(id)) = args.iter().find(|a| matches!(a, Ty::FnName(_))) else {
            return None;
        };
        let (id, name) = (id.clone(), name.clone());
        let subject = args.first()?.clone();
        // [cmp-carry] A *hash* container's pair is handed to the runtime container
        // as two function values rather than as a comparator, so it is resolved by
        // `keyed_pair` instead — this path is the sorted pair's comparator.
        if matches!(name.as_str(), "Set" | "Map") {
            return None;
        }
        let key = (id.clone(), subject);
        let decl = self
            .checked
            .carried_identities
            .get(&key)
            .copied()
            .and_then(|k| self.fn_by_key(k));
        match decl {
            Some(decl) => Some(self.kotlin_fn_name(decl)),
            None => {
                self.error(format!(
                    "the ordering `{id}` of this collection has no resolved declaration"
                ));
                None
            }
        }
    }

    /// Renders a checker type to Kotlin (qualifiers erased, unions as
    /// sealed wrappers).
    fn emit_ty(&mut self, ty: &Ty) -> String {
        match ty {
            // [qual-depend] A place argument is the checker's alone: it
            // lives inside an erased qualifier and never reaches output.
            Ty::ValueRef { .. } => String::new(),
            Ty::Named { name, args } => {
                // [cmp-carry] An identity a keyed container carries is the
                // checker's, not a rendering: the container's own machinery holds
                // the ordering, so the emitted type names only its elements.
                let arg_strs: Vec<String> = args
                    .iter()
                    .filter(|a| !matches!(a, Ty::FnName(_)))
                    .map(|a| self.emit_ty(a))
                    .collect();
                self.emit_named_parts(name, &arg_strs)
            }
            Ty::Qualified { quals, base } => {
                // [fn-effects] A claiming producer is its own type here too.
                // The intrinsic `Mut` mapping survives erasure
                // [type-canbe-mut].
                if quals.iter().any(|q| q.name == "Mut") {
                    if let Ty::Named { name, args } = base.as_ref() {
                        let name = name.clone();
                        let arg_strs: Vec<String> =
                            args.iter().map(|a| self.emit_ty(a)).collect();
                        if let Some(code) = self.expand_mut_type(&name, &arg_strs) {
                            return code;
                        }
                    }
                }
                self.emit_ty(base)
            }
            Ty::Union(_) => {
                let value_arms = ty.value_arms();
                let suffix = if ty.has_none_arm() { "?" } else { "" };
                match value_arms.len() {
                    0 => "Any?".to_string(),
                    1 => format!("{}{suffix}", self.emit_ty(value_arms[0])),
                    n => {
                        self.union_sizes.insert(n);
                        let args: Vec<String> =
                            value_arms.iter().map(|a| self.emit_ty(a)).collect();
                        format!("Union{n}<{}>{suffix}", args.join(", "))
                    }
                }
            }
            Ty::Tuple(elems) => {
                let strs: Vec<String> = elems.iter().map(|e| self.emit_ty(e)).collect();
                self.tuple_type(&strs)
            }
            Ty::Array(elem) => format!("Array<{}>", self.emit_ty(elem)),
            Ty::Fn {
                params,
                ret,
                effects,
                ..
            } => {
                // [fn-effects]
                let mut ps: Vec<String> =
                    effects.iter().map(|e| self.kotlin_ty(e)).collect();
                ps.extend(params.iter().map(|p| self.emit_ty(p)));
                format!("({}) -> {}", ps.join(", "), self.emit_ty(ret))
            }
            Ty::Var(name) => name.clone(),
            // [cmp-carry] An identity is not a value's type. On a
            // *qualifier* it erases with the qualifier [qual-erasure], and a
            // keyed container reads its own slots; reaching the general
            // renderer means one was written where a type belongs, which the
            // checker refuses — so this is an error, never output
            // [backend-never-wrong].
            Ty::FnName(id) => {
                self.error(format!(
                    "the function identity `{id}` reached code generation as a \
                     type: an identity may only fill a declaration's fn slot"
                ));
                "Any".to_string()
            }
            Ty::Any | Ty::Unknown => "Any".to_string(),
            Ty::Never => "Nothing".to_string(), // Kotlin's own bottom type keeps its name
        }
    }

    // ================= statements =================

    fn emit_block_stmts(&mut self, block: &Block, indent: usize) -> String {
        // [kt-effect-fusion] Save the whole environment, not just its
        // depth: a `use` inside the scope *rebases* enclosing entries onto
        // its fused value, and that mutation must roll back with the scope.
        let saved_effect_env = self.effect_env.clone();
        let out = self.emit_stmts(&block.stmts, indent);
        self.effect_env = saved_effect_env;
        out
    }

    fn emit_stmts(&mut self, stmts: &[Stmt], indent: usize) -> String {
        let mut out = String::new();
        for stmt in stmts {
            out.push_str(&self.emit_stmt(stmt, indent));
        }
        out
    }

    fn emit_stmt(&mut self, stmt: &Stmt, indent: usize) -> String {
        let pad = "    ".repeat(indent);
        self.expr_indent = indent;
        match stmt {
            // [fn-rename] Erased: a rename is a compile-time name for an
            // overload the call sites already resolved [fn-overload-scope].
            Stmt::Rename(_) => String::new(),
            Stmt::Let {
                pattern,
                ty,
                value,
                ..
            } => self.emit_let(pattern, ty.as_ref(), value, indent),
            Stmt::Assign { target, value, .. } => {
                let t = self.emit_expr_raw(target);
                let v = self.emit_expr(value);
                format!("{pad}{t} = {v}\n")
            }
            // [expr-escape] The escapes are expressions now (2026-09-21) and
            // arrive wrapped in a statement; these arms precede the general
            // `Stmt::Expr` case and keep every rule they had.
            Stmt::Expr(Expr::Return { value, .. }) => match (self.stmt_ctx, value.as_deref()) {
                // [kt-none-unit] A `None` value in a `Unit`-returning fn has no
                // payload to hand back: evaluate it for its effects (it may be
                // a call) and return bare. `return null` would be a kotlinc
                // error — and in a *nullable*-returning fn it is exactly right,
                // which is why this asks the fn and not the value.
                (_, Some(v))
                    if self.ret_is_unit
                        && self.ty_of(v.span()).is_some_and(|t| t.is_none_ty()) =>
                {
                    // The literal `None` has nothing to evaluate; anything else
                    // may be a call, so it runs for its effects. Emitting
                    // `null` as a statement would warn ("expression is
                    // unused") in code the user cannot edit.
                    if matches!(v, Expr::Ident(id) if id.name == "None") {
                        format!("{pad}return\n")
                    } else {
                        let stmt = self.emit_expr_stmt(v, indent);
                        format!("{stmt}{pad}return\n")
                    }
                }
                (_, Some(v)) => {
                    let v = self.emit_expr(v);
                    format!("{pad}return {v}\n")
                }
                (_, None) => format!("{pad}return\n"),
            },
            Stmt::Expr(Expr::Break { value, .. }) => {
                let value = value.as_deref();
                let target = self.loop_results.last().cloned().flatten();
                match (value, target) {
                    // [while-value] route the value into the enclosing
                    // loop's result local before breaking.
                    (Some(v), Some(result)) => {
                        if self.ty_of(v.span()).is_some_and(|t| t.is_none_ty()) {
                            // A `None`-typed value has no Kotlin payload:
                            // evaluate for effects, record `null`.
                            let stmt = self.emit_expr_stmt(v, indent);
                            format!("{stmt}{pad}{result} = null\n{pad}break\n")
                        } else {
                            let code = self.emit_expr(v);
                            format!("{pad}{result} = {code}\n{pad}break\n")
                        }
                    }
                    // The loop's value is discarded (statement position):
                    // evaluate the operand for side effects only.
                    (Some(v), None) => {
                        let stmt = self.emit_expr_stmt(v, indent);
                        format!("{stmt}{pad}break\n")
                    }
                    (None, _) => format!("{pad}break\n"),
                }
            }
            Stmt::Expr(Expr::Continue { .. }) => format!("{pad}continue\n"),
            Stmt::Use {
                handler,
                local,
                with_items,
                span,
            } => self.emit_use(handler, *local, with_items, *span, indent),
            Stmt::Expr(expr) => self.emit_expr_stmt(expr, indent),
        }
    }

    fn emit_let(
        &mut self,
        pattern: &Pattern,
        ty: Option<&Type>,
        value: &Expr,
        indent: usize,
    ) -> String {
        let pad = "    ".repeat(indent);
        // Bare struct literals pick up the annotated type.
        let value_code = match (value, ty) {
            (
                Expr::StructLit {
                    ty: None, fields, span,
                },
                Some(annot),
            ) => self.emit_struct_lit(Some(annot), fields, *span),
            _ => self.emit_expr(value),
        };
        match pattern {
            Pattern::Ident(name) => {
                let kw = if self.mutated.contains(&name.name) {
                    "var"
                } else {
                    "val"
                };
                let annot = match ty {
                    Some(t) => format!(": {}", self.emit_type(t)),
                    None => String::new(),
                };
                format!(
                    "{pad}{kw} {}{annot} = {value_code}\n",
                    kt_ident(&name.name)
                )
            }
            Pattern::Tuple { elems, .. } => {
                let names: Vec<String> = elems
                    .iter()
                    .map(|p| match p {
                        Pattern::Ident(id) => kt_ident(&id.name),
                        _ => {
                            self.error("nested destructuring patterns are not supported yet");
                            "_".to_string()
                        }
                    })
                    .collect();
                format!("{pad}val ({}) = {value_code}\n", names.join(", "))
            }
            Pattern::Struct { fields, .. } => {
                // Unique per fn: two struct-destructuring `let`s in one
                // block must not collide [let-destructure].
                self.destructure_id += 1;
                let temp = format!("__destructured{}", self.destructure_id);
                let mut out = format!("{pad}val {temp} = {value_code}\n");
                for f in fields {
                    let kw = if self.mutated.contains(&f.binding.name) {
                        "var"
                    } else {
                        "val"
                    };
                    out.push_str(&format!(
                        "{pad}{kw} {} = {temp}.{}\n",
                        kt_ident(&f.binding.name),
                        kt_ident(&f.field.name)
                    ));
                }
                out
            }
        }
    }

    /// `use Handler(...)` — instantiate the handler, bind it, and register
    /// it in the effect environment for the rest of the scope
    /// [effect-use] [effect-scope]. All handlers are Kotlin classes; a
    /// bare `use Handler` is sugar for `Handler()`.
    ///
    /// The registered effect type comes from the checker (`use_effects`),
    /// which infers handler generics from the constructor arguments (e.g.
    /// `Random<Int>` for `use CyclicRandom([1,2,3])`); the handler's
    /// declared `of` type is the fallback for unchecked contexts.
    /// [platform-handler] [kt-platform-handler] The class a `use` constructs.
    /// An ordinary handler's is the emitted class of the same name; a
    /// platform handler's is the *host's*, in the `platform/` package of the
    /// module that declared it — named in full, because the host package is
    /// not among a module's generated imports and a `use` may sit in any
    /// module.
    fn handler_ctor_name(&mut self, name: &str, decl: &HandlerDecl) -> String {
        if !decl.platform {
            return kt_ident(name);
        }
        match self.symbols.handler_modules.get(name) {
            Some(module) => {
                self.platform_hosts.insert((*module).clone());
                format!("{}.{}", host_package(module), kt_ident(name))
            }
            None => {
                self.error(format!(
                    "internal: the declaring module of platform handler `{name}` \
                     could not be located"
                ));
                kt_ident(name)
            }
        }
    }

    /// [actor-spawn-expr] [kt-actor] `spawn H(args) capacity N on P` →
    /// `SalvoSched.spawn(pool, bound, __Actor_H(H(args)))`, whose value is the
    /// addr. Construction is the `use` path's, minus the registration: a spawn
    /// does not put the handler in *this* scope.
    /// [kt-mailbox] [actor-mailbox] The `capacity` expression of a handler's
    /// `mailbox` slot. The checker has already required the slot on a handler of
    /// an `actor effect` and confined it to constructor parameters, so a missing
    /// one is an internal inconsistency rather than a language cut.
    fn mailbox_capacity_code(&mut self, h: &HandlerDecl) -> String {
        match h.mailbox.as_ref().and_then(mailbox_capacity_expr) {
            Some(expr) => self.emit_expr(expr),
            None => {
                self.error(format!(
                    "internal: actor handler `{}` has no `mailbox {{ capacity: … }}`",
                    h.name.name
                ));
                "0".to_string()
            }
        }
    }

    fn emit_spawn(
        &mut self,
        handler: &Expr,
        uses: &[Expr],
        pool: Option<&Expr>,
        span: Span,
    ) -> String {
        let (handler_name, args): (String, Vec<String>) = match handler {
            Expr::Ident(id) => (id.name.clone(), Vec::new()),
            Expr::Call { callee, args, .. } => match callee.as_ref() {
                Expr::Ident(id) => (
                    id.name.clone(),
                    args.iter().map(|a| self.emit_expr(a)).collect(),
                ),
                _ => {
                    self.error("`spawn` expects a handler name or constructor call");
                    return "TODO()".to_string();
                }
            },
            _ => {
                self.error("`spawn` expects a handler name or constructor call");
                return "TODO()".to_string();
            }
        };
        let Some(decl) = self.symbols.handlers.get(handler_name.as_str()).copied() else {
            self.error(format!("unknown handler `{handler_name}` in `spawn`"));
            return "TODO()".to_string();
        };
        if !decl.generics.is_empty() {
            self.error(format!(
                "spawning generic handler `{handler_name}` is not supported yet"
            ));
            return "TODO()".to_string();
        }
        // [monitor-handler] [kt-monitor] Every face a plain effect: the
        // **monitor spawn** — no mailbox, no scheduler, no pool. One shared
        // instance behind a lock, handed out as the effect's `__Mon_E`
        // wrapper. The checker restricted the handler (single face, no
        // dependencies, sendable state) and refused the `on` clause.
        let plain_faces = !decl.of.is_empty()
            && decl.of.iter().all(|of| {
                type_base_name(of).is_some_and(|n| {
                    self.symbols
                        .effects
                        .get(n)
                        .is_some_and(|e| !e.is_actor)
                })
            });
        if plain_faces {
            if decl.of.len() > 1 {
                self.error(format!(
                    "handler `{handler_name}` implements several plain effects, and a \
                     shared instance behind several faces is not supported yet"
                ));
                return "TODO()".to_string();
            }
            let ctor = self.handler_ctor_name(&handler_name, decl);
            // [mixed-handler] [kt-mixed] The mixed spawn: build the handler,
            // spawn the servant, answer the façade. Constructor arguments are
            // evaluated **once** into locals shared by handler and façade —
            // the checker refused `Mut` parameters, which is what makes the
            // JVM's sharing and Rust's cloning observably identical.
            if decl.fns.iter().any(|f| f.is_send) {
                self.needs_scheduler = true;
                let mut lets = String::new();
                let mut handler_args: Vec<String> = Vec::new();
                let mut fac_args: Vec<String> = vec!["__a".to_string()];
                for (i, code) in args.iter().enumerate() {
                    lets.push_str(&format!("val __c{i} = {code}; "));
                    handler_args.push(format!("__c{i}"));
                    fac_args.push(format!("__c{i}"));
                }
                let pool_code = match pool {
                    Some(pool) => self.emit_expr(pool),
                    None => "salvo.SalvoSched.currentPool()".to_string(),
                };
                return format!(
                    "run {{ {lets}val __h = {ctor}({}); \
                     val __a = salvo.SalvoSched.spawn({pool_code}, __h.__mailboxCapacity, \
                     {}(__h)); {}({}) }}",
                    handler_args.join(", "),
                    actor_class_name(&handler_name),
                    facade_class_name(&handler_name),
                    fac_args.join(", ")
                );
            }
            let effect = decl.of.first().and_then(type_base_name).unwrap_or_default();
            let _ = effect;
            return format!(
                "{}({ctor}({}))",
                monitor_class_name(
                    decl.of.first().and_then(type_base_name).unwrap_or_default()
                ),
                args.join(", ")
            );
        }
        self.needs_scheduler = true;
        // [effect-handler-deps] [kt-effect-fusion] The child's carrier, built
        // here instead of at a `use` site: the same generated `__Fx_N` class,
        // its arguments the clause's instances in the handler's *declaration*
        // order — which is what the checker's `spawn_dep_items` records, since
        // the program wrote them in its own.
        let mut args = args;
        let deps = self.handler_dep_effects(decl);
        if !deps.is_empty() {
            match self.spawn_carrier(&handler_name, &deps, uses, span) {
                Some(code) => args.push(code),
                None => return "TODO()".to_string(),
            }
        }
        let ctor = self.handler_ctor_name(&handler_name, decl);
        // [main-pool] An omitted `on` clause means the pool current where the
        // spawn runs — main's own pool in `main`, the actor's in a member.
        let pool_code = match pool {
            Some(pool) => self.emit_expr(pool),
            None => "salvo.SalvoSched.currentPool()".to_string(),
        };
        // [actor-mailbox] The bound is the handler's own, so the instance is
        // built into a local and read before it is wrapped.
        let spawn_call = format!(
            "salvo.SalvoSched.spawn({pool_code}, __h.__mailboxCapacity, {}(__h))",
            actor_class_name(&handler_name)
        );
        // [effect-handler-multi] One addr per implemented effect. There is one
        // actor, one mailbox and one scheduler id; the tuple hands the same id
        // out under each protocol's type, so least authority costs nothing at
        // run time. `Pair`/`Triple` are the tuple renderings [type-tuple], so a
        // handler of more than three faces is the ordinary too-large-tuple
        // codegen error.
        if decl.of.len() > 1 {
            let ctor_name = match decl.of.len() {
                2 => "Pair",
                3 => "Triple",
                _ => {
                    self.error(format!(
                        "handler `{handler_name}` implements {} effects, and the kotlin \
                         backend renders a spawn's addr tuple as `Pair`/`Triple` — at \
                         most three faces",
                        decl.of.len()
                    ));
                    return "TODO()".to_string();
                }
            };
            let faces: Vec<&str> = (0..decl.of.len()).map(|_| "__a").collect();
            return format!(
                "run {{ val __h = {ctor}({}); val __a = {spawn_call}; {ctor_name}({}) }}",
                args.join(", "),
                faces.join(", ")
            );
        }
        format!(
            "run {{ val __h = {ctor}({}); {spawn_call} }}",
            args.join(", ")
        )
    }

    /// [actor-spawn-expr] [kt-actor] The carrier a spawned dependent handler
    /// stores: `__Fx_N(d0, d1)` over the clause's instances, in the handler's
    /// declaration order (the fused class takes them in its own canonical
    /// order, which is what `emit_fx_class` answers).
    fn spawn_carrier(
        &mut self,
        handler_name: &str,
        deps: &[String],
        uses: &[Expr],
        span: Span,
    ) -> Option<String> {
        let items = match self.checked.spawn_dep_items.get(&(self.file_idx, span)) {
            Some(items) if items.len() == deps.len() => items.clone(),
            _ => {
                self.error(format!(
                    "internal: spawning `{handler_name}` has no resolved dependency \
                     items for its {} declared dependencies",
                    deps.len()
                ));
                return None;
            }
        };
        // [spawn-inherit] A written clause item is constructed here; a
        // **synthesized** dependency (`None`) is the spawning scope's own
        // instance — a JVM reference, so the child shares it by holding it.
        let mut instances: Vec<String> = Vec::new();
        for (i, at) in items.iter().enumerate() {
            match at {
                Some(at) => {
                    let Some(item) = uses.get(*at) else {
                        self.error(format!(
                            "internal: spawning `{handler_name}` names clause item \
                             {at}, which the form does not have"
                        ));
                        return None;
                    };
                    instances.push(self.spawn_dep_instance(item, &deps[i]));
                }
                None => instances.push(self.lookup_effect_handler_by_type(&deps[i])),
            }
        }
        let (class, _props, order) = self.emit_fx_class(deps);
        let ordered: Vec<String> = order.iter().map(|i| instances[*i].clone()).collect();
        Some(format!("{class}({})", ordered.join(", ")))
    }

    /// [actor-spawn-expr] One clause item as the expression that *makes* an
    /// instance: a handler construction is `D(args)`, an `Addr` is the
    /// forwarding stub over it (`__Stub_D(addr)`) — the same two shapes `use`
    /// binds, which is what lets a child's dependency be a local handler in
    /// one program and an actor in the next.
    fn spawn_dep_instance(&mut self, item: &Expr, dep: &str) -> String {
        let named = match item {
            Expr::Ident(id) => Some((id.name.clone(), Vec::new())),
            Expr::Call { callee, args, .. } => match callee.as_ref() {
                Expr::Ident(id) => Some((id.name.clone(), args.iter().collect::<Vec<_>>())),
                _ => None,
            },
            _ => None,
        };
        if let Some((name, args)) = named {
            if let Some(decl) = self.symbols.handlers.get(name.as_str()).copied() {
                let arg_code: Vec<String> =
                    args.iter().map(|a| self.emit_expr(a)).collect();
                let ctor = self.handler_ctor_name(&name, decl);
                return format!("{ctor}({})", arg_code.join(", "));
            }
        }
        // Not a handler name, so it is an addr: the stub is the instance.
        // [monitor-handler] [kt-monitor] A plain effect's handle *is* an
        // instance already — the effect's lock wrapper — so it passes through
        // as itself, and the same monitor serves every actor it is supplied
        // to. An actor effect's addr is an index the send stub wraps.
        let addr_code = self.emit_expr(item);
        let base = base_of_rendered(dep);
        if self
            .symbols
            .effects
            .get(base)
            .is_some_and(|e| !e.is_actor)
        {
            return addr_code;
        }
        format!("{}({addr_code})", stub_class_name(base))
    }

    /// [actor-waitfor] `waitfor out: Reply<T> { … }` → a `run { }` expression:
    /// mint the waiter, run the block that sends the token somewhere, then
    /// block this thread for the answer and cast it.
    fn emit_waitfor(
        &mut self,
        binding: &Ident,
        ty: &Option<Type>,
        body: &Block,
        _span: Span,
    ) -> String {
        self.needs_scheduler = true;
        let indent = self.expr_indent;
        let pad = "    ".repeat(indent + 1);
        let close = "    ".repeat(indent);
        // [waitfor-infer] Off the written `Reply<T>` when there is one, else the
        // type the checker recorded for the expression — which *is* the payload.
        let payload = match ty {
            Some(Type::Named { base, .. })
                if base.name.name == "Reply" && base.args.len() == 1 =>
            {
                self.emit_type(&base.args[0])
            }
            None => match self.ty_of(_span).cloned() {
                Some(t) => self.emit_ty(&t),
                None => {
                    self.error("`waitfor` binds a `Reply<T>`");
                    return "TODO()".to_string();
                }
            },
            _ => {
                self.error("`waitfor` binds a `Reply<T>`");
                return "TODO()".to_string();
            }
        };
        let mut out = String::from("run {\n");
        out.push_str(&format!(
            "{pad}val ({}, __wid) = salvo.SalvoSched.waiter()\n",
            binding.name
        ));
        let saved = std::mem::replace(&mut self.expr_indent, indent + 1);
        out.push_str(&self.emit_stmts(&body.stmts, indent + 1));
        self.expr_indent = saved;
        out.push_str(&format!(
            "{pad}salvo.SalvoSched.awaitReply(__wid) as {payload}\n{close}}}"
        ));
        out
    }

    /// [actor-use-addr] [kt-actor] `addr.member(args)` →
    /// `SalvoSched.send(addr, __Msg_E.Member(args))`.
    fn emit_addr_send(
        &mut self,
        effect: &salvo_core::Ty,
        callee: &Expr,
        args: &[Expr],
        span: Span,
    ) -> String {
        self.needs_scheduler = true;
        let Expr::Field { base, field, .. } = callee else {
            self.error("internal: an addr send whose callee is not a dot-call");
            return "TODO()".to_string();
        };
        let effect_name = match effect {
            salvo_core::Ty::Named { name, .. } => name.clone(),
            _ => {
                self.error("internal: an addr send with no effect recorded");
                return "TODO()".to_string();
            }
        };
        let member = self.called_member_name(&effect_name, &field.name, span);
        let msg = msg_class_name(&effect_name);
        let variant = msg_variant_name(&member);
        let payload: Vec<String> = args.iter().map(|a| self.emit_expr(a)).collect();
        let target = self.emit_expr(base);
        format!(
            "salvo.SalvoSched.send({target}, {msg}.{variant}({}))",
            payload.join(", ")
        )
    }

    /// [mixed-handler] [kt-mixed] A façade send: enqueue on the servant's
    /// mailbox through the façade's own addr. The message class is the
    /// *handler's* (`__Msg_H`), not an effect's.
    fn emit_facade_send(&mut self, member: &str, args: &[Expr]) -> String {
        self.needs_scheduler = true;
        let Some(handler) = self.current_handler.clone() else {
            self.error("internal: a façade send outside a handler member");
            return "TODO()".to_string();
        };
        let msg = msg_class_name(&handler);
        let variant = msg_variant_name(member);
        let payload: Vec<String> = args.iter().map(|a| self.emit_expr(a)).collect();
        // An empty payload is an `object` subclass of `__Msg_H`.
        let built = if payload.is_empty() {
            format!("{msg}.{variant}")
        } else {
            format!("{msg}.{variant}({})", payload.join(", "))
        };
        format!("salvo.SalvoSched.send(__addr, {built})")
    }

    /// [mixed-handler] [kt-mixed] Whether a handler is mixed: every face a
    /// plain effect, and at least one `send fn` member — the checker's
    /// classification, mirrored.
    fn handler_is_mixed(&self, h: &HandlerDecl) -> bool {
        h.fns.iter().any(|f| f.is_send)
            && !h.of.is_empty()
            && h.of.iter().all(|of| {
                type_base_name(of).is_some_and(|n| {
                    self.symbols.effects.get(n).is_some_and(|e| !e.is_actor)
                })
            })
    }

    /// [use-local] [effect-handler-deps] The shared dependency-form
    /// predicate, with the symbol table answering the face kind.
    /// [with-clause] A written `with` item as the instance the depending
    /// handler stores: a handler construction is a **private instance** built
    /// here, and an `Addr` value is already a handle. On the JVM a reference
    /// *is* the handle, so neither needs wrapping — which is why this side
    /// needs no lock machinery where Rust's does
    /// ([rs-platform-handler] records the same asymmetry).
    fn with_item_instance(&mut self, item: &Expr) -> String {
        let named = match item {
            Expr::Ident(id) => Some((id.name.clone(), Vec::new())),
            Expr::Call { callee, args, .. } => match callee.as_ref() {
                Expr::Ident(id) => Some((id.name.clone(), args.iter().collect())),
                _ => None,
            },
            _ => None,
        };
        if let Some((name, args)) = named {
            if let Some(decl) = self.symbols.handlers.get(name.as_str()).copied() {
                let arg_code: Vec<String> = args.iter().map(|a| self.emit_expr(a)).collect();
                let ctor = self.handler_ctor_name(&name, decl);
                return format!("{ctor}({})", arg_code.join(", "));
            }
        }
        self.emit_expr(item)
    }

    fn handler_is_handle_dep(&self, decl: &HandlerDecl) -> bool {
        salvo_core::handler_handle_deps(decl, |name| {
            self.symbols
                .effects
                .get(name)
                .is_some_and(|e| e.is_actor)
        })
    }

    /// [mixed-handler] [kt-mixed] The façade: the servant's addr plus the
    /// constructor parameters, implementing each plain face with the sync
    /// member bodies.
    fn emit_facade(&mut self, h: &'p HandlerDecl) -> String {
        let fac = facade_class_name(&h.name.name);
        let of = h
            .of
            .clone()
            .iter()
            .map(|of| self.emit_type(of))
            .collect::<Vec<String>>()
            .join(", ");
        let mut params: Vec<String> = vec!["private val __addr: Int".to_string()];
        for p in &h.params {
            params.push(format!(
                "private val {}: {}",
                kt_ident(&p.name.name),
                self.emit_type(&p.ty)
            ));
        }
        let mut out = format!("\nclass {fac}({}) : {of} {{\n", params.join(", "));
        let saved_member_effect = std::mem::replace(&mut self.handler_member_of, Some(h));
        let saved_handler =
            std::mem::replace(&mut self.current_handler, Some(h.name.name.clone()));
        for f in &h.fns {
            if f.is_send {
                continue;
            }
            out.push_str(&self.emit_fn_inner(f, "override fun", 1, false));
        }
        self.current_handler = saved_handler;
        self.handler_member_of = saved_member_effect;
        out.push_str("}\n");
        out
    }

    /// [mixed-handler] [kt-mixed] The servant's runtime parts: the
    /// handler-keyed message class (`__Msg_H`, one subclass per `send fn`
    /// member) and the actor body dispatching it — resume included, since a
    /// servant parks continuations of its own [defer-deduction]. Simpler
    /// than the face-keyed twin: a mixed servant has (for now) no
    /// dependencies and one protocol.
    fn emit_mixed_actor_parts(&mut self, h: &HandlerDecl) -> String {
        self.needs_scheduler = true;
        let name = h.name.name.clone();
        let msg = msg_class_name(&name);
        let actor = actor_class_name(&name);
        let sends: Vec<&FnDecl> = h.fns.iter().filter(|f| f.is_send).collect();
        let mut out = format!("\nsealed class {msg} {{\n");
        for f in &sends {
            let variant = msg_variant_name(&f.name.name);
            let payload: Vec<String> = f
                .params
                .iter()
                .filter(|p| !p.implicit)
                .map(|p| format!("val {}: {}", p.name.name, self.emit_type(&p.ty)))
                .collect();
            if payload.is_empty() {
                out.push_str(&format!("    object {variant} : {msg}()\n"));
            } else {
                out.push_str(&format!(
                    "    class {variant}({}) : {msg}()\n",
                    payload.join(", ")
                ));
            }
        }
        out.push_str("}\n");
        out.push_str(&format!(
            "\nclass {actor}(private val handler: {name}) : salvo.SalvoActor {{\n    \
             override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {{\n        \
             handler.__addr = ctx.addr\n        \
             when (msg) {{\n"
        ));
        for f in &sends {
            let variant = msg_variant_name(&f.name.name);
            let has_payload = f.params.iter().any(|p| !p.implicit);
            let args: Vec<String> = f
                .params
                .iter()
                .filter(|p| !p.implicit)
                .map(|p| format!("msg.{}", p.name.name))
                .collect();
            if has_payload {
                out.push_str(&format!(
                    "            is {msg}.{variant} -> handler.{}({})\n",
                    f.name.name,
                    args.join(", ")
                ));
            } else {
                out.push_str(&format!(
                    "            is {msg}.{variant} -> handler.{}()\n",
                    f.name.name
                ));
            }
        }
        out.push_str(
            "            else -> error(\"a message of this servant's protocol\")\n        \
             }\n    }\n",
        );
        // [actor-replyto] [defer-deduction] The other half of the servant's
        // parked table, mirroring the face-keyed resume: the slot names the
        // continuation, and its subclass says which send member to run and
        // therefore what the answer casts to — its *trailing* parameter's
        // type.
        match self.handler_cont_type(h) {
            None => out.push_str(
                "\n    override fun resume(ctx: salvo.SalvoCtx, slot: Long, value: Any?) {\n        \
                 error(\"this servant's protocol has no continuation targets\")\n    }\n",
            ),
            Some(cont) => {
                out.push_str(
                    "\n    override fun resume(ctx: salvo.SalvoCtx, slot: Long, value: Any?) {\n        \
                     handler.__addr = ctx.addr\n        \
                     // A reply whose continuation is gone: nothing to run.\n        \
                     val c = handler.__parked.remove(slot) ?: return\n        \
                     when (c) {\n",
                );
                for f in &sends {
                    let fixed: Vec<&Param> = f.params.iter().filter(|p| !p.implicit).collect();
                    if fixed.is_empty() {
                        continue;
                    }
                    let variant = msg_variant_name(&f.name.name);
                    let mut args: Vec<String> = fixed[..fixed.len() - 1]
                        .iter()
                        .map(|p| format!("c.{}", p.name.name))
                        .collect();
                    let answer_ty = self.emit_type(&fixed[fixed.len() - 1].ty);
                    args.push(format!("value as {answer_ty}"));
                    out.push_str(&format!(
                        "            is {cont}.{variant} -> handler.{}({})\n",
                        f.name.name,
                        args.join(", ")
                    ));
                }
                out.push_str("        }\n    }\n");
            }
        }
        out.push_str("}\n");
        out
    }

    fn emit_use(
        &mut self,
        handler: &Expr,
        local: bool,
        with_items: &[Expr],
        span: Span,
        indent: usize,
    ) -> String {
        let _ = local; // classification travels in `use_kinds` [use-local]
        // [actor-use-addr] `use addr` binds the effect to a **forwarding stub**
        // over the addr: `__Stub_E(addr)` is an ordinary instance of the effect
        // as far as the rest of this scope is concerned, which is exactly why
        // nothing downstream needs to know the difference.
        if let Some(effect) = self.checked.use_addrs.get(&(self.file_idx, span)).cloned() {
            let rendered = self.kotlin_ty(&effect);
            let addr_code = self.emit_expr(handler);
            // [monitor-handler] [kt-monitor] A plain effect's handle already
            // *is* the effect's lock wrapper, so binding it is binding the
            // value itself; an actor's addr is an index that the send stub
            // turns into an instance.
            let instance = match &effect {
                salvo_core::Ty::Named { name, .. } => {
                    if self
                        .symbols
                        .effects
                        .get(name.as_str())
                        .is_some_and(|e| !e.is_actor)
                    {
                        addr_code
                    } else {
                        format!("{}({addr_code})", stub_class_name(name))
                    }
                }
                _ => {
                    self.error("internal: a `use addr` with no effect recorded");
                    return String::new();
                }
            };
            return self.bind_effect_instance(vec![(Some(effect), rendered)], instance, indent);
        }
        let (handler_name, written_args) = match handler {
            Expr::Ident(id) => (id.name.clone(), Vec::new()),
            Expr::Call { callee, args, .. } => match callee.as_ref() {
                Expr::Ident(id) => (
                    id.name.clone(),
                    args.iter().map(|a| self.emit_expr(a)).collect(),
                ),
                _ => {
                    self.error("`use` expects a handler name or constructor call");
                    return String::new();
                }
            },
            _ => {
                self.error("`use` expects a handler name or constructor call");
                return String::new();
            }
        };
        let Some(decl) = self.symbols.handlers.get(handler_name.as_str()) else {
            self.error(format!("unknown handler `{handler_name}` in `use`"));
            return String::new();
        };
        // [use-local] The binding kind, decided by the checker (shareable by
        // default, user decision 2026-09-20): a monitor binds behind the
        // per-effect `synchronized` wrapper — `__Mon_E(H(args))` — and a
        // stateless handler binds bare. `use local` keeps the pre-2026-09-20
        // emission.
        let kind = self
            .checked
            .use_kinds
            .get(&(self.file_idx, span))
            .copied()
            .unwrap_or(salvo_core::UseKind::Local);
        // [effect-handler-deps] Dependencies are not written at the `use`
        // site: the compiler supplies them, and it supplies them as **one
        // fused value the handler stores** for its lifetime
        // ([kt-effect-fusion]) — built here, out of the effects in scope
        // *before* this registration, which is exactly what makes an
        // intercepting handler wrap the instance it shadows
        // [effect-intercept].
        let dep_effects = self.handler_dep_effects(decl);
        let mut written = written_args.into_iter();
        let mut ctor_args: Vec<String> = Vec::new();
        for p in &decl.params {
            if p.implicit {
                // [copy-implicit] An implicit constructor parameter arrives
                // as the adapter the checker resolved at this `use` — the
                // same rendering a fn call's implicit arguments get.
                let filled = self.emit_implicit_args(&[], span);
                let idx = decl
                    .params
                    .iter()
                    .filter(|q| q.implicit)
                    .position(|q| q.name.name == p.name.name)
                    .unwrap_or(0);
                match filled.get(idx) {
                    Some(code) => ctor_args.push(code.clone()),
                    None => {
                        self.error(format!(
                            "internal: no value resolved for implicit constructor \
                             parameter `{}`",
                            p.name.name
                        ));
                        ctor_args.push("TODO()".to_string());
                    }
                }
            } else if let Some(code) = written.next() {
                ctor_args.push(code);
            }
        }
        ctor_args.extend(written);
        // The carrier the handler's dependencies arrive in, when it has any:
        // its class also completes the handler's type-argument list below.
        // [use-local] A handle-dep handler takes them as individual trailing
        // arguments instead — the binding expression per dep, in declaration
        // order (JVM references are the handles).
        let mut dep_class: Option<String> = None;
        if !dep_effects.is_empty() {
            if self.handler_is_handle_dep(decl) {
                // [with-clause] A written item is a **private instance**,
                // constructed here; JVM references are the handles, so the
                // constructor argument is simply that construction.
                let clause_of = self
                    .checked
                    .use_with_items
                    .get(&(self.file_idx, span))
                    .cloned()
                    .unwrap_or_default();
                for (i, dep) in dep_effects.iter().enumerate() {
                    if let Some(Some(at)) = clause_of.get(i) {
                        match with_items.get(*at) {
                            Some(item) => {
                                let code = self.with_item_instance(item);
                                // [with-clause] The parameter is typed as the
                                // dep's accessor interface, so a private
                                // instance goes through the one-instance
                                // adapter emitted beside it.
                                let wrapped =
                                    format!("__One_{}({code})", sanitize_instance(dep));
                                // Registers the interface (and its adapter).
                                let _ = self.has_iface(dep);
                                ctor_args.push(wrapped);
                                continue;
                            }
                            None => {
                                self.error(format!(
                                    "internal: the `with` clause has no item {at} for \
                                     dependency `{dep}`"
                                ));
                                ctor_args.push("TODO()".to_string());
                                continue;
                            }
                        }
                    }
                    let arg = self.thread_effect_fused_by_type(dep);
                    ctor_args.push(arg);
                }
            } else {
                let (class, _props, order) = self.emit_fx_class(&dep_effects);
                let args: Vec<String> = order
                    .iter()
                    .map(|i| self.lookup_effect_handler_by_type(&dep_effects[*i]))
                    .collect();
                ctor_args.push(format!("{class}({})", args.join(", ")));
                dep_class = Some(class);
            }
        }
        // [effect-handler-generics] A generic handler is constructed *at* a
        // type: Kotlin cannot infer the class's parameter from an empty
        // argument list, so the `use` site's type arguments are written out.
        // A *dependent* handler carries one more parameter than Salvo wrote
        // — the carrier's [kt-effect-fusion] — and Kotlin takes a type
        // argument list whole or not at all, so it is appended here.
        let type_args = match self.checked.use_handler_args.get(&(self.file_idx, span)) {
            Some(args) if args.iter().all(ty_is_concrete) => {
                let args = args.clone();
                let mut rendered: Vec<String> =
                    args.iter().map(|a| self.kotlin_ty(a)).collect();
                rendered.extend(dep_class.clone());
                format!("<{}>", rendered.join(", "))
            }
            _ => String::new(),
        };
        let handler_code = format!(
            "{}{type_args}({})",
            self.handler_ctor_name(&handler_name, decl),
            ctor_args.join(", ")
        );
        // [use-local] [kt-monitor] The monitor binding: the construction
        // wrapped in the per-effect lock wrapper, which implements the same
        // interface, so everything downstream is unchanged.
        let handler_code = if kind == salvo_core::UseKind::Monitor {
            match self.checked.use_effects.get(&(self.file_idx, span)) {
                Some(tys) => match tys.first() {
                    Some(salvo_core::Ty::Named { name, .. }) => {
                        format!("{}({handler_code})", monitor_class_name(name))
                    }
                    _ => handler_code,
                },
                None => handler_code,
            }
        } else {
            handler_code
        };
        // [effect-handler-multi] One entry per implemented effect, in
        // declaration order: a `use` binds every face the handler wears.
        let faces: Vec<(Option<salvo_core::Ty>, String)> =
            match self.checked.use_effects.get(&(self.file_idx, span)) {
                Some(tys) if tys.iter().all(ty_is_concrete) => {
                    let tys = tys.clone();
                    tys.into_iter()
                        .map(|ty| {
                            let rendered = self.kotlin_ty(&ty);
                            (Some(ty), rendered)
                        })
                        .collect()
                }
                _ => decl
                    .of
                    .clone()
                    .iter()
                    .map(|of| (None, self.emit_type(of)))
                    .collect(),
            };
        self.bind_effect_instance(faces, handler_code, indent)
    }

    /// [kt-effect-fusion] [actor-use-addr] Bind one effect **instance** into
    /// the current scope, given the expression that constructs it: a handler
    /// construction from `use H(…)`, or a forwarding stub from `use addr`.
    /// Everything past this point is identical for the two, which is the whole
    /// point of the stub — the scope cannot tell them apart.
    fn bind_effect_instance(
        &mut self,
        // [effect-handler-multi] One entry per bound effect: the checker's
        // instance where it is concrete, and the rendered interface type.
        faces: Vec<(Option<salvo_core::Ty>, String)>,
        instance: String,
        indent: usize,
    ) -> String {
        let pad = "    ".repeat(indent);
        let rendered = faces[0].1.clone();
        // [kt-effect-fusion] Under the fusion a `use` builds one fused
        // value for the whole scope: a generated class with an `override
        // val` per effect — the inherited ones initialized from their
        // current expressions (objects alias, so the rebuild is *flat*,
        // unlike Rust's chaining), the new one from the handler
        // constructor. Every effect in scope then threads through it.
        if self.fusion {
            // One field per distinct instance, innermost resolution winning.
            let mut covered: Vec<(String, String)> = Vec::new(); // (rendered, expr)
            for e in &self.effect_env {
                match covered.iter_mut().find(|(r, _)| *r == e.rendered) {
                    Some(slot) => slot.1 = e.expr.clone(),
                    None => covered.push((e.rendered.clone(), e.expr.clone())),
                }
            }
            let new_rendered: Vec<String> = faces.iter().map(|(_, r)| r.clone()).collect();
            covered.retain(|(r, _)| !new_rendered.contains(r));
            let mut all_rendered: Vec<String> =
                covered.iter().map(|(r, _)| r.clone()).collect();
            all_rendered.extend(new_rendered.clone());
            let (class, props, order) = self.emit_fx_class(&all_rendered);
            let var = self.unique_name("__fx".to_string());
            let mut args: Vec<String> = covered.iter().map(|(_, e)| e.clone()).collect();
            // [effect-handler-multi] One instance behind every face, so a
            // handler of several effects is **built once** and named: pushing
            // the constructor call per face would construct one handler per
            // face, each with its own state.
            let mut prelude = String::new();
            if faces.len() > 1 {
                let held = self.unique_name("__h".to_string());
                prelude = format!("{pad}val {held} = {instance}\n");
                for _ in &faces {
                    args.push(held.clone());
                }
            } else {
                args.push(instance);
            }
            // The fused class takes its effects in canonical order, which is
            // not the environment's [kt-effect-fusion].
            let args: Vec<String> = order.iter().map(|i| args[*i].clone()).collect();
            // Rebase every entry onto the new fused value.
            for entry in self.effect_env.iter_mut() {
                if let Some(prop) = all_rendered
                    .iter()
                    .position(|r| *r == entry.rendered)
                    .and_then(|i| props.get(i))
                {
                    entry.expr = format!("{var}.{prop}");
                    entry.fused = Some(var.clone());
                }
            }
            for (ty, rendered) in faces {
                let prop = all_rendered
                    .iter()
                    .position(|r| *r == rendered)
                    .and_then(|i| props.get(i))
                    .cloned()
                    .unwrap_or_default();
                self.effect_env.push(EffectEntry {
                    ty,
                    rendered,
                    expr: format!("{var}.{prop}"),
                    fused: Some(var.clone()),
                });
            }
            return format!(
                "{prelude}{pad}val {var} = {class}({})\n",
                args.join(", ")
            );
        }
        let var = self.unique_name(effect_param_name(&rendered));
        // [effect-handler-multi] With several faces the annotation is dropped:
        // the val's type is the handler's own class, which satisfies every
        // interface it implements, and Kotlin infers it.
        let annotation = if faces.len() == 1 {
            format!(": {rendered}")
        } else {
            String::new()
        };
        for (ty, rendered) in faces {
            self.effect_env.push(EffectEntry {
                ty,
                rendered,
                expr: var.clone(),
                fused: None,
            });
        }
        format!("{pad}val {var}{annotation} = {instance}\n")
    }

    fn emit_expr_stmt(&mut self, expr: &Expr, indent: usize) -> String {
        let pad = "    ".repeat(indent);
        match expr {
            Expr::If {
                branches,
                else_block,
                ..
            } => self.emit_if(branches, else_block.as_ref(), indent),
            Expr::While {
                cond,
                body,
                else_block,
                ..
            } => {
                // [while-value] statement position: the value is discarded;
                // `else` still needs a ran-flag (it runs only if the loop
                // never did).
                let ran = else_block
                    .as_ref()
                    .map(|_| format!("{}_ran", self.fresh_loop_var()));
                let inner_pad = "    ".repeat(indent + 1);
                let mut out = String::new();
                if let Some(ran) = &ran {
                    out.push_str(&format!("{pad}var {ran} = false\n"));
                }
                // [is-bind-once] A subject that is not a place is evaluated
                // **once per iteration**, into a `val` the test and the
                // binding share: `while <call> is T x` used to emit the call
                // twice per turn, silently dropping every other value.
                match self.hoistable_is(cond) {
                    Some(subject) => {
                        out.push_str(&format!("{pad}while (true) {{\n"));
                        let temp = self.emit_is_temp(subject, indent + 1);
                        out.push_str(&temp);
                        let c = self.emit_expr(cond);
                        out.push_str(&format!("{inner_pad}if (!({c})) break\n"));
                    }
                    None => {
                        let c = self.emit_expr(cond);
                        out.push_str(&format!("{pad}while ({c}) {{\n"));
                    }
                }
                if let Some(ran) = &ran {
                    out.push_str(&format!("{inner_pad}{ran} = true\n"));
                }
                // `while x is T name` re-binds per iteration.
                out.push_str(&self.emit_is_bindings(cond, indent + 1));
            out.push_str(&self.emit_widen_shadows(cond, indent + 1));
                out.push_str(&self.emit_widen_shadows(cond, indent + 1));
                self.loop_results.push(None);
                out.push_str(&self.emit_block_stmts(body, indent + 1));
                self.loop_results.pop();
                out.push_str(&format!("{pad}}}\n"));
                if let (Some(ran), Some(b)) = (&ran, else_block) {
                    out.push_str(&format!("{pad}if (!{ran}) {{\n"));
                    out.push_str(&self.emit_block_stmts(b, indent + 1));
                    out.push_str(&format!("{pad}}}\n"));
                }
                out
            }
            Expr::For {
                pattern,
                iterable,
                body,
                else_block,
                ..
            } => {
                let ran = else_block
                    .as_ref()
                    .map(|_| format!("{}_ran", self.fresh_loop_var()));
                let inner_pad = "    ".repeat(indent + 1);
                // [iter-protocol] A **pass** is *driven*, not iterated: the
                // header calls the `next` the checker resolved. Everything
                // after it is the same as for any other loop.
                // [linear-group] No implicit discharge sites (user decision
                // 2026-09-12): the loop never closes a pass — a linear one
                // is checker-refused unless kept, so nothing here splices.
                let claiming: Option<(String, String)> = None;
                let pass = if claiming.is_some() {
                    None
                } else {
                    self.pass_driver_of(iterable)
                        .filter(|d| !d.origin)
                        .map(|driver| {
                            self.emit_pass_loop_header(driver, pattern, iterable, indent)
                        })
                };
                // The pass and claiming headers bind the element themselves
                // (and a destructuring pattern's prologue rides with them), so
                // only a *native* `for` needs a loop variable here — asking for
                // one twice would mint two temporaries for one loop.
                let var = if claiming.is_some() || pass.is_some() {
                    String::new()
                } else {
                    self.for_pattern_var(pattern, indent + 1)
                };
                let iter = if pass.is_none() && claiming.is_none() {
                    let code = self.emit_expr(iterable);
                    self.native_for_subject(iterable, code)
                } else {
                    String::new()
                };
                let mut out = String::new();
                if let Some(ran) = &ran {
                    out.push_str(&format!("{pad}var {ran} = false\n"));
                }
                match (&claiming, &pass) {
                    (Some((header, ..)), _) => out.push_str(header),
                    (None, Some(header)) => out.push_str(header),
                    (None, None) => out.push_str(&format!("{pad}for ({var} in {iter}) {{\n")),
                }
                if let Some(ran) = &ran {
                    out.push_str(&format!("{inner_pad}{ran} = true\n"));
                }
                // [let-destructure] A destructuring pattern's bindings open the
                // body, read off the element the header bound.
                out.push_str(&self.take_loop_destructure());
                self.loop_results.push(None);
                out.push_str(&self.emit_block_stmts(body, indent + 1));
                self.loop_results.pop();
                out.push_str(&format!("{pad}}}\n"));
                // [fn-effects] The injected `close`, in a `finally` so that
                // `break`, `return` and exhaustion all reach it
                // [kt-exit-finally]. The flags inside make landing there
                // twice harmless.
                if let Some((_, trailer)) = &claiming {
                    out.push_str(trailer);
                }
                if let (Some(ran), Some(b)) = (&ran, else_block) {
                    out.push_str(&format!("{pad}if (!{ran}) {{\n"));
                    out.push_str(&self.emit_block_stmts(b, indent + 1));
                    out.push_str(&format!("{pad}}}\n"));
                }
                out
            }
            Expr::When {
                subject, branches, ..
            } => {
                let code = self.emit_when(subject, branches, indent, false);
                format!("{pad}{code}\n")
            }
            // [when-condition] Statement position: Kotlin's own
            // subject-less `when` [kt-when-cond].
            Expr::WhenCond {
                branches,
                else_block,
                ..
            } => {
                let code = self.emit_when_cond(branches, else_block, indent, false);
                format!("{pad}{code}\n")
            }
            _ => {
                let code = self.emit_expr(expr);
                format!("{pad}{code}\n")
            }
        }
    }

    /// An `if`/`elif`/`else` chain as a statement, inserting `is`-binding
    /// declarations at the top of the matching branch.
    fn emit_if(
        &mut self,
        branches: &[(Expr, Block)],
        else_block: Option<&Block>,
        indent: usize,
    ) -> String {
        let pad = "    ".repeat(indent);
        let mut out = String::new();
        for (i, (cond, block)) in branches.iter().enumerate() {
            // [is-bind-once] The subject is evaluated once, before the test.
            // Only the first branch can carry a `val` here, which is what the
            // checker allows; a later one is refused there, so this arm guards
            // against the two drifting apart [backend-never-wrong].
            if let Some(subject) = self.hoistable_is(cond) {
                if i == 0 {
                    let temp = self.emit_is_temp(subject, indent);
                    out.push_str(&temp);
                } else {
                    self.error(
                        "an `is` binding over a call in an `elif` condition cannot be \
                         lowered: bind the subject with `let` first"
                            .to_string(),
                    );
                }
            }
            let kw = if i == 0 {
                format!("{pad}if")
            } else {
                "else if".to_string()
            };
            let c = self.emit_expr(cond);
            out.push_str(&format!("{kw} ({c}) {{\n"));
            out.push_str(&self.emit_is_bindings(cond, indent + 1));
            out.push_str(&self.emit_widen_shadows(cond, indent + 1));
            out.push_str(&self.emit_block_stmts(block, indent + 1));
            out.push_str(&format!("{pad}}} "));
        }
        if let Some(block) = else_block {
            out.push_str("else {\n");
            out.push_str(&self.emit_block_stmts(block, indent + 1));
            out.push_str(&format!("{pad}}}\n"));
        } else {
            // Trim the trailing space from the last `}`.
            out.pop();
            out.push('\n');
        }
        out
    }

    /// A `when` expression over a union-typed subject. Wrapper unions lower
    /// to a Kotlin `when` over the sealed hierarchy; nullable `T?` subjects
    /// lower to a subject-less `when` on null tests.
    fn emit_when(
        &mut self,
        subject: &Expr,
        branches: &[WhenBranch],
        indent: usize,
        value_pos: bool,
    ) -> String {
        let pad = "    ".repeat(indent);
        let subj = self.emit_expr_raw(subject);
        let tests: Vec<Option<UnionTest>> = branches
            .iter()
            .map(|b| self.is_test_of(b.span).cloned())
            .collect();
        let Some(size) = tests.iter().flatten().map(|t| t.size).next() else {
            self.error("`when` could not be lowered (subject is not a checked union)");
            return String::new();
        };

        let mut out = if size >= 2 {
            format!("when ({subj}) {{\n")
        } else {
            "when {\n".to_string()
        };
        let count = branches.len();
        for (i, (branch, test)) in branches.iter().zip(&tests).enumerate() {
            let Some(test) = test else { continue };
            let label = if size >= 2 {
                if test.match_none {
                    "null".to_string()
                } else {
                    self.union_sizes.insert(test.size);
                    let stars = vec!["*"; test.size].join(", ");
                    test.arms
                        .iter()
                        .map(|a| format!("is U{}_{}<{stars}>", test.size, a + 1))
                        .collect::<Vec<_>>()
                        .join(", ")
                }
            } else if i + 1 == count {
                // Subject-less `when` needs a trailing `else`; the checker
                // proved exhaustiveness.
                "else".to_string()
            } else if test.match_none {
                format!("{subj} == null")
            } else {
                format!("{subj} != null")
            };
            out.push_str(&format!("{pad}    {label} -> {{\n"));
            if let Some(b) = &branch.binding {
                let kt = self
                    .ty_of(b.span)
                    .cloned()
                    .map(|t| self.emit_ty(&t))
                    .unwrap_or_else(|| "Any".to_string());
                let bind = if size >= 2 {
                    let access = if test.nullable { "?" } else { "" };
                    self.note_payload_cast();
                    format!(
                        "{pad}        val {} = {subj}{access}.value as {kt}\n",
                        kt_ident(&b.name)
                    )
                } else {
                    self.note_payload_cast();
                    format!("{pad}        val {} = {subj} as {kt}\n", kt_ident(&b.name))
                };
                out.push_str(&bind);
            }
            // [qual-lift] A `^` branch head peels the arm it matched.
            if branch.widen {
                if let Some(target) = self
                    .checked
                    .widen_targets
                    .get(&(self.file_idx, branch.span))
                    .cloned()
                {
                    match subject {
                        Expr::Ident(id) => {
                            let kt = self.emit_ty(&target);
                            let access = if test.nullable { "?" } else { "" };
                            self.note_payload_cast();
                            out.push_str(&format!(
                                "{pad}        val {} = {subj}{access}.value as {kt}\n",
                                kt_ident(&id.name)
                            ));
                        }
                        other => {
                            let place = self.emit_expr_raw(other);
                            self.error(format!(
                                "`^` on a projection is not supported yet: \
                                 widening materializes a local for the \
                                 branch, which needs a plain variable — bind \
                                 `{place}` to one first"
                            ));
                        }
                    }
                }
            }
            if value_pos {
                out.push_str(&self.emit_value_block(&branch.body, indent + 2));
            } else {
                out.push_str(&self.emit_block_stmts(&branch.body, indent + 2));
            }
            out.push_str(&format!("{pad}    }}\n"));
        }
        out.push_str(&format!("{pad}}}"));
        out
    }

    /// [when-condition] The subject-less `when`: a condition chain with a
    /// mandatory `else`. Kotlin has the same form [kt-when-cond], so the
    /// shape survives the translation — `cond -> { … }` arms closed by
    /// `else -> { … }`. Being total, it is a Kotlin *expression* in value
    /// position without the `else null` filler an `if` chain needs.
    fn emit_when_cond(
        &mut self,
        branches: &[(Expr, Block)],
        else_block: &Block,
        indent: usize,
        value_pos: bool,
    ) -> String {
        let pad = "    ".repeat(indent);
        let mut out = String::from("when {\n");
        for (cond, block) in branches {
            let c = self.emit_expr(cond);
            out.push_str(&format!("{pad}    {c} -> {{\n"));
            out.push_str(&self.emit_is_bindings(cond, indent + 2));
            out.push_str(&self.emit_widen_shadows(cond, indent + 2));
            if value_pos {
                out.push_str(&self.emit_value_block(block, indent + 2));
            } else {
                out.push_str(&self.emit_block_stmts(block, indent + 2));
            }
            out.push_str(&format!("{pad}    }}\n"));
        }
        out.push_str(&format!("{pad}    else -> {{\n"));
        if value_pos {
            out.push_str(&self.emit_value_block(else_block, indent + 2));
        } else {
            out.push_str(&self.emit_block_stmts(else_block, indent + 2));
        }
        out.push_str(&format!("{pad}    }}\n"));
        out.push_str(&format!("{pad}}}"));
        out
    }

    /// [qual-lift] Materializes the peel a `^` check performs: the widened
    /// value is bound to a **shadowing** local for the branch, so reads of
    /// the subject — and any nested `when` on it — see it at the widened
    /// type. (Kotlin warns about the shadowing; the alternative, a fresh
    /// name, would need every read rewritten.)
    fn emit_widen_shadows(&mut self, cond: &Expr, indent: usize) -> String {
        let pad = "    ".repeat(indent);
        let mut sites: Vec<(&Expr, Span)> = Vec::new();
        collect_widen_checks(cond, &mut |subject, span| sites.push((subject, span)));
        let mut out = String::new();
        for (subject, span) in sites {
            let Some(target) = self.checked.widen_targets.get(&(self.file_idx, span)).cloned()
            else {
                continue;
            };
            let Expr::Ident(id) = subject else {
                let place = self.emit_expr_raw(subject);
                self.error(format!(
                    "`^` on a projection is not supported yet: widening \
                     materializes a local for the branch, which needs a \
                     plain variable — bind `{place}` to one first"
                ));
                continue;
            };
            let subj = self.emit_place_storage(subject);
            let kt = self.emit_ty(&target);
            let nullable = self
                .is_test_of(span)
                .map(|t| t.nullable)
                .unwrap_or(false);
            let access = if nullable { "?" } else { "" };
            self.note_payload_cast();
            out.push_str(&format!(
                "{pad}val {} = {subj}{access}.value as {kt}\n",
                kt_ident(&id.name)
            ));
        }
        out
    }

    /// For a condition containing `x is T name`, emits the binding
    /// declaration inside the branch: unwrapping the union arm value for
    /// wrapper unions, a plain cast otherwise.
    /// [is-bind-once] The `is` subject a condition needs hoisted, if any — the
    /// mirror of the Rust backend's rule, and the same shapes: a binding `is`
    /// over a non-place subject, as the whole condition.
    fn hoistable_is<'a>(&self, cond: &'a Expr) -> Option<&'a Expr> {
        match cond {
            Expr::Is {
                subject,
                binding: Some(_),
                ..
            } if !is_place_expr(subject) => Some(subject),
            _ => None,
        }
    }

    /// Emits the single evaluation of a hoisted subject and registers the
    /// temporary. Answers the line to place before the test.
    fn emit_is_temp(&mut self, subject: &Expr, indent: usize) -> String {
        self.is_temp_id += 1;
        let name = format!("__is{}", self.is_temp_id);
        let code = self.emit_expr(subject);
        self.is_temps
            .insert((self.file_idx, subject.span()), name.clone());
        // A `var`, not a `val`: kotlinc smart-casts a `val` after the null
        // test and then calls the binding's cast "useless" — a warning in
        // generated code the author cannot edit. A `var` is not smart-cast, so
        // the cast the general (wrapper-union) case needs stays warning-free
        // here too, and nothing ever reassigns it [kt-suppress-cast].
        format!("{}var {name} = {code}\n", "    ".repeat(indent))
    }

    fn emit_is_bindings(&mut self, cond: &Expr, indent: usize) -> String {
        let pad = "    ".repeat(indent);
        let mut collected: Vec<(&Expr, &[TypeRef], &Ident, Span, bool)> = Vec::new();
        collect_is_bindings(cond, &mut |subject, check, binding, is_span, lift| {
            collected.push((subject, check, binding, is_span, lift));
        });
        let mut out = String::new();
        for (subject, check, binding, is_span, lift) in collected {
            // The binding reads the payload out of the storage: a narrowed
            // subject place must not unwrap twice [flow-place].
            let subj = self.emit_place_storage(subject);
            // [qual-lift] A lift whose check peels **no wrapper** (`is ^Mut
            // plain`) binds the subject itself: qualifiers are erased, so the
            // lifted value *is* the value, and any `Mut`-drop conversion is the
            // coercion table's business [str-drop-mut]. Without this the
            // `is`-binding path built a cast to the check's terms — `list as
            // Mut`, which names no Kotlin type.
            // [rewrap] A multi-arm lift binds a *sub-union*: built by mapping
            // arm to arm, not by reading one payload.
            if let Some(from) = self
                .checked
                .rewrap_from
                .get(&(self.file_idx, binding.span))
                .cloned()
            {
                if let Some(to) = self.ty_of(binding.span).cloned() {
                    let code = self.emit_rewrap(subj.clone(), &from, &to);
                    out.push_str(&format!(
                        "{pad}val {} = {code}\n",
                        kt_ident(&binding.name)
                    ));
                    continue;
                }
            }
            if lift && self.is_test_of(is_span).is_none() {
                out.push_str(&format!(
                    "{pad}val {} = {subj}\n",
                    kt_ident(&binding.name)
                ));
                continue;
            }
            let code = match self.is_test_of(is_span).cloned() {
                Some(test) if test.size >= 2 => {
                    let kt = self
                        .ty_of(binding.span)
                        .cloned()
                        .map(|t| self.emit_ty(&t))
                        .unwrap_or_else(|| "Any".to_string());
                    let access = if test.nullable { "?" } else { "" };
                    self.note_payload_cast();
                    format!(
                        "val {} = {subj}{access}.value as {kt}",
                        kt_ident(&binding.name)
                    )
                }
                _ => {
                    if self
                        .checked
                        .predicate_tests
                        .contains_key(&(self.file_idx, is_span))
                    {
                        // Predicate checks refine only the qualifiers, which
                        // are erased: the binding is the subject itself.
                        format!("val {} = {subj}", kt_ident(&binding.name))
                    } else {
                        // [kt-suppress-cast] kotlinc smart-casts a local after
                        // the null test and then calls this cast "useless" — a
                        // warning in code the author cannot edit — while for a
                        // property (a handler's state field) the cast is
                        // genuinely needed. One spelling for both, with the
                        // suppression the mechanism already carries.
                        self.note_payload_cast();
                        let ty = self.emit_is_check_type(check);
                        format!("val {} = {subj} as {ty}", kt_ident(&binding.name))
                    }
                }
            };
            out.push_str(&format!("{pad}{code}\n"));
        }
        out
    }

    // ================= expressions =================

    /// Emits an expression, applying identifier narrowing unwraps and any
    /// checker-recorded representation coercion for this site.
    fn emit_expr(&mut self, expr: &Expr) -> String {
        let code = self.emit_expr_base(expr);
        self.apply_coercion(expr.span(), code)
    }

    /// Emits an expression with the narrowing unwrap but *without* boundary
    /// coercions (both are skipped for plain identifiers via `raw`).
    fn emit_expr_base(&mut self, expr: &Expr) -> String {
        // A narrowed place — an identifier or a projection [flow-place] —
        // reads its payload out of the declared representation.
        match self.place_unwrap_kind(expr) {
            Some((PlaceUnwrap::ArmValue, logical)) => {
                let logical = logical.clone();
                let nullable = self
                    .repr_of(expr.span())
                    .is_some_and(|repr| repr.has_none_arm());
                let kt = self.emit_ty(&logical);
                let access = if nullable { "?" } else { "" };
                let place = self.emit_place_storage(expr);
                // [kt-suppress-cast] Reading a narrowed place is a payload
                // cast like any other: the arms of a union are erased, so a
                // parameterized payload (`Union8<…>` for a nested union,
                // `List<Str>`) draws kotlinc's unchecked warning. Missed
                // until std's fs read one — every other cast site noted it.
                self.note_payload_cast();
                format!("({place}{access}.value as {kt})")
            }
            Some((PlaceUnwrap::NonNull, _)) => {
                let place = self.emit_place_storage(expr);
                format!("{place}!!")
            }
            None => self.emit_expr_raw(expr),
        }
    }

    /// [assert-trap] The text a failed assertion reports: `salvo: <what> at
    /// <file>:<line>:<col>`, with the Salvo source location rather than the
    /// generated one. A written message replaces `<what>` and is composed
    /// **here**, inside the `throw`, so it is evaluated only on failure.
    fn trap_message(&mut self, what: &str, message: Option<&Expr>, span: Span) -> String {
        let at = self.salvo_location(span);
        match message {
            Some(m) => {
                let text = self.emit_expr(m);
                format!("(\"salvo: \" + ({text}) + \" at {at}\")")
            }
            None => format!("\"salvo: {what} at {at}\""),
        }
    }

    /// The Salvo location of a span, for a trap message [assert-trap]:
    /// `<module>:<line>:<col>`.
    ///
    /// The **module path**, not the file name: a file's display name depends on
    /// how it was loaded (the CLI names an embedded std file `std/core/list.sv`,
    /// a directory walk names the same file `core/list.sv`), and a location that
    /// varies by loader would make the emitted output depend on who generated
    /// it. A module path is the language's own identity for a file and is the
    /// same either way.
    fn salvo_location(&self, span: Span) -> String {
        let file = &self.program.files[self.file_idx];
        let (line, col) = salvo_syntax::span::line_col(&file.content, span.start);
        format!("{}:{line}:{col}", file.module)
    }

    /// How a narrowed place read reaches its value [flow-place]: `None`
    /// when the place is not narrowed, so the plain read stands.
    ///
    /// [kt-narrow-field-assert] A `T?` *field* read cannot rely on Kotlin's
    /// smart cast — a struct field is a property, and a `canbe Mut`
    /// struct's is a `var`, which Kotlin refuses to smart-cast ("could be
    /// mutated concurrently") — so it asserts instead. Local variables do
    /// smart-cast, and keep the plain read.
    fn place_unwrap_kind(&self, expr: &Expr) -> Option<(PlaceUnwrap, &'p Ty)> {
        if !matches!(
            expr,
            Expr::Ident(_) | Expr::Field { .. } | Expr::TupleIndex { .. }
        ) {
            return None;
        }
        let span = expr.span();
        let (repr, logical) = (self.repr_of(span)?, self.ty_of(span)?);
        if repr.is_wrapper_union()
            && !matches!(logical, Ty::Union(_))
            && !logical.is_none_ty()
        {
            return Some((PlaceUnwrap::ArmValue, logical));
        }
        if matches!(expr, Expr::Field { .. } | Expr::TupleIndex { .. })
            && matches!(repr, Ty::Union(_))
            && repr.has_none_arm()
            && !repr.is_wrapper_union()
            && !logical.has_none_arm()
            && !logical.is_none_ty()
            && !matches!(logical, Ty::Union(_))
            && self.field_is_var(expr)
        {
            return Some((PlaceUnwrap::NonNull, logical));
        }
        None
    }

    /// Whether a field read goes through a Kotlin `var` property, which is
    /// what makes the smart cast unavailable [kt-narrow-field-assert]:
    /// fields of a `canbe Mut` struct emit as `var`, everything else as
    /// `val`. Unknown bases answer `true` — an unnecessary `!!` is a
    /// kotlinc *warning*, a missing one is a compile error
    /// ([backend-never-wrong]).
    fn field_is_var(&self, expr: &Expr) -> bool {
        // A tuple element is a `Pair`/`Triple` component: a `val`, but one
        // declared in the Kotlin *stdlib*, and kotlinc only smart-casts
        // properties from the module being compiled — so it needs the
        // assert [kt-narrow-field-assert].
        if matches!(expr, Expr::TupleIndex { .. }) {
            return true;
        }
        let Expr::Field { base, .. } = expr else {
            return false;
        };
        let Some(base_ty) = self.ty_of(base.span()) else {
            return true;
        };
        let Ty::Named { name, .. } = base_ty.strip_quals() else {
            return true;
        };
        let decl = self.program.modules.iter().flat_map(|m| &m.items).find_map(
            |item| match item {
                Item::Struct(s) if s.name.name == *name => Some(s),
                _ => None,
            },
        );
        match decl {
            Some(s) => s.auto_qualifiers.iter().any(|q| q.name.name == "Mut"),
            None => true,
        }
    }

    /// The Kotlin component name of a tuple position [kt-tuple-component]:
    /// `Pair`/`Triple` expose `first`/`second`/`third`, and a generated
    /// `SalvoTupleN` [kt-tuple-class] declares **those same three names**
    /// followed by `v3`, `v4`, … — so one rule serves every arity and the
    /// index alone decides the spelling. (An index past 2 can only ever be a
    /// generated tuple's: a `Pair` with a `.3` does not type check.)
    fn tuple_component(index: usize) -> String {
        match index {
            0 => "first".to_string(),
            1 => "second".to_string(),
            2 => "third".to_string(),
            n => format!("v{n}"),
        }
    }

    /// [kt-tuple-class] The Kotlin type of a tuple, given its rendered
    /// elements: `Pair`/`Triple` for two and three — Kotlin's own, so tuples
    /// keep interoperating with the standard library — and a **generated**
    /// `SalvoTupleN` past them, since Kotlin has no larger tuple type. The
    /// arity is recorded so `tuples.kt` declares the class the way
    /// `unions.kt` declares a union wrapper.
    fn tuple_type(&mut self, elems: &[String]) -> String {
        match elems.len() {
            2 => format!("Pair<{}, {}>", elems[0], elems[1]),
            3 => format!("Triple<{}, {}, {}>", elems[0], elems[1], elems[2]),
            n => {
                self.tuple_sizes.insert(n);
                // The generated class orders through `__salvoCompare`, whose
                // file also declares the marker interface it implements.
                self.needs_compare = true;
                format!("SalvoTuple{n}<{}>", elems.join(", "))
            }
        }
    }

    /// A place's storage rendering: the read *without* its own narrowing
    /// unwrap [flow-place]. The base keeps its unwraps — a narrowed base
    /// must be unwrapped before its field can be reached — and `is` tests,
    /// `is` bindings and `when` subjects read through this, since they
    /// operate on the declared representation.
    fn emit_place_storage(&mut self, expr: &Expr) -> String {
        // [is-bind-once] A hoisted subject *is* its temporary.
        if let Some(temp) = self.is_temps.get(&(self.file_idx, expr.span())) {
            return temp.clone();
        }
        match expr {
            Expr::Ident(_)
            | Expr::Field { .. }
            | Expr::TupleIndex { .. }
            | Expr::Index { .. } => self.emit_expr_raw(expr),
            other => self.emit_expr_base(other),
        }
    }

    /// The physical Kotlin-level type of an emitted expression (accounting
    /// for the place unwrap rule [flow-place]).
    fn emitted_repr(&self, expr: &Expr) -> Option<&'p Ty> {
        let logical = self.ty_of(expr.span())?;
        // Unwrapped at the use site: the emitted code has the narrowed
        // type, not the storage's.
        if let Some((_, narrowed)) = self.place_unwrap_kind(expr) {
            return Some(narrowed);
        }
        if matches!(
            expr,
            Expr::Ident(_) | Expr::Field { .. } | Expr::TupleIndex { .. }
        ) {
            if let Some(repr) = self.repr_of(expr.span()) {
                return Some(repr);
            }
        }
        Some(logical)
    }

    /// Applies a checker-recorded representation change (union wrap or
    /// re-wrap) to already-emitted code.
    fn apply_coercion(&mut self, span: Span, code: String) -> String {
        let Some(coercion) = self.coercion_of(span) else {
            return code;
        };
        self.apply_coercion_value(coercion.clone(), code)
    }

    /// The same, for a coercion already in hand — which is what lets a
    /// `DropMut` carry the change that would have been recorded in its
    /// place [str-drop-mut].
    fn apply_coercion_value(&mut self, coercion: Coercion, code: String) -> String {
        match coercion {
            // Kotlin nullability is transparent: a bare value is already a
            // valid `T?` [type-nullable].
            Coercion::WrapOption { .. } => code,
            // [type-none-unit] The `None` *value* in a `None`-typed slot is
            // the unit value, not `null`: the rendered code is replaced.
            Coercion::NoneUnit => "Unit".to_string(),
            // [str-drop-mut] [kt-mut-str] `StringBuilder` is not a
            // `String`; `MutableList<T>` *is* a `List<T>`, so that drop
            // renders nothing.
            Coercion::DropMut { from, then } => {
                let code = match ty_base_name(&from)
                    .and_then(crate::intrinsics::drop_mut_suffix)
                {
                    // A bare name takes the suffix directly; anything else
                    // is parenthesized, since the suffix binds tighter than
                    // whatever the expression ends with.
                    Some(suffix) if is_plain_name(&code) => format!("{code}{suffix}"),
                    Some(suffix) => format!("({code}){suffix}"),
                    None => code,
                };
                match then {
                    Some(inner) => self.apply_coercion_value(*inner, code),
                    None => code,
                }
            }
            Coercion::WrapUnion { target, arm, inner } => {
                // [qual-group] The inner wrap of a flattened nested group
                // runs first: the value is physically the bare inner value.
                let code = match inner {
                    Some(inner) => self.apply_coercion_value(*inner, code),
                    None => code,
                };
                let value_arms = target.value_arms();
                let n = value_arms.len();
                self.union_sizes.insert(n);
                let args: Vec<String> = value_arms
                    .iter()
                    .map(|a| {
                        let a = (*a).clone();
                        self.emit_ty(&a)
                    })
                    .collect();
                format!("U{n}_{}<{}>({code})", arm + 1, args.join(", "))
            }
            Coercion::Rewrap { from, to } => self.emit_rewrap(code, &from, &to),
        }
    }

    /// [pick] The payload of a single matched arm, at its own type: the shape
    /// the `is`-binding path emits, factored out so a qualifier pick can use it
    /// for both the picked and the unpicked side.
    fn emit_arm_payload(
        &mut self,
        subject: &Expr,
        test: Option<&UnionTest>,
        ty: Option<&Ty>,
    ) -> String {
        let subj = self.emit_place_storage(subject);
        match test {
            Some(t) if t.size >= 2 => {
                let kt = ty
                    .map(|t| self.emit_ty(t))
                    .unwrap_or_else(|| "Any".to_string());
                let access = if t.nullable { "?" } else { "" };
                self.note_payload_cast();
                format!("{subj}{access}.value as {kt}")
            }
            // A `T?` representation, or a single-arm union: the value is the
            // storage itself once the test has passed.
            _ => format!("{subj}!!"),
        }
    }

    /// [let-infer] [rewrap] Maps a value from one union representation to
    /// another: each source arm to the target arm of the same type, and an arm
    /// the target does not have to an unreachable. Factored out of the `Rewrap`
    /// coercion so the sites that produce a **sub-union value with no slot of
    /// its own** can call it — a multi-arm lift binding, and a pick's picked or
    /// unpicked side [qual-lift] [pick].
    fn emit_rewrap(&mut self, code: String, from: &Ty, to: &Ty) -> String {
                let from_arms: Vec<Ty> = from.value_arms().into_iter().cloned().collect();
                let to_arms: Vec<Ty> = to.value_arms().into_iter().cloned().collect();
                let n = from_arms.len();
                let m = to_arms.len();
                self.union_sizes.insert(n);
                self.union_sizes.insert(m);
                let stars = vec!["*"; n].join(", ");
                let to_args: Vec<String> =
                    to_arms.iter().map(|a| self.emit_ty(a)).collect();
                let to_args = to_args.join(", ");
                let mut branches = String::new();
                for (i, fa) in from_arms.iter().enumerate() {
                    match to_arms.iter().position(|ta| ta == fa) {
                        Some(j) => {
                            let cast = self.emit_ty(fa);
                            branches.push_str(&format!(
                                "is U{n}_{}<{stars}> -> U{m}_{}<{to_args}>(it.value as {cast}); ",
                                i + 1,
                                j + 1
                            ));
                        }
                        None => {
                            branches.push_str(&format!(
                                "is U{n}_{}<{stars}> -> throw IllegalStateException(\"unreachable union arm\"); ",
                                i + 1
                            ));
                        }
                    }
                }
                if from.has_none_arm() {
                    if to.has_none_arm() {
                        branches.push_str("null -> null; ");
                    } else {
                        branches.push_str(
                            "null -> throw IllegalStateException(\"unreachable union arm\"); ",
                        );
                    }
                }
                format!("{code}.let {{ when (it) {{ {branches}}} }}")
    }

    fn emit_union_test(&mut self, subj: &str, test: &UnionTest) -> String {
        if test.match_none {
            return format!("{subj} == null");
        }
        if test.size == 1 {
            // Nullable `T?` representation.
            return if test.arms.is_empty() {
                "false".to_string()
            } else {
                format!("{subj} != null")
            };
        }
        self.union_sizes.insert(test.size);
        if test.arms.is_empty() {
            return "false".to_string();
        }
        if test.arms.len() == test.size {
            return if test.nullable {
                format!("{subj} != null")
            } else {
                "true".to_string()
            };
        }
        let stars = vec!["*"; test.size].join(", ");
        let parts: Vec<String> = test
            .arms
            .iter()
            .map(|i| format!("{subj} is U{}_{}<{stars}>", test.size, i + 1))
            .collect();
        if parts.len() == 1 {
            parts.into_iter().next().unwrap()
        } else {
            format!("({})", parts.join(" || "))
        }
    }

    /// The runtime call chain for a predicate-qualifier `is` check on a
    /// non-union subject [is-qualifies]: each qualifier's `qualifies`
    /// function is invoked (with its effect handlers threaded as leading
    /// arguments [is-qualifies-effects]).
    fn emit_predicate_test(&mut self, subj: &str, quals: &[PredicateCheck]) -> String {
        let mut parts: Vec<String> = Vec::new();
        for check in quals {
            let q = &check.name;
            let mut args: Vec<String> = Vec::new();
            // [qual-overload] Same-named qualifiers over different subjects
            // need no mangling here — the JVM overloads on the parameter type
            // — but the *effects* threaded into the call are the resolved
            // declaration's, so the subject still picks.
            if let Some(decl) = self
                .symbols
                .qualifiers
                .get(q.as_str())
                .and_then(|ds| ds.first())
                .copied()
            {
                if let Some(f) = decl.fns.iter().find(|f| f.name.name == "qualifies") {
                    for eff in f.effects.iter().flatten() {
                        if let EffectRef::Effect(r) | EffectRef::LocalEffect(r) = eff {
                            let ty = self.emit_type_ref(r);
                            args.push(self.thread_effect_fused_by_type(&ty));
                        }
                    }
                    // [kt-effect-fusion] One fused carrier covers the set.
                    if self.fusion {
                        args.dedup();
                        args.truncate(1);
                    }
                }
            } else {
                self.error(format!("unknown qualifier `{q}` in predicate check"));
            }
            args.push(subj.to_string());
            // [qual-depend] The places filling a dependent qualifier's value
            // slots trail the subject, in slot order: `KeyOf_qualifies(k, m)`.
            for a in &check.args {
                args.push(a.path.clone());
            }
            parts.push(format!("{q}_qualifies({})", args.join(", ")));
        }
        if parts.len() == 1 {
            parts.into_iter().next().unwrap()
        } else {
            format!("({})", parts.join(" && "))
        }
    }

    /// A binary/unary operand, parenthesized when its precedence is lower
    /// than the parent operator's (Salvo's parser preserved the grouping;
    /// flat re-rendering must not change it). `is`, lambdas, and
    /// `if`/`when` expressions always parenthesize in operand position
    /// (Kotlin would otherwise swallow the trailing operator chain).
    fn emit_operand(&mut self, expr: &Expr, parent_prec: u8) -> String {
        let code = self.emit_expr(expr);
        match expr {
            Expr::Binary { op, .. } if bin_prec(*op) <= parent_prec => format!("({code})"),
            Expr::Is { .. }
            | Expr::Widen { .. }
            | Expr::Lambda { .. }
            | Expr::If { .. }
            | Expr::When { .. }
            | Expr::WhenCond { .. } => {
                format!("({code})")
            }
            _ => code,
        }
    }

    /// Left operands of the same precedence stay flat (left-assoc).
    fn emit_operand_left(&mut self, expr: &Expr, parent_prec: u8) -> String {
        let code = self.emit_expr(expr);
        match expr {
            Expr::Binary { op, .. } if bin_prec(*op) < parent_prec => format!("({code})"),
            Expr::Is { .. }
            | Expr::Widen { .. }
            | Expr::Lambda { .. }
            | Expr::If { .. }
            | Expr::When { .. }
            | Expr::WhenCond { .. } => {
                format!("({code})")
            }
            _ => code,
        }
    }

    fn emit_expr_raw(&mut self, expr: &Expr) -> String {
        match expr {
            // [safe-call] [kt-safe-call] Kotlin's `?.` cannot be used directly:
            // Salvo's dot-notation is a *free function* call, so `xs?.size()` is
            // `size(xs)` guarded on `xs`. The receiver is named once with `let`
            // and the inner access emitted inside it, which is Kotlin's own idiom
            // for the same thing (`xs?.let { … }`) and keeps the single
            // evaluation.
            Expr::SafeField { base, inner, .. } => {
                // The receiver is a place [safe-call], so the test reads the
                // storage and the member reads the narrowed payload — the two
                // reads the checker allowed by requiring a place.
                let subj = self.emit_place_storage(base);
                let body = self.emit_expr(inner);
                format!("(if ({subj} != null) {body} else null)")
            }
            // [elvis] [kt-elvis] Kotlin's own `?:` is this operator, because the
            // `T?` representation is a Kotlin nullable: the subject is `null`
            // exactly when Salvo says `None`. So the lowering is direct, and a
            // `return` on the right is legal there for the same reason it is in
            // Salvo since step 2.
            // [pick] The qualifier form: the arm test the checker recorded,
            // the picked payload read, and the unpicked payload for `_` — the
            // narrowed-read machinery `is` already uses, with a conditional
            // around it. Kotlin's own `?:` cannot serve here: the test is an arm
            // test, not a null test.
            Expr::Elvis {
                subject,
                pick: Some(_),
                rhs,
                span,
            } => {
                // [is-bind-once] The subject is evaluated **once**, into a
                // temporary registered the way a hoisted `is` subject is. `run`
                // is inline, so a `return` on the right side still returns from
                // the enclosing function [expr-escape].
                self.pick_vars += 1;
                let tmp = format!("__pick{}", self.pick_vars);
                let subj_code = self.emit_expr(subject);
                let saved_subject = self
                    .is_temps
                    .insert((self.file_idx, subject.span()), tmp.clone());
                let test = self.is_test_of(*span).cloned();
                let cond = match &test {
                    Some(t) => self.emit_union_test(&tmp, t),
                    None => "true".to_string(),
                };
                let picked = self.checked.elvis_picks.get(&(self.file_idx, *span)).cloned();
                // [rewrap] A side spanning several arms is a *sub-union*: mapped
                // arm to arm out of the storage, not read as one payload.
                let body = match (
                    self.checked.rewrap_from.get(&(self.file_idx, *span)).cloned(),
                    picked.clone(),
                ) {
                    (Some(from), Some(to)) => {
                        let storage = self.emit_place_storage(subject);
                        self.emit_rewrap(storage, &from, &to)
                    }
                    _ => self.emit_arm_payload(subject, test.as_ref(), picked.as_ref()),
                };
                let left = self.checked.pick_left.get(&(self.file_idx, *span)).cloned();
                let else_test = self
                    .checked
                    .pick_else_tests
                    .get(&(self.file_idx, *span))
                    .cloned();
                let else_read = match (
                    self.checked.pick_left_from.get(&(self.file_idx, *span)).cloned(),
                    left.clone(),
                ) {
                    (Some(from), Some(to)) => {
                        let storage = self.emit_place_storage(subject);
                        self.emit_rewrap(storage, &from, &to)
                    }
                    _ => self.emit_arm_payload(subject, else_test.as_ref(), left.as_ref()),
                };
                let saved = self.placeholder_code.replace(else_read);
                let r = self.emit_expr(rhs);
                self.placeholder_code = saved;
                match saved_subject {
                    Some(prev) => {
                        self.is_temps.insert((self.file_idx, subject.span()), prev);
                    }
                    None => {
                        self.is_temps.remove(&(self.file_idx, subject.span()));
                    }
                }
                format!("run {{ val {tmp} = {subj_code}; if ({cond}) {body} else {r} }}")
            }
            Expr::Elvis { subject, rhs, .. } => {
                let s = self.emit_expr(subject);
                let r = self.emit_expr(rhs);
                format!("({s} ?: {r})")
            }
            // [placeholder] The unpicked arm, read out of the subject.
            Expr::Placeholder { .. } => self
                .placeholder_code
                .clone()
                .unwrap_or_else(|| "null".to_string()),
            // [expr-escape] An escape *nested inside* an expression — the form
            // statement position never produces, since `emit_stmt` takes those.
            // Kotlin has the same three as expressions, so the rendering is
            // direct; a `break`/`continue` carrying a loop-result assignment or
            // a `return` under an owed cleanup belongs to statement position,
            // which is where those rules live.
            Expr::Return { .. } | Expr::Break { .. } | Expr::Continue { .. } => {
                let value = match expr {
                    Expr::Return { value, .. } | Expr::Break { value, .. } => value.as_deref(),
                    _ => None,
                };
                let word = match expr {
                    Expr::Return { .. } => "return",
                    Expr::Break { .. } => "break",
                    _ => "continue",
                };
                match value {
                    Some(v) => {
                        let code = self.emit_expr(v);
                        format!("{word} {code}")
                    }
                    None => word.to_string(),
                }
            }
            // Literal suffixes map 1:1 onto Kotlin's [lit-numeric]:
            // `1L` -> `1L` (Long), `1.2f` -> `1.2f` (Float).
            // [lit-numeric] [lit-adopt] Suffixes render explicitly (`1L`,
            // `1.2f`), and an *unsuffixed* literal renders at its
            // **checked** type: `let x: Long = 1` emits `1L` (a bare `1`
            // does not conform to a `Long` parameter on the JVM), `let d:
            // Double = 3` emits `3.0`, `let f: Float = 0.5` emits `0.5f`.
            Expr::Int { value, long, span } => {
                if *long {
                    format!("{value}L")
                } else {
                    match self.ty_of(*span).map(|t| t.strip_quals()) {
                        Some(Ty::Named { name, .. }) if name == "Long" => format!("{value}L"),
                        Some(Ty::Named { name, .. }) if name == "Double" => format!("{value}.0"),
                        Some(Ty::Named { name, .. }) if name == "Float" => format!("{value}.0f"),
                        _ => value.to_string(),
                    }
                }
            }
            Expr::Float { value, single, span } => {
                let s = value.to_string();
                let s = if s.contains('.') { s } else { format!("{s}.0") };
                let float = *single
                    || matches!(
                        self.ty_of(*span).map(|t| t.strip_quals()),
                        Some(Ty::Named { name, .. }) if name == "Float"
                    );
                format!("{s}{}", if float { "f" } else { "" })
            }
            Expr::Bool { value, .. } => value.to_string(),
            Expr::Char { value, .. } => format!("'{}'", escape_char(*value)),
            Expr::Str { parts, .. } => self.emit_string(parts),
            Expr::Ident(id) => {
                if id.name == "None" {
                    "null".to_string()
                } else if !self.taken_names.contains(&id.name)
                    && self
                        .checked
                        .fn_refs
                        .contains_key(&(self.file_idx, id.span))
                {
                    // Passing a named fn by value [fn-contract]: Kotlin
                    // needs the function-reference syntax — unless the
                    // position expects a different effect list than the fn
                    // declares [fn-effects], in which case an adapter lambda
                    // takes what the caller passes and forwards what the fn
                    // needs (a pure fn ignores the rest).
                    self.named_fn_value(&id.name, id.span)
                } else if self.gen_slots.contains(id.name.as_str()) {
                    // [iter-fn] A pass property with no zero value is
                    // nullable; the machine assigns it before every read.
                    format!("{}!!", kt_ident(&id.name))
                } else {
                    kt_ident(&id.name)
                }
            }
            // [fn-overload-at] [fn-value-select] A scope-selected fn *value*
            // (`describe@main`): the selector is erased — the checker already
            // recorded which declaration it means — so this is the ordinary
            // function reference.
            Expr::Scoped { name, .. } => self.named_fn_value(&name.name, name.span),
            // [effect-at] Checker-refused as a value — *unless* the capitalized
            // name is a type, which makes this a canonical named as a value
            // ([cmp-canonical] `cmp = cmp@Person`): the checker resolved a
            // function, so the selector is erased like any other.
            Expr::EffectScoped { name, .. } => {
                if self
                    .checked
                    .fn_refs
                    .contains_key(&(self.file_idx, name.span))
                {
                    self.named_fn_value(&name.name, name.span)
                } else {
                    self.error(format!(
                        "internal error: `{}@Effect` reached the Kotlin emitter as a \
                         value (the checker refuses member values)",
                        name.name
                    ));
                    "TODO()".to_string()
                }
            }
            Expr::Field { base, field, span } => {
                let code = format!("{}.{}", self.emit_expr(base), kt_ident(&field.name));
                // Predicate-qualifier field overrides cast + assert.
                if let Some(cast_ty) = self.checked.field_casts.get(&(self.file_idx, *span)) {
                    let cast_ty = cast_ty.clone();
                    let kt = self.emit_ty(&cast_ty);
                    return format!("({code} as {kt})");
                }
                code
            }
            Expr::TupleIndex { base, index, .. } => {
                // [kt-tuple-component] `Pair`/`Triple` and the generated
                // `SalvoTupleN` name their elements alike.
                let code = self.emit_expr(base);
                format!("{code}.{}", Self::tuple_component(*index))
            }
            Expr::Call {
                callee,
                type_args,
                args,
                named,
                span,
            } => self.emit_call(callee, type_args, args, named, *span),
            Expr::Index { base, index, .. } => {
                format!("{}[{}]", self.emit_expr(base), self.emit_expr(index))
            }
            // [col-literal] `[1, 2]` is a **List** literal; it renders as an
            // `arrayOf` only where the checker typed it as an array, which is
            // a literal standing in a variadic position [fn-variadic]. A
            // `Mut` list literal needs the mutable builder, since
            // `MutableList` is what the declared type will be.
            Expr::ArrayLit { elems, span } => {
                let items: Vec<String> = elems.iter().map(|e| self.emit_expr(e)).collect();
                let ty = self.ty_of(*span).cloned();
                let is_array = matches!(ty.as_ref().map(|t| t.strip_quals()), Some(Ty::Array(_)));
                if is_array {
                    format!("arrayOf({})", items.join(", "))
                } else {
                    let mutable = ty
                        .as_ref()
                        .is_some_and(|t| t.quals().iter().any(|q| q.name == "Mut"));
                    let elem = match ty.as_ref().map(|t| t.strip_quals()) {
                        Some(Ty::Named { name, args }) if name == "List" && args.len() == 1 => {
                            self.emit_ty(&args[0])
                        }
                        _ => "Any".to_string(),
                    };
                    if mutable {
                        format!("mutableListOf<{}>({})", elem, items.join(", "))
                    } else {
                        format!("listOf<{}>({})", elem, items.join(", "))
                    }
                }
            }
            // [col-literal] The brace literals lower to the same ordered
            // constructors `set_of`/`map_of` use [col-insertion-order], with
            // the element types spelled out because kotlinc cannot infer
            // them from an empty literal.
            Expr::SetLit { elems, span } => {
                let items: Vec<String> = elems.iter().map(|e| self.emit_expr(e)).collect();
                // [col-literal] An empty `{}` takes its kind from the
                // position; the checker resolved it, so follow the checked
                // type rather than the node.
                match self.ty_of(*span).map(|t| t.strip_quals()) {
                    Some(Ty::Named { name, args }) if name == "Map" && args.len() == 2 => {
                        let (kt, vt) = (self.emit_ty(&args[0]), self.emit_ty(&args[1]));
                        match self.keyed_pair(*span) {
                            Some((h, e)) => {
                                format!("salvo.SalvoHashMap<{kt}, {vt}>({h}, {e})")
                            }
                            None => format!("linkedMapOf<{}, {}>()", kt, vt),
                        }
                    }
                    Some(Ty::Named { name, args }) if name == "List" && args.len() == 1 => {
                        let elem = self.emit_ty(&args[0]);
                        let mutable = self
                            .ty_of(*span)
                            .is_some_and(|t| t.quals().iter().any(|q| q.name == "Mut"));
                        if mutable {
                            format!("mutableListOf<{}>({})", elem, items.join(", "))
                        } else {
                            format!("listOf<{}>({})", elem, items.join(", "))
                        }
                    }
                    Some(Ty::Named { name, args }) if name == "Set" && args.len() == 1 => {
                        {
                            let elem = self.emit_ty(&args[0]);
                            match self.keyed_pair(*span) {
                                Some((h, e)) => format!(
                                    "salvo.SalvoHashSet<{elem}>({h}, {e}).also {{ __s -> \
                                     __s.addAll(listOf({})) }}",
                                    items.join(", ")
                                ),
                                None => format!("linkedSetOf<{elem}>({})", items.join(", ")),
                            }
                        }
                    }
                    _ => format!("linkedSetOf<Any>({})", items.join(", ")),
                }
            }
            Expr::MapLit { entries, span } => {
                let items: Vec<String> = entries
                    .iter()
                    .map(|(k, v)| format!("({} to {})", self.emit_expr(k), self.emit_expr(v)))
                    .collect();
                let (kt, vt) = match self.ty_of(*span).map(|t| t.strip_quals()) {
                    Some(Ty::Named { name, args }) if name == "Map" && args.len() == 2 => {
                        (self.emit_ty(&args[0]), self.emit_ty(&args[1]))
                    }
                    _ => ("Any".to_string(), "Any".to_string()),
                };
                match self.keyed_pair(*span) {
                    Some((h, e)) => format!(
                        "salvo.SalvoHashMap<{kt}, {vt}>({h}, {e}).also {{ __m -> \
                         __m.putAll(listOf({})) }}",
                        items.join(", ")
                    ),
                    None => format!("linkedMapOf<{}, {}>({})", kt, vt, items.join(", ")),
                }
            }
            Expr::Tuple { elems, .. } => {
                let items: Vec<String> = elems.iter().map(|e| self.emit_expr(e)).collect();
                // [kt-tuple-class] `Pair`/`Triple` for two and three, a
                // generated `SalvoTupleN` past them.
                match items.len() {
                    2 => format!("Pair({})", items.join(", ")),
                    3 => format!("Triple({})", items.join(", ")),
                    n => {
                        self.tuple_sizes.insert(n);
                        self.needs_compare = true;
                        format!("SalvoTuple{n}({})", items.join(", "))
                    }
                }
            }
            Expr::StructLit { ty, fields, span } => {
                self.emit_struct_lit(ty.as_ref(), fields, *span)
            }
            Expr::Unary { op, operand, .. } => {
                let inner = self.emit_operand(operand, 7);
                match op {
                    UnaryOp::Neg => format!("-{inner}"),
                    UnaryOp::Not => format!("!{inner}"),
                }
            }
            Expr::Binary { op, lhs, rhs, span } => {
                // [op-order] [op-equality] A comparison the checker resolved to
                // a `cmp`/`eq` **is** that call. Absent from the table means the
                // host's own operator — numerics, and equality at the other
                // intrinsic types.
                if let Some(via) = self
                    .checked
                    .comparisons
                    .get(&(self.file_idx, *span))
                    .cloned()
                {
                    return self.emit_compare_via(*op, lhs, rhs, &via, *span);
                }
                let prec = bin_prec(*op);
                let l = self.emit_operand_left(lhs, prec);
                let r = self.emit_operand(rhs, prec);
                // [op-promote] [kt-op-promote] Kotlin's own operators cover
                // the mixed widths for arithmetic and ordering (`Long + Int`,
                // `Long < Int`), so a promotion needs no rendering there — but
                // **equality is type-strict**: `Long == Int` is refused by
                // kotlinc. Since equality promotes too (user decision
                // 2026-09-18), the narrower operand is converted explicitly
                // wherever the checker recorded a widening.
                if matches!(op, BinaryOp::Eq | BinaryOp::NotEq) {
                    let l = self.promote_operand(lhs.span(), l);
                    let r = self.promote_operand(rhs.span(), r);
                    return format!("{l} {} {r}", binary_op(*op));
                }
                format!("{l} {} {r}", binary_op(*op))
            }
            // [qual-lift] The dual of `is`: the *same* runtime test (the
            // qualifier is erased, so widening is a typing act), or `true`
            // when the qualifiers are statically present and nothing has to
            // be tested.
            Expr::Widen { subject, span, .. } => {
                let subj = self.emit_place_storage(subject);
                match self.is_test_of(*span) {
                    Some(test) => {
                        let test = test.clone();
                        self.emit_union_test(&subj, &test)
                    }
                    None => "true".to_string(),
                }
            }
            Expr::Is {
                subject, check, span, ..
            } => {
                // The test reads the storage, never a narrowed payload
                // [flow-place].
                let subj = self.emit_place_storage(subject);
                if let Some(test) = self.is_test_of(*span) {
                    let test = test.clone();
                    return self.emit_union_test(&subj, &test);
                }
                if let Some(quals) = self.checked.predicate_tests.get(&(self.file_idx, *span)) {
                    let quals = quals.clone();
                    return self.emit_predicate_test(&subj, &quals);
                }
                if check.len() == 1 && check[0].name.name == "None" {
                    return format!("{subj} == null");
                }
                let ty = self.emit_is_check_type(check);
                format!("{subj} is {ty}")
            }
            // [assert-trap] [kt-assert-trap] A failed assertion is *Salvo's*
            // failure, not the host's: the message names the Salvo source and
            // reads identically on both backends (user decision 2026-09-23,
            // A-2). The mechanism stays each host's own trap — an
            // `AssertionError` here, a panic on Rust — since neither program is
            // meant to continue.
            Expr::NonNull { operand, span } => format!(
                "({} ?: throw AssertionError({}))",
                self.emit_expr(operand),
                self.trap_message("value is absent", None, *span)
            ),
            Expr::Assert {
                cond,
                message,
                span,
            } => {
                let text = self.trap_message(
                    "assertion failed",
                    message.as_deref(),
                    *span,
                );
                format!(
                    "(if (!({})) throw AssertionError({}) else Unit)",
                    self.emit_expr(cond),
                    text
                )
            }
            Expr::Unreachable { message, span } => format!(
                "throw AssertionError({})",
                self.trap_message("unreachable", message.as_deref(), *span)
            ),
            // [inc-dec] [kt-inc-dec] Kotlin has both fixities and both
            // directions, with the same value semantics, so this is a direct
            // rendering.
            Expr::IncDec { operand, down, prefix, .. } => {
                let place = self.emit_expr_raw(operand);
                let op = if *down { "--" } else { "++" };
                if *prefix {
                    format!("{op}{place}")
                } else {
                    format!("{place}{op}")
                }
            }
            Expr::If {
                branches,
                else_block,
                ..
            } => {
                // Captured before emitting: statement emission inside the
                // branches moves `expr_indent`.
                let indent = self.expr_indent;
                self.emit_if_expr(branches, else_block.as_ref(), indent)
            }
            Expr::Lambda { params, body, span } => self.emit_lambda(params, body, *span),
            Expr::Try { body, span } => self.emit_try(body, *span),
            Expr::Spread { operand, .. } => format!("*{}", self.emit_expr(operand)),
            Expr::While { .. } | Expr::For { .. } => self.emit_loop_value(expr),
            Expr::When {
                subject, branches, ..
            } => {
                let indent = self.expr_indent;
                self.emit_when(subject, branches, indent, true)
            }
            Expr::WhenCond {
                branches,
                else_block,
                ..
            } => {
                let indent = self.expr_indent;
                self.emit_when_cond(branches, else_block, indent, true)
            }
            Expr::Error { .. } => "TODO()".to_string(),
            // [actor-spawn-expr] [kt-actor] Construct the handler, wrap it
            // in its generated actor body, hand it to the scheduler; the
            // value is the addr.
            Expr::Spawn {
                handler,
                with_items,
                pool,
                span,
            } => self.emit_spawn(handler, with_items, pool.as_deref(), *span),
            // [actor-waitfor] `main`'s bridge, as a `run { }` expression.
            Expr::WaitFor {
                binding,
                ty,
                body,
                span,
            } => self.emit_waitfor(binding, ty, body, *span),
            // [actor-self-send] A bare selector outside a call is a checker
            // error; the call form is lowered in `emit_call`.
            Expr::SelfScoped { .. } => {
                self.error("internal: `@self` outside a call reached emission");
                "TODO()".to_string()
            }
            // [actor-replyto] The mint: allocate a slot, park the
            // continuation, hand back the token.
            Expr::ReplyTo {
                member,
                captures,
                gated,
                pool,
                span,
            } => self.emit_replyto(member, captures, *gated, pool.as_deref(), *span),
        }
    }

    /// [actor-replyto] [kt-actor] `replyto k(captures)` → mint a slot on
    /// **this actor**, store `__Cont_E.K(captures)` under it, and evaluate to
    /// the token:
    ///
    /// ```text
    /// run { val (r, s) = SalvoSched.mint(handlerAddr); __parked[s] = __Cont_E.K(caps); r }
    /// ```
    ///
    /// `replyto!` differs only in the mint (`mintGated`), which is where the
    /// gate lives — the runtime's business, not the emitter's.
    ///
    /// The `!!` cannot fire: a handler that mints may only be spawned
    /// ([actor-replyto], checked), so its members run as activations and
    /// `__addr` was written before the body ran.
    fn emit_replyto(
        &mut self,
        member: &Ident,
        captures: &[Expr],
        gated: bool,
        pool: Option<&Expr>,
        span: Span,
    ) -> String {
        self.needs_scheduler = true;
        // [task-mint] [kt-task] A mint whose target is a free `send fn` needs
        // no continuation class, no slot and no parked table: the lambda *is*
        // the continuation, and the runtime schedules it on the pool the mint
        // chose.
        if let Some(key) = self
            .checked
            .replyto_tasks
            .get(&(self.file_idx, span))
            .copied()
        {
            return self.emit_task_mint(member, key, captures, pool);
        }
        let Some(target) = self
            .checked
            .replyto_members
            .get(&(self.file_idx, span))
            .cloned()
        else {
            self.error(format!(
                "internal: no continuation target recorded for `replyto {}`",
                member.name
            ));
            return "TODO()".to_string();
        };
        let Some(cont) = self.current_cont_type() else {
            self.error(format!(
                "internal: `replyto {}` has no continuation class in scope",
                member.name
            ));
            return "TODO()".to_string();
        };
        let variant = msg_variant_name(&target);
        let caps: Vec<String> = captures.iter().map(|c| self.emit_expr(c)).collect();
        let mint = if gated { "mintGated" } else { "mint" };
        format!(
            "run {{ val (__r, __s) = salvo.SalvoSched.{mint}(__addr!!);              __parked[__s] = {cont}.{variant}({}); __r }}",
            caps.join(", ")
        )
    }

    /// [task-mint] [kt-task] `replyto k(caps) on P` where `k` is a free
    /// `send fn`: `run { val __c0 = …; SalvoSched.mintTask(P) { __v ->
    /// k(__c0, __v as Payload) } }`.
    ///
    /// The captures are bound to `val`s *outside* the lambda, which is what
    /// makes them the values as they were at the mint rather than at the
    /// answer. The payload is cast to the target's trailing parameter type, the
    /// same way the `waitfor` bridge casts what a waiter was sent.
    fn emit_task_mint(
        &mut self,
        member: &Ident,
        key: salvo_core::FnKey,
        captures: &[Expr],
        pool: Option<&Expr>,
    ) -> String {
        let Some(target) = self.fn_by_key(key) else {
            self.error(format!(
                "internal: no declaration for the task target `{}`",
                member.name
            ));
            return "TODO()".to_string();
        };
        let params: Vec<&Param> = target.params.iter().filter(|p| !p.implicit).collect();
        let Some(last) = params.last() else {
            self.error(format!(
                "internal: task target `{}` has no answer parameter",
                member.name
            ));
            return "TODO()".to_string();
        };
        let payload = self.emit_type(&last.ty);
        let name = self.kotlin_fn_name(target);
        let mut lets = String::new();
        let mut args: Vec<String> = Vec::new();
        for (i, c) in captures.iter().enumerate() {
            let code = self.emit_expr(c);
            lets.push_str(&format!("val __c{i} = {code}; "));
            args.push(format!("__c{i}"));
        }
        args.push(format!("__v as {payload}"));
        // [task-pool-inherit] An omitted `on` clause is the pool current where
        // the mint runs.
        let pool_code = match pool {
            Some(p) => self.emit_expr(p),
            None => "salvo.SalvoSched.currentPool()".to_string(),
        };
        format!(
            "run {{ {lets}salvo.SalvoSched.mintTask({pool_code}) {{ __v -> {name}({}) }} }}",
            args.join(", ")
        )
    }

    /// [actor-replyto] The `__Cont_E` class for the handler whose member is
    /// being emitted — the enclosing handler, since a mint is lexical.
    fn current_cont_type(&self) -> Option<String> {
        let name = self.current_handler.as_deref()?;
        let h = self.symbols.handlers.get(name)?;
        self.handler_cont_type(h)
    }

    /// [actor-self-send] [kt-actor] `k@self(args)` — send to **the actor the
    /// enclosing member belongs to**, and its two readings, chosen at run time
    /// off `__addr` because a handler is compiled once and bound many ways: an
    /// enqueue on its own mailbox when this instance is an actor, and the
    /// ordinary inline member call when it was bound with `use`.
    ///
    /// Kotlin needs no fusion care here that Rust needed: a member call on
    /// `this` reaches the handler's own dependencies through the stored carrier
    /// [kt-effect-fusion].
    fn emit_self_send(&mut self, member: &str, args: &[Expr]) -> String {
        self.needs_scheduler = true;
        let Some(handler) = self.current_handler.clone() else {
            self.error("internal: a self-send outside a handler member");
            return "TODO()".to_string();
        };
        let Some(decl) = self.symbols.handlers.get(handler.as_str()).copied() else {
            self.error(format!("internal: no handler `{handler}` for a self-send"));
            return "TODO()".to_string();
        };
        let payload: Vec<String> = args.iter().map(|a| self.emit_expr(a)).collect();
        // [mixed-handler] [kt-mixed] A mixed servant's self-send — a bare
        // sibling call or `k@self(…)` in a send member (user decision
        // 2026-09-19): the message class is the *handler's* (`__Msg_H`),
        // and the enqueue is unconditional — a mixed handler is spawn-only,
        // so its send members always run as activations and `__addr` is
        // written before the body runs.
        if self.handler_is_mixed(decl) {
            let msg = msg_class_name(&handler);
            let variant = msg_variant_name(member);
            // An empty payload is an `object` subclass of `__Msg_H`.
            let built = if payload.is_empty() {
                format!("{msg}.{variant}")
            } else {
                format!("{msg}.{variant}({})", payload.join(", "))
            };
            return format!("salvo.SalvoSched.send(__addr!!, {built})");
        }
        // [effect-handler-multi] The message class is the *face's*, so the
        // protocol is the one whose members include this one.
        let Some(effect_name) = self
            .handler_faces(decl)
            .into_iter()
            .find(|e| e.fns.iter().any(|f| f.name.name == member))
            .map(|e| e.name.name.clone())
        else {
            self.error(format!(
                "internal: handler `{handler}` has no face declaring `{member}`"
            ));
            return "TODO()".to_string();
        };
        let msg = msg_class_name(&effect_name);
        let variant = msg_variant_name(member);
        format!(
            "run {{ val __a = __addr; if (__a != null)              salvo.SalvoSched.send(__a, {msg}.{variant}({})) else this.{member}({}) }}",
            payload.join(", "),
            payload.join(", ")
        )
    }

    /// The Kotlin type used for fallback `is` checks (non-union subjects).
    /// Union subjects are lowered through the checker's `is_tests` table
    /// before reaching this.
    fn emit_is_check_type(&mut self, check: &[TypeRef]) -> String {
        if check.len() > 1 {
            self.error(
                "qualifier checks in `is` are only supported on union-typed subjects",
            );
        }
        let base = check.last().expect("is-check has at least one ref");
        self.emit_type_ref(base)
    }

    fn emit_string(&mut self, parts: &[StrExprPart]) -> String {
        let mut out = String::from("\"");
        for part in parts {
            match part {
                StrExprPart::Text(text) => out.push_str(&escape_string(text)),
                StrExprPart::Interp(expr) => {
                    let mut code = self.emit_expr(expr);
                    // Union-wrapped values interpolate their payload.
                    if let Some(repr) = self.emitted_repr(expr) {
                        if repr.is_wrapper_union() {
                            let access = if repr.has_none_arm() { "?" } else { "" };
                            code = format!("{code}{access}.value");
                        }
                    }
                    // [interp-to-str] [kt-interp-to-str] A value with no
                    // native text form is rendered by the `to_str` the
                    // checker resolved here.
                    code = self.apply_interp_to_str(expr, code);
                    // Simple names can use the short form.
                    if code.chars().all(|c| c.is_alphanumeric() || c == '_') {
                        out.push_str(&format!("${code}"));
                    } else {
                        out.push_str(&format!("${{{code}}}"));
                    }
                }
            }
        }
        out.push('"');
        out
    }

    /// An `if`/`elif`/`else` chain in value position. `indent` is the column
    /// of the *statement* the expression sits in: the opening `if (` is
    /// written inline after whatever precedes it, so only the branch bodies
    /// and the closing braces need padding.
    fn emit_if_expr(
        &mut self,
        branches: &[(Expr, Block)],
        else_block: Option<&Block>,
        indent: usize,
    ) -> String {
        let pad = "    ".repeat(indent);
        // [is-bind-once] An `if` *expression* has nowhere to put a statement,
        // so a hoisted subject wraps the whole thing: `run { val __is1 = …; if
        // … }`. Only the first branch can need it — a later one is refused by
        // the checker.
        let hoist = branches
            .first()
            .and_then(|(cond, _)| self.hoistable_is(cond))
            .map(|subject| self.emit_is_temp(subject, indent + 1));
        let mut out = String::new();
        if let Some(temp) = &hoist {
            out.push_str("run {\n");
            out.push_str(temp);
            out.push_str(&pad);
        }
        for (i, (cond, block)) in branches.iter().enumerate() {
            let kw = if i == 0 { "if" } else { " else if" };
            let c = self.emit_expr(cond);
            out.push_str(&format!("{kw} ({c}) {{\n"));
            out.push_str(&self.emit_is_bindings(cond, indent + 1));
            out.push_str(&self.emit_widen_shadows(cond, indent + 1));
            out.push_str(&self.emit_value_block(block, indent + 1));
            out.push_str(&format!("{pad}}}"));
        }
        match else_block {
            Some(block) => {
                out.push_str(" else {\n");
                out.push_str(&self.emit_value_block(block, indent + 1));
                out.push_str(&format!("{pad}}}"));
            }
            None => {
                // A missing else means the expression's value is None.
                let inner = "    ".repeat(indent + 1);
                out.push_str(&format!(" else {{\n{inner}null\n{pad}}}"));
            }
        }
        // [is-bind-once] Close the `run { }` the hoisted subject opened.
        if hoist.is_some() {
            out.push_str(&format!("\n{pad}}}"));
        }
        out
    }

    /// A block in value position: all statements plus the trailing
    /// expression as the block's value. `indent` is the column its
    /// statements sit at.
    fn emit_value_block(&mut self, block: &Block, indent: usize) -> String {
        // [kt-effect-fusion] Save the whole environment, not just its
        // depth: a `use` inside the scope *rebases* enclosing entries onto
        // its fused value, and that mutation must roll back with the scope.
        let saved_effect_env = self.effect_env.clone();
        let out = self.emit_value_stmts(&block.stmts, indent);
        self.effect_env = saved_effect_env;
        out
    }

    /// The statements of a value-position block.
    fn emit_value_stmts(&mut self, stmts: &[Stmt], indent: usize) -> String {
        let pad = "    ".repeat(indent);
        let mut out = String::new();
        let n = stmts.len();
        for (i, stmt) in stmts.iter().enumerate() {
            // Kotlin loops are never expressions, so a trailing loop (the
            // block's value [while-value]) needs the value lowering.
            if i + 1 == n {
                if let Stmt::Expr(e @ (Expr::While { .. } | Expr::For { .. })) = stmt {
                    self.expr_indent = indent;
                    let code = self.emit_expr(e);
                    out.push_str(&format!("{pad}{code}\n"));
                    continue;
                }
            }
            out.push_str(&self.emit_stmt(stmt, indent));
        }
        out
    }

    // ================= throw and `try` [kt-throw-signal] =================

    /// `try { ... }` [kt-throw-signal]: Kotlin's own `try`/`catch`, which is
    /// an *expression*, so the outcome falls out of it — the body's value
    /// wrapped in the `Ok` arm, or the caught signal's message wrapped in
    /// the `Thrown` arm. Catching the innermost signal is exactly
    /// [try-innermost]; a `finally` a loop's release put inside the body runs
    /// while unwinding [kt-exit-finally].
    fn emit_try(&mut self, body: &Block, span: Span) -> String {
        self.needs_throw = true;
        let outcome = self.ty_of(span).cloned();
        if let Some(ty) = &outcome {
            let arms = ty.value_arms().len();
            if arms >= 2 {
                self.union_sizes.insert(arms);
            }
        }
        let pad = "    ".repeat(self.expr_indent);
        let inner_pad = "    ".repeat(self.expr_indent + 1);
        let inner = self.emit_try_body(body, outcome.as_ref());
        let message_ty = outcome
            .as_ref()
            .and_then(|ty| ty.value_arms().get(1).map(|a| (*a).clone()));
        // The caught payload becomes the outcome's thrown arm. With one
        // message type that is a cast; with several, the signal's tag says
        // which arm of `M` it is [kt-throw-signal] — a tag from outside this
        // delimiter's set is rethrown rather than mis-wrapped
        // [backend-never-wrong].
        let stripped = message_ty
            .as_ref()
            .map(|m| m.clone().strip_quals().clone());
        let thrown = match (&outcome, &stripped) {
            (Some(out_ty), Some(msg)) if msg.is_wrapper_union() => {
                let arms: Vec<Ty> = msg.value_arms().into_iter().cloned().collect();
                let mut cases: Vec<String> = Vec::new();
                for (i, arm) in arms.iter().enumerate() {
                    let tag = format!("{arm}");
                    let rendered = self.emit_ty(arm);
                    let inner_wrap =
                        self.wrap_union_value(msg, i, format!("__signal.payload as {rendered}"));
                    let outer = self.wrap_union_value(out_ty, 1, inner_wrap);
                    cases.push(format!("{inner_pad}    \"{tag}\" -> {outer}"));
                }
                cases.push(format!("{inner_pad}    else -> throw __signal"));
                format!(
                    "when (__signal.tag) {{\n{}\n{inner_pad}}}",
                    cases.join("\n")
                )
            }
            (Some(out_ty), Some(msg)) => {
                let rendered = self.emit_ty(msg);
                self.wrap_union_value(out_ty, 1, format!("__signal.payload as {rendered}"))
            }
            _ => "__signal.payload".to_string(),
        };
        format!(
            "try {{\n{inner}{pad}}} catch (__signal: ThrowSignal) {{\n\
             {inner_pad}{thrown}\n{pad}}}"
        )
    }

    /// The body of a `try`: ordinary statements with the tail wrapped into
    /// the outcome's `Ok` arm. A body that always leaves still needs a
    /// value, which the checker made `Ok None` [try].
    fn emit_try_body(&mut self, body: &Block, outcome: Option<&Ty>) -> String {
        let mut out = String::new();
        // Captured before emitting: statement emission moves `expr_indent`.
        let indent = self.expr_indent + 1;
        let pad = "    ".repeat(indent);
        // [kt-effect-fusion] Save the whole environment, not just its
        // depth: a `use` inside the scope *rebases* enclosing entries onto
        // its fused value, and that mutation must roll back with the scope.
        let saved_effect_env = self.effect_env.clone();
        let stmts = &body.stmts;
        let n = stmts.len();
        let mut tail: Option<String> = None;
        {
            {
                for (i, stmt) in stmts.iter().enumerate() {
                    if i + 1 == n {
                        if let Stmt::Expr(e) = stmt {
                            if !matches!(self.ty_of(e.span()), Some(Ty::Never)) {
                                tail = Some(self.emit_expr(e));
                                continue;
                            }
                        }
                    }
                    out.push_str(&self.emit_stmt(stmt, indent));
                }
            }
        }
        let value = tail.unwrap_or_else(|| "Unit".to_string());
        let wrapped = match outcome {
            Some(ty) => self.wrap_union_value(ty, 0, value),
            None => value,
        };
        out.push_str(&format!("{pad}{wrapped}\n"));
        self.effect_env = saved_effect_env;
        out
    }

    /// Wraps a value into arm `arm` of a wrapper union
    /// [union-arm-identity]: `U2_1<Int, String>(value)`.
    fn wrap_union_value(&mut self, target: &Ty, arm: usize, code: String) -> String {
        let value_arms = target.value_arms();
        let n = value_arms.len();
        if n < 2 {
            return code;
        }
        self.union_sizes.insert(n);
        let args: Vec<String> = value_arms
            .iter()
            .map(|a| {
                let a = (*a).clone();
                self.emit_ty(&a)
            })
            .collect();
        format!("U{n}_{}<{}>({code})", arm + 1, args.join(", "))
    }

    /// The statements of a Kotlin lambda block body: the trailing
    /// `return X` becomes the lambda's value.
    fn emit_lambda_stmts(&mut self, stmts: &[Stmt]) -> String {
        let mut out = String::new();
        let n = stmts.len();
        for (i, stmt) in stmts.iter().enumerate() {
            if i + 1 == n {
                if let Stmt::Expr(Expr::Return { value: Some(v), .. }) = stmt {
                    let code = self.emit_expr(v);
                    out.push_str(&format!("    {code}\n"));
                    continue;
                }
            }
            if matches!(stmt, Stmt::Expr(Expr::Return { .. })) {
                self.error(
                    "early `return` inside a lambda is not supported yet \
                     (only as the final statement)",
                );
                continue;
            }
            out.push_str(&self.emit_stmt(stmt, 1));
        }
        out
    }

    /// A fresh `__loopN` local name for loop lowering.
    fn fresh_loop_var(&mut self) -> String {
        self.loop_id += 1;
        format!("__loop{}", self.loop_id)
    }

    /// Reserves a name that does not collide with the current fn's
    /// parameters or locals [kt-effect-params]: `base`, then `base2`, ...
    fn unique_name(&mut self, base: String) -> String {
        let mut name = base.clone();
        let mut i = 1;
        while !self.taken_names.insert(name.clone()) {
            i += 1;
            name = format!("{base}{i}");
        }
        name
    }

    /// The Kotlin `for (<var> in ...)` binding for a Salvo loop pattern.
    ///
    /// [let-destructure] A **destructuring** pattern binds the element to a
    /// temporary here and leaves its own bindings to
    /// [`Self::take_loop_destructure`], which the body opens with. Kotlin could
    /// destructure a `Pair` in the header — `for ((k, v) in pairs)` — but not
    /// the *pass-driven* header, whose element arrives as a cast payload rather
    /// than as a loop variable, and not a struct at all (a Salvo struct is a
    /// plain class, with no `componentN`). One shape therefore serves every
    /// loop and both pattern kinds, and it is the Rust backend's shape too.
    fn for_pattern_var(&mut self, pattern: &Pattern, body_indent: usize) -> String {
        match pattern {
            Pattern::Ident(id) => kt_ident(&id.name),
            Pattern::Tuple { .. } | Pattern::Struct { .. } => {
                let temp = self.unique_name("__elem".to_string());
                let prologue = self.loop_destructure(pattern, &temp, body_indent);
                self.loop_destructures.push(prologue);
                temp
            }
        }
    }

    /// [let-destructure] The statements a destructuring loop body opens with:
    /// one `val` per name, read off the element temporary — a tuple's by
    /// component name [kt-tuple-component], a struct's by field.
    fn loop_destructure(&mut self, pattern: &Pattern, temp: &str, indent: usize) -> String {
        let pad = "    ".repeat(indent);
        let mut out = String::new();
        match pattern {
            Pattern::Ident(_) => {}
            Pattern::Tuple { elems, .. } => {
                for (i, p) in elems.iter().enumerate() {
                    match p {
                        Pattern::Ident(id) => {
                            // A name the body assigns to is a `var`, exactly as
                            // a `let` binding is.
                            let kw = if self.mutated.contains(&id.name) {
                                "var"
                            } else {
                                "val"
                            };
                            out.push_str(&format!(
                                "{pad}{kw} {} = {temp}.{}\n",
                                kt_ident(&id.name),
                                Self::tuple_component(i)
                            ));
                        }
                        _ => self.error("nested destructuring patterns are not supported yet"),
                    }
                }
            }
            Pattern::Struct { fields, .. } => {
                for f in fields {
                    let kw = if self.mutated.contains(&f.binding.name) {
                        "var"
                    } else {
                        "val"
                    };
                    out.push_str(&format!(
                        "{pad}{kw} {} = {temp}.{}\n",
                        kt_ident(&f.binding.name),
                        kt_ident(&f.field.name)
                    ));
                }
            }
        }
        out
    }

    /// [let-destructure] The pending destructuring prologue for the loop whose
    /// header was just emitted, if its pattern needs one.
    fn take_loop_destructure(&mut self) -> String {
        self.loop_destructures.pop().unwrap_or_default()
    }

    /// Lowers a value-position loop [while-value] [kt-loop-value] to a
    /// `run {}` block: a result local is assigned by the body's tail
    /// expression, by `break value`s, and by the `else` tail (which runs
    /// only when the loop never did); the loop itself stays a plain
    /// Kotlin loop. The local is a nullable temp, unwrapped with `!!` at
    /// the end when the checked join type has no `None` arm.
    fn emit_loop_value(&mut self, expr: &Expr) -> String {
        let join = self.ty_of(expr.span()).cloned();
        let (kt, needs_unwrap) = match &join {
            Some(t)
                if ty_is_concrete(t)
                    && !t.is_none_ty()
                    && !matches!(t, Ty::Never) =>
            {
                if t.has_none_arm() {
                    // Renders with the trailing `?` already.
                    (self.emit_ty(t), false)
                } else {
                    (format!("{}?", self.emit_ty(t)), true)
                }
            }
            // `None`-typed or unchecked loops flow through untyped.
            _ => ("Any?".to_string(), false),
        };
        let result = self.fresh_loop_var();
        let ran = format!("{result}_ran");
        let mut out = String::new();
        out.push_str("run {\n");
        out.push_str(&format!("var {result}: {kt} = null\n"));
        match expr {
            Expr::While {
                cond,
                body,
                else_block,
                ..
            } => {
                if else_block.is_some() {
                    out.push_str(&format!("var {ran} = false\n"));
                }
                // [is-bind-once] As in statement position.
                match self.hoistable_is(cond) {
                    Some(subject) => {
                        out.push_str("while (true) {\n");
                        let temp = self.emit_is_temp(subject, 0);
                        out.push_str(&temp);
                        let c = self.emit_expr(cond);
                        out.push_str(&format!("if (!({c})) break\n"));
                    }
                    None => {
                        let c = self.emit_expr(cond);
                        out.push_str(&format!("while ({c}) {{\n"));
                    }
                }
                if else_block.is_some() {
                    out.push_str(&format!("{ran} = true\n"));
                }
                // `while x is T name` re-binds per iteration.
                out.push_str(&self.emit_is_bindings(cond, 0));
                out.push_str(&self.emit_widen_shadows(cond, 0));
            out.push_str(&self.emit_widen_shadows(cond, 0));
                self.loop_results.push(Some(result.clone()));
                out.push_str(&self.emit_loop_body_value(body, &result));
                self.loop_results.pop();
                out.push_str("}\n");
                if let Some(b) = else_block {
                    out.push_str(&format!("if (!{ran}) {{\n"));
                    out.push_str(&self.emit_loop_body_value(b, &result));
                    out.push_str("}\n");
                }
            }
            Expr::For {
                pattern,
                iterable,
                body,
                else_block,
                ..
            } => {
                // [fn-effects] A claiming producer in *value* position would
                // need its `close` in a `finally` around a block that is also
                // producing a value; refused rather than driven without one
                // [backend-never-wrong].
                if self
                    .ty_of(iterable.span())
                    .is_some_and(|t| !t.effect_claims().is_empty())
                {
                    self.error(
                        "a `for` over a producer that performs effects is not supported in \
                         value position yet: write the loop as a statement",
                    );
                }
                // [iter-fn] [backend-never-wrong] Same cut for an
                // origin: its machine has to be closed after the loop, which a
                // value-position loop has nowhere to put.
                if self.pass_driver_of(iterable).is_some_and(|d| d.origin) {
                    self.error(
                        "a `for` over an origin in value position is not supported yet: \
                         its state machine has to be closed after the loop, so write \
                         the loop as a statement",
                    );
                }
                // [iter-protocol] A pass is driven; see the statement arm.
                let pass = self.pass_driver_of(iterable).map(|driver| {
                    self.emit_pass_loop_header(driver, pattern, iterable, 0)
                });
                if else_block.is_some() {
                    out.push_str(&format!("var {ran} = false\n"));
                }
                match &pass {
                    Some(header) => out.push_str(header),
                    None => {
                        let var = self.for_pattern_var(pattern, 0);
                        let code = self.emit_expr(iterable);
                        let iter = self.native_for_subject(iterable, code);
                        out.push_str(&format!("for ({var} in {iter}) {{\n"));
                    }
                }
                if else_block.is_some() {
                    out.push_str(&format!("{ran} = true\n"));
                }
                // [let-destructure] As in statement position.
                out.push_str(&self.take_loop_destructure());
                self.loop_results.push(Some(result.clone()));
                out.push_str(&self.emit_loop_body_value(body, &result));
                self.loop_results.pop();
                out.push_str("}\n");
                if let Some(b) = else_block {
                    out.push_str(&format!("if (!{ran}) {{\n"));
                    out.push_str(&self.emit_loop_body_value(b, &result));
                    out.push_str("}\n");
                }
            }
            _ => unreachable!("emit_loop_value only receives loops"),
        }
        if needs_unwrap {
            out.push_str(&format!("{result}!!\n}}"));
        } else {
            out.push_str(&format!("{result}\n}}"));
        }
        out
    }

    /// A loop body (or loop `else` block) whose tail expression assigns
    /// the loop's result local [while-value].
    fn emit_loop_body_value(&mut self, block: &Block, result: &str) -> String {
        // [kt-effect-fusion] Save the whole environment, not just its
        // depth: a `use` inside the scope *rebases* enclosing entries onto
        // its fused value, and that mutation must roll back with the scope.
        let saved_effect_env = self.effect_env.clone();
        let out = self.emit_loop_body_stmts(&block.stmts, result);
        self.effect_env = saved_effect_env;
        out
    }

    /// The statements of a loop body in value position. The tail assigns the
    /// result local, so the value flows out regardless.
    fn emit_loop_body_stmts(&mut self, stmts: &[Stmt], result: &str) -> String {
        let mut out = String::new();
        let n = stmts.len();
        for (i, stmt) in stmts.iter().enumerate() {
            if i + 1 == n {
                if let Stmt::Expr(e) = stmt {
                    out.push_str(&self.emit_tail_assign(e, result));
                    continue;
                }
            }
            out.push_str(&self.emit_stmt(stmt, 0));
        }
        out
    }

    /// Assigns a block-tail expression to a loop result local.
    /// `Never`-typed tails never fall through (statement as-is);
    /// `None`-typed tails have no Kotlin payload (statement, then `null`).
    fn emit_tail_assign(&mut self, e: &Expr, result: &str) -> String {
        match self.ty_of(e.span()) {
            Some(Ty::Never) => self.emit_expr_stmt(e, 0),
            Some(t) if t.is_none_ty() => {
                let stmt = self.emit_expr_stmt(e, 0);
                format!("{stmt}{result} = null\n")
            }
            _ => {
                let code = self.emit_expr(e);
                format!("{result} = {code}\n")
            }
        }
    }

    /// [implicit-param] A member's parameter list *including* its implicit
    /// parameters: an effect member's interface method, a handler's override
    /// and a generated host skeleton all have to agree, so they all render
    /// through here.
    fn emit_member_param_list_with_implicits(&mut self, f: &FnDecl) -> String {
        let mut params = self.emit_param_list(&f.params);
        let implicits = self.implicits_of(f);
        for imp in &implicits {
            let ty = self.kotlin_ty(&imp.ty);
            if !params.is_empty() {
                params.push_str(", ");
            }
            params.push_str(&format!("{}: {ty}", kt_ident(&imp.name)));
        }
        params
    }

    /// [implicit-param] The implicit parameters of a fn or member: a
    /// top-level fn is keyed by its `FnKey`, a member (an effect member's
    /// signature, or a handler's implementation of one) by its own name span.
    /// Both render the same way — trailing parameters of function type.
    fn implicits_of(&self, f: &FnDecl) -> Vec<salvo_core::ImplicitParam> {
        self.checked
            .fn_refs
            .get(&(self.file_idx, f.name.span))
            .and_then(|key| self.checked.implicit_params.get(key).cloned())
            .or_else(|| {
                self.checked
                    .implicit_members
                    .get(&(self.file_idx, f.name.span))
                    .cloned()
            })
            .unwrap_or_default()
    }

    /// A named fn used as a value [fn-contract] [fn-effects]: a Kotlin
    /// function reference when the effect lists line up, an adapter lambda
    /// when they do not.
    fn named_fn_value(&mut self, name: &str, span: Span) -> String {
        // [fn-value-select] [fn-rename] The *declaration* decides the emitted
        // name: an overloaded name is mangled [kt-fn-mangling], and a renamed
        // one does not exist in the output at all.
        let target = self
            .checked
            .fn_refs
            .get(&(self.file_idx, span))
            .copied()
            .and_then(|k| self.fn_by_key(k))
            .map(|decl| self.kotlin_fn_name(decl))
            .unwrap_or_else(|| kt_ident(name));
        let reference = format!("::{target}");
        let taken: Vec<Ty> = self
            .checked
            .lambda_effects
            .get(&(self.file_idx, span))
            .cloned()
            .unwrap_or_default();
        let key = self.checked.fn_refs.get(&(self.file_idx, span)).copied();
        let declared: Vec<Ty> = key
            .and_then(|k| self.checked.fn_effects.get(&k).cloned())
            .unwrap_or_default();
        // [kt-effect-fusion] A fused declaration takes one `__Fx` value, so
        // a bare reference can never satisfy the per-effect fn-value ABI:
        // an effectful fn is always wrapped, constructing the fused carrier
        // inline from the adapter's effect parameters.
        if self.fusion {
            let effectful = declared.iter().any(|t| !is_throw_effect_ty(t));
            if effectful {
                let Some(decl) = key.and_then(|k| self.fn_by_key(k)) else {
                    return reference;
                };
                let arity = decl.params.len();
                let mut params: Vec<String> = Vec::new();
                let mut fx_args: Vec<String> = Vec::new();
                let mut rendered_list: Vec<String> = Vec::new();
                for (i, ty) in taken.iter().enumerate() {
                    let rendered = self.kotlin_ty(ty);
                    let var = format!("__fx{i}");
                    params.push(format!("{var}: {rendered}"));
                    fx_args.push(var);
                    rendered_list.push(rendered);
                }
                let (class, _props, order) = self.emit_fx_class(&rendered_list);
                let fx_args: Vec<String> = order.iter().map(|i| fx_args[*i].clone()).collect();
                let mut args: Vec<String> = vec![format!("{class}({})", fx_args.join(", "))];
                let value_params: Vec<String> = (0..arity).map(|i| format!("__a{i}")).collect();
                params.extend(value_params.clone());
                args.extend(value_params);
                return format!(
                    "{{ {} -> {}({}) }}",
                    params.join(", "),
                    self.kotlin_fn_name(decl),
                    args.join(", ")
                );
            }
        }
        if taken == declared {
            return reference;
        }
        let Some(decl) = key.and_then(|k| self.fn_by_key(k)) else {
            return reference;
        };
        let arity = decl.params.len();
        let mut params: Vec<String> = Vec::new();
        let mut effect_args: Vec<(String, String)> = Vec::new();
        for (i, ty) in taken.iter().enumerate() {
            let rendered = self.kotlin_ty(ty);
            let var = format!("__fx{i}");
            params.push(format!("{var}: {rendered}"));
            effect_args.push((rendered, var));
        }
        let mut args: Vec<String> = Vec::new();
        for ty in &declared {
            let rendered = self.kotlin_ty(ty);
            match effect_args.iter().find(|(key, _)| *key == rendered) {
                Some((_, var)) => args.push(var.clone()),
                None => {
                    // Unreachable under the checker's fits rule
                    // [fn-effects] [backend-never-wrong].
                    self.error(format!(
                        "internal error: `{name}` needs effect `{rendered}`, \
                         which the function type it is passed as does not \
                         declare"
                    ));
                }
            }
        }
        let value_params: Vec<String> = (0..arity).map(|i| format!("__a{i}")).collect();
        params.extend(value_params.clone());
        args.extend(value_params);
        format!(
            "{{ {} -> {}({}) }}",
            params.join(", "),
            self.kotlin_fn_name(decl),
            args.join(", ")
        )
    }

    fn emit_lambda(
        &mut self,
        params: &[LambdaParam],
        body: &LambdaBody,
        span: salvo_syntax::Span,
    ) -> String {
        // [fn-effects] The effects a call of this value performs arrive as
        // *leading parameters* rather than captures, so the value carries no
        // handler and can be stored or passed freely.
        let effects: Vec<Ty> = self
            .checked
            .lambda_effects
            .get(&(self.file_idx, span))
            .cloned()
            .unwrap_or_default();
        // [kt-effect-fusion] Save the whole environment, not just its
        // depth: a `use` inside the scope *rebases* enclosing entries onto
        // its fused value, and that mutation must roll back with the scope.
        let saved_effect_env = self.effect_env.clone();
        let mut param_list: Vec<String> = Vec::new();
        // [kt-effect-fusion] The fn-value ABI stays per-effect (Kotlin's
        // nominal typing has no blanket impls, so no provider interface
        // could cover subsets); under the fusion the body opens by
        // combining the parameters into one fused carrier for calls to
        // fused callees.
        let mut fx_prelude = String::new();
        let mut entries: Vec<EffectEntry> = Vec::new();
        for ty in &effects {
            let rendered = self.kotlin_ty(ty);
            let var = self.unique_name(effect_param_name(&rendered));
            entries.push(EffectEntry {
                ty: Some(ty.clone()),
                rendered: rendered.clone(),
                expr: var.clone(),
                fused: None,
            });
            param_list.push(format!("{var}: {rendered}"));
        }
        if self.fusion && !entries.is_empty() {
            let rendered: Vec<String> = entries.iter().map(|e| e.rendered.clone()).collect();
            let (class, _props, order) = self.emit_fx_class(&rendered);
            let var = self.unique_name("__fx".to_string());
            let args: Vec<String> = order
                .iter()
                .map(|i| entries[*i].expr.clone())
                .collect();
            fx_prelude = format!("val {var} = {class}({})\n", args.join(", "));
            for entry in entries.iter_mut() {
                entry.fused = Some(var.clone());
            }
        }
        self.effect_env.extend(entries);
        param_list.extend(params.iter().map(|p| match &p.ty {
            Some(t) => {
                let ty = self.emit_type(t);
                format!("{}: {ty}", kt_ident(&p.name.name))
            }
            None => kt_ident(&p.name.name),
        }));
        let out = match body {
            LambdaBody::Expr(expr) => {
                let expr_code = self.emit_expr(expr);
                if fx_prelude.is_empty() {
                    format!("{{ {} -> {expr_code} }}", param_list.join(", "))
                } else {
                    format!(
                        "{{ {} ->\n    {fx_prelude}    {expr_code}\n}}",
                        param_list.join(", ")
                    )
                }
            }
            LambdaBody::Block(block) => {
                // Kotlin lambdas return their last expression; a trailing
                // `return X` becomes the value. The lambda body is a
                // `return` barrier: bare returns never target an
                // enclosing `iterator {}` builder.
                let saved_ctx = self.stmt_ctx;
                self.stmt_ctx = StmtCtx::Normal;
                let mut out = format!("{{ {} ->\n", param_list.join(", "));
                if !fx_prelude.is_empty() {
                    out.push_str(&format!("    {fx_prelude}"));
                }
                out.push_str(&self.emit_lambda_stmts(&block.stmts));
                out.push('}');
                self.stmt_ctx = saved_ctx;
                out
            }
        };
        self.effect_env = saved_effect_env;
        out
    }

    fn emit_struct_lit(
        &mut self,
        ty: Option<&Type>,
        fields: &[StructLitField],
        _span: salvo_syntax::Span,
    ) -> String {
        let type_name = match ty {
            Some(Type::Named { base, .. }) => Some(base.name.name.clone()),
            Some(other) => Some(format!("{:?}", other.span())),
            None => None,
        };

        let spreads: Vec<&Expr> = fields
            .iter()
            .filter_map(|f| match &f.kind {
                StructLitFieldKind::Spread(e) => Some(e),
                _ => None,
            })
            .collect();
        let named: Vec<(String, String)> = fields
            .iter()
            .filter_map(|f| match &f.kind {
                StructLitFieldKind::Named { name, value } => {
                    Some((kt_ident(&name.name), self.emit_expr(value)))
                }
                _ => None,
            })
            .collect();
        let named_args: Vec<String> = named
            .iter()
            .map(|(n, v)| format!("{n} = {v}"))
            .collect();

        match spreads.len() {
            0 => {
                let Some(name) = type_name else {
                    self.error(
                        "struct literals without a type annotation are not supported here",
                    );
                    return "TODO()".to_string();
                };
                format!("{name}({})", named_args.join(", "))
            }
            1 => {
                // `Person {...base, field: v}` -> `base.copy(field = v)`
                let base = self.emit_expr(spreads[0]);
                format!("{base}.copy({})", named_args.join(", "))
            }
            _ => {
                self.error("struct literals with multiple spreads are not supported yet");
                "TODO()".to_string()
            }
        }
    }

    // ================= calls =================

    fn emit_call(
        &mut self,
        callee: &Expr,
        type_args: &[Type],
        args: &[Expr],
        named: &[NamedArg],
        span: Span,
    ) -> String {
        // [actor-use-addr] [kt-actor] A send to an actor: build the
        // protocol's message and enqueue it. The receiver names where it goes,
        // not an argument.
        if let Some(effect) = self.checked.addr_calls.get(&(self.file_idx, span)).cloned() {
            return self.emit_addr_send(&effect, callee, args, span);
        }
        // [mixed-handler] [kt-mixed] A façade send: a sync member of a mixed
        // handler calling one of the handler's own `send fn` members — an
        // enqueue on the servant's mailbox through the façade's own addr.
        if let Some(member) = self
            .checked
            .facade_sends
            .get(&(self.file_idx, span))
            .cloned()
        {
            return self.emit_facade_send(&member, args);
        }
        // [actor-self-send] `k@self(args)`: a message to the actor the
        // enclosing member belongs to.
        if let Some(member) = self
            .checked
            .self_sends
            .get(&(self.file_idx, span))
            .cloned()
        {
            return self.emit_self_send(&member, args);
        }
        // [throw] [kt-throw-signal] `throw(message)` is the control
        // transfer itself: a throw the innermost `try` catches. A call that
        // merely *propagates* a throw needs nothing — the JVM unwinds.
        if let Some(site) = self.checked.may_throw.get(&(self.file_idx, span)).cloned() {
            if site.performs {
                self.needs_throw = true;
                let message = match args.first() {
                    Some(a) => self.emit_expr(a),
                    None => {
                        self.error("`throw` needs a message argument");
                        "null".to_string()
                    }
                };
                // The signal carries the message untouched plus the Salvo
                // type it was written at: the delimiter picks the arm
                // [kt-throw-signal], since a throwing frame cannot know
                // which `try` will catch it.
                let tag = format!("{}", site.message);
                return format!("throw ThrowSignal({message}, \"{tag}\")");
            }
        }
        // Normalize dot-notation [fn-dot]: `base.f(args)` == `f(base, args)`
        // when `f` resolves to a known function or effect member.
        if let Expr::Field { base, field, .. } = callee {
            let name = field.name.as_str();
            let total = args.len() + 1;
            if self.symbols.effect_of_fn.contains_key(name)
                || self.symbols.resolve_fn(name, total).is_some()
            {
                let mut all_args: Vec<&Expr> = Vec::with_capacity(total);
                all_args.push(base);
                all_args.extend(args.iter());
                return self.emit_resolved_call(name, type_args, &all_args, named, span);
            }
            // [call-resolve] The checker rejects an undeclared dot-call, so
            // reaching here means a resolution table lost an entry without
            // reporting it — a compiler bug. Say so instead of inventing a
            // Kotlin method call: emitted code must never be a guess
            // [backend-never-wrong].
            self.error(format!(
                "internal error: dot-call `{name}` reached the Kotlin emitter \
                 unresolved (the checker should have rejected it, or resolved \
                 it to a fn or effect member)"
            ));
            return "TODO()".to_string();
        }

        if let Expr::Ident(id) = callee {
            let arg_refs: Vec<&Expr> = args.iter().collect();
            return self.emit_resolved_call(&id.name, type_args, &arg_refs, named, span);
        }

        // [fn-overload-at] `f@core.list(x)` / `xs.f@core.list(y)`: the scope
        // selector is a *checker* mechanism — it only narrowed which
        // declaration the call resolves to, which `call_fn` already records
        // — so emission is the ordinary call, with the receiver folded in for
        // the dot form [fn-dot].
        if let Expr::Scoped { base, name, .. } = callee {
            let mut all_args: Vec<&Expr> = Vec::with_capacity(args.len() + 1);
            if let Some(base) = base {
                all_args.push(base);
            }
            all_args.extend(args.iter());
            return self.emit_resolved_call(&name.name, type_args, &all_args, named, span);
        }

        // [effect-at] `close@Fs(s)` / `s.close@Fs()`: likewise a checker
        // mechanism — it only narrowed which *effect* the member call
        // resolves through, which `effect_calls` already records at this
        // span — so emission is the ordinary member call.
        if let Expr::EffectScoped { base, name, .. } = callee {
            let mut all_args: Vec<&Expr> = Vec::with_capacity(args.len() + 1);
            if let Some(base) = base {
                all_args.push(base);
            }
            all_args.extend(args.iter());
            return self.emit_resolved_call(&name.name, type_args, &all_args, named, span);
        }

        // Calling a computed value (lambda etc.) — [fn-effects] threads its
        // effects first, like any fn value.
        let callee_code = self.emit_expr(callee);
        let mut arg_code: Vec<String> = self.fn_value_effect_args(span);
        arg_code.extend(args.iter().map(|a| self.emit_expr(a)));
        format!("{callee_code}({})", arg_code.join(", "))
    }

    fn emit_resolved_call(
        &mut self,
        name: &str,
        type_args: &[Type],
        args: &[&Expr],
        named: &[NamedArg],
        span: Span,
    ) -> String {
        // 1. Effect member call: dispatch through the handler in scope
        // [kt-effect-params]. The checker records which effect instance
        // the call resolved to ([effect-disambiguation], `effect_calls`);
        // string matching on the effect name remains the fallback for
        // unchecked contexts.
        // ...unless the checker resolved this callee to a fn-typed **local**
        // [call-resolve], which outranks any same-named declaration:
        // `effect_of_fn` is program-wide and cannot see scopes, so without
        // this a user effect member could hijack a std function's own
        // parameter (`filter`'s `keep`).
        if let Some(owners) = self
            .symbols
            .effect_of_fn
            .get(name)
            .filter(|_| {
                !self.checked.local_calls.contains(&(self.file_idx, span))
                    // [effect-available] ...or to an ordinary fn, because no
                    // instance of the owning effect was in scope: the name is
                    // a member somewhere, this call is not.
                    && !self
                        .checked
                        .fn_over_member_calls
                        .contains(&(self.file_idx, span))
            })
        {
            let owners = owners.clone();
            // [effect-member-overload] Several effects may declare the
            // member: the checker's per-call resolution names the owner;
            // with a sole owner the map answers directly (the unchecked
            // fallback path).
            let effect: &str = match self.checked.effect_calls.get(&(self.file_idx, span)) {
                Some(Ty::Named { name: n, .. }) => owners
                    .iter()
                    .copied()
                    .find(|o| *o == n.as_str())
                    .unwrap_or(owners[0]),
                _ if owners.len() == 1 => owners[0],
                _ => {
                    self.error(format!(
                        "internal: `{name}` is a member of several effects and \
                         the checker recorded no resolution for this call"
                    ));
                    owners[0]
                }
            };
            let handler = match self.checked.effect_calls.get(&(self.file_idx, span)) {
                Some(ty) if ty_is_concrete(ty) => {
                    let ty = ty.clone();
                    self.lookup_effect_handler_by_ty(&ty)
                }
                _ => self.lookup_effect_handler(effect, type_args),
            };
            let mut arg_code: Vec<String> = args.iter().map(|a| self.emit_expr(a)).collect();
            // [implicit-param] A member's implicit parameters are part of its
            // signature, so they arrive as trailing arguments here exactly as
            // for a plain fn call [implicit-resolve].
            arg_code.extend(self.emit_implicit_args(named, span));
            let called = self.called_member_name(effect, name, span);
            return format!("{handler}.{called}({})", arg_code.join(", "));
        }

        // [implicit-param] An implicit parameter shadows the fns of the same
        // name inside the body: it *is* one of them, chosen by the caller, so
        // the call goes through the parameter rather than resolving again.
        if self.implicits.iter().any(|i| i.name == name) || self.ctor_implicits.contains(name) {
            let arg_code: Vec<String> = args.iter().map(|a| self.emit_expr(a)).collect();
            return format!("{}({})", kt_ident(name), arg_code.join(", "));
        }

        // 2. Checker-resolved fn target (type-based overloads win).
        // An `intrinsic fn` lowers in the emitter [intrinsic-fn]; anything
        // else has a body, since a bodiless top-level fn is a parse error
        // [decl-body].
        let checker_resolved = self
            .checked
            .call_fn
            .get(&(self.file_idx, span))
            .and_then(|key| self.fn_by_key(*key));
        if let Some(f) = checker_resolved {
            if f.intrinsic {
                return self.emit_intrinsic_call(f, args, span);
            }
            return self.emit_fn_call(name, f, type_args, args, named, span);
        }

        // 3. Known function (unchecked contexts): arity narrowed by the
        // checked argument types; ambiguous dispatch is a codegen error,
        // never a guess [backend-never-wrong] [fn-overload].
        let fn_cands = self.symbols.fns_matching_arity(name, args.len());
        if !fn_cands.is_empty() {
            let Some(f) = disambiguate_unchecked(self, &fn_cands, |f| f.params.as_slice(), args)
            else {
                self.error(format!(
                    "call to `{name}` is ambiguous here: multiple same-arity \
                     overloads match and the checker did not resolve the \
                     overload; annotate the argument types"
                ));
                return "TODO()".to_string();
            };
            if f.intrinsic {
                return self.emit_intrinsic_call(f, args, span);
            }
            return self.emit_fn_call(name, f, type_args, args, named, span);
        }

        // 4. Handler constructor / struct / local callable: pass through.
        // [fn-effects] A *fn value* takes its effects as leading arguments;
        // the checker recorded which instances to thread at this call.
        let mut arg_code: Vec<String> = self.fn_value_effect_args(span);
        arg_code.extend(args.iter().map(|a| self.emit_expr(a)));
        let generics = self.emit_type_args(type_args);
        format!("{}{generics}({})", kt_ident(name), arg_code.join(", "))
    }

    /// [fn-effects] The effect arguments a fn-value call threads, from the
    /// instances the checker resolved for it.
    fn fn_value_effect_args(&mut self, span: Span) -> Vec<String> {
        let effects: Vec<Ty> = self
            .checked
            .call_effects
            .get(&(self.file_idx, span))
            .cloned()
            .unwrap_or_default();
        effects
            .iter()
            .map(|ty| self.lookup_effect_handler_by_ty(ty))
            .collect()
    }

    /// A call to an `intrinsic fn`, lowered directly by the compiler
    /// [intrinsic-fn]. Two kinds live here:
    ///
    /// - `copy` and `discard` dispatch on the argument's *type*, which is
    ///   the whole reason they are intrinsics — `copy` is identity for
    ///   transitively immutable types (duplicating a reference to
    ///   immutable data is a copy), a real copy where mutation is
    ///   possible, and a codegen error where no correct copy exists yet
    ///   [kt-copy] [linear-discard].
    /// - everything else std declares is looked up in
    ///   [`crate::intrinsics::fn_call`], keyed by the declaration the
    ///   checker resolved.
    ///
    /// An intrinsic with no lowering is a codegen error naming it, never a
    /// pass-through [backend-never-wrong].
    fn emit_intrinsic_call(&mut self, f: &FnDecl, args: &[&Expr], span: Span) -> String {
        // [linear-discard] `discard(x)` evaluates the value and drops
        // it: `.let {}` yields `Unit` (Salvo `None`).
        if f.name.name == "discard" && args.len() == 1 {
            let code = self.emit_expr(args[0]);
            return format!("({code}).let {{}}");
        }
        if f.name.name != "copy" || args.len() != 1 {
            let recv = f.params.first().and_then(|p| type_base_name(&p.ty));
            let arg_code = self.intrinsic_arg_code(f, args);
            let type_args = self.intrinsic_type_args(f, span);
            // [kt-actor] The scheduler's own intrinsics: answering a reply
            // token, building a pool (plain or dedicated), registering a
            // death watch, and registering a quiescence hook
            // [actor-on-idle].
            if recv == Some("Reply")
                || matches!(
                    f.name.name.as_str(),
                    "pool" | "thread" | "watch" | "on_idle"
                )
            {
                self.needs_scheduler = true;
            }
            // [col-sorted] [kt-ordered] The sorted constructors build their
            // tree with Salvo's comparator. The `Sorted List` surface no
            // longer needs it: its primitives are handed the ordering the
            // claim names [col-sorted-list], and where that is the canonical
            // `cmp(Str, Str)` the case below asks for the file.
            if matches!(
                f.name.name.as_str(),
                "sorted_set_of" | "mut_sorted_set_of" | "sorted_map_of" | "mut_sorted_map_of"
            ) {
                self.needs_compare = true;
            }
            // [cmp-groups] The canonical `cmp(Str, Str)` goes through the same
            // comparator, for the same reason: `String.compareTo` is UTF-16
            // code-unit order where Salvo's `Str` order is code point.
            if f.name.name == "cmp" && recv == Some("Str") {
                self.needs_compare = true;
            }
            // [time-types] [kt-time] The two clock readings live in their own
            // runtime object, so a program that reads a clock gets it and one
            // that never asks the time carries nothing.
            if matches!(
                f.name.name.as_str(),
                "monotonic_nanos" | "epoch_nanos" | "fire_after"
            ) {
                self.needs_time = true;
            }
            // [time-timer] A deadline is the scheduler's, and its reading comes
            // from the time runtime — so registering one needs both files.
            if f.name.name == "fire_after" {
                self.needs_scheduler = true;
            }
            // [cmp-carry] A keyed container kept by a *named* ordering needs a
            // runtime container with a slot for one, which this backend does not
            // have yet (`TreeSet` takes a comparator, but `LinkedHashSet` keys off
            // `hashCode`/`equals`, so the pair has to be built together). Refused
            // rather than emitted as a container that ignores the ordering it was
            // told to keep [backend-never-wrong].
            let ordering = self.container_ordering(span);
            let keyed = self.keyed_pair(span);
            if let Some(code) = crate::intrinsics::fn_call(
                &f.name.name,
                recv,
                &arg_code,
                &type_args,
                ordering.as_deref(),
                keyed.as_ref().map(|(h, e)| (h.as_str(), e.as_str())),
            )
            {
                return code;
            }
            self.error(format!(
                "intrinsic fn `{}` is not supported by the kotlin backend",
                f.name.name
            ));
            return "TODO()".to_string();
        }
        let arg = args[0];
        let ty = self
            .checked
            .expr_ty
            .get(&(self.file_idx, arg.span()))
            .cloned()
            .unwrap_or(Ty::Unknown);
        let code = self.emit_expr(arg);
        match self.copy_code(&ty, &code) {
            Some(copied) => return copied,
            None => {
                self.error(format!(
                    "the kotlin backend cannot `copy` a value of type `{ty}` yet"
                ));
                return "TODO()".to_string();
            }
        }
    }

    /// [kt-copy] The copy of a value of type `ty`, given its code, or `None`
    /// where this backend cannot copy the shape correctly (a shallow copy
    /// would alias mutable parts). Shared by `copy(x)` calls and by `copy`
    /// resolved as an implicit *value* at a concrete type [copy-implicit].
    fn copy_code(&mut self, ty: &Ty, code: &str) -> Option<String> {
        let code = code.to_string();
        // Identity: no Salvo operation can mutate any part of the value.
        if self.ty_immutable(ty, &mut Vec::new()) {
            return Some(code);
        }
        // Real copies for the mutable shapes Kotlin can copy correctly.
        let has_mut = ty.quals().iter().any(|q| q.name == "Mut");
        match ty.strip_quals() {
            // [kt-mut-str] A `Mut Str` is a `StringBuilder`, whose copy is
            // a new builder over the same characters — identity here would
            // alias the buffer, which is the whole point of [kt-copy].
            Ty::Named { name, .. } if has_mut && name == "Str" => {
                return Some(format!("StringBuilder({code})"));
            }
            // [kt-bytes] A buffer copies through its copy constructor, and
            // for `Bytes` as well as `Mut Bytes`: one class serves both, so a
            // plain `Bytes` can be the very object something else holds as a
            // `Mut Bytes` — identity would alias it [kt-copy].
            Ty::Named { name, .. } if name == "Bytes" => {
                return Some(format!("salvo.SalvoBytes({code})"));
            }
            Ty::Named { name, args: targs } if has_mut && name == "List" => {
                if targs.iter().all(|t| self.ty_immutable(t, &mut Vec::new())) {
                    return Some(format!("{code}.toMutableList()"));
                }
            }
            Ty::Named { name, args: targs } if has_mut => {
                // A `Mut` struct whose fields are all immutable copies
                // correctly with the data class's shallow `.copy()`.
                if let Some(s) = self.symbols.structs.get(name.as_str()) {
                    let subst: HashMap<String, Ty> = s
                        .generics
                        .iter()
                        .map(|g| g.name.clone())
                        .zip(targs.iter().cloned())
                        .collect();
                    let mut visiting = vec![name.clone()];
                    let all_immutable = s.fields.iter().all(|field| {
                        match Self::approx_ty(&field.ty, &subst) {
                            Some(t) => self.ty_immutable(&t, &mut visiting),
                            None => false,
                        }
                    });
                    if all_immutable {
                        return Some(format!("{code}.copy()"));
                    }
                }
            }
            Ty::Array(elem) => {
                if self.ty_immutable(elem, &mut Vec::new()) {
                    return Some(format!("{code}.copyOf()"));
                }
            }
            _ => {}
        }
        None
    }

    /// The rendered arguments of an `intrinsic fn` call, in declaration
    /// order. Kotlin renders every argument the ordinary way — there is no
    /// place/owned distinction to preserve, unlike the Rust backend
    /// [rs-borrows] — so the only shaping is that a `...` spread becomes
    /// Kotlin's own spread (`*arr`): a variadic lowering that splices the
    /// array *as one argument* builds a collection of one array
    /// [fn-variadic].
    fn intrinsic_arg_code(&mut self, f: &FnDecl, args: &[&Expr]) -> Vec<String> {
        let _ = f;
        args.iter()
            .map(|arg| match arg {
                Expr::Spread { operand, .. } => format!("*{}", self.emit_expr(operand)),
                other => self.emit_expr(other),
            })
            .collect()
    }

    /// The call's resolved type arguments in the declaration's generic
    /// order ([call-type-args]), rendered as Kotlin. This is what lets the
    /// list constructors spell out their element type, which kotlinc
    /// cannot infer from an empty argument list.
    ///
    /// An unresolved (`Unknown`) argument is only reachable through a
    /// checker gap — [call-type-args] rejects a call whose type arguments
    /// nothing determines — so it is an error rather than a guess
    /// [backend-never-wrong].
    fn intrinsic_type_args(&mut self, f: &FnDecl, span: Span) -> Vec<String> {
        if f.generics.is_empty() {
            return Vec::new();
        }
        let Some(tys) = self
            .checked
            .call_type_args
            .get(&(self.file_idx, span))
            .cloned()
        else {
            return Vec::new();
        };
        let mut out = Vec::with_capacity(tys.len());
        for ty in &tys {
            if ty.is_unknown() {
                self.error(format!(
                    "call to intrinsic fn `{}` has an unresolved type argument, \
                     which its kotlin lowering needs",
                    f.name.name
                ));
                out.push("Any".to_string());
                continue;
            }
            out.push(self.kotlin_ty(ty));
        }
        out
    }

    /// [platform-effect] Whether an effect is host-implemented, from either
    /// the checker's resolved type or (in unchecked contexts) its rendered
    /// name — the same table/fallback pair every effect lookup here uses.
    fn is_platform_effect(&self, ty: Option<&Ty>, rendered: &str) -> bool {
        if let Some(Ty::Named { name, .. }) = ty.map(Ty::strip_quals) {
            if let Some(e) = self.symbols.effects.get(name.as_str()) {
                return e.platform;
            }
        }
        // The rendered Kotlin name of an effect is its Salvo name (effects
        // are not remapped), minus any generic arguments.
        let base = rendered.split('<').next().unwrap_or(rendered);
        self.symbols
            .effects
            .get(base)
            .is_some_and(|e| e.platform)
    }

    /// [platform-effect] Whether this fn's declared effect list mentions a
    /// platform effect — which is what turns `main` into an entry point the
    /// host calls rather than a `main` of its own.
    fn declares_platform_effect(&self, f: &FnDecl) -> bool {
        f.effects.iter().flatten().any(|eff| match eff {
            EffectRef::Effect(r) | EffectRef::LocalEffect(r) => self
                .symbols
                .effects
                .get(r.name.name.as_str())
                .is_some_and(|e| e.platform),
            _ => false,
        })
    }

    /// Whether no Salvo operation can mutate any part of a value of this
    /// type — the condition under which identity is a correct `copy` on
    /// the JVM [kt-copy]. Conservative: anything unknown is mutable.
    /// `visiting` breaks struct cycles (a cycle through immutable
    /// spines stays immutable).
    fn ty_immutable(&self, ty: &Ty, visiting: &mut Vec<String>) -> bool {
        match ty {
            Ty::ValueRef { .. } => true,
            Ty::Qualified { quals, base } => {
                !quals.iter().any(|q| q.name == "Mut")
                    && self.ty_immutable(base, visiting)
            }
            Ty::Named { name, args } => match name.as_str() {
                "Byte" | "Int" | "Long" | "Float" | "Double" | "Char" | "Bool"
                | "Str" | "None" => true,
                // A non-`Mut` list is read-only [type-canbe-mut].
                "List" => args.iter().all(|a| self.ty_immutable(a, visiting)),
                _ => {
                    let Some(s) = self.symbols.structs.get(name.as_str()) else {
                        return false;
                    };
                    if visiting.iter().any(|v| v == name) {
                        return true;
                    }
                    // A struct value without the `Mut` qualifier cannot
                    // have fields assigned [struct-mut]; its fields must
                    // still be transitively immutable themselves.
                    visiting.push(name.clone());
                    let subst: HashMap<String, Ty> = s
                        .generics
                        .iter()
                        .map(|g| g.name.clone())
                        .zip(args.iter().cloned())
                        .collect();
                    let ok = s.fields.iter().all(|field| {
                        match Self::approx_ty(&field.ty, &subst) {
                            Some(t) => self.ty_immutable(&t, visiting),
                            None => false,
                        }
                    });
                    visiting.pop();
                    ok
                }
            },
            Ty::Union(arms) => arms.iter().all(|a| self.ty_immutable(a, visiting)),
            Ty::Tuple(elems) => elems.iter().all(|e| self.ty_immutable(e, visiting)),
            // Arrays are index-assignable without `Mut`.
            Ty::Array(_) => false,
            // Function values are opaque and immutable.
            Ty::Fn { .. } => true,
            // [cmp-carry] An identity is not a value, so no value of this
            // "type" exists to be immutable; conservative, like the rest.
            Ty::FnName(_) => false,
            Ty::Var(_) | Ty::Any | Ty::Never | Ty::Unknown => false,
        }
    }

    /// Approximates a written field type as a checker `Ty` under a
    /// generic substitution — just enough structure for the
    /// immutability analysis [kt-copy].
    fn approx_ty(t: &Type, subst: &HashMap<String, Ty>) -> Option<Ty> {
        match t {
            Type::Named { qualifiers, base } => {
                if qualifiers.is_empty() && base.args.is_empty() {
                    if let Some(ty) = subst.get(&base.name.name) {
                        return Some(ty.clone());
                    }
                }
                let args: Option<Vec<Ty>> =
                    base.args.iter().map(|a| Self::approx_ty(a, subst)).collect();
                let named = Ty::Named {
                    name: base.name.name.clone(),
                    args: args?,
                };
                let quals: Vec<salvo_core::Qual> = qualifiers
                    .iter()
                    .map(|q| salvo_core::Qual {
                        effect: false,
                        name: q.name.name.clone(),
                        args: Vec::new(),
                    })
                    .collect();
                Some(named.qualify(quals))
            }
            Type::QualifiedGroup { qualifiers, base, .. } => {
                let inner = Self::approx_ty(base, subst)?;
                let quals: Vec<salvo_core::Qual> = qualifiers
                    .iter()
                    .map(|q| salvo_core::Qual {
                        effect: false,
                        name: q.name.name.clone(),
                        args: Vec::new(),
                    })
                    .collect();
                Some(inner.qualify(quals))
            }
            Type::Union { arms, .. } => {
                let arms: Option<Vec<Ty>> =
                    arms.iter().map(|a| Self::approx_ty(a, subst)).collect();
                Some(Ty::Union(arms?))
            }
            Type::Tuple { elems, .. } => {
                let elems: Option<Vec<Ty>> =
                    elems.iter().map(|e| Self::approx_ty(e, subst)).collect();
                Some(Ty::Tuple(elems?))
            }
            Type::Array { elem, .. } => {
                Some(Ty::Array(Box::new(Self::approx_ty(elem, subst)?)))
            }
            Type::Nullable { inner, .. } => {
                Some(Ty::Union(vec![Self::approx_ty(inner, subst)?, Ty::none()]))
            }
            Type::Fn { .. } => Some(Ty::Fn {
                contract: None,
                effects: Vec::new(),
                params: Vec::new(),
                ret: Box::new(Ty::Unknown),
            }),
        }
    }

    /// [implicit-resolve] What a call passes for each implicit parameter:
    /// the value written at the call site, the enclosing fn's own implicit
    /// forwarded on, or a reference to the fn resolution found.
    fn emit_implicit_args(&mut self, named: &[NamedArg], span: Span) -> Vec<String> {
        let filled = match self.checked.implicit_args.get(&(self.file_idx, span)) {
            Some(filled) => filled.clone(),
            None => return Vec::new(),
        };
        let mut out = Vec::new();
        for arg in &filled {
            match arg {
                salvo_core::ImplicitArg::Given { name, .. } => {
                    match named.iter().find(|a| a.name.name == *name) {
                        Some(a) => out.push(self.emit_expr(&a.value)),
                        None => {
                            // The checker recorded a value that is not in the
                            // AST: a table lost an entry [backend-never-wrong].
                            self.error(format!(
                                "internal: no value for implicit parameter `{name}`"
                            ));
                            out.push("TODO()".to_string());
                        }
                    }
                }
                salvo_core::ImplicitArg::Forwarded { name } => {
                    out.push(kt_ident(name));
                }
                salvo_core::ImplicitArg::Resolved { name, key, want } => {
                    match self.fn_by_key(*key) {
                        Some(decl) => {
                            // [copy-implicit] `copy` lowers by the argument's
                            // *shape*, which as a value is the position's
                            // concrete parameter type — known here, where a
                            // generic body could not know it [kt-copy].
                            if decl.intrinsic && decl.name.name == "copy" {
                                let param_ty = match want.strip_quals() {
                                    Ty::Fn { params, .. } => params.first().cloned(),
                                    _ => None,
                                };
                                let copied = param_ty
                                    .as_ref()
                                    .and_then(|t| self.copy_code(t, "__i0"));
                                match copied {
                                    Some(body) => out.push(format!("{{ __i0 -> {body} }}")),
                                    None => {
                                        self.error(format!(
                                            "the kotlin backend cannot `copy` a value of type `{}` yet",
                                            param_ty.map(|t| t.to_string()).unwrap_or_default()
                                        ));
                                        out.push("TODO()".to_string());
                                    }
                                }
                                continue;
                            }
                            // [implicit-intrinsic] An `intrinsic fn` has no
                            // Kotlin name to reference: it *is* a lowering.
                            // So the value passed is an adapter lambda whose
                            // body is that lowering — which is what makes
                            // std's `iter` fill an `?Iterable<It, T>`.
                            if decl.intrinsic {
                                out.push(self.intrinsic_fn_value(decl));
                            } else {
                                let target = self.kotlin_fn_name(decl);
                                out.push(format!("::{target}"));
                            }
                        }
                        None => {
                            self.error(format!(
                                "internal: implicit parameter `{name}` resolved to no fn"
                            ));
                            out.push("TODO()".to_string());
                        }
                    }
                }
                salvo_core::ImplicitArg::OriginNext { name: _, next_fn } => {
                    // [iter-fn] The pass is the machine minted at the
                    // argument, so the adapter wraps its own advance into the
                    // protocol's two arms — the same machine API the `for`
                    // lowering drives (`__advance` → `Boolean`, then
                    // `__current()`).
                    let Some(decl) = self.fn_by_key(*next_fn) else {
                        self.error(
                            "the `yield fn` behind a minted pass is not available".to_string(),
                        );
                        out.push("TODO()".to_string());
                        continue;
                    };
                    let elem = match decl.return_type.as_ref() {
                        Some(t) => self.emit_type(t),
                        None => "Unit".to_string(),
                    };
                    let effects: Vec<Ty> = self
                        .checked
                        .fn_effects
                        .get(next_fn)
                        .cloned()
                        .unwrap_or_default()
                        .into_iter()
                        .filter(|t| !is_throw_effect_ty(t))
                        .collect();
                    let mut hs: Vec<String> = effects
                        .iter()
                        .map(|ty| self.thread_effect_fused_by_ty(ty))
                        .collect();
                    // [kt-effect-fusion] The machine's advance is generated
                    // under the same fused convention as any callee.
                    if self.fusion {
                        hs.dedup();
                        hs.truncate(1);
                    }
                    let hs = hs.join(", ");
                    self.union_sizes.insert(2);
                    // The machine's type is written on the adapter's parameter:
                    // it is what pins the callee's `It`, which kotlinc cannot
                    // infer from a bare lambda parameter.
                    let ann = match self.mint_machines.last() {
                        Some(machine) => format!("__p: {machine}"),
                        None => "__p".to_string(),
                    };
                    // An anonymous *function* rather than a lambda: its
                    // declared return type is what pins the callee's element
                    // type, which kotlinc will not infer from two `U2_n`
                    // branches ("cannot infer type for type parameter 'T'").
                    out.push(format!(
                        "fun({ann}): Union2<{elem}, Finished> {{ return \
                         if (__p.__advance({hs})) U2_1<{elem}, Finished>(__p.__current()) \
                         else U2_2<{elem}, Finished>(finished()) }}"
                    ));
                }
            }
        }
        out
    }

    /// [implicit-intrinsic] An `intrinsic fn` passed as a *value*: there is    /// no Kotlin function to reference, so the value is an adapter lambda
    /// whose body is the intrinsic's own lowering, applied to the adapter's
    /// parameters. Reached from implicit resolution [implicit-resolve],
    /// where std's `iter` overloads are what fill an `?Iterable<It, T>`.
    ///
    /// A lowering that needs the call's *type arguments* has none here (a
    /// fn value's are the caller's), so an intrinsic like `list` would not
    /// render correctly as a value; it is a codegen error rather than a
    /// guess [backend-never-wrong].
    fn intrinsic_fn_value(&mut self, decl: &FnDecl) -> String {
        let params: Vec<String> = (0..decl.params.iter().filter(|p| !p.implicit).count())
            .map(|i| format!("__i{i}"))
            .collect();
        let recv = decl.params.first().and_then(|p| type_base_name(&p.ty));
        // [cmp-groups] The lowering the adapter's body becomes may need a
        // runtime file of its own, exactly as a direct call to it would:
        // `cmp(Str, Str)` is `__salvoCompare`, and an adapter is the one place
        // a program can reach it without ever calling `cmp` directly.
        if decl.name.name == "cmp" && recv == Some("Str") {
            self.needs_compare = true;
        }
        match crate::intrinsics::fn_call(&decl.name.name, recv, &params, &[], None, None) {
            Some(body) => format!("{{ {} -> {body} }}", params.join(", ")),
            None => {
                self.error(format!(
                    "intrinsic fn `{}` is not supported by the kotlin backend",
                    decl.name.name
                ));
                "TODO()".to_string()
            }
        }
    }

    /// [op-order] [op-equality] A comparison that goes through the capability's
    /// function: the call, then the operator applied to its answer. The call
    /// itself is emitted by the ordinary paths, so an intrinsic (`cmp(Str, Str)`
    /// → the runtime comparator, which is how this backend gets code-point
    /// order), a canonical and a generated structural member [cmp-auto] all
    /// need no special handling.
    fn emit_compare_via(
        &mut self,
        op: BinaryOp,
        lhs: &Expr,
        rhs: &Expr,
        via: &salvo_core::CompareVia,
        span: Span,
    ) -> String {
        let args: Vec<&Expr> = vec![lhs, rhs];
        let call = match via {
            salvo_core::CompareVia::Call(key) => match self.fn_by_key(*key) {
                Some(decl) => {
                    let name = decl.name.name.clone();
                    if decl.intrinsic {
                        self.emit_intrinsic_call(decl, &args, span)
                    } else {
                        self.emit_fn_call(&name, decl, &[], &args, &[], span)
                    }
                }
                None => {
                    self.error(
                        "internal: the `cmp`/`eq` a comparison resolved to is not available"
                            .to_string(),
                    );
                    "TODO()".to_string()
                }
            },
            // [implicit-forward] Through the enclosing fn's own parameter — the
            // only thing that can compare an opaque `T`. No conventions to
            // bridge on this backend: everything is a reference.
            salvo_core::CompareVia::Implicit(name) => {
                let code: Vec<String> = args.iter().map(|a| self.emit_expr(a)).collect();
                format!("{}({})", kt_ident(name), code.join(", "))
            }
        };
        match op {
            BinaryOp::Eq => call,
            BinaryOp::NotEq => format!("!{call}"),
            other => format!("{call} {} 0", binary_op(other)),
        }
    }

    fn emit_fn_call(
        &mut self,
        name: &str,
        f: &FnDecl,
        type_args: &[Type],
        args: &[&Expr],
        named: &[NamedArg],
        span: Span,
    ) -> String {
        let mut all: Vec<String> = Vec::new();
        // [fn-effects] A producer call mints a pass and runs none of the
        // body, so it takes no handlers — even though `call_effects` records
        // the claim at this span when the call is a `for` subject: that entry
        // is the *drive* site's, and the drive site is where it is read.
        match self
            .checked
            .call_effects
            .get(&(self.file_idx, span))
            .cloned()
        {
            Some(effs) if effs.iter().all(ty_is_concrete) => {
                for ty in &effs {
                    // [kt-throw-signal] Not a capability: nothing to pass.
                    if is_throw_effect_ty(ty) {
                        continue;
                    }
                    all.push(self.thread_effect_fused_by_ty(ty));
                }
            }
            _ => {
                for eff in f.effects.iter().flatten() {
                    if let EffectRef::Effect(r) | EffectRef::LocalEffect(r) = eff {
                        if r.name.name == salvo_core::THROW_EFFECT {
                            continue;
                        }
                        let ty = self.emit_type_ref(r);
                        all.push(self.thread_effect_fused_by_type(&ty));
                    }
                }
            }
        }
        // [kt-effect-fusion] The callee takes *one* fused value, whatever
        // the size of its effect set: every effect in scope threads through
        // the same carrier, so the per-effect arguments collapse to one.
        if self.fusion {
            all.dedup();
            all.truncate(1);
        }
        let outer_mints = std::mem::take(&mut self.pending_mints);
        // [kt-variadic] [fn-variadic] The variadic tail is one `Array<T>`
        // argument, matching the parameter. `arrayOf` takes Kotlin's own
        // spread, so a plain tail, a lone `...spread` and a mixture all build
        // the same way — and a lone spread passes straight through, since it
        // already *is* the array.
        let variadic_at = f.params.iter().filter(|p| !p.implicit).position(|p| p.variadic);
        let fixed = variadic_at.unwrap_or(args.len());
        for a in args.iter().take(fixed) {
            all.push(self.emit_expr(a));
        }
        if variadic_at.is_some() {
            let tail: Vec<&&Expr> = args.iter().skip(fixed).collect();
            let lone_spread =
                tail.len() == 1 && matches!(tail[0], Expr::Spread { .. });
            if lone_spread {
                if let Expr::Spread { operand, .. } = tail[0] {
                    all.push(self.emit_expr(operand));
                }
            } else {
                let items: Vec<String> =
                    tail.into_iter().map(|a| self.emit_expr(a)).collect();
                all.push(format!("arrayOf({})", items.join(", ")));
            }
        }
        // [implicit-resolve] The implicit parameters, in the callee's order:
        // ordinary trailing arguments, so nothing about them is visible in
        // the emitted Kotlin.
        all.extend(self.emit_implicit_args(named, span));
        let generics = self.emit_type_args(type_args);
        // A call through an import alias keeps the alias: the generated
        // Kotlin alias import maps it to the declaration [kt-imports].
        // A mangled qualified overload keeps the same `__Qual` suffix on
        // the alias [kt-qual-mangling].
        let kotlin_name = self.kotlin_fn_name(f);
        // [fn-rename] A rename is erased: the call spells the declaration's
        // own (mangled) name, where an *alias* import keeps the alias
        // [kt-imports]. The checker says which of the two this is.
        let renamed = self.checked.renamed_calls.contains(&(self.file_idx, span));
        let kt_name = if name != f.name.name && !renamed {
            aliased_symbol(&kotlin_name, &f.name.name, name)
        } else {
            kotlin_name
        };
        let call = format!("{kt_name}{generics}({})", all.join(", "));
        // [iter-fn] A minted pass is released here, so a combinator
        // that abandons it early cannot leak it. `__close` is idempotent, so
        // a drained pass pays nothing.
        let mints = std::mem::replace(&mut self.pending_mints, outer_mints);
        if mints.is_empty() {
            return call;
        }
        let ctors: String = mints
            .iter()
            .map(|(_, ctor, _)| format!("{ctor}; "))
            .collect();
        let closes: String = mints
            .iter()
            .filter(|(_, _, close)| !close.is_empty())
            .map(|(_, _, close)| format!("{close}; "))
            .collect();
        format!("run {{ {ctors}val __call = {call}; {closes}__call }}")
    }

    /// [use-no-dup] [effect-intercept] The environment read as a *scope*:
    /// innermost first, with a registration another one shadows hidden. A
    /// `use` may shadow an earlier one for the same instance (interception),
    /// so every resolution below reads this rather than the raw stack —
    /// otherwise the outer handler would answer a call the inner one
    /// shadowed, which is a silent divergence from Rust (found exactly that
    /// way, 2026-09-14: a shadowed `Greeter` printed the *outer* greeting
    /// here and the inner one there).
    fn visible_effects(&self) -> Vec<EffectEntry> {
        let mut out: Vec<EffectEntry> = Vec::new();
        for e in self.effect_env.iter().rev() {
            let seen = out.iter().any(|k: &EffectEntry| match (&k.ty, &e.ty) {
                (Some(x), Some(y)) => x == y,
                _ => k.rendered == e.rendered,
            });
            if !seen {
                out.push(e.clone());
            }
        }
        out
    }

    /// Resolves the handler expression for a call to an effect member fn.
    fn lookup_effect_handler(&mut self, effect: &str, type_args: &[Type]) -> String {
        if !type_args.is_empty() {
            let full = format!("{effect}{}", self.emit_type_args(type_args));
            return self.lookup_effect_handler_by_type(&full);
        }
        let visible = self.visible_effects();
        let matches: Vec<&EffectEntry> = visible
            .iter()
            .filter(|e| rendered_base(&e.rendered) == effect)
            .collect();
        match matches.as_slice() {
            [one] => one.expr.clone(),
            [] => {
                self.error(format!(
                    "no handler for effect `{effect}` in scope (declare it in the \
                     function's effect list or `use` a handler)"
                ));
                "TODO()".to_string()
            }
            many => {
                // Genuine ambiguity is *different* instances of a generic
                // effect; repeats of one instance were already collapsed by
                // `visible_effects` [use-no-dup].
                let first = many[0].expr.clone();
                self.error(format!(
                    "ambiguous effect call: multiple `{effect}` handlers in scope; \
                     specify the type, e.g. `next_random<Int>()`"
                ));
                first
            }
        }
    }

    /// Resolves a handler by the checker's effect type — the primary,
    /// rendering-drift-immune path. Falls back to the rendered form for
    /// entries that only exist as AST renderings.
    fn lookup_effect_handler_by_ty(&mut self, ty: &Ty) -> String {
        if let Some(e) = self
            .visible_effects()
            .into_iter()
            .find(|e| e.ty.as_ref() == Some(ty))
        {
            return e.expr;
        }
        let rendered = self.kotlin_ty(ty);
        self.lookup_effect_handler_by_type(&rendered)
    }

    fn lookup_effect_handler_by_type(&mut self, effect_ty: &str) -> String {
        let visible = self.visible_effects();
        if let Some(e) = visible.iter().find(|e| e.rendered == effect_ty) {
            return e.expr.clone();
        }
        // Fall back to a unique same-base-name match (generic callee effects
        // like `Random<T>` against a concrete `Random<Int>` in scope).
        let base = rendered_base(effect_ty);
        let matches: Vec<&EffectEntry> = visible
            .iter()
            .filter(|e| rendered_base(&e.rendered) == base)
            .collect();
        if matches.len() == 1 {
            return matches[0].expr.clone();
        }
        self.error(format!(
            "no handler for effect `{effect_ty}` in scope (declare it in the \
             function's effect list or `use` a handler)"
        ));
        "TODO()".to_string()
    }

    /// The environment entry behind an effect instance, resolved exactly as
    /// [`Self::lookup_effect_handler_by_ty`] resolves handler expressions.
    fn effect_entry_by_ty(&mut self, ty: &Ty) -> Option<EffectEntry> {
        if let Some(e) = self
            .visible_effects()
            .into_iter()
            .find(|e| e.ty.as_ref() == Some(ty))
        {
            return Some(e);
        }
        let rendered = self.kotlin_ty(ty);
        self.effect_entry_by_type(&rendered)
    }

    fn effect_entry_by_type(&mut self, effect_ty: &str) -> Option<EffectEntry> {
        let visible = self.visible_effects();
        if let Some(e) = visible.iter().find(|e| e.rendered == effect_ty) {
            return Some(e.clone());
        }
        let base = rendered_base(effect_ty);
        let matches: Vec<&EffectEntry> = visible
            .iter()
            .filter(|e| rendered_base(&e.rendered) == base)
            .collect();
        if matches.len() == 1 {
            return Some(matches[0].clone());
        }
        None
    }

    /// [kt-effect-fusion] The threading expression for one effect of a
    /// *fused callee*: the fused value carrying it. Call sites collect one
    /// per effect and collapse the run of identical carriers to a single
    /// argument. Plain mode: the per-effect handler expression, unchanged.
    fn thread_effect_fused_by_ty(&mut self, ty: &Ty) -> String {
        if !self.fusion {
            return self.lookup_effect_handler_by_ty(ty);
        }
        match self.effect_entry_by_ty(ty) {
            Some(EffectEntry {
                fused: Some(var), ..
            }) => var,
            Some(e) => {
                self.error(format!(
                    "internal: effect `{}` has no fused carrier in scope",
                    e.rendered
                ));
                e.expr
            }
            None => {
                self.error(format!(
                    "no handler for effect `{ty}` in scope (declare it in the \
                     function's effect list or `use` a handler)"
                ));
                "TODO()".to_string()
            }
        }
    }

    /// Rendered-type variant of [`Self::thread_effect_fused_by_ty`], for
    /// unchecked contexts.
    fn thread_effect_fused_by_type(&mut self, effect_ty: &str) -> String {
        if !self.fusion {
            return self.lookup_effect_handler_by_type(effect_ty);
        }
        match self.effect_entry_by_type(effect_ty) {
            Some(EffectEntry {
                fused: Some(var), ..
            }) => var,
            Some(e) => {
                self.error(format!(
                    "internal: effect `{}` has no fused carrier in scope",
                    e.rendered
                ));
                e.expr
            }
            None => {
                self.error(format!(
                    "no handler for effect `{effect_ty}` in scope (declare it in the \
                     function's effect list or `use` a handler)"
                ));
                "TODO()".to_string()
            }
        }
    }

    /// [kt-effect-fusion] Registers (and names) the Has-accessor interface
    /// of an effect *instance*: `Random<Int>` → interface `__Has_Random_Int`
    /// with property `__fx_Random_Int`. The definitions are merged
    /// program-wide into `fx.kt`; per instance, not per declaration,
    /// because erasure forbids one class implementing two instances of a
    /// generic interface — the reason [kt-effect-facets] existed.
    fn has_iface(&mut self, rendered: &str) -> (String, String) {
        let stem = sanitize_instance(rendered);
        let iface = format!("__Has_{stem}");
        let prop = format!("__fx_{stem}");
        match self.has_ifaces.get(&iface) {
            Some(existing) if existing != rendered => {
                self.error(format!(
                    "the generated accessor interface `{iface}` is claimed by two \
                     different effects (`{existing}` and `{rendered}`) — rename one \
                     of the effects"
                ));
            }
            Some(_) => {}
            None => {
                self.has_ifaces.insert(iface.clone(), rendered.to_string());
            }
        }
        (iface, prop)
    }

    /// [iter-for-native] [col-map-iter] The subject of a *native* `for`, as
    /// Kotlin iterates it. A list, an array, a `Set` and a `Str` iterate their
    /// elements already; a **map** iterates its entries, where Salvo iterates
    /// its **keys** (`iter(map)` answers a `MapKeyYield`) — so the keys are
    /// asked for. Without this a `for k in m` bound each key to a whole
    /// `k=v` entry and printed it, while the Rust backend did not compile at
    /// all: found 2026-09-14 while writing `MemFs`, and the silent half is
    /// exactly what [backend-never-wrong] forbids.
    fn native_for_subject(&mut self, iterable: &Expr, code: String) -> String {
        let base = match self.ty_of(iterable.span()).map(|t| t.strip_quals()) {
            Some(Ty::Named { name, .. }) => Some(name.clone()),
            _ => None,
        };
        match base.as_deref() {
            Some("Map") | Some("SortedMap") => format!("{code}.keys"),
            _ => code,
        }
    }

    /// [kt-effect-fusion] Whether an effect set can be fused *here*: the
    /// Has-accessor interfaces are program-wide generated types, so an
    /// effect instance that is still **generic** at this site cannot be one
    /// of their properties (nor a bound on a carrier). Reported rather than
    /// emitted as an undeclared `T` [backend-never-wrong] — the same cut
    /// Rust states as "a `use` whose effect instance is still generic"
    /// ([rs-effect-fusion]); before 2026-09-14 Kotlin let it through and
    /// kotlinc reported the unresolved name.
    fn check_fusable(&mut self, rendered: &[String]) {
        let unresolved: Vec<String> = self
            .generics
            .iter()
            .filter(|g| rendered.iter().any(|r| mentions_ident(r, g)))
            .cloned()
            .collect();
        if !unresolved.is_empty() {
            let mut names = unresolved;
            names.sort();
            self.error(format!(
                "an effect set fused here is still generic (`{}`): the kotlin \
                 backend needs concrete effect instances for its accessor \
                 classes — a handler's dependency may not mention the handler's \
                 own type parameters",
                names.join("`, `")
            ));
        }
    }

    /// [kt-effect-fusion] One fused class: `override val` per effect in the
    /// set, implementing each instance's Has interface. Used for `use`-site
    /// fusions, for a dependent handler's stored environment, and for the
    /// boundary combiners (platform `main`, lambdas, named-fn adapters).
    ///
    /// Returns the class name, the property name per *input* index (a
    /// property is named after its effect, so it does not move), and the
    /// order the constructor takes its arguments in, as indices into the
    /// input list.
    ///
    /// **Deduplicated** (user decision 2026-09-14): the effects are sorted
    /// into a canonical order and an identical class is reused rather than
    /// re-emitted, so a program with the same effect set in several scopes
    /// gets one class. Sorting is what makes two orderings of one set the
    /// same class — a handler declares its dependencies in its own order
    /// while a `use` site follows the environment's, and those differ
    /// routinely.
    fn emit_fx_class(&mut self, rendered: &[String]) -> (String, Vec<String>, Vec<usize>) {
        self.check_fusable(rendered);
        let props: Vec<String> = rendered.iter().map(|r| self.has_iface(r).1).collect();
        let mut order: Vec<usize> = (0..rendered.len()).collect();
        order.sort_by(|a, b| rendered[*a].cmp(&rendered[*b]));
        let mut fields: Vec<String> = Vec::new();
        let mut ifaces: Vec<String> = Vec::new();
        for i in &order {
            let (iface, prop) = self.has_iface(&rendered[*i]);
            fields.push(format!("    override val {prop}: {},", rendered[*i]));
            ifaces.push(iface);
        }
        let body = format!("(\n{}\n) : {}\n", fields.join("\n"), ifaces.join(", "));
        if let Some(existing) = self.fx_classes.get(&body) {
            return (existing.clone(), props, order);
        }
        self.fusion_id += 1;
        let class = format!("__Fx_{}", self.fusion_id);
        self.fx_classes.insert(body.clone(), class.clone());
        self.generated_items.push(format!("\nclass {class}{body}"));
        (class, props, order)
    }
}

#[derive(Clone, Copy, PartialEq)]
enum StmtCtx {
    /// The only context left: the `IteratorBody` one died with the
    /// `iterator { … }` builder — an iterator fn's `yield`s and `return`s are
    /// steps of the generated machine now [iter-fn], so they never
    /// reach `emit_stmt`. Kept because the next context to need one is
    /// cheaper to add than to thread (removal is part of the I6 sweep).
    Normal,
}

/// How a narrowed place read reaches its value [flow-place].
#[derive(Clone, Copy, PartialEq)]
enum PlaceUnwrap {
    /// Wrapper-union storage: read the matched arm's payload and cast.
    ArmValue,
    /// `T?` storage narrowed to its value arm, where Kotlin's smart cast
    /// does not apply: assert non-null [kt-narrow-field-assert].
    NonNull,
}

/// True when a checker type contains no `Unknown` (inference fully
/// resolved it) — only then is it safe to render it into emitted code.
/// The base type name of an AST type, used to disambiguate overloads in
/// unchecked contexts. `None` for shapes without a single base name
/// (unions, tuples, fn types).
fn type_base_name(ty: &Type) -> Option<&str> {
    match ty {
        Type::Named { base, .. } => Some(base.name.name.as_str()),
        Type::Nullable { inner, .. } => type_base_name(inner),
        Type::QualifiedGroup { base, .. } => type_base_name(base),
        Type::Array { .. } => Some("[]"),
        _ => None,
    }
}

/// Whether emitted code is a bare Kotlin name — the case where a suffix
/// (`.toString()` [str-drop-mut]) needs no parentheses around it.
fn is_plain_name(code: &str) -> bool {
    !code.is_empty()
        && !code.starts_with(|c: char| c.is_ascii_digit())
        && code.chars().all(|c| c.is_alphanumeric() || c == '_')
}

/// The base type name of a *checker* type, aligned with
/// [`type_base_name`]'s conventions so the two are comparable.
fn ty_base_name(ty: &Ty) -> Option<&str> {
    match ty {
        Ty::Named { name, .. } => Some(name),
        Ty::Qualified { base, .. } => ty_base_name(base),
        Ty::Array(_) => Some("[]"),
        Ty::Union(_) => {
            // `T?` compares as its value arm (AST `Nullable` does too).
            let arms = ty.value_arms();
            if arms.len() == 1 {
                ty_base_name(arms[0])
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Narrows same-arity unchecked-call candidates by comparing the checked
/// argument types' base names against the declared parameter types.
/// `None` unless exactly one candidate survives — ambiguous dispatch must
/// error, never guess [backend-never-wrong].
fn disambiguate_unchecked<'p, T: Copy>(
    em: &Emitter<'p>,
    candidates: &[T],
    params: impl Fn(T) -> &'p [Param],
    args: &[&Expr],
) -> Option<T> {
    if candidates.len() == 1 {
        return Some(candidates[0]);
    }
    let survivors: Vec<T> = candidates
        .iter()
        .copied()
        .filter(|c| {
            params(*c).iter().zip(args).all(|(p, a)| {
                let Some(pb) = type_base_name(&p.ty) else { return true };
                let Some(at) = em.ty_of(a.span()) else { return true };
                match ty_base_name(at) {
                    Some(ab) => pb == ab,
                    None => true,
                }
            })
        })
        .collect();
    if survivors.len() == 1 {
        Some(survivors[0])
    } else {
        None
    }
}

fn ty_is_concrete(ty: &Ty) -> bool {
    match ty {
        Ty::Unknown => false,
        Ty::Named { args, .. } => args.iter().all(ty_is_concrete),
        Ty::Qualified { base, .. } => ty_is_concrete(base),
        Ty::Union(arms) => arms.iter().all(ty_is_concrete),
        Ty::Tuple(elems) => elems.iter().all(ty_is_concrete),
        Ty::Array(elem) => ty_is_concrete(elem),
        Ty::Fn { params, ret, .. } => params.iter().all(ty_is_concrete) && ty_is_concrete(ret),
        _ => true,
    }
}
fn effect_param_name(effect_ty: &str) -> String {
    let mut out = String::new();
    for c in effect_ty.chars() {
        match c {
            '<' | ',' => out.push('_'),
            '>' | ' ' | '?' => {}
            c if c.is_uppercase() => {
                if out
                    .chars()
                    .last()
                    .is_some_and(|p| p.is_lowercase() || p.is_ascii_digit())
                {
                    out.push('_');
                }
                out.push(c.to_ascii_lowercase());
            }
            c => out.push(c),
        }
    }
    out
}

/// Kotlin operator precedence for parenthesization (higher binds
/// tighter). Unlike Rust, Kotlin gives comparison a tighter level than
/// equality — the two must not share a slot or `(a == b) < c` would
/// re-render flat and silently re-associate.
fn bin_prec(op: BinaryOp) -> u8 {
    match op {
        BinaryOp::Or => 1,
        BinaryOp::And => 2,
        BinaryOp::Eq | BinaryOp::NotEq => 3,
        BinaryOp::Lt | BinaryOp::Gt | BinaryOp::LtEq | BinaryOp::GtEq => 4,
        BinaryOp::Add | BinaryOp::Sub => 5,
        BinaryOp::Mul | BinaryOp::Div | BinaryOp::Rem => 6,
    }
}

/// The base name of a rendered effect type (`Random<Int>` -> `Random`).
fn rendered_base(rendered: &str) -> &str {
    rendered.split('<').next().unwrap_or(rendered)
}

/// The alias-side Kotlin name for an aliased fn import: a mangled
/// qualified overload (`name__Qual` [kt-qual-mangling]) keeps the same
/// suffix on the alias, so the generated alias import and aliased call
/// sites agree on the symbol.
fn aliased_symbol(kotlin_name: &str, decl_name: &str, alias: &str) -> String {
    match kotlin_name.strip_prefix(&format!("{decl_name}__")) {
        Some(suffix) => format!("{alias}__{suffix}"),
        None => kt_ident(alias),
    }
}

fn binary_op(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::Add => "+",
        BinaryOp::Sub => "-",
        BinaryOp::Mul => "*",
        BinaryOp::Div => "/",
        BinaryOp::Rem => "%",
        BinaryOp::Eq => "==",
        BinaryOp::NotEq => "!=",
        BinaryOp::Lt => "<",
        BinaryOp::Gt => ">",
        BinaryOp::LtEq => "<=",
        BinaryOp::GtEq => ">=",
        BinaryOp::And => "&&",
        BinaryOp::Or => "||",
    }
}

fn escape_string(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            '$' => out.push_str("\\$"),
            c => out.push(c),
        }
    }
    out
}

fn escape_char(c: char) -> String {
    match c {
        '\'' => "\\'".to_string(),
        '\\' => "\\\\".to_string(),
        '\n' => "\\n".to_string(),
        '\t' => "\\t".to_string(),
        '\r' => "\\r".to_string(),
        c => c.to_string(),
    }
}

fn is_none_type(ty: &Type) -> bool {
    matches!(ty, Type::Named { qualifiers, base } if qualifiers.is_empty() && base.name.name == "None")
}

/// The qualifier names appearing on a fn's parameter types, joined for
/// overload-mangling suffixes (`__Surname`).
fn qual_suffix(decl: &FnDecl) -> String {
    let mut parts: Vec<String> = Vec::new();
    for p in &decl.params {
        match &p.ty {
            Type::Named { qualifiers, .. } | Type::QualifiedGroup { qualifiers, .. } => {
                for q in qualifiers {
                    // Dot-names canonicalize to their flattened spelling:
                    // a mangled fn name is a single identifier [name-dot].
                    parts.push(q.name.name.replace('.', ""));
                }
            }
            _ => {}
        }
    }
    parts.join("_")
}

/// Substitutes generic parameters in an AST type (for alias expansion).
fn subst_ast_type(ty: &Type, map: &std::collections::HashMap<&str, &Type>) -> Type {
    match ty {
        Type::Named { qualifiers, base } => {
            if qualifiers.is_empty() && base.args.is_empty() {
                if let Some(replacement) = map.get(base.name.name.as_str()) {
                    return (*replacement).clone();
                }
            }
            Type::Named {
                qualifiers: qualifiers.clone(),
                base: TypeRef {
                    value_args: Vec::new(),
                    at: None,
                    binder: false,
                    established: false,
                    alias: None,
                    name: base.name.clone(),
                    args: base.args.iter().map(|a| subst_ast_type(a, map)).collect(),
                    from: base.from.clone(),
                    span: base.span,
                },
            }
        }
        Type::QualifiedGroup {
            qualifiers,
            base,
            span,
        } => Type::QualifiedGroup {
            qualifiers: qualifiers.clone(),
            base: Box::new(subst_ast_type(base, map)),
            span: *span,
        },
        Type::Union { arms, span } => Type::Union {
            arms: arms.iter().map(|a| subst_ast_type(a, map)).collect(),
            span: *span,
        },
        Type::Tuple { elems, span } => Type::Tuple {
            elems: elems.iter().map(|e| subst_ast_type(e, map)).collect(),
            span: *span,
        },
        Type::Array { elem, span } => Type::Array {
            elem: Box::new(subst_ast_type(elem, map)),
            span: *span,
        },
        Type::Nullable { inner, span } => Type::Nullable {
            inner: Box::new(subst_ast_type(inner, map)),
            span: *span,
        },
        Type::Fn {
            params,
            param_names,
            effects,
            deductions,
            ret,
            span,
        } => Type::Fn {
            params: params.iter().map(|p| subst_ast_type(p, map)).collect(),
            param_names: param_names.clone(),
            effects: effects.clone(),
            deductions: deductions.clone(),
            ret: Box::new(subst_ast_type(ret, map)),
            span: *span,
        },
    }
}

/// Collects names that are assigned or incremented anywhere in the block
/// (they must become `var` in Kotlin).
fn collect_mutated(block: &Block, out: &mut HashSet<String>) {
    for stmt in &block.stmts {
        match stmt {
            Stmt::Assign { target, value, .. } => {
                if let Expr::Ident(id) = target {
                    out.insert(id.name.clone());
                }
                collect_mutated_expr(target, out);
                collect_mutated_expr(value, out);
            }
            Stmt::Let { value, .. } => collect_mutated_expr(value, out),
            Stmt::Use { handler, .. } => collect_mutated_expr(handler, out),
            Stmt::Expr(e) => collect_mutated_expr(e, out),
            _ => {}
        }
    }
}

fn collect_mutated_expr(expr: &Expr, out: &mut HashSet<String>) {
    match expr {
        // [assert-fn] A condition or a message may mutate, like any expression.
        Expr::Assert { cond, message, .. } => {
            collect_mutated_expr(cond, out);
            if let Some(m) = message {
                collect_mutated_expr(m, out);
            }
        }
        Expr::Unreachable { message, .. } => {
            if let Some(m) = message {
                collect_mutated_expr(m, out);
            }
        }
        // [elvis] Both sides may mutate.
        Expr::Elvis { subject, rhs, .. } => {
            collect_mutated_expr(subject, out);
            collect_mutated_expr(rhs, out);
        }
        Expr::SafeField { inner, .. } => collect_mutated_expr(inner, out),
        Expr::Placeholder { .. } => {}
        // [expr-escape] The escapes carry a value expression.
        Expr::Return { value, .. } | Expr::Break { value, .. } => {
            if let Some(v) = value {
                collect_mutated_expr(v, out);
            }
        }
        Expr::Continue { .. } => {}
        Expr::IncDec { operand, .. } => {
            if let Expr::Ident(id) = operand.as_ref() {
                out.insert(id.name.clone());
            }
        }
        // [fn-overload-at] Only the dot-notation receiver is an expression.
        Expr::Scoped { base, .. } | Expr::EffectScoped { base, .. } => {
            if let Some(base) = base {
                collect_mutated_expr(base, out);
            }
        }
        Expr::If {
            branches,
            else_block,
            ..
        } => {
            for (cond, block) in branches {
                collect_mutated_expr(cond, out);
                collect_mutated(block, out);
            }
            if let Some(b) = else_block {
                collect_mutated(b, out);
            }
        }
        Expr::While {
            cond,
            body,
            else_block,
            ..
        } => {
            collect_mutated_expr(cond, out);
            collect_mutated(body, out);
            if let Some(b) = else_block {
                collect_mutated(b, out);
            }
        }
        Expr::For {
            iterable,
            body,
            else_block,
            ..
        } => {
            collect_mutated_expr(iterable, out);
            collect_mutated(body, out);
            if let Some(b) = else_block {
                collect_mutated(b, out);
            }
        }
        Expr::When {
            subject, branches, ..
        } => {
            collect_mutated_expr(subject, out);
            for b in branches {
                collect_mutated(&b.body, out);
            }
        }
        // [when-condition]
        Expr::WhenCond {
            branches,
            else_block,
            ..
        } => {
            for (cond, block) in branches {
                collect_mutated_expr(cond, out);
                collect_mutated(block, out);
            }
            collect_mutated(else_block, out);
        }
        Expr::Call { callee, args, .. } => {
            collect_mutated_expr(callee, out);
            for a in args {
                collect_mutated_expr(a, out);
            }
        }
        Expr::Binary { lhs, rhs, .. } => {
            collect_mutated_expr(lhs, out);
            collect_mutated_expr(rhs, out);
        }
        Expr::Unary { operand, .. }
        | Expr::NonNull { operand, .. }
        | Expr::Spread { operand, .. } => collect_mutated_expr(operand, out),
        Expr::Field { base, .. } | Expr::TupleIndex { base, .. } => {
            collect_mutated_expr(base, out)
        }
        Expr::Index { base, index, .. } => {
            collect_mutated_expr(base, out);
            collect_mutated_expr(index, out);
        }
        Expr::Is { subject, .. } | Expr::Widen { subject, .. } => {
            collect_mutated_expr(subject, out)
        }
        Expr::Str { parts, .. } => {
            for p in parts {
                if let StrExprPart::Interp(e) = p {
                    collect_mutated_expr(e, out);
                }
            }
        }
        Expr::ArrayLit { elems, .. }
        | Expr::SetLit { elems, .. }
        | Expr::Tuple { elems, .. } => {
            for e in elems {
                collect_mutated_expr(e, out);
            }
        }
        Expr::MapLit { entries, .. } => {
            for (k, v) in entries {
                collect_mutated_expr(k, out);
                collect_mutated_expr(v, out);
            }
        }
        Expr::StructLit { fields, .. } => {
            for f in fields {
                match &f.kind {
                    StructLitFieldKind::Named { value, .. } => collect_mutated_expr(value, out),
                    StructLitFieldKind::Spread(e) => collect_mutated_expr(e, out),
                }
            }
        }
        Expr::Lambda { body, .. } => match body {
            LambdaBody::Expr(e) => collect_mutated_expr(e, out),
            LambdaBody::Block(b) => collect_mutated(b, out),
        },
        // [try] The delimiter's body is ordinary code: a variable mutated
        // only inside it still needs the mutable declaration.
        Expr::Try { body, .. } => collect_mutated(body, out),
        // [actor-spawn-expr] [actor-replyto] [actor-waitfor] The clauses,
        // captures and bridge block are ordinary code.
        Expr::Spawn {
            handler,
            with_items,
            pool,
            ..
        } => {
            collect_mutated_expr(handler, out);
            for handler in with_items {
                collect_mutated_expr(handler, out);
            }
            if let Some(pool) = pool {
                collect_mutated_expr(pool, out);
            }
        }
        Expr::ReplyTo { captures, .. } => {
            for capture in captures {
                collect_mutated_expr(capture, out);
            }
        }
        // [actor-self-send] A leaf.
        Expr::SelfScoped { .. } => {}
        Expr::WaitFor { body, .. } => collect_mutated(body, out),
        // Leaves: no sub-expression, so nothing can be mutated inside.
        // Listed rather than defaulted, because a missed form emits an
        // immutable declaration for a variable the code assigns and the
        // target compiler is what reports it [backend-never-wrong].
        Expr::Ident(_)
        | Expr::Int { .. }
        | Expr::Float { .. }
        | Expr::Bool { .. }
        | Expr::Char { .. }
        | Expr::Error { .. } => {}
    }
}

/// Collects every name a block binds (`let` patterns, `is` bindings,
/// `when` bindings, `for` patterns, lambda parameters), recursively.
/// Used to keep generated effect-parameter and `use` variable names from
/// colliding with user locals [kt-effect-params].
fn collect_declared(block: &Block, out: &mut HashSet<String>) {
    for stmt in &block.stmts {
        match stmt {
            Stmt::Let { pattern, value, .. } => {
                collect_pattern_names(pattern, out);
                collect_declared_expr(value, out);
            }
            Stmt::Assign { target, value, .. } => {
                collect_declared_expr(target, out);
                collect_declared_expr(value, out);
            }
            Stmt::Use { handler, .. } => collect_declared_expr(handler, out),
            Stmt::Expr(e) => collect_declared_expr(e, out),
            _ => {}
        }
    }
}

fn collect_pattern_names(pattern: &Pattern, out: &mut HashSet<String>) {
    match pattern {
        Pattern::Ident(id) => {
            out.insert(id.name.clone());
        }
        Pattern::Tuple { elems, .. } => {
            for p in elems {
                collect_pattern_names(p, out);
            }
        }
        Pattern::Struct { fields, .. } => {
            for f in fields {
                out.insert(f.binding.name.clone());
            }
        }
    }
}

fn collect_declared_expr(expr: &Expr, out: &mut HashSet<String>) {
    match expr {
        Expr::Is {
            subject, binding, ..
        } => {
            if let Some(b) = binding {
                out.insert(b.name.clone());
            }
            collect_declared_expr(subject, out);
        }
        Expr::If {
            branches,
            else_block,
            ..
        } => {
            for (c, b) in branches {
                collect_declared_expr(c, out);
                collect_declared(b, out);
            }
            if let Some(b) = else_block {
                collect_declared(b, out);
            }
        }
        Expr::When {
            subject, branches, ..
        } => {
            collect_declared_expr(subject, out);
            for b in branches {
                if let Some(binding) = &b.binding {
                    out.insert(binding.name.clone());
                }
                collect_declared(&b.body, out);
            }
        }
        Expr::While {
            cond,
            body,
            else_block,
            ..
        } => {
            collect_declared_expr(cond, out);
            collect_declared(body, out);
            if let Some(b) = else_block {
                collect_declared(b, out);
            }
        }
        Expr::For {
            pattern,
            iterable,
            body,
            else_block,
            ..
        } => {
            collect_pattern_names(pattern, out);
            collect_declared_expr(iterable, out);
            collect_declared(body, out);
            if let Some(b) = else_block {
                collect_declared(b, out);
            }
        }
        Expr::Lambda { params, body, .. } => {
            for p in params {
                out.insert(p.name.name.clone());
            }
            match body {
                LambdaBody::Expr(e) => collect_declared_expr(e, out),
                LambdaBody::Block(b) => collect_declared(b, out),
            }
        }
        Expr::Call { callee, args, .. } => {
            collect_declared_expr(callee, out);
            for a in args {
                collect_declared_expr(a, out);
            }
        }
        Expr::Binary { lhs, rhs, .. } => {
            collect_declared_expr(lhs, out);
            collect_declared_expr(rhs, out);
        }
        Expr::Unary { operand, .. }
        | Expr::NonNull { operand, .. }
        | Expr::IncDec { operand, .. }
        | Expr::Spread { operand, .. } => collect_declared_expr(operand, out),
        Expr::Field { base, .. } | Expr::TupleIndex { base, .. } => {
            collect_declared_expr(base, out)
        }
        Expr::Index { base, index, .. } => {
            collect_declared_expr(base, out);
            collect_declared_expr(index, out);
        }
        Expr::Str { parts, .. } => {
            for p in parts {
                if let StrExprPart::Interp(e) = p {
                    collect_declared_expr(e, out);
                }
            }
        }
        Expr::ArrayLit { elems, .. }
        | Expr::SetLit { elems, .. }
        | Expr::Tuple { elems, .. } => {
            for e in elems {
                collect_declared_expr(e, out);
            }
        }
        Expr::MapLit { entries, .. } => {
            for (k, v) in entries {
                collect_declared_expr(k, out);
                collect_declared_expr(v, out);
            }
        }
        Expr::StructLit { fields, .. } => {
            for f in fields {
                match &f.kind {
                    StructLitFieldKind::Named { value, .. } => collect_declared_expr(value, out),
                    StructLitFieldKind::Spread(e) => collect_declared_expr(e, out),
                }
            }
        }
        _ => {}
    }
}

/// Walks a condition for `is`-checks with bindings and invokes `f` for each
/// (with the span of the `is` expression itself).
/// [is-bind-once] Whether an expression is a **place** — a name or a
/// field/index/tuple chain over one — and so safe to read twice.
/// [actor-mailbox] The `capacity` field's value inside a handler's `mailbox`
/// slot — the slot is a struct literal, so this is one field lookup.
fn mailbox_capacity_expr(mailbox: &Expr) -> Option<&Expr> {
    let Expr::StructLit { fields, .. } = mailbox else {
        return None;
    };
    fields.iter().find_map(|f| match &f.kind {
        salvo_syntax::ast::StructLitFieldKind::Named { name, value } if name.name == "capacity" => {
            Some(value)
        }
        _ => None,
    })
}

fn is_place_expr(expr: &Expr) -> bool {
    match expr {
        Expr::Ident(_) => true,
        Expr::Field { base, .. } | Expr::TupleIndex { base, .. } => is_place_expr(base),
        Expr::Index { base, index, .. } => is_place_expr(base) && is_place_expr(index),
        _ => false,
    }
}

fn collect_is_bindings<'a>(
    cond: &'a Expr,
    f: &mut impl FnMut(&'a Expr, &'a [TypeRef], &'a Ident, Span, bool),
) {
    match cond {
        Expr::Is {
            subject,
            check,
            binding: Some(b),
            span,
        } => f(subject, check, b, *span, false),
        // [qual-lift] `is ^Ok inner` binds too, at the *lifted* type. The
        // binding's own recorded type is what the read is built against, so
        // the same emission serves both — which is why the widen shadow is
        // only needed when there is no binding [rs-widen-shadow].
        Expr::Widen {
            subject,
            quals,
            binding: Some(b),
            span,
        } => f(subject, quals, b, *span, true),
        Expr::Binary {
            op: BinaryOp::And,
            lhs,
            rhs,
            ..
        } => {
            collect_is_bindings(lhs, f);
            collect_is_bindings(rhs, f);
        }
        _ => {}
    }
}
