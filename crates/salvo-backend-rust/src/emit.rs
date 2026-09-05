//! The Rust code emitter.
//!
//! Mirrors the Kotlin emitter's architecture (checker side tables drive
//! all representation decisions), with the Rust-specific lowerings of
//! BACKEND_SPEC.rust.md: unions as generated enums [rs-union-enums],
//! `T?` as a physical `Option` [rs-option], effects as traits with
//! `&mut dyn` threading [rs-effects], and — the heart of the backend —
//! deduction-driven parameter modes: kept parameters borrow (`&`/`&mut`
//! per `Mut`), omitted parameters move [rs-borrows].
//!
//! rustc is the safety net: anything this emitter gets wrong about
//! ownership fails to *compile*, never silently misbehaves
//! [backend-never-wrong].

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use salvo_core::check::{AbortSite, Checked, Coercion, UnionTest};
use salvo_core::types::Ty;
use salvo_core::{ModulePath, Program, SourceKind, Symbols};
use salvo_syntax::ast::*;
use salvo_syntax::Span;

pub struct EmittedFile {
    /// Path relative to the target dir, e.g. `core/console.rs`.
    pub rel_path: std::path::PathBuf,
    pub content: String,
}

/// Crate-root lint allowances [rs-crate]: the generator does not fight
/// cosmetic lints (every local is `let mut`, Salvo naming is snake/camel
/// mixed, defensive code may be unreachable).
const CRATE_ATTRS: &str = "#![allow(non_snake_case, non_camel_case_types, unused_mut, \
                           unused_parens, unused_imports, dead_code, unreachable_code, \
                           unused_variables, path_statements, unused_must_use, suspicious_double_ref_op)]\n";

/// Emits Rust for every *reachable* module that produces code
/// [mod-used-only], plus the generated `unions.rs` and the crate-root
/// module header [rs-crate].
pub fn emit_program(program: &Program) -> Result<Vec<EmittedFile>, Vec<String>> {
    emit_program_with_entry(program, None)
}

/// [rs-crate] As [`emit_program`], with the crate root chosen explicitly:
/// `entry` names the module whose `main` is the program's entry point
/// (`salvo run --main`). `None` keeps the historical behaviour — the first
/// emitted module that declares one.
pub fn emit_program_with_entry(
    program: &Program,
    entry: Option<&ModulePath>,
) -> Result<Vec<EmittedFile>, Vec<String>> {
    let symbols = Symbols::collect(program);
    let resolution = salvo_core::resolve(program);
    let checked = salvo_core::check_program(program, &resolution, &symbols);
    if !checked.errors.is_empty() {
        // Checker diagnostics are structured [diag-structured]; render
        // them here at the backend boundary.
        return Err(checked
            .errors
            .iter()
            .map(|d| d.render(&program.files))
            .collect());
    }
    let reachable = salvo_core::reachable_modules(program, &resolution);
    let emitted_modules: HashSet<&ModulePath> = program
        .units()
        .filter(|u| {
            u.file.kind == SourceKind::Language
                && reachable.contains(&u.file.module)
                && module_produces_code(u.ast)
        })
        .map(|u| &u.file.module)
        .collect();

    // The Rust module name of every emitted Salvo module [rs-crate]:
    // path parts joined with `_` (`core.console` -> `core_console`).
    let mut mod_names: BTreeMap<ModulePath, String> = BTreeMap::new();
    let mut used: HashSet<String> = HashSet::new();
    used.insert("unions".to_string());
    for module in emitted_modules.iter().copied().collect::<BTreeSet<_>>() {
        let mut name = module
            .0
            .iter()
            .map(|p| p.as_str())
            .collect::<Vec<_>>()
            .join("_");
        while !used.insert(name.clone()) {
            name.push('_');
        }
        mod_names.insert(module.clone(), name);
    }

    // The module declaring `fn main` becomes the crate root [rs-crate]. The
    // driver's choice wins when it made one: with several `main`s, only it
    // knows which the user asked for, and the crate root is the one file
    // that carries the `mod` declarations.
    let declares_main = |u: &salvo_core::program::Unit| {
        u.file.kind == SourceKind::Language
            && emitted_modules.contains(&u.file.module)
            && u.ast.items.iter().any(|item| {
                matches!(item, Item::Fn(f) if f.name.name == "main" && f.body.is_some())
            })
    };
    let root_module: Option<&ModulePath> = program
        .units()
        .find(|u| declares_main(u) && entry.is_some_and(|e| *e == u.file.module))
        .or_else(|| program.units().find(declares_main))
        .map(|u| &u.file.module);

    // [backend-external] Everything external in `core.*` must be covered
    // by the backend's define files (core is implicitly imported).
    let mut errors = check_core_define_coverage(program, &symbols);
    // [decl-explicit] Every define implements exactly one external:
    // the external carries the contract, the define the template.
    errors.extend(salvo_core::check_define_pairing(program, &symbols, "rust"));

    let mut files = Vec::new();
    let mut union_sizes: BTreeSet<usize> = BTreeSet::new();
    // [rs-effect-fusion] The fusion switch is program-wide: a fn's
    // signature cannot depend on which of its callers happens to hold a
    // fusion, so either every effect site fuses or none does.
    let fusion = program_needs_fusion(&symbols);
    for (file_idx, unit) in program.units().enumerate() {
        if unit.file.kind != SourceKind::Language
            || !reachable.contains(&unit.file.module)
            || !module_produces_code(unit.ast)
        {
            continue;
        }
        let generated = generated_imports(
            unit.ast,
            &unit.file.module,
            &resolution.scopes[file_idx],
            &mod_names,
            root_module,
        );
        let mut emitter = Emitter::new(&symbols, &checked, program, file_idx, &unit.file.name);
        emitter.fusion = fusion;
        emitter.generated_imports = generated;
        let content = emitter.emit_module(unit.ast);
        errors.extend(emitter.errors);
        union_sizes.extend(emitter.union_sizes);
        let mut rel_path = std::path::PathBuf::new();
        for part in &unit.file.module.0 {
            rel_path.push(part);
        }
        rel_path.set_extension("rs");
        files.push(EmittedFile { rel_path, content });
    }
    if !union_sizes.is_empty() {
        files.push(EmittedFile {
            rel_path: std::path::PathBuf::from("unions.rs"),
            content: generate_unions_file(&union_sizes),
        });
    }
    // [backend-companion] Companion `.rs` files are copied verbatim and
    // mounted like generated modules.
    let mut companion_mods: Vec<(String, std::path::PathBuf)> = Vec::new();
    for comp in &program.companions {
        if !reachable.contains(&comp.module) {
            continue;
        }
        if files.iter().any(|f| f.rel_path == comp.rel_path) {
            errors.push(format!(
                "companion file `{}` collides with the generated file of module \
                 `{}` (companion modules should only declare `external` items)",
                comp.rel_path.display(),
                comp.module
            ));
            continue;
        }
        let mut name = comp
            .module
            .0
            .iter()
            .map(|p| p.as_str())
            .collect::<Vec<_>>()
            .join("_");
        while !used.insert(name.clone()) {
            name.push('_');
        }
        companion_mods.push((name, comp.rel_path.clone()));
        files.push(EmittedFile {
            rel_path: comp.rel_path.clone(),
            content: comp.content.clone(),
        });
    }

    // Crate-root assembly [rs-crate]: attributes + `#[path]` mod
    // declarations for every other emitted file, prepended to the
    // main-declaring module's file (or a synthetic `lib.rs`).
    if !files.is_empty() {
        let root_rel: std::path::PathBuf = match root_module {
            Some(module) => {
                let mut p = std::path::PathBuf::new();
                for part in &module.0 {
                    p.push(part);
                }
                p.set_extension("rs");
                p
            }
            None => std::path::PathBuf::from("lib.rs"),
        };
        let root_dir = root_rel.parent().map(|p| p.to_path_buf()).unwrap_or_default();
        let mut header = String::from(CRATE_ATTRS);
        let mut mounts: Vec<(String, std::path::PathBuf)> = Vec::new();
        if union_sizes.is_empty() {
            // no unions.rs
        } else {
            mounts.push(("unions".to_string(), std::path::PathBuf::from("unions.rs")));
        }
        for (module, name) in &mod_names {
            if Some(module) == root_module {
                continue;
            }
            let mut p = std::path::PathBuf::new();
            for part in &module.0 {
                p.push(part);
            }
            p.set_extension("rs");
            mounts.push((name.clone(), p));
        }
        mounts.extend(companion_mods);
        for (name, path) in mounts {
            let rel = relative_path(&root_dir, &path);
            header.push_str(&format!("#[path = \"{}\"]\npub mod {};\n", rel, rs_ident(&name)));
        }
        header.push('\n');
        match files.iter_mut().find(|f| f.rel_path == root_rel) {
            Some(root_file) => {
                root_file.content = format!("{header}{}", root_file.content);
            }
            None => {
                // Library compile: a synthetic lib.rs mounts everything.
                files.push(EmittedFile {
                    rel_path: root_rel,
                    content: header,
                });
            }
        }
    }

    if errors.is_empty() {
        Ok(files)
    } else {
        Err(errors)
    }
}

/// The path of `target` relative to the directory `from` (both relative to
/// the output root): `#[path]` attributes resolve relative to the file
/// that contains them [rs-crate].
fn relative_path(from_dir: &std::path::Path, target: &std::path::Path) -> String {
    let ups = from_dir.components().count();
    let mut out = String::new();
    for _ in 0..ups {
        out.push_str("../");
    }
    let target = target
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect::<Vec<_>>()
        .join("/");
    out.push_str(&target);
    out
}

/// The generated `use` items of one file [rs-imports]: a glob per foreign
/// emitted module whose names the file uses, plus alias imports for
/// aliased Salvo imports of Rust-visible items.
fn generated_imports(
    ast: &Module,
    own: &ModulePath,
    scope: &salvo_core::ModuleScope<'_>,
    mod_names: &BTreeMap<ModulePath, String>,
    root_module: Option<&ModulePath>,
) -> BTreeSet<String> {
    let mut imports = BTreeSet::new();
    let module_use = |module: &ModulePath| -> Option<String> {
        if Some(module) == root_module {
            // Root-module items live at the crate root.
            Some("use crate::*;".to_string())
        } else {
            mod_names
                .get(module)
                .map(|m| format!("use crate::{}::*;", rs_ident(m)))
        }
    };
    for name in salvo_core::reach::used_names(ast) {
        for module in scope.name_origins.get(name).into_iter().flatten() {
            if *module != own {
                if let Some(import) = module_use(module) {
                    imports.insert(import);
                }
            }
        }
    }
    for item in &ast.items {
        let Item::Import(imp) = item else { continue };
        let (Some(alias), Some(item_name)) = (&imp.alias, imp.path.last()) else {
            continue;
        };
        let alias_name = alias.name.as_str();
        // Only items with a Rust symbol can be alias-imported: fns with
        // bodies, structs, effects, handlers.
        let has_symbol = scope
            .fns
            .get(alias_name)
            .is_some_and(|entries| entries.iter().any(|e| e.decl.body.is_some()))
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
        let prefix = if Some(*module) == root_module {
            "crate".to_string()
        } else {
            match mod_names.get(*module) {
                Some(m) => format!("crate::{}", rs_ident(m)),
                None => continue,
            }
        };
        imports.insert(format!(
            "use {prefix}::{} as {};",
            rs_ident(&item_name.name),
            rs_ident(alias_name)
        ));
    }
    imports
}

/// [backend-external] Every `external` item in the implicitly imported
/// `core.*` modules must have a define for this backend.
fn check_core_define_coverage(program: &Program, symbols: &Symbols<'_>) -> Vec<String> {
    let mut errors = Vec::new();
    for unit in program.units() {
        if unit.file.kind != SourceKind::Language
            || unit.file.module.0.first().map(String::as_str) != Some("core")
        {
            continue;
        }
        for item in &unit.ast.items {
            match item {
                Item::Fn(f) if f.backing == Some(BackingMod::External) => {
                    let covered = symbols.define_fns.get(f.name.name.as_str()).is_some_and(
                        |defs| defs.iter().any(|d| d.sig.params.len() == f.params.len()),
                    );
                    if !covered {
                        errors.push(format!(
                            "{}: external fn `{}` in core has no rust `define fn`",
                            unit.file.name, f.name.name
                        ));
                    }
                }
                Item::Type(t) if t.backing == Some(BackingMod::External) => {
                    if !symbols.define_types.contains_key(t.name.name.as_str()) {
                        errors.push(format!(
                            "{}: external type `{}` in core has no rust `define type`",
                            unit.file.name, t.name.name
                        ));
                    }
                }
                Item::Handler(h) if h.backing == Some(BackingMod::External) => {
                    if !symbols.define_handlers.contains_key(h.name.name.as_str()) {
                        errors.push(format!(
                            "{}: external handler `{}` in core has no rust `define handler`",
                            unit.file.name, h.name.name
                        ));
                    }
                }
                _ => {}
            }
        }
    }
    errors
}

/// Generates the union enums for every needed size [rs-union-enums]:
/// `pub enum UnionN<T1..TN>` with per-arm accessors and a `Display` impl
/// (so still-union values interpolate directly).
fn generate_unions_file(sizes: &BTreeSet<usize>) -> String {
    let mut out = String::from(
        "// Generated by the Salvo compiler: enums for union types.\n",
    );
    for &n in sizes {
        let params: Vec<String> = (1..=n).map(|i| format!("T{i}")).collect();
        let params = params.join(", ");
        out.push_str(&format!("\n#[derive(Clone, Debug)]\npub enum Union{n}<{params}> {{\n"));
        for i in 1..=n {
            out.push_str(&format!("    U{i}(T{i}),\n"));
        }
        out.push_str("}\n");
        out.push_str(&format!("\nimpl<{params}> Union{n}<{params}> {{\n"));
        for i in 1..=n {
            out.push_str(&format!(
                "    pub fn u{i}(&self) -> &T{i} {{\n        match self {{\n            \
                 Union{n}::U{i}(v) => v,\n            _ => panic!(\"unreachable union arm\"),\n        \
                 }}\n    }}\n"
            ));
        }
        out.push_str("}\n");
        let bounds: Vec<String> = (1..=n)
            .map(|i| format!("T{i}: std::fmt::Display"))
            .collect();
        out.push_str(&format!(
            "\nimpl<{params}> std::fmt::Display for Union{n}<{params}>\nwhere\n    {},\n{{\n    \
             fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {{\n        \
             match self {{\n",
            bounds.join(",\n    ")
        ));
        for i in 1..=n {
            out.push_str(&format!(
                "            Union{n}::U{i}(v) => write!(f, \"{{}}\", v),\n"
            ));
        }
        out.push_str("        }\n    }\n}\n");
    }
    out
}

/// Does this module contain anything that turns into Rust code?
fn module_produces_code(module: &Module) -> bool {
    module.items.iter().any(|item| match item {
        Item::Struct(_) | Item::Effect(_) => true,
        Item::Handler(_) => true,
        Item::Fn(f) => f.body.is_some(),
        Item::Qualifier(q) => q.fns.iter().any(|f| f.body.is_some()),
        _ => false,
    })
}

/// [rs-effect-fusion] Does any handler in the program declare an effect
/// dependency (a constructor parameter of effect type,
/// [effect-handler-deps])? If so the whole program switches to the fusion
/// emission; otherwise effects thread as one `&mut dyn` parameter each
/// [rs-effects] and nothing below this line runs.
fn program_needs_fusion(symbols: &Symbols<'_>) -> bool {
    symbols
        .handlers
        .values()
        .any(|h| !handler_dep_params(h, symbols).is_empty())
}

/// The constructor parameters of `h` that are effect dependencies, in
/// declaration order [effect-handler-deps]. Matches the checker's rule
/// exactly — a *bare* effect name; a qualified or optional one is data, and
/// the checker has already rejected it ([effect-not-data]) — so the fusion
/// never has to render a dependency it cannot name.
fn handler_dep_params<'a>(h: &'a HandlerDecl, symbols: &Symbols<'_>) -> Vec<&'a Param> {
    h.params
        .iter()
        .filter(|p| match &p.ty {
            Type::Named { qualifiers, base } => {
                qualifiers.is_empty() && symbols.effects.contains_key(base.name.name.as_str())
            }
            _ => false,
        })
        .collect()
}

/// Rust reserved words that need escaping as identifiers.
const RUST_KEYWORDS: &[&str] = &[
    "abstract", "as", "async", "await", "become", "box", "break", "const", "continue",
    "do", "dyn", "else", "enum", "extern", "false", "final", "fn", "for", "if", "impl",
    "in", "let", "loop", "macro", "match", "mod", "move", "mut", "override", "priv",
    "pub", "ref", "return", "static", "struct", "trait", "true", "try", "type",
    "typeof", "unsafe", "unsized", "use", "virtual", "where", "while", "yield",
];

/// Keywords that cannot be raw identifiers: rename with a trailing `_`.
const RUST_UNRAW: &[&str] = &["self", "Self", "super", "crate"];

/// Renders a Salvo name as a Rust identifier. Dot-names are *flattened*
/// (`Environment.Id` → `EnvironmentId`) [name-dot]: Rust puts modules and
/// structs in one type namespace, so a nested `mod Environment` beside
/// `struct Environment` is E0428 — concatenation is the only clean
/// rendering, which is why nothing else in scope may claim that spelling
/// (enforced in `resolve`). A dot cannot occur in any other Salvo name
/// that reaches here: dots are invalid in Rust identifiers.
fn rs_ident(name: &str) -> String {
    let flat = if name.contains('.') {
        name.replace('.', "")
    } else {
        name.to_string()
    };
    let name = flat.as_str();
    if RUST_KEYWORDS.contains(&name) {
        format!("r#{name}")
    } else if RUST_UNRAW.contains(&name) {
        format!("{name}_")
    } else {
        flat
    }
}

/// How a name in scope is bound in the emitted Rust [rs-borrows].
#[derive(Clone, Copy, PartialEq)]
enum BindKind {
    /// An owned binding (locals, moved parameters, lambda/loop bindings).
    Owned,
    /// A `&T` parameter.
    Ref,
    /// A `&mut T` parameter.
    RefMut,
    /// A handler constructor param or state field: accessed as `self.x`
    /// inside handler members.
    SelfField,
}

/// How a callee expects one parameter [rs-borrows].
#[derive(Clone, Copy, PartialEq)]
enum ParamMode {
    Owned,
    Ref,
    RefMut,
}

/// A handler registered in the effect environment: the Rust expression
/// used at member-call sites and the one used when threading it as a
/// `&mut dyn` argument differ for locals [rs-effects].
#[derive(Clone)]
struct EffectEntry {
    /// The checker's effect type when known — the primary lookup key,
    /// immune to rendering drift [effect-disambiguation].
    ty: Option<Ty>,
    /// Canonical rendered effect type, e.g. `Random<i32>`.
    key: String,
    /// Variable name (a `use` local or an effect parameter).
    var: String,
    /// True for `use` locals (thread as `&mut var`); false for `&mut dyn`
    /// parameters (thread as `var`, implicit reborrow).
    is_local: bool,
}

struct Emitter<'p> {
    symbols: &'p Symbols<'p>,
    checked: &'p Checked,
    program: &'p Program,
    file_idx: usize,
    file_name: String,
    imports: BTreeSet<String>,
    errors: Vec<String>,
    union_sizes: BTreeSet<usize>,
    effect_env: Vec<EffectEntry>,
    /// How each name in the current fn is bound [rs-borrows].
    bindings: HashMap<String, BindKind>,
    /// Parameter names reassigned in the body (they get a `mut` binder).
    mutated: HashSet<String>,
    generics: HashSet<String>,
    loop_results: Vec<Option<String>>,
    /// Whether each loop result local's join type is optional (assigns
    /// skip the `Some(...)`) [rs-loop-value].
    loop_optional: HashMap<String, bool>,
    loop_id: usize,
    destructure_id: usize,
    /// The current fn has a derived return (`ReadOnly[from: ...]`
    /// [readonly-return]): `return` values render as borrows.
    derived_return_fn: bool,
    taken_names: HashSet<String>,
    generated_imports: BTreeSet<String>,
    /// [rs-effect-fusion] Program-wide: some handler declares an effect
    /// dependency, so every effect site threads one *fused* value.
    fusion: bool,
    /// Items the fusion generates for this file (conjunction traits,
    /// fusion structs and their forwarding impls, dependency adapters),
    /// appended after the module body [rs-effect-fusion].
    generated_items: Vec<String>,
    /// Conjunction traits already generated in this file, by name, with
    /// their supertrait list (to catch a name claimed by two effect sets).
    conj_traits: BTreeMap<String, String>,
    /// Fusion-struct counter for this file.
    fusion_id: usize,
    /// Hoisted-temporary counter for the current fn [rs-effect-fusion].
    hoist_id: usize,
    /// The fn currently being emitted, for readable generated names.
    current_fn: String,
    /// Generic-parameter substitutions while rendering a forwarding impl
    /// for an *instantiated* effect (`Random<T>` members at `Random<i32>`).
    type_subst: HashMap<String, String>,
    /// [rs-defer-splice] Deferred blocks registered so far in the fn being
    /// emitted, innermost/latest last, each already rendered at indent 0.
    /// `defer` has no runtime representation in Rust: the body is spliced
    /// at every exit of its block, so it is rendered once — in the scope
    /// it was written in — and re-indented at each splice site.
    defers: Vec<String>,
    /// Index into `defers` below which entries belong to an enclosing
    /// function: a `return` inside a closure only runs the closure's own
    /// deferred blocks.
    defer_floor: usize,
    /// `defers` length at each enclosing loop's body entry: `break` and
    /// `continue` run the deferred blocks registered inside the loop.
    loop_defer_floors: Vec<usize>,
    /// `return`-value temporary counter [rs-defer-splice].
    defer_id: usize,
    /// [rs-abort-controlflow] The declared abort message type of the fn
    /// being emitted, when it declares `[Abort<M>]`: its Rust return type
    /// is then `ControlFlow<M, T>`, `return v` becomes
    /// `ControlFlow::Continue(v)`, and a propagating call unwraps.
    abort_message: Option<Ty>,
    /// Enclosing `try` delimiters being emitted [rs-try-label], innermost
    /// last: an abort inside one breaks its labelled block instead of
    /// returning.
    try_frames: Vec<TryFrame>,
    /// Labelled-block counter for `try` [rs-try-label].
    try_id: usize,
    /// The indentation of the statement being emitted: expression-position
    /// control transfers ([rs-abort-controlflow]) splice deferred blocks,
    /// which are statements, so they need a column to write at.
    expr_indent: usize,
}

/// One `try` delimiter while its body is emitted [rs-try-label].
struct TryFrame {
    /// The Rust block label (`'try_0`).
    label: String,
    /// `defers.len()` at entry: an abort into this delimiter runs the
    /// deferred blocks registered inside the `try` body, and only those.
    defer_floor: usize,
    /// The outcome union `Ok T | Aborted M`, for wrapping both arms.
    outcome: Option<Ty>,
}

#[derive(Clone, Copy, PartialEq)]
enum StmtCtx {
    Normal,
    /// Inside an eager iterator body [rs-iter-vec]: `yield` pushes into
    /// the `__yielded` local; bare `return` returns it.
    IteratorBody,
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
            effect_env: Vec::new(),
            bindings: HashMap::new(),
            mutated: HashSet::new(),
            generics: HashSet::new(),
            loop_results: Vec::new(),
            loop_optional: HashMap::new(),
            loop_id: 0,
            destructure_id: 0,
            derived_return_fn: false,
            taken_names: HashSet::new(),
            generated_imports: BTreeSet::new(),
            fusion: false,
            generated_items: Vec::new(),
            conj_traits: BTreeMap::new(),
            fusion_id: 0,
            hoist_id: 0,
            current_fn: String::new(),
            type_subst: HashMap::new(),
            defers: Vec::new(),
            defer_floor: 0,
            loop_defer_floors: Vec::new(),
            defer_id: 0,
            abort_message: None,
            try_frames: Vec::new(),
            try_id: 0,
            expr_indent: 0,
        }
    }

    // ================= checker table access =================

    fn ty_of(&self, span: Span) -> Option<&'p Ty> {
        self.checked.expr_ty.get(&(self.file_idx, span))
    }

    fn repr_of(&self, span: Span) -> Option<&'p Ty> {
        self.checked.repr_ty.get(&(self.file_idx, span))
    }

    fn coercion_of(&self, span: Span) -> Option<&'p Coercion> {
        self.checked.coerce.get(&(self.file_idx, span))
    }

    fn is_test_of(&self, span: Span) -> Option<&'p UnionTest> {
        self.checked.is_tests.get(&(self.file_idx, span))
    }

    fn fn_by_key(&self, key: salvo_core::FnKey) -> Option<&'p FnDecl> {
        match self.program.modules.get(key.file)?.items.get(key.item)? {
            Item::Fn(f) => Some(f),
            _ => None,
        }
    }

    /// The stable key of a checker-resolved fn declaration (needed to look
    /// up its deductions [rs-borrows]).
    fn key_of_fn(&self, decl: &FnDecl) -> Option<salvo_core::FnKey> {
        for (file, module) in self.program.modules.iter().enumerate() {
            for (item, it) in module.items.iter().enumerate() {
                if let Item::Fn(f) = it {
                    if std::ptr::eq(f as *const FnDecl, decl as *const FnDecl) {
                        return Some(salvo_core::FnKey { file, item });
                    }
                }
            }
        }
        None
    }

    fn error(&mut self, msg: impl Into<String>) {
        self.errors.push(format!("{}: {}", self.file_name, msg.into()));
    }

    // ================= module =================

    fn emit_module(&mut self, module: &Module) -> String {
        let mut body = String::new();
        for item in &module.items {
            match item {
                Item::Struct(s) => body.push_str(&self.emit_struct(s)),
                // [abort] The abort effect has no handlers — `try` delimits
                // it — so there is nothing to implement: emitting an
                // interface for it would be dead, misleading code.
                Item::Effect(e) if e.name.name == salvo_core::ABORT_EFFECT => {}
                Item::Effect(e) => body.push_str(&self.emit_effect(e)),
                Item::Handler(h) => body.push_str(&self.emit_handler(h)),
                Item::Fn(f) if f.body.is_some() => body.push_str(&self.emit_fn(f)),
                Item::Qualifier(q) => body.push_str(&self.emit_qualifier(q)),
                _ => {}
            }
        }
        // [rs-imports] Generated module imports, template `imports:`
        // lines, and the union enums when this file uses any.
        let mut imports = self.generated_imports.clone();
        imports.extend(self.imports.iter().cloned());
        if !self.union_sizes.is_empty() {
            imports.insert("use crate::unions::*;".to_string());
        }
        let mut out = String::new();
        if !imports.is_empty() {
            for import in &imports {
                out.push_str(import);
                out.push('\n');
            }
        }
        out.push_str(&body);
        // [rs-effect-fusion] Generated items go last: they are plain items
        // and Rust has no ordering requirement, so nothing above needs to
        // know they exist.
        for item in std::mem::take(&mut self.generated_items) {
            out.push_str(&item);
        }
        out
    }

    // ================= declarations =================

    fn emit_struct(&mut self, s: &StructDecl) -> String {
        let saved = self.enter_generics(&s.generics);
        let generics = self.emit_generic_params(&s.generics);
        let mut out = format!(
            "\n#[derive(Clone, Debug)]\npub struct {}{generics} {{\n",
            rs_ident(&s.name.name)
        );
        for field in &s.fields {
            let ty = self.emit_type(&field.ty);
            out.push_str(&format!("    pub {}: {ty},\n", rs_ident(&field.name.name)));
        }
        out.push_str("}\n");
        self.generics = saved;
        out
    }

    fn emit_effect(&mut self, e: &EffectDecl) -> String {
        let saved = self.enter_generics(&e.generics);
        let generics = self.emit_generic_params_unbounded(&e.generics);
        let mut out = format!("\npub trait {}{generics} {{\n", rs_ident(&e.name.name));
        for f in &e.fns {
            // [rs-effects] `dyn` traits cannot have generic methods.
            if !f.generics.is_empty() {
                self.error(format!(
                    "effect member `{}` has its own generic parameters, which the \
                     rust backend cannot dispatch dynamically yet",
                    f.name.name
                ));
                continue;
            }
            let params = self.emit_member_param_list(&f.params);
            let ret = self.emit_return_type(f.return_type.as_ref());
            out.push_str(&format!(
                "    fn {}(&mut self{params}){ret};\n",
                rs_ident(&f.name.name)
            ));
        }
        out.push_str("}\n");
        self.generics = saved;
        out
    }

    /// Effect/handler member parameters follow the default kept rule
    /// [rs-borrows]: scalars by value, everything else `&T`.
    fn emit_member_param_list(&mut self, params: &[Param]) -> String {
        let mut out = String::new();
        for p in params {
            let mode = self.default_param_mode(&p.ty, p.variadic);
            out.push_str(", ");
            out.push_str(&format!(
                "{}: {}",
                rs_ident(&p.name.name),
                self.param_type(&p.ty, p.variadic, mode)
            ));
        }
        out
    }

    fn emit_handler(&mut self, h: &HandlerDecl) -> String {
        if h.backing == Some(BackingMod::Intrinsic) {
            return self.emit_intrinsic_handler(h);
        }
        // [effect-handler-deps] A dependency is a constructor parameter of
        // effect type. The compiler supplies it, so it is neither a field
        // nor a `new` parameter: the member bodies receive it as a fused
        // value from the `use` site's fusion [rs-effect-fusion].
        let deps: Vec<Param> = handler_dep_params(h, self.symbols)
            .into_iter()
            .cloned()
            .collect();
        if !deps.is_empty() && !self.fusion {
            // The gate is *exactly* "some handler declares a dependency",
            // so this is an internal inconsistency, not a language cut.
            self.error(format!(
                "internal: handler `{}` declares effect dependencies but the \
                 fusion emission is off",
                h.name.name
            ));
            return String::new();
        }
        let saved = self.enter_generics(&h.generics);
        let generics = self.emit_generic_params(&h.generics);
        let generic_args = self.emit_generic_args_plain(&h.generics);
        let of = self.emit_type(&h.of);
        let name = rs_ident(&h.name.name);
        let own: Vec<Param> = h
            .params
            .iter()
            .filter(|p| !deps.iter().any(|d| d.name.name == p.name.name))
            .cloned()
            .collect();

        // Struct: own ctor params + state fields.
        let mut out = format!("\npub struct {name}{generics} {{\n");
        for p in &own {
            let ty = self.param_type(&p.ty, p.variadic, ParamMode::Owned);
            out.push_str(&format!("    {}: {ty},\n", rs_ident(&p.name.name)));
        }
        for field in &h.state {
            let ty = self.emit_type(&field.ty);
            out.push_str(&format!("    {}: {ty},\n", rs_ident(&field.name.name)));
        }
        out.push_str("}\n");

        // Constructor: `new` takes ctor params owned (a `use` argument is
        // a move [deduce-infer]) and initializes state from defaults.
        out.push_str(&format!("\nimpl{generics} {name}{generic_args} {{\n"));
        let ctor_params: Vec<String> = own
            .iter()
            .map(|p| {
                format!(
                    "{}: {}",
                    rs_ident(&p.name.name),
                    self.param_type(&p.ty, p.variadic, ParamMode::Owned)
                )
            })
            .collect();
        out.push_str(&format!(
            "    pub fn new({}) -> Self {{\n        Self {{\n",
            ctor_params.join(", ")
        ));
        for p in &own {
            out.push_str(&format!("            {},\n", rs_ident(&p.name.name)));
        }
        for field in &h.state {
            let init = match &field.default {
                Some(expr) => self.emit_expr(expr),
                None => {
                    self.error(format!(
                        "handler state field `{}` has no initializer",
                        field.name.name
                    ));
                    "Default::default()".to_string()
                }
            };
            out.push_str(&format!(
                "            {}: {init},\n",
                rs_ident(&field.name.name)
            ));
        }
        out.push_str("        }\n    }\n}\n");

        if deps.is_empty() {
            // Trait impl with the member bodies.
            out.push_str(&format!("\nimpl{generics} {of} for {name}{generic_args} {{\n"));
            for f in &h.fns {
                out.push_str(&self.emit_fn_inner(f, FnStyle::HandlerMember(h), 1));
            }
            out.push_str("}\n");
        } else {
            out.push_str(&self.emit_dependent_members(h, &deps));
        }
        self.generics = saved;
        out
    }

    /// [rs-effect-fusion] The member bodies of a *dependent* handler. They
    /// cannot live in `impl Effect for H` — the trait signature has no room
    /// for the dependencies — so they move into a generated
    /// `trait __Impl_H` whose methods take the dependencies as one fused
    /// value. `&mut self` is kept, so `self.state` still works.
    ///
    /// A single dependency travels as `&mut dyn D`; two or more need a
    /// Sized generic (`__Fx: D1 + D2`), because a `dyn` value cannot
    /// satisfy a Sized bound and a `?Sized` one could not be narrowed
    /// again further down. The `__Deps_H` adapter is what makes such a
    /// value out of the fusion's single provider field.
    fn emit_dependent_members(&mut self, h: &HandlerDecl, deps: &[Param]) -> String {
        let name = rs_ident(&h.name.name);
        let generics = self.emit_generic_params(&h.generics);
        let generic_args = self.emit_generic_args_plain(&h.generics);
        let mut dep_effects: Vec<(String, Vec<String>)> = Vec::new();
        for p in deps {
            if let Some(parts) = self.named_type_parts(&p.ty) {
                dep_effects.push(parts);
            }
        }
        let trait_name = format!("__Impl_{name}");
        let mut sigs = String::new();
        for f in &h.fns {
            sigs.push_str(&self.emit_fn_inner(f, FnStyle::DepMemberSig(h, deps), 1));
        }
        // The generated trait is *not* generic: the fusion owns the handler
        // behind an opaque `__H` and never derives its type arguments, so it
        // could not supply one. A member signature or dependency that names
        // the handler's own generics is therefore a reported cut, not a
        // mis-emission [backend-never-wrong].
        for g in &h.generics {
            if mentions_ident(&sigs, &g.name) {
                self.error(format!(
                    "handler `{}` declares effect dependencies and uses its own \
                     generic parameters (`{}`) in a member signature — the rust \
                     backend cannot fuse that yet",
                    h.name.name, g.name
                ));
                return String::new();
            }
        }
        let mut out = String::new();
        if dep_effects.len() >= 2 {
            out.push_str(&self.emit_deps_adapter(&name, &dep_effects));
        }
        out.push_str(&format!("\npub trait {trait_name} {{\n{sigs}}}\n"));
        out.push_str(&format!(
            "\nimpl{generics} {trait_name} for {name}{generic_args} {{\n"
        ));
        for f in &h.fns {
            out.push_str(&self.emit_fn_inner(f, FnStyle::DepMember(h, deps), 1));
        }
        out.push_str("}\n");
        out
    }

    /// [rs-effect-fusion] `__Deps_H`: a Sized view over one provider that
    /// implements every dependency of `H`, so a multi-dependency member can
    /// take a single Sized fused value.
    fn emit_deps_adapter(&mut self, handler: &str, deps: &[(String, Vec<String>)]) -> String {
        let name = format!("__Deps_{handler}");
        let mut out = format!(
            "\npub struct {name}<'a, __P: ?Sized> {{\n    __p: &'a mut __P,\n}}\n"
        );
        for (base, args) in deps {
            let bound = trait_path(base, args);
            out.push_str(&self.emit_forward_impl(
                &format!("<'a, __P: {bound} + ?Sized>"),
                &format!("{name}<'a, __P>"),
                base,
                args,
                &Forward::Provider,
            ));
        }
        out
    }

    /// [rs-effect-fusion] `impl Effect for <fusion or adapter>`: every
    /// member forwards, with the effect's own type parameters substituted
    /// by the instance's arguments.
    fn emit_forward_impl(
        &mut self,
        impl_generics: &str,
        self_ty: &str,
        effect_name: &str,
        effect_args: &[String],
        forward: &Forward,
    ) -> String {
        let Some(decl) = self.symbols.effects.get(effect_name).copied() else {
            self.error(format!(
                "internal: no effect `{effect_name}` to forward from a fusion"
            ));
            return String::new();
        };
        let saved_subst = std::mem::take(&mut self.type_subst);
        for (g, arg) in decl.generics.iter().zip(effect_args) {
            self.type_subst.insert(g.name.clone(), arg.clone());
        }
        let saved_generics = self.enter_generics(&decl.generics);
        let path = trait_path(effect_name, effect_args);
        let mut out = format!(
            "\nimpl{impl_generics} {} for {self_ty} {{\n",
            trait_type(effect_name, effect_args)
        );
        for f in &decl.fns {
            if !f.generics.is_empty() {
                continue; // already reported by `emit_effect`
            }
            let params = self.emit_member_param_list(&f.params);
            let ret = self.emit_return_type(f.return_type.as_ref());
            let member = rs_ident(&f.name.name);
            let arg_names: Vec<String> = f
                .params
                .iter()
                .map(|p| rs_ident(&p.name.name))
                .collect();
            let mut body = String::new();
            let recv = match forward {
                Forward::Outer => "&mut *self.__outer".to_string(),
                Forward::Handler => "&mut self.__h".to_string(),
                Forward::Provider => "&mut *self.__p".to_string(),
                Forward::Dependent { trait_name, deps } => {
                    body.push_str("        let Self { __outer, __h } = self;\n");
                    if *deps >= 2 {
                        body.push_str(&format!(
                            "        let mut __deps = __Deps_{}{{ __p: &mut **__outer }};\n",
                            trait_name.trim_start_matches("__Impl_")
                        ));
                    }
                    let dep_arg = if *deps >= 2 {
                        "&mut __deps".to_string()
                    } else {
                        "&mut **__outer".to_string()
                    };
                    let mut all = vec!["__h".to_string(), dep_arg];
                    all.extend(arg_names.iter().cloned());
                    body.push_str(&format!(
                        "        {trait_name}::{member}({})\n",
                        all.join(", ")
                    ));
                    String::new()
                }
            };
            if body.is_empty() {
                let mut all = vec![recv];
                all.extend(arg_names);
                body = format!("        {path}::{member}({})\n", all.join(", "));
            }
            out.push_str(&format!(
                "    fn {member}(&mut self{params}){ret} {{\n{body}    }}\n"
            ));
        }
        out.push_str("}\n");
        self.type_subst = saved_subst;
        self.generics = saved_generics;
        out
    }

    /// The base name and rendered type arguments of a named type
    /// (`Random<Int>` -> `("Random", ["i32"])`).
    fn named_type_parts(&mut self, ty: &Type) -> Option<(String, Vec<String>)> {
        match ty {
            Type::Named { base, .. } => {
                let name = base.name.name.clone();
                let args: Vec<Type> = base.args.clone();
                let rendered = args.iter().map(|a| self.emit_type(a)).collect();
                Some((name, rendered))
            }
            _ => None,
        }
    }

    /// An `intrinsic handler` [backend-intrinsic]: a std handler whose
    /// members this backend implements directly — struct + `new()` + trait
    /// impl. The signatures come from the *effect* it implements (the
    /// handler declaration is bodyless), and the bodies from
    /// [`crate::intrinsics::handler_member`].
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
        let name = rs_ident(&h.name.name);
        let mut out = format!("\npub struct {name} {{\n");
        for p in &h.params {
            let ty = self.param_type(&p.ty, p.variadic, ParamMode::Owned);
            out.push_str(&format!("    {}: {ty},\n", rs_ident(&p.name.name)));
        }
        out.push_str("}\n");
        let ctor_params: Vec<String> = h
            .params
            .iter()
            .map(|p| {
                format!(
                    "{}: {}",
                    rs_ident(&p.name.name),
                    self.param_type(&p.ty, p.variadic, ParamMode::Owned)
                )
            })
            .collect();
        out.push_str(&format!(
            "\nimpl {name} {{\n    pub fn new({}) -> Self {{\n        Self {{",
            ctor_params.join(", ")
        ));
        if h.params.is_empty() {
            out.push_str(" }\n    }\n}\n");
        } else {
            out.push('\n');
            for p in &h.params {
                out.push_str(&format!("            {},\n", rs_ident(&p.name.name)));
            }
            out.push_str("        }\n    }\n}\n");
        }
        out.push_str(&format!("\nimpl {of} for {name} {{\n"));
        for member in &effect.fns {
            let params = self.emit_member_param_list(&member.params);
            let ret = self.emit_return_type(member.return_type.as_ref());
            let arg_names: Vec<String> = member
                .params
                .iter()
                .map(|p| rs_ident(&p.name.name))
                .collect();
            let Some(body) = crate::intrinsics::handler_member(
                &h.name.name,
                &member.name.name,
                &arg_names,
            ) else {
                self.error(format!(
                    "intrinsic handler `{}` has no rust lowering for member `{}`",
                    h.name.name, member.name.name
                ));
                continue;
            };
            out.push_str(&format!(
                "    fn {}(&mut self{params}){ret} {{\n",
                rs_ident(&member.name.name)
            ));
            for line in body.lines() {
                out.push_str(&format!("        {line}\n"));
            }
            out.push_str("    }\n");
        }
        out.push_str("}\n");
        out
    }

    fn emit_fn(&mut self, f: &FnDecl) -> String {
        self.emit_fn_inner(f, FnStyle::TopLevel, 0)
    }

    /// A predicate qualifier's `qualifies` fns become top-level
    /// `pub fn Q_qualifies(...)` [is-qualifies].
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
            out.push_str(&self.emit_fn_inner(&renamed, FnStyle::QualifierFn, 0));
        }
        out
    }

    /// The parameter mode of one top-level-fn parameter [rs-borrows]:
    /// deduction-driven (omitted = moved = by value; kept = borrowed,
    /// `&mut` when the declared type carries `Mut`), with the Copy-scalar
    /// and variadic exceptions.
    fn param_mode(
        &mut self,
        key: Option<salvo_core::FnKey>,
        param: &Param,
    ) -> ParamMode {
        if param.variadic {
            return ParamMode::Owned; // callers assemble a fresh Vec
        }
        if self.is_copy_ast_type(&param.ty) {
            return ParamMode::Owned;
        }
        if is_fn_group(&param.ty) {
            return ParamMode::Owned; // `Once` closures pass by value
        }
        if matches!(param.ty, Type::Fn { .. }) {
            // [fn-contract] Fn values are borrowed: `&mut impl FnMut`.
            return ParamMode::RefMut;
        }
        let kept = key
            .and_then(|k| self.checked.deductions.get(&k))
            .and_then(|ds| ds.iter().find(|d| d.param == param.name.name))
            .map(|d| d.kept)
            .unwrap_or(true); // default kept (lenient, like deduce.rs)
        if !kept {
            return ParamMode::Owned;
        }
        if type_has_mut(&param.ty) {
            ParamMode::RefMut
        } else {
            ParamMode::Ref
        }
    }

    /// The default kept rule for fns outside the deduction tables
    /// (qualifier/effect/handler members) [rs-borrows].
    fn default_param_mode(&mut self, ty: &Type, variadic: bool) -> ParamMode {
        if variadic || self.is_copy_ast_type(ty) || is_fn_group(ty) {
            return ParamMode::Owned;
        }
        if matches!(ty, Type::Fn { .. }) {
            // [fn-contract] Fn values are borrowed: `&mut impl FnMut`.
            return ParamMode::RefMut;
        }
        if type_has_mut(ty) {
            ParamMode::RefMut
        } else {
            ParamMode::Ref
        }
    }

    /// Renders a parameter's Rust type for its mode.
    fn param_type(&mut self, ty: &Type, variadic: bool, mode: ParamMode) -> String {
        let base = if variadic {
            match ty {
                Type::Array { elem, .. } => format!("Vec<{}>", self.emit_type(elem)),
                other => self.emit_type(other),
            }
        } else {
            self.emit_type(ty)
        };
        match mode {
            ParamMode::Owned => base,
            ParamMode::Ref => format!("&{base}"),
            ParamMode::RefMut => format!("&mut {base}"),
        }
    }

    fn emit_fn_inner<'a>(&mut self, f: &FnDecl, style: FnStyle<'a>, indent: usize) -> String {
        let Some(body) = &f.body else {
            return String::new();
        };
        let saved_generics = self.enter_generics(&f.generics);
        let saved_env = std::mem::take(&mut self.effect_env);
        let saved_bindings = std::mem::take(&mut self.bindings);
        let saved_mutated = std::mem::take(&mut self.mutated);
        let saved_derived = self.derived_return_fn;
        collect_mutated(body, &mut self.mutated);
        let saved_taken = std::mem::take(&mut self.taken_names);
        for p in &f.params {
            self.taken_names.insert(p.name.name.clone());
        }
        collect_declared(body, &mut self.taken_names);

        let top_level = matches!(style, FnStyle::TopLevel);
        let is_main = top_level && f.name.name == "main";
        let fn_key = if top_level { self.key_of_fn(f) } else { None };
        // [rs-abort-controlflow] A fn declaring `[Abort<M>]` returns
        // `ControlFlow<M, T>`: the abort *is* the return, so intermediate
        // frames stay silent (no handler, no dispatch, no allocation).
        let saved_abort = std::mem::replace(
            &mut self.abort_message,
            fn_key
                .and_then(|key| self.checked.fn_effects.get(&key))
                .and_then(|effects| effects.iter().find_map(abort_message_of)),
        );
        let saved_try_frames = std::mem::take(&mut self.try_frames);
        let sig_only = matches!(style, FnStyle::DepMemberSig(..));
        let saved_fn = std::mem::replace(&mut self.current_fn, f.name.name.clone());
        let saved_hoist = std::mem::replace(&mut self.hoist_id, 0);

        // Handler members see ctor params and state as `self.` fields —
        // except a dependency, which is not a field at all under the fusion
        // [rs-effect-fusion]: it arrives as the fused parameter.
        let handler_of_style = match style {
            FnStyle::HandlerMember(h) => Some((h, &[] as &[Param])),
            FnStyle::DepMember(h, deps) | FnStyle::DepMemberSig(h, deps) => Some((h, deps)),
            _ => None,
        };
        if let Some((h, deps)) = handler_of_style {
            for p in &h.params {
                if deps.iter().any(|d| d.name.name == p.name.name) {
                    continue;
                }
                self.bindings.insert(p.name.name.clone(), BindKind::SelfField);
            }
            for field in &h.state {
                self.bindings
                    .insert(field.name.name.clone(), BindKind::SelfField);
            }
        }

        let mut generic_parts: Vec<String> = f
            .generics
            .iter()
            .map(|g| format!("{}: Clone", g.name))
            .collect();
        let mut params: Vec<String> = Vec::new();
        if handler_of_style.is_some() {
            params.push("&mut self".to_string());
        }
        // Effect dependencies become leading parameters [rs-effects],
        // sourced from the checker's lowered effect list when available
        // (checker-`Ty` keys; the AST rendering is the unchecked fallback).
        // Under the fusion they collapse into *one* value [rs-effect-fusion].
        if let Some((_, deps)) = handler_of_style.filter(|(_, d)| !d.is_empty()) {
            let mut dep_effects: Vec<(String, Vec<String>)> = Vec::new();
            for p in deps {
                if let Some(parts) = self.named_type_parts(&p.ty) {
                    dep_effects.push(parts);
                }
            }
            let bounds: Vec<String> = dep_effects
                .iter()
                .map(|(b, a)| trait_type(b, a))
                .collect();
            let var = self.unique_name("__fx".to_string());
            let param_ty = if bounds.len() == 1 {
                format!("&mut dyn {}", bounds[0])
            } else {
                generic_parts.push(format!("__Fx: {}", bounds.join(" + ")));
                "&mut __Fx".to_string()
            };
            for bound in bounds {
                self.effect_env.push(EffectEntry {
                    ty: None,
                    key: bound,
                    var: var.clone(),
                    is_local: false,
                });
            }
            self.bindings.insert(var.clone(), BindKind::RefMut);
            params.push(format!("{var}: {param_ty}"));
        } else if !is_main && handler_of_style.is_none() {
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
                        // [rs-abort-controlflow] `Abort` is not a
                        // capability parameter: it changes the *return*
                        // shape instead, so it never becomes a `&mut dyn`.
                        if is_abort_effect_ty(&ty) {
                            continue;
                        }
                        let rendered = self.rust_ty(&ty);
                        effects.push((Some(ty), rendered));
                    }
                }
                None => {
                    for eff in f.effects.iter().flatten() {
                        if let EffectRef::Effect(r) = eff {
                            if r.name.name == salvo_core::ABORT_EFFECT {
                                continue;
                            }
                            let rendered = self.emit_type_ref(r);
                            effects.push((None, rendered));
                        }
                    }
                }
            }
            if self.fusion {
                if !effects.is_empty() {
                    let var = self.unique_name("__fx".to_string());
                    let param_ty = if effects.len() == 1 {
                        format!("&mut dyn {}", effects[0].1)
                    } else {
                        let bounds: Vec<String> =
                            effects.iter().map(|(_, r)| r.clone()).collect();
                        generic_parts.push(format!("__Fx: {}", bounds.join(" + ")));
                        "&mut __Fx".to_string()
                    };
                    for (ty, rendered) in effects {
                        self.effect_env.push(EffectEntry {
                            ty,
                            key: rendered,
                            var: var.clone(),
                            is_local: false,
                        });
                    }
                    self.bindings.insert(var.clone(), BindKind::RefMut);
                    params.push(format!("{var}: {param_ty}"));
                }
            } else {
                for (ty, rendered) in effects {
                    let param = self.unique_name(effect_param_name(&rendered));
                    self.effect_env.push(EffectEntry {
                        ty,
                        key: rendered.clone(),
                        var: param.clone(),
                        is_local: false,
                    });
                    self.bindings.insert(param.clone(), BindKind::RefMut);
                    params.push(format!("{param}: &mut dyn {rendered}"));
                }
            }
        }
        let mut ref_param_count = 0usize;
        let mut derived_param_idx: Option<usize> = None;
        for (i, p) in f.params.iter().enumerate() {
            let mode = match style {
                FnStyle::TopLevel => self.param_mode(fn_key, p),
                _ => self.default_param_mode(&p.ty, p.variadic),
            };
            if matches!(mode, ParamMode::Ref | ParamMode::RefMut) {
                ref_param_count += 1;
            }
            if f.derived_return
                .as_ref()
                .is_some_and(|d| d.name == p.name.name)
            {
                derived_param_idx = Some(i);
            }
            let kind = match mode {
                ParamMode::Owned => BindKind::Owned,
                ParamMode::Ref => BindKind::Ref,
                ParamMode::RefMut => BindKind::RefMut,
            };
            self.bindings.insert(p.name.name.clone(), kind);
            let mut_kw = if mode == ParamMode::Owned && self.mutated.contains(&p.name.name)
            {
                "mut "
            } else {
                ""
            };
            params.push(format!(
                "{mut_kw}{}: {}",
                rs_ident(&p.name.name),
                self.param_type(&p.ty, p.variadic, mode)
            ));
        }

        // [readonly-return] A derived-return fn returns a borrow of its
        // annotated parameter: `&T` (plain) or `Option<&T>` (optional).
        // With a single reference parameter, lifetime elision covers it;
        // with more, a `'a` is generated mechanically and tags the
        // annotated parameter and the return.
        let mut lifetime_generics = String::new();
        self.derived_return_fn = f.derived_return.is_some();
        let ret = if is_main {
            String::new()
        } else if f.derived_return.is_some() {
            let lt = if ref_param_count > 1 {
                lifetime_generics = "'a".to_string();
                if let Some(i) = derived_param_idx {
                    // Retag the annotated parameter's type with 'a.
                    let idx = if params.len() > f.params.len() {
                        // Leading self/effect params shift positions.
                        params.len() - f.params.len() + i
                    } else {
                        i
                    };
                    if let Some(entry) = params.get_mut(idx) {
                        *entry = entry
                            .replacen(": &mut ", ": &'a mut ", 1)
                            .replacen(": &", ": &'a ", 1);
                    }
                }
                "'a "
            } else {
                ""
            };
            match f.return_type.as_ref() {
                Some(Type::Nullable { inner, .. }) => {
                    format!(" -> Option<&{lt}{}>", self.emit_type(inner))
                }
                Some(Type::Named { .. }) | Some(Type::Array { .. })
                | Some(Type::Tuple { .. }) => {
                    let ty = self.emit_return_type(f.return_type.as_ref());
                    format!(" -> &{lt}{}", ty.trim_start_matches(" -> "))
                }
                other => {
                    self.error(format!(
                        "`ReadOnly[from: ...]` returns support only plain and \
                         optional types, not `{other:?}`"
                    ));
                    self.emit_return_type(f.return_type.as_ref())
                }
            }
        } else {
            self.emit_return_type(f.return_type.as_ref())
        };
        // [rs-abort-controlflow] The declared return type becomes
        // `ControlFlow`'s `Continue` payload; the message type is its
        // `Break` payload, which is why propagation is `?`.
        let ret = match self.abort_message.clone() {
            Some(message) => {
                self.imports.insert("use std::ops::ControlFlow;".to_string());
                let msg = self.rust_ty(&message);
                let value = ret.trim_start_matches(" -> ").trim();
                let value = if value.is_empty() { "()" } else { value };
                format!(" -> ControlFlow<{msg}, {value}>")
            }
            None => ret,
        };

        let pad = "    ".repeat(indent);
        let name = if is_main {
            "main".to_string()
        } else if top_level || matches!(style, FnStyle::QualifierFn) {
            self.rust_fn_name(f)
        } else {
            rs_ident(&f.name.name)
        };
        let vis = if handler_of_style.is_some() {
            ""
        } else {
            "pub "
        };
        let mut all_generics: Vec<String> = Vec::new();
        if !lifetime_generics.is_empty() {
            all_generics.push(lifetime_generics.clone());
        }
        all_generics.extend(generic_parts);
        let generics = if all_generics.is_empty() {
            String::new()
        } else {
            format!("<{}>", all_generics.join(", "))
        };
        if sig_only {
            self.generics = saved_generics;
            self.effect_env = saved_env;
            self.bindings = saved_bindings;
            self.mutated = saved_mutated;
            self.derived_return_fn = saved_derived;
            self.taken_names = saved_taken;
            self.current_fn = saved_fn;
            self.hoist_id = saved_hoist;
            return format!(
                "\n{pad}{vis}fn {name}{generics}({}){ret};\n",
                params.join(", ")
            );
        }
        let mut out = format!(
            "\n{pad}{vis}fn {name}{generics}({}){ret} {{\n",
            params.join(", ")
        );
        // [rs-defer-splice] Deferred blocks never cross a fn boundary.
        let saved_defers = std::mem::take(&mut self.defers);
        let saved_defer_floor = std::mem::replace(&mut self.defer_floor, 0);
        let saved_loop_floors = std::mem::take(&mut self.loop_defer_floors);

        // Iterator functions collect eagerly [rs-iter-vec] [fn-iterator].
        if contains_yield(body) {
            let elem = match f.return_type.as_ref() {
                Some(Type::Named { base, .. }) if base.name.name == "Iter" => base
                    .args
                    .first()
                    .map(|t| self.emit_type(t))
                    .unwrap_or_else(|| "()".to_string()),
                _ => {
                    self.error(format!(
                        "fn `{}` uses `yield` but does not return Iter<T>",
                        f.name.name
                    ));
                    "()".to_string()
                }
            };
            out.push_str(&format!(
                "{pad}    let mut __yielded: Vec<{elem}> = Vec::new();\n"
            ));
            out.push_str(&self.emit_block_stmts(body, indent + 1, StmtCtx::IteratorBody));
            let yielded = self.wrap_continue("__yielded".to_string());
            out.push_str(&format!("{pad}    return {yielded};\n"));
        } else {
            out.push_str(&self.emit_block_stmts(body, indent + 1, StmtCtx::Normal));
            // [rs-abort-controlflow] A `None`-returning fn that may abort
            // still has to produce a `ControlFlow` value on the way out.
            if self.abort_message.is_some()
                && self.emit_return_type(f.return_type.as_ref()).is_empty()
            {
                out.push_str(&format!("{pad}    return ControlFlow::Continue(());\n"));
            }
        }
        out.push_str(&format!("{pad}}}\n"));

        self.generics = saved_generics;
        self.effect_env = saved_env;
        self.bindings = saved_bindings;
        self.mutated = saved_mutated;
        self.derived_return_fn = saved_derived;
        self.taken_names = saved_taken;
        self.current_fn = saved_fn;
        self.hoist_id = saved_hoist;
        self.defers = saved_defers;
        self.defer_floor = saved_defer_floor;
        self.loop_defer_floors = saved_loop_floors;
        self.abort_message = saved_abort;
        self.try_frames = saved_try_frames;
        out
    }

    /// The Rust name for a top-level fn [rs-fn-mangling]: Rust has no
    /// overloading, so after the qualifier-suffix rule (shared with
    /// Kotlin, [kt-qual-mangling]) any *still*-colliding overloads with
    /// bodies get positional suffixes (`name__2`, `name__3`, ... in
    /// declaration order).
    fn rust_fn_name(&mut self, decl: &FnDecl) -> String {
        let name = decl.name.name.clone();
        let overloads: Vec<&FnDecl> = match self.symbols.fns.get(name.as_str()) {
            Some(o) if o.len() > 1 => o.iter().filter(|f| f.body.is_some()).copied().collect(),
            _ => return rs_ident(&name),
        };
        if overloads.len() <= 1 {
            return rs_ident(&name);
        }
        // Qualifier-suffix pass: an overload whose erased signature
        // collides with another's gets its qualifier suffix.
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
                return rs_ident(final_name);
            }
        }
        rs_ident(&name)
    }

    /// The erased parameter signature, for overload collision detection.
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

    /// Generic parameters with the blanket `Clone` bound [rs-borrows].
    fn emit_generic_params(&self, generics: &[Ident]) -> String {
        if generics.is_empty() {
            String::new()
        } else {
            format!(
                "<{}>",
                generics
                    .iter()
                    .map(|g| format!("{}: Clone", g.name))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
    }

    /// Generic parameters without bounds (trait declarations: bounds live
    /// on impls, keeping the traits dyn-friendly [rs-effects]).
    fn emit_generic_params_unbounded(&self, generics: &[Ident]) -> String {
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

    fn emit_generic_args_plain(&self, generics: &[Ident]) -> String {
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

    fn emit_return_type(&mut self, ty: Option<&Type>) -> String {
        match ty {
            None => String::new(),
            Some(Type::Named { base, .. }) if base.name.name == "None" => String::new(),
            Some(Type::Named { qualifiers, base }) if base.name.name == "Iter" => {
                // [rs-iter-vec]
                let _ = qualifiers;
                let elem = base
                    .args
                    .first()
                    .map(|t| self.emit_type(t))
                    .unwrap_or_else(|| "()".to_string());
                format!(" -> Vec<{elem}>")
            }
            Some(t) => format!(" -> {}", self.emit_type(t)),
        }
    }

    // ================= types =================

    fn emit_type(&mut self, ty: &Type) -> String {
        match ty {
            Type::Named { qualifiers, base } => self.emit_named_type(qualifiers, base),
            Type::Nullable { inner, .. } => format!("Option<{}>", self.emit_type(inner)),
            Type::Array { elem, .. } => format!("Vec<{}>", self.emit_type(elem)),
            Type::Union { arms, .. } => self.emit_union_type(arms),
            Type::Fn {
                params,
                param_names,
                deductions,
                ret,
                effects,
                ..
            } => {
                // Only meaningful in parameter position [fn-lambda].
                // [fn-contract] Argument types follow the contract:
                // kept non-Copy borrow, kept `Mut` borrows mutably,
                // moved (or Copy) owned. `FnMut` accepts both plain and
                // handler-mutating closures.
                let ps: Vec<String> = params
                    .iter()
                    .enumerate()
                    .map(|(i, p)| {
                        let (kept, is_mut) =
                            ast_fn_param_contract(params, param_names, deductions, i);
                        let base = self.emit_type(p);
                        if kept && is_mut {
                            format!("&mut {base}")
                        } else if kept && !self.is_copy_ast_type(p) {
                            format!("&{base}")
                        } else {
                            base
                        }
                    })
                    .collect();
                let ret = match ret.as_ref() {
                    Type::Named { base, .. } if base.name.name == "None" => String::new(),
                    other => format!(" -> {}", self.emit_type(other)),
                };
                // [fn-effects] The effects a call performs are threaded in,
                // so they are leading `&mut dyn E` parameters of the closure
                // type — nothing is captured, which is what lets an
                // effect-using fn value cross an effectful call.
                let mut all = self.fn_type_effect_params(effects.as_deref());
                all.extend(ps);
                format!("impl FnMut({}){ret}", all.join(", "))
            }
            Type::Tuple { elems, .. } => {
                let parts: Vec<String> = elems.iter().map(|e| self.emit_type(e)).collect();
                format!("({})", parts.join(", "))
            }
            Type::QualifiedGroup {
                qualifiers, base, ..
            } => {
                // [once-fn] A `Once` fn type emits `impl FnOnce`: the
                // checker guarantees at most one call, and rustc's
                // capture inference makes consuming closures `FnOnce`
                // on its own.
                if qualifiers.iter().any(|q| q.name.name == "Once") {
                    if let Type::Fn {
                        params,
                        ret,
                        effects,
                        ..
                    } = base.as_ref()
                    {
                        let ps: Vec<String> =
                            params.iter().map(|p| self.emit_type(p)).collect();
                        let ret = match ret.as_ref() {
                            Type::Named { base, .. } if base.name.name == "None" => {
                                String::new()
                            }
                            other => format!(" -> {}", self.emit_type(other)),
                        };
                        // [fn-effects] as for `FnMut` above.
                        let mut all = self.fn_type_effect_params(effects.as_deref());
                        all.extend(ps);
                        return format!("impl FnOnce({}){ret}", all.join(", "));
                    }
                }
                self.emit_type(base)
            }
        }
    }

    /// [fn-effects] The leading parameters a fn type's effects contribute:
    /// one `&mut dyn Effect` each, in declaration order. A fused value
    /// reborrows into them at the call site, so this stays fusion-agnostic.
    fn fn_type_effect_params(&mut self, effects: Option<&[EffectRef]>) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for eff in effects.into_iter().flatten() {
            if let EffectRef::Effect(r) = eff {
                let rendered = self.emit_type_ref(r);
                out.push(format!("&mut dyn {rendered}"));
            }
        }
        out
    }

    fn emit_named_type(&mut self, qualifiers: &[TypeRef], base: &TypeRef) -> String {
        let name = base.name.name.clone();
        // A `Mut`-qualified type maps through its define's `Mut inline:`
        // template when one exists [type-canbe-mut]; the std rust defines
        // deliberately provide none (mutability is in bindings, not
        // types [rs-borrows]).
        if qualifiers.iter().any(|q| q.name.name == "Mut") {
            let arg_strs: Vec<String> = base.args.iter().map(|a| self.emit_type(a)).collect();
            if let Some(code) = self.expand_mut_type(&name, &arg_strs) {
                return code;
            }
        }
        self.emit_type_ref_named(&name, &base.args)
    }

    fn expand_mut_type(&mut self, name: &str, arg_strs: &[String]) -> Option<String> {
        let def = self.symbols.define_types.get(name).copied()?;
        let mut_inline = def.body.mut_inline.clone()?;
        if let Some(imports) = &def.body.imports {
            self.add_template_imports(imports);
        }
        Some(self.expand_type_template(&mut_inline, &def.generics, arg_strs))
    }

    fn emit_type_ref(&mut self, r: &TypeRef) -> String {
        self.emit_type_ref_named(&r.name.name, &r.args)
    }

    fn emit_type_ref_named(&mut self, name: &str, args: &[Type]) -> String {
        // Type aliases expand structurally [type-alias].
        if let Some(alias) = self.symbols.type_aliases.get(name) {
            let alias = *alias;
            if let Some(target) = &alias.alias {
                if alias.generics.is_empty() {
                    return self.emit_type(target);
                }
                let map: HashMap<&str, &Type> = alias
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

    /// Maps a named type to Rust: intrinsic types [backend-intrinsic],
    /// `define type` templates [backend-define-type], or pass-through.
    fn emit_named_parts(&mut self, name: &str, arg_strs: &[String]) -> String {
        // [rs-effect-fusion] Inside a forwarding impl for an instantiated
        // effect, the effect's own type parameters render as the instance's
        // arguments (`T` -> `i32` for `Random<i32>`).
        if arg_strs.is_empty() {
            if let Some(rendered) = self.type_subst.get(name) {
                return rendered.clone();
            }
        }
        let args = if arg_strs.is_empty() {
            String::new()
        } else {
            format!("<{}>", arg_strs.join(", "))
        };
        if let Some(rs) = crate::intrinsics::type_name(name) {
            return format!("{rs}{args}");
        }
        if name == "Any" {
            // [backend-never-wrong] No Rust mapping yet.
            self.error("the `Any` type is not supported by the rust backend yet");
            return "()".to_string();
        }
        if let Some(def) = self.symbols.define_types.get(name) {
            let def = *def;
            if let Some(imports) = &def.body.imports {
                self.add_template_imports(imports);
            }
            if let Some(inline) = &def.body.inline {
                return self.expand_type_template(inline, &def.generics, arg_strs);
            }
        }
        if self.symbols.external_types.contains_key(name) {
            self.error(format!("external type `{name}` has no rust `define type`"));
        }
        // Structs, generics, effects, and unknown names pass through.
        format!("{}{args}", rs_ident(name))
    }

    /// Unions lower to the enum encoding [rs-union-enums]: `None` arms
    /// become an outer `Option`, a single remaining arm is `Option<T>`,
    /// two or more become `UnionN<...>`.
    fn emit_union_type(&mut self, arms: &[Type]) -> String {
        let mut nullable = false;
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
        let inner = match value_arms.len() {
            0 => "()".to_string(),
            1 => value_arms[0].1.clone(),
            n => {
                self.union_sizes.insert(n);
                let args: Vec<String> = value_arms.into_iter().map(|(_, e)| e).collect();
                format!("Union{n}<{}>", args.join(", "))
            }
        };
        if nullable {
            format!("Option<{inner}>")
        } else {
            inner
        }
    }

    /// Renders a checker `Ty` as Rust (qualifiers erased, unions as the
    /// enum encoding). Must agree with `emit_type` on the same source
    /// type — checker-resolved effect types key the effect environment.
    fn rust_ty(&mut self, ty: &Ty) -> String {
        match ty {
            Ty::Named { name, args } => {
                let arg_strs: Vec<String> = args.iter().map(|a| self.rust_ty(a)).collect();
                self.emit_named_parts(name, &arg_strs)
            }
            Ty::Qualified { quals, base } => {
                if quals.iter().any(|q| q.name == "Mut") {
                    if let Ty::Named { name, args } = base.as_ref() {
                        let name = name.clone();
                        let arg_strs: Vec<String> =
                            args.iter().map(|a| self.rust_ty(a)).collect();
                        if let Some(code) = self.expand_mut_type(&name, &arg_strs) {
                            return code;
                        }
                    }
                }
                self.rust_ty(base)
            }
            Ty::Union(_) => {
                let value_arms = ty.value_arms();
                let inner = match value_arms.len() {
                    0 => "()".to_string(),
                    1 => self.rust_ty(value_arms[0]),
                    n => {
                        self.union_sizes.insert(n);
                        let args: Vec<String> =
                            value_arms.iter().map(|a| self.rust_ty(a)).collect();
                        format!("Union{n}<{}>", args.join(", "))
                    }
                };
                if ty.has_none_arm() {
                    format!("Option<{inner}>")
                } else {
                    inner
                }
            }
            Ty::Tuple(elems) => {
                let strs: Vec<String> = elems.iter().map(|e| self.rust_ty(e)).collect();
                format!("({})", strs.join(", "))
            }
            Ty::Array(elem) => format!("Vec<{}>", self.rust_ty(elem)),
            Ty::Fn { params, ret, .. } => {
                let ps: Vec<String> = params.iter().map(|p| self.rust_ty(p)).collect();
                format!("impl Fn({}) -> {}", ps.join(", "), self.rust_ty(ret))
            }
            Ty::Var(name) => name.clone(),
            Ty::Any | Ty::Unknown => {
                self.error(
                    "a value of unknown/`Any` type reached rust code generation \
                     (the rust backend cannot represent it yet)",
                );
                "()".to_string()
            }
            Ty::Nothing => "()".to_string(),
        }
    }

    /// Whether a checker type is a Copy scalar in Rust [rs-borrows].
    fn is_copy_ty(ty: &Ty) -> bool {
        matches!(
            ty.strip_quals(),
            Ty::Named { name, args }
                if args.is_empty()
                    && matches!(
                        name.as_str(),
                        "Int" | "Long" | "Float" | "Double" | "Bool" | "Char" | "Byte"
                    )
        )
    }

    /// Whether an AST type is a Copy scalar (post alias expansion).
    fn is_copy_ast_type(&mut self, ty: &Type) -> bool {
        let Type::Named { qualifiers, base } = ty else {
            return false;
        };
        if !qualifiers.is_empty() || !base.args.is_empty() {
            return false;
        }
        if let Some(alias) = self.symbols.type_aliases.get(base.name.name.as_str()) {
            if let Some(target) = &(*alias).alias.clone() {
                return self.is_copy_ast_type(target);
            }
        }
        matches!(
            base.name.name.as_str(),
            "Int" | "Long" | "Float" | "Double" | "Bool" | "Char" | "Byte"
        )
    }
}

/// [rs-effect-fusion] Where a generated forwarding impl sends its members.
enum Forward {
    /// The fusion's provider field: `&mut *self.__outer`.
    Outer,
    /// The handler the fusion owns: `&mut self.__h`.
    Handler,
    /// A `__Deps_H` adapter's provider: `&mut *self.__p`.
    Provider,
    /// A dependent handler: destructure `&mut self` into disjoint field
    /// borrows, then call through the handler's `__Impl_H` trait.
    Dependent { trait_name: String, deps: usize },
}

/// An effect as a *type* (impl target, trait bound): `Random<i32>`.
fn trait_type(name: &str, args: &[String]) -> String {
    if args.is_empty() {
        rs_ident(name)
    } else {
        format!("{}<{}>", rs_ident(name), args.join(", "))
    }
}

/// An effect as a *call path* (UFCS member dispatch): `Random::<i32>`.
fn trait_path(name: &str, args: &[String]) -> String {
    if args.is_empty() {
        rs_ident(name)
    } else {
        format!("{}::<{}>", rs_ident(name), args.join(", "))
    }
}

/// The same, from an already rendered effect type (`Random<i32>`) — the
/// unchecked fallback path, where no `Ty` is available.
fn trait_path_of_rendered(rendered: &str) -> String {
    match rendered.split_once('<') {
        Some((base, rest)) => format!("{base}::<{rest}"),
        None => rendered.to_string(),
    }
}

/// Does `code` use `name` as a whole identifier?
fn mentions_ident(code: &str, name: &str) -> bool {
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

fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// What kind of fn is being emitted (parameter-mode and naming rules).
#[derive(Clone, Copy)]
enum FnStyle<'p> {
    TopLevel,
    QualifierFn,
    HandlerMember(&'p HandlerDecl),
    /// [rs-effect-fusion] A *dependent* handler's member, emitted into
    /// `impl __Impl_H for H` with the dependencies as one fused parameter.
    DepMember(&'p HandlerDecl, &'p [Param]),
    /// The same, signature only, for the generated `trait __Impl_H`.
    DepMemberSig(&'p HandlerDecl, &'p [Param]),
}

impl<'p> Emitter<'p> {
    // ================= statements =================

    fn emit_block_stmts(&mut self, block: &Block, indent: usize, ctx: StmtCtx) -> String {
        let mut out = String::new();
        // [rs-effect-fusion] Save the whole environment, not just its
        // depth: a `use` inside the block *rewrites* the outer entries to
        // thread through the inner fusion, which dies with the block.
        let saved_env = self.effect_env.clone();
        // [rs-defer-splice] Deferred blocks registered inside this block
        // run when it ends.
        let defer_floor = self.defers.len();
        for stmt in &block.stmts {
            out.push_str(&self.emit_stmt(stmt, indent, ctx));
        }
        // A block whose last statement exits already ran them there.
        if block_terminates(block) {
            self.defers.truncate(defer_floor);
        } else {
            out.push_str(&self.splice_defers(defer_floor, indent, true));
        }
        self.effect_env = saved_env;
        out
    }

    /// [rs-defer-splice] Renders the deferred blocks registered at or
    /// above `floor`, latest first, re-indented to `indent`. `pop`
    /// discards them (the block they belong to is ending); an early exit
    /// leaves them in place, since the block's own exit runs them too.
    fn splice_defers(&mut self, floor: usize, indent: usize, pop: bool) -> String {
        if self.defers.len() <= floor {
            return String::new();
        }
        let pad = "    ".repeat(indent);
        let mut out = String::new();
        for body in self.defers[floor..].iter().rev() {
            for line in body.lines() {
                if line.trim().is_empty() {
                    out.push('\n');
                } else {
                    out.push_str(&format!("{pad}{line}\n"));
                }
            }
        }
        if pop {
            self.defers.truncate(floor);
        }
        out
    }

    /// [rs-defer-splice] The deferred blocks an early exit runs: every one
    /// registered inside the construct being left.
    fn exit_defers(&mut self, indent: usize, loop_exit: bool) -> String {
        let floor = if loop_exit {
            self.loop_defer_floors
                .last()
                .copied()
                .unwrap_or(self.defer_floor)
        } else {
            self.defer_floor
        };
        self.splice_defers(floor, indent, false)
    }

    fn emit_stmt(&mut self, stmt: &Stmt, indent: usize, ctx: StmtCtx) -> String {
        let pad = "    ".repeat(indent);
        self.expr_indent = indent;
        match stmt {
            Stmt::Let {
                pattern,
                ty,
                value,
                span,
            } => self.emit_let(pattern, ty.as_ref(), value, *span, indent),
            Stmt::Assign { target, value, span } => {
                let t = self.emit_raw(target);
                // A bare-identifier source is a fate link, not a move
                // [fate-link] — unless the assignment is a move-mode bind
                // event [fate-move-mode].
                let v = self.emit_bound_value(value, *span);
                format!("{pad}{t} = {v};\n")
            }
            Stmt::Return { value, .. } => match (ctx, value) {
                (StmtCtx::IteratorBody, None) => {
                    // [rs-iter-vec] short-circuit returns what was
                    // collected so far.
                    let defers = self.exit_defers(indent, false);
                    let yielded = self.wrap_continue("__yielded".to_string());
                    format!("{defers}{pad}return {yielded};\n")
                }
                (StmtCtx::IteratorBody, Some(_)) => {
                    self.error("`return` with a value is not allowed in an iterator function");
                    format!("{pad}return __yielded;\n")
                }
                (_, Some(v)) => {
                    let code = self.emit_return_value(v);
                    // [rs-defer-splice] The value is evaluated before the
                    // deferred blocks run, so it is hoisted into a
                    // temporary when any of them follow it.
                    let defers = self.exit_defers(indent, false);
                    if defers.is_empty() {
                        let code = self.wrap_continue(code);
                        format!("{pad}return {code};\n")
                    } else {
                        let tmp = self.fresh_defer_var();
                        let out = self.wrap_continue(tmp.clone());
                        format!("{pad}let {tmp} = {code};\n{defers}{pad}return {out};\n")
                    }
                }
                (_, None) => {
                    let defers = self.exit_defers(indent, false);
                    let unit = self.wrap_continue("()".to_string());
                    let value = if self.abort_message.is_some() {
                        format!(" {unit}")
                    } else {
                        String::new()
                    };
                    format!("{defers}{pad}return{value};\n")
                }
            },
            Stmt::Break { value, .. } => {
                let target = self.loop_results.last().cloned().flatten();
                // [rs-defer-splice] Leaving the loop runs the deferred
                // blocks registered inside it; the break value is
                // evaluated first.
                match (value, target) {
                    // [while-value] route the value into the enclosing
                    // loop's result local before breaking [rs-loop-value].
                    (Some(v), Some(result)) => {
                        if self.ty_of(v.span()).is_some_and(|t| t.is_none_ty()) {
                            let stmt = self.emit_expr_stmt(v, indent, ctx);
                            let defers = self.exit_defers(indent, true);
                            format!("{stmt}{pad}{result} = None;\n{defers}{pad}break;\n")
                        } else {
                            let code = self.emit_loop_value_assign(v, &result);
                            let defers = self.exit_defers(indent, true);
                            format!("{pad}{code}\n{defers}{pad}break;\n")
                        }
                    }
                    (Some(v), None) => {
                        let stmt = self.emit_expr_stmt(v, indent, ctx);
                        let defers = self.exit_defers(indent, true);
                        format!("{stmt}{defers}{pad}break;\n")
                    }
                    (None, _) => {
                        let defers = self.exit_defers(indent, true);
                        format!("{defers}{pad}break;\n")
                    }
                }
            }
            Stmt::Continue { .. } => {
                let defers = self.exit_defers(indent, true);
                format!("{defers}{pad}continue;\n")
            }
            Stmt::Yield { value, .. } => {
                // [rs-iter-vec]
                let v = self.emit_expr(value);
                format!("{pad}__yielded.push({v});\n")
            }
            Stmt::Use { handler, span } => self.emit_use(handler, *span, indent),
            // [rs-defer-splice] `defer` emits nothing here: the body is
            // rendered now (in the scope it was written in) and spliced at
            // every exit of the enclosing block.
            Stmt::Defer { body, .. } => {
                let code = self.emit_block_stmts(body, 0, ctx);
                self.defers.push(code);
                String::new()
            }
            Stmt::Expr(expr) => self.emit_expr_stmt(expr, indent, ctx),
        }
    }

    /// The code for a `return`'s value expression.
    ///
    /// [readonly-return] Derived-return fns return borrows: the place is
    /// borrowed (bare for already-`&` bindings), `Some(...)`-wrapped when
    /// the checker recorded the optional coercion, and `None` passes
    /// through.
    fn emit_return_value(&mut self, v: &Expr) -> String {
        if self.derived_return_fn && !matches!(v, Expr::Ident(id) if id.name == "None") {
            // A forwarded derived-return call already produces a borrow:
            // pass it through.
            let forwarded = matches!(
                v,
                Expr::Call { span, .. }
                    if self
                        .checked
                        .derived_calls
                        .contains_key(&(self.file_idx, *span))
            );
            if forwarded {
                return self.emit_expr(v);
            }
            let wrap = matches!(
                self.coercion_of(v.span()),
                Some(Coercion::WrapOption { .. })
            );
            if let Some(borrowed) = self.borrow_place(v) {
                return if wrap {
                    format!("Some({borrowed})")
                } else {
                    borrowed
                };
            }
            self.error(format!(
                "a `ReadOnly[from: ...]` return value must be a projection or \
                 alias of the annotated parameter (got `{v:?}`)"
            ));
        }
        self.emit_expr(v)
    }

    /// A fresh local for a value that must be computed before deferred
    /// blocks run [rs-defer-splice].
    fn fresh_defer_var(&mut self) -> String {
        self.defer_id += 1;
        format!("__deferred_value{}", self.defer_id)
    }

    // ================= abort and `try` [rs-abort-controlflow] =================

    /// Wraps a returned value in `ControlFlow::Continue` when the current
    /// fn may abort [rs-abort-controlflow]; a pass-through otherwise.
    fn wrap_continue(&mut self, code: String) -> String {
        match self.abort_message {
            Some(_) => {
                self.imports.insert("use std::ops::ControlFlow;".to_string());
                format!("ControlFlow::Continue({code})")
            }
            None => code,
        }
    }

    /// A fresh labelled-block label for a `try` [rs-try-label].
    fn fresh_try_label(&mut self) -> String {
        self.try_id += 1;
        format!("'try_{}", self.try_id)
    }

    /// The message value at an abort site, wrapped into the target's arm
    /// when several message types meet there [union-arm-identity].
    fn abort_message_value(&mut self, site: &AbortSite, code: String) -> String {
        match site.arm {
            Some(arm) => self.wrap_union_value(&site.target, arm, code),
            None => code,
        }
    }

    /// The Rust code that *takes* the abort at a site: breaking the
    /// enclosing `try`'s labelled block with the aborted arm of its
    /// outcome, or returning `ControlFlow::Break` out of the fn
    /// [rs-abort-controlflow]. Deferred blocks pending inside the
    /// construct being left run first [defer].
    fn abort_transfer(&mut self, site: &AbortSite, message: String, indent: usize) -> String {
        match self.try_frames.last() {
            Some(frame) => {
                let label = frame.label.clone();
                let floor = frame.defer_floor;
                let outcome = frame.outcome.clone();
                let payload = self.abort_message_value(site, message);
                // The aborted arm is arm 1 of `Ok T | Aborted M`.
                let wrapped = match &outcome {
                    Some(ty) => self.wrap_union_value(ty, 1, payload),
                    None => payload,
                };
                let defers = self.splice_defers(floor, indent, false);
                if defers.is_empty() {
                    format!("break {label} {wrapped}")
                } else {
                    let pad = "    ".repeat(indent);
                    format!("{{\n{defers}{pad}break {label} {wrapped};\n{pad}}}")
                }
            }
            None => {
                self.imports.insert("use std::ops::ControlFlow;".to_string());
                let payload = self.abort_message_value(site, message);
                let defers = self.exit_defers(indent, false);
                if defers.is_empty() {
                    format!("return ControlFlow::Break({payload})")
                } else {
                    let pad = "    ".repeat(indent);
                    format!("{{\n{defers}{pad}return ControlFlow::Break({payload});\n{pad}}}")
                }
            }
        }
    }

    /// `abort(message)` [abort]: the control transfer itself, in expression
    /// position (`break`/`return` are expressions in Rust, so a
    /// `Nothing`-typed operand needs no special casing).
    fn emit_abort_call(&mut self, site: &AbortSite, args: &[Expr], indent: usize) -> String {
        let message = match args.first() {
            Some(a) => self.emit_expr(a),
            None => {
                self.error("`abort` needs a message argument");
                "()".to_string()
            }
        };
        self.abort_transfer(site, message, indent)
    }

    /// A call to a fn that may abort [rs-abort-controlflow]: its
    /// `ControlFlow` result is unwrapped here. `?` does it in one character
    /// — but only when the abort would leave *this* fn unchanged: inside a
    /// `try`, when the message needs wrapping into a union, or when
    /// deferred blocks must run first, the propagation is written out as a
    /// `match` (an expression, so no hoisting is needed).
    fn wrap_may_abort_call(&mut self, site: &AbortSite, call: String, indent: usize) -> String {
        self.imports.insert("use std::ops::ControlFlow;".to_string());
        let inside_try = !self.try_frames.is_empty();
        let pending_defers = match self.try_frames.last() {
            Some(frame) => self.defers.len() > frame.defer_floor,
            None => self.defers.len() > self.defer_floor,
        };
        if !inside_try && !pending_defers && site.arm.is_none() {
            return format!("{call}?");
        }
        let transfer = self.abort_transfer(site, "__m".to_string(), indent);
        format!(
            "match {call} {{ ControlFlow::Continue(__v) => __v, \
             ControlFlow::Break(__m) => {transfer} }}"
        )
    }

    /// `try { ... }` [rs-try-label]: a labelled block. No closure, so
    /// nothing is captured — the body reads the fn's effect parameters and
    /// locals directly — and an abort inside it `break`s the label with the
    /// aborted arm of the outcome.
    fn emit_try(&mut self, body: &Block, span: Span, indent: usize) -> String {
        let outcome = self.ty_of(span).cloned();
        if let Some(ty) = &outcome {
            let arms = ty.value_arms().len();
            if arms >= 2 {
                self.union_sizes.insert(arms);
            }
        }
        let label = self.fresh_try_label();
        self.try_frames.push(TryFrame {
            label: label.clone(),
            defer_floor: self.defers.len(),
            outcome: outcome.clone(),
        });
        let inner = self.emit_try_body(body, outcome.as_ref(), indent + 1);
        self.try_frames.pop();
        let pad = "    ".repeat(indent);
        format!("{label}: {{\n{inner}{pad}}}")
    }

    /// The body of a `try`: ordinary statements, with the tail wrapped into
    /// the outcome's `Ok` arm (arm 0). A body that always leaves — every
    /// path aborts or returns — still needs a value for the block, which
    /// the checker made `Ok None` [try].
    fn emit_try_body(&mut self, body: &Block, outcome: Option<&Ty>, indent: usize) -> String {
        let pad = "    ".repeat(indent);
        let mut out = String::new();
        let saved_env = self.effect_env.clone();
        let defer_floor = self.defers.len();
        let mut tail_code: Option<String> = None;
        let n = body.stmts.len();
        for (i, stmt) in body.stmts.iter().enumerate() {
            if i + 1 == n {
                if let Stmt::Expr(e) = stmt {
                    if !matches!(self.ty_of(e.span()), Some(Ty::Nothing)) {
                        tail_code = Some(self.emit_expr(e));
                        continue;
                    }
                }
            }
            out.push_str(&self.emit_stmt(stmt, indent, StmtCtx::Normal));
        }
        let value = tail_code.unwrap_or_else(|| "()".to_string());
        let wrapped = match outcome {
            Some(ty) => self.wrap_union_value(ty, 0, value),
            None => value,
        };
        // Deferred blocks in the body run before the block's value is
        // produced on the normal path [defer].
        let tmp = if self.defers.len() > defer_floor {
            let tmp = self.fresh_defer_var();
            out.push_str(&format!("{pad}let {tmp} = {wrapped};\n"));
            out.push_str(&self.splice_defers(defer_floor, indent, true));
            tmp
        } else {
            wrapped
        };
        out.push_str(&format!("{pad}{tmp}\n"));
        self.effect_env = saved_env;
        out
    }

    fn emit_let(
        &mut self,
        pattern: &Pattern,
        ty: Option<&Type>,
        value: &Expr,
        stmt_span: Span,
        indent: usize,
    ) -> String {
        let pad = "    ".repeat(indent);
        // [rs-borrow-locals] S3: a borrow-mode binding from a pure place
        // emits a real borrow — the local holds `&T` and reads thread
        // through the existing reference-binding rendering (clone in
        // owned positions, bare in borrow positions). Requires: an ident
        // pattern, not a move-mode bind event, the name never reassigned
        // in this fn (one Rust type per local), and a pure place value;
        // everything else keeps the fate-link clone. Decided before the
        // value is emitted so no spurious coercions are recorded.
        if let Pattern::Ident(name) = pattern {
            if !self
                .checked
                .binding_modes
                .contains(&(self.file_idx, stmt_span))
                && !self.mutated.contains(name.name.as_str())
            {
                if let Some(borrow) = self.borrow_value(value) {
                    self.bindings.insert(name.name.clone(), BindKind::Ref);
                    let annot = match ty {
                        Some(t) if !is_fn_type(t) => format!(": &{}", self.emit_type(t)),
                        _ => String::new(),
                    };
                    return format!(
                        "{pad}let mut {}{annot} = {borrow};\n",
                        rs_ident(&name.name)
                    );
                }
            }
        }
        // Bare struct literals pick up the annotated type.
        let value_code = match (value, ty) {
            (
                Expr::StructLit {
                    ty: None,
                    fields,
                    span,
                },
                Some(annot),
            ) => {
                let code = self.emit_struct_lit(Some(annot), fields, *span);
                self.apply_coercion(*span, code)
            }
            _ => self.emit_bound_value(value, stmt_span),
        };
        match pattern {
            Pattern::Ident(name) => {
                self.bindings.insert(name.name.clone(), BindKind::Owned);
                // A fn-type annotation cannot be spelled on a Rust
                // binding (`impl Trait` is invalid there [fn-contract]);
                // the closure's inferred type is already exact, so the
                // annotation is dropped.
                let annot = match ty {
                    Some(t) if !is_fn_type(t) => format!(": {}", self.emit_type(t)),
                    _ => String::new(),
                };
                // Every local is `let mut` [rs-borrows] (Salvo mutability
                // is not locally decidable; `unused_mut` is allowed).
                format!(
                    "{pad}let mut {}{annot} = {value_code};\n",
                    rs_ident(&name.name)
                )
            }
            Pattern::Tuple { elems, .. } => {
                let names: Vec<String> = elems
                    .iter()
                    .map(|p| match p {
                        Pattern::Ident(id) => {
                            self.bindings.insert(id.name.clone(), BindKind::Owned);
                            format!("mut {}", rs_ident(&id.name))
                        }
                        _ => {
                            self.error("nested destructuring patterns are not supported yet");
                            "_".to_string()
                        }
                    })
                    .collect();
                format!("{pad}let ({}) = {value_code};\n", names.join(", "))
            }
            Pattern::Struct { fields, .. } => {
                // [let-destructure] unique per fn; fields move out of the
                // owned temp (partial moves are fine — the temp is dead).
                self.destructure_id += 1;
                let temp = format!("__destructured{}", self.destructure_id);
                let mut out = format!("{pad}let {temp} = {value_code};\n");
                for f in fields {
                    self.bindings.insert(f.binding.name.clone(), BindKind::Owned);
                    out.push_str(&format!(
                        "{pad}let mut {} = {temp}.{};\n",
                        rs_ident(&f.binding.name),
                        rs_ident(&f.field.name)
                    ));
                }
                out
            }
        }
    }

    /// `use Handler(...)` [effect-use] [rs-effects]: instantiate the
    /// handler into a `let mut` local and register it for the rest of the
    /// scope; member calls go through the local, threading takes
    /// `&mut local`.
    fn emit_use(&mut self, handler: &Expr, span: Span, indent: usize) -> String {
        let pad = "    ".repeat(indent);
        let (handler_name, args): (String, Vec<&Expr>) = match handler {
            Expr::Ident(id) => (id.name.clone(), Vec::new()),
            Expr::Call { callee, args, .. } => match callee.as_ref() {
                Expr::Ident(id) => (id.name.clone(), args.iter().collect()),
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
        let (checked_ty, effect_ty) = match self.checked.use_effects.get(&(self.file_idx, span)) {
            Some(ty) if ty_is_concrete(ty) => {
                let ty = ty.clone();
                let rendered = self.rust_ty(&ty);
                (Some(ty), rendered)
            }
            _ => (None, self.emit_type(&decl.of)),
        };
        // Ctor args are owned (a `use` argument is a move [deduce-infer]).
        let arg_code: Vec<String> = args.iter().map(|a| self.emit_expr(a)).collect();
        if self.fusion {
            return self.emit_fusion_use(
                decl,
                &handler_name,
                arg_code,
                checked_ty,
                effect_ty,
                indent,
            );
        }
        let var = self.unique_name(effect_param_name(&effect_ty));
        self.effect_env.push(EffectEntry {
            ty: checked_ty,
            key: effect_ty,
            var: var.clone(),
            is_local: true,
        });
        self.bindings.insert(var.clone(), BindKind::Owned);
        format!(
            "{pad}let mut {var} = {}::new({});\n",
            rs_ident(&handler_name),
            arg_code.join(", ")
        )
    }

    /// [rs-effect-fusion] `use H(...)` under the fusion: one generated
    /// struct owning the handler, chained to whatever provided the effects
    /// already in scope, and implementing all of them.
    ///
    /// Chaining (rather than holding the outer scope's handlers as
    /// individual fields) is what lets an inner scope's fusion coexist with
    /// an outer one that is used again after the inner block: lexical
    /// nesting becomes borrow nesting. It also means a dependency is
    /// *always* reachable through `__outer` — it had to be registered
    /// before its dependent ([effect-handler-deps]), so it can never be a
    /// sibling field.
    fn emit_fusion_use(
        &mut self,
        decl: &HandlerDecl,
        handler_name: &str,
        arg_code: Vec<String>,
        checked_ty: Option<Ty>,
        effect_ty: String,
        indent: usize,
    ) -> String {
        let pad = "    ".repeat(indent);
        let covered: Vec<EffectEntry> = self.effect_env.clone();
        let provider = self.fused_recv();
        let deps: Vec<Param> = handler_dep_params(decl, self.symbols)
            .into_iter()
            .cloned()
            .collect();
        if !deps.is_empty() && covered.is_empty() {
            self.error(format!(
                "internal: handler `{handler_name}` has dependencies but no \
                 effect is in scope at its `use`"
            ));
            self.register_failed_fusion(checked_ty, effect_ty);
            return String::new();
        }
        // The effect this handler adds, preferring the checker's instance.
        let new_effect = match &checked_ty {
            Some(Ty::Named { name, args }) => {
                let name = name.clone();
                let args = args.clone();
                let rendered: Vec<String> =
                    args.iter().map(|a| self.rust_ty(a)).collect();
                (name, rendered)
            }
            _ => match self.named_type_parts(&decl.of) {
                Some(parts) => parts,
                None => {
                    self.error(format!(
                        "handler `{handler_name}` implements a type the rust \
                         backend cannot fuse"
                    ));
                    return String::new();
                }
            },
        };
        // A fusion names its effects in impl headers, so every one of them
        // must be a *concrete* instance. An unresolved type argument (a
        // generic handler whose arguments the checker could not infer, or an
        // effect list generic in the enclosing fn) is reported rather than
        // emitted as an undeclared `T` [backend-never-wrong].
        let mut named: Vec<String> = covered.iter().map(|e| e.key.clone()).collect();
        named.push(trait_type(&new_effect.0, &new_effect.1));
        let unresolved: Option<String> = decl
            .generics
            .iter()
            .map(|g| g.name.clone())
            .chain(self.generics.iter().cloned())
            .find(|g| named.iter().any(|n| mentions_ident(n, g)));
        if let Some(g) = unresolved {
            self.error(format!(
                "`use {handler_name}` fuses effects whose type arguments are still \
                 generic (`{g}`) — the rust backend needs a concrete effect \
                 instance here"
            ));
            self.register_failed_fusion(checked_ty, effect_ty);
            return String::new();
        }
        // The `__outer` field type: the single inherited effect, or a
        // generated conjunction over all of them (one field, because N
        // reborrows of the same provider would alias).
        let outer_ty = match covered.len() {
            0 => None,
            1 => Some(covered[0].key.clone()),
            _ => Some(self.conj_trait(&covered)),
        };
        self.fusion_id += 1;
        let struct_name = format!(
            "__Fx_{}_{}",
            sanitize_ident(&self.current_fn),
            self.fusion_id
        );
        let (impl_generics, self_ty) = match &outer_ty {
            Some(_) => (
                "<'a, __H>".to_string(),
                format!("{struct_name}<'a, __H>"),
            ),
            None => ("<__H>".to_string(), format!("{struct_name}<__H>")),
        };
        let mut item = match &outer_ty {
            Some(ty) => format!(
                "\npub struct {struct_name}<'a, __H> {{\n    __outer: &'a mut dyn {ty},\n    \
                 __h: __H,\n}}\n"
            ),
            None => format!("\npub struct {struct_name}<__H> {{\n    __h: __H,\n}}\n"),
        };
        // Inherited effects forward to the provider.
        for entry in &covered {
            let (base, args) = self.entry_effect_parts(entry);
            item.push_str(&self.emit_forward_impl(
                &impl_generics,
                &self_ty,
                &base,
                &args,
                &Forward::Outer,
            ));
        }
        // The new effect forwards to the owned handler: directly for an
        // independent one, through the handler's `__Impl_H` trait (and
        // disjoint field borrows) for a dependent one.
        let handler_ident = rs_ident(handler_name);
        let (bound, forward) = if deps.is_empty() {
            (
                trait_type(&new_effect.0, &new_effect.1),
                Forward::Handler,
            )
        } else {
            let trait_name = format!("__Impl_{handler_ident}");
            (
                trait_name.clone(),
                Forward::Dependent {
                    trait_name,
                    deps: deps.len(),
                },
            )
        };
        let bounded_generics = match &outer_ty {
            Some(_) => format!("<'a, __H: {bound}>"),
            None => format!("<__H: {bound}>"),
        };
        item.push_str(&self.emit_forward_impl(
            &bounded_generics,
            &self_ty,
            &new_effect.0,
            &new_effect.1,
            &forward,
        ));
        self.generated_items.push(item);

        // The fusion local. Constructor arguments that reach the provider
        // must be evaluated before the struct literal borrows it.
        let (prelude, arg_code) = self.hoist_fused_args(arg_code);
        let var = self.unique_name("__fx".to_string());
        let mut out = String::new();
        for line in &prelude {
            out.push_str(&format!("{pad}{line}\n"));
        }
        let fields = match provider {
            Some(p) => format!("__outer: {p}, __h: {handler_ident}::new({})", arg_code.join(", ")),
            None => format!("__h: {handler_ident}::new({})", arg_code.join(", ")),
        };
        out.push_str(&format!("{pad}let mut {var} = {struct_name} {{ {fields} }};\n"));
        // Every effect in scope now threads through the new fusion.
        for entry in self.effect_env.iter_mut() {
            entry.var = var.clone();
            entry.is_local = true;
        }
        self.effect_env.push(EffectEntry {
            ty: checked_ty,
            key: effect_ty,
            var: var.clone(),
            is_local: true,
        });
        self.bindings.insert(var, BindKind::Owned);
        out
    }

    /// After a reported fusion failure, still register the effect so the
    /// rest of the scope reports its *own* problems rather than a cascade of
    /// "no handler for effect" [type-unknown-lenient].
    fn register_failed_fusion(&mut self, checked_ty: Option<Ty>, effect_ty: String) {
        self.effect_env.push(EffectEntry {
            ty: checked_ty,
            key: effect_ty,
            var: "__fx_unemittable".to_string(),
            is_local: true,
        });
    }

    /// [rs-effect-fusion] The conjunction trait for a set of effects — the    /// only place a fused value needs a *nameable* type. Generated per file
    /// that needs it; the blanket impl makes duplication across files
    /// harmless, since any type satisfying the parts satisfies every copy.
    fn conj_trait(&mut self, covered: &[EffectEntry]) -> String {
        let mut parts: Vec<String> = covered.iter().map(|e| e.key.clone()).collect();
        parts.sort();
        parts.dedup();
        let name = format!(
            "__Conj_{}",
            parts
                .iter()
                .map(|p| sanitize_ident(p))
                .collect::<Vec<_>>()
                .join("_")
        );
        let supers = parts.join(" + ");
        match self.conj_traits.get(&name) {
            Some(existing) if *existing != supers => {
                // Sanitizing `<`/`,` to `_` is not injective (an effect
                // literally named `Random_i32` collides with `Random<i32>`).
                // Vanishingly unlikely, and silently reusing the wrong trait
                // would be wrong code, so it is reported [backend-never-wrong].
                self.error(format!(
                    "the generated conjunction trait `{name}` is claimed by two \
                     different effect sets (`{existing}` and `{supers}`) — rename \
                     one of the effects"
                ));
            }
            Some(_) => {}
            None => {
                self.conj_traits.insert(name.clone(), supers.clone());
                self.generated_items.push(format!(
                    "\npub trait {name}: {supers} {{}}\nimpl<T: {supers} + ?Sized> {name} for T {{}}\n"
                ));
            }
        }
        name
    }

    /// The effect base name and rendered type arguments behind an effect
    /// environment entry, preferring the checker's `Ty` (the rendered key
    /// is the unchecked fallback).
    fn entry_effect_parts(&mut self, entry: &EffectEntry) -> (String, Vec<String>) {
        if let Some(Ty::Named { name, args }) = &entry.ty {
            let name = name.clone();
            let args = args.clone();
            let rendered: Vec<String> = args.iter().map(|a| self.rust_ty(a)).collect();
            return (name, rendered);
        }
        split_rendered_generic(&entry.key)
    }

    fn emit_expr_stmt(&mut self, expr: &Expr, indent: usize, ctx: StmtCtx) -> String {
        let pad = "    ".repeat(indent);
        match expr {
            Expr::If {
                branches,
                else_block,
                ..
            } => self.emit_if_stmt(branches, else_block.as_ref(), indent, ctx),
            // [when-condition] The subject-less `when` is a condition
            // chain, and Rust has no subject-less `match`: it lowers to
            // `if`/`else if`/`else` [rs-when-cond]. The `else` is
            // mandatory, so the chain is total without a filler arm.
            Expr::WhenCond {
                branches,
                else_block,
                ..
            } => self.emit_if_stmt(branches, Some(else_block), indent, ctx),
            Expr::While {
                cond,
                body,
                else_block,
                ..
            } => {
                // Statement position: value discarded; `else` runs only
                // if the loop never did [while-value].
                let ran = else_block
                    .as_ref()
                    .map(|_| format!("{}_ran", self.fresh_loop_var()));
                let inner_pad = "    ".repeat(indent + 1);
                let mut out = String::new();
                if let Some(ran) = &ran {
                    out.push_str(&format!("{pad}let mut {ran} = false;\n"));
                }
                let c = self.emit_expr(cond);
                out.push_str(&format!("{pad}while {} {{\n", cond_code(c)));
                if let Some(ran) = &ran {
                    out.push_str(&format!("{inner_pad}{ran} = true;\n"));
                }
                out.push_str(&self.emit_is_bindings(cond, indent + 1));
                self.loop_results.push(None);
                self.loop_defer_floors.push(self.defers.len());
                out.push_str(&self.emit_block_stmts(body, indent + 1, ctx));
                self.loop_defer_floors.pop();
                self.loop_results.pop();
                out.push_str(&format!("{pad}}}\n"));
                if let (Some(ran), Some(b)) = (&ran, else_block) {
                    out.push_str(&format!("{pad}if !{ran} {{\n"));
                    out.push_str(&self.emit_block_stmts(b, indent + 1, ctx));
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
                // [rs-borrow-locals] S3: a borrow-mode loop (not made
                // by-value by a move-mode event [fate-move-mode])
                // over a pure-place iterable with a plain ident binding
                // iterates *by reference* — no clone of the collection,
                // and the loop variable is a reference binding. Guarded
                // to concrete non-union element types: union/optional
                // elements go through `matches!`/unwrap lowering that
                // expects owned subjects, and keep the clone path.
                let by_value = self
                    .checked
                    .binding_modes
                    .contains(&(self.file_idx, iterable.span()));
                let elem_ok = self
                    .ty_of(iterable.span())
                    .map(|t| match t.strip_quals() {
                        Ty::Named { name, args } if name == "List" || name == "Iter" => {
                            args.first().is_some_and(|e| {
                                matches!(e.strip_quals(), Ty::Named { .. })
                                    && !matches!(e.strip_quals(), Ty::Union(_))
                            })
                        }
                        Ty::Array(e) => {
                            matches!(e.strip_quals(), Ty::Named { .. })
                        }
                        _ => false,
                    })
                    .unwrap_or(false);
                let borrow_iter = if !by_value && elem_ok && matches!(pattern, Pattern::Ident(_))
                {
                    self.borrow_value(iterable)
                } else {
                    None
                };
                let by_ref = borrow_iter.is_some();
                let var = self.for_pattern_var(pattern, by_ref);
                let iter = match borrow_iter {
                    Some(code) => code,
                    None => self.emit_bound_value(iterable, iterable.span()),
                };
                let mut out = String::new();
                if let Some(ran) = &ran {
                    out.push_str(&format!("{pad}let mut {ran} = false;\n"));
                }
                out.push_str(&format!("{pad}for {var} in {iter} {{\n"));
                if let Some(ran) = &ran {
                    out.push_str(&format!("{inner_pad}{ran} = true;\n"));
                }
                self.loop_results.push(None);
                self.loop_defer_floors.push(self.defers.len());
                out.push_str(&self.emit_block_stmts(body, indent + 1, ctx));
                self.loop_defer_floors.pop();
                self.loop_results.pop();
                out.push_str(&format!("{pad}}}\n"));
                if let (Some(ran), Some(b)) = (&ran, else_block) {
                    out.push_str(&format!("{pad}if !{ran} {{\n"));
                    out.push_str(&self.emit_block_stmts(b, indent + 1, ctx));
                    out.push_str(&format!("{pad}}}\n"));
                }
                out
            }
            Expr::When {
                subject, branches, ..
            } => {
                let code = self.emit_when(subject, branches, indent, ctx, false);
                format!("{pad}{code}\n")
            }
            Expr::PostIncrement { operand, .. } => {
                // [rs-postincrement] statement position: plain `+= 1`.
                let t = self.emit_raw(operand);
                format!("{pad}{t} += 1;\n")
            }
            _ => {
                let code = self.emit_expr(expr);
                format!("{pad}{code};\n")
            }
        }
    }

    /// An `if`/`elif`/`else` chain in statement position: branch tails
    /// are emitted as statements.
    fn emit_if_stmt(
        &mut self,
        branches: &[(Expr, Block)],
        else_block: Option<&Block>,
        indent: usize,
        ctx: StmtCtx,
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
            out.push_str(&format!("{kw} {} {{\n", cond_code(c)));
            out.push_str(&self.emit_is_bindings(cond, indent + 1));
            // [qual-widen] A `^` condition peels a wrapper for the branch.
            let (shadows, saved) = self.emit_widen_shadows(cond, indent + 1);
            out.push_str(&shadows);
            out.push_str(&self.emit_block_stmts(block, indent + 1, ctx));
            self.restore_bindings(saved);
            out.push_str(&format!("{pad}}} "));
        }
        if let Some(block) = else_block {
            out.push_str("else {\n");
            out.push_str(&self.emit_block_stmts(block, indent + 1, ctx));
            out.push_str(&format!("{pad}}}\n"));
        } else {
            out.pop();
            out.push('\n');
        }
        out
    }

    /// For a condition containing `x is T name`, emits the binding
    /// declarations at the top of the matched branch.
    fn emit_is_bindings(&mut self, cond: &Expr, indent: usize) -> String {
        let pad = "    ".repeat(indent);
        let mut collected: Vec<(&Expr, &[TypeRef], &Ident, Span)> = Vec::new();
        collect_is_bindings(cond, &mut |subject, check, binding, is_span| {
            collected.push((subject, check, binding, is_span));
        });
        let mut out = String::new();
        for (subject, _check, binding, is_span) in collected {
            self.bindings.insert(binding.name.clone(), BindKind::Owned);
            let code = if self
                .checked
                .predicate_tests
                .contains_key(&(self.file_idx, is_span))
            {
                // Predicate checks refine only qualifiers (erased): the
                // binding is an owned copy of the subject.
                self.emit_expr(subject)
            } else {
                let target = self.ty_of(binding.span).cloned();
                self.emit_narrowed_read(subject, target.as_ref(), self.is_test_of(is_span).cloned())
            };
            out.push_str(&format!(
                "{pad}let mut {} = {code};\n",
                rs_ident(&binding.name)
            ));
        }
        out
    }

    /// [qual-widen] Materializes the peel a `^` check performs: the widened
    /// value is bound to a **shadowing** local for the branch, so reads of
    /// the subject — and any nested `when` on it — see it at the widened
    /// type. Returns the emitted lines and the binding kinds to restore when
    /// the branch ends (the shadow is owned; the outer binding may not be).
    fn emit_widen_shadows(
        &mut self,
        cond: &Expr,
        indent: usize,
    ) -> (String, Vec<(String, Option<BindKind>)>) {
        let pad = "    ".repeat(indent);
        let mut sites: Vec<(&Expr, Span)> = Vec::new();
        collect_widen_checks(cond, &mut |subject, span| sites.push((subject, span)));
        let mut out = String::new();
        let mut saved: Vec<(String, Option<BindKind>)> = Vec::new();
        for (subject, span) in sites {
            let Some(target) = self.checked.widen_targets.get(&(self.file_idx, span)).cloned()
            else {
                // No wrapper was peeled (the qualifier was statically
                // present and is erased): nothing to materialize.
                continue;
            };
            let Expr::Ident(id) = subject else {
                let place = self.emit_raw(subject);
                self.error(format!(
                    "`^` on a projection is not supported yet: widening                      materializes a local for the branch, which needs a plain                      variable — bind `{place}` to one first"
                ));
                continue;
            };
            let test = self.is_test_of(span).cloned();
            let code = self.emit_narrowed_read(subject, Some(&target), test);
            let displaced = self.bindings.insert(id.name.clone(), BindKind::Owned);
            saved.push((id.name.clone(), displaced));
            out.push_str(&format!("{pad}let mut {} = {code};\n", rs_ident(&id.name)));
        }
        (out, saved)
    }

    /// Restores binding kinds displaced by a `^` shadow [qual-widen].
    fn restore_bindings(&mut self, saved: Vec<(String, Option<BindKind>)>) {
        for (name, kind) in saved.into_iter().rev() {
            match kind {
                Some(k) => self.bindings.insert(name, k),
                None => self.bindings.remove(&name),
            };
        }
    }

    /// Reads the narrowed payload out of a subject for an `is` binding:
    /// enum-arm accessor for wrapper unions, `Option` unwrap for `T?`
    /// representations [rs-union-enums] [rs-option].
    fn emit_narrowed_read(
        &mut self,
        subject: &Expr,
        target: Option<&Ty>,
        test: Option<UnionTest>,
    ) -> String {
        // The payload comes out of the *storage*: if the subject place is
        // itself narrowed (a nested check on the same place), its own
        // unwrap must not be applied on top [flow-place].
        let subj = self.place_storage(subject);
        match test {
            Some(test) if test.size >= 2 => {
                // Wrapper union: unwrap the (unique) matched arm.
                let arm = test.arms.first().copied().unwrap_or(0);
                let access = if test.nullable {
                    format!("{subj}.as_ref().unwrap()")
                } else {
                    subj
                };
                let clone = match target {
                    Some(t) if Self::is_copy_ty(t) => "*",
                    _ => "",
                };
                if clone == "*" {
                    format!("*{access}.u{}()", arm + 1)
                } else {
                    format!("{access}.u{}().clone()", arm + 1)
                }
            }
            _ => {
                // `T?` representation (or a checked field): Option unwrap.
                match target {
                    Some(t) if Self::is_copy_ty(t) => format!("{subj}.unwrap()"),
                    _ => format!("{subj}.as_ref().unwrap().clone()"),
                }
            }
        }
    }
}

impl<'p> Emitter<'p> {
    // ================= expressions =================

    /// Emits an expression as an *owned* value with any checker-recorded
    /// coercion applied [rs-borrows].
    fn emit_expr(&mut self, expr: &Expr) -> String {
        let code = self.emit_owned(expr);
        self.apply_coercion(expr.span(), code)
    }

    /// The narrowing unwrap for identifier uses [rs-union-enums]
    /// [rs-option]: `None` when the ident is not narrowed. The result is
    /// owned.
    fn ident_unwrap(&mut self, id: &Ident) -> Option<String> {
        if !self.checked.repr_ty.contains_key(&(self.file_idx, id.span)) {
            return None;
        }
        let storage = self.binding_place(&id.name);
        self.narrow_unwrap(id.span, storage)
    }

    /// The narrowing unwrap for a *projection place* read [flow-place]:
    /// `h.field` narrowed by `h.field is T` reads its payload out of the
    /// declared representation, exactly as a narrowed identifier does. The
    /// storage code keeps the base's own unwraps (a narrowed base must be
    /// unwrapped before its field can be reached) but not this place's.
    fn place_unwrap(&mut self, expr: &Expr) -> Option<String> {
        if !matches!(expr, Expr::Field { .. } | Expr::TupleIndex { .. }) {
            return None;
        }
        if !self
            .checked
            .repr_ty
            .contains_key(&(self.file_idx, expr.span()))
        {
            return None;
        }
        let storage = self.place_storage(expr);
        self.narrow_unwrap(expr.span(), storage)
    }

    /// A place's storage rendering: the read *without* its own narrowing
    /// unwrap [flow-place]. What an `is` test, an `is` binding, and the
    /// narrowing unwrap itself must read.
    fn place_storage(&mut self, expr: &Expr) -> String {
        match expr {
            Expr::Field { base, field, span } => {
                let base_code = self.emit_place(base);
                let code = format!("{base_code}.{}", rs_ident(&field.name));
                if let Some(cast_ty) = self.checked.field_casts.get(&(self.file_idx, *span)) {
                    let cast_ty = cast_ty.clone();
                    return if Self::is_copy_ty(&cast_ty) {
                        format!("{code}.unwrap()")
                    } else {
                        format!("{code}.as_ref().unwrap().clone()")
                    };
                }
                code
            }
            Expr::TupleIndex { base, index, .. } => {
                // [rs-tuple-index] Rust tuples index natively.
                let base_code = self.emit_place(base);
                format!("{base_code}.{index}")
            }
            Expr::Ident(id) if id.name != "None" => self.binding_place(&id.name),
            other => self.emit_place(other),
        }
    }

    /// Reads a narrowed value out of the declared representation at
    /// `span`, given the code for its storage [rs-union-enums]
    /// [rs-option]. Shared by identifier and projection-place reads.
    fn narrow_unwrap(&mut self, span: Span, storage: String) -> Option<String> {
        let (repr, logical) = (self.repr_of(span)?, self.ty_of(span)?);
        let name = storage;
        // Wrapper union narrowed to a single non-`None` arm.
        if repr.is_wrapper_union()
            && !matches!(logical, Ty::Union(_))
            && !logical.is_none_ty()
        {
            let logical = logical.clone();
            let arm = repr
                .value_arms()
                .iter()
                .position(|a| **a == logical)
                .or_else(|| {
                    // Fall back to subtype matching (e.g. extra quals).
                    let arms: Vec<&Ty> = repr.value_arms();
                    arms.iter()
                        .position(|a| salvo_core::is_subtype(&logical, a))
                });
            let Some(arm) = arm else {
                self.error(format!(
                    "narrowed type `{logical}` matches no arm of `{repr}`"
                ));
                return Some(name);
            };
            let access = if repr.has_none_arm() {
                format!("{name}.as_ref().unwrap()")
            } else {
                name
            };
            return Some(if Self::is_copy_ty(&logical) {
                format!("*{access}.u{}()", arm + 1)
            } else {
                format!("{access}.u{}().clone()", arm + 1)
            });
        }
        // `T?` representation narrowed to its value arm: unlike Kotlin,
        // Rust must unwrap the Option physically [rs-option].
        if matches!(repr, Ty::Union(_))
            && repr.has_none_arm()
            && !repr.is_wrapper_union()
            && !logical.has_none_arm()
            && !logical.is_none_ty()
            && !matches!(logical, Ty::Union(_))
        {
            let logical = logical.clone();
            return Some(if Self::is_copy_ty(&logical) {
                format!("{name}.unwrap()")
            } else {
                format!("{name}.as_ref().unwrap().clone()")
            });
        }
        None
    }

    /// The place expression for a bound name: `self.x` for handler
    /// fields, the (possibly escaped) name otherwise.
    fn binding_place(&self, name: &str) -> String {
        match self.bindings.get(name) {
            Some(BindKind::SelfField) => format!("self.{}", rs_ident(name)),
            _ => rs_ident(name),
        }
    }

    /// Whether an expression is a *pure place*: a bare identifier of a
    /// bound local/parameter or a field/index chain over one, with no
    /// coercion, narrowing unwrap, or field cast anywhere — i.e. it can
    /// be borrowed directly [rs-borrow-locals].
    fn place_is_pure(&self, expr: &Expr) -> bool {
        if self.coercion_of(expr.span()).is_some() {
            return false;
        }
        match expr {
            Expr::Ident(id) => {
                id.name != "None"
                    && !self
                        .checked
                        .repr_ty
                        .contains_key(&(self.file_idx, id.span))
                    && self.bindings.contains_key(id.name.as_str())
            }
            Expr::Field { base, span, .. } => {
                !self
                    .checked
                    .field_casts
                    .contains_key(&(self.file_idx, *span))
                    // A narrowed projection read is an owned temporary
                    // (the payload out of the wrapper), not a place
                    // [flow-place].
                    && !self.checked.repr_ty.contains_key(&(self.file_idx, *span))
                    && self.place_is_pure(base)
            }
            Expr::TupleIndex { base, span, .. } => {
                // A narrowed element read is an owned temporary
                // [flow-place].
                !self.checked.repr_ty.contains_key(&(self.file_idx, *span))
                    && self.place_is_pure(base)
            }
            Expr::Index { base, .. } => self.place_is_pure(base),
            _ => false,
        }
    }

    /// A borrow of a pure place [rs-borrow-locals]: `&place` for owned
    /// roots, the bare name for an already-`&` binding, a reborrow for
    /// `&mut` roots. `None` when the value is not a pure place (the
    /// caller falls back to the clone path).
    fn borrow_value(&mut self, value: &Expr) -> Option<String> {
        if !self.place_is_pure(value) {
            return None;
        }
        match value {
            Expr::Ident(id) => match self.bindings.get(id.name.as_str()) {
                Some(BindKind::Ref) => Some(self.binding_place(&id.name)),
                Some(BindKind::RefMut) => {
                    Some(format!("&*{}", self.binding_place(&id.name)))
                }
                Some(BindKind::Owned) | Some(BindKind::SelfField) => {
                    Some(format!("&{}", self.binding_place(&id.name)))
                }
                None => None,
            },
            Expr::Field { .. } | Expr::TupleIndex { .. } | Expr::Index { .. } => {
                Some(format!("&{}", self.emit_place(value)))
            }
            _ => None,
        }
    }

    /// Like `borrow_value`, but tolerant of a coercion at the *top*
    /// span (the caller handles it — e.g. the derived-return `Some`
    /// wrap [readonly-return]); nested places must still be pure.
    fn borrow_place(&mut self, value: &Expr) -> Option<String> {
        match value {
            Expr::Ident(id) => {
                if id.name == "None"
                    || self
                        .checked
                        .repr_ty
                        .contains_key(&(self.file_idx, id.span))
                {
                    return None;
                }
                match self.bindings.get(id.name.as_str()) {
                    Some(BindKind::Ref) => Some(self.binding_place(&id.name)),
                    Some(BindKind::RefMut) => {
                        Some(format!("&*{}", self.binding_place(&id.name)))
                    }
                    Some(BindKind::Owned) | Some(BindKind::SelfField) => {
                        Some(format!("&{}", self.binding_place(&id.name)))
                    }
                    None => None,
                }
            }
            Expr::Field { base, span, .. } => {
                if self
                    .checked
                    .field_casts
                    .contains_key(&(self.file_idx, *span))
                    // A narrowed read is an owned temporary [flow-place].
                    || self.checked.repr_ty.contains_key(&(self.file_idx, *span))
                    || !self.place_is_pure(base)
                {
                    return None;
                }
                Some(format!("&{}", self.emit_place(value)))
            }
            Expr::TupleIndex { base, span, .. } => {
                if self.checked.repr_ty.contains_key(&(self.file_idx, *span))
                    || !self.place_is_pure(base)
                {
                    return None;
                }
                Some(format!("&{}", self.emit_place(value)))
            }
            Expr::Index { base, .. } => {
                if !self.place_is_pure(base) {
                    return None;
                }
                Some(format!("&{}", self.emit_place(value)))
            }
            _ => None,
        }
    }

    /// The value of a `let`/assignment bind event: a *move-mode* binding
    /// [fate-move-mode] took ownership — the checker consumed the
    /// ancestors at the binding — so the place is rendered directly (a
    /// real move, partial for projections, no clone). Borrow-mode falls
    /// back to the fate-link clone (`emit_linked_value`).
    fn emit_bound_value(&mut self, value: &Expr, bind_span: Span) -> String {
        if self
            .checked
            .binding_modes
            .contains(&(self.file_idx, bind_span))
        {
            let code = self.emit_place(value);
            return self.apply_coercion(value.span(), code);
        }
        self.emit_linked_value(value)
    }

    /// The value of a `let`/assignment/`for` whose source is a bare
    /// identifier: the binding fate-links to the source and the checker
    /// keeps both usable (reads never consume [fate-link]), so an owned
    /// non-Copy local must be *cloned*, not moved. S1 emission is
    /// clone-by-default; borrow emission is roadmap stage S3.
    fn emit_linked_value(&mut self, value: &Expr) -> String {
        if let Expr::Ident(id) = value {
            if self.ident_unwrap(id).is_none()
                && matches!(self.bindings.get(id.name.as_str()), Some(BindKind::Owned))
                && !self.ty_of(id.span).is_some_and(|t| Self::is_copy_ty(t))
            {
                let code = format!("{}.clone()", self.binding_place(&id.name));
                return self.apply_coercion(value.span(), code);
            }
        }
        self.emit_expr(value)
    }

    /// Owned rendering [rs-borrows]: reference-bound identifiers and
    /// field/index reads of non-Copy types clone; owned locals move;
    /// constructed values pass through.
    fn emit_owned(&mut self, expr: &Expr) -> String {
        match expr {
            Expr::Ident(id) => {
                if id.name == "None" {
                    return "None".to_string();
                }
                if let Some(unwrapped) = self.ident_unwrap(id) {
                    return unwrapped;
                }
                let place = self.binding_place(&id.name);
                let copy = self
                    .ty_of(id.span)
                    .is_some_and(|t| Self::is_copy_ty(t));
                match self.bindings.get(id.name.as_str()) {
                    Some(BindKind::Ref) | Some(BindKind::RefMut) if !copy => {
                        format!("{place}.clone()")
                    }
                    Some(BindKind::Ref) | Some(BindKind::RefMut) => format!("*{place}"),
                    Some(BindKind::SelfField) if !copy => format!("{place}.clone()"),
                    _ => place,
                }
            }
            Expr::Field { .. } | Expr::TupleIndex { .. } | Expr::Index { .. } => {
                // A narrowed projection read unwraps to an owned payload
                // [flow-place].
                if let Some(unwrapped) = self.place_unwrap(expr) {
                    return unwrapped;
                }
                let place = self.emit_place(expr);
                // A projection in a moved position whose roots the
                // checker consumed renders as the raw place — a real
                // partial move, no clone [fate-move-mode].
                if self
                    .checked
                    .moved_projections
                    .contains(&(self.file_idx, expr.span()))
                {
                    return place;
                }
                // Field-cast reads are already owned (clone inside).
                if let Expr::Field { span, .. } = expr {
                    if self.checked.field_casts.contains_key(&(self.file_idx, *span)) {
                        return place;
                    }
                }
                let needs_clone = self
                    .ty_of(expr.span())
                    .map(|t| !Self::is_copy_ty(t) && ty_is_concrete(t))
                    .unwrap_or(false);
                if needs_clone {
                    format!("{place}.clone()")
                } else {
                    place
                }
            }
            other => self.emit_raw_like(other),
        }
    }

    /// Place rendering: raw chains for idents/fields/indexes (narrowing
    /// unwraps and field casts still apply — their results are owned
    /// temporaries, which method calls and borrows accept).
    fn emit_place(&mut self, expr: &Expr) -> String {
        match expr {
            Expr::Ident(id) => {
                if id.name == "None" {
                    return "None".to_string();
                }
                if let Some(unwrapped) = self.ident_unwrap(id) {
                    return unwrapped;
                }
                self.binding_place(&id.name)
            }
            Expr::Field { .. } | Expr::TupleIndex { .. } => {
                // A narrowed projection place reads its payload out of the
                // declared representation [flow-place]; otherwise it is
                // the plain storage read (which applies a field cast
                // [qual-field-override]).
                if let Some(unwrapped) = self.place_unwrap(expr) {
                    return unwrapped;
                }
                self.place_storage(expr)
            }
            Expr::Index { base, index, .. } => {
                let base_code = self.emit_place(base);
                let idx = self.emit_owned(index);
                format!("{base_code}[({idx}) as usize]")
            }
            other => self.emit_raw_like(other),
        }
    }

    /// Raw rendering for assignment targets and `++` operands: no
    /// narrowing unwraps, no casts, no clones.
    fn emit_raw(&mut self, expr: &Expr) -> String {
        match expr {
            Expr::Ident(id) => {
                if id.name == "None" {
                    "None".to_string()
                } else {
                    self.binding_place(&id.name)
                }
            }
            Expr::Field { base, field, .. } => {
                format!("{}.{}", self.emit_raw(base), rs_ident(&field.name))
            }
            Expr::TupleIndex { base, index, .. } => {
                format!("{}.{index}", self.emit_raw(base))
            }
            Expr::Index { base, index, .. } => {
                let idx = self.emit_owned(index);
                format!("{}[({idx}) as usize]", self.emit_raw(base))
            }
            other => self.emit_raw_like(other),
        }
    }

    /// Every non-place expression form (shared tail of the owned/place/raw
    /// renderings — these all produce owned values).
    fn emit_raw_like(&mut self, expr: &Expr) -> String {
        match expr {
            // Literal suffixes emit explicit Rust types [lit-numeric]
            // [type-basic]: `1L` -> `1i64`, `1.2f` -> `1.2f32`;
            // unsuffixed literals stay bare for inference.
            Expr::Int { value, long, .. } => {
                format!("{value}{}", if *long { "i64" } else { "" })
            }
            Expr::Float { value, single, .. } => {
                let s = value.to_string();
                let s = if s.contains('.') { s } else { format!("{s}.0") };
                format!("{s}{}", if *single { "f32" } else { "" })
            }
            Expr::Bool { value, .. } => value.to_string(),
            Expr::Char { value, .. } => format!("'{}'", escape_char(*value)),
            Expr::Str { parts, .. } => self.emit_string(parts),
            Expr::Ident(_)
            | Expr::Field { .. }
            | Expr::TupleIndex { .. }
            | Expr::Index { .. } => {
                unreachable!("place expressions are handled by the callers")
            }
            Expr::Call {
                callee,
                type_args,
                args,
                span,
            } => self.emit_call(callee, type_args, args, *span),
            Expr::ArrayLit { elems, .. } => {
                let items: Vec<String> = elems.iter().map(|e| self.emit_expr(e)).collect();
                format!("vec![{}]", items.join(", "))
            }
            Expr::ArrayInit {
                elem_type,
                size,
                init,
                ..
            } => {
                let elem = self.emit_type_ref(elem_type);
                let size = self.emit_owned(size);
                let lambda = self.emit_owned(init);
                format!("(0..({size})).map({lambda}).collect::<Vec<{elem}>>()")
            }
            Expr::Tuple { elems, .. } => {
                let items: Vec<String> = elems.iter().map(|e| self.emit_expr(e)).collect();
                format!("({})", items.join(", "))
            }
            Expr::StructLit { ty, fields, span } => {
                self.emit_struct_lit(ty.as_ref(), fields, *span)
            }
            Expr::Unary { op, operand, .. } => {
                let inner = self.emit_operand(operand, 6);
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
            Expr::Is {
                subject,
                check,
                span,
                ..
            } => self.emit_is_check(subject, check, *span),
            // [qual-widen] The same runtime test as `is`, or `true` when the
            // qualifiers are statically present: qualifiers are erased, so
            // widening is a typing act, not a run-time one.
            Expr::Widen { subject, span, .. } => match self.is_test_of(*span).cloned() {
                Some(test) => {
                    let subj = self.place_storage(subject);
                    self.emit_union_test(&subj, &test)
                }
                None => "true".to_string(),
            },
            Expr::NonNull { operand, .. } => {
                format!("{}.unwrap()", self.emit_owned(operand))
            }
            Expr::PostIncrement { operand, .. } => {
                // [rs-postincrement] value position.
                let t = self.emit_raw(operand);
                format!("({{ let __t = {t}; {t} += 1; __t }})")
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
            Expr::Try { body, span } => {
                let indent = self.expr_indent;
                self.emit_try(body, *span, indent)
            }
            Expr::Spread { operand, .. } => self.emit_owned(operand),
            Expr::While { .. } | Expr::For { .. } => self.emit_loop_value(expr),
            Expr::When {
                subject, branches, ..
            } => {
                let indent = self.expr_indent;
                self.emit_when(subject, branches, indent, StmtCtx::Normal, true)
            }
            // [when-condition] / [rs-when-cond]
            Expr::WhenCond {
                branches,
                else_block,
                ..
            } => {
                let indent = self.expr_indent;
                self.emit_if_expr(branches, Some(else_block), indent)
            }
            Expr::Error { .. } => "todo!()".to_string(),
        }
    }

    /// A binary/unary operand, parenthesized when its precedence is lower
    /// than the parent operator's (Salvo's parser preserved the grouping;
    /// flat re-rendering must not change it).
    fn emit_operand(&mut self, expr: &Expr, parent_prec: u8) -> String {
        let code = self.emit_expr(expr);
        match expr {
            Expr::Binary { op, .. } if bin_prec(*op) <= parent_prec => format!("({code})"),
            Expr::Is { .. } | Expr::Widen { .. } | Expr::Lambda { .. } => {
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
            Expr::Is { .. } | Expr::Widen { .. } | Expr::Lambda { .. } => {
                format!("({code})")
            }
            _ => code,
        }
    }

    /// An `is` check in expression position: the checker's tables decide
    /// the lowering [rs-union-enums]; for unchecked `T?` subjects
    /// (struct fields) a null test still works [rs-option]; anything
    /// else is a codegen error [backend-never-wrong].
    fn emit_is_check(&mut self, subject: &Expr, check: &[TypeRef], span: Span) -> String {
        if let Some(test) = self.is_test_of(span).cloned() {
            // The test reads the storage, never the narrowed payload
            // [flow-place].
            let subj = self.place_storage(subject);
            return self.emit_union_test(&subj, &test);
        }
        if let Some(quals) = self
            .checked
            .predicate_tests
            .get(&(self.file_idx, span))
            .cloned()
        {
            return self.emit_predicate_test(subject, &quals);
        }
        // Fallbacks for unchecked subjects with an optional repr (e.g.
        // `person.surname is Str`).
        let subj = self.place_storage(subject);
        if check.len() == 1 && check[0].name.name == "None" {
            return format!("{subj}.is_none()");
        }
        if self
            .ty_of(subject.span())
            .is_some_and(|t| matches!(t, Ty::Union(_)) && t.has_none_arm() && !t.is_wrapper_union())
        {
            return format!("{subj}.is_some()");
        }
        self.error(
            "`is` check could not be lowered for the rust backend \
             (subject is not a checked union or optional)",
        );
        "false".to_string()
    }

    /// The runtime test for an `is` check against a union representation
    /// [rs-union-enums] [union-arm-identity].
    fn emit_union_test(&mut self, subj: &str, test: &UnionTest) -> String {
        if test.match_none {
            return format!("{subj}.is_none()");
        }
        if test.size == 1 {
            // `T?` representation.
            return if test.arms.is_empty() {
                "false".to_string()
            } else {
                format!("{subj}.is_some()")
            };
        }
        self.union_sizes.insert(test.size);
        if test.arms.is_empty() {
            return "false".to_string();
        }
        if test.arms.len() == test.size {
            return if test.nullable {
                format!("{subj}.is_some()")
            } else {
                "true".to_string()
            };
        }
        let pats: Vec<String> = test
            .arms
            .iter()
            .map(|i| {
                let pat = format!("Union{}::U{}(_)", test.size, i + 1);
                if test.nullable {
                    format!("Some({pat})")
                } else {
                    pat
                }
            })
            .collect();
        format!("matches!({subj}, {})", pats.join(" | "))
    }

    /// A predicate-qualifier `is` check [is-qualifies]: `Q_qualifies`
    /// calls with the subject borrowed (default kept rule [rs-borrows])
    /// and `qualifies` effects threaded [is-qualifies-effects].
    fn emit_predicate_test(&mut self, subject: &Expr, quals: &[String]) -> String {
        // The `qualifies` parameter is `&T` for non-Copy subjects.
        let subject_ty = self.ty_of(subject.span()).cloned();
        let copy = subject_ty.as_ref().is_some_and(Self::is_copy_ty);
        let subj = if copy {
            self.emit_owned(subject)
        } else {
            self.borrowed_arg(subject)
        };
        let mut parts: Vec<String> = Vec::new();
        for q in quals {
            let mut args: Vec<String> = Vec::new();
            if let Some(decl) = self.symbols.qualifiers.get(q.as_str()).copied() {
                if let Some(f) = decl.fns.iter().find(|f| f.name.name == "qualifies") {
                    for eff in f.effects.iter().flatten() {
                        if let EffectRef::Effect(r) = eff {
                            let ty = self.emit_type_ref(r);
                            args.push(self.thread_effect_by_key(&ty));
                        }
                    }
                }
            } else {
                self.error(format!("unknown qualifier `{q}` in predicate check"));
            }
            // [rs-effect-fusion] One fused value covers the whole set.
            if self.fusion {
                args.dedup();
                args.truncate(1);
            }
            args.push(subj.clone());
            parts.push(format!("{q}_qualifies({})", args.join(", ")));
        }
        if parts.len() == 1 {
            parts.into_iter().next().unwrap()
        } else {
            format!("({})", parts.join(" && "))
        }
    }

    /// An adapter closure for a named fn passed as a fn value
    /// [fn-contract]: `&mut |a0, a1| name(a0, &a1)` — adapter parameters
    /// arrive per the fn-type contract (owned for moved/Copy, references
    /// for kept), and the body forwards per the declaration's actual
    /// modes. `None` when the name is not a known fn.
    fn named_fn_adapter(
        &mut self,
        name: &str,
        span: Span,
        expected: &Type,
    ) -> Option<String> {
        let key = self.checked.fn_refs.get(&(self.file_idx, span)).copied();
        let decl = match key.and_then(|k| self.fn_by_key(k)) {
            Some(d) => d,
            None => self.symbols.fns.get(name).and_then(|v| v.first()).copied()?,
        };
        let Type::Fn {
            params: exp_params,
            param_names,
            deductions,
            effects: exp_effects,
            ..
        } = expected
        else {
            return None;
        };
        let rust_name = self.rust_fn_name(decl);
        // [fn-effects] The adapter takes the *expected* effect parameters —
        // the caller passes them whatever this fn does with them — and
        // forwards the ones the declaration actually needs. A named fn with
        // fewer effects simply ignores the rest (the variance rule).
        let mut effect_params: Vec<String> = Vec::new();
        let mut effect_args: Vec<(String, String)> = Vec::new();
        for eff in exp_effects.iter().flatten() {
            if let EffectRef::Effect(r) = eff {
                let rendered = self.emit_type_ref(r);
                let var = format!("__fx{}", effect_params.len());
                effect_params.push(format!("{var}: &mut dyn {rendered}"));
                effect_args.push((rendered, var));
            }
        }
        let mut forwarded_effects: Vec<String> = Vec::new();
        for eff in decl.effects.iter().flatten() {
            if let EffectRef::Effect(r) = eff {
                if r.name.name == salvo_core::ABORT_EFFECT {
                    continue;
                }
                let rendered = self.emit_type_ref(r);
                match effect_args.iter().find(|(key, _)| *key == rendered) {
                    Some((_, var)) => forwarded_effects.push(format!("&mut *{var}")),
                    None => {
                        // The checker's fits rule makes this unreachable:
                        // a fn value may only perform effects its type
                        // declares [fn-effects] [backend-never-wrong].
                        self.error(format!(
                            "internal error: `{name}` needs effect `{rendered}`, which                              the function type it is passed as does not declare"
                        ));
                    }
                }
            }
        }
        let names: Vec<String> = (0..decl.params.len())
            .map(|i| format!("__a{i}"))
            .collect();
        let fwd: Vec<String> = decl
            .params
            .iter()
            .enumerate()
            .map(|(i, p)| {
                // Incoming mode per the expected contract; forwarding per
                // the declaration's actual mode. Illegal combinations are
                // checker-rejected before emission [fn-contract].
                let (in_kept, in_mut) =
                    ast_fn_param_contract(exp_params, param_names, deductions, i);
                let in_ref = in_kept
                    && !exp_params
                        .get(i)
                        .is_some_and(|t| self.is_copy_ast_type(t));
                let mode = self.param_mode(key, p);
                match (mode, in_ref, in_mut) {
                    (ParamMode::Owned, false, _) => format!("__a{i}"),
                    (ParamMode::Ref, true, false) => format!("__a{i}"),
                    (ParamMode::Ref, true, true) => format!("&*__a{i}"),
                    (ParamMode::Ref, false, _) => format!("&__a{i}"),
                    (ParamMode::RefMut, true, true) => format!("__a{i}"),
                    (ParamMode::RefMut, false, _) => format!("&mut __a{i}"),
                    // Owned decl fed by a reference: checker-rejected
                    // (consuming where keeping); render defensively.
                    (ParamMode::Owned, true, _) => format!("__a{i}.clone()"),
                    (ParamMode::RefMut, true, false) => format!("&mut *__a{i}"),
                }
            })
            .collect();
        let mut all_params = effect_params;
        all_params.extend(names.iter().map(|p| format!("mut {p}")));
        let mut all_args = forwarded_effects;
        all_args.extend(fwd);
        Some(format!(
            "&mut |{}| {rust_name}({})",
            all_params.join(", "),
            all_args.join(", ")
        ))
    }

    /// Renders an argument for a `&T` parameter position [rs-borrows].
    fn borrowed_arg(&mut self, expr: &Expr) -> String {
        if let Expr::Ident(id) = expr {
            if id.name != "None" && self.ident_unwrap(id).is_none() {
                match self.bindings.get(id.name.as_str()) {
                    // Already a reference: pass through (deref coercion
                    // turns `&mut T` into `&T`).
                    Some(BindKind::Ref) | Some(BindKind::RefMut) => {
                        return self.binding_place(&id.name)
                    }
                    Some(BindKind::SelfField) => {
                        return format!("&{}", self.binding_place(&id.name))
                    }
                    _ => return format!("&{}", self.binding_place(&id.name)),
                }
            }
        }
        match expr {
            Expr::Field { .. } | Expr::TupleIndex { .. } | Expr::Index { .. } => {
                format!("&{}", self.emit_place(expr))
            }
            other => {
                let code = self.emit_expr(other);
                format!("&({code})")
            }
        }
    }

    /// Renders an argument for a `&mut T` parameter position [rs-borrows].
    fn borrowed_mut_arg(&mut self, expr: &Expr) -> String {
        if let Expr::Ident(id) = expr {
            if id.name != "None" && self.ident_unwrap(id).is_none() {
                match self.bindings.get(id.name.as_str()) {
                    // Already `&mut`: implicit reborrow at the call.
                    Some(BindKind::RefMut) => return self.binding_place(&id.name),
                    Some(BindKind::SelfField) => {
                        return format!("&mut {}", self.binding_place(&id.name))
                    }
                    _ => return format!("&mut {}", self.binding_place(&id.name)),
                }
            }
        }
        match expr {
            Expr::Field { .. } | Expr::TupleIndex { .. } | Expr::Index { .. } => {
                format!("&mut {}", self.emit_place(expr))
            }
            other => {
                let code = self.emit_expr(other);
                format!("&mut ({code})")
            }
        }
    }
}

impl<'p> Emitter<'p> {
    // ================= coercions =================

    /// Applies a checker-recorded representation change [union-arm-identity]
    /// [rs-union-enums] [rs-option].
    fn apply_coercion(&mut self, span: Span, code: String) -> String {
        let Some(coercion) = self.coercion_of(span) else {
            return code;
        };
        match coercion.clone() {
            // [rs-option] Optionals are physical in Rust.
            Coercion::WrapOption { .. } => format!("Some({code})"),
            Coercion::WrapUnion { target, arm } => self.wrap_union_value(&target, arm, code),
            Coercion::Rewrap { from, to } => self.emit_rewrap(code, &from, &to),
        }
    }

    /// Wraps a value into arm `arm` of a wrapper union
    /// [union-arm-identity]: `Union2::<i32, String>::U1(value)`, in a
    /// `Some(...)` when the target also has a `None` arm.
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
                self.rust_ty(&a)
            })
            .collect();
        let wrapped = format!("Union{n}::<{}>::U{}({code})", args.join(", "), arm + 1);
        if target.has_none_arm() {
            format!("Some({wrapped})")
        } else {
            wrapped
        }
    }

    /// Re-wraps a value between two union representations: a `match`
    /// mapping arms by type equality [union-arm-identity].
    fn emit_rewrap(&mut self, code: String, from: &Ty, to: &Ty) -> String {
        let from_arms: Vec<Ty> = from.value_arms().into_iter().cloned().collect();
        let to_arms: Vec<Ty> = to.value_arms().into_iter().cloned().collect();
        let n = from_arms.len();
        let m = to_arms.len();
        if n >= 2 {
            self.union_sizes.insert(n);
        }
        if m >= 2 {
            self.union_sizes.insert(m);
        }
        let to_args: Vec<String> = to_arms.iter().map(|a| self.rust_ty(a)).collect();
        let to_args = to_args.join(", ");
        let mut branches = String::new();
        for (i, fa) in from_arms.iter().enumerate() {
            // Source arm pattern: enum variant for wrappers, the bare
            // payload for a `T?` source.
            let pat = if n >= 2 {
                format!("Union{n}::U{}(__v)", i + 1)
            } else {
                "__v".to_string()
            };
            let pat = if from.has_none_arm() {
                format!("Some({pat})")
            } else {
                pat
            };
            let result = match to_arms.iter().position(|ta| ta == fa) {
                Some(j) => {
                    let wrapped = if m >= 2 {
                        format!("Union{m}::<{to_args}>::U{}(__v)", j + 1)
                    } else {
                        "__v".to_string()
                    };
                    if to.has_none_arm() {
                        format!("Some({wrapped})")
                    } else {
                        wrapped
                    }
                }
                None => "unreachable!(\"unreachable union arm\")".to_string(),
            };
            branches.push_str(&format!("{pat} => {result}, "));
        }
        if from.has_none_arm() {
            if to.has_none_arm() {
                branches.push_str("None => None, ");
            } else {
                branches.push_str("None => unreachable!(\"unreachable union arm\"), ");
            }
        }
        format!("(match {code} {{ {branches}}})")
    }

    // ================= strings =================

    /// String literals and interpolation [type-str]: plain literals get
    /// `.to_string()`; interpolations lower to `format!` (still-union
    /// values are `Display` via the generated enum impls
    /// [rs-union-enums]).
    fn emit_string(&mut self, parts: &[StrExprPart]) -> String {
        let interps: Vec<&Expr> = parts
            .iter()
            .filter_map(|p| match p {
                StrExprPart::Interp(e) => Some(e.as_ref()),
                _ => None,
            })
            .collect();
        if interps.is_empty() {
            let text: String = parts
                .iter()
                .map(|p| match p {
                    StrExprPart::Text(t) => t.as_str(),
                    _ => "",
                })
                .collect();
            return format!("\"{}\".to_string()", escape_string(&text));
        }
        let mut fmt = String::new();
        let mut args: Vec<String> = Vec::new();
        for part in parts {
            match part {
                StrExprPart::Text(text) => fmt.push_str(&escape_format_text(text)),
                StrExprPart::Interp(expr) => {
                    fmt.push_str("{}");
                    args.push(self.emit_expr(expr));
                }
            }
        }
        format!("format!(\"{fmt}\", {})", args.join(", "))
    }

    // ================= value-position control flow =================

    /// An `if`/`elif`/`else` chain in value position: Rust `if` is an
    /// expression; a missing `else` contributes `None` [if-else-none]
    /// (branch values carry their `WrapOption`/wrap coercions). `indent` is
    /// the column of the *statement* the expression sits in — the opening
    /// `if ` is written inline after whatever precedes it, so only the branch
    /// bodies and the closing braces need padding.
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
            out.push_str(&format!("{kw} {} {{\n", cond_code(c)));
            out.push_str(&self.emit_is_bindings(cond, indent + 1));
            // [qual-widen]
            let (shadows, saved) = self.emit_widen_shadows(cond, indent + 1);
            out.push_str(&shadows);
            out.push_str(&self.emit_value_block(block, indent + 1));
            self.restore_bindings(saved);
            out.push_str(&format!("{pad}}}"));
        }
        match else_block {
            Some(block) => {
                out.push_str(" else {\n");
                out.push_str(&self.emit_value_block(block, indent + 1));
                out.push_str(&format!("{pad}}}"));
            }
            None => {
                let inner = "    ".repeat(indent + 1);
                out.push_str(&format!(" else {{\n{inner}None\n{pad}}}"));
            }
        }
        out
    }

    /// A block in value position: statements plus the trailing expression
    /// as the block's value. `indent` is the column its statements sit at.
    fn emit_value_block(&mut self, block: &Block, indent: usize) -> String {
        let mut out = String::new();
        // [rs-effect-fusion] Save the whole environment, not just its
        // depth: a `use` inside the block *rewrites* the outer entries to
        // thread through the inner fusion, which dies with the block.
        let saved_env = self.effect_env.clone();
        let defer_floor = self.defers.len();
        // [rs-defer-splice] Deferred blocks run *after* the block's value
        // is computed, so a tail with deferred code behind it is hoisted
        // into a temporary.
        let mut tail_tmp: Option<String> = None;
        let pad = "    ".repeat(indent);
        let n = block.stmts.len();
        for (i, stmt) in block.stmts.iter().enumerate() {
            if i + 1 == n {
                if let Stmt::Expr(e) = stmt {
                    // The tail is the value. `Nothing`-typed tails
                    // (`return`-like) stay statements.
                    if matches!(self.ty_of(e.span()), Some(Ty::Nothing)) {
                        out.push_str(&self.emit_expr_stmt(e, indent, StmtCtx::Normal));
                    } else {
                        self.expr_indent = indent;
                        let code = self.emit_expr(e);
                        if self.defers.len() > defer_floor {
                            let tmp = self.fresh_defer_var();
                            out.push_str(&format!("{pad}let {tmp} = {code};\n"));
                            tail_tmp = Some(tmp);
                        } else {
                            out.push_str(&format!("{pad}{code}\n"));
                        }
                    }
                    continue;
                }
            }
            out.push_str(&self.emit_stmt(stmt, indent, StmtCtx::Normal));
        }
        out.push_str(&self.splice_defers(defer_floor, indent, true));
        if let Some(tmp) = tail_tmp {
            out.push_str(&format!("{pad}{tmp}\n"));
        }
        self.effect_env = saved_env;
        out
    }

    /// A `when` over a union-typed subject lowers to a `match`
    /// [when-union-subject]; `match` is an expression, so the value form
    /// is free. A wildcard arm covers repr arms the (possibly narrowed)
    /// subject can no longer hold [when-exhaustive].
    fn emit_when(
        &mut self,
        subject: &Expr,
        branches: &[WhenBranch],
        indent: usize,
        ctx: StmtCtx,
        value_pos: bool,
    ) -> String {
        let pad = "    ".repeat(indent);
        // `match` scrutinizes the storage, not a narrowed read
        // [flow-place].
        let subj = self.place_storage(subject);
        let tests: Vec<Option<UnionTest>> = branches
            .iter()
            .map(|b| self.is_test_of(b.span).cloned())
            .collect();
        let Some(size) = tests.iter().flatten().map(|t| t.size).next() else {
            self.error("`when` could not be lowered (subject is not a checked union)");
            return String::new();
        };
        let nullable = tests.iter().flatten().any(|t| t.nullable);
        if size >= 2 {
            self.union_sizes.insert(size);
        }

        let mut out = format!("match {subj} {{\n");
        let mut covered_arms: BTreeSet<usize> = BTreeSet::new();
        let mut covered_none = !nullable;
        for (branch, test) in branches.iter().zip(&tests) {
            let Some(test) = test else { continue };
            let pattern = if test.match_none {
                covered_none = true;
                "None".to_string()
            } else if size == 1 {
                covered_arms.insert(0);
                if nullable {
                    "Some(_)".to_string()
                } else {
                    "_".to_string()
                }
            } else {
                let pats: Vec<String> = test
                    .arms
                    .iter()
                    .map(|i| {
                        covered_arms.insert(*i);
                        let pat = format!("Union{}::U{}(_)", test.size, i + 1);
                        if nullable {
                            format!("Some({pat})")
                        } else {
                            pat
                        }
                    })
                    .collect();
                pats.join(" | ")
            };
            out.push_str(&format!("{pad}    {pattern} => {{\n"));
            if let Some(b) = &branch.binding {
                self.bindings.insert(b.name.clone(), BindKind::Owned);
                let target = self.ty_of(b.span).cloned();
                let read =
                    self.emit_narrowed_read(subject, target.as_ref(), Some(test.clone()));
                out.push_str(&format!(
                    "{pad}        let mut {} = {read};\n",
                    rs_ident(&b.name)
                ));
            }
            // [qual-widen] A `^` branch head peels the arm it matched: bind
            // the widened value to a shadowing local so the body — and any
            // nested `when` on the subject — sees it at the widened type.
            let mut saved_widen: Vec<(String, Option<BindKind>)> = Vec::new();
            if branch.widen {
                if let Some(target) = self
                    .checked
                    .widen_targets
                    .get(&(self.file_idx, branch.span))
                    .cloned()
                {
                    match subject {
                        Expr::Ident(id) => {
                            let code = self.emit_narrowed_read(
                                subject,
                                Some(&target),
                                Some(test.clone()),
                            );
                            let displaced =
                                self.bindings.insert(id.name.clone(), BindKind::Owned);
                            saved_widen.push((id.name.clone(), displaced));
                            out.push_str(&format!(
                                "{pad}        let mut {} = {code};\n",
                                rs_ident(&id.name)
                            ));
                        }
                        other => {
                            let place = self.emit_raw(other);
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
                out.push_str(&self.emit_block_stmts(&branch.body, indent + 2, ctx));
            }
            self.restore_bindings(saved_widen);
            out.push_str(&format!("{pad}    }}\n"));
        }
        // Arms the narrowed subject can no longer hold: the checker
        // proved them impossible [when-exhaustive]; rustc still needs the
        // match to be exhaustive over the repr.
        if covered_arms.len() < size || !covered_none {
            out.push_str(&format!(
                "{pad}    _ => unreachable!(\"unreachable union arm\"),\n"
            ));
        }
        out.push_str(&format!("{pad}}}"));
        out
    }

    /// A fresh `__loopN` local name for loop lowering.
    fn fresh_loop_var(&mut self) -> String {
        self.loop_id += 1;
        format!("__loop{}", self.loop_id)
    }

    fn unique_name(&mut self, base: String) -> String {
        let mut name = base.clone();
        let mut i = 1;
        while !self.taken_names.insert(name.clone()) {
            i += 1;
            name = format!("{base}{i}");
        }
        name
    }

    /// The `for <pat> in ...` binding for a Salvo loop pattern.
    fn for_pattern_var(&mut self, pattern: &Pattern, by_ref: bool) -> String {
        match pattern {
            Pattern::Ident(id) => {
                // [rs-borrow-locals] A by-reference loop binds `&T`.
                if by_ref {
                    self.bindings.insert(id.name.clone(), BindKind::Ref);
                    return rs_ident(&id.name);
                }
                self.bindings.insert(id.name.clone(), BindKind::Owned);
                format!("mut {}", rs_ident(&id.name))
            }
            Pattern::Tuple { elems, .. } => {
                let names: Vec<String> = elems
                    .iter()
                    .map(|p| match p {
                        Pattern::Ident(id) => {
                            self.bindings.insert(id.name.clone(), BindKind::Owned);
                            format!("mut {}", rs_ident(&id.name))
                        }
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

    /// Lowers a value-position loop [while-value] [rs-loop-value] to a
    /// block expression with an `Option` result local. When the checked
    /// join type is itself optional, the local *is* the join type (tail
    /// coercions already produce `Option` values); otherwise the local is
    /// `Option<join>` and the block ends `.unwrap()`.
    fn emit_loop_value(&mut self, expr: &Expr) -> String {
        let join = self.ty_of(expr.span()).cloned();
        let (local_ty, join_optional, needs_unwrap) = match &join {
            Some(t)
                if ty_is_concrete(t) && !t.is_none_ty() && !matches!(t, Ty::Nothing) =>
            {
                if t.has_none_arm() {
                    (self.rust_ty(t), true, false)
                } else {
                    (format!("Option<{}>", self.rust_ty(t)), false, true)
                }
            }
            _ => {
                self.error(
                    "a value-position loop with an unchecked value type is not \
                     supported by the rust backend",
                );
                ("Option<()>".to_string(), false, false)
            }
        };
        let result = self.fresh_loop_var();
        let ran = format!("{result}_ran");
        let mut out = String::new();
        out.push_str("({\n");
        out.push_str(&format!("let mut {result}: {local_ty} = None;\n"));
        let (cond_or_iter, body, else_block, is_for, pattern) = match expr {
            Expr::While {
                cond,
                body,
                else_block,
                ..
            } => (cond.as_ref(), body, else_block, false, None),
            Expr::For {
                pattern,
                iterable,
                body,
                else_block,
                ..
            } => (iterable.as_ref(), body, else_block, true, Some(pattern)),
            _ => unreachable!("emit_loop_value only receives loops"),
        };
        if else_block.is_some() {
            out.push_str(&format!("let mut {ran} = false;\n"));
        }
        if is_for {
            // Value-position loops keep owned iteration (no borrow
            // refinement yet [rs-borrow-locals]).
            let var = self.for_pattern_var(pattern.unwrap(), false);
            let iter = self.emit_expr(cond_or_iter);
            out.push_str(&format!("for {var} in {iter} {{\n"));
        } else {
            let c = self.emit_expr(cond_or_iter);
            out.push_str(&format!("while {} {{\n", cond_code(c)));
        }
        if else_block.is_some() {
            out.push_str(&format!("{ran} = true;\n"));
        }
        if !is_for {
            out.push_str(&self.emit_is_bindings(cond_or_iter, 0));
        }
        self.loop_results.push(Some(result.clone()));
        self.loop_defer_floors.push(self.defers.len());
        out.push_str(&self.emit_loop_body_value(body, &result, join_optional));
        self.loop_defer_floors.pop();
        self.loop_results.pop();
        out.push_str("}\n");
        if let Some(b) = else_block {
            out.push_str(&format!("if !{ran} {{\n"));
            out.push_str(&self.emit_loop_body_value(b, &result, join_optional));
            out.push_str("}\n");
        }
        if needs_unwrap {
            out.push_str(&format!("{result}.unwrap()\n}})"));
        } else {
            out.push_str(&format!("{result}\n}})"));
        }
        out
    }

    /// Whether the enclosing value-loop's join type is optional (its
    /// result local holds coerced values directly, no `Some(...)`).
    fn loop_join_optional(&self, result: &str) -> bool {
        // Recorded at emission time via the naming convention: resolved
        // through `emit_loop_value`'s bookkeeping (see loop_optional map).
        self.loop_optional.get(result).copied().unwrap_or(false)
    }

    /// Assigns a loop-value expression into the result local: wrapped in
    /// `Some(...)` unless the join is optional (tail coercions then
    /// already produce `Option` values) [rs-loop-value].
    fn emit_loop_value_assign(&mut self, value: &Expr, result: &str) -> String {
        let code = self.emit_expr(value);
        if self.loop_join_optional(result) {
            format!("{result} = {code};")
        } else {
            format!("{result} = Some({code});")
        }
    }

    /// A loop body (or `else` block) whose tail expression assigns the
    /// loop's result local [while-value].
    fn emit_loop_body_value(
        &mut self,
        block: &Block,
        result: &str,
        join_optional: bool,
    ) -> String {
        self.loop_optional.insert(result.to_string(), join_optional);
        let mut out = String::new();
        // [rs-effect-fusion] Save the whole environment, not just its
        // depth: a `use` inside the block *rewrites* the outer entries to
        // thread through the inner fusion, which dies with the block.
        let saved_env = self.effect_env.clone();
        let defer_floor = self.defers.len();
        let n = block.stmts.len();
        for (i, stmt) in block.stmts.iter().enumerate() {
            if i + 1 == n {
                if let Stmt::Expr(e) = stmt {
                    out.push_str(&self.emit_tail_assign(e, result, join_optional));
                    continue;
                }
            }
            out.push_str(&self.emit_stmt(stmt, 0, StmtCtx::Normal));
        }
        // [rs-defer-splice] The tail already assigned the result local, so
        // deferred blocks run after it, like in any other block.
        out.push_str(&self.splice_defers(defer_floor, 0, true));
        self.effect_env = saved_env;
        out
    }

    /// Assigns a block-tail expression to a loop result local.
    /// `Nothing`-typed tails never fall through; `None`-typed tails
    /// record `None`.
    fn emit_tail_assign(&mut self, e: &Expr, result: &str, join_optional: bool) -> String {
        match self.ty_of(e.span()) {
            Some(Ty::Nothing) => self.emit_expr_stmt(e, 0, StmtCtx::Normal),
            Some(t) if t.is_none_ty() => {
                let stmt = self.emit_expr_stmt(e, 0, StmtCtx::Normal);
                format!("{stmt}{result} = None;\n")
            }
            _ => {
                let code = self.emit_expr(e);
                if join_optional {
                    format!("{result} = {code};\n")
                } else {
                    format!("{result} = Some({code});\n")
                }
            }
        }
    }

    fn emit_lambda(
        &mut self,
        params: &[LambdaParam],
        body: &LambdaBody,
        span: Span,
    ) -> String {
        // [fn-contract] Parameters the expected contract keeps are
        // reference bindings (mutably for `Mut`); moved (and
        // uncontracted) parameters stay owned.
        let contract = self
            .checked
            .lambda_contracts
            .get(&(self.file_idx, span))
            .cloned();
        let saved: Vec<(String, Option<BindKind>)> = params
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let kind = match contract.as_ref().and_then(|c| c.get(i)) {
                    Some(e) if e.kept && e.mutable => BindKind::RefMut,
                    Some(e)
                        if e.kept
                            && !p
                                .ty
                                .as_ref()
                                .is_some_and(|t| self.is_copy_ast_type(t)) =>
                    {
                        BindKind::Ref
                    }
                    _ => BindKind::Owned,
                };
                let old = self.bindings.insert(p.name.name.clone(), kind);
                (p.name.name.clone(), old)
            })
            .collect();
        // [fn-effects] The effects a call of this value performs arrive as
        // *leading parameters*, not captures: that is what keeps the closure
        // from holding a borrow across an effectful call [rs-effect-fusion].
        let lambda_effects: Vec<Ty> = self
            .checked
            .lambda_effects
            .get(&(self.file_idx, span))
            .cloned()
            .unwrap_or_default();
        let saved_effect_env = self.effect_env.clone();
        let mut param_list: Vec<String> = Vec::new();
        for ty in &lambda_effects {
            let rendered = self.rust_ty(ty);
            let var = self.unique_name(effect_param_name(&rendered));
            self.effect_env.push(EffectEntry {
                ty: Some(ty.clone()),
                key: rendered.clone(),
                var: var.clone(),
                is_local: false,
            });
            self.bindings.insert(var.clone(), BindKind::RefMut);
            param_list.push(format!("{var}: &mut dyn {rendered}"));
        }
        param_list.extend(params.iter().enumerate().map(|(i, p)| match &p.ty {
            Some(t) => {
                let ty = self.emit_type(t);
                // [fn-contract] Annotations match the binding mode.
                let ty = match contract.as_ref().and_then(|c| c.get(i)) {
                    Some(e) if e.kept && e.mutable => format!("&mut {ty}"),
                    Some(e) if e.kept && !self.is_copy_ast_type(t) => {
                        format!("&{ty}")
                    }
                    _ => ty,
                };
                format!("{}: {ty}", rs_ident(&p.name.name))
            }
            None => rs_ident(&p.name.name),
        }));
        let out = match body {
            LambdaBody::Expr(expr) => {
                format!("|{}| {}", param_list.join(", "), self.emit_expr(expr))
            }
            LambdaBody::Block(block) => {
                // Closures return their last expression; a trailing
                // `return X` becomes the value.
                let mut out = format!("|{}| {{\n", param_list.join(", "));
                // [rs-defer-splice] A closure is a function boundary: its
                // `return` runs only the deferred blocks written inside it.
                let saved_floor = self.defer_floor;
                self.defer_floor = self.defers.len();
                let saved_loop_floors = std::mem::take(&mut self.loop_defer_floors);
                let body_floor = self.defers.len();
                let n = block.stmts.len();
                for (i, stmt) in block.stmts.iter().enumerate() {
                    if i + 1 == n {
                        if let Stmt::Return { value: Some(v), .. } = stmt {
                            let code = self.emit_expr(v);
                            let defers = self.splice_defers(body_floor, 1, true);
                            if defers.is_empty() {
                                out.push_str(&format!("    {code}\n"));
                            } else {
                                let tmp = self.fresh_defer_var();
                                out.push_str(&format!("    let {tmp} = {code};\n"));
                                out.push_str(&defers);
                                out.push_str(&format!("    {tmp}\n"));
                            }
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
                    out.push_str(&self.emit_stmt(stmt, 1, StmtCtx::Normal));
                }
                out.push_str(&self.splice_defers(body_floor, 1, true));
                self.loop_defer_floors = saved_loop_floors;
                self.defer_floor = saved_floor;
                out.push('}');
                out
            }
        };
        self.effect_env = saved_effect_env;
        for (name, old) in saved {
            match old {
                Some(kind) => self.bindings.insert(name, kind),
                None => self.bindings.remove(&name),
            };
        }
        out
    }

    /// A struct literal [struct-decl] [struct-defaults] [struct-spread]:
    /// omitted fields inline their declared defaults; a spread becomes
    /// Rust functional-update syntax with a cloned base.
    fn emit_struct_lit(
        &mut self,
        ty: Option<&Type>,
        fields: &[StructLitField],
        _span: Span,
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
                    Some((rs_ident(&name.name), self.emit_expr(value)))
                }
                _ => None,
            })
            .collect();
        let mut named_args: Vec<String> =
            named.iter().map(|(n, v)| format!("{n}: {v}")).collect();

        let Some(name) = type_name else {
            self.error("struct literals without a type annotation are not supported here");
            return "todo!()".to_string();
        };
        match spreads.len() {
            0 => {
                // Inline declared defaults for omitted fields
                // [struct-defaults] (Rust has no default arguments).
                if let Some(decl) = self.symbols.structs.get(name.as_str()).copied() {
                    let provided: HashSet<String> = fields
                        .iter()
                        .filter_map(|f| match &f.kind {
                            StructLitFieldKind::Named { name, .. } => {
                                Some(rs_ident(&name.name))
                            }
                            _ => None,
                        })
                        .collect();
                    for df in &decl.fields {
                        let fname = rs_ident(&df.name.name);
                        if provided.contains(&fname) {
                            continue;
                        }
                        if let Some(default) = &df.default {
                            let default = default.clone();
                            let code = self.emit_default_value(&default, &df.ty);
                            named_args.push(format!("{fname}: {code}"));
                        }
                    }
                }
                format!("{} {{ {} }}", rs_ident(&name), named_args.join(", "))
            }
            1 => {
                // `P {...base, f: v}` -> `P { f: v, ..base.clone() }`
                // [struct-spread]. The checker consumes the spread base
                // [deduce-consume], so the clone-vs-move choice is
                // unobservable (a real move is a deferred perf
                // refinement); the clone also keeps unchecked contexts
                // safe.
                let base = match spreads[0] {
                    e @ (Expr::Ident(_)
                    | Expr::Field { .. }
                    | Expr::TupleIndex { .. }
                    | Expr::Index { .. }) => {
                        format!("{}.clone()", self.emit_place(e))
                    }
                    other => self.emit_expr(other),
                };
                format!(
                    "{} {{ {}..{base} }}",
                    rs_ident(&name),
                    if named_args.is_empty() {
                        String::new()
                    } else {
                        format!("{}, ", named_args.join(", "))
                    }
                )
            }
            _ => {
                self.error("struct literals with multiple spreads are not supported yet");
                "todo!()".to_string()
            }
        }
    }

    /// A struct-field default expression inlined at a literal site
    /// [struct-defaults]: emitted without checker tables (the spans
    /// belong to the struct's file), so the `T?` wrap is applied
    /// syntactically.
    fn emit_default_value(&mut self, default: &Expr, field_ty: &Type) -> String {
        let code = self.emit_expr(default);
        let optional_field = matches!(field_ty, Type::Nullable { .. })
            || matches!(field_ty, Type::Union { arms, .. } if arms.iter().any(is_none_type));
        if optional_field && !matches!(default, Expr::Ident(id) if id.name == "None") {
            format!("Some({code})")
        } else {
            code
        }
    }
}

impl<'p> Emitter<'p> {
    // ================= calls =================

    fn emit_call(
        &mut self,
        callee: &Expr,
        type_args: &[Type],
        args: &[Expr],
        span: Span,
    ) -> String {
        // [abort] [rs-abort-controlflow] A call that may abort is not an
        // ordinary call: `abort` itself *is* the control transfer, and a
        // call that propagates one unwraps its `ControlFlow`.
        if let Some(site) = self.checked.may_abort.get(&(self.file_idx, span)).cloned() {
            if site.performs {
                return self.emit_abort_call(&site, args, self.expr_indent);
            }
            let call = self.emit_call_inner(callee, type_args, args, span);
            return self.wrap_may_abort_call(&site, call, self.expr_indent);
        }
        self.emit_call_inner(callee, type_args, args, span)
    }

    fn emit_call_inner(
        &mut self,
        callee: &Expr,
        type_args: &[Type],
        args: &[Expr],
        span: Span,
    ) -> String {
        // Normalize dot-notation [fn-dot].
        if let Expr::Field { base, field, .. } = callee {
            let name = field.name.as_str();
            let total = args.len() + 1;
            if self.symbols.effect_of_fn.contains_key(name)
                || self.symbols.resolve_define_fn(name, total).is_some()
                || self.symbols.resolve_fn(name, total).is_some()
            {
                let mut all_args: Vec<&Expr> = Vec::with_capacity(total);
                all_args.push(base);
                all_args.extend(args.iter());
                return self.emit_resolved_call(name, type_args, &all_args, span);
            }
            // [call-resolve] The checker rejects an undeclared dot-call, so
            // reaching here means a resolution table lost an entry without
            // reporting it — a compiler bug. Say so instead of inventing a
            // Rust method call: emitted code must never be a guess
            // [backend-never-wrong].
            self.error(format!(
                "internal error: dot-call `{name}` reached the Rust emitter \
                 unresolved (the checker should have rejected it, or resolved \
                 it to a fn, define, or effect member)"
            ));
            "todo!()".to_string()
        } else if let Expr::Ident(id) = callee {
            let arg_refs: Vec<&Expr> = args.iter().collect();
            self.emit_resolved_call(&id.name, type_args, &arg_refs, span)
        } else {
            // Calling a computed value (lambda etc.): owned args [fn-lambda],
            // with its effects threaded first [fn-effects].
            let callee_code = self.emit_owned(callee);
            let mut arg_code: Vec<String> = self.fn_value_effect_args(span);
            arg_code.extend(args.iter().map(|a| self.emit_expr(a)));
            format!("{callee_code}({})", arg_code.join(", "))
        }
    }

    fn emit_resolved_call(
        &mut self,
        name: &str,
        type_args: &[Type],
        args: &[&Expr],
        span: Span,
    ) -> String {
        // 1. Effect member call: dispatch through the handler in scope
        // [rs-effects] ([effect-disambiguation], `effect_calls`). Under the
        // fusion one value implements every effect in scope, so dispatch is
        // UFCS: plain method syntax would be ambiguous between two effects
        // with a same-named member, and between two instances of a generic
        // effect [rs-effect-fusion].
        if let Some(effect) = self.symbols.effect_of_fn.get(name).copied() {
            let checked_effect = self.checked.effect_calls.get(&(self.file_idx, span)).cloned();
            let handler = match &checked_effect {
                Some(ty) if ty_is_concrete(ty) => {
                    let ty = ty.clone();
                    self.member_dispatch_by_ty(&ty)
                }
                _ => self.member_dispatch_fallback(effect, type_args),
            };
            // Effect member params: default kept rule [rs-borrows].
            let member = self
                .symbols
                .effects
                .get(effect)
                .and_then(|e| e.fns.iter().find(|f| f.name.name == name));
            let arg_code = match member {
                Some(m) => {
                    let m = m.clone();
                    self.emit_args_for_params(&m.params, args, None)
                }
                None => args.iter().map(|a| self.emit_expr(a)).collect(),
            };
            if !self.fusion {
                return format!("{handler}.{}({})", rs_ident(name), arg_code.join(", "));
            }
            let path = match &checked_effect {
                Some(ty) if ty_is_concrete(ty) => {
                    let ty = ty.clone();
                    let (base, args) = self.ty_effect_parts(&ty);
                    trait_path(&base, &args)
                }
                _ => {
                    let rendered = self.member_dispatch_key(effect, type_args);
                    trait_path_of_rendered(&rendered)
                }
            };
            let (prelude, arg_code) =
                self.hoist_effect_args(Some(&[handler.clone()]), arg_code);
            let mut all = vec![handler];
            all.extend(arg_code);
            return Self::wrap_hoisted(
                &prelude,
                format!("{path}::{}({})", rs_ident(name), all.join(", ")),
            );
        }

        // 2. Checker-resolved fn target (type-based overloads win)
        // [fn-overload]. Intrinsic fns lower in the emitter [intrinsic-fn];
        // external signatures route to their define.
        let checker_resolved = self
            .checked
            .call_fn
            .get(&(self.file_idx, span))
            .and_then(|key| self.fn_by_key(*key).map(|f| (*key, f)));
        if let Some((key, f)) = checker_resolved {
            if f.backing == Some(BackingMod::Intrinsic) {
                return self.emit_intrinsic_call(f, args, span);
            }
            if f.body.is_none() {
                if let Some(def) = self.define_for_decl(name, f) {
                    return self.emit_define_call(name, def, args, span);
                }
                self.error(format!("external fn `{name}` has no rust `define fn`"));
                return "todo!()".to_string();
            }
            return self.emit_fn_call(name, f, Some(key), args, span);
        }

        // 3. `define fn` template (unchecked contexts): arity narrowed by
        // the checked argument types; ambiguous dispatch is a codegen
        // error, never a guess [backend-never-wrong] [fn-overload].
        let define_cands = self.symbols.defines_matching_arity(name, args.len());
        if !define_cands.is_empty() {
            return match disambiguate_unchecked(
                self,
                &define_cands,
                |d| d.sig.params.as_slice(),
                args,
            ) {
                Some(def) => self.emit_define_call(name, def, args, span),
                None => {
                    self.error(format!(
                        "call to `{name}` is ambiguous here: multiple same-arity \
                         `define fn` templates match and the checker did not \
                         resolve the overload; annotate the argument types"
                    ));
                    "todo!()".to_string()
                }
            };
        }

        // 4. Known function (unchecked contexts): same ambiguity rule.
        let fn_cands = self.symbols.fns_matching_arity(name, args.len());
        if !fn_cands.is_empty() {
            let Some(f) = disambiguate_unchecked(self, &fn_cands, |f| f.params.as_slice(), args)
            else {
                self.error(format!(
                    "call to `{name}` is ambiguous here: multiple same-arity \
                     overloads match and the checker did not resolve the \
                     overload; annotate the argument types"
                ));
                return "todo!()".to_string();
            };
            if f.backing == Some(BackingMod::Intrinsic) {
                return self.emit_intrinsic_call(f, args, span);
            }
            if f.body.is_none() {
                self.error(format!("external fn `{name}` has no rust `define fn`"));
                return "todo!()".to_string();
            }
            let key = self.key_of_fn(f);
            return self.emit_fn_call(name, f, key, args, span);
        }

        // 5. Local callable / interop. A call through a fn-typed value
        // renders its arguments per the recorded contract [fn-contract]:
        // kept non-Copy borrows, kept `Mut` borrows mutably, moved (or
        // Copy) owned. Interop calls (no contract) keep owned
        // pass-through.
        let contract = self
            .checked
            .fn_value_calls
            .get(&(self.file_idx, span))
            .cloned();
        let arg_code: Vec<String> = match contract {
            Some(entries) => args
                .iter()
                .enumerate()
                .map(|(i, a)| {
                    let (kept, is_mut) = entries
                        .get(i)
                        .map(|e| (e.kept, e.mutable))
                        .unwrap_or((true, false));
                    let copy = self
                        .ty_of(a.span())
                        .is_some_and(|t| Self::is_copy_ty(t));
                    if kept && is_mut {
                        self.borrowed_mut_arg(a)
                    } else if kept && !copy {
                        self.borrowed_arg(a)
                    } else {
                        self.emit_expr(a)
                    }
                })
                .collect(),
            None => args.iter().map(|a| self.emit_expr(a)).collect(),
        };
        // [fn-effects] A fn value takes its effects as leading arguments.
        let mut all = self.fn_value_effect_args(span);
        all.extend(arg_code);
        let generics = self.emit_call_type_args(type_args);
        format!("{}{generics}({})", rs_ident(name), all.join(", "))
    }

    /// [fn-effects] The effect arguments a fn-value call threads, from the
    /// instances the checker resolved for it. A fused value reborrows into
    /// the `&mut dyn Effect` parameter, so this works under the fusion too.
    fn fn_value_effect_args(&mut self, span: Span) -> Vec<String> {
        let effects: Vec<Ty> = self
            .checked
            .call_effects
            .get(&(self.file_idx, span))
            .cloned()
            .unwrap_or_default();
        effects
            .iter()
            .map(|ty| self.thread_effect_by_ty(ty))
            .collect()
    }

    /// Explicit call-site generic args (`next_random<Int>()` outside the
    /// effect path) render as turbofish.
    fn emit_call_type_args(&mut self, type_args: &[Type]) -> String {
        if type_args.is_empty() {
            String::new()
        } else {
            let strs: Vec<String> = type_args.iter().map(|t| self.emit_type(t)).collect();
            format!("::<{}>", strs.join(", "))
        }
    }

    /// Renders the arguments of a call against the callee's declared
    /// parameters: mode per parameter [rs-borrows], trailing arguments
    /// collected into a `vec![...]` for a variadic parameter
    /// [fn-variadic].
    fn emit_args_for_params(
        &mut self,
        params: &[Param],
        args: &[&Expr],
        fn_key: Option<salvo_core::FnKey>,
    ) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let variadic_at = params.iter().position(|p| p.variadic);
        let fixed = variadic_at.unwrap_or(params.len());
        for (i, param) in params.iter().enumerate().take(fixed) {
            let Some(arg) = args.get(i) else { break };
            let mode = match fn_key {
                Some(_) => self.param_mode(fn_key, param),
                None => self.default_param_mode(&param.ty, param.variadic),
            };
            out.push(self.emit_arg(arg, mode, Some(&param.ty)));
        }
        if variadic_at.is_some() {
            let rest = args.get(fixed..).unwrap_or(&[]);
            // A single spread forwards the whole vector [fn-variadic].
            if rest.len() == 1 {
                if let Expr::Spread { operand, .. } = rest[0] {
                    out.push(self.emit_owned(operand));
                    return out;
                }
            }
            if rest.iter().any(|a| matches!(a, Expr::Spread { .. })) {
                self.error(
                    "mixing spread and plain arguments in a variadic call is not \
                     supported yet",
                );
            }
            let items: Vec<String> = rest.iter().map(|a| self.emit_expr(a)).collect();
            out.push(format!("vec![{}]", items.join(", ")));
        }
        out
    }

    fn emit_arg(&mut self, arg: &Expr, mode: ParamMode, param_ty: Option<&Type>) -> String {
        // [fn-contract] A named fn passed into a fn-typed position wraps
        // in an adapter closure matching the expected contract.
        if let (Some(pt @ Type::Fn { .. }), Expr::Ident(id)) = (param_ty, arg) {
            if !self.bindings.contains_key(id.name.as_str()) {
                if let Some(code) = self.named_fn_adapter(&id.name, id.span, pt) {
                    return code;
                }
            }
        }
        match mode {
            ParamMode::Owned => self.emit_expr(arg),
            ParamMode::Ref => {
                // A coerced argument is a fresh temporary: borrow it.
                if self.coercion_of(arg.span()).is_some() {
                    let code = self.emit_expr(arg);
                    format!("&({code})")
                } else {
                    self.borrowed_arg(arg)
                }
            }
            ParamMode::RefMut => {
                if self.coercion_of(arg.span()).is_some() {
                    let code = self.emit_expr(arg);
                    format!("&mut ({code})")
                } else {
                    self.borrowed_mut_arg(arg)
                }
            }
        }
    }

    /// A call to an `intrinsic fn`, lowered directly by the compiler
    /// [intrinsic-fn]. Two kinds live here:
    ///
    /// - `copy` and `discard` dispatch on the argument's *shape*, which is
    ///   the whole reason they are intrinsics — `copy` is `.clone()` on the
    ///   argument's place (reads never consume, and every generated type
    ///   derives `Clone`), while a non-place argument is already a fresh
    ///   owned value and passes through [rs-copy] [linear-discard].
    /// - everything else std declares is looked up in
    ///   [`crate::intrinsics::fn_call`], keyed by the declaration the
    ///   checker resolved.
    ///
    /// An intrinsic with no lowering is a codegen error naming it, never a
    /// pass-through [backend-never-wrong].
    fn emit_intrinsic_call(&mut self, f: &FnDecl, args: &[&Expr], span: Span) -> String {
        // [linear-discard] `discard(x)` moves the value into `drop`.
        if f.name.name == "discard" && args.len() == 1 {
            let code = self.emit_expr(args[0]);
            return format!("drop({code})");
        }
        if f.name.name != "copy" || args.len() != 1 {
            let recv = f.params.first().and_then(|p| type_base_name(&p.ty));
            let arg_code = self.intrinsic_arg_code(f, args);
            if let Some(code) =
                crate::intrinsics::fn_call(&f.name.name, recv, &arg_code, &[])
            {
                return code;
            }
            self.error(format!(
                "intrinsic fn `{}` is not supported by the rust backend",
                f.name.name
            ));
            return "todo!()".to_string();
        }
        let _ = span;
        let arg = args[0];
        match arg {
            Expr::Ident(id) => {
                if let Some(unwrapped) = self.ident_unwrap(id) {
                    // The narrowing unwrap is already an owned clone.
                    return unwrapped;
                }
                format!("{}.clone()", self.binding_place(&id.name))
            }
            // Field/index reads already clone in owned position.
            Expr::Field { .. } | Expr::TupleIndex { .. } | Expr::Index { .. } => {
                self.emit_owned(arg)
            }
            other => self.emit_expr(other),
        }
    }

    /// The rendered arguments of an `intrinsic fn` call, in declaration
    /// order, preserving the distinction the define templates relied on
    /// [rs-borrows]: a *place* splices raw so a method-style lowering
    /// borrows it natively (`list.push(..)`), while a variadic tail
    /// splices owned because it lands inside a constructor (`vec![..]`).
    /// Getting this backwards either double-clones or moves out of a
    /// borrow.
    fn intrinsic_arg_code(&mut self, f: &FnDecl, args: &[&Expr]) -> Vec<String> {
        let variadic_at = f.params.iter().position(|p| p.variadic);
        args.iter()
            .enumerate()
            .map(|(i, arg)| {
                let is_variadic_part = variadic_at.is_some_and(|v| i >= v);
                match arg {
                    Expr::Ident(_)
                    | Expr::Field { .. }
                    | Expr::TupleIndex { .. }
                    | Expr::Index { .. }
                        if !is_variadic_part =>
                    {
                        self.emit_place(arg)
                    }
                    Expr::Spread { operand, .. } => self.emit_owned(operand),
                    other => self.emit_expr(other),
                }
            })
            .collect()
    }

    /// Finds the define template matching an external fn signature
    /// [backend-define-inline] (same base-type matching as Kotlin).
    fn define_for_decl(&self, name: &str, decl: &FnDecl) -> Option<&'p DefineFn> {
        let defs = self.symbols.define_fns.get(name)?;
        let shape_matches = |d: &DefineFn| {
            d.sig.params.len() == decl.params.len()
                && d.sig
                    .params
                    .iter()
                    .zip(&decl.params)
                    .all(|(a, b)| a.variadic == b.variadic)
        };
        let types_match = |d: &DefineFn| {
            d.sig.params.iter().zip(&decl.params).all(|(a, b)| {
                match (type_base_name(&a.ty), type_base_name(&b.ty)) {
                    (Some(a), Some(b)) => a == b,
                    _ => true,
                }
            })
        };
        defs.iter()
            .find(|d| shape_matches(d) && types_match(d))
            .or_else(|| defs.iter().find(|d| shape_matches(d)))
            .or_else(|| defs.first())
            .copied()
    }

    /// Inline expansion of a `define fn` template [backend-define-inline]:
    /// place arguments splice raw (method-style templates borrow them
    /// natively); everything else splices owned.
    fn emit_define_call(
        &mut self,
        name: &str,
        def: &'p DefineFn,
        args: &[&Expr],
        span: Span,
    ) -> String {
        if let Some(imports) = &def.body.imports {
            self.add_template_imports(imports);
        }
        if let Some(inline) = def.body.inline.clone() {
            let mut arg_code: Vec<String> = Vec::new();
            let variadic_at = def.sig.params.iter().position(|p| p.variadic);
            for (i, arg) in args.iter().enumerate() {
                let is_variadic_part = variadic_at.is_some_and(|v| i >= v);
                let code = match arg {
                    // Places splice raw so templates like
                    // `${list}.push(..)` borrow natively [rs-borrows] —
                    // except variadic parts, which are spliced into
                    // constructors (`vec![${...elems}]`) and must be
                    // owned.
                    Expr::Ident(_)
                        | Expr::Field { .. }
                        | Expr::TupleIndex { .. }
                        | Expr::Index { .. }
                        if !is_variadic_part =>
                    {
                        self.emit_place(arg)
                    }
                    Expr::Spread { operand, .. } => self.emit_owned(operand),
                    other => self.emit_expr(other),
                };
                arg_code.push(code);
            }
            let type_args = self.define_type_args(name, def, span);
            return self
                .expand_template(
                    &inline,
                    &def.sig.params,
                    &arg_code,
                    &def.sig.generics,
                    &type_args,
                )
                .trim()
                .to_string();
        }
        self.error(format!("define fn `{name}` has no inline section"));
        "todo!()".to_string()
    }

    /// [backend-define-generics] The rendered type arguments of this call,
    /// in the define's generic order, from the checker's `call_type_args`
    /// ([call-type-args]). Positional: a define's generics line up with its
    /// external's, which is what pairs the two declarations.
    fn define_type_args(&mut self, name: &str, def: &'p DefineFn, span: Span) -> Vec<String> {
        if def.sig.generics.is_empty() {
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
                // Only reachable through a checker gap: [call-type-args]
                // rejects a call whose type arguments nothing determines,
                // so rendering a guess here would be silently wrong code
                // [backend-never-wrong].
                self.error(format!(
                    "call to `{name}` has an unresolved type argument, which its \
                     `define fn` template needs"
                ));
                out.push("()".to_string());
                continue;
            }
            out.push(self.rust_ty(ty));
        }
        out
    }

    /// A call to a declared function: effect handlers thread as leading
    /// `&mut` arguments [rs-effects]; parameter modes come from the
    /// deductions [rs-borrows].
    fn emit_fn_call(
        &mut self,
        name: &str,
        f: &FnDecl,
        key: Option<salvo_core::FnKey>,
        args: &[&Expr],
        span: Span,
    ) -> String {
        let mut all: Vec<String> = Vec::new();
        match self
            .checked
            .call_effects
            .get(&(self.file_idx, span))
            .cloned()
        {
            Some(effs) if effs.iter().all(ty_is_concrete) => {
                for ty in &effs {
                    // [rs-abort-controlflow] Aborting is a return shape,
                    // not a capability the caller hands over.
                    if is_abort_effect_ty(ty) {
                        continue;
                    }
                    all.push(self.thread_effect_by_ty(ty));
                }
            }
            _ => {
                for eff in f.effects.iter().flatten() {
                    if let EffectRef::Effect(r) = eff {
                        if r.name.name == salvo_core::ABORT_EFFECT {
                            continue;
                        }
                        let ty = self.emit_type_ref(r);
                        all.push(self.thread_effect_by_key(&ty));
                    }
                }
            }
        }
        // [rs-effect-fusion] The callee takes *one* fused value, whatever
        // the size of its effect set: every effect in scope threads through
        // the same value, so the per-effect arguments collapse to one. A
        // Sized fused value unsizes to the callee's `dyn` when the callee
        // needs fewer effects.
        if self.fusion {
            all.dedup();
            all.truncate(1);
        }
        let (prelude, args) = {
            let rendered = self.emit_args_for_params(&f.params, args, key);
            if all.is_empty() {
                (Vec::new(), rendered)
            } else {
                let threaded = all.clone();
                self.hoist_effect_args(Some(&threaded), rendered)
            }
        };
        all.extend(args);
        // A call through an import alias keeps the alias [rs-imports].
        let rs_name = if name != f.name.name {
            rs_ident(name)
        } else {
            self.rust_fn_name(f)
        };
        Self::wrap_hoisted(&prelude, format!("{rs_name}({})", all.join(", ")))
    }

    // ================= effect environment =================

    /// [rs-effect-fusion] The variable every effect in scope threads
    /// through under the fusion: the current scope's fusion local, or the
    /// enclosing fn's fused parameter.
    fn fused_var(&self) -> Option<(String, bool)> {
        if !self.fusion {
            return None;
        }
        self.effect_env
            .last()
            .map(|e| (e.var.clone(), e.is_local))
    }

    /// The receiver expression for the fused value: a fresh reborrow, so
    /// the value stays usable afterwards.
    fn fused_recv(&self) -> Option<String> {
        self.fused_var().map(|(var, is_local)| {
            if is_local {
                format!("&mut {var}")
            } else {
                format!("&mut *{var}")
            }
        })
    }

    /// [rs-effect-fusion] An argument that itself reaches the fused value
    /// must be evaluated *before* the call borrows it: under the fusion one
    /// value carries every effect, so `fx.a(&fx.b())` is two overlapping
    /// `&mut` (`E0499`) where the per-effect parameters were disjoint.
    fn hoist_fused_args(&mut self, args: Vec<String>) -> (Vec<String>, Vec<String>) {
        self.hoist_effect_args(None, args)
    }

    /// [rs-effects] Hoists the arguments that reach an effect value *this
    /// call threads* into a prelude, so an effectful call inside an
    /// effectful call's arguments does not borrow the same `&mut dyn`
    /// parameter twice (`E0499`) — a shape Kotlin accepts and Rust rejects.
    /// `threaded` is the call's own effect-argument code; `None` means every
    /// effect in scope (the fusion's single value, or a constructor whose
    /// arguments reach the provider).
    fn hoist_effect_args(
        &mut self,
        threaded: Option<&[String]>,
        args: Vec<String>,
    ) -> (Vec<String>, Vec<String>) {
        let mut vars: Vec<String> = match self.fused_var() {
            Some((var, _)) => vec![var],
            None => Vec::new(),
        };
        for entry in &self.effect_env {
            let used = match threaded {
                // The effect value is threaded by this very call.
                Some(codes) => codes.iter().any(|c| mentions_ident(c, &entry.var)),
                None => true,
            };
            if used && !vars.contains(&entry.var) {
                vars.push(entry.var.clone());
            }
        }
        if vars.is_empty() {
            return (Vec::new(), args);
        }
        let mut prelude: Vec<String> = Vec::new();
        let mut out: Vec<String> = Vec::new();
        for code in args {
            if vars.iter().any(|var| mentions_ident(&code, var)) {
                self.hoist_id += 1;
                let name = format!("__a{}", self.hoist_id);
                prelude.push(format!("let {name} = {code};"));
                out.push(name);
            } else {
                out.push(code);
            }
        }
        (prelude, out)
    }

    /// Wraps a call whose arguments were hoisted in a block expression, so
    /// it stays usable in expression position.
    fn wrap_hoisted(prelude: &[String], call: String) -> String {
        if prelude.is_empty() {
            call
        } else {
            format!("{{ {} {call} }}", prelude.join(" "))
        }
    }

    /// The expression a member call dispatches through for an effect
    /// instance (local handler variables and `&mut dyn` parameters both
    /// auto-reborrow on method calls) [rs-effects]. Under the fusion,
    /// dispatch is UFCS, so the receiver is spelled out as an explicit
    /// reborrow [rs-effect-fusion].
    fn member_dispatch_by_ty(&mut self, ty: &Ty) -> String {
        match self.effect_entry_by_ty(ty) {
            Some(entry) => self.entry_recv(&entry),
            None => {
                self.error(format!(
                    "no handler for effect `{ty}` in scope (declare it in the \
                     function's effect list or `use` a handler)"
                ));
                "todo!()".to_string()
            }
        }
    }

    /// The receiver/threading expression for one environment entry.
    fn entry_recv(&self, entry: &EffectEntry) -> String {
        if !self.fusion {
            return entry.var.clone();
        }
        if entry.is_local {
            format!("&mut {}", entry.var)
        } else {
            format!("&mut *{}", entry.var)
        }
    }

    /// Threading variant of [`Self::member_dispatch_by_ty`].
    fn thread_effect_by_ty(&mut self, ty: &Ty) -> String {
        match self.effect_entry_by_ty(ty) {
            Some(entry) => {
                if self.fusion {
                    self.entry_recv(&entry)
                } else if entry.is_local {
                    format!("&mut {}", entry.var)
                } else {
                    entry.var
                }
            }
            None => {
                self.error(format!(
                    "no handler for effect `{ty}` in scope (declare it in the \
                     function's effect list or `use` a handler)"
                ));
                "todo!()".to_string()
            }
        }
    }

    fn member_dispatch_by_key(&mut self, effect_ty: &str) -> String {
        match self.effect_entry(effect_ty) {
            Some(entry) => self.entry_recv(&entry),
            None => {
                self.error(format!(
                    "no handler for effect `{effect_ty}` in scope (declare it in the \
                     function's effect list or `use` a handler)"
                ));
                "todo!()".to_string()
            }
        }
    }

    /// The expression that threads an effect instance as a `&mut dyn`
    /// argument: `&mut local` for `use` locals, the parameter itself
    /// (implicit reborrow) otherwise [rs-effects].
    fn thread_effect_by_key(&mut self, effect_ty: &str) -> String {
        match self.effect_entry(effect_ty) {
            Some(entry) => {
                if self.fusion {
                    self.entry_recv(&entry)
                } else if entry.is_local {
                    format!("&mut {}", entry.var)
                } else {
                    entry.var
                }
            }
            None => {
                self.error(format!(
                    "no handler for effect `{effect_ty}` in scope (declare it in the \
                     function's effect list or `use` a handler)"
                ));
                "todo!()".to_string()
            }
        }
    }

    /// Exact-key lookup, then a unique same-base-name fallback (generic
    /// callee effects like `Random<T>` against a concrete `Random<i32>`).
    fn effect_entry(&self, effect_ty: &str) -> Option<EffectEntry> {
        // Innermost first, as in `effect_entry_by_ty` [fn-effects].
        if let Some(entry) = self.effect_env.iter().rev().find(|e| e.key == effect_ty) {
            return Some(entry.clone());
        }
        let base = effect_ty.split('<').next().unwrap_or(effect_ty);
        let matches: Vec<&EffectEntry> = self
            .effect_env
            .iter()
            .rev()
            .filter(|e| e.key.split('<').next().unwrap_or(&e.key) == base)
            .collect();
        if matches.len() == 1 {
            Some(matches[0].clone())
        } else if matches
            .first()
            .is_some_and(|first| matches.iter().all(|e| e.key == first.key))
        {
            // The same effect twice: an inner scope (a `use`, or a lambda's
            // effect parameter [fn-effects]) shadowing an outer entry. The
            // innermost wins; genuine ambiguity — *different* instances of a
            // generic effect — stays an error [effect-disambiguation].
            matches.first().copied().cloned()
        } else {
            None
        }
    }

    /// Lookup by the *checker's* effect type — the primary,
    /// rendering-drift-immune path; falls back to the rendered key for
    /// entries that only exist as AST renderings.
    fn effect_entry_by_ty(&mut self, ty: &Ty) -> Option<EffectEntry> {
        // Innermost first: a lambda's own effect *parameters* shadow the
        // enclosing fn's, which is what keeps the closure from capturing
        // them [fn-effects].
        if let Some(entry) = self
            .effect_env
            .iter()
            .rev()
            .find(|e| e.ty.as_ref() == Some(ty))
        {
            return Some(entry.clone());
        }
        let rendered = self.rust_ty(ty);
        self.effect_entry(&rendered)
    }

    /// Fallback dispatch for unchecked effect-member calls: unique
    /// base-name match (with explicit type args when given).
    fn member_dispatch_fallback(&mut self, effect: &str, type_args: &[Type]) -> String {
        let key = self.member_dispatch_key(effect, type_args);
        self.member_dispatch_by_key(&key)
    }

    /// The environment key an unchecked effect-member call looks up.
    fn member_dispatch_key(&mut self, effect: &str, type_args: &[Type]) -> String {
        if type_args.is_empty() {
            return effect.to_string();
        }
        let args: Vec<String> = type_args.iter().map(|t| self.emit_type(t)).collect();
        format!("{effect}<{}>", args.join(", "))
    }

    /// The effect base name and rendered type arguments of a checker `Ty`.
    fn ty_effect_parts(&mut self, ty: &Ty) -> (String, Vec<String>) {
        match ty {
            Ty::Named { name, args } => {
                let name = name.clone();
                let args = args.clone();
                let rendered: Vec<String> = args.iter().map(|a| self.rust_ty(a)).collect();
                (name, rendered)
            }
            other => {
                let rendered = self.rust_ty(other);
                split_rendered_generic(&rendered)
            }
        }
    }

    // ================= templates =================

    fn add_template_imports(&mut self, template: &Template) {
        // [backend-define-imports] hoisted and deduped per generated file.
        let text: String = template
            .parts
            .iter()
            .map(|p| match p {
                TemplatePart::Text(t) => t.as_str(),
                _ => "",
            })
            .collect();
        for line in text.lines() {
            let line = line.trim();
            if !line.is_empty() {
                self.imports.insert(line.to_string());
            }
        }
    }

    /// Expands a `define fn` inline template [backend-define-inline].
    fn expand_template(
        &mut self,
        template: &Template,
        params: &[Param],
        args: &[String],
        generics: &[Ident],
        type_args: &[String],
    ) -> String {
        let mut out = String::new();
        for part in &template.parts {
            match part {
                TemplatePart::Text(t) => out.push_str(t),
                TemplatePart::Interp(name) => {
                    match params.iter().position(|p| p.name.name == name.name) {
                        Some(idx) if idx < args.len() => out.push_str(&args[idx]),
                        // [backend-define-generics] A name that is not a
                        // parameter may be one of the define's *type*
                        // parameters, interpolated like `define type` does.
                        _ => match generics.iter().position(|g| g.name == name.name) {
                            Some(gi) if gi < type_args.len() => {
                                out.push_str(&type_args[gi])
                            }
                            _ => {
                                self.error(format!(
                                    "template refers to unknown or missing parameter \
                                     or type parameter `${{{}}}`",
                                    name.name
                                ));
                                out.push_str("todo!()");
                            }
                        },
                    }
                }
                TemplatePart::InterpVariadic(name) => {
                    match params.iter().position(|p| p.name.name == name.name) {
                        Some(idx) => {
                            let rest = args.get(idx..).unwrap_or(&[]);
                            out.push_str(&rest.join(", "));
                        }
                        None => {
                            self.error(format!(
                                "template refers to unknown variadic parameter `${{...{}}}`",
                                name.name
                            ));
                        }
                    }
                }
            }
        }
        out
    }

    /// Expands a `define type` inline template [backend-define-type].
    fn expand_type_template(
        &mut self,
        template: &Template,
        generics: &[Ident],
        args: &[String],
    ) -> String {
        let mut out = String::new();
        for part in &template.parts {
            match part {
                TemplatePart::Text(t) => out.push_str(t),
                TemplatePart::Interp(name) => {
                    match generics.iter().position(|g| g.name == name.name) {
                        Some(idx) if idx < args.len() => out.push_str(&args[idx]),
                        _ => {
                            self.error(format!(
                                "type template refers to unknown generic `${{{}}}`",
                                name.name
                            ));
                            out.push_str("()");
                        }
                    }
                }
                TemplatePart::InterpVariadic(name) => {
                    self.error(format!(
                        "variadic interpolation `${{...{}}}` is not valid in a type template",
                        name.name
                    ));
                }
            }
        }
        out.trim().to_string()
    }
}

// ================= helpers =================

/// Wraps a condition in parens when it starts with a block (Rust parses
/// `while { .. } < x` ambiguously) [rs-postincrement].
fn cond_code(code: String) -> String {
    if code.starts_with('(') || code.starts_with('{') {
        format!("({code})")
    } else {
        code
    }
}

/// Whether an AST type is a fn type (possibly under qualifiers like
/// `Once`): such types render as `impl Fn…`, which Rust only allows in
/// parameter/return position — never on a `let` binding [fn-contract].
fn is_fn_type(ty: &Type) -> bool {
    match ty {
        Type::Fn { .. } => true,
        Type::QualifiedGroup { base, .. } => is_fn_type(base),
        _ => false,
    }
}

/// Operator precedence for parenthesization (higher binds tighter).
fn bin_prec(op: BinaryOp) -> u8 {
    match op {
        BinaryOp::Or => 1,
        BinaryOp::And => 2,
        BinaryOp::Eq
        | BinaryOp::NotEq
        | BinaryOp::Lt
        | BinaryOp::Gt
        | BinaryOp::LtEq
        | BinaryOp::GtEq => 3,
        BinaryOp::Add | BinaryOp::Sub => 4,
        BinaryOp::Mul | BinaryOp::Div | BinaryOp::Rem => 5,
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

/// The base type name of an AST type (pairs define templates with
/// overloaded external declarations).
/// The (kept, mutable) contract of one fn-type parameter [fn-contract]:
/// kept unless the written deduction list omits its name; mutable when
/// the declared type carries `Mut`. Defaults keep everything.
fn ast_fn_param_contract(
    params: &[Type],
    param_names: &[Option<salvo_syntax::ast::Ident>],
    deductions: &Option<Vec<salvo_syntax::ast::Deduction>>,
    i: usize,
) -> (bool, bool) {
    let is_mut = params
        .get(i)
        .map(|t| match t {
            Type::Named { qualifiers, .. } | Type::QualifiedGroup { qualifiers, .. } => {
                qualifiers.iter().any(|q| q.name.name == "Mut")
            }
            _ => false,
        })
        .unwrap_or(false);
    let kept = match (param_names.get(i).and_then(|n| n.as_ref()), deductions) {
        (Some(name), Some(list)) => list.iter().any(|d| d.param.name == name.name),
        _ => true,
    };
    (kept, is_mut)
}

/// A qualified group over a fn type (`Once (A) -> B`) [once-fn].
fn is_fn_group(ty: &Type) -> bool {
    matches!(
        ty,
        Type::QualifiedGroup { base, .. } if matches!(base.as_ref(), Type::Fn { .. })
    )
}

fn type_base_name(ty: &Type) -> Option<&str> {
    match ty {
        Type::Named { base, .. } => Some(base.name.name.as_str()),
        Type::Nullable { inner, .. } => type_base_name(inner),
        Type::QualifiedGroup { base, .. } => type_base_name(base),
        Type::Array { .. } => Some("[]"),
        _ => None,
    }
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

/// Whether an AST type carries the `Mut` qualifier [rs-borrows].
fn type_has_mut(ty: &Type) -> bool {
    match ty {
        Type::Named { qualifiers, .. } | Type::QualifiedGroup { qualifiers, .. } => {
            qualifiers.iter().any(|q| q.name.name == "Mut")
        }
        Type::Nullable { inner, .. } => type_has_mut(inner),
        _ => false,
    }
}

/// True when a checker type contains no `Unknown`.
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

/// A parameter-ish name derived from a rendered effect type
/// (`Random<i32>` -> `random_i32`).
fn effect_param_name(effect_ty: &str) -> String {
    let mut out = String::new();
    for c in effect_ty.chars() {
        match c {
            '<' | ',' => out.push('_'),
            '>' | ' ' | '?' | '&' => {}
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

/// A rendered type turned into an identifier fragment for a generated
/// name (`Random<i32>` -> `Random_i32`) [rs-effect-fusion].
fn sanitize_ident(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        if c.is_ascii_alphanumeric() || c == '_' {
            out.push(c);
        } else if !out.ends_with('_') {
            out.push('_');
        }
    }
    out.trim_matches('_').to_string()
}

/// Splits a rendered generic type into its base and top-level arguments
/// (`Random<List<i32>, u8>` -> `("Random", ["List<i32>", "u8"])`). The
/// unchecked fallback for effect environment entries with no checker `Ty`.
fn split_rendered_generic(rendered: &str) -> (String, Vec<String>) {
    let Some(open) = rendered.find('<') else {
        return (rendered.to_string(), Vec::new());
    };
    let base = rendered[..open].to_string();
    let inner = rendered[open + 1..]
        .strip_suffix('>')
        .unwrap_or(&rendered[open + 1..]);
    let mut args: Vec<String> = Vec::new();
    let mut depth = 0usize;
    let mut current = String::new();
    for c in inner.chars() {
        match c {
            '<' => {
                depth += 1;
                current.push(c);
            }
            '>' => {
                depth = depth.saturating_sub(1);
                current.push(c);
            }
            ',' if depth == 0 => {
                args.push(current.trim().to_string());
                current = String::new();
            }
            c => current.push(c),
        }
    }
    if !current.trim().is_empty() {
        args.push(current.trim().to_string());
    }
    (base, args)
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
            c => out.push(c),
        }
    }
    out
}

/// `format!` literal text additionally escapes braces.
fn escape_format_text(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        match c {
            '{' => out.push_str("{{"),
            '}' => out.push_str("}}"),
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
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

/// The qualifier names on a fn's parameter types, joined for
/// overload-mangling suffixes [rs-fn-mangling].
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

/// Substitutes generic parameters in an AST type (alias expansion).
fn subst_ast_type(ty: &Type, map: &HashMap<&str, &Type>) -> Type {
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

/// Names assigned or incremented anywhere in the block (reassigned
/// parameters need a `mut` binder [rs-borrows]).
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
            | Stmt::Yield { value: v, .. } => collect_mutated_expr(v, out),
            Stmt::Use { handler, .. } => collect_mutated_expr(handler, out),
            Stmt::Expr(e) => collect_mutated_expr(e, out),
            _ => {}
        }
    }
}

fn collect_mutated_expr(expr: &Expr, out: &mut HashSet<String>) {
    match expr {
        Expr::PostIncrement { operand, .. } => {
            if let Expr::Ident(id) = operand.as_ref() {
                out.insert(id.name.clone());
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
        Expr::Is { subject, .. } => collect_mutated_expr(subject, out),
        Expr::Str { parts, .. } => {
            for p in parts {
                if let StrExprPart::Interp(e) = p {
                    collect_mutated_expr(e, out);
                }
            }
        }
        Expr::ArrayLit { elems, .. } | Expr::Tuple { elems, .. } => {
            for e in elems {
                collect_mutated_expr(e, out);
            }
        }
        Expr::ArrayInit { size, init, .. } => {
            collect_mutated_expr(size, out);
            collect_mutated_expr(init, out);
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
        // [qual-widen] The check reads its subject.
        Expr::Widen { subject, .. } => collect_mutated_expr(subject, out),
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

/// Every name a block binds (generated effect-parameter and `use`
/// variable names must avoid them).
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
            | Stmt::Yield { value: v, .. } => collect_declared_expr(v, out),
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
        // [when-condition]
        Expr::WhenCond {
            branches,
            else_block,
            ..
        } => {
            for (c, b) in branches {
                collect_declared_expr(c, out);
                collect_declared(b, out);
            }
            collect_declared(else_block, out);
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
        | Expr::PostIncrement { operand, .. }
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
        Expr::ArrayLit { elems, .. } | Expr::Tuple { elems, .. } => {
            for e in elems {
                collect_declared_expr(e, out);
            }
        }
        Expr::ArrayInit { size, init, .. } => {
            collect_declared_expr(size, out);
            collect_declared_expr(init, out);
        }
        Expr::StructLit { fields, .. } => {
            for f in fields {
                match &f.kind {
                    StructLitFieldKind::Named { value, .. } => collect_declared_expr(value, out),
                    StructLitFieldKind::Spread(e) => collect_declared_expr(e, out),
                }
            }
        }
        // [qual-widen] No binding of its own — the subject reads widened —
        // but the subject expression may declare one.
        Expr::Widen { subject, .. } => collect_declared_expr(subject, out),
        // [try] The delimiter's body is ordinary code and declares its own
        // locals.
        Expr::Try { body, .. } => collect_declared(body, out),
        // Leaves: nothing declared inside. Listed rather than defaulted so
        // a new binding form cannot escape the name census that keeps
        // generated locals from colliding with user names.
        Expr::Ident(_)
        | Expr::Int { .. }
        | Expr::Float { .. }
        | Expr::Bool { .. }
        | Expr::Char { .. }
        | Expr::Error { .. } => {}
    }
}

/// [qual-widen] Visits every `^` check a condition applies, including
/// through `&&` chains and a `!`-free `||` (the same shape `is` bindings
/// walk).
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

/// Whether an effect instance is the abort effect [abort]: it is not a
/// capability parameter, it is a return-shape change
/// [rs-abort-controlflow].
fn is_abort_effect_ty(ty: &Ty) -> bool {
    matches!(ty, Ty::Named { name, .. } if name == salvo_core::ABORT_EFFECT)
}

/// The message type of an `Abort<M>` effect instance [abort].
fn abort_message_of(ty: &Ty) -> Option<Ty> {
    match ty {
        Ty::Named { name, args } if name == salvo_core::ABORT_EFFECT => {
            Some(args.first().cloned().unwrap_or(Ty::Unknown))
        }
        _ => None,
    }
}

/// [rs-defer-splice] Whether the block's own statements always leave it/// (`return`/`break`/`continue`, or a branching construct all of whose
/// branches do). Deferred blocks were already spliced at those exits, so
/// splicing them again at the block's end would be dead code — and, for a
/// body that gave a value away, dead code rustc still borrow-checks.
fn block_terminates(block: &Block) -> bool {
    block.stmts.iter().any(|stmt| match stmt {
        Stmt::Return { .. } | Stmt::Break { .. } | Stmt::Continue { .. } => true,
        Stmt::Expr(e) => expr_terminates(e),
        _ => false,
    })
}

fn expr_terminates(expr: &Expr) -> bool {
    match expr {
        Expr::If {
            branches,
            else_block,
            ..
        } => {
            else_block.as_ref().is_some_and(block_terminates)
                && branches.iter().all(|(_, b)| block_terminates(b))
        }
        Expr::When { branches, .. } => {
            !branches.is_empty() && branches.iter().all(|b| block_terminates(&b.body))
        }
        // [when-condition] Total by construction: the `else` is mandatory.
        Expr::WhenCond {
            branches,
            else_block,
            ..
        } => {
            block_terminates(else_block)
                && branches.iter().all(|(_, b)| block_terminates(b))
        }
        // Nothing else terminates the enclosing Rust block by itself.
        // Listed rather than defaulted: a missed control-flow form would
        // get unreachable code emitted after it, which rustc rejects.
        Expr::While { .. }
        | Expr::For { .. }
        | Expr::Lambda { .. }
        | Expr::Try { .. }
        | Expr::Int { .. }
        | Expr::Float { .. }
        | Expr::Bool { .. }
        | Expr::Char { .. }
        | Expr::Str { .. }
        | Expr::Ident(_)
        | Expr::Field { .. }
        | Expr::TupleIndex { .. }
        | Expr::Call { .. }
        | Expr::Index { .. }
        | Expr::ArrayLit { .. }
        | Expr::ArrayInit { .. }
        | Expr::Tuple { .. }
        | Expr::StructLit { .. }
        | Expr::Unary { .. }
        | Expr::Binary { .. }
        | Expr::Is { .. }
        | Expr::Widen { .. }
        | Expr::NonNull { .. }
        | Expr::PostIncrement { .. }
        | Expr::Spread { .. }
        | Expr::Error { .. } => false,
    }
}

fn contains_yield(block: &Block) -> bool {
    block.stmts.iter().any(|stmt| match stmt {
        Stmt::Yield { .. } => true,
        Stmt::Expr(e) => expr_contains_yield(e),
        Stmt::Let { value, .. } => expr_contains_yield(value),
        _ => false,
    })
}

fn expr_contains_yield(expr: &Expr) -> bool {
    match expr {
        Expr::If {
            branches,
            else_block,
            ..
        } => {
            branches.iter().any(|(_, b)| contains_yield(b))
                || else_block.as_ref().is_some_and(contains_yield)
        }
        Expr::While {
            body, else_block, ..
        }
        | Expr::For {
            body, else_block, ..
        } => contains_yield(body) || else_block.as_ref().is_some_and(contains_yield),
        Expr::When { branches, .. } => branches.iter().any(|b| contains_yield(&b.body)),
        _ => false,
    }
}

/// Walks a condition for `is`-checks with bindings.
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
