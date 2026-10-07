//! [kt-ir] The Kotlin emitter over the IR (IR.md §10 step 2): renders
//! `salvo_ir` modules. Beside the AST emitter until it passes every test;
//! selected by `SALVO_KOTLIN_IR=1`.
//!
//! What it decides is representation and idiom only: how a union is laid out
//! (`UnionN`), how an optional is spelled (`T?`), how `Mut Str` is held
//! (`StringBuilder`), how a reference is reached (fully qualified, so no
//! imports are needed), and the Kotlin shape of each IR node.

use std::collections::{BTreeSet, HashMap, HashSet};

use salvo_core::types::Ty;
use salvo_core::{ModulePath, Program};
use salvo_ir::{
    ArmTest, Block, Decl, DeclId, Expr, ExprKind, FnDecl, FnKind, FnRef, ImplDecl, InterfaceDecl, Lit,
    Local, Module as IrModule, Op, Place, Program as IrProgram, Stmt, Step, StructDecl,
};

use crate::emit::{escape_string, host_package, kotlin_package, kt_ident};
use crate::EmittedFile;

pub fn emit_program_ir(program: &Program) -> Result<(Vec<EmittedFile>, Vec<String>), Vec<String>> {
    let (erased_program, _erased, mut checked, warnings) = salvo_backend::driver::check_for_emission(program)?;
    let program = &erased_program;
    let (symbols, resolution) = salvo_backend::driver::resolve_for_emission(program, &mut checked);
    let reach = salvo_backend::driver::reach(program, &resolution, &symbols, &checked, false);
    let wanted: HashSet<&ModulePath> = reach.emitted.iter().copied().collect();
    let (ir, build_errors) = salvo_ir::build_program(program, &symbols, &resolution, &checked, Some(&wanted));
    if !build_errors.is_empty() {
        return Err(build_errors);
    }
    let fn_names = salvo_core::naming::FnNames::compute(program);
    let mut shared = Shared {
        program,
        symbols: &symbols,
        ir: &ir,
        fn_names: &fn_names,
        union_sizes: BTreeSet::new(),
        tuple_sizes: BTreeSet::new(),
        needs_throw: false,
        needs_compare: false,
        needs_bytes: false,
        errors: Vec::new(),
        decl_modules: HashMap::new(),
        platform_impls: HashMap::new(),
        recursive_copies: BTreeSet::new(),
    };
    for m in &ir.modules {
        for d in &m.decls {
            shared.decl_modules.insert(d.id().clone(), m.path.clone());
            if let Decl::Impl(h) = d {
                if h.platform {
                    for face in &h.faces {
                        if let Ty::Named { name, .. } = face.strip_quals() {
                            shared.platform_impls.entry(name.clone()).or_default().push(h.id.clone());
                        }
                    }
                }
            }
        }
    }
    let mut files = Vec::new();
    for m in &ir.modules {
        let text = ModuleEmitter::new(&mut shared, m).module();
        let mut rel_path = std::path::PathBuf::new();
        for part in &m.path.0 {
            rel_path.push(part);
        }
        rel_path.set_extension("kt");
        files.push(EmittedFile { rel_path, content: text });
    }
    // Runtime files [kt-runtime].
    let features = {
        let mut f = salvo_core::features::RuntimeFeatures::default();
        for (file_idx, unit) in program.units().enumerate() {
            if wanted.contains(&unit.file.module) {
                f.or(salvo_core::features::module_features(&symbols, &checked, file_idx, unit.ast, false, |_| true));
            }
        }
        f
    };
    for &n in &shared.union_sizes {
        files.push(EmittedFile {
            rel_path: std::path::PathBuf::from(format!("unions/Union{n}.kt")),
            content: crate::emit::generate_unions_file(&BTreeSet::from([n]), false),
        });
        if features.wire {
            files.push(EmittedFile {
                rel_path: std::path::PathBuf::from(format!("unions/Union{n}Codec.kt")),
                content: crate::emit::generate_union_codec_file(n),
            });
        }
    }
    for &n in &shared.tuple_sizes {
        files.push(EmittedFile {
            rel_path: std::path::PathBuf::from(format!("tuples/Tuple{n}.kt")),
            content: crate::emit::generate_tuples_file(&BTreeSet::from([n])),
        });
    }
    if shared.needs_throw {
        files.push(EmittedFile { rel_path: "throwsignal.kt".into(), content: crate::emit::generate_throw_file() });
    }
    if shared.needs_compare {
        files.push(EmittedFile { rel_path: "compare.kt".into(), content: crate::emit::generate_compare_file() });
    }
    if shared.needs_bytes || features.wire {
        files.push(EmittedFile { rel_path: "bytes.kt".into(), content: crate::emit::generate_bytes_file() });
    }
    if features.wire {
        files.push(EmittedFile { rel_path: "wire.kt".into(), content: include_str!("../runtime/wire.kt").to_string() });
    }
    if features.scheduler {
        files.push(EmittedFile { rel_path: "scheduler.kt".into(), content: crate::emit::generate_scheduler_file() });
    }
    if features.time() {
        files.push(EmittedFile { rel_path: "hosttime.kt".into(), content: crate::emit::generate_time_file() });
    }
    // Host companions travel with their module [backend-companion].
    for comp in &program.companions {
        if !reach.reachable.contains(&comp.module) {
            continue;
        }
        if files.iter().any(|f| f.rel_path == comp.rel_path) {
            shared.errors.push(format!("companion file `{}` collides with a generated file", comp.rel_path.display()));
            continue;
        }
        files.push(EmittedFile { rel_path: comp.rel_path.clone(), content: comp.content.clone() });
    }
    if !shared.errors.is_empty() {
        return Err(shared.errors);
    }
    Ok((files, warnings))
}

/// Program-wide state the module emitters share.
struct Shared<'p> {
    program: &'p Program,
    symbols: &'p salvo_core::Symbols<'p>,
    ir: &'p IrProgram,
    fn_names: &'p salvo_core::naming::FnNames,
    union_sizes: BTreeSet<usize>,
    tuple_sizes: BTreeSet<usize>,
    needs_throw: bool,
    needs_compare: bool,
    needs_bytes: bool,
    errors: Vec<String>,
    decl_modules: HashMap<DeclId, ModulePath>,
    /// Per interface key: the platform impls of it [platform-abi].
    platform_impls: HashMap<String, Vec<DeclId>>,
    /// [kt-copy] structs that hold themselves, needing a generated copy fn.
    recursive_copies: BTreeSet<(String, bool)>,
}

struct ModuleEmitter<'a, 'p> {
    s: &'a mut Shared<'p>,
    module: &'a IrModule,
    out: String,
    /// Narrowing bindings Kotlin smart-casts: the narrowed local reads as its
    /// source.
    aliases: HashMap<String, String>,
    in_fn_params: HashSet<String>,
    /// The switch arm being rendered: its subject's code, type and test.
    narrowing_subject: Option<(String, Ty, ArmTest)>,
    /// The enclosing fn returns `None` (Kotlin `Unit`).
    ret_is_unit: bool,
}

fn kt_local(l: &Local) -> String {
    let s = l.0.replace('~', "_");
    if s.starts_with("__") { s } else { kt_ident(&s) }
}

impl<'a, 'p> ModuleEmitter<'a, 'p> {
    fn new(s: &'a mut Shared<'p>, module: &'a IrModule) -> Self {
        ModuleEmitter { s, module, out: String::new(), aliases: HashMap::new(), in_fn_params: HashSet::new(), narrowing_subject: None, ret_is_unit: false }
    }

    fn error(&mut self, msg: impl Into<String>) {
        self.s.errors.push(format!("{}: {}", self.module.file_name, msg.into()));
    }

    fn module(mut self) -> String {
        let mut body = String::new();
        for d in &self.module.decls {
            self.out.clear();
            self.decl(d);
            body.push_str(&self.out);
            body.push('\n');
        }
        // [kt-copy] The copy fn of each struct of this module that holds
        // itself; rendering one may ask for another.
        let mut done: BTreeSet<(String, bool)> = BTreeSet::new();
        loop {
            let next = self.s.recursive_copies.iter().find(|e| !done.contains(*e)).cloned();
            let Some((name, mutable)) = next else { break };
            done.insert((name.clone(), mutable));
            if self.s.symbols.key_modules.get(name.as_str()).is_some_and(|m| **m != self.module.path) {
                continue;
            }
            let mut ty = Ty::named(name.clone());
            if mutable {
                ty = ty.qualify(vec![salvo_core::types::Qual::plain("Mut", Vec::new())]);
            }
            let Some(plan) = salvo_core::copyplan::copy_plan(self.s.symbols, &ty) else { continue };
            let code = self.render_copy(&plan, "__v");
            let kt = self.ty(&ty);
            let plain = salvo_core::typekey::plain(&name).replace('.', "_");
            body.push_str(&format!("\nfun __copy{}_{plain}(__v: {kt}): {kt} = {code}\n", if mutable { "Mut" } else { "" }));
        }
        let mut out = format!("package {}\n\nimport salvo.*\n\n", kotlin_package(&self.module.path));
        out.push_str(&body);
        out
    }

    // ------------------------------------------------------------- names --

    /// A Kotlin name for a fn reference: the emitted (overload-mangled) name,
    /// fully qualified when the declaration is in another module.
    fn fn_name(&mut self, id: &DeclId) -> String {
        let file_idx = self.s.program.files.iter().position(|f| f.module == id.module);
        let decl = file_idx.and_then(|fi| self.s.program.modules[fi].items.get(id.item));
        let name = match decl {
            Some(salvo_syntax::ast::Item::Fn(f)) => self.s.fn_names.get(f).unwrap_or(&f.name.name).to_string(),
            Some(salvo_syntax::ast::Item::Qualifier(q)) => {
                let f = q.fns.get(id.sub as usize - 1).map(|f| f.name.name.clone()).unwrap_or_default();
                format!("{}_{}", q.name.name, f)
            }
            _ => self.s.ir.ref_name(id),
        };
        let local = kt_ident(&name);
        if id.module == self.module.path {
            local
        } else {
            format!("{}.{local}", kotlin_package(&id.module))
        }
    }

    /// A type's Kotlin path, by its symbol key.
    fn type_path(&self, key: &str) -> String {
        let plain = salvo_core::typekey::plain(key);
        match self.s.symbols.key_modules.get(key) {
            Some(m) if **m != self.module.path => format!("{}.{plain}", kotlin_package(m)),
            _ => plain.to_string(),
        }
    }

    // ------------------------------------------------------------- types --

    fn ty(&mut self, t: &Ty) -> String {
        match t {
            Ty::Named { name, args } => {
                // [cmp-carry] an identity a container carries is not a type argument.
                let args: Vec<Ty> = args.iter().filter(|a| !matches!(a, Ty::FnName(_))).cloned().collect();
                let args = &args;
                if let Some(kt) = crate::intrinsics::type_name(name) {
                    if name == "Bytes" {
                        self.s.needs_bytes = true;
                    }
                    if matches!(name.as_str(), "Addr" | "Pool" | "Reply") {
                        return kt.to_string();
                    }
                    let a: Vec<String> = args.iter().map(|x| self.ty(x)).collect();
                    return if a.is_empty() { kt.to_string() } else { format!("{kt}<{}>", a.join(", ")) };
                }
                if self.s.symbols.intrinsic_types.get(name.as_str()).is_some_and(|t| t.platform) {
                    if let Some(m) = self.s.symbols.key_modules.get(name.as_str()) {
                        let a: Vec<String> = args.iter().map(|x| self.ty(x)).collect();
                        let base = format!("{}.{}", host_package(m), salvo_core::typekey::plain(name));
                        return if a.is_empty() { base } else { format!("{base}<{}>", a.join(", ")) };
                    }
                }
                let a: Vec<String> = args.iter().map(|x| self.ty(x)).collect();
                // A platform handler is its adapter class [platform-abi].
                let base = if self.s.symbols.handlers.get(name.as_str()).is_some_and(|h| h.platform) {
                    let plain = salvo_core::typekey::plain(name);
                    match self.s.symbols.key_modules.get(name.as_str()) {
                        Some(m) if **m != self.module.path => format!("{}.__Platform_{plain}", kotlin_package(m)),
                        _ => format!("__Platform_{plain}"),
                    }
                } else {
                    self.type_path(name)
                };
                if a.is_empty() { base } else { format!("{base}<{}>", a.join(", ")) }
            }
            Ty::Qualified { quals, base } => {
                if quals.iter().any(|q| q.name == "Mut") {
                    if let Ty::Named { name, args } = base.as_ref() {
                        if let Some(kt) = crate::intrinsics::mut_type_name(name) {
                            let a: Vec<String> = args.iter().map(|x| self.ty(x)).collect();
                            return if a.is_empty() { kt.to_string() } else { format!("{kt}<{}>", a.join(", ")) };
                        }
                    }
                }
                self.ty(base)
            }
            Ty::Union(arms) => {
                let nullable = arms.iter().any(|a| a.is_none_ty());
                let values: Vec<String> = arms.iter().filter(|a| !a.is_none_ty()).map(|a| self.ty(a)).collect();
                let suffix = if nullable { "?" } else { "" };
                match values.len() {
                    0 => "Unit?".to_string(),
                    1 => format!("{}{suffix}", values[0]),
                    n => {
                        self.s.union_sizes.insert(n);
                        format!("Union{n}<{}>{suffix}", values.join(", "))
                    }
                }
            }
            Ty::Tuple(es) => {
                let strs: Vec<String> = es.iter().map(|e| self.ty(e)).collect();
                match strs.len() {
                    2 => format!("Pair<{}, {}>", strs[0], strs[1]),
                    3 => format!("Triple<{}, {}, {}>", strs[0], strs[1], strs[2]),
                    n => {
                        self.s.tuple_sizes.insert(n);
                        format!("Tuple{n}<{}>", strs.join(", "))
                    }
                }
            }
            Ty::Array(e) => format!("Array<{}>", self.ty(e)),
            Ty::Fn { params, ret, effects, .. } => {
                // [fn-effects] the effects a fn value performs are its leading
                // parameters.
                let mut ps: Vec<String> = effects.iter().map(|p| self.ty(p)).collect();
                ps.extend(params.iter().map(|p| self.ty(p)));
                let r = if ret.is_none_ty() { "Unit".to_string() } else { self.ty(ret) };
                format!("({}) -> {r}", ps.join(", "))
            }
            Ty::Var(v) => v.clone(),
            Ty::Never => "Nothing".to_string(),
            Ty::Lit(l) => self.ty(&Ty::named(l.base())),
            _ if t.is_none_ty() => "Unit".to_string(),
            _ => "Any".to_string(),
        }
    }

    /// The `UnionN.Uk<…>` of arm `arm` of union `ty`.
    fn union_arm(&mut self, ty: &Ty, arm: usize) -> Option<(String, usize)> {
        let values: Vec<&Ty> = ty.strip_quals().value_arms();
        let n = values.len();
        if n < 2 {
            return None;
        }
        self.s.union_sizes.insert(n);
        let args: Vec<String> = values.iter().map(|a| self.ty(a)).collect();
        Some((format!("Union{n}.U{}<{}>", arm + 1, args.join(", ")), n))
    }

    // ------------------------------------------------------------- decls --

    fn type_params(&self, tps: &[salvo_ir::TypeParam]) -> String {
        if tps.is_empty() {
            String::new()
        } else {
            format!("<{}>", tps.iter().map(|t| t.name.clone()).collect::<Vec<_>>().join(", "))
        }
    }

    fn decl(&mut self, d: &Decl) {
        match d {
            Decl::Struct(s) => self.struct_decl(s),
            Decl::Union(_) => {}
            Decl::Interface(i) => self.interface_decl(i),
            Decl::Impl(h) => self.impl_decl(h),
            Decl::Fn(f) => self.fn_decl(f, 0, false),
            Decl::PlatformType(_) => {}
            Decl::Static(st) => {
                // [mod-use] a lazy module-level instance.
                let ty = self.ty(&st.ty);
                let name = kt_local(&st.local);
                let _ = &name;
                self.out.push_str(&format!("val {name}: {ty} by lazy {{\n"));
                let saved = std::mem::take(&mut self.out);
                self.stmts(&st.stmts, 1);
                let body = std::mem::replace(&mut self.out, saved);
                self.out.push_str(&body);
                self.out.push_str(&format!("    {name}\n}}\n"));
            }
        }
    }

    fn struct_decl(&mut self, s: &StructDecl) {
        let tps = self.type_params(&s.type_params);
        if s.fields.is_empty() {
            self.out.push_str(&format!("class {}{tps}\n", s.name));
            return;
        }
        self.out.push_str(&format!("data class {}{tps}(\n", s.name));
        for f in &s.fields {
            let ty = self.ty(&f.ty);
            let kw = if s.canbe_mut { "var" } else { "val" };
            self.out.push_str(&format!("    {kw} {}: {ty},\n", kt_ident(&f.name)));
        }
        self.out.push_str(")\n");
    }

    fn params(&mut self, params: &[salvo_ir::Param]) -> String {
        let mut parts = Vec::new();
        for p in params {
            // [fn-variadic] the tail arrives as one array of the element type.
            let ty = self.ty(&p.ty);
            parts.push(format!("{}: {ty}", kt_local(&p.local)));
        }
        parts.join(", ")
    }

    fn interface_decl(&mut self, i: &InterfaceDecl) {
        let tps = self.type_params(&i.type_params);
        self.out.push_str(&format!("interface {}{tps} {{\n", i.name));
        for m in &i.members {
            let ps = self.params(&m.params);
            let ret = self.ret_ty(&m.ret);
            let mtps = self.type_params(&m.type_params);
            self.out.push_str(&format!("    fun {mtps}{}({ps}){ret}\n", kt_ident(&m.emitted_name)));
        }
        self.out.push_str("}\n");
        // [platform-abi] The host-facing interface and adapter, when a
        // platform impl implements this interface.
        if let Some(impls) = self.s.platform_impls.get(&i.name).cloned() {
            self.out.push_str(&format!("\ninterface {}Platform{tps} {{\n", i.name));
            for m in &i.members {
                let ps = self.params(&m.params);
                let ret = self.ret_ty(&m.ret);
                self.out.push_str(&format!("    fun {}({ps}){ret}\n", kt_ident(&m.emitted_name)));
            }
            self.out.push_str("}\n");
            self.out.push_str(&format!("\nopen class __Platform_{0}{tps}(private val impl: {0}Platform{tps}) : {0}{tps} {{\n", i.name));
            for m in &i.members {
                let ps = self.params(&m.params);
                let ret = self.ret_ty(&m.ret);
                let names: Vec<String> = m.params.iter().map(|p| kt_local(&p.local)).collect();
                self.out.push_str(&format!("    override fun {0}({ps}){ret} = impl.{0}({1})\n", kt_ident(&m.emitted_name), names.join(", ")));
            }
            self.out.push_str("}\n");
            for id in impls {
                let Some(Decl::Impl(h)) = self.s.ir.modules.iter().flat_map(|m| &m.decls).find(|d| d.id() == &id) else { continue };
                if h.id.module != self.module.path {
                    continue;
                }
                let ps = self.params(&h.ctor_params);
                let args: Vec<String> = h.ctor_params.iter().map(|p| kt_local(&p.local)).collect();
                self.out.push_str(&format!(
                    "\nclass __Platform_{}({ps}) : __Platform_{}({}.{}({}))\n",
                    h.name,
                    i.name,
                    host_package(&h.id.module),
                    h.name,
                    args.join(", ")
                ));
            }
        }
    }

    fn ret_ty(&mut self, ret: &Ty) -> String {
        if ret.is_none_ty() {
            String::new()
        } else {
            format!(": {}", self.ty(ret))
        }
    }

    fn impl_decl(&mut self, h: &ImplDecl) {
        if h.platform || h.intrinsic {
            // The adapter class is emitted beside the interface.
            return;
        }
        let tps = self.type_params(&h.type_params);
        let mut ctor: Vec<String> = Vec::new();
        for p in &h.ctor_params {
            let ty = self.ty(&p.ty);
            ctor.push(format!("private val {}: {ty}", kt_local(&p.local)));
        }
        for (i, d) in h.deps.iter().enumerate() {
            let ty = self.ty(d);
            ctor.push(format!("private val __dep{i}: {ty}"));
        }
        let faces: Vec<String> = h.faces.iter().map(|f| self.ty(f)).collect();
        self.out.push_str(&format!("class {}{tps}({}) : {} {{\n", h.name, ctor.join(", "), faces.join(", ")));
        for f in &h.state {
            let ty = self.ty(&f.ty);
            let init = match &f.default {
                Some(e) => self.expr(e, 1),
                None => "TODO()".to_string(),
            };
            self.out.push_str(&format!("    var {}: {ty} = {init}\n", kt_ident(&f.name)));
        }
        if let Some(init) = &h.init {
            self.out.push_str("    init {\n");
            if let Some(b) = &init.body {
                self.block_stmts(b, 2);
            }
            self.out.push_str("    }\n");
        }
        for m in &h.members {
            self.fn_decl(m, 1, true);
        }
        self.out.push_str("}\n");
    }

    fn fn_decl(&mut self, f: &FnDecl, indent: usize, member: bool) {
        if f.kind == FnKind::Intrinsic {
            return;
        }
        let pad = "    ".repeat(indent);
        let tps = self.type_params(&f.type_params);
        let ps = self.params(&f.params);
        let ret = self.ret_ty(&f.ret);
        let name = if member {
            kt_ident(&f.name)
        } else {
            let own = self.fn_name(&f.id);
            own.rsplit('.').next().unwrap_or(&own).to_string()
        };
        let kw = if member { "override fun" } else { "fun" };
        self.in_fn_params = f.params.iter().map(|p| kt_local(&p.local)).collect();
        self.aliases.clear();
        self.ret_is_unit = f.ret.is_none_ty();
        if f.kind == FnKind::Platform {
            // [platform-fn] the host's fn, by its package.
            let args: Vec<String> = f
                .params
                .iter()
                .skip(f.effect_params)
                .map(|p| kt_local(&p.local))
                .collect();
            self.out.push_str(&format!(
                "{pad}{kw} {tps}{name}({ps}){ret} = {}.{}({})\n",
                host_package(&f.id.module),
                kt_ident(&f.name),
                args.join(", ")
            ));
            return;
        }
        let Some(body) = &f.body else {
            self.out.push_str(&format!("{pad}{kw} {tps}{name}({ps}){ret}\n"));
            return;
        };
        self.out.push_str(&format!("{pad}{kw} {tps}{name}({ps}){ret} {{\n"));
        self.block_stmts(body, indent + 1);
        if let Some(v) = &body.value {
            let code = self.expr(v, indent + 1);
            self.out.push_str(&format!("{}return {code}\n", "    ".repeat(indent + 1)));
        }
        self.out.push_str(&format!("{pad}}}\n"));
    }

    // -------------------------------------------------------- statements --

    fn block_stmts(&mut self, b: &Block, indent: usize) {
        self.stmts(&b.stmts, indent);
    }

    fn stmts(&mut self, stmts: &[Stmt], indent: usize) {
        for s in stmts {
            self.stmt(s, indent);
        }
    }

    fn stmt(&mut self, s: &Stmt, indent: usize) {
        let pad = "    ".repeat(indent);
        match s {
            Stmt::Loop { body, .. } => {
                self.out.push_str(&format!("{pad}while (true) {{\n"));
                self.block_stmts(body, indent + 1);
                self.out.push_str(&format!("{pad}}}\n"));
            }
            Stmt::ForEach { local, iterable, body, .. } => {
                let mut it = self.expr(iterable, indent);
                // [platform-iterable] an iterable platform type loops over its
                // host's `each`.
                if let Ty::Named { name, .. } = iterable.ty.strip_quals() {
                    let host_iterable = self.s.symbols.intrinsic_types.get(name.as_str()).is_some_and(|d| d.platform && d.iterable);
                    if host_iterable {
                        if let Some(m) = self.s.symbols.key_modules.get(name.as_str()).cloned() {
                            let each = salvo_core::case::camel(&format!("each{}", salvo_core::naming::each_suffix(self.s.program, &m, name)));
                            it = format!("{}.{each}({it})", host_package(&m));
                        }
                    }
                }
                self.out.push_str(&format!("{pad}for ({} in {it}) {{\n", kt_local(local)));
                self.block_stmts(body, indent + 1);
                self.out.push_str(&format!("{pad}}}\n"));
            }
            Stmt::Let { local, ty, value, .. } => {
                let code = self.expr(value, indent);
                let t = self.ty(ty);
                let kw = "var";
                self.out.push_str(&format!("{pad}{kw} {}: {t} = {code}\n", kt_local(local)));
            }
            Stmt::Narrow { local, ty, from, from_ty, .. } => {
                let src = self.place(from, indent);
                let code = self.narrow_code(&src, from_ty, ty);
                let t = self.ty(ty);
                self.out.push_str(&format!("{pad}val {}: {t} = {code}\n", kt_local(local)));
            }
            Stmt::Assign { place, value } => {
                let v = self.expr(value, indent);
                let p = self.place(place, indent);
                self.out.push_str(&format!("{pad}{p} = {v}\n"));
            }
            Stmt::Expr(e) => {
                let code = self.expr(e, indent);
                self.out.push_str(&format!("{pad}{code}\n"));
            }
            Stmt::Return(None) => self.out.push_str(&format!("{pad}return\n")),
            Stmt::Return(Some(e)) => {
                let code = self.expr(e, indent);
                // [kt-none-unit] a `None`-returning fn returns bare.
                if self.ret_is_unit && !matches!(e.kind, ExprKind::Unit) {
                    self.out.push_str(&format!("{pad}{code}\n{pad}return\n"));
                } else if self.ret_is_unit {
                    self.out.push_str(&format!("{pad}return\n"));
                } else {
                    self.out.push_str(&format!("{pad}return {code}\n"));
                }
            }
            Stmt::Break => self.out.push_str(&format!("{pad}break\n")),
            Stmt::Continue => self.out.push_str(&format!("{pad}continue\n")),
        }
    }

    /// [ir-narrow] Reading a narrowed value out of its storage: the arm's
    /// payload of a wrapper union, the value of an optional, or the storage
    /// itself when Kotlin's own type suffices.
    fn narrow_code(&mut self, src: &str, from_ty: &Ty, ty: &Ty) -> String {
        let from_arms: Vec<Ty> = from_ty.strip_quals().value_arms().into_iter().cloned().collect();
        let n = from_arms.len();
        if ty.is_none_ty() {
            return "null".to_string();
        }
        if n >= 2 {
            // A wrapper union: one arm's payload (the arm may itself be a
            // union), or a sub-union rewrapped.
            if let Some(i) = from_arms.iter().position(|a| erase_eq(a, ty)) {
                let stars = vec!["*"; n].join(", ");
                let t = self.ty(ty);
                return format!("(({src} as Union{n}.U{}<{stars}>).value as {t})", i + 1);
            }
            return self.rewrap(src, from_ty, ty);
        }
        if from_ty.strip_quals().has_none_arm() && !ty.strip_quals().has_none_arm() {
            return format!("{src}!!");
        }
        src.to_string()
    }

    fn place(&mut self, p: &Place, indent: usize) -> String {
        let mut out = self.local_read(&p.root);
        for s in &p.steps {
            match s {
                Step::Field(f) => {
                    out.push('.');
                    out.push_str(&kt_ident(f));
                }
                Step::Tuple(i) => {
                    out.push('.');
                    out.push_str(&match i {
                        0 => "first".to_string(),
                        1 => "second".to_string(),
                        2 => "third".to_string(),
                        n => format!("v{n}"),
                    });
                }
                Step::Index(e) => {
                    let i = self.expr(e, indent);
                    out.push_str(&format!("[{i}]"));
                }
            }
        }
        out
    }

    fn local_read(&self, l: &Local) -> String {
        let name = kt_local(l);
        self.aliases.get(&name).cloned().unwrap_or(name)
    }

    // ------------------------------------------------------- expressions --

    fn exprs(&mut self, es: &[Expr], indent: usize) -> Vec<String> {
        es.iter().map(|e| self.expr(e, indent)).collect()
    }

    fn block_value(&mut self, b: &Block, indent: usize) -> String {
        // A block in value position: `run { … }` unless it is a bare value.
        if b.stmts.is_empty() {
            return match &b.value {
                Some(v) => self.expr(v, indent),
                None => "Unit".to_string(),
            };
        }
        let saved = std::mem::take(&mut self.out);
        self.block_stmts(b, indent + 1);
        if let Some(v) = &b.value {
            let code = self.expr(v, indent + 1);
            self.out.push_str(&format!("{}{code}\n", "    ".repeat(indent + 1)));
        }
        let body = std::mem::replace(&mut self.out, saved);
        format!("run {{\n{body}{}}}", "    ".repeat(indent))
    }

    fn expr(&mut self, e: &Expr, indent: usize) -> String {
        let pad = "    ".repeat(indent);
        match &e.kind {
            ExprKind::Int(v) => v.to_string(),
            ExprKind::Long(v) => format!("{v}L"),
            ExprKind::Float(v) => format!("{v:?}f"),
            ExprKind::Double(v) => format!("{v:?}"),
            ExprKind::Bool(v) => v.to_string(),
            ExprKind::Char(c) => format!("'{}'", escape_string(&c.to_string())),
            ExprKind::Str(s) => format!("\"{}\"", escape_string(s)),
            ExprKind::Unit => "Unit".to_string(),
            ExprKind::MakeNone => "null".to_string(),
            ExprKind::Read { place, .. } => self.place(place, indent),
            ExprKind::Call { target, type_args, args } => self.call(e, target, type_args, args, indent),
            ExprKind::MemberCall { instance, member, args, .. } => {
                let inst = self.expr(instance, indent);
                let name = self.member_name(member);
                let a = self.exprs(args, indent);
                format!("{inst}.{name}({})", a.join(", "))
            }
            ExprKind::Op { op, args } => {
                let a = self.exprs(args, indent);
                match op {
                    Op::Add => format!("({} + {})", a[0], a[1]),
                    Op::Sub => format!("({} - {})", a[0], a[1]),
                    Op::Mul => format!("({} * {})", a[0], a[1]),
                    Op::Div => format!("({} / {})", a[0], a[1]),
                    Op::Rem => format!("({} % {})", a[0], a[1]),
                    Op::And => format!("({} && {})", a[0], a[1]),
                    Op::Or => format!("({} || {})", a[0], a[1]),
                    Op::Not => format!("!({})", a[0]),
                    Op::Neg => format!("(-{})", a[0]),
                    Op::Lt => format!("({} < {})", a[0], a[1]),
                    Op::Gt => format!("({} > {})", a[0], a[1]),
                    Op::LtEq => format!("({} <= {})", a[0], a[1]),
                    Op::GtEq => format!("({} >= {})", a[0], a[1]),
                }
            }
            ExprKind::Construct { fields } => {
                let ty = self.construct_ty(&e.ty);
                let fs: Vec<String> = fields
                    .iter()
                    .map(|(n, v)| {
                        let code = self.expr(v, indent);
                        if n.starts_with("__") { code } else { format!("{} = {code}", kt_ident(n)) }
                    })
                    .collect();
                format!("{ty}({})", fs.join(", "))
            }
            ExprKind::MakeUnion { arm, value } => {
                let v = self.expr(value, indent);
                match self.union_arm(&e.ty, *arm) {
                    Some((ctor, _)) => format!("{ctor}({v})"),
                    None => v,
                }
            }
            ExprKind::Rewrap { from, value } => {
                let v = self.expr(value, indent);
                self.rewrap(&v, from, &e.ty)
            }
            ExprKind::DropMut { value } => {
                let v = self.expr(value, indent);
                match value.ty.strip_quals() {
                    Ty::Named { name, .. } => match crate::intrinsics::drop_mut_suffix(name) {
                        Some(suffix) => format!("{v}{suffix}"),
                        None => v,
                    },
                    _ => v,
                }
            }
            ExprKind::Tuple(es) => {
                let a = self.exprs(es, indent);
                match a.len() {
                    2 => format!("Pair({}, {})", a[0], a[1]),
                    3 => format!("Triple({}, {}, {})", a[0], a[1], a[2]),
                    n => {
                        self.s.tuple_sizes.insert(n);
                        format!("Tuple{n}({})", a.join(", "))
                    }
                }
            }
            ExprKind::List(es) => {
                let a = self.exprs(es, indent);
                let elem = match e.ty.strip_quals() {
                    Ty::Named { args, .. } if !args.is_empty() => self.ty(&args[0]),
                    _ => "Any".to_string(),
                };
                let mutable = e.ty.quals().iter().any(|q| q.name == "Mut");
                let f = if mutable { "mutableListOf" } else { "listOf" };
                format!("{f}<{elem}>({})", a.join(", "))
            }
            ExprKind::Array(es) => {
                let a = self.exprs(es, indent);
                let elem = match e.ty.strip_quals() {
                    Ty::Array(el) => self.ty(el),
                    _ => "Any".to_string(),
                };
                format!("arrayOf<{elem}>({})", a.join(", "))
            }
            ExprKind::Concat(parts) => {
                let a = self.exprs(parts, indent);
                format!("({})", a.join(" + "))
            }
            ExprKind::Branch { arms, otherwise, .. } => self.branch(e, arms, otherwise.as_ref(), indent),
            ExprKind::Switch { subject, arms, .. } => self.switch(e, subject, arms, indent),
            ExprKind::Test { subject, test, .. } => {
                let s = self.expr(subject, indent);
                self.test_code(&s, &subject.ty, test)
            }
            ExprKind::Lambda { params, ret, body, .. } => {
                let ps = self.params(params);
                let r = self.ret_ty(ret);
                let saved_unit = std::mem::replace(&mut self.ret_is_unit, ret.is_none_ty());
                let saved = std::mem::take(&mut self.out);
                self.block_stmts(body, indent + 1);
                if let Some(v) = &body.value {
                    let code = self.expr(v, indent + 1);
                    self.out.push_str(&format!("{}return {code}\n", "    ".repeat(indent + 1)));
                }
                let b = std::mem::replace(&mut self.out, saved);
                self.ret_is_unit = saved_unit;
                format!("fun({ps}){r} {{\n{b}{pad}}}")
            }
            ExprKind::FnValue(r) => match r {
                FnRef::Decl(id) => {
                    let (params, ret) = match e.ty.strip_quals() {
                        Ty::Fn { params, ret, .. } => (params.clone(), (**ret).clone()),
                        _ => (Vec::new(), Ty::Unknown),
                    };
                    let names: Vec<String> = (0..params.len()).map(|i| format!("__a{i}")).collect();
                    // An intrinsic has no Kotlin fn: a lambda around its lowering.
                    let file_idx = self.s.program.files.iter().position(|f| f.module == id.module);
                    let decl = file_idx.and_then(|fi| self.s.program.modules[fi].items.get(id.item));
                    if let Some(salvo_syntax::ast::Item::Fn(f)) = decl {
                        if f.intrinsic {
                            let args: Vec<Expr> = names
                                .iter()
                                .zip(&params)
                                .map(|(n, t)| Expr { ty: t.clone(), span: e.span, kind: ExprKind::Read { place: Place { root: Local(n.clone()), steps: Vec::new() }, consume: false } })
                                .collect();
                            let call = Expr { ty: ret, span: e.span, kind: ExprKind::Call { target: FnRef::Decl(id.clone()), type_args: Vec::new(), args } };
                            let body = self.expr(&call, indent);
                            return format!("{{ {} -> {body} }}", names.join(", "));
                        }
                    }
                    // A callable reference cannot name a package, so a foreign
                    // fn is a lambda forwarding to it.
                    let n = self.fn_name(id);
                    if !n.contains('.') {
                        return format!("::{n}");
                    }
                    format!("{{ {} -> {n}({}) }}", names.join(", "), names.join(", "))
                }
                FnRef::Local(l) => kt_local(l),
            },
            ExprKind::Try { body } => self.try_expr(e, body, indent),
            ExprKind::Throw { message } => {
                // [kt-throw-signal] the tag names the message's type, which the
                // catch matches against a union message's arms.
                self.s.needs_throw = true;
                let m = self.expr(message, indent);
                let tag = message.ty.strip_quals().to_string();
                format!("throw ThrowSignal({m}, \"{}\")", escape_string(&tag))
            }
            ExprKind::Assert { cond, message } => {
                let c = self.expr(cond, indent);
                let m = match message {
                    Some(m) => self.expr(m, indent),
                    None => "\"assertion failed\"".to_string(),
                };
                format!("(if (!({c})) throw AssertionError({m}) else Unit)")
            }
            ExprKind::Unreachable { message } => {
                let m = match message {
                    Some(m) => self.expr(m, indent),
                    None => "\"unreachable\"".to_string(),
                };
                format!("throw AssertionError({m})")
            }
            ExprKind::Spawn { .. }
            | ExprKind::Send { .. }
            | ExprKind::ReplyTo { .. }
            | ExprKind::WaitFor { .. }
            | ExprKind::SelfAddr
            | ExprKind::SelfSend { .. } => {
                self.error("actors are not rendered by the IR emitter yet");
                "TODO()".to_string()
            }
            ExprKind::Unsupported(what) => {
                self.error(format!("unsupported IR node: {what}"));
                "TODO()".to_string()
            }
        }
    }

    /// [try] `try { … }`: the body's value in the `Ok` arm of the outcome, a
    /// thrown message in the `Thrown` arm; a union message is told apart by
    /// the signal's tag.
    fn try_expr(&mut self, e: &Expr, body: &Block, indent: usize) -> String {
        self.s.needs_throw = true;
        let pad = "    ".repeat(indent);
        let inner = "    ".repeat(indent + 1);
        let outcome = &e.ty;
        let arms: Vec<Ty> = outcome.strip_quals().value_arms().into_iter().cloned().collect();
        let saved = std::mem::take(&mut self.out);
        self.block_stmts(body, indent + 1);
        let value = match &body.value {
            Some(v) => {
                let code = self.expr(v, indent + 1);
                match self.union_arm(outcome, 0) {
                    Some((ctor, _)) => format!("{ctor}({code})"),
                    None => code,
                }
            }
            None => match self.union_arm(outcome, 0) {
                Some((ctor, _)) => format!("{ctor}(Unit)"),
                None => "Unit".to_string(),
            },
        };
        self.out.push_str(&format!("{inner}{value}\n"));
        let body_code = std::mem::replace(&mut self.out, saved);
        let thrown = match arms.get(1) {
            Some(msg) if msg.strip_quals().value_arms().len() >= 2 => {
                let msg_arms: Vec<Ty> = msg.strip_quals().value_arms().into_iter().cloned().collect();
                let mut cases = Vec::new();
                for (i, a) in msg_arms.iter().enumerate() {
                    let at = self.ty(a);
                    let inner_wrap = match self.union_arm(msg, i) {
                        Some((c, _)) => format!("{c}(__signal.payload as {at})"),
                        None => format!("(__signal.payload as {at})"),
                    };
                    let outer = match self.union_arm(outcome, 1) {
                        Some((c, _)) => format!("{c}({inner_wrap})"),
                        None => inner_wrap,
                    };
                    cases.push(format!("{inner}    \"{}\" -> {outer}", escape_string(&a.strip_quals().to_string())));
                }
                cases.push(format!("{inner}    else -> throw __signal"));
                format!("when (__signal.tag) {{\n{}\n{inner}}}", cases.join("\n"))
            }
            Some(msg) => {
                let at = self.ty(msg);
                match self.union_arm(outcome, 1) {
                    Some((c, _)) => format!("{c}(__signal.payload as {at})"),
                    None => format!("(__signal.payload as {at})"),
                }
            }
            None => "throw __signal".to_string(),
        };
        format!("try {{\n{body_code}{pad}}} catch (__signal: ThrowSignal) {{\n{inner}{thrown}\n{pad}}}")
    }

    fn construct_ty(&mut self, ty: &Ty) -> String {
        match ty.strip_quals() {
            Ty::Named { name, args } => {
                // A platform handler constructs its adapter class.
                let is_platform_handler = self.s.symbols.handlers.get(name.as_str()).is_some_and(|h| h.platform);
                let base = if is_platform_handler {
                    let plain = salvo_core::typekey::plain(name);
                    match self.s.symbols.key_modules.get(name.as_str()) {
                        Some(m) if **m != self.module.path => format!("{}.__Platform_{plain}", kotlin_package(m)),
                        _ => format!("__Platform_{plain}"),
                    }
                } else {
                    self.type_path(name)
                };
                let a: Vec<String> = args.iter().map(|x| self.ty(x)).collect();
                if a.is_empty() { base } else { format!("{base}<{}>", a.join(", ")) }
            }
            other => self.ty(other),
        }
    }

    fn member_name(&self, m: &salvo_ir::MemberRef) -> String {
        let name = self
            .s
            .ir
            .modules
            .iter()
            .flat_map(|md| &md.decls)
            .find_map(|d| match d {
                Decl::Interface(i) if &i.id == &m.interface => i.members.get(m.index).map(|x| x.emitted_name.clone()),
                _ => None,
            })
            .unwrap_or_else(|| format!("member{}", m.index));
        kt_ident(&name)
    }

    fn call(&mut self, e: &Expr, target: &FnRef, type_args: &[Ty], args: &[Expr], indent: usize) -> String {
        match target {
            FnRef::Local(l) => {
                let a = self.exprs(args, indent);
                format!("{}({})", kt_local(l), a.join(", "))
            }
            FnRef::Decl(id) => {
                let file_idx = self.s.program.files.iter().position(|f| f.module == id.module);
                let decl = file_idx.and_then(|fi| self.s.program.modules[fi].items.get(id.item));
                if let Some(salvo_syntax::ast::Item::Fn(f)) = decl {
                    if f.intrinsic {
                        return self.intrinsic_call(e, f, type_args, args, indent);
                    }
                }
                let a = self.exprs(args, indent);
                let name = self.fn_name(id);
                let tas: Vec<String> = type_args.iter().map(|t| self.ty(t)).collect();
                let ta = if tas.is_empty() || tas.iter().any(|t| t == "Any") { String::new() } else { format!("<{}>", tas.join(", ")) };
                let _ = ta;
                format!("{name}({})", a.join(", "))
            }
        }
    }

    fn intrinsic_call(&mut self, e: &Expr, f: &salvo_syntax::ast::FnDecl, type_args: &[Ty], args: &[Expr], indent: usize) -> String {
        let name = f.name.name.as_str();
        let recv = f.params.first().and_then(|p| salvo_backend::emit_util::type_base_name(&p.ty));
        if name == "copy" {
            let v = self.expr(&args[0], indent);
            return match salvo_core::copyplan::copy_plan(self.s.symbols, &args[0].ty) {
                Some(plan) => self.render_copy(&plan, &v),
                None => {
                    self.error(format!("cannot `copy` a value of type `{}`", args[0].ty));
                    v
                }
            };
        }
        if name == "discard" {
            let v = self.expr(&args[0], indent);
            return format!("run {{ {v}; Unit }}");
        }
        if name == "cmp" && recv == Some("Str") {
            self.s.needs_compare = true;
        }
        let a: Vec<String> = args
            .iter()
            .map(|x| {
                let code = self.expr(x, indent);
                // A variadic tail arrives as one array; the lowering wants it spread.
                if matches!(x.ty.strip_quals(), Ty::Array(_)) && f.params.iter().any(|p| p.variadic) {
                    format!("*({code})")
                } else {
                    code
                }
            })
            .collect();
        let tas: Vec<String> = type_args.iter().map(|t| self.ty(t)).collect();
        match crate::intrinsics::fn_call(name, recv, &a, &tas) {
            Some(code) => code,
            None => {
                self.error(format!("intrinsic fn `{name}` is not supported by the kotlin backend"));
                let _ = e;
                "TODO()".to_string()
            }
        }
    }

    fn render_copy(&mut self, plan: &salvo_core::copyplan::CopyPlan, code: &str) -> String {
        use salvo_core::copyplan::CopyPlan;
        match plan {
            CopyPlan::Identity => code.to_string(),
            CopyPlan::StrBuilder => format!("StringBuilder({code})"),
            CopyPlan::Platform(name) => match self.s.symbols.key_modules.get(name.as_str()) {
                Some(m) => format!("{}.copy({code})", host_package(m)),
                None => code.to_string(),
            },
            CopyPlan::Elements { deque, mutable, elem } => {
                let inner = self.render_copy(elem, "__c");
                let mapped = if inner == "__c" { None } else { Some(format!("{code}.map {{ __c -> {inner} }}")) };
                match (*deque, mapped) {
                    (true, Some(m)) => format!("kotlin.collections.ArrayDeque({m})"),
                    (true, None) => format!("kotlin.collections.ArrayDeque({code})"),
                    (false, Some(m)) if *mutable => format!("{m}.toMutableList()"),
                    (false, Some(m)) => m,
                    (false, None) if *mutable => format!("{code}.toMutableList()"),
                    (false, None) => code.to_string(),
                }
            }
            CopyPlan::Struct { fields, .. } => {
                if fields.is_empty() {
                    return format!("{code}.copy()");
                }
                let parts: Vec<String> = fields
                    .iter()
                    .map(|(f, p)| {
                        let inner = self.render_copy(p, &format!("__s.{}", kt_ident(f)));
                        format!("{} = {inner}", kt_ident(f))
                    })
                    .collect();
                format!("{code}.let {{ __s -> __s.copy({}) }}", parts.join(", "))
            }
            CopyPlan::Array => format!("{code}.copyOf()"),
            CopyPlan::Recur { name, mutable } => {
                // [kt-copy] the struct's own copy fn, generated at module end.
                self.s.recursive_copies.insert((name.clone(), *mutable));
                let plain = salvo_core::typekey::plain(name).replace('.', "_");
                let fname = format!("__copy{}_{plain}", if *mutable { "Mut" } else { "" });
                match self.s.symbols.key_modules.get(name.as_str()) {
                    Some(m) if **m != self.module.path => format!("{}.{fname}({code})", kotlin_package(m)),
                    _ => format!("{fname}({code})"),
                }
            }
        }
    }

    fn rewrap(&mut self, code: &str, from: &Ty, to: &Ty) -> String {
        let from_arms: Vec<&Ty> = from.strip_quals().value_arms();
        let to_arms: Vec<&Ty> = to.strip_quals().value_arms();
        if to_arms.len() < 2 {
            return format!("({code} as {})", self.ty(to));
        }
        let n = from_arms.len();
        let mut arms = Vec::new();
        for (i, a) in from_arms.iter().enumerate() {
            if let Some(j) = to_arms.iter().position(|b| b == a) {
                if let Some((ctor, _)) = self.union_arm(to, j) {
                    let at = self.ty(a);
                    let stars = vec!["*"; n].join(", ");
                    arms.push(format!("is Union{n}.U{}<{stars}> -> {ctor}(it.value as {at})", i + 1));
                }
            }
        }
        if from.strip_quals().has_none_arm() {
            arms.push("null -> null".to_string());
        }
        arms.push("else -> throw IllegalStateException(\"salvo: unreachable union arm\")".to_string());
        format!("{code}.let {{ when (it) {{ {} }} }}", arms.join("; "))
    }

    fn test_code(&mut self, subject: &str, subj_ty: &Ty, test: &ArmTest) -> String {
        let n = subj_ty.strip_quals().value_arms().len();
        match test {
            ArmTest::None => format!("({subject} == null)"),
            ArmTest::Else => "true".to_string(),
            ArmTest::Arm(a) => {
                if n < 2 {
                    format!("({subject} != null)")
                } else {
                    let stars = vec!["*"; n].join(", ");
                    format!("({subject} is Union{n}.U{}<{stars}>)", a + 1)
                }
            }
            ArmTest::Arms(arms) => {
                let stars = vec!["*"; n].join(", ");
                let parts: Vec<String> = arms.iter().map(|a| format!("{subject} is Union{n}.U{}<{stars}>", a + 1)).collect();
                format!("({})", parts.join(" || "))
            }
            ArmTest::Lit(lits) => {
                let parts: Vec<String> = lits
                    .iter()
                    .map(|l| match l {
                        Lit::Str(s) => format!("{subject} == \"{}\"", escape_string(s)),
                        Lit::Int(i) => format!("{subject} == {i}"),
                        Lit::Long(i) => format!("{subject} == {i}L"),
                        Lit::Bool(b) => format!("{subject} == {b}"),
                    })
                    .collect();
                format!("({})", parts.join(" || "))
            }
        }
    }

    fn branch(&mut self, e: &Expr, arms: &[(Expr, Block)], otherwise: Option<&Block>, indent: usize) -> String {
        let pad = "    ".repeat(indent);
        let value = !e.ty.is_none_ty() && arms.iter().any(|(_, b)| b.value.is_some());
        // A block expression: one `true` arm.
        if arms.len() == 1 && matches!(arms[0].0.kind, ExprKind::Bool(true)) && otherwise.is_none() {
            return if value {
                self.block_value(&arms[0].1, indent)
            } else {
                let saved = std::mem::take(&mut self.out);
                self.block_stmts(&arms[0].1, indent + 1);
                let b = std::mem::replace(&mut self.out, saved);
                format!("run {{\n{b}{pad}}}")
            };
        }
        let mut out = String::new();
        for (i, (c, b)) in arms.iter().enumerate() {
            let cond = self.expr(c, indent);
            out.push_str(if i == 0 { "if (" } else { " else if (" });
            out.push_str(&cond);
            out.push_str(") {\n");
            let saved = std::mem::take(&mut self.out);
            self.block_stmts(b, indent + 1);
            if let Some(v) = &b.value {
                let code = self.expr(v, indent + 1);
                self.out.push_str(&format!("{}{code}\n", "    ".repeat(indent + 1)));
            }
            let body = std::mem::replace(&mut self.out, saved);
            out.push_str(&body);
            out.push_str(&pad);
            out.push('}');
        }
        match otherwise {
            Some(b) => {
                out.push_str(" else {\n");
                let saved = std::mem::take(&mut self.out);
                self.block_stmts(b, indent + 1);
                if let Some(v) = &b.value {
                    let code = self.expr(v, indent + 1);
                    self.out.push_str(&format!("{}{code}\n", "    ".repeat(indent + 1)));
                }
                let body = std::mem::replace(&mut self.out, saved);
                out.push_str(&body);
                out.push_str(&pad);
                out.push('}');
            }
            None if value => out.push_str(" else null"),
            None => {}
        }
        if value { format!("({out})") } else { out }
    }

    fn switch(&mut self, e: &Expr, subject: &Expr, arms: &[salvo_ir::SwitchArm], indent: usize) -> String {
        let pad = "    ".repeat(indent);
        let value = !e.ty.is_none_ty() && arms.iter().any(|a| a.body.value.is_some());
        let s = self.expr(subject, indent);
        let mut out = String::from("when {\n");
        for arm in arms {
            let cond = self.test_code(&s, &subject.ty, &arm.test);
            let cond = if matches!(arm.test, ArmTest::Else) { "else".to_string() } else { cond };
            out.push_str(&format!("{}{cond} -> {{\n", "    ".repeat(indent + 1)));
            let saved = std::mem::take(&mut self.out);
            // Narrowing bindings from this subject read the arm's payload.
            self.narrowing_subject = Some((s.clone(), subject.ty.clone(), arm.test.clone()));
            self.block_stmts(&arm.body, indent + 2);
            if let Some(v) = &arm.body.value {
                let code = self.expr(v, indent + 2);
                self.out.push_str(&format!("{}{code}\n", "    ".repeat(indent + 2)));
            }
            self.narrowing_subject = None;
            let body = std::mem::replace(&mut self.out, saved);
            out.push_str(&body);
            out.push_str(&format!("{}}}\n", "    ".repeat(indent + 1)));
        }
        if !arms.iter().any(|a| matches!(a.test, ArmTest::Else)) && value {
            // The checker proved the arms total; Kotlin wants an `else` anyway.
            out.push_str(&format!("{}else -> throw IllegalStateException(\"salvo: unreachable arm\")\n", "    ".repeat(indent + 1)));
        }
        out.push_str(&pad);
        out.push('}');
        out
    }
}

/// Two types equal up to qualifiers (the IR keeps only `Mut`).
fn erase_eq(a: &Ty, b: &Ty) -> bool {
    a.strip_quals() == b.strip_quals()
}
