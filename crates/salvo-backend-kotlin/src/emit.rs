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

use salvo_core::check::{Checked, Coercion, UnionTest};
use salvo_core::types::Ty;
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
    // [kt-throw-signal] Generated once for the whole program, when
    // anything throws.
    let mut needs_throw = false;
    // [kt-ordered] And for the structural comparison a `canbe ordered`
    // struct's `compareTo` uses [col-hashed-ordered].
    let mut needs_compare = false;
    for (file_idx, unit) in program.units().enumerate() {
        if !reachable.contains(&unit.file.module) || !module_produces_code(unit.ast) {
            continue;
        }
        // Generated Kotlin imports: a wildcard per foreign emitted module
        // this file references, plus alias imports for aliased Salvo
        // imports of Kotlin-visible items [kt-imports].
        let mut emitter = Emitter::new(&symbols, &checked, program, file_idx, &unit.file.name);
        emitter.effect_paths = effect_paths.clone();
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
        needs_throw |= emitter.needs_throw;
        needs_compare |= emitter.needs_compare;
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

    if errors.is_empty() {
        Ok((files, warnings))
    } else {
        Err(errors)
    }
}

fn kotlin_package(module: &ModulePath) -> String {
    let mut out = String::from("salvo");
    for part in &module.0 {
        out.push('.');
        out.push_str(&kt_ident(part));
    }
    out
}

/// [qual-widen] Visits every `^` check a condition applies, through `&&`
/// chains as well.
fn collect_widen_checks<'a>(cond: &'a Expr, f: &mut impl FnMut(&'a Expr, Span)) {
    match cond {
        Expr::Widen { subject, span, .. } => f(subject, *span),
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

/// [kt-ordered] The structural comparison a `canbe ordered` struct's
/// `compareTo` uses for each field [col-hashed-ordered].
///
/// Source in `runtime/compare.kt`, included verbatim and compiled directly
/// by `runtime_tests.rs`.
fn generate_compare_file() -> String {
    include_str!("../runtime/compare.kt").to_string()
}

/// Whether an effect instance is the throw effect [throw]: the JVM unwinds
/// to the delimiter, so it is never a handler parameter [kt-throw-signal].
fn is_throw_effect_ty(ty: &Ty) -> bool {
    matches!(ty, Ty::Named { name, .. } if name == salvo_core::THROW_EFFECT)
}

/// [kt-suppress-cast] Identifier-shaped tokens of a rendered Kotlin type,
/// for matching bare generic parameter names without matching substrings
/// (the `T` in `Triple` is not a token).
fn word_tokens(s: &str) -> impl Iterator<Item = &str> {
    s.split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .filter(|t| !t.is_empty())
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

/// Does this module contain anything that turns into Kotlin code?
fn module_produces_code(module: &Module) -> bool {
    module.items.iter().any(|item| match item {
        Item::Struct(_) | Item::Effect(_) => true,
        Item::Handler(_) => true,
        Item::Fn(f) => f.body.is_some(),
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

    let mut files = Vec::new();
    let mut errors = Vec::new();
    for (file_idx, unit) in program.units().enumerate() {
        if unit.file.is_std {
            continue;
        }
        let effects = salvo_core::platform_effects(unit.ast);
        let entry = salvo_core::platform_entry(unit.ast, &symbols);
        if effects.is_empty() && entry.is_none() {
            continue;
        }
        let module = &unit.file.module;
        let mut emitter =
            Emitter::new(&symbols, &checked, program, file_idx, &unit.file.name);

        let mut body = String::new();
        for e in &effects {
            body.push_str(&emitter.host_impl(e));
        }
        // Imports: the module's own generated package always (the
        // interfaces and the entry point live there), plus the packages of
        // any platform effect declared elsewhere that the entry needs.
        let mut imports: BTreeSet<String> = BTreeSet::new();
        imports.insert(format!("import {}.*", kotlin_package(module)));
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

        let mut content = format!(
            "// Host implementation of the platform effects of Salvo module \
             `{module}`.\n//\n// Generated once by `salvo platform generate`; the \
             compiler never writes\n// this file again — it is yours. Nothing here \
             is checked by Salvo: the\n// Kotlin compiler checks it, against the \
             interfaces the backend generates\n// from the `platform effect` \
             declarations.\npackage {}\n\n",
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
    symbols: &'p Symbols<'p>,
    checked: &'p Checked,
    program: &'p Program,
    file_idx: usize,
    file_name: String,
    imports: BTreeSet<String>,
    errors: Vec<String>,
    /// Wrapper union sizes this emitter has rendered.
    union_sizes: BTreeSet<usize>,
    /// [fn-effects] Every effect set whose generated pass interface this
    /// file mentions, mapped to the *Kotlin* effect types the interface's
    /// `advance` takes as handler parameters. Merged program-wide into
    /// `iter_effects.kt`, the way `union_sizes` drives `unions.kt`.
    /// [fn-effects] Fully-qualified package prefix per effect name, for the
    /// generated pass-interface file (which imports nothing).
    effect_paths: HashMap<String, String>,
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
    /// [kt-ordered] Whether this module declared a `canbe ordered` struct, so
    /// the comparison runtime is emitted.
    needs_compare: bool,
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
/// effect type, and the expression providing the handler.
#[derive(Clone)]
struct EffectEntry {
    ty: Option<Ty>,
    rendered: String,
    expr: String,
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
            symbols,
            checked,
            program,
            file_idx,
            file_name: file_name.to_string(),
            imports: BTreeSet::new(),
            errors: Vec::new(),
            union_sizes: BTreeSet::new(),
            effect_paths: HashMap::new(),
            ret_is_unit: false,
            needs_throw: false,
            needs_compare: false,
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
                    handler_args.push(self.lookup_effect_handler_by_ty(ty));
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
        let var = self.for_pattern_var(pattern);
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
                self.note_payload_cast(&elem);
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

    fn emit_module(&mut self, module: &Module) -> String {
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
                Item::Fn(f) if f.body.is_some() => body.push_str(&self.emit_fn(f)),
                Item::Qualifier(q) => body.push_str(&self.emit_qualifier(q)),
                _ => {}
            }
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
        // [col-hashed-ordered] [kt-ordered] `canbe ordered` needs a real
        // `Comparable`: a Kotlin data class gets `equals`/`hashCode` for free
        // but *not* comparison, so `p < q` would be an unresolved
        // `compareTo`. The order is lexicographic by field declaration order,
        // which is the language's rule and matches Rust's derived `Ord`.
        let ordered = s.auto_qualifiers.iter().any(|q| q.name.name == "ordered");
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
        // barred from `canbe hashed` [col-hashed-ordered], so it never
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
        for f in &e.fns {
            // A member's own generics render on the member
            // [effect-member-generics].
            let member_saved = self.enter_generics(&f.generics);
            let member_generics = self.emit_generic_params(&f.generics);
            let params = self.emit_member_param_list_with_implicits(f);
            let ret = self.emit_return_type(f.return_type.as_ref());
            out.push_str(&format!(
                "    fun{member_generics} {}({params}){ret}\n",
                kt_ident(&f.name.name)
            ));
            self.generics = member_saved;
        }
        out.push_str("}\n");
        self.generics = saved;
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
        for f in &e.fns {
            let member_saved = self.enter_generics(&f.generics);
            let params = self.emit_member_param_list_with_implicits(f);
            let ret = self.emit_return_type(f.return_type.as_ref());
            out.push_str(&format!(
                "    override fun {}({params}){ret} {{\n        \
                 TODO(\"implement {}.{}\")\n    }}\n",
                kt_ident(&f.name.name),
                e.name.name,
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
                    if let EffectRef::Effect(r) = eff {
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

    fn emit_handler(&mut self, h: &HandlerDecl) -> String {
        if h.intrinsic {
            return self.emit_intrinsic_handler(h);
        }
        let saved = self.enter_generics(&h.generics);
        let generics = self.emit_generic_params(&h.generics);
        let of = self.emit_type(&h.of);
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
        let mut out = format!("\nclass {}{generics}{ctor} : {of} {{\n", h.name.name);
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
        let deps = self.handler_dep_entries(h);
        let saved_deps = std::mem::replace(&mut self.handler_deps, deps);
        let ctor_implicits: HashSet<String> = h
            .params
            .iter()
            .filter(|p| p.implicit)
            .map(|p| p.name.name.clone())
            .collect();
        let saved_ctor = std::mem::replace(&mut self.ctor_implicits, ctor_implicits);
        for f in &h.fns {
            out.push_str(&self.emit_fn_inner(f, "override fun", 1, false));
        }
        self.ctor_implicits = saved_ctor;
        self.handler_deps = saved_deps;
        out.push_str("}\n");
        self.generics = saved;
        out
    }

    /// [effect-handler-deps] The handler's effect dependencies as effect
    /// environment entries pointing at their constructor fields.
    fn handler_dep_entries(&mut self, h: &HandlerDecl) -> Vec<EffectEntry> {
        let params: Vec<(String, Type)> = h
            .params
            .iter()
            .filter(|p| {
                type_base_name(&p.ty)
                    .is_some_and(|n| self.symbols.effects.contains_key(n))
            })
            .map(|p| (p.name.name.clone(), p.ty.clone()))
            .collect();
        params
            .into_iter()
            .map(|(name, ty)| EffectEntry {
                ty: None,
                rendered: self.emit_type(&ty),
                expr: kt_ident(&name),
            })
            .collect()
    }

    /// An `intrinsic handler` [backend-intrinsic]: a std handler whose
    /// members this backend implements directly. The signatures come from
    /// the *effect* it implements (the handler declaration is bodyless),
    /// and the bodies from [`crate::intrinsics::handler_member`].
    ///
    /// Like every handler it emits as a class instantiated at its `use`
    /// site — `object` was an artifact of `StdOutConsole` being stateless.
    fn emit_intrinsic_handler(&mut self, h: &HandlerDecl) -> String {
        let of = self.emit_type(&h.of);
        let Some(effect) = type_base_name(&h.of)
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
        for member in &effect.fns {
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
                kt_ident(&member.name.name)
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

    fn emit_fn(&mut self, f: &FnDecl) -> String {
        self.emit_fn_inner(f, "fun", 0, true)
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
        let generics = self.emit_generic_params(&f.generics);
        // Effect dependencies become leading parameters [kt-effect-params],
        // sourced from the checker's lowered effect list when available
        // (checker-`Ty` keys; the AST rendering is the unchecked
        // fallback).
        let mut params: Vec<String> = Vec::new();
        // [effect-handler-deps] A handler member reaches its handler's
        // dependencies through constructor *fields*, so those effects are
        // already provided and must not become leading parameters — the
        // signature has to match the effect interface.
        for entry in self.handler_deps.clone() {
            self.effect_env.push(entry);
        }
        let provided: Vec<String> =
            self.effect_env.iter().map(|e| e.rendered.clone()).collect();
        // [platform-effect] `main` takes no effect parameters — except a
        // *platform* effect, which the host supplies by calling the entry
        // point. Every other effect `main` needs is registered inside it
        // with `use`.
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
                        if provided.contains(&rendered) {
                            continue;
                        }
                        if !keep(self, Some(&ty), &rendered) {
                            continue;
                        }
                        let param = self.unique_name(effect_param_name(&rendered));
                        self.effect_env.push(EffectEntry {
                            ty: Some(ty),
                            rendered: rendered.clone(),
                            expr: param.clone(),
                        });
                        params.push(format!("{param}: {rendered}"));
                    }
                }
                None => {
                    for eff in f.effects.iter().flatten() {
                        if let EffectRef::Effect(r) = eff {
                            if r.name.name == salvo_core::THROW_EFFECT {
                                continue;
                            }
                            let rendered = self.emit_type_ref(r);
                            if provided.contains(&rendered) {
                                continue;
                            }
                            if !keep(self, None, &rendered) {
                                continue;
                            }
                            let param = self.unique_name(effect_param_name(&rendered));
                            self.effect_env.push(EffectEntry {
                                ty: None,
                                rendered: rendered.clone(),
                                expr: param.clone(),
                            });
                            params.push(format!("{param}: {rendered}"));
                        }
                    }
                }
            }
        }
        for p in &f.params {
            if p.implicit {
                continue; // appended below, in the checker's order
            }
            let ty = self.emit_type(&p.ty);
            if p.variadic {
                let elem = self.variadic_elem_type(&p.ty);
                params.push(format!("vararg {}: {elem}", kt_ident(&p.name.name)));
            } else {
                params.push(format!("{}: {ty}", kt_ident(&p.name.name)));
            }
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
            kt_ident(&f.name.name)
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
            format!("{pad}@Suppress(\"UNCHECKED_CAST\")\n")
        } else {
            String::new()
        };
        self.unchecked_cast = saved_cast;
        let mut out = format!(
            "\n{suppress}{pad}{kw}{generics} {name}({}){ret} {{\n",
            params.join(", ")
        );
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
            Some(o) if o.len() > 1 => o.iter().filter(|f| f.body.is_some()).copied().collect(),
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
                if p.variadic {
                    let elem = self.variadic_elem_type(&p.ty);
                    format!("vararg {}: {elem}", kt_ident(&p.name.name))
                } else {
                    format!("{}: {}", kt_ident(&p.name.name), self.emit_type(&p.ty))
                }
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// The element type of a variadic `...args: T[]` parameter.
    fn variadic_elem_type(&mut self, ty: &Type) -> String {
        match ty {
            Type::Array { elem, .. } => self.emit_type(elem),
            other => self.emit_type(other),
        }
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

    /// [kt-suppress-cast] Notes a cast of an erased (`Any?`) payload to
    /// `rendered`, when kotlinc would flag it as unchecked: a bare generic
    /// parameter in scope (erased at run time), or any parameterized type
    /// (whose arguments are). Casts to concrete non-generic types are
    /// checked at run time and draw no warning, so they are not noted.
    fn note_payload_cast(&mut self, rendered: &str) {
        let generic_word = self
            .generics
            .iter()
            .any(|g| word_tokens(rendered).any(|tok| tok == g));
        if generic_word || rendered.contains('<') {
            self.unchecked_cast = true;
        }
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
                    if let EffectRef::Effect(r) = eff {
                        ps.push(self.emit_type_ref(r));
                    }
                }
                ps.extend(params.iter().map(|p| self.emit_type(p)));
                format!("({}) -> {}", ps.join(", "), self.emit_type(ret))
            }
            Type::Tuple { elems, .. } => match elems.len() {
                2 => format!(
                    "Pair<{}, {}>",
                    self.emit_type(&elems[0]),
                    self.emit_type(&elems[1])
                ),
                3 => format!(
                    "Triple<{}, {}, {}>",
                    self.emit_type(&elems[0]),
                    self.emit_type(&elems[1]),
                    self.emit_type(&elems[2])
                ),
                n => {
                    self.error(format!("tuples of size {n} are not supported yet")); // [type-tuple]
                    "Any".to_string()
                }
            },
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
            let arg_strs: Vec<String> = base.args.iter().map(|a| self.emit_type(a)).collect();
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
        let arg_strs: Vec<String> = args.iter().map(|a| self.emit_type(a)).collect();
        self.emit_named_parts(name, &arg_strs)
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
            return format!("{kt}{args}");
        }
        // Structs, generics, effects, and unknown names pass through.
        format!("{name}{args}")
    }

    fn emit_type_args(&mut self, args: &[Type]) -> String {
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
            Ty::Tuple(elems) if elems.len() == 2 => format!(
                "Pair<{}, {}>",
                self.kotlin_ty(&elems[0]),
                self.kotlin_ty(&elems[1])
            ),
            Ty::Tuple(elems) if elems.len() == 3 => format!(
                "Triple<{}, {}, {}>",
                self.kotlin_ty(&elems[0]),
                self.kotlin_ty(&elems[1]),
                self.kotlin_ty(&elems[2])
            ),
            Ty::Var(v) => v.clone(),
            Ty::Any => "Any".to_string(),
            Ty::Nothing => "Nothing".to_string(),
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
            Ty::Tuple(_) => self.emit_ty(ty),
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

    /// Renders a checker type to Kotlin (qualifiers erased, unions as
    /// sealed wrappers).
    fn emit_ty(&mut self, ty: &Ty) -> String {
        match ty {
            Ty::Named { name, args } => {
                let arg_strs: Vec<String> = args.iter().map(|a| self.emit_ty(a)).collect();
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
                match strs.len() {
                    2 => format!("Pair<{}, {}>", strs[0], strs[1]),
                    3 => format!("Triple<{}, {}, {}>", strs[0], strs[1], strs[2]),
                    _ => "Any".to_string(),
                }
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
            Ty::Any | Ty::Unknown => "Any".to_string(),
            Ty::Nothing => "Nothing".to_string(),
        }
    }

    // ================= statements =================

    fn emit_block_stmts(&mut self, block: &Block, indent: usize) -> String {
        let env_depth = self.effect_env.len();
        let out = self.emit_stmts(&block.stmts, indent);
        self.effect_env.truncate(env_depth);
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
            Stmt::Return { value, .. } => match (self.stmt_ctx, value) {
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
            Stmt::Break { value, .. } => {
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
            Stmt::Continue { .. } => format!("{pad}continue\n"),
            Stmt::Use { handler, span } => self.emit_use(handler, *span, indent),
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
    fn emit_use(&mut self, handler: &Expr, span: Span, indent: usize) -> String {
        let pad = "    ".repeat(indent);
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
        // [effect-handler-deps] Dependencies are not written at the `use`
        // site: the compiler supplies them from the enclosing scope, in the
        // handler's declaration order, interleaved with the written
        // arguments exactly as the constructor declares them.
        let dep_tys: Vec<Ty> = self
            .checked
            .use_deps
            .get(&(self.file_idx, span))
            .cloned()
            .unwrap_or_default();
        let mut deps = dep_tys.iter();
        let mut written = written_args.into_iter();
        let mut ctor_args: Vec<String> = Vec::new();
        for p in &decl.params {
            let is_dep = type_base_name(&p.ty)
                .is_some_and(|n| self.symbols.effects.contains_key(n));
            if is_dep {
                match deps.next() {
                    Some(ty) => {
                        let ty = ty.clone();
                        ctor_args.push(self.lookup_effect_handler_by_ty(&ty));
                    }
                    None => {
                        // Unchecked context: fall back to the rendered type.
                        let rendered = self.emit_type(&p.ty);
                        ctor_args.push(self.lookup_effect_handler_by_type(&rendered));
                    }
                }
            } else if p.implicit {
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
        // [effect-handler-generics] A generic handler is constructed *at* a
        // type: Kotlin cannot infer the class's parameter from an empty
        // argument list, so the `use` site's type arguments are written out.
        let type_args = match self.checked.use_handler_args.get(&(self.file_idx, span)) {
            Some(args) if args.iter().all(ty_is_concrete) => {
                let args = args.clone();
                let rendered: Vec<String> =
                    args.iter().map(|a| self.kotlin_ty(a)).collect();
                format!("<{}>", rendered.join(", "))
            }
            _ => String::new(),
        };
        let handler_code = format!(
            "{}{type_args}({})",
            kt_ident(&handler_name),
            ctor_args.join(", ")
        );
        let (effect_ty, rendered) = match self.checked.use_effects.get(&(self.file_idx, span)) {
            Some(ty) if ty_is_concrete(ty) => {
                let ty = ty.clone();
                let rendered = self.kotlin_ty(&ty);
                (Some(ty), rendered)
            }
            _ => (None, self.emit_type(&decl.of)),
        };
        let var = self.unique_name(effect_param_name(&rendered));
        self.effect_env.push(EffectEntry {
            ty: effect_ty,
            rendered: rendered.clone(),
            expr: var.clone(),
        });
        format!("{pad}val {var}: {rendered} = {handler_code}\n")
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
                let c = self.emit_expr(cond);
                out.push_str(&format!("{pad}while ({c}) {{\n"));
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
                let var = if claiming.is_some() {
                    String::new()
                } else {
                    self.for_pattern_var(pattern)
                };
                let iter = if pass.is_none() && claiming.is_none() {
                    self.emit_expr(iterable)
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
                    self.note_payload_cast(&kt);
                    format!(
                        "{pad}        val {} = {subj}{access}.value as {kt}\n",
                        kt_ident(&b.name)
                    )
                } else {
                    self.note_payload_cast(&kt);
                    format!("{pad}        val {} = {subj} as {kt}\n", kt_ident(&b.name))
                };
                out.push_str(&bind);
            }
            // [qual-widen] A `^` branch head peels the arm it matched.
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
                            self.note_payload_cast(&kt);
                            out.push_str(&format!(
                                "{pad}        val {} = {subj}{access}.value as {kt}\n",
                                kt_ident(&id.name)
                            ));
                        }
                        other => {
                            let place = self.emit_expr_raw(other);
                            self.error(format!(
                                "`^` on a projection is not supported yet: widening                                  materializes a local for the branch, which needs a                                  plain variable — bind `{place}` to one first"
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

    /// [qual-widen] Materializes the peel a `^` check performs: the widened
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
                    "`^` on a projection is not supported yet: widening                      materializes a local for the branch, which needs a plain                      variable — bind `{place}` to one first"
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
            self.note_payload_cast(&kt);
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
    fn emit_is_bindings(&mut self, cond: &Expr, indent: usize) -> String {
        let pad = "    ".repeat(indent);
        let mut collected: Vec<(&Expr, &[TypeRef], &Ident, Span)> = Vec::new();
        collect_is_bindings(cond, &mut |subject, check, binding, is_span| {
            collected.push((subject, check, binding, is_span));
        });
        let mut out = String::new();
        for (subject, check, binding, is_span) in collected {
            // The binding reads the payload out of the storage: a narrowed
            // subject place must not unwrap twice [flow-place].
            let subj = self.emit_place_storage(subject);
            let code = match self.is_test_of(is_span).cloned() {
                Some(test) if test.size >= 2 => {
                    let kt = self
                        .ty_of(binding.span)
                        .cloned()
                        .map(|t| self.emit_ty(&t))
                        .unwrap_or_else(|| "Any".to_string());
                    let access = if test.nullable { "?" } else { "" };
                    self.note_payload_cast(&kt);
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
                format!("({place}{access}.value as {kt})")
            }
            Some((PlaceUnwrap::NonNull, _)) => {
                let place = self.emit_place_storage(expr);
                format!("{place}!!")
            }
            None => self.emit_expr_raw(expr),
        }
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

    /// The Kotlin component name of a tuple position
    /// [kt-tuple-component]: `Pair`/`Triple` expose `first`/`second`/
    /// `third`, and Kotlin has no larger tuple type ([type-tuple]), so
    /// anything beyond is unsupported.
    fn tuple_component(index: usize) -> Option<&'static str> {
        match index {
            0 => Some("first"),
            1 => Some("second"),
            2 => Some("third"),
            _ => None,
        }
    }

    /// A place's storage rendering: the read *without* its own narrowing
    /// unwrap [flow-place]. The base keeps its unwraps — a narrowed base
    /// must be unwrapped before its field can be reached — and `is` tests,
    /// `is` bindings and `when` subjects read through this, since they
    /// operate on the declared representation.
    fn emit_place_storage(&mut self, expr: &Expr) -> String {
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
            Coercion::Rewrap { from, to } => {
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
        }
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
    fn emit_predicate_test(&mut self, subj: &str, quals: &[String]) -> String {
        let mut parts: Vec<String> = Vec::new();
        for q in quals {
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
                        if let EffectRef::Effect(r) = eff {
                            let ty = self.emit_type_ref(r);
                            args.push(self.lookup_effect_handler_by_type(&ty));
                        }
                    }
                }
            } else {
                self.error(format!("unknown qualifier `{q}` in predicate check"));
            }
            args.push(subj.to_string());
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
            // Literal suffixes map 1:1 onto Kotlin's [lit-numeric]:
            // `1L` -> `1L` (Long), `1.2f` -> `1.2f` (Float).
            Expr::Int { value, long, .. } => {
                format!("{value}{}", if *long { "L" } else { "" })
            }
            Expr::Float { value, single, .. } => {
                let s = value.to_string();
                let s = if s.contains('.') { s } else { format!("{s}.0") };
                format!("{s}{}", if *single { "f" } else { "" })
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
                // [kt-tuple-component] `Pair`/`Triple` name their elements.
                let code = self.emit_expr(base);
                match Self::tuple_component(*index) {
                    Some(name) => format!("{code}.{name}"),
                    None => {
                        self.error(format!(
                            "tuple element `.{index}` is not supported: Kotlin \
                             tuples map to `Pair`/`Triple` [type-tuple]"
                        ));
                        code
                    }
                }
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
                        format!("linkedMapOf<{}, {}>()", kt, vt)
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
                        format!("linkedSetOf<{}>({})", self.emit_ty(&args[0]), items.join(", "))
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
                format!("linkedMapOf<{}, {}>({})", kt, vt, items.join(", "))
            }
            Expr::Tuple { elems, .. } => {
                let items: Vec<String> = elems.iter().map(|e| self.emit_expr(e)).collect();
                match items.len() {
                    2 => format!("Pair({})", items.join(", ")),
                    3 => format!("Triple({})", items.join(", ")),
                    n => {
                        self.error(format!("tuples of size {n} are not supported yet")); // [type-tuple]
                        format!("arrayOf({})", items.join(", "))
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
            Expr::Binary { op, lhs, rhs, .. } => {
                let prec = bin_prec(*op);
                let l = self.emit_operand_left(lhs, prec);
                let r = self.emit_operand(rhs, prec);
                format!("{l} {} {r}", binary_op(*op))
            }
            // [qual-widen] The dual of `is`: the *same* runtime test (the
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
            Expr::NonNull { operand, .. } => format!("{}!!", self.emit_expr(operand)),
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
        }
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
        let mut out = String::new();
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
        out
    }

    /// A block in value position: all statements plus the trailing
    /// expression as the block's value. `indent` is the column its
    /// statements sit at.
    fn emit_value_block(&mut self, block: &Block, indent: usize) -> String {
        let env_depth = self.effect_env.len();
        let out = self.emit_value_stmts(&block.stmts, indent);
        self.effect_env.truncate(env_depth);
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
        let env_depth = self.effect_env.len();
        let stmts = &body.stmts;
        let n = stmts.len();
        let mut tail: Option<String> = None;
        {
            {
                for (i, stmt) in stmts.iter().enumerate() {
                    if i + 1 == n {
                        if let Stmt::Expr(e) = stmt {
                            if !matches!(self.ty_of(e.span()), Some(Ty::Nothing)) {
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
        self.effect_env.truncate(env_depth);
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
                if let Stmt::Return { value: Some(v), .. } = stmt {
                    let code = self.emit_expr(v);
                    out.push_str(&format!("    {code}\n"));
                    continue;
                }
            }
            if matches!(stmt, Stmt::Return { .. }) {
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
    fn for_pattern_var(&mut self, pattern: &Pattern) -> String {
        match pattern {
            Pattern::Ident(id) => kt_ident(&id.name),
            Pattern::Tuple { elems, .. } => {
                let names: Vec<String> = elems
                    .iter()
                    .map(|p| match p {
                        Pattern::Ident(id) => kt_ident(&id.name),
                        _ => "_".to_string(),
                    })
                    .collect();
                format!("({})", names.join(", "))
            }
            Pattern::Struct { .. } => {
                self.error("struct destructuring in `for` is not supported yet");
                "_".to_string()
            }
        }
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
                    && !matches!(t, Ty::Nothing) =>
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
                let c = self.emit_expr(cond);
                out.push_str(&format!("while ({c}) {{\n"));
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
                        let var = self.for_pattern_var(pattern);
                        let iter = self.emit_expr(iterable);
                        out.push_str(&format!("for ({var} in {iter}) {{\n"));
                    }
                }
                if else_block.is_some() {
                    out.push_str(&format!("{ran} = true\n"));
                }
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
        let env_depth = self.effect_env.len();
        let out = self.emit_loop_body_stmts(&block.stmts, result);
        self.effect_env.truncate(env_depth);
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
    /// `Nothing`-typed tails never fall through (statement as-is);
    /// `None`-typed tails have no Kotlin payload (statement, then `null`).
    fn emit_tail_assign(&mut self, e: &Expr, result: &str) -> String {
        match self.ty_of(e.span()) {
            Some(Ty::Nothing) => self.emit_expr_stmt(e, 0),
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
                        "internal error: `{name}` needs effect `{rendered}`, which the                          function type it is passed as does not declare"
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
        let env_depth = self.effect_env.len();
        let mut param_list: Vec<String> = Vec::new();
        for ty in &effects {
            let rendered = self.kotlin_ty(ty);
            let var = self.unique_name(effect_param_name(&rendered));
            self.effect_env.push(EffectEntry {
                ty: Some(ty.clone()),
                rendered: rendered.clone(),
                expr: var.clone(),
            });
            param_list.push(format!("{var}: {rendered}"));
        }
        param_list.extend(params.iter().map(|p| match &p.ty {
            Some(t) => {
                let ty = self.emit_type(t);
                format!("{}: {ty}", kt_ident(&p.name.name))
            }
            None => kt_ident(&p.name.name),
        }));
        let out = match body {
            LambdaBody::Expr(expr) => {
                format!("{{ {} -> {} }}", param_list.join(", "), self.emit_expr(expr))
            }
            LambdaBody::Block(block) => {
                // Kotlin lambdas return their last expression; a trailing
                // `return X` becomes the value. The lambda body is a
                // `return` barrier: bare returns never target an
                // enclosing `iterator {}` builder.
                let saved_ctx = self.stmt_ctx;
                self.stmt_ctx = StmtCtx::Normal;
                let mut out = format!("{{ {} ->\n", param_list.join(", "));
                out.push_str(&self.emit_lambda_stmts(&block.stmts));
                out.push('}');
                self.stmt_ctx = saved_ctx;
                out
            }
        };
        self.effect_env.truncate(env_depth);
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
        if let Some(effect) = self
            .symbols
            .effect_of_fn
            .get(name)
            .copied()
            .filter(|_| !self.checked.local_calls.contains(&(self.file_idx, span)))
        {
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
            return format!("{handler}.{}({})", kt_ident(name), arg_code.join(", "));
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
            // [col-sorted] [kt-ordered] The sorted constructors build their
            // tree with Salvo's comparator, and the `Sorted List` surface
            // compares with it too [col-sorted-list], so both need its
            // runtime file.
            if matches!(
                f.name.name.as_str(),
                "sorted_set_of"
                    | "mut_sorted_set_of"
                    | "sorted_map_of"
                    | "mut_sorted_map_of"
                    | "sort"
                    | "mut_sort"
                    | "add_sorted"
                    | "binary_search"
            ) {
                self.needs_compare = true;
            }
            if let Some(code) =
                crate::intrinsics::fn_call(&f.name.name, recv, &arg_code, &type_args)
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
            EffectRef::Effect(r) => self
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
            Ty::Var(_) | Ty::Any | Ty::Nothing | Ty::Unknown => false,
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
                    let hs: Vec<String> = effects
                        .iter()
                        .map(|ty| self.lookup_effect_handler_by_ty(ty))
                        .collect();
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
        match crate::intrinsics::fn_call(&decl.name.name, recv, &params, &[]) {
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
                    all.push(self.lookup_effect_handler_by_ty(ty));
                }
            }
            _ => {
                for eff in f.effects.iter().flatten() {
                    if let EffectRef::Effect(r) = eff {
                        if r.name.name == salvo_core::THROW_EFFECT {
                            continue;
                        }
                        let ty = self.emit_type_ref(r);
                        all.push(self.lookup_effect_handler_by_type(&ty));
                    }
                }
            }
        }
        let outer_mints = std::mem::take(&mut self.pending_mints);
        for a in args.iter() {
            all.push(self.emit_expr(a));
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

    /// Resolves the handler expression for a call to an effect member fn.
    fn lookup_effect_handler(&mut self, effect: &str, type_args: &[Type]) -> String {
        if !type_args.is_empty() {
            let full = format!("{effect}{}", self.emit_type_args(type_args));
            return self.lookup_effect_handler_by_type(&full);
        }
        let matches: Vec<String> = self
            .effect_env
            .iter()
            .filter(|e| rendered_base(&e.rendered) == effect)
            .map(|e| e.expr.clone())
            .collect();
        match matches.len() {
            1 => matches[0].clone(),
            0 => {
                self.error(format!(
                    "no handler for effect `{effect}` in scope (declare it in the \
                     function's effect list or `use` a handler)"
                ));
                "TODO()".to_string()
            }
            _ => {
                self.error(format!(
                    "ambiguous effect call: multiple `{effect}` handlers in scope; \
                     specify the type, e.g. `next_random<Int>()`"
                ));
                matches[0].clone()
            }
        }
    }

    /// Resolves a handler by the checker's effect type — the primary,
    /// rendering-drift-immune path. Falls back to the rendered form for
    /// entries that only exist as AST renderings.
    fn lookup_effect_handler_by_ty(&mut self, ty: &Ty) -> String {
        if let Some(e) = self
            .effect_env
            .iter()
            .find(|e| e.ty.as_ref() == Some(ty))
        {
            return e.expr.clone();
        }
        let rendered = self.kotlin_ty(ty);
        self.lookup_effect_handler_by_type(&rendered)
    }

    fn lookup_effect_handler_by_type(&mut self, effect_ty: &str) -> String {
        if let Some(e) = self.effect_env.iter().find(|e| e.rendered == effect_ty) {
            return e.expr.clone();
        }
        // Fall back to a unique same-base-name match (generic callee effects
        // like `Random<T>` against a concrete `Random<Int>` in scope).
        let base = rendered_base(effect_ty);
        let matches: Vec<&EffectEntry> = self
            .effect_env
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
            Stmt::Return { value: Some(v), .. }
            | Stmt::Break { value: Some(v), .. }
            => collect_mutated_expr(v, out),
            Stmt::Use { handler, .. } => collect_mutated_expr(handler, out),
            Stmt::Expr(e) => collect_mutated_expr(e, out),
            _ => {}
        }
    }
}

fn collect_mutated_expr(expr: &Expr, out: &mut HashSet<String>) {
    match expr {
        Expr::IncDec { operand, .. } => {
            if let Expr::Ident(id) = operand.as_ref() {
                out.insert(id.name.clone());
            }
        }
        // [fn-overload-at] Only the dot-notation receiver is an expression.
        Expr::Scoped { base, .. } => {
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
            Stmt::Return { value: Some(v), .. }
            | Stmt::Break { value: Some(v), .. }
            => collect_declared_expr(v, out),
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
fn collect_is_bindings<'a>(
    cond: &'a Expr,
    f: &mut impl FnMut(&'a Expr, &'a [TypeRef], &'a Ident, Span),
) {
    match cond {
        Expr::Is {
            subject,
            check,
            binding: Some(b),
            span,
        } => f(subject, check, b, *span),
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
