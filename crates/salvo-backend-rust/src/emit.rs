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

use salvo_core::check::{Checked, Coercion, UnionTest};
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

    // The module declaring `fn main` becomes the crate root [rs-crate].
    let root_module: Option<&ModulePath> = program
        .units()
        .find(|u| {
            u.file.kind == SourceKind::Language
                && emitted_modules.contains(&u.file.module)
                && u.ast.items.iter().any(|item| {
                    matches!(item, Item::Fn(f) if f.name.name == "main" && f.body.is_some())
                })
        })
        .map(|u| &u.file.module);

    // [backend-external] Everything external in `core.*` must be covered
    // by the backend's define files (core is implicitly imported).
    let mut errors = check_core_define_coverage(program, &symbols);
    // [decl-explicit] Every define implements exactly one external:
    // the external carries the contract, the define the template.
    errors.extend(salvo_core::check_define_pairing(program, &symbols, "rust"));

    let mut files = Vec::new();
    let mut union_sizes: BTreeSet<usize> = BTreeSet::new();
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

fn rs_ident(name: &str) -> String {
    if RUST_KEYWORDS.contains(&name) {
        format!("r#{name}")
    } else if RUST_UNRAW.contains(&name) {
        format!("{name}_")
    } else {
        name.to_string()
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
        if h.backing == Some(BackingMod::External) {
            return self.emit_define_handler(h);
        }
        let saved = self.enter_generics(&h.generics);
        let generics = self.emit_generic_params(&h.generics);
        let generic_args = self.emit_generic_args_plain(&h.generics);
        let of = self.emit_type(&h.of);
        let name = rs_ident(&h.name.name);

        // Struct: ctor params + state fields.
        let mut out = format!("\npub struct {name}{generics} {{\n");
        for p in &h.params {
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
            "    pub fn new({}) -> Self {{\n        Self {{\n",
            ctor_params.join(", ")
        ));
        for p in &h.params {
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

        // Trait impl with the member bodies.
        out.push_str(&format!("\nimpl{generics} {of} for {name}{generic_args} {{\n"));
        for f in &h.fns {
            out.push_str(&self.emit_fn_inner(f, FnStyle::HandlerMember(h), 1));
        }
        out.push_str("}\n");
        self.generics = saved;
        out
    }

    /// An `external handler` implemented by `define handler` templates
    /// [backend-define-handler]: struct + `new()` + trait impl whose
    /// method bodies inline the templates.
    fn emit_define_handler(&mut self, h: &HandlerDecl) -> String {
        let Some(def) = self.symbols.define_handlers.get(h.name.name.as_str()) else {
            self.error(format!(
                "external handler `{}` has no rust `define handler`",
                h.name.name
            ));
            return String::new();
        };
        let of = self.emit_type(&h.of);
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
        for dfn in &def.fns {
            if let Some(imports) = &dfn.body.imports {
                self.add_template_imports(imports);
            }
            let params = self.emit_member_param_list(&dfn.sig.params);
            let ret = self.emit_return_type(dfn.sig.return_type.as_ref());
            let Some(inline) = &dfn.body.inline else {
                self.error(format!(
                    "define fn `{}` in handler `{}` has no inline section",
                    dfn.sig.name.name, h.name.name
                ));
                continue;
            };
            let args: Vec<String> = dfn
                .sig
                .params
                .iter()
                .map(|p| rs_ident(&p.name.name))
                .collect();
            let body = self.expand_template(inline, &dfn.sig.params, &args);
            out.push_str(&format!(
                "    fn {}(&mut self{params}){ret} {{\n",
                rs_ident(&dfn.sig.name.name)
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

        // Handler members see ctor params and state as `self.` fields.
        if let FnStyle::HandlerMember(h) = style {
            for p in &h.params {
                self.bindings.insert(p.name.name.clone(), BindKind::SelfField);
            }
            for field in &h.state {
                self.bindings
                    .insert(field.name.name.clone(), BindKind::SelfField);
            }
        }

        let generics = self.emit_generic_params(&f.generics);
        let mut params: Vec<String> = Vec::new();
        if matches!(style, FnStyle::HandlerMember(_)) {
            params.push("&mut self".to_string());
        }
        // Effect dependencies become leading `&mut dyn` parameters
        // [rs-effects], sourced from the checker's lowered effect list
        // when available (checker-`Ty` keys; the AST rendering is the
        // unchecked fallback).
        if !is_main {
            let checked_effects: Option<Vec<Ty>> = self
                .checked
                .fn_refs
                .get(&(self.file_idx, f.name.span))
                .and_then(|key| self.checked.fn_effects.get(key))
                .cloned();
            match checked_effects {
                Some(tys) => {
                    for ty in tys {
                        let rendered = self.rust_ty(&ty);
                        let param = self.unique_name(effect_param_name(&rendered));
                        self.effect_env.push(EffectEntry {
                            ty: Some(ty),
                            key: rendered.clone(),
                            var: param.clone(),
                            is_local: false,
                        });
                        self.bindings.insert(param.clone(), BindKind::RefMut);
                        params.push(format!("{param}: &mut dyn {rendered}"));
                    }
                }
                None => {
                    for eff in f.effects.iter().flatten() {
                        if let EffectRef::Effect(r) = eff {
                            let ty = self.emit_type_ref(r);
                            let param = self.unique_name(effect_param_name(&ty));
                            self.effect_env.push(EffectEntry {
                                ty: None,
                                key: ty.clone(),
                                var: param.clone(),
                                is_local: false,
                            });
                            self.bindings.insert(param.clone(), BindKind::RefMut);
                            params.push(format!("{param}: &mut dyn {ty}"));
                        }
                    }
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

        let pad = "    ".repeat(indent);
        let name = if is_main {
            "main".to_string()
        } else if top_level || matches!(style, FnStyle::QualifierFn) {
            self.rust_fn_name(f)
        } else {
            rs_ident(&f.name.name)
        };
        let vis = if matches!(style, FnStyle::HandlerMember(_)) {
            ""
        } else {
            "pub "
        };
        let generics = if lifetime_generics.is_empty() {
            generics
        } else if generics.is_empty() {
            format!("<{lifetime_generics}>")
        } else {
            // `<T>` -> `<'a, T>`
            format!("<{lifetime_generics}, {}", &generics[1..])
        };
        let mut out = format!(
            "\n{pad}{vis}fn {name}{generics}({}){ret} {{\n",
            params.join(", ")
        );

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
            out.push_str(&format!("{pad}    return __yielded;\n"));
        } else {
            out.push_str(&self.emit_block_stmts(body, indent + 1, StmtCtx::Normal));
        }
        out.push_str(&format!("{pad}}}\n"));

        self.generics = saved_generics;
        self.effect_env = saved_env;
        self.bindings = saved_bindings;
        self.mutated = saved_mutated;
        self.derived_return_fn = saved_derived;
        self.taken_names = saved_taken;
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
                format!("impl FnMut({}){ret}", ps.join(", "))
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
                    if let Type::Fn { params, ret, .. } = base.as_ref() {
                        let ps: Vec<String> =
                            params.iter().map(|p| self.emit_type(p)).collect();
                        let ret = match ret.as_ref() {
                            Type::Named { base, .. } if base.name.name == "None" => {
                                String::new()
                            }
                            other => format!(" -> {}", self.emit_type(other)),
                        };
                        return format!("impl FnOnce({}){ret}", ps.join(", "));
                    }
                }
                self.emit_type(base)
            }
        }
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

    /// Maps a named type to Rust: internal types [backend-internal],
    /// `define type` templates [backend-define-type], or pass-through.
    fn emit_named_parts(&mut self, name: &str, arg_strs: &[String]) -> String {
        let args = if arg_strs.is_empty() {
            String::new()
        } else {
            format!("<{}>", arg_strs.join(", "))
        };
        let internal = match name {
            "Str" => Some("String"),
            "Int" => Some("i32"),
            "Long" => Some("i64"),
            "Float" => Some("f32"),
            "Double" => Some("f64"),
            "Bool" => Some("bool"),
            "Char" => Some("char"),
            "Byte" => Some("u8"),
            "None" => Some("()"),
            "Nothing" => Some("()"), // only reachable in dead positions
            "Iter" => Some("Vec"),   // [rs-iter-vec]
            _ => None,
        };
        if let Some(rs) = internal {
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

/// What kind of fn is being emitted (parameter-mode and naming rules).
#[derive(Clone, Copy)]
enum FnStyle<'p> {
    TopLevel,
    QualifierFn,
    HandlerMember(&'p HandlerDecl),
}

impl<'p> Emitter<'p> {
    // ================= statements =================

    fn emit_block_stmts(&mut self, block: &Block, indent: usize, ctx: StmtCtx) -> String {
        let mut out = String::new();
        let env_depth = self.effect_env.len();
        for stmt in &block.stmts {
            out.push_str(&self.emit_stmt(stmt, indent, ctx));
        }
        self.effect_env.truncate(env_depth);
        out
    }

    fn emit_stmt(&mut self, stmt: &Stmt, indent: usize, ctx: StmtCtx) -> String {
        let pad = "    ".repeat(indent);
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
                    format!("{pad}return __yielded;\n")
                }
                (StmtCtx::IteratorBody, Some(_)) => {
                    self.error("`return` with a value is not allowed in an iterator function");
                    format!("{pad}return __yielded;\n")
                }
                (_, Some(v)) => {
                    // [readonly-return] Derived-return fns return
                    // borrows: the place is borrowed (bare for
                    // already-`&` bindings), `Some(...)`-wrapped when
                    // the checker recorded the optional coercion, and
                    // `None` passes through.
                    if self.derived_return_fn
                        && !matches!(v, Expr::Ident(id) if id.name == "None")
                    {
                        // A forwarded derived-return call already
                        // produces a borrow: pass it through.
                        let forwarded = matches!(
                            v,
                            Expr::Call { span, .. }
                                if self
                                    .checked
                                    .derived_calls
                                    .contains_key(&(self.file_idx, *span))
                        );
                        if forwarded {
                            let code = self.emit_expr(v);
                            return format!("{pad}return {code};\n");
                        }
                        let wrap = matches!(
                            self.coercion_of(v.span()),
                            Some(Coercion::WrapOption { .. })
                        );
                        if let Some(borrowed) = self.borrow_place(v) {
                            let code = if wrap {
                                format!("Some({borrowed})")
                            } else {
                                borrowed
                            };
                            return format!("{pad}return {code};\n");
                        }
                        self.error(format!(
                            "a `ReadOnly[from: ...]` return value must be a \
                             projection or alias of the annotated parameter \
                             (got `{v:?}`)"
                        ));
                    }
                    let v = self.emit_expr(v);
                    format!("{pad}return {v};\n")
                }
                (_, None) => format!("{pad}return;\n"),
            },
            Stmt::Break { value, .. } => {
                let target = self.loop_results.last().cloned().flatten();
                match (value, target) {
                    // [while-value] route the value into the enclosing
                    // loop's result local before breaking [rs-loop-value].
                    (Some(v), Some(result)) => {
                        if self.ty_of(v.span()).is_some_and(|t| t.is_none_ty()) {
                            let stmt = self.emit_expr_stmt(v, indent, ctx);
                            format!("{stmt}{pad}{result} = None;\n{pad}break;\n")
                        } else {
                            let code = self.emit_loop_value_assign(v, &result);
                            format!("{pad}{code}\n{pad}break;\n")
                        }
                    }
                    (Some(v), None) => {
                        let stmt = self.emit_expr_stmt(v, indent, ctx);
                        format!("{stmt}{pad}break;\n")
                    }
                    (None, _) => format!("{pad}break;\n"),
                }
            }
            Stmt::Continue { .. } => format!("{pad}continue;\n"),
            Stmt::Yield { value, .. } => {
                // [rs-iter-vec]
                let v = self.emit_expr(value);
                format!("{pad}__yielded.push({v});\n")
            }
            Stmt::Use { handler, span } => self.emit_use(handler, *span, indent),
            Stmt::Expr(expr) => self.emit_expr_stmt(expr, indent, ctx),
        }
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

    fn emit_expr_stmt(&mut self, expr: &Expr, indent: usize, ctx: StmtCtx) -> String {
        let pad = "    ".repeat(indent);
        match expr {
            Expr::If {
                branches,
                else_block,
                ..
            } => self.emit_if_stmt(branches, else_block.as_ref(), indent, ctx),
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
                out.push_str(&self.emit_block_stmts(body, indent + 1, ctx));
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
                out.push_str(&self.emit_block_stmts(body, indent + 1, ctx));
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
            out.push_str(&self.emit_block_stmts(block, indent + 1, ctx));
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

    /// Reads the narrowed payload out of a subject for an `is` binding:
    /// enum-arm accessor for wrapper unions, `Option` unwrap for `T?`
    /// representations [rs-union-enums] [rs-option].
    fn emit_narrowed_read(
        &mut self,
        subject: &Expr,
        target: Option<&Ty>,
        test: Option<UnionTest>,
    ) -> String {
        let subj = self.emit_place(subject);
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
        let (repr, logical) = (self.repr_of(id.span)?, self.ty_of(id.span)?);
        let name = self.binding_place(&id.name);
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
            Expr::Field { .. } | Expr::Index { .. } => {
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
            Expr::Field { .. } | Expr::Index { .. } => {
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
            Expr::Field { base, field, span } => {
                let base_code = self.emit_place(base);
                let code = format!("{base_code}.{}", rs_ident(&field.name));
                // [qual-field-override] cast-and-assert reads the value
                // out of the declared representation.
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
            Expr::Ident(_) | Expr::Field { .. } | Expr::Index { .. } => {
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
            } => self.emit_if_expr(branches, else_block.as_ref()),
            Expr::Lambda { params, body, span } => self.emit_lambda(params, body, *span),
            Expr::Spread { operand, .. } => self.emit_owned(operand),
            Expr::While { .. } | Expr::For { .. } => self.emit_loop_value(expr),
            Expr::When {
                subject, branches, ..
            } => self.emit_when(subject, branches, 0, StmtCtx::Normal, true),
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
            Expr::Is { .. } | Expr::Lambda { .. } => format!("({code})"),
            _ => code,
        }
    }

    /// Left operands of the same precedence stay flat (left-assoc).
    fn emit_operand_left(&mut self, expr: &Expr, parent_prec: u8) -> String {
        let code = self.emit_expr(expr);
        match expr {
            Expr::Binary { op, .. } if bin_prec(*op) < parent_prec => format!("({code})"),
            Expr::Is { .. } | Expr::Lambda { .. } => format!("({code})"),
            _ => code,
        }
    }

    /// An `is` check in expression position: the checker's tables decide
    /// the lowering [rs-union-enums]; for unchecked `T?` subjects
    /// (struct fields) a null test still works [rs-option]; anything
    /// else is a codegen error [backend-never-wrong].
    fn emit_is_check(&mut self, subject: &Expr, check: &[TypeRef], span: Span) -> String {
        if let Some(test) = self.is_test_of(span).cloned() {
            let subj = self.emit_place(subject);
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
        let subj = self.emit_place(subject);
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
            ..
        } = expected
        else {
            return None;
        };
        let rust_name = self.rust_fn_name(decl);
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
        Some(format!(
            "&mut |{}| {rust_name}({})",
            names
                .iter()
                .map(|p| format!("mut {p}"))
                .collect::<Vec<_>>()
                .join(", "),
            fwd.join(", ")
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
            Expr::Field { .. } | Expr::Index { .. } => {
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
            Expr::Field { .. } | Expr::Index { .. } => {
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
            Coercion::WrapUnion { target, arm } => {
                let value_arms = target.value_arms();
                let n = value_arms.len();
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
            Coercion::Rewrap { from, to } => self.emit_rewrap(code, &from, &to),
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
    /// (branch values carry their `WrapOption`/wrap coercions).
    fn emit_if_expr(&mut self, branches: &[(Expr, Block)], else_block: Option<&Block>) -> String {
        let mut out = String::new();
        for (i, (cond, block)) in branches.iter().enumerate() {
            let kw = if i == 0 { "if" } else { " else if" };
            let c = self.emit_expr(cond);
            out.push_str(&format!("{kw} {} {{\n", cond_code(c)));
            out.push_str(&self.emit_is_bindings(cond, 0));
            out.push_str(&self.emit_value_block(block));
            out.push('}');
        }
        match else_block {
            Some(block) => {
                out.push_str(" else {\n");
                out.push_str(&self.emit_value_block(block));
                out.push('}');
            }
            None => {
                out.push_str(" else {\nNone\n}");
            }
        }
        out
    }

    /// A block in value position: statements plus the trailing expression
    /// as the block's value.
    fn emit_value_block(&mut self, block: &Block) -> String {
        let mut out = String::new();
        let env_depth = self.effect_env.len();
        let n = block.stmts.len();
        for (i, stmt) in block.stmts.iter().enumerate() {
            if i + 1 == n {
                if let Stmt::Expr(e) = stmt {
                    // The tail is the value. `Nothing`-typed tails
                    // (`return`-like) stay statements.
                    if matches!(self.ty_of(e.span()), Some(Ty::Nothing)) {
                        out.push_str(&self.emit_expr_stmt(e, 0, StmtCtx::Normal));
                    } else {
                        let code = self.emit_expr(e);
                        out.push_str(&code);
                        out.push('\n');
                    }
                    continue;
                }
            }
            out.push_str(&self.emit_stmt(stmt, 0, StmtCtx::Normal));
        }
        self.effect_env.truncate(env_depth);
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
        let subj = self.emit_place(subject);
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
            if value_pos {
                out.push_str(&self.emit_value_block(&branch.body));
            } else {
                out.push_str(&self.emit_block_stmts(&branch.body, indent + 2, ctx));
            }
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
        out.push_str(&self.emit_loop_body_value(body, &result, join_optional));
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
        let env_depth = self.effect_env.len();
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
        self.effect_env.truncate(env_depth);
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
        let param_list: Vec<String> = params
            .iter()
            .enumerate()
            .map(|(i, p)| match &p.ty {
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
            })
            .collect();
        let out = match body {
            LambdaBody::Expr(expr) => {
                format!("|{}| {}", param_list.join(", "), self.emit_expr(expr))
            }
            LambdaBody::Block(block) => {
                // Closures return their last expression; a trailing
                // `return X` becomes the value.
                let mut out = format!("|{}| {{\n", param_list.join(", "));
                let n = block.stmts.len();
                for (i, stmt) in block.stmts.iter().enumerate() {
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
                    out.push_str(&self.emit_stmt(stmt, 1, StmtCtx::Normal));
                }
                out.push('}');
                out
            }
        };
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
                    e @ (Expr::Ident(_) | Expr::Field { .. } | Expr::Index { .. }) => {
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
            // Unknown method: pass through as a Rust method call
            // (companion-code interop [type-unknown-lenient]).
            let base_code = self.emit_place(base);
            let arg_code: Vec<String> = args.iter().map(|a| self.emit_expr(a)).collect();
            return format!("{base_code}.{}({})", rs_ident(name), arg_code.join(", "));
        }

        if let Expr::Ident(id) = callee {
            let arg_refs: Vec<&Expr> = args.iter().collect();
            return self.emit_resolved_call(&id.name, type_args, &arg_refs, span);
        }

        // Calling a computed value (lambda etc.): owned args [fn-lambda].
        let callee_code = self.emit_owned(callee);
        let arg_code: Vec<String> = args.iter().map(|a| self.emit_expr(a)).collect();
        format!("{callee_code}({})", arg_code.join(", "))
    }

    fn emit_resolved_call(
        &mut self,
        name: &str,
        type_args: &[Type],
        args: &[&Expr],
        span: Span,
    ) -> String {
        // 1. Effect member call: dispatch through the handler in scope
        // [rs-effects] ([effect-disambiguation], `effect_calls`).
        if let Some(effect) = self.symbols.effect_of_fn.get(name).copied() {
            let handler = match self.checked.effect_calls.get(&(self.file_idx, span)) {
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
            return format!("{handler}.{}({})", rs_ident(name), arg_code.join(", "));
        }

        // 2. Checker-resolved fn target (type-based overloads win)
        // [fn-overload]. Internal fns lower intrinsically [internal-fn];
        // external signatures route to their define.
        let checker_resolved = self
            .checked
            .call_fn
            .get(&(self.file_idx, span))
            .and_then(|key| self.fn_by_key(*key).map(|f| (*key, f)));
        if let Some((key, f)) = checker_resolved {
            if f.backing == Some(BackingMod::Internal) {
                return self.emit_internal_call(f, args);
            }
            if f.body.is_none() {
                if let Some(def) = self.define_for_decl(name, f) {
                    return self.emit_define_call(name, def, args);
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
                Some(def) => self.emit_define_call(name, def, args),
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
            if f.backing == Some(BackingMod::Internal) {
                return self.emit_internal_call(f, args);
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
        let generics = self.emit_call_type_args(type_args);
        format!("{}{generics}({})", rs_ident(name), arg_code.join(", "))
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

    /// A call to an `internal fn`, lowered directly by the compiler
    /// [internal-fn]. The only internal fn today is `copy` [copy-fn],
    /// lowered to `.clone()` on the argument's place [rs-copy]: reads
    /// never consume, and every generated type derives `Clone`.
    /// Non-place arguments are already fresh owned values and pass
    /// through.
    fn emit_internal_call(&mut self, f: &FnDecl, args: &[&Expr]) -> String {
        // [linear-discard] `discard(x)` moves the value into `drop`.
        if f.name.name == "discard" && args.len() == 1 {
            let code = self.emit_expr(args[0]);
            return format!("drop({code})");
        }
        if f.name.name != "copy" || args.len() != 1 {
            self.error(format!(
                "internal fn `{}` is not supported by the rust backend",
                f.name.name
            ));
            return "todo!()".to_string();
        }
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
            Expr::Field { .. } | Expr::Index { .. } => self.emit_owned(arg),
            other => self.emit_expr(other),
        }
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
    fn emit_define_call(&mut self, name: &str, def: &'p DefineFn, args: &[&Expr]) -> String {
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
                    Expr::Ident(_) | Expr::Field { .. } | Expr::Index { .. }
                        if !is_variadic_part =>
                    {
                        self.emit_place(arg)
                    }
                    Expr::Spread { operand, .. } => self.emit_owned(operand),
                    other => self.emit_expr(other),
                };
                arg_code.push(code);
            }
            return self
                .expand_template(&inline, &def.sig.params, &arg_code)
                .trim()
                .to_string();
        }
        self.error(format!("define fn `{name}` has no inline section"));
        "todo!()".to_string()
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
                    all.push(self.thread_effect_by_ty(ty));
                }
            }
            _ => {
                for eff in f.effects.iter().flatten() {
                    if let EffectRef::Effect(r) = eff {
                        let ty = self.emit_type_ref(r);
                        all.push(self.thread_effect_by_key(&ty));
                    }
                }
            }
        }
        all.extend(self.emit_args_for_params(&f.params, args, key));
        // A call through an import alias keeps the alias [rs-imports].
        let rs_name = if name != f.name.name {
            rs_ident(name)
        } else {
            self.rust_fn_name(f)
        };
        format!("{rs_name}({})", all.join(", "))
    }

    // ================= effect environment =================

    /// The expression a member call dispatches through for an effect
    /// instance (local handler variables and `&mut dyn` parameters both
    /// auto-reborrow on method calls) [rs-effects].
    fn member_dispatch_by_ty(&mut self, ty: &Ty) -> String {
        match self.effect_entry_by_ty(ty) {
            Some(entry) => entry.var,
            None => {
                self.error(format!(
                    "no handler for effect `{ty}` in scope (declare it in the \
                     function's effect list or `use` a handler)"
                ));
                "todo!()".to_string()
            }
        }
    }

    /// Threading variant of [`Self::member_dispatch_by_ty`].
    fn thread_effect_by_ty(&mut self, ty: &Ty) -> String {
        match self.effect_entry_by_ty(ty) {
            Some(entry) => {
                if entry.is_local {
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
            Some(entry) => entry.var,
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
                if entry.is_local {
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
        if let Some(entry) = self.effect_env.iter().find(|e| e.key == effect_ty) {
            return Some(entry.clone());
        }
        let base = effect_ty.split('<').next().unwrap_or(effect_ty);
        let matches: Vec<&EffectEntry> = self
            .effect_env
            .iter()
            .filter(|e| e.key.split('<').next().unwrap_or(&e.key) == base)
            .collect();
        if matches.len() == 1 {
            Some(matches[0].clone())
        } else {
            None
        }
    }

    /// Lookup by the *checker's* effect type — the primary,
    /// rendering-drift-immune path; falls back to the rendered key for
    /// entries that only exist as AST renderings.
    fn effect_entry_by_ty(&mut self, ty: &Ty) -> Option<EffectEntry> {
        if let Some(entry) = self
            .effect_env
            .iter()
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
        if !type_args.is_empty() {
            let args: Vec<String> = type_args.iter().map(|t| self.emit_type(t)).collect();
            let full = format!("{effect}<{}>", args.join(", "));
            return self.member_dispatch_by_key(&full);
        }
        self.member_dispatch_by_key(effect)
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
    ) -> String {
        let mut out = String::new();
        for part in &template.parts {
            match part {
                TemplatePart::Text(t) => out.push_str(t),
                TemplatePart::Interp(name) => {
                    match params.iter().position(|p| p.name.name == name.name) {
                        Some(idx) if idx < args.len() => out.push_str(&args[idx]),
                        _ => {
                            self.error(format!(
                                "template refers to unknown or missing parameter `${{{}}}`",
                                name.name
                            ));
                            out.push_str("todo!()");
                        }
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
                    parts.push(q.name.name.clone());
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
        Expr::Field { base, .. } => collect_mutated_expr(base, out),
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
        _ => {}
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
        Expr::Field { base, .. } => collect_declared_expr(base, out),
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
        _ => {}
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
