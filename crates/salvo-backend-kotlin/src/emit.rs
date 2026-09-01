//! The Kotlin code emitter.
//!
//! M2 scope: functions (with effects as leading parameters), structs (data
//! classes), effects (interfaces), handlers (classes/objects, including
//! `define handler` templates), `define fn`/`define type` inline expansion,
//! string interpolation, `if`/`is` with bindings, iterator functions
//! (`yield` -> Kotlin `iterator {}` builder), `while`/`for` statements.
//! M6 adds loops as values: `break value` and loop `else` lower through a
//! `run {}` block with a result local [while-value] [kt-loop-value].
//!
//! Not yet supported (reported as codegen errors, never silently wrong
//! code [backend-never-wrong]): tuples beyond Pair/Triple, multi-spread
//! struct literals.

use std::collections::{BTreeSet, HashSet};

use salvo_core::check::{Checked, Coercion, UnionTest};
use salvo_core::types::Ty;
use salvo_core::{ModulePath, Program, SourceKind, Symbols};
use salvo_syntax::ast::*;
use salvo_syntax::Span;

pub struct EmittedFile {
    /// Path relative to the target dir, e.g. `core/console.kt`.
    pub rel_path: std::path::PathBuf,
    pub content: String,
}

/// Emits Kotlin for every *reachable* module that produces code
/// [mod-used-only]. Returns the files or the accumulated codegen/type
/// errors.
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
    // [mod-used-only] Only modules the program uses are transpiled.
    let reachable = salvo_core::reachable_modules(program, &resolution);
    // The modules that will exist as Kotlin files: targets for generated
    // imports [kt-package].
    let emitted_modules: HashSet<&ModulePath> = program
        .units()
        .filter(|u| {
            u.file.kind == SourceKind::Language
                && reachable.contains(&u.file.module)
                && module_produces_code(u.ast)
        })
        .map(|u| &u.file.module)
        .collect();

    // [backend-external] Everything external in `core.*` must be covered
    // by the backend's define files (core is implicitly imported).
    let mut errors = check_core_define_coverage(program, &symbols);

    let mut files = Vec::new();
    let mut union_sizes: BTreeSet<usize> = BTreeSet::new();
    for (file_idx, unit) in program.units().enumerate() {
        if unit.file.kind != SourceKind::Language
            || !reachable.contains(&unit.file.module)
            || !module_produces_code(unit.ast)
        {
            continue;
        }
        // Generated Kotlin imports: a wildcard per foreign emitted module
        // this file references, plus alias imports for aliased Salvo
        // imports of Kotlin-visible items [kt-imports].
        let generated = generated_imports(
            unit.ast,
            &unit.file.module,
            &resolution.scopes[file_idx],
            &emitted_modules,
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
        rel_path.set_extension("kt");
        files.push(EmittedFile { rel_path, content });
    }
    if !union_sizes.is_empty() {
        files.push(EmittedFile {
            rel_path: std::path::PathBuf::from("unions.kt"),
            content: generate_unions_file(&union_sizes),
        });
    }
    // [backend-companion] Backend-native companion files are copied
    // verbatim whenever their module is needed. A companion must not
    // collide with a generated file (its module should be externals-only).
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
        files.push(EmittedFile {
            rel_path: comp.rel_path.clone(),
            content: comp.content.clone(),
        });
    }
    if errors.is_empty() {
        Ok(files)
    } else {
        Err(errors)
    }
}

/// The Kotlin package of a Salvo module [kt-package]: `salvo.` plus the
/// module path (`core.console` -> `salvo.core.console`). The generated
/// `unions.kt` lives in the root package `salvo`.
fn kotlin_package(module: &ModulePath) -> String {
    let mut out = String::from("salvo");
    for part in &module.0 {
        out.push('.');
        out.push_str(&kt_ident(part));
    }
    out
}

/// The generated Kotlin imports of one file [kt-imports]: a wildcard
/// import per foreign emitted module whose names the file uses, plus a
/// Kotlin alias import for every aliased Salvo import of an item that
/// exists as a Kotlin symbol (inlined externals have none).
fn generated_imports(
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
    for item in &ast.items {
        let Item::Import(imp) = item else { continue };
        let (Some(alias), Some(item_name)) = (&imp.alias, imp.path.last()) else {
            continue;
        };
        // Only items with a Kotlin symbol can be alias-imported: fns with
        // bodies, structs, effects, handlers. Inlined externals, type
        // aliases, and qualifiers resolve without one.
        let alias_name = alias.name.as_str();
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
        if emitted_modules.contains(*module) {
            imports.insert(format!(
                "import {}.{} as {}",
                kotlin_package(module),
                kt_ident(&item_name.name),
                kt_ident(alias_name)
            ));
        }
    }
    imports
}

/// [backend-external] Every `external` item in the implicitly imported
/// `core.*` modules must have a define for this backend. Outside core,
/// missing defines are reported where the item is actually referenced.
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
                            "{}: external fn `{}` in core has no kotlin `define fn`",
                            unit.file.name, f.name.name
                        ));
                    }
                }
                Item::Type(t) if t.backing == Some(BackingMod::External) => {
                    if !symbols.define_types.contains_key(t.name.name.as_str()) {
                        errors.push(format!(
                            "{}: external type `{}` in core has no kotlin `define type`",
                            unit.file.name, t.name.name
                        ));
                    }
                }
                Item::Handler(h) if h.backing == Some(BackingMod::External) => {
                    if !symbols.define_handlers.contains_key(h.name.name.as_str()) {
                        errors.push(format!(
                            "{}: external handler `{}` in core has no kotlin `define handler`",
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

/// Generates the sealed union wrapper hierarchy for every needed size
/// [kt-union-wrappers].
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

/// Kotlin reserved words that need backtick-escaping as identifiers.
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
    /// Effect environment: canonical effect type (e.g. `Random<Int>`) ->
    /// Kotlin expression providing the handler.
    effect_env: Vec<(String, String)>,
    /// Names that are reassigned (or `++`-incremented) in the current
    /// function; these become `var`.
    mutated: HashSet<String>,
    /// Generic parameters in scope (treated as opaque type names).
    generics: HashSet<String>,
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
    /// Generated Kotlin imports for this file [kt-imports] (wildcards for
    /// referenced foreign modules, aliases for aliased imports).
    generated_imports: BTreeSet<String>,
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
            mutated: HashSet::new(),
            generics: HashSet::new(),
            loop_results: Vec::new(),
            loop_id: 0,
            destructure_id: 0,
            taken_names: HashSet::new(),
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

    /// Looks up a checker-resolved fn declaration by its stable key.
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
                Item::Struct(s) => body.push_str(&self.emit_struct(s)),
                Item::Effect(e) => body.push_str(&self.emit_effect(e)),
                Item::Handler(h) => body.push_str(&self.emit_handler(h)),
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

    fn emit_struct(&mut self, s: &StructDecl) -> String {
        let is_mut = s
            .auto_qualifiers
            .iter()
            .any(|q| q.name.name == "Mut");
        let saved = self.enter_generics(&s.generics);
        let generics = self.emit_generic_params(&s.generics);
        let mut out = format!("\ndata class {}{generics}(\n", s.name.name);
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
        out.push_str(")\n");
        self.generics = saved;
        out
    }

    fn emit_effect(&mut self, e: &EffectDecl) -> String {
        let saved = self.enter_generics(&e.generics);
        let generics = self.emit_generic_params(&e.generics);
        let mut out = format!("\ninterface {}{generics} {{\n", e.name.name);
        for f in &e.fns {
            let params = self.emit_param_list(&f.params);
            let ret = self.emit_return_type(f.return_type.as_ref());
            out.push_str(&format!("    fun {}({params}){ret}\n", kt_ident(&f.name.name)));
        }
        out.push_str("}\n");
        self.generics = saved;
        out
    }

    fn emit_handler(&mut self, h: &HandlerDecl) -> String {
        if h.backing == Some(BackingMod::External) {
            return self.emit_define_handler(h);
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
        for f in &h.fns {
            out.push_str(&self.emit_fn_inner(f, "override fun", 1, false));
        }
        out.push_str("}\n");
        self.generics = saved;
        out
    }

    /// An `external handler`, implemented by a `define handler` template:
    /// emits a Kotlin class whose methods inline the templates. All
    /// handlers are classes (external ones included) and are instantiated
    /// at their `use` site.
    fn emit_define_handler(&mut self, h: &HandlerDecl) -> String {
        let Some(def) = self.symbols.define_handlers.get(h.name.name.as_str()) else {
            self.error(format!(
                "external handler `{}` has no kotlin `define handler`",
                h.name.name
            ));
            return String::new();
        };
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
        let mut out = format!("\nclass {}{ctor} : {of} {{\n", h.name.name);
        for dfn in &def.fns {
            if let Some(imports) = &dfn.body.imports {
                self.add_template_imports(imports);
            }
            let params = self.emit_param_list(&dfn.sig.params);
            let ret = self.emit_return_type(dfn.sig.return_type.as_ref());
            let Some(inline) = &dfn.body.inline else {
                self.error(format!(
                    "define fn `{}` in handler `{}` has no inline section",
                    dfn.sig.name.name, h.name.name
                ));
                continue;
            };
            // Interpolations refer to the define fn's own parameter names.
            let args: Vec<String> = dfn
                .sig
                .params
                .iter()
                .map(|p| kt_ident(&p.name.name))
                .collect();
            let body = self.expand_template(inline, &dfn.sig.params, &args);
            out.push_str(&format!(
                "    override fun {}({params}){ret} {{\n",
                kt_ident(&dfn.sig.name.name)
            ));
            // A value-returning member returns its template's value; `run`
            // makes multi-line templates (statements + final expression)
            // work unchanged [kt-handler-template-return].
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
        // Effect dependencies become leading parameters [kt-effect-params].
        let mut params: Vec<String> = Vec::new();
        if !is_main {
            for eff in f.effects.iter().flatten() {
                if let EffectRef::Effect(r) = eff {
                    let ty = self.emit_type_ref(r);
                    let param = self.unique_name(effect_param_name(&ty));
                    self.effect_env.push((ty.clone(), param.clone()));
                    params.push(format!("{param}: {ty}"));
                }
            }
        }
        for p in &f.params {
            let ty = self.emit_type(&p.ty);
            if p.variadic {
                let elem = self.variadic_elem_type(&p.ty);
                params.push(format!("vararg {}: {elem}", kt_ident(&p.name.name)));
            } else {
                params.push(format!("{}: {ty}", kt_ident(&p.name.name)));
            }
        }

        let ret = if is_main {
            String::new()
        } else {
            self.emit_return_type(f.return_type.as_ref())
        };

        let pad = "    ".repeat(indent);
        let name = if is_main {
            "main".to_string()
        } else if top_level {
            self.kotlin_fn_name(f)
        } else {
            kt_ident(&f.name.name)
        };
        let mut out = format!(
            "\n{pad}{kw}{generics} {name}({}){ret} {{\n",
            params.join(", ")
        );

        // Iterator functions (`yield` in the body) compile to an
        // `Iterable { iterator { ... } }` builder [fn-iterator]
        // [kt-iter-iterable].
        if contains_yield(body) {
            let elem = match f.return_type.as_ref() {
                Some(Type::Named { base, .. }) if base.name.name == "Iter" => base
                    .args
                    .first()
                    .map(|t| self.emit_type(t))
                    .unwrap_or_else(|| "Any".to_string()),
                _ => {
                    self.error(format!(
                        "fn `{}` uses `yield` but does not return Iter<T>",
                        f.name.name
                    ));
                    "Any".to_string()
                }
            };
            out.push_str(&format!(
                "{pad}    return Iterable<{elem}> {{\n{pad}        iterator {{\n"
            ));
            out.push_str(&self.emit_block_stmts(body, indent + 3, StmtCtx::IteratorBody));
            out.push_str(&format!("{pad}        }}\n{pad}    }}\n"));
        } else {
            out.push_str(&self.emit_block_stmts(body, indent + 1, StmtCtx::Normal));
        }
        out.push_str(&format!("{pad}}}\n"));

        self.generics = saved_generics;
        self.effect_env = saved_env;
        self.mutated = saved_mutated;
        self.taken_names = saved_taken;
        out
    }

    /// The Kotlin name for a top-level fn. Qualifiers are erased from
    /// types [qual-erasure], so overloads that differ only in qualifiers
    /// (`full_name(p: Person)` vs `full_name(p: Surname Person)`) would
    /// collide; the qualified overload gets a deterministic `__Qual`
    /// suffix instead [kt-qual-mangling].
    fn kotlin_fn_name(&mut self, decl: &FnDecl) -> String {
        let name = decl.name.name.clone();
        let overloads: Vec<&FnDecl> = match self.symbols.fns.get(name.as_str()) {
            Some(o) if o.len() > 1 => o.clone(),
            _ => return kt_ident(&name),
        };
        let suffix = qual_suffix(decl);
        if suffix.is_empty() {
            return kt_ident(&name);
        }
        let mine = self.erased_sig(decl);
        for other in overloads {
            if std::ptr::eq(other as *const FnDecl, decl as *const FnDecl) {
                continue;
            }
            if self.erased_sig(other) == mine {
                return format!("{name}__{suffix}");
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
            Type::Fn { params, ret, .. } => {
                let ps: Vec<String> = params.iter().map(|p| self.emit_type(p)).collect();
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

        // A `Mut`-qualified type whose define provides a `Mut inline:`
        // template maps through it [type-with-mut] (e.g. `Mut List<T>` ->
        // `MutableList<T>`).
        if has_mut {
            let arg_strs: Vec<String> = base.args.iter().map(|a| self.emit_type(a)).collect();
            if let Some(code) = self.expand_mut_type(&name, &arg_strs) {
                return code;
            }
        }
        self.emit_type_ref_named(&name, &base.args)
    }

    /// Expands the `Mut inline:` template of a type's define, when the
    /// define provides one [type-with-mut]. `None` falls back to the
    /// plain mapping (`Mut` erases like other qualifiers).
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
    /// internal types [backend-internal] [kt-none-unit], `define type`
    /// templates [backend-define-type], or a pass-through name
    /// [type-unknown-lenient].
    fn emit_named_parts(&mut self, name: &str, arg_strs: &[String]) -> String {
        let args = if arg_strs.is_empty() {
            String::new()
        } else {
            format!("<{}>", arg_strs.join(", "))
        };
        // Internal (compiler-mapped) types.
        let internal = match name {
            "Str" => Some("String"),
            "Int" => Some("Int"),
            "Long" => Some("Long"),
            "Float" => Some("Float"),
            "Double" => Some("Double"),
            "Bool" => Some("Boolean"),
            "Char" => Some("Char"),
            "Byte" => Some("Byte"),
            "None" => Some("Unit"),
            "Any" => Some("Any"),
            "Nothing" => Some("Nothing"),
            "Iter" => Some("Iterable"),
            _ => None,
        };
        if let Some(kt) = internal {
            return format!("{kt}{args}");
        }
        // External types via define templates.
        if let Some(def) = self.symbols.define_types.get(name) {
            let def = *def;
            if let Some(imports) = &def.body.imports {
                self.add_template_imports(imports);
            }
            if let Some(inline) = &def.body.inline {
                return self.expand_type_template(inline, &def.generics, arg_strs);
            }
        }
        // [backend-external] A declared external type with no define for
        // this backend must not silently pass through.
        if self.symbols.external_types.contains_key(name) {
            self.error(format!(
                "external type `{name}` has no kotlin `define type`"
            ));
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
                // A `Mut`-qualified type maps through its define's
                // `Mut inline:` template [type-with-mut]; other qualifiers
                // erase.
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
            // Unions/fn types/unknowns do not occur as effect types; the
            // Salvo-side rendering keeps the lookup falling back to
            // base-name matching for anything unexpected.
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
                // The `Mut inline:` define mapping survives erasure
                // [type-with-mut].
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
            Ty::Fn { params, ret } => {
                let ps: Vec<String> = params.iter().map(|p| self.emit_ty(p)).collect();
                format!("({}) -> {}", ps.join(", "), self.emit_ty(ret))
            }
            Ty::Var(name) => name.clone(),
            Ty::Any | Ty::Unknown => "Any".to_string(),
            Ty::Nothing => "Nothing".to_string(),
        }
    }

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
                ..
            } => self.emit_let(pattern, ty.as_ref(), value, indent),
            Stmt::Assign { target, value, .. } => {
                let t = self.emit_expr_raw(target);
                let v = self.emit_expr(value);
                format!("{pad}{t} = {v}\n")
            }
            Stmt::Return { value, .. } => match (ctx, value) {
                (StmtCtx::IteratorBody, None) => format!("{pad}return@iterator\n"),
                (StmtCtx::IteratorBody, Some(_)) => {
                    self.error("`return` with a value is not allowed in an iterator function");
                    format!("{pad}return@iterator\n")
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
                            let stmt = self.emit_expr_stmt(v, indent, ctx);
                            format!("{stmt}{pad}{result} = null\n{pad}break\n")
                        } else {
                            let code = self.emit_expr(v);
                            format!("{pad}{result} = {code}\n{pad}break\n")
                        }
                    }
                    // The loop's value is discarded (statement position):
                    // evaluate the operand for side effects only.
                    (Some(v), None) => {
                        let stmt = self.emit_expr_stmt(v, indent, ctx);
                        format!("{stmt}{pad}break\n")
                    }
                    (None, _) => format!("{pad}break\n"),
                }
            }
            Stmt::Continue { .. } => format!("{pad}continue\n"),
            Stmt::Yield { value, .. } => {
                let v = self.emit_expr(value);
                format!("{pad}yield({v})\n")
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
        let (handler_name, handler_code) = match handler {
            Expr::Ident(id) => (id.name.clone(), format!("{}()", kt_ident(&id.name))),
            Expr::Call { callee, .. } => match callee.as_ref() {
                Expr::Ident(id) => (id.name.clone(), self.emit_expr(handler)),
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
        let effect_ty = match self.checked.use_effects.get(&(self.file_idx, span)) {
            Some(ty) if ty_is_concrete(ty) => {
                let ty = ty.clone();
                self.kotlin_ty(&ty)
            }
            _ => self.emit_type(&decl.of),
        };
        let var = self.unique_name(effect_param_name(&effect_ty));
        self.effect_env.push((effect_ty.clone(), var.clone()));
        format!("{pad}val {var}: {effect_ty} = {handler_code}\n")
    }

    fn emit_expr_stmt(&mut self, expr: &Expr, indent: usize, ctx: StmtCtx) -> String {
        let pad = "    ".repeat(indent);
        match expr {
            Expr::If {
                branches,
                else_block,
                ..
            } => self.emit_if(branches, else_block.as_ref(), indent, ctx),
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
                self.loop_results.push(None);
                out.push_str(&self.emit_block_stmts(body, indent + 1, ctx));
                self.loop_results.pop();
                out.push_str(&format!("{pad}}}\n"));
                if let (Some(ran), Some(b)) = (&ran, else_block) {
                    out.push_str(&format!("{pad}if (!{ran}) {{\n"));
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
                let var = self.for_pattern_var(pattern);
                let iter = self.emit_expr(iterable);
                let mut out = String::new();
                if let Some(ran) = &ran {
                    out.push_str(&format!("{pad}var {ran} = false\n"));
                }
                out.push_str(&format!("{pad}for ({var} in {iter}) {{\n"));
                if let Some(ran) = &ran {
                    out.push_str(&format!("{inner_pad}{ran} = true\n"));
                }
                self.loop_results.push(None);
                out.push_str(&self.emit_block_stmts(body, indent + 1, ctx));
                self.loop_results.pop();
                out.push_str(&format!("{pad}}}\n"));
                if let (Some(ran), Some(b)) = (&ran, else_block) {
                    out.push_str(&format!("{pad}if (!{ran}) {{\n"));
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
            out.push_str(&format!("{kw} ({c}) {{\n"));
            out.push_str(&self.emit_is_bindings(cond, indent + 1));
            out.push_str(&self.emit_block_stmts(block, indent + 1, ctx));
            out.push_str(&format!("{pad}}} "));
        }
        if let Some(block) = else_block {
            out.push_str("else {\n");
            out.push_str(&self.emit_block_stmts(block, indent + 1, ctx));
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
        ctx: StmtCtx,
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
                    format!(
                        "{pad}        val {} = {subj}{access}.value as {kt}\n",
                        kt_ident(&b.name)
                    )
                } else {
                    format!("{pad}        val {} = {subj} as {kt}\n", kt_ident(&b.name))
                };
                out.push_str(&bind);
            }
            if value_pos {
                out.push_str(&self.emit_value_block(&branch.body));
            } else {
                out.push_str(&self.emit_block_stmts(&branch.body, indent + 2, ctx));
            }
            out.push_str(&format!("{pad}    }}\n"));
        }
        out.push_str(&format!("{pad}}}"));
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
            let subj = self.emit_expr_base(subject);
            let code = match self.is_test_of(is_span).cloned() {
                Some(test) if test.size >= 2 => {
                    let kt = self
                        .ty_of(binding.span)
                        .cloned()
                        .map(|t| self.emit_ty(&t))
                        .unwrap_or_else(|| "Any".to_string());
                    let access = if test.nullable { "?" } else { "" };
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
        if let Expr::Ident(id) = expr {
            if let (Some(repr), Some(logical)) = (self.repr_of(id.span), self.ty_of(id.span)) {
                // Narrowed to a single arm of a wrapper union: unwrap.
                if repr.is_wrapper_union()
                    && !matches!(logical, Ty::Union(_))
                    && !logical.is_none_ty()
                {
                    let logical = logical.clone();
                    let nullable = repr.has_none_arm();
                    let kt = self.emit_ty(&logical);
                    let access = if nullable { "?" } else { "" };
                    return format!("({}{access}.value as {kt})", kt_ident(&id.name));
                }
            }
        }
        self.emit_expr_raw(expr)
    }

    /// The physical Kotlin-level type of an emitted expression (accounting
    /// for the identifier unwrap rule).
    fn emitted_repr(&self, expr: &Expr) -> Option<&'p Ty> {
        let logical = self.ty_of(expr.span())?;
        if let Expr::Ident(id) = expr {
            if let Some(repr) = self.repr_of(id.span) {
                if repr.is_wrapper_union()
                    && !matches!(logical, Ty::Union(_))
                    && !logical.is_none_ty()
                {
                    return Some(logical); // unwrapped at the use site
                }
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
        match coercion.clone() {
            // Kotlin nullability is transparent: a bare value is already a
            // valid `T?` [type-nullable].
            Coercion::WrapOption { .. } => code,
            Coercion::WrapUnion { target, arm } => {
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

    /// Emits the runtime test for an `is` check lowered against a union
    /// representation.
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
            if let Some(decl) = self.symbols.qualifiers.get(q.as_str()).copied() {
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

    fn emit_expr_raw(&mut self, expr: &Expr) -> String {
        match expr {
            Expr::Int { value, .. } => value.to_string(),
            Expr::Float { value, .. } => {
                let s = value.to_string();
                if s.contains('.') {
                    s
                } else {
                    format!("{s}.0")
                }
            }
            Expr::Bool { value, .. } => value.to_string(),
            Expr::Char { value, .. } => format!("'{}'", escape_char(*value)),
            Expr::Str { parts, .. } => self.emit_string(parts),
            Expr::Ident(id) => {
                if id.name == "None" {
                    "null".to_string()
                } else {
                    kt_ident(&id.name)
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
            Expr::Call {
                callee,
                type_args,
                args,
                span,
            } => self.emit_call(callee, type_args, args, *span),
            Expr::Index { base, index, .. } => {
                format!("{}[{}]", self.emit_expr(base), self.emit_expr(index))
            }
            Expr::ArrayLit { elems, .. } => {
                let items: Vec<String> = elems.iter().map(|e| self.emit_expr(e)).collect();
                format!("arrayOf({})", items.join(", "))
            }
            Expr::ArrayInit {
                elem_type,
                size,
                init,
                ..
            } => {
                let elem = self.emit_type_ref(elem_type);
                let size = self.emit_expr(size);
                let lambda = self.emit_expr(init);
                format!("Array<{elem}>({size}) {lambda}")
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
                let inner = self.emit_expr(operand);
                match op {
                    UnaryOp::Neg => format!("-{inner}"),
                    UnaryOp::Not => format!("!{inner}"),
                }
            }
            Expr::Binary { op, lhs, rhs, .. } => {
                let l = self.emit_expr(lhs);
                let r = self.emit_expr(rhs);
                format!("{l} {} {r}", binary_op(*op))
            }
            Expr::Is {
                subject, check, span, ..
            } => {
                let subj = self.emit_expr_base(subject);
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
            Expr::PostIncrement { operand, .. } => {
                format!("{}++", self.emit_expr_raw(operand))
            }
            Expr::If {
                branches,
                else_block,
                ..
            } => self.emit_if_expr(branches, else_block.as_ref()),
            Expr::Lambda { params, body, .. } => self.emit_lambda(params, body),
            Expr::Spread { operand, .. } => format!("*{}", self.emit_expr(operand)),
            Expr::While { .. } | Expr::For { .. } => self.emit_loop_value(expr),
            Expr::When {
                subject, branches, ..
            } => self.emit_when(subject, branches, 0, StmtCtx::Normal, true),
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

    fn emit_if_expr(&mut self, branches: &[(Expr, Block)], else_block: Option<&Block>) -> String {
        let mut out = String::new();
        for (i, (cond, block)) in branches.iter().enumerate() {
            let kw = if i == 0 { "if" } else { " else if" };
            let c = self.emit_expr(cond);
            out.push_str(&format!("{kw} ({c}) {{\n"));
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
                // A missing else means the expression's value is None.
                out.push_str(" else {\nnull\n}");
            }
        }
        out
    }

    /// A block in value position: all statements plus the trailing
    /// expression as the block's value.
    fn emit_value_block(&mut self, block: &Block) -> String {
        let mut out = String::new();
        let env_depth = self.effect_env.len();
        let n = block.stmts.len();
        for (i, stmt) in block.stmts.iter().enumerate() {
            // Kotlin loops are never expressions, so a trailing loop (the
            // block's value [while-value]) needs the value lowering.
            if i + 1 == n {
                if let Stmt::Expr(e @ (Expr::While { .. } | Expr::For { .. })) = stmt {
                    let code = self.emit_expr(e);
                    out.push_str(&code);
                    out.push('\n');
                    continue;
                }
            }
            out.push_str(&self.emit_stmt(stmt, 0, StmtCtx::Normal));
        }
        self.effect_env.truncate(env_depth);
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
                if else_block.is_some() {
                    out.push_str(&format!("var {ran} = false\n"));
                }
                let var = self.for_pattern_var(pattern);
                let iter = self.emit_expr(iterable);
                out.push_str(&format!("for ({var} in {iter}) {{\n"));
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
        let mut out = String::new();
        let env_depth = self.effect_env.len();
        let n = block.stmts.len();
        for (i, stmt) in block.stmts.iter().enumerate() {
            if i + 1 == n {
                if let Stmt::Expr(e) = stmt {
                    out.push_str(&self.emit_tail_assign(e, result));
                    continue;
                }
            }
            out.push_str(&self.emit_stmt(stmt, 0, StmtCtx::Normal));
        }
        self.effect_env.truncate(env_depth);
        out
    }

    /// Assigns a block-tail expression to a loop result local.
    /// `Nothing`-typed tails never fall through (statement as-is);
    /// `None`-typed tails have no Kotlin payload (statement, then `null`).
    fn emit_tail_assign(&mut self, e: &Expr, result: &str) -> String {
        match self.ty_of(e.span()) {
            Some(Ty::Nothing) => self.emit_expr_stmt(e, 0, StmtCtx::Normal),
            Some(t) if t.is_none_ty() => {
                let stmt = self.emit_expr_stmt(e, 0, StmtCtx::Normal);
                format!("{stmt}{result} = null\n")
            }
            _ => {
                let code = self.emit_expr(e);
                format!("{result} = {code}\n")
            }
        }
    }

    fn emit_lambda(&mut self, params: &[LambdaParam], body: &LambdaBody) -> String {
        let param_list: Vec<String> = params
            .iter()
            .map(|p| match &p.ty {
                Some(t) => {
                    let ty = self.emit_type(t);
                    format!("{}: {ty}", kt_ident(&p.name.name))
                }
                None => kt_ident(&p.name.name),
            })
            .collect();
        match body {
            LambdaBody::Expr(expr) => {
                format!("{{ {} -> {} }}", param_list.join(", "), self.emit_expr(expr))
            }
            LambdaBody::Block(block) => {
                // Kotlin lambdas return their last expression; a trailing
                // `return X` becomes the value.
                let mut out = format!("{{ {} ->\n", param_list.join(", "));
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
        }
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

    fn emit_call(&mut self, callee: &Expr, type_args: &[Type], args: &[Expr], span: Span) -> String {
        // Normalize dot-notation [fn-dot]: `base.f(args)` == `f(base, args)`
        // when `f` resolves to a known function/define/effect member.
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
            // Unknown method: pass through as a Kotlin method call.
            let base_code = self.emit_expr(base);
            let arg_code: Vec<String> = args.iter().map(|a| self.emit_expr(a)).collect();
            return format!("{base_code}.{}({})", kt_ident(name), arg_code.join(", "));
        }

        if let Expr::Ident(id) = callee {
            let arg_refs: Vec<&Expr> = args.iter().collect();
            return self.emit_resolved_call(&id.name, type_args, &arg_refs, span);
        }

        // Calling a computed value (lambda etc.).
        let callee_code = self.emit_expr(callee);
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
        // [kt-effect-params]. The checker records which effect instance
        // the call resolved to ([effect-disambiguation], `effect_calls`);
        // string matching on the effect name remains the fallback for
        // unchecked contexts.
        if let Some(effect) = self.symbols.effect_of_fn.get(name).copied() {
            let handler = match self.checked.effect_calls.get(&(self.file_idx, span)) {
                Some(ty) if ty_is_concrete(ty) => {
                    let ty = ty.clone();
                    let key = self.kotlin_ty(&ty);
                    self.lookup_effect_handler_by_type(&key)
                }
                _ => self.lookup_effect_handler(effect, type_args),
            };
            let arg_code: Vec<String> = args.iter().map(|a| self.emit_expr(a)).collect();
            return format!("{handler}.{}({})", kt_ident(name), arg_code.join(", "));
        }

        // 2. Checker-resolved fn target (type-based overloads win). External
        // signatures route to their backend define template.
        let checker_resolved = self
            .checked
            .call_fn
            .get(&(self.file_idx, span))
            .and_then(|key| self.fn_by_key(*key));
        if let Some(f) = checker_resolved {
            if f.body.is_none() {
                if let Some(def) = self.define_for_decl(name, f) {
                    return self.emit_define_call(name, def, args);
                }
                self.error(format!(
                    "external fn `{name}` has no kotlin `define fn`"
                ));
                return "TODO()".to_string();
            }
            return self.emit_fn_call(name, f, type_args, args, span);
        }

        // 3. `define fn` template by arity (unchecked contexts).
        if let Some(def) = self.symbols.resolve_define_fn(name, args.len()) {
            return self.emit_define_call(name, def, args);
        }

        // 4. Known function by arity.
        if let Some(f) = self.symbols.resolve_fn(name, args.len()) {
            // [backend-external] An external signature that reached this
            // point has no define (step 3 would have matched one).
            if f.body.is_none() {
                self.error(format!(
                    "external fn `{name}` has no kotlin `define fn`"
                ));
                return "TODO()".to_string();
            }
            return self.emit_fn_call(name, f, type_args, args, span);
        }

        // 5. Handler constructor / struct / local callable: pass through.
        let arg_code: Vec<String> = args.iter().map(|a| self.emit_expr(a)).collect();
        let generics = self.emit_type_args(type_args);
        format!("{}{generics}({})", kt_ident(name), arg_code.join(", "))
    }

    /// Finds the define template matching an external fn signature
    /// [backend-define-inline]. Overloaded externals (e.g. `size(Str)` vs
    /// `size(List<T>)`) share a define name, so templates whose parameter
    /// types match the resolved declaration take precedence over a mere
    /// arity match.
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

    /// Inline expansion of a `define fn` template.
    fn emit_define_call(&mut self, name: &str, def: &'p DefineFn, args: &[&Expr]) -> String {
        if let Some(imports) = &def.body.imports {
            self.add_template_imports(imports);
        }
        if let Some(inline) = def.body.inline.clone() {
            let arg_code: Vec<String> = args.iter().map(|a| self.emit_expr(a)).collect();
            return self
                .expand_template(&inline, &def.sig.params, &arg_code)
                .trim()
                .to_string();
        }
        self.error(format!("define fn `{name}` has no inline section"));
        "TODO()".to_string()
    }

    /// A call to a declared function: effect handlers become leading args.
    /// The checker records the resolved effect instances per call site
    /// (`call_effects`); the declared effect refs are the string-matching
    /// fallback for unchecked contexts.
    fn emit_fn_call(
        &mut self,
        name: &str,
        f: &FnDecl,
        type_args: &[Type],
        args: &[&Expr],
        span: Span,
    ) -> String {
        let mut all: Vec<String> = Vec::new();
        match self.checked.call_effects.get(&(self.file_idx, span)).cloned() {
            Some(effs) if effs.iter().all(ty_is_concrete) => {
                for ty in &effs {
                    let key = self.kotlin_ty(ty);
                    all.push(self.lookup_effect_handler_by_type(&key));
                }
            }
            _ => {
                for eff in f.effects.iter().flatten() {
                    if let EffectRef::Effect(r) = eff {
                        let ty = self.emit_type_ref(r);
                        all.push(self.lookup_effect_handler_by_type(&ty));
                    }
                }
            }
        }
        for a in args {
            all.push(self.emit_expr(a));
        }
        let generics = self.emit_type_args(type_args);
        // A call through an import alias keeps the alias: the generated
        // Kotlin alias import maps it to the declaration [kt-imports].
        let kt_name = if name != f.name.name {
            kt_ident(name)
        } else {
            self.kotlin_fn_name(f)
        };
        format!("{kt_name}{generics}({})", all.join(", "))
    }

    /// Resolves the handler expression for a call to an effect member fn.
    fn lookup_effect_handler(&mut self, effect: &str, type_args: &[Type]) -> String {
        if !type_args.is_empty() {
            let full = format!("{effect}{}", self.emit_type_args(type_args));
            return self.lookup_effect_handler_by_type(&full);
        }
        let matches: Vec<(String, String)> = self
            .effect_env
            .iter()
            .filter(|(ty, _)| ty == effect || ty.starts_with(&format!("{effect}<")))
            .cloned()
            .collect();
        match matches.len() {
            1 => matches[0].1.clone(),
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
                matches[0].1.clone()
            }
        }
    }

    fn lookup_effect_handler_by_type(&mut self, effect_ty: &str) -> String {
        if let Some((_, expr)) = self.effect_env.iter().find(|(ty, _)| ty == effect_ty) {
            return expr.clone();
        }
        // Fall back to a unique same-base-name match (generic callee effects
        // like `Random<T>` against a concrete `Random<Int>` in scope).
        let base = effect_ty.split('<').next().unwrap_or(effect_ty);
        let matches: Vec<&(String, String)> = self
            .effect_env
            .iter()
            .filter(|(ty, _)| ty.split('<').next().unwrap_or(ty) == base)
            .collect();
        if matches.len() == 1 {
            return matches[0].1.clone();
        }
        self.error(format!(
            "no handler for effect `{effect_ty}` in scope (declare it in the \
             function's effect list or `use` a handler)"
        ));
        "TODO()".to_string()
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

    /// Expands a `define fn` inline template [backend-define-inline]:
    /// `${param}` becomes the argument's code, `${...param}` splices the
    /// remaining arguments.
    fn expand_template(&mut self, template: &Template, params: &[Param], args: &[String]) -> String {
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
                            out.push_str("TODO()");
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

    /// Expands a `define type` inline template: `${T}` becomes the emitted
    /// Kotlin type argument.
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
                            out.push_str("Any");
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

#[derive(Clone, Copy, PartialEq)]
enum StmtCtx {
    Normal,
    /// Inside an `iterator {}` builder: bare `return` becomes
    /// `return@iterator`.
    IteratorBody,
}

/// True when a checker type contains no `Unknown` (inference fully
/// resolved it) — only then is it safe to render it into emitted code.
/// The base type name of an AST type, used to pair define templates with
/// overloaded external declarations. `None` for shapes without a single
/// base name (unions, tuples, fn types).
fn type_base_name(ty: &Type) -> Option<&str> {
    match ty {
        Type::Named { base, .. } => Some(base.name.name.as_str()),
        Type::Nullable { inner, .. } => type_base_name(inner),
        Type::QualifiedGroup { base, .. } => type_base_name(base),
        Type::Array { .. } => Some("[]"),
        _ => None,
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
        Ty::Fn { params, ret } => params.iter().all(ty_is_concrete) && ty_is_concrete(ret),
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
                    parts.push(q.name.name.clone());
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
            effects,
            ret,
            span,
        } => Type::Fn {
            params: params.iter().map(|p| subst_ast_type(p, map)).collect(),
            effects: effects.clone(),
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
