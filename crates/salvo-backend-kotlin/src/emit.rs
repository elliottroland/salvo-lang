//! The Kotlin code emitter.
//!
//! M2 scope: functions (with effects as leading parameters), structs (data
//! classes), effects (interfaces), handlers (classes/objects, including
//! `define handler` templates), `define fn`/`define type` inline expansion,
//! string interpolation, `if`/`is` with bindings, iterator functions
//! (`yield` -> Kotlin `iterator {}` builder), `while`/`for` statements.
//!
//! Not yet supported (reported as codegen errors): general union types,
//! `when` expressions, loop-as-value, value `break`, tuples beyond
//! Pair/Triple, qualifiers with runtime semantics.

use std::collections::{BTreeSet, HashSet};

use salvo_core::{Program, SourceKind, Symbols};
use salvo_syntax::ast::*;

pub struct EmittedFile {
    /// Path relative to the target dir, e.g. `core/console.kt`.
    pub rel_path: std::path::PathBuf,
    pub content: String,
}

/// Emits Kotlin for every module that produces code. Returns the files or
/// the accumulated codegen errors.
pub fn emit_program(program: &Program) -> Result<Vec<EmittedFile>, Vec<String>> {
    let symbols = Symbols::collect(program);
    let mut files = Vec::new();
    let mut errors = Vec::new();
    for unit in program.units() {
        if unit.file.kind != SourceKind::Language {
            continue;
        }
        if !module_produces_code(unit.ast) {
            continue;
        }
        let mut emitter = Emitter::new(&symbols, &unit.file.name);
        let content = emitter.emit_module(unit.ast);
        errors.extend(emitter.errors);
        let mut rel_path = std::path::PathBuf::new();
        for part in &unit.file.module.0 {
            rel_path.push(part);
        }
        rel_path.set_extension("kt");
        files.push(EmittedFile { rel_path, content });
    }
    if errors.is_empty() {
        Ok(files)
    } else {
        Err(errors)
    }
}

/// Does this module contain anything that turns into Kotlin code?
fn module_produces_code(module: &Module) -> bool {
    module.items.iter().any(|item| match item {
        Item::Struct(_) | Item::Effect(_) => true,
        Item::Handler(_) => true,
        Item::Fn(f) => f.body.is_some(),
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
    file_name: String,
    imports: BTreeSet<String>,
    errors: Vec<String>,
    /// Effect environment: canonical effect type (e.g. `Random<Int>`) ->
    /// Kotlin expression providing the handler.
    effect_env: Vec<(String, String)>,
    /// Names that are reassigned (or `++`-incremented) in the current
    /// function; these become `var`.
    mutated: HashSet<String>,
    /// Generic parameters in scope (treated as opaque type names).
    generics: HashSet<String>,
}

impl<'p> Emitter<'p> {
    fn new(symbols: &'p Symbols<'p>, file_name: &str) -> Self {
        Emitter {
            symbols,
            file_name: file_name.to_string(),
            imports: BTreeSet::new(),
            errors: Vec::new(),
            effect_env: Vec::new(),
            mutated: HashSet::new(),
            generics: HashSet::new(),
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
                _ => {}
            }
        }
        let mut out = String::from("package salvo\n");
        if !self.imports.is_empty() {
            out.push('\n');
            for import in &self.imports {
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
    /// emits a Kotlin `object` whose methods inline the templates.
    fn emit_define_handler(&mut self, h: &HandlerDecl) -> String {
        let Some(def) = self.symbols.define_handlers.get(h.name.name.as_str()) else {
            self.error(format!(
                "external handler `{}` has no kotlin `define handler`",
                h.name.name
            ));
            return String::new();
        };
        let of = self.emit_type(&h.of);
        let mut out = format!("\nobject {} : {of} {{\n", h.name.name);
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
            for line in body.lines() {
                out.push_str(&format!("        {line}\n"));
            }
            out.push_str("    }\n");
        }
        out.push_str("}\n");
        out
    }

    fn emit_fn(&mut self, f: &FnDecl) -> String {
        self.emit_fn_inner(f, "fun", 0, true)
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

        let is_main = top_level && f.name.name == "main";
        let generics = self.emit_generic_params(&f.generics);

        // Effect dependencies become leading parameters.
        let mut params: Vec<String> = Vec::new();
        if !is_main {
            for eff in f.effects.iter().flatten() {
                if let EffectRef::Effect(r) = eff {
                    let ty = self.emit_type_ref(r);
                    let param = effect_param_name(&ty);
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
        } else {
            kt_ident(&f.name.name)
        };
        let mut out = format!(
            "\n{pad}{kw}{generics} {name}({}){ret} {{\n",
            params.join(", ")
        );

        // Iterator functions (`yield` in the body) compile to an
        // `Iterable { iterator { ... } }` builder.
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
        out
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
                    self.error(format!("tuples of size {n} are not supported yet"));
                    "Any".to_string()
                }
            },
            Type::QualifiedGroup { base, .. } => self.emit_type(base),
        }
    }

    fn emit_named_type(&mut self, qualifiers: &[TypeRef], base: &TypeRef) -> String {
        let name = base.name.name.as_str();
        let has_mut = qualifiers.iter().any(|q| q.name.name == "Mut");

        // Internal `Mut List<T>` maps to Kotlin MutableList.
        if has_mut && name == "List" {
            let args = self.emit_type_args(&base.args);
            return format!("MutableList{args}");
        }
        self.emit_type_ref_named(name, &base.args)
    }

    fn emit_type_ref(&mut self, r: &TypeRef) -> String {
        self.emit_type_ref_named(&r.name.name, &r.args)
    }

    fn emit_type_ref_named(&mut self, name: &str, args: &[Type]) -> String {
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
            return format!("{kt}{}", self.emit_type_args(args));
        }
        // External types via define templates.
        if let Some(def) = self.symbols.define_types.get(name) {
            let def = *def;
            if let Some(imports) = &def.body.imports {
                self.add_template_imports(imports);
            }
            if let Some(inline) = &def.body.inline {
                let arg_strs: Vec<String> = args.iter().map(|a| self.emit_type(a)).collect();
                return self.expand_type_template(inline, &def.generics, &arg_strs);
            }
        }
        // Type aliases expand structurally.
        if let Some(alias) = self.symbols.type_aliases.get(name) {
            if let Some(target) = &alias.alias {
                // (Generic alias substitution is not implemented yet; only
                // non-generic aliases expand cleanly.)
                if alias.generics.is_empty() {
                    return self.emit_type(target);
                }
            }
        }
        // Structs, generics, effects, and unknown names pass through.
        format!("{name}{}", self.emit_type_args(args))
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

    /// `T | None` (in any arm order) becomes Kotlin `T?`; other unions are
    /// not supported until the sealed-interface encoding lands.
    fn emit_union_type(&mut self, arms: &[Type]) -> String {
        let (none_arms, value_arms): (Vec<&Type>, Vec<&Type>) =
            arms.iter().partition(|t| is_none_type(t));
        if value_arms.len() == 1 && !none_arms.is_empty() {
            return format!("{}?", self.emit_type(value_arms[0]));
        }
        self.error(
            "general union types are not supported by the kotlin backend yet \
             (only `T | None`)",
        );
        "Any".to_string()
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
                let t = self.emit_expr(target);
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
                if value.is_some() {
                    self.error("`break` with a value is not supported yet");
                }
                format!("{pad}break\n")
            }
            Stmt::Continue { .. } => format!("{pad}continue\n"),
            Stmt::Yield { value, .. } => {
                let v = self.emit_expr(value);
                format!("{pad}yield({v})\n")
            }
            Stmt::Use { handler, .. } => self.emit_use(handler, indent),
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
                let mut out = format!("{pad}val __destructured = {value_code}\n");
                for f in fields {
                    let kw = if self.mutated.contains(&f.binding.name) {
                        "var"
                    } else {
                        "val"
                    };
                    out.push_str(&format!(
                        "{pad}{kw} {} = __destructured.{}\n",
                        kt_ident(&f.binding.name),
                        kt_ident(&f.field.name)
                    ));
                }
                out
            }
        }
    }

    /// `use Handler(...)` — instantiate the handler, bind it, and register
    /// it in the effect environment for the rest of the scope.
    fn emit_use(&mut self, handler: &Expr, indent: usize) -> String {
        let pad = "    ".repeat(indent);
        let (handler_name, handler_code) = match handler {
            Expr::Ident(id) => (id.name.clone(), self.emit_expr(handler)),
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
        let effect_ty = self.emit_type(&decl.of);
        let var = effect_param_name(&effect_ty);
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
                if else_block.is_some() {
                    self.error("`while ... else` is not supported yet");
                }
                if let Expr::Is { .. } = cond.as_ref() {
                    self.error("`while x is T` conditions are not supported yet");
                }
                let c = self.emit_expr(cond);
                let mut out = format!("{pad}while ({c}) {{\n");
                out.push_str(&self.emit_block_stmts(body, indent + 1, ctx));
                out.push_str(&format!("{pad}}}\n"));
                out
            }
            Expr::For {
                pattern,
                iterable,
                body,
                else_block,
                ..
            } => {
                if else_block.is_some() {
                    self.error("`for ... else` is not supported yet");
                }
                let var = match pattern {
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
                };
                let iter = self.emit_expr(iterable);
                let mut out = format!("{pad}for ({var} in {iter}) {{\n");
                out.push_str(&self.emit_block_stmts(body, indent + 1, ctx));
                out.push_str(&format!("{pad}}}\n"));
                out
            }
            Expr::When { .. } => {
                self.error("`when` expressions are not supported by the kotlin backend yet");
                String::new()
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

    /// For a condition containing `x is T name`, emits
    /// `val name = x as T` inside the branch.
    fn emit_is_bindings(&mut self, cond: &Expr, indent: usize) -> String {
        let pad = "    ".repeat(indent);
        let mut out = String::new();
        collect_is_bindings(cond, &mut |subject, check, binding| {
            let subj = self.emit_expr(subject);
            let ty = self.emit_is_check_type(check);
            out.push_str(&format!(
                "{pad}val {} = {subj} as {ty}\n",
                kt_ident(&binding.name)
            ));
        });
        out
    }

    // ================= expressions =================

    fn emit_expr(&mut self, expr: &Expr) -> String {
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
            Expr::Field { base, field, .. } => {
                format!("{}.{}", self.emit_expr(base), kt_ident(&field.name))
            }
            Expr::Call {
                callee,
                type_args,
                args,
                ..
            } => self.emit_call(callee, type_args, args),
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
                        self.error(format!("tuples of size {n} are not supported yet"));
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
                subject, check, ..
            } => {
                let subj = self.emit_expr(subject);
                if check.len() == 1 && check[0].name.name == "None" {
                    return format!("{subj} == null");
                }
                let ty = self.emit_is_check_type(check);
                format!("{subj} is {ty}")
            }
            Expr::As { value, .. } => {
                // Constructive qualifier casts are erased in Kotlin.
                self.emit_expr(value)
            }
            Expr::NonNull { operand, .. } => format!("{}!!", self.emit_expr(operand)),
            Expr::PostIncrement { operand, .. } => format!("{}++", self.emit_expr(operand)),
            Expr::If {
                branches,
                else_block,
                ..
            } => self.emit_if_expr(branches, else_block.as_ref()),
            Expr::Lambda { params, body, .. } => self.emit_lambda(params, body),
            Expr::Spread { operand, .. } => format!("*{}", self.emit_expr(operand)),
            Expr::While { .. } | Expr::For { .. } => {
                self.error("loops as value expressions are not supported yet");
                "Unit".to_string()
            }
            Expr::When { .. } => {
                self.error("`when` expressions are not supported by the kotlin backend yet");
                "Unit".to_string()
            }
            Expr::Error { .. } => "TODO()".to_string(),
        }
    }

    /// The Kotlin type used for `is` checks: qualifiers are dropped, only
    /// the base type matters (`is Err Str` needs union support).
    fn emit_is_check_type(&mut self, check: &[TypeRef]) -> String {
        if check.len() > 1 {
            self.error(
                "qualifier checks in `is` are not supported by the kotlin backend yet",
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
                    let code = self.emit_expr(expr);
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
        for stmt in &block.stmts {
            out.push_str(&self.emit_stmt(stmt, 0, StmtCtx::Normal));
        }
        self.effect_env.truncate(env_depth);
        out
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

    fn emit_call(&mut self, callee: &Expr, type_args: &[Type], args: &[Expr]) -> String {
        // Normalize dot-notation: `base.f(args)` == `f(base, args)` when `f`
        // resolves to a known function/define/effect member.
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
                return self.emit_resolved_call(name, type_args, &all_args);
            }
            // Unknown method: pass through as a Kotlin method call.
            let base_code = self.emit_expr(base);
            let arg_code: Vec<String> = args.iter().map(|a| self.emit_expr(a)).collect();
            return format!("{base_code}.{}({})", kt_ident(name), arg_code.join(", "));
        }

        if let Expr::Ident(id) = callee {
            let arg_refs: Vec<&Expr> = args.iter().collect();
            return self.emit_resolved_call(&id.name, type_args, &arg_refs);
        }

        // Calling a computed value (lambda etc.).
        let callee_code = self.emit_expr(callee);
        let arg_code: Vec<String> = args.iter().map(|a| self.emit_expr(a)).collect();
        format!("{callee_code}({})", arg_code.join(", "))
    }

    fn emit_resolved_call(&mut self, name: &str, type_args: &[Type], args: &[&Expr]) -> String {
        // 1. Effect member call: dispatch through the handler in scope.
        if let Some(effect) = self.symbols.effect_of_fn.get(name).copied() {
            let handler = self.lookup_effect_handler(effect, type_args);
            let arg_code: Vec<String> = args.iter().map(|a| self.emit_expr(a)).collect();
            return format!("{handler}.{}({})", kt_ident(name), arg_code.join(", "));
        }

        // 2. `define fn` template: inline expansion.
        if let Some(def) = self.symbols.resolve_define_fn(name, args.len()) {
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
            return "TODO()".to_string();
        }

        // 3. Known function: prepend effect arguments.
        if let Some(f) = self.symbols.resolve_fn(name, args.len()) {
            let mut all: Vec<String> = Vec::new();
            for eff in f.effects.iter().flatten() {
                if let EffectRef::Effect(r) = eff {
                    let ty = self.emit_type_ref(r);
                    all.push(self.lookup_effect_handler_by_type(&ty));
                }
            }
            for a in args {
                all.push(self.emit_expr(a));
            }
            let generics = self.emit_type_args(type_args);
            return format!("{}{generics}({})", kt_ident(name), all.join(", "));
        }

        // 4. Handler constructor / struct / local callable: pass through.
        let arg_code: Vec<String> = args.iter().map(|a| self.emit_expr(a)).collect();
        let generics = self.emit_type_args(type_args);
        format!("{}{generics}({})", kt_ident(name), arg_code.join(", "))
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

    /// Expands a `define fn` inline template: `${param}` becomes the
    /// argument's code, `${...param}` splices the remaining arguments.
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
        Expr::As { value, .. } => collect_mutated_expr(value, out),
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

/// Walks a condition for `is`-checks with bindings and invokes `f` for each.
fn collect_is_bindings<'a>(
    cond: &'a Expr,
    f: &mut impl FnMut(&'a Expr, &'a [TypeRef], &'a Ident),
) {
    match cond {
        Expr::Is {
            subject,
            check,
            binding: Some(b),
            ..
        } => f(subject, check, b),
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
