//! [kt-ir] The Kotlin emitter: renders `salvo_ir` modules (IR record). It
//! replaced the AST emitter on 2026-10-06.
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

mod actors;
mod boundary;
pub mod skeletons;

pub fn emit_program_ir(program: &Program, abi: bool) -> Result<(Vec<EmittedFile>, Vec<String>), Vec<String>> {
    // The IR is built from the *checked* program itself (the checker's
    // `written_types` are keyed by AST node); effect-generic erasure
    // [effect-generic-decl] is applied where a type is rendered.
    let (_erased_program, erased, mut checked, warnings) = salvo_backend::driver::check_for_emission(program)?;
    let (symbols, resolution) = salvo_backend::driver::resolve_for_emission(program, &mut checked);
    let reach = salvo_backend::driver::reach(program, &resolution, &symbols, &checked, abi);
    let wanted: HashSet<&ModulePath> = reach.emitted.iter().copied().collect();
    let (ir, build_errors) = salvo_ir::build_program(program, &symbols, &resolution, &checked, Some(&wanted));
    if !build_errors.is_empty() {
        return Err(build_errors);
    }
    let fn_names = salvo_core::naming::FnNames::compute(program);
    let mut shared = Shared {
        program,
        symbols: &symbols,
        erased: &erased,
        abi,
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
    // [platform-handler] [platform-fn] [platform-tree] A module whose
    // platform declarations are emitted needs its host implementation file.
    for m in &ir.modules {
        if abi {
            break;
        }
        let declares = m.decls.iter().any(|d| match d {
            Decl::Fn(f) => f.kind == FnKind::Platform,
            Decl::Impl(h) => h.platform,
            Decl::PlatformType(t) => t.platform,
            _ => false,
        });
        if declares && salvo_core::host_file(&program.companions, &m.path).is_none() {
            let what: String = program.units().filter(|u| u.file.module == m.path).map(|u| salvo_core::platform_declarations(u.ast)).collect::<Vec<_>>().join(", ");
            let rel = salvo_core::host_rel_path(&m.path, "kt");
            shared.errors.push(salvo_core::missing_handler_host_error(&what, &m.path, &rel));
        }
    }
    let mut files = Vec::new();
    for m in &ir.modules {
        // [platform-abi] A host project's module carries the declarations the
        // platform surface reaches (and its platform items); the runtime and
        // the stream table's service travel whole.
        let keep: Option<&BTreeSet<String>> = if abi && !reach.abi_full.contains(&m.path) { reach.closure.as_ref() } else { None };
        let text = ModuleEmitter::new(&mut shared, m).module_with(keep);
        let mut rel_path = std::path::PathBuf::new();
        for part in &m.path.0 {
            rel_path.push(part);
        }
        // [platform-abi] a host project's declaration files are `<m>.sv.kt`.
        rel_path.set_extension(if abi { "sv.kt" } else { "kt" });
        files.push(EmittedFile { rel_path, content: text });
    }
    // Runtime files [kt-runtime].
    let features = {
        let mut f = salvo_core::features::RuntimeFeatures::default();
        for (file_idx, unit) in program.units().enumerate() {
            if wanted.contains(&unit.file.module) {
                let keep = if abi && !reach.abi_full.contains(&unit.file.module) { reach.closure.as_ref() } else { None };
                f.or(salvo_core::features::module_features(&symbols, &checked, file_idx, unit.ast, keep.is_some(), |i| salvo_backend::emit_util::abi_emits(i, keep)));
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
        files.push(EmittedFile { rel_path: "wire.kt".into(), content: include_str!("../../runtime/wire.kt").to_string() });
    }
    if features.scheduler {
        files.push(EmittedFile { rel_path: "scheduler.kt".into(), content: crate::emit::generate_scheduler_file() });
    }
    if features.time() {
        files.push(EmittedFile { rel_path: "hosttime.kt".into(), content: crate::emit::generate_time_file() });
    }
    // [stream-table] Host code's entry points into the stream table ship
    // wherever the table (the Salvo service `runtime.streams`) does.
    if reach.emitted.iter().any(|m| m.0 == ["runtime", "streams"]) {
        files.push(EmittedFile { rel_path: "hoststreams.kt".into(), content: include_str!("../../runtime/hoststreams.kt").to_string() });
    }
    // Host companions travel with their module [backend-companion]. In ABI
    // mode only the platform companions a host project needs: the runtime's
    // and every other emitted module's that is not the project's own.
    for comp in &program.companions {
        if abi
            && !(comp.platform
                && (reach.abi_full.contains(&comp.module)
                    || (reach.emitted.contains(&comp.module) && !salvo_backend::emit_util::is_project_module(program, &comp.module))))
        {
            continue;
        }
        if !abi && !reach.reachable.contains(&comp.module) {
            continue;
        }
        if files.iter().any(|f| f.rel_path == comp.rel_path) {
            shared.errors.push(format!("companion file `{}` collides with a generated file", comp.rel_path.display()));
            continue;
        }
        files.push(EmittedFile { rel_path: comp.rel_path.clone(), content: comp.content.clone() });
    }
    if abi {
        // [platform-abi] The runtime lives under `salvo/` in the root, beside
        // the module declaration files, every file as `.sv.kt`.
        for f in &mut files {
            if !f.rel_path.to_string_lossy().ends_with(".sv.kt") {
                let mut rel = f.rel_path.clone();
                if let Ok(rest) = rel.strip_prefix(salvo_core::source::PLATFORM_DIR) {
                    rel = rest.to_path_buf();
                }
                rel.set_extension("sv.kt");
                f.rel_path = std::path::PathBuf::from("salvo").join(rel);
            }
        }
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
    /// [effect-generic-decl] the declarations rendered without generics.
    erased: &'p salvo_core::Erased,
    /// [platform-abi] writing a host project's declaration files.
    abi: bool,
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
    /// The impl whose member is being rendered, for `replyto`.
    current_impl: Option<String>,
    /// [kt-monitor] the lock each handled instance's monitors share, per fn.
    handle_locks: HashMap<Local, String>,
    /// [kt-mixed] inside `__Fac_H`: a self-send goes to the servant's addr.
    in_facade: bool,
    /// The arm test behind each `Test` node (arm `usize::MAX`) and each
    /// `Switch` arm, by node id: what a `Narrow`'s justification names.
    test_arms: HashMap<(u32, usize), ArmTest>,
    /// The locals the current fn assigns after binding (`var`, not `val`).
    assigned: HashSet<Local>,
}

fn kt_local(l: &Local) -> String {
    let s = l.0.replace('~', "_");
    if s.starts_with("__") { s } else { kt_ident(&s) }
}

impl<'a, 'p> ModuleEmitter<'a, 'p> {
    fn new(s: &'a mut Shared<'p>, module: &'a IrModule) -> Self {
        ModuleEmitter { s, module, out: String::new(), aliases: HashMap::new(), in_fn_params: HashSet::new(), narrowing_subject: None, ret_is_unit: false, current_impl: None, handle_locks: HashMap::new(), in_facade: false, test_arms: HashMap::new(), assigned: HashSet::new() }
    }

    fn error(&mut self, msg: impl Into<String>) {
        self.s.errors.push(format!("{}: {}", self.module.file_name, msg.into()));
    }

    #[cfg(test)]
    fn module(self) -> String {
        self.module_with(None)
    }

    /// The module's Kotlin; with `keep`, only the declarations named there
    /// and the platform items, as declarations [platform-abi].
    fn module_with(mut self, keep: Option<&BTreeSet<String>>) -> String {
        let kept = |d: &Decl| -> bool {
            let Some(keep) = keep else { return true };
            match d {
                Decl::Struct(x) => keep.contains(&x.name) && !x.name.contains('.'),
                Decl::Union(x) => keep.contains(&x.name),
                Decl::Interface(x) => keep.contains(&x.name),
                Decl::Impl(h) => h.platform,
                Decl::Fn(f) => f.kind == FnKind::Platform,
                Decl::PlatformType(t) => t.platform,
                Decl::Static(_) => false,
                Decl::Enum(e) => matches!(e.kind, salvo_ir::EnumKind::Message { .. }) && keep.contains(e.name.trim_start_matches("__Msg_")),
            }
        };
        let mut body = String::new();
        // [kt-nested-dot-name] Dot-named structs nest in their namespace:
        // the namespace struct's body, or an `object` for a `type` namespace.
        let nested: Vec<&StructDecl> = self.module.decls.iter().filter_map(|d| match d {
            Decl::Struct(s) if s.name.contains('.') => Some(s),
            _ => None,
        }).collect();
        let mut namespaces_done: HashSet<String> = HashSet::new();
        for d in &self.module.decls {
            if !kept(d) {
                continue;
            }
            self.out.clear();
            match d {
                Decl::Struct(s) if s.name.contains('.') => {
                    let ns = s.name.split('.').next().unwrap().to_string();
                    let has_struct = self.module.decls.iter().any(|d| matches!(d, Decl::Struct(x) if x.name == ns));
                    if !has_struct && !namespaces_done.contains(&ns) {
                        namespaces_done.insert(ns.clone());
                        self.out.push_str(&format!("object {ns} {{\n"));
                        let members: Vec<&StructDecl> = nested.iter().copied().filter(|x| x.name.split('.').next() == Some(ns.as_str())).collect();
                        for m in &members {
                            self.struct_class(m, 1);
                        }
                        self.out.push_str("}\n");
                        for m in &members {
                            if m.has_wire_form {
                                self.struct_codec(m);
                            }
                        }
                    }
                }
                Decl::Struct(s) if nested.iter().any(|x| x.name.split('.').next() == Some(s.name.as_str())) => {
                    let members: Vec<&StructDecl> = nested.iter().copied().filter(|x| x.name.split('.').next() == Some(s.name.as_str())).collect();
                    self.struct_decl_with(s, &members);
                }
                _ => self.decl(d),
            }
            if !self.out.is_empty() {
                body.push_str(&self.out);
                body.push('\n');
            }
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
            Some(_) => plain.to_string(),
            // [actor-msg] A generated enum is keyed by its module.
            None => match self.generated_enum(key) {
                Some((_, m)) if m != self.module.path => format!("{}.{plain}", kotlin_package(&m)),
                _ => plain.to_string(),
            },
        }
    }

    /// [actor-msg] The generated enum a type key names, and its module.
    fn generated_enum(&self, key: &str) -> Option<(&'p salvo_ir::EnumDecl, salvo_core::ModulePath)> {
        let module = salvo_core::typekey::module_of(key)?;
        self.s.ir.modules.iter().filter(|m| m.path.0.join(".") == module).flat_map(|m| &m.decls).find_map(|d| match d {
            Decl::Enum(e) if salvo_ir::enum_key(&e.name, &e.id.module) == key => Some((e, e.id.module.clone())),
            _ => None,
        })
    }

    // ------------------------------------------------------------- types --

    fn ty(&mut self, t: &Ty) -> String {
        match t {
            Ty::Named { name, args } => {
                // [cmp-carry] an identity a container carries is not a type argument.
                // [effect-generic-decl] an erased declaration's arguments are dropped.
                let erased = self.s.erased.is_erased(salvo_core::typekey::plain(name));
                let args: Vec<Ty> = if erased { Vec::new() } else { args.iter().filter(|a| !matches!(a, Ty::FnName(_))).cloned().collect() };
                let args = &args;
                if let Some(kt) = crate::intrinsics::type_name(name) {
                    if name == "Bytes" {
                        self.s.needs_bytes = true;
                    }
                    if name == "Addr" && args.len() == 1 {
                        // [monitor-handler] [kt-monitor] a plain effect's addr
                        // is the effect's handle, not a scheduler index.
                        if let Ty::Named { name: eff, .. } = args[0].strip_quals() {
                            let plain = salvo_core::typekey::plain(eff).to_string();
                            if self.interface_by_name(&plain).is_some_and(|i| !i.actor) {
                                return self.ty(&args[0]);
                            }
                        }
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
                        // [cmp-carry] an identity a container carries is not a type argument.
                        let args: Vec<Ty> = args.iter().filter(|a| !matches!(a, Ty::FnName(_))).cloned().collect();
                        let args = &args;
                        // [platform-value-type] The host names a value type's
                        // mutable kind `Mut<Name>` beside it.
                        let platform_mut = self.s.symbols.intrinsic_types.get(name.as_str()).is_some_and(|t| t.platform && !t.linear && t.auto_qualifiers.iter().any(|q| q.name.name == "Mut"));
                        if platform_mut {
                            if let Some(m) = self.s.symbols.key_modules.get(name.as_str()) {
                                let a: Vec<String> = args.iter().map(|x| self.ty(x)).collect();
                                let base_name = format!("{}.Mut{}", host_package(m), salvo_core::typekey::plain(name));
                                return if a.is_empty() { base_name } else { format!("{base_name}<{}>", a.join(", ")) };
                            }
                        }
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

    /// A declaration's generic parameter list; empty for an erased one
    /// [effect-generic-decl].
    fn decl_type_params(&self, name: &str, tps: &[salvo_ir::TypeParam]) -> String {
        if self.s.erased.is_erased(name) {
            String::new()
        } else {
            self.type_params(tps)
        }
    }

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
            Decl::Union(u) => {
                // [platform-factory] a named union a host builds.
                if let Some(fx) = u.factories.clone() {
                    self.factory_object(&format!("{}s", u.name), &[fx]);
                }
            }
            Decl::Interface(i) => self.interface_decl(i),
            Decl::Impl(h) => self.impl_decl(h),
            Decl::Fn(f) => self.fn_decl(f, 0, None, false),
            Decl::PlatformType(_) => {}
            Decl::Enum(e) => self.enum_decl(e),
            Decl::Static(st) => {
                // [mod-use] a lazy module-level instance.
                let ty = self.ty(&st.ty);
                let name = kt_local(&st.local);
                let _ = &name;
                self.out.push_str(&format!("val {name}: {ty} by lazy {{\n"));
                self.assigned = assigned_locals(&Block { stmts: st.stmts.clone(), value: None });
                let saved = std::mem::take(&mut self.out);
                self.stmts(&st.stmts, 1);
                let body = std::mem::replace(&mut self.out, saved);
                self.out.push_str(&body);
                self.out.push_str(&format!("    {name}\n}}\n"));
            }
        }
    }

    fn struct_decl(&mut self, s: &StructDecl) {
        self.struct_decl_with(s, &[]);
    }

    /// A struct, with the dot-named structs nested in it [kt-nested-dot-name].
    fn struct_decl_with(&mut self, s: &StructDecl, members: &[&StructDecl]) {
        self.struct_class(s, 0);
        if !members.is_empty() {
            // Reopen the class with a body for the nested members.
            let trimmed = self.out.trim_end_matches('\n').to_string();
            self.out = format!("{trimmed} {{\n");
            for m in members {
                self.struct_class(m, 1);
            }
            self.out.push_str("}\n");
        }
        if s.has_wire_form {
            self.struct_codec(s);
        }
        for m in members {
            if m.has_wire_form {
                self.struct_codec(m);
            }
        }
    }

    /// The class line(s) of a struct, by its last name segment.
    fn struct_class(&mut self, s: &StructDecl, indent: usize) {
        let pad = "    ".repeat(indent);
        let tps = self.decl_type_params(&s.name, &s.type_params);
        let name = s.name.rsplit('.').next().unwrap_or(&s.name).to_string();
        if s.fields.is_empty() {
            self.out.push_str(&format!("{pad}class {name}{tps}\n"));
        } else {
            self.out.push_str(&format!("{pad}data class {name}{tps}(\n"));
            for f in &s.fields {
                // [kt-field-canbe-mut] One property for both shapes, which
                // needs the `Mut` rendering to be a subtype of the plain one.
                if f.canbe_mut {
                    if let Ty::Named { name, .. } = f.ty.strip_quals() {
                        if crate::intrinsics::drop_mut_suffix(name).is_some() {
                            self.error(format!("the kotlin backend cannot store `canbe Mut` field `{}` yet: its type's `Mut` form is not a subtype of the plain one [kt-field-canbe-mut]", f.name));
                        }
                    }
                }
                let ty = self.ty(&f.ty);
                let kw = if s.canbe_mut { "var" } else { "val" };
                // A field's default is the parameter's too, so a host can
                // leave it out [struct-defaults].
                let default = match &f.default {
                    Some(e) => format!(" = {}", self.expr(e, indent + 1)),
                    None => String::new(),
                };
                self.out.push_str(&format!("{pad}    {kw} {}: {ty}{default},\n", kt_ident(&f.name)));
            }
            self.out.push_str(&format!("{pad})\n"));
        }
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
        let tps = self.decl_type_params(&i.name, &i.type_params);
        self.out.push_str(&format!("interface {}{tps} {{\n", i.name));
        for m in &i.members {
            let ps = self.params(&m.params);
            let ret = self.ret_ty(&m.ret);
            let mtps = self.type_params(&m.type_params);
            self.out.push_str(&format!("    fun{mtps} {}({ps}){ret}\n", kt_ident(&m.emitted_name)));
        }
        self.out.push_str("}\n");
        self.monitor_stub(i);
        if i.actor {
            self.actor_interface(i);
        }
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
                // [platform-check] what the host hands back is checked (D7):
                // the result, and what it sends on a `Reply<T>`. A host
                // project's adapter only has to type-check the implementation.
                let checking = !self.s.abi;
                let mut args: Vec<String> = Vec::new();
                for p in &m.params {
                    let name = kt_local(&p.local);
                    let payload = match p.ty.strip_quals() {
                        Ty::Named { name: n, args } if n == "Reply" && args.len() == 1 => Some(args[0].clone()),
                        _ => None,
                    };
                    match (p.check.as_ref().filter(|_| checking), payload) {
                        (Some(plan), Some(payload)) => {
                            let ty = self.ty(&payload);
                            let what = format!("what the host sent on `{}.{}`", i.name, m.name);
                            let body = self.boundary_check(plan, "__c", &what, 3);
                            args.push(format!("{name}.checked {{ __any ->\n            @Suppress(\"UNCHECKED_CAST\") val __c = __any as {ty}\n{body}            __c\n        }}"));
                        }
                        _ => args.push(name),
                    }
                }
                let call = format!("impl.{}({})", kt_ident(&m.emitted_name), args.join(", "));
                match m.result_check.as_ref().filter(|_| checking) {
                    Some(plan) => {
                        let what = format!("`{}.{}`'s result", i.name, m.name);
                        let body = self.boundary_check(plan, "__r", &what, 2);
                        self.out.push_str(&format!("    override fun {}({ps}){ret} {{\n        val __r = {call}\n{body}        return __r\n    }}\n", kt_ident(&m.emitted_name)));
                    }
                    None => self.out.push_str(&format!("    override fun {}({ps}){ret} = {call}\n", kt_ident(&m.emitted_name))),
                }
            }
            self.out.push_str("}\n");
            // [platform-factory] the factories of each member's result and
            // reply payloads.
            for m in &i.members {
                if !m.factories.is_empty() {
                    let sets = m.factories.clone();
                    self.factory_object(&salvo_core::abi::upper_camel(&m.name), &sets);
                }
            }
            let _ = impls;
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
        if h.intrinsic {
            // [backend-intrinsic] [backend-never-wrong] no intrinsic handler
            // has a Kotlin lowering.
            self.error(format!("intrinsic handler `{}` is not supported by the kotlin backend", h.name));
            return;
        }
        if h.platform {
            // [platform-abi] [kt-platform-handler] A platform handler is its
            // effect's adapter with the handler's constructor: what a `use`
            // constructs, so the implementation is only reached through it.
            let Some(Ty::Named { name: face, .. }) = h.faces.first().map(|f| f.strip_quals().clone()) else { return };
            let plain = salvo_core::typekey::plain(&face).to_string();
            let Some(iface) = self.interface_by_name(&plain) else {
                self.error(format!("platform handler `{}` implements `{plain}`, whose declaration could not be located", h.name));
                return;
            };
            let ps = self.params(&h.ctor_params);
            let args: Vec<String> = h.ctor_params.iter().map(|p| kt_local(&p.local)).collect();
            self.out.push_str(&format!(
                "\nclass __Platform_{}({ps}) : {}__Platform_{}({}.{}({}))\n",
                h.name,
                self.pkg_prefix(&iface.id.module),
                iface.name,
                host_package(&h.id.module),
                h.name,
                args.join(", ")
            ));
            return;
        }
        let tps = self.decl_type_params(&h.name, &h.type_params);
        let mut ctor: Vec<String> = Vec::new();
        for p in &h.ctor_params {
            let ty = self.ty(&p.ty);
            ctor.push(format!("private val {}: {ty}", kt_local(&p.local)));
        }
        for (i, d) in h.deps.iter().enumerate() {
            let ty = self.ty(d);
            ctor.push(format!("private val __dep{i}: {ty}"));
        }
        let faces = self.actor_faces(h);
        let is_actor = !faces.is_empty();
        if is_actor && faces.len() != h.faces.len() {
            self.error(format!("handler `{}` mixes actor and plain effects, which the IR emitter does not render yet", h.name));
        }
        // [mixed-handler] [kt-mixed] plain faces with `send fn` members: the
        // class is the servant alone; the faces are worn by `__Fac_H`.
        let mixed = self.is_mixed(h);
        let face_tys: Vec<String> = h.faces.iter().map(|f| self.ty(f)).collect();
        let supers = if mixed || face_tys.is_empty() { String::new() } else { format!(" : {}", face_tys.join(", ")) };
        let ctor_list = if ctor.is_empty() { String::new() } else { format!("({})", ctor.join(", ")) };
        self.out.push_str(&format!("class {}{tps}{ctor_list}{supers} {{\n", h.name));
        if is_actor || mixed {
            let fields = self.actor_fields(h);
            self.out.push_str(&fields);
        }
        self.current_impl = Some(h.name.clone());
        self.in_facade = false;
        for f in &h.state {
            let ty = self.ty(&f.ty);
            let init = match &f.default {
                Some(e) => self.expr(e, 1),
                None => "TODO()".to_string(),
            };
            self.out.push_str(&format!("    var {}: {ty} = {init}\n", kt_ident(&f.name)));
        }
        if let Some(init) = &h.init {
            if is_actor || mixed {
                // [handler-init] an actor's `init` is its first activation,
                // sent by the spawn as `__Priv_H.Init`.
                self.in_fn_params.clear();
                self.aliases.clear();
                self.handle_locks.clear();
                self.ret_is_unit = true;
                self.out.push_str("    fun init() {\n");
            } else {
                self.out.push_str("    init {\n");
            }
            self.assigned = init.body.as_ref().map(assigned_locals).unwrap_or_default();
            if let Some(b) = &init.body {
                self.block_stmts(b, 2);
            }
            self.out.push_str("    }\n");
        }
        let privs: Vec<String> = Self::private_sends(h, &faces).iter().map(|m| m.name.clone()).collect();
        for m in &h.members {
            if mixed && !m.send {
                continue;
            }
            let private = privs.contains(&m.name);
            let name = self.impl_member_name_of(h, &m.name, Some(m));
            self.fn_decl(m, 1, Some(name), !private);
        }
        self.out.push_str("}\n");
        if mixed {
            // The façade: the sync members, sending to the servant.
            let mut fac_ctor: Vec<String> = vec!["private val __addr: Int".to_string()];
            for p in &h.ctor_params {
                let ty = self.ty(&p.ty);
                fac_ctor.push(format!("private val {}: {ty}", kt_local(&p.local)));
            }
            self.out.push_str(&format!("\nclass __Fac_{}{tps}({}) : {} {{\n", h.name, fac_ctor.join(", "), face_tys.join(", ")));
            self.in_facade = true;
            for m in h.members.iter().filter(|m| !m.send) {
                let name = self.impl_member_name_of(h, &m.name, Some(m));
                self.fn_decl(m, 1, Some(name), true);
            }
            self.in_facade = false;
            self.out.push_str("}\n");
        }
        self.current_impl = None;
        if is_actor || mixed {
            self.actor_body(h, &faces);
        }
    }

    /// [mixed-handler] Every face a plain effect, and a `send fn` member.
    pub(crate) fn is_mixed(&self, h: &ImplDecl) -> bool {
        !h.faces.is_empty() && h.members.iter().any(|m| m.send) && self.actor_faces(h).is_empty()
    }

    fn fn_decl(&mut self, f: &FnDecl, indent: usize, member: Option<String>, overrides: bool) {
        if f.kind == FnKind::Intrinsic {
            return;
        }
        let pad = "    ".repeat(indent);
        // [effect-generic-decl] a fn generic only over effects is emitted without them.
        let file_idx = self.s.program.files.iter().position(|x| x.module == f.id.module);
        let erased_fn = file_idx.is_some_and(|fi| match self.s.program.modules[fi].items.get(f.id.item) {
            Some(salvo_syntax::ast::Item::Fn(af)) => self.s.erased.fns.contains(&(fi, af.name.span.start)),
            _ => false,
        });
        let tps = if erased_fn { String::new() } else { self.type_params(&f.type_params) };
        let ps = self.params(&f.params);
        let ret = self.ret_ty(&f.ret);
        let is_member = member.is_some();
        let kw = if is_member && overrides { "override fun" } else { "fun" };
        let name = match member {
            Some(n) => n,
            None => {
                let own = self.fn_name(&f.id);
                own.rsplit('.').next().unwrap_or(&own).to_string()
            }
        };
        self.in_fn_params = f.params.iter().map(|p| kt_local(&p.local)).collect();
        self.assigned = f.body.as_ref().map(assigned_locals).unwrap_or_default();
        self.aliases.clear();
        self.handle_locks.clear();
        self.ret_is_unit = f.ret.is_none_ty();
        if f.kind == FnKind::Platform {
            // [platform-fn] the host's fn, by its package.
            let args: Vec<String> = f
                .params
                .iter()
                .skip(f.effect_params)
                .map(|p| kt_local(&p.local))
                .collect();
            let call = format!("{}.{}({})", host_package(&f.id.module), kt_ident(&f.name), args.join(", "));
            // [platform-check] the wrapper checks the result (D7); a host
            // project's wrappers carry no checks.
            match f.result_check.as_ref().filter(|_| !self.s.abi) {
                Some(plan) => {
                    let what = format!("`platform fn {}`'s result", f.name);
                    let body = self.boundary_check(plan, "__r", &what, indent + 1);
                    let inner = "    ".repeat(indent + 1);
                    self.out.push_str(&format!("{pad}{kw}{tps} {name}({ps}){ret} {{\n{inner}val __r = {call}\n{body}{inner}return __r\n{pad}}}\n"));
                }
                None => self.out.push_str(&format!("{pad}{kw}{tps} {name}({ps}){ret} = {call}\n")),
            }
            // [platform-factory] the result's factories, after the wrapper.
            if let Some(fx) = f.factories.clone() {
                self.factory_object(&salvo_core::abi::upper_camel(&f.name), &[fx]);
            }
            return;
        }
        let Some(body) = &f.body else {
            self.out.push_str(&format!("{pad}{kw}{tps} {name}({ps}){ret}\n"));
            return;
        };
        let saved = std::mem::take(&mut self.out);
        if !is_member && f.name == "main" && indent == 0 {
            let prelude = self.protocol_prelude();
            self.out.push_str(&prelude);
        }
        self.block_stmts(body, indent + 1);
        if let Some(v) = &body.value {
            let code = self.expr(v, indent + 1);
            self.out.push_str(&format!("{}return {code}\n", "    ".repeat(indent + 1)));
        }
        let body_code = std::mem::replace(&mut self.out, saved);
        // [kt-suppress-cast] A body that casts (a union payload, a `canbe
        // Mut` field) is annotated: neither the unchecked nor the useless
        // cast warning is the author's to silence.
        if body_code.contains(" as ") {
            self.out.push_str(&format!("{pad}@Suppress(\"UNCHECKED_CAST\", \"USELESS_CAST\", \"UNNECESSARY_SAFE_CALL\")\n"));
        }
        self.out.push_str(&format!("{pad}{kw}{tps} {name}({ps}){ret} {{\n{body_code}{pad}}}\n"));
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
            Stmt::Unpack { locals, from, from_ty, variant, .. } => {
                // [actor-dispatch] The payload of the variant just tested.
                let msg = kt_local(&from.root);
                if let Ty::Named { name, .. } = from_ty.strip_quals() {
                    if let Some((e, _)) = self.generated_enum(name) {
                        let path = self.type_path(name);
                        let v = &e.variants[*variant];
                        for ((l, _), (field, _)) in locals.iter().zip(&v.fields) {
                            self.out.push_str(&format!("{pad}val {} = ({msg} as {path}.{}).{}\n", kt_local(l), v.name, kt_local(&Local(field.clone()))));
                        }
                    }
                }
            }
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
                if let ExprKind::Handle { instance } = &value.kind {
                    if let ExprKind::Read { place, .. } = &instance.kind {
                        if place.steps.is_empty() && !self.handle_locks.contains_key(&place.root) {
                            let lock = format!("__lock_{}", kt_local(&place.root));
                            self.out.push_str(&format!("{pad}val {lock} = java.util.concurrent.locks.ReentrantLock()\n"));
                            self.handle_locks.insert(place.root.clone(), lock);
                        }
                    }
                }
                let code = self.expr(value, indent);
                let t = self.ty(ty);
                // `val` unless the fn assigns the local somewhere.
                let kw = if self.assigned.contains(local) { "var" } else { "val" };
                self.out.push_str(&format!("{pad}{kw} {}: {t} = {code}\n", kt_local(local)));
            }
            Stmt::Alias { local, ty, place, .. } => {
                // [ir-alias] a reference to the place's object.
                let code = self.place(place, indent);
                let t = self.ty(ty);
                self.out.push_str(&format!("{pad}val {}: {t} = {code}\n", kt_local(local)));
            }
            Stmt::Narrow { local, ty, from, from_ty, because, .. } => {
                let src = self.place(from, indent);
                // The arm the justification names tells same-typed arms
                // apart (`Ok Int | Thrown Int`).
                let test = match because {
                    salvo_ir::Justification::Test { test } => self.test_arms.get(&(test.0, usize::MAX)).cloned(),
                    salvo_ir::Justification::Arm { switch, arm } => self.test_arms.get(&(switch.0, *arm)).cloned(),
                    _ => None,
                };
                let code = match test {
                    Some(ArmTest::Arm(i)) if from_ty.strip_quals().value_arms().len() >= 2 => {
                        let n = from_ty.strip_quals().value_arms().len();
                        let stars = vec!["*"; n].join(", ");
                        let t = self.ty(ty);
                        format!("(({src} as Union{n}.U{}<{stars}>).value as {t})", i + 1)
                    }
                    _ => self.narrow_code(&src, from_ty, ty),
                };
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
            ExprKind::Read { place, .. } => {
                let code = self.place(place, indent);
                // [kt-field-canbe-mut] a `canbe Mut` field is declared at the
                // plain type; a read the checker typed `Mut` casts back.
                if matches!(place.steps.last(), Some(Step::Field(_))) {
                    if let Ty::Qualified { quals, base } = &e.ty {
                        if quals.iter().any(|q| q.name == "Mut") {
                            let mut_ty = self.ty(&e.ty);
                            let plain_ty = self.ty(base);
                            if mut_ty != plain_ty {
                                return format!("({code} as {mut_ty})");
                            }
                        }
                    }
                }
                code
            }
            ExprKind::Call { target, type_args, args } => self.call(e, target, type_args, args, indent),
            // [actor-dispatch] A member of a handler instance, called directly.
            ExprKind::HandlerCall { instance, member, args } => {
                let inst = self.expr(instance, indent);
                let name = match member {
                    salvo_ir::HandlerMember::Face(m) => self.member_name(m),
                    salvo_ir::HandlerMember::Own(n) => kt_ident(n),
                };
                let a = self.exprs(args, indent);
                format!("{inst}.{name}({})", a.join(", "))
            }
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
                    Op::Eq => format!("({} == {})", a[0], a[1]),
                    Op::NotEq => format!("({} != {})", a[0], a[1]),
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
            ExprKind::Spread { value } => {
                // [kt-variadic] Kotlin's spread of an array into a vararg.
                let v = self.expr(value, indent);
                format!("*({v})")
            }
            ExprKind::Present { value } => self.expr(value, indent),
            // [interp-union] `UnionN.toStr`, given the function of each arm.
            ExprKind::UnionToStr { value, arms } => {
                let v = self.expr(value, indent);
                let fs: Vec<String> = arms.iter().map(|a| self.expr(a, indent)).collect();
                format!("({v}).toStr({})", fs.join(", "))
            }
            ExprKind::Widen { value } => {
                // [kt-op-promote] the explicit conversion.
                let v = self.expr(value, indent);
                match e.ty.strip_quals() {
                    Ty::Named { name, .. } if name == "Long" => format!("({v}).toLong()"),
                    Ty::Named { name, .. } if name == "Double" => format!("({v}).toDouble()"),
                    _ => v,
                }
            }
            ExprKind::DropMut { value } => {
                let v = self.expr(value, indent);
                match value.ty.strip_quals() {
                    Ty::Named { name, .. } => match crate::intrinsics::drop_mut_suffix(name) {
                        Some(suffix) => format!("{v}{suffix}"),
                        None => v,
                    },
                    // [proj-opt-slot] An optional drops `Mut` on its present arm.
                    Ty::Union(arms) if arms.len() == 2 && arms.iter().any(|a| a.is_none_ty()) => {
                        let arm = arms.iter().find(|a| !a.is_none_ty()).map(|a| a.strip_quals());
                        match arm {
                            Some(Ty::Named { name, .. }) => match crate::intrinsics::drop_mut_suffix(name) {
                                Some(suffix) => format!("({v})?{suffix}"),
                                None => v,
                            },
                            _ => v,
                        }
                    }
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
                let elem_ty = match e.ty.strip_quals() {
                    Ty::Array(el) => (**el).clone(),
                    _ => Ty::Unknown,
                };
                let elem = if elem_ty.is_unknown() { "Any".to_string() } else { self.ty(&elem_ty) };
                // [kt-variadic] `arrayOf` wants a reified element: over a bare
                // type variable the elements go into an `Array<Any?>`, which is
                // what an erased `Array<T>` is at run time.
                if matches!(elem_ty.strip_quals(), Ty::Var(_)) {
                    return format!("@Suppress(\"UNCHECKED_CAST\") (arrayOf<Any?>({}) as Array<{elem}>)", a.join(", "));
                }
                format!("arrayOf<{elem}>({})", a.join(", "))
            }
            ExprKind::Concat(parts) => {
                // [interp-to-str] a Kotlin string template: literal text as
                // is, every other part as `${…}`.
                let mut out = String::from("\"");
                for p in parts {
                    match &p.kind {
                        ExprKind::Str(text) => out.push_str(&escape_string(text)),
                        _ => {
                            // A scalar's `to_str` is the template's own
                            // conversion [interp-float]: the scalar alone.
                            let scalar = self.scalar_to_str_arg(p);
                            let code = match scalar {
                                Some(arg) => self.expr(arg, indent),
                                None => self.expr(p, indent),
                            };
                            out.push_str(&format!("${{{code}}}"));
                        }
                    }
                }
                out.push('"');
                out
            }
            ExprKind::Branch { arms, otherwise, .. } => self.branch(e, arms, otherwise.as_ref(), indent),
            ExprKind::Switch { subject, arms, .. } => self.switch(e, subject, arms, indent),
            ExprKind::Test { id, subject, test } => {
                self.test_arms.insert((id.0, usize::MAX), test.clone());
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
            ExprKind::Assert { cond, message, at } => {
                // [assert-trap] [kt-assert-trap] Salvo's failure, naming the
                // Salvo location; the message is composed only on failure.
                let c = self.expr(cond, indent);
                let m = match message {
                    Some(m) => {
                        let text = self.expr(m, indent);
                        format!("(\"salvo: \" + ({text}) + \" at {at}\")")
                    }
                    None => format!("\"salvo: assertion failed at {at}\""),
                };
                format!("(if (!({c})) throw AssertionError({m}) else Unit)")
            }
            ExprKind::Unreachable { message, at } => {
                let m = match message {
                    Some(m) => {
                        let text = self.expr(m, indent);
                        format!("(\"salvo: \" + ({text}) + \" at {at}\")")
                    }
                    None => format!("\"salvo: unreachable at {at}\""),
                };
                format!("throw AssertionError({m})")
            }
            ExprKind::Spawn { handler, pool, join, .. } => self.spawn(e, handler, pool.as_deref(), join.as_deref(), indent),
            ExprKind::Send { addr, member, args } => self.send(addr, member, args, indent),
            ExprKind::ReplyTo { target, captures, gated, pool } => self.replyto(target, captures, *gated, pool.as_deref(), indent),
            ExprKind::WaitFor { local, token_ty, body } => self.waitfor(local, token_ty, body, indent),
            ExprKind::SelfAddr => "__addr!!".to_string(),
            ExprKind::Handle { instance } => {
                // [effect-handle] [kt-monitor] the effect's monitor over the
                // instance; handles of one instance in one fn share a lock
                // [effect-handler-multi].
                let inst = self.expr(instance, indent);
                let face = self.ty(&e.ty);
                let mon = self.monitor_for(&face);
                let root = match &instance.kind {
                    ExprKind::Read { place, .. } if place.steps.is_empty() => Some(place.root.clone()),
                    _ => None,
                };
                match root.and_then(|r| self.handle_locks.get(&r).cloned()) {
                    Some(lock) => format!("{mon}({inst}, {lock})"),
                    None => format!("{mon}({inst})"),
                }
            }
            ExprKind::AddrInstance { addr } => {
                // [actor-use-addr] the send stub of the effect.
                let a = self.expr(addr, indent);
                let Ty::Named { name, .. } = e.ty.strip_quals() else {
                    self.error("an addr instance of a non-effect type");
                    return a;
                };
                let plain = salvo_core::typekey::plain(name).to_string();
                match self.interface_by_name(&plain) {
                    Some(iface) if iface.actor => format!("{}__Stub_{plain}({a})", self.pkg_prefix(&iface.id.module)),
                    // [monitor-handler] a plain effect's addr is its handle already.
                    Some(_) => a,
                    None => {
                        self.error(format!("`{plain}` is not an effect, so an addr is not an instance of it"));
                        a
                    }
                }
            }
            ExprKind::SelfSend { member, args } => self.self_send(member, args, indent),
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
        // The slot may be optional; the construction is of the value type.
        match ty.strip_quals().without_none().strip_quals() {
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
                // [effect-generic-decl] an erased declaration takes no arguments.
                let a: Vec<String> = if self.s.erased.is_erased(salvo_core::typekey::plain(name)) { Vec::new() } else { args.iter().filter(|a| !matches!(a, Ty::FnName(_))).map(|x| self.ty(x)).collect() };
                if a.is_empty() { base } else { format!("{base}<{}>", a.join(", ")) }
            }
            other => self.ty(other),
        }
    }

    /// The Kotlin name of an impl's member: the face member's emitted name
    /// when a face declares it [effect-member-overload], its own otherwise.
    pub(crate) fn impl_member_name(&self, h: &ImplDecl, member: &str) -> String {
        let decl = h.members.iter().find(|m| m.name == member);
        self.impl_member_name_of(h, member, decl)
    }

    /// The same for a known member declaration, which tells overloads apart.
    pub(crate) fn impl_member_name_of(&self, h: &ImplDecl, member: &str, decl: Option<&FnDecl>) -> String {
        let declared: Option<Vec<&Ty>> = decl.map(|m| m.params.iter().skip(m.effect_params).take(m.params.len() - m.effect_params - m.implicit_params).map(|p| &p.ty).collect());
        for f in &h.faces {
            if let Ty::Named { name, .. } = f.strip_quals() {
                if let Some(i) = self.interface_by_name(&salvo_core::typekey::plain(name)) {
                    let same: Vec<&salvo_ir::Member> = i.members.iter().filter(|m| m.name == member).collect();
                    // [effect-member-overload] several members of the name:
                    // the one whose parameters match.
                    let pick = match (same.as_slice(), &declared) {
                        ([one], _) => Some(*one),
                        (many, Some(d)) => many.iter().copied().find(|m| {
                            let ps: Vec<&Ty> = m.params.iter().map(|p| &p.ty).collect();
                            ps.len() >= d.len() && d.iter().zip(&ps).all(|(a, b)| erase_eq(a, b))
                        }),
                        _ => None,
                    };
                    if let Some(m) = pick {
                        return kt_ident(&m.emitted_name);
                    }
                }
            }
        }
        kt_ident(member)
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
            // [proj-opt-slot] An optional copies its present arm.
            if let Ty::Union(arms) = &args[0].ty {
                if arms.len() == 2 && arms.iter().any(|a| a.is_none_ty()) {
                    let arm = arms.iter().find(|a| !a.is_none_ty()).cloned().unwrap_or(Ty::Unknown);
                    return match salvo_core::copyplan::copy_plan(self.s.symbols, &arm) {
                        Some(salvo_core::copyplan::CopyPlan::Identity) => v,
                        Some(plan) => {
                            let inner = self.render_copy(&plan, "__c");
                            format!("({v})?.let {{ __c -> {inner} }}")
                        }
                        None => {
                            self.error(format!("cannot `copy` a value of type `{}`", args[0].ty));
                            v
                        }
                    };
                }
            }
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
        // [kt-actor] a reply's answer travels encoded.
        if name == "send" && recv == Some("Reply") && args.len() == 2 {
            if let Ty::Named { name: r, args: targs } = args[0].ty.strip_quals() {
                if r == "Reply" && targs.len() == 1 && salvo_core::wire_blocker(self.s.symbols, targs[0].strip_quals()).is_none() {
                    let codec = self.codec(&targs[0]);
                    let a0 = self.expr(&args[0], indent);
                    let a1 = self.expr(&args[1], indent);
                    return format!("salvo.SalvoSched.replyWire({a0}, {a1}, {codec})");
                }
            }
        }
        if matches!(name, "encode" | "decode") && args.len() == 1 {
            // The target type: written, or what the call answers (`decode`
            // answers `T?`), or what is encoded.
            let target = if name == "encode" {
                type_args.first().cloned().unwrap_or_else(|| args[0].ty.clone())
            } else {
                type_args.first().cloned().unwrap_or_else(|| e.ty.strip_quals().without_none())
            };
            let codec = self.codec(&target);
            let v = self.expr(&args[0], indent);
            return if name == "encode" { format!("salvo.salvoEncode({v}, {codec})") } else { format!("salvo.salvoDecode({v}, {codec})") };
        }
        if name == "cmp" && recv == Some("Str") {
            self.s.needs_compare = true;
        }
        // [addr-routable] [kt-wire] The routing intrinsics of std `net`.
        if name == "watch_control" && args.len() == 2 {
            // [node-group] [actor-group] [actor-private-send] The control
            // frames of the channel arrive as the handler's private
            // `control(from, data)` message.
            self.s.needs_bytes = true;
            let channel = self.expr(&args[0], indent);
            let sink = self.expr(&args[1], indent);
            let Some(hname) = self.current_impl.clone() else {
                self.error("`watch_control` is called from a handler's `init`");
                return "TODO()".to_string();
            };
            let control = self.s.ir.modules.iter().flat_map(|m| &m.decls).find_map(|d| match d {
                Decl::Impl(h) if h.name == hname => h.members.iter().find(|m| m.name == "control"),
                _ => None,
            });
            let Some(control) = control else {
                self.error(format!("`watch_control`: handler `{hname}` has no private `control(from, data)` member"));
                return "TODO()".to_string();
            };
            let from_ty = control.params.get(control.effect_params).map(|p| p.ty.clone()).unwrap_or(Ty::Unknown);
            let nid = self.ty(&from_ty);
            return format!("salvo.SalvoSched.watchControl({channel}, {sink}) {{ __n, __d -> __Priv_{hname}.Control({nid}(__n), salvo.SalvoBytes(__d)) }}");
        }
        if name == "protocol" && args.is_empty() {
            // [protocol-hash] [actor-group] the literal of the effect the
            // result type names — the written type argument or the implicit
            // position's type, both of which the call's type carries.
            let effect = match e.ty.strip_quals() {
                Ty::Named { name, args } if name == "Protocol" && args.len() == 1 => match args[0].strip_quals() {
                    Ty::Named { name, .. } => Some(salvo_core::typekey::plain(name).to_string()),
                    _ => None,
                },
                _ => None,
            };
            let Some(effect) = effect else {
                self.error("`protocol` needs an effect as its type argument");
                return "TODO()".to_string();
            };
            let Some(iface) = self.interface_by_name(&effect).filter(|i| i.protocol_hash.is_some()) else {
                self.error(format!("`protocol<{effect}>`: `{effect}` has no wire form, so it cannot be a group's protocol"));
                return "TODO()".to_string();
            };
            let st = self.ty(&e.ty);
            let st = st.split('<').next().unwrap_or(&st).to_string();
            return format!("{st}(\"{effect}\", {})", self.proto_const(iface));
        }
        if name == "key_hash" && args.len() == 1 {
            let codec = self.codec(&args[0].ty);
            let k = self.expr(&args[0], indent);
            return format!("salvo.SalvoSched.keyHash(salvo.salvoEncode({k}, {codec}).toByteArray())");
        }
        let variadic = f.params.iter().any(|p| p.variadic);
        let mut a: Vec<String> = Vec::new();
        for x in args {
            // A variadic tail arrives as one array; the lowering wants it
            // spread — an array literal's elements as they are.
            match &x.kind {
                ExprKind::Array(es) if variadic && matches!(x.ty.strip_quals(), Ty::Array(_)) => {
                    for el in es {
                        a.push(self.expr(el, indent));
                    }
                }
                _ => {
                    let code = self.expr(x, indent);
                    if variadic && matches!(x.ty.strip_quals(), Ty::Array(_)) {
                        a.push(format!("*({code})"));
                    } else {
                        a.push(code);
                    }
                }
            }
        }
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

    /// The argument of a `to_str` call on a scalar (whose Kotlin text is
    /// `toString()`'s), for an interpolation part.
    fn scalar_to_str_arg<'e>(&self, e: &'e Expr) -> Option<&'e Expr> {
        let ExprKind::Call { target: FnRef::Decl(id), args, .. } = &e.kind else { return None };
        let [arg] = args.as_slice() else { return None };
        let file_idx = self.s.program.files.iter().position(|f| f.module == id.module)?;
        let is_to_str = matches!(self.s.program.modules[file_idx].items.get(id.item), Some(salvo_syntax::ast::Item::Fn(f)) if f.intrinsic && f.name.name == "to_str");
        let scalar = matches!(arg.ty.strip_quals(), Ty::Named { name, .. } if matches!(name.as_str(), "Int" | "Long" | "Byte" | "Char" | "Bool" | "Double" | "Float" | "Str"));
        (is_to_str && scalar).then_some(arg)
    }

    /// Whether `args` is `cmp(a, b)` on a primitive, against `0`: the shape
    /// a comparison operator lowered to.
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
        if from_arms == to_arms {
            // The same wrapper: only the `None` arm may go.
            return if from.strip_quals().has_none_arm() && !to.strip_quals().has_none_arm() { format!("{code}!!") } else { code.to_string() };
        }
        let n = from_arms.len();
        let mut arms = Vec::new();
        for (i, a) in from_arms.iter().enumerate() {
            // The same arm, tag and all; else the arm of the same runtime
            // type (a lift drops the tag: `Ok Str` lands in `Str`).
            let found = to_arms.iter().position(|b| b == a).or_else(|| {
                let same: Vec<usize> = to_arms.iter().enumerate().filter(|(_, b)| erase_eq(b, a)).map(|(j, _)| j).collect();
                (same.len() == 1).then(|| same[0])
            });
            if let Some(j) = found {
                if let Some((ctor, _)) = self.union_arm(to, j) {
                    let at = self.ty(a);
                    let stars = vec!["*"; n].join(", ");
                    arms.push(format!("is Union{n}.U{}<{stars}> -> {ctor}(it.value as {at})", i + 1));
                }
            }
        }
        if from.strip_quals().has_none_arm() && to.strip_quals().has_none_arm() {
            arms.push("null -> null".to_string());
        }
        arms.push("else -> throw IllegalStateException(\"salvo: unreachable union arm\")".to_string());
        format!("{code}.let {{ when (it) {{ {} }} }}", arms.join("; "))
    }

    fn test_code(&mut self, subject: &str, subj_ty: &Ty, test: &ArmTest) -> String {
        // [actor-dispatch] A generated enum: its variant.
        if let (Ty::Named { name, .. }, ArmTest::Arm(i)) = (subj_ty.strip_quals(), test) {
            if let Some((e, _)) = self.generated_enum(name) {
                let path = self.type_path(name);
                return format!("({subject} is {path}.{})", e.variants[*i].name);
            }
        }
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
            ArmTest::Lit(arms) => {
                // [type-literal] the arm, then the value on it.
                let wrapped = n >= 2;
                let nullable = subj_ty.strip_quals().has_none_arm();
                let stars = vec!["*"; n.max(1)].join(", ");
                let lit = |l: &Lit| match l {
                    Lit::Str(s) => format!("\"{}\"", escape_string(s)),
                    Lit::Int(i) => i.to_string(),
                    Lit::Long(i) => format!("{i}L"),
                    Lit::Bool(b) => b.to_string(),
                };
                let mut parts: Vec<String> = Vec::new();
                for a in arms {
                    let (is_arm, value) = if wrapped {
                        (format!("{subject} is Union{n}.U{}<{stars}>", a.arm + 1), format!("({subject} as Union{n}.U{}<{stars}>).value", a.arm + 1))
                    } else if nullable {
                        (format!("{subject} != null"), subject.to_string())
                    } else {
                        ("true".to_string(), subject.to_string())
                    };
                    if a.lits.is_empty() {
                        parts.push(is_arm);
                        continue;
                    }
                    let op = if a.negate { "!=" } else { "==" };
                    let cmp: Vec<String> = a.lits.iter().map(|l| format!("{value} {op} {}", lit(l))).collect();
                    let joined = cmp.join(if a.negate { " && " } else { " || " });
                    parts.push(if is_arm == "true" { format!("({joined})") } else { format!("({is_arm} && ({joined}))") });
                }
                match parts.len() {
                    0 => "false".to_string(),
                    1 => parts.pop().unwrap(),
                    _ => format!("({})", parts.join(" || ")),
                }
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
        for (i, arm) in arms.iter().enumerate() {
            self.test_arms.insert((e_id(e), i), arm.test.clone());
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
        if !arms.iter().any(|a| matches!(a.test, ArmTest::Else)) {
            // The checker proved the arms total; Kotlin wants an `else` anyway
            // wherever the `when` is read as a value.
            out.push_str(&format!("{}else -> throw IllegalStateException(\"salvo: unreachable arm\")\n", "    ".repeat(indent + 1)));
        }
        let _ = value;
        out.push_str(&pad);
        out.push('}');
        out
    }
}

/// Two types equal up to qualifiers (the IR keeps only `Mut`).
fn erase_eq(a: &Ty, b: &Ty) -> bool {
    a.strip_quals() == b.strip_quals()
}

/// The node id of a `Switch`/`Branch` expression.
fn e_id(e: &Expr) -> u32 {
    match &e.kind {
        ExprKind::Switch { id, .. } | ExprKind::Branch { id, .. } | ExprKind::Test { id, .. } => id.0,
        _ => 0,
    }
}

/// Every local a block assigns (as a whole or through a projection),
/// nested blocks and lambdas included.
fn assigned_locals(b: &Block) -> HashSet<Local> {
    fn block(b: &Block, out: &mut HashSet<Local>) {
        for s in &b.stmts {
            match s {
                Stmt::Assign { place, value } => {
                    out.insert(place.root.clone());
                    expr(value, out);
                }
                Stmt::Let { value, .. } | Stmt::Expr(value) | Stmt::Return(Some(value)) => expr(value, out),
                Stmt::Loop { body, .. } => block(body, out),
                Stmt::ForEach { body, .. } => block(body, out),
                _ => {}
            }
        }
        if let Some(v) = &b.value {
            expr(v, out);
        }
    }
    fn expr(e: &Expr, out: &mut HashSet<Local>) {
        match &e.kind {
            ExprKind::Branch { arms, otherwise, .. } => {
                for (c, b) in arms {
                    expr(c, out);
                    block(b, out);
                }
                if let Some(o) = otherwise {
                    block(o, out);
                }
            }
            ExprKind::Switch { subject, arms, .. } => {
                expr(subject, out);
                for a in arms {
                    block(&a.body, out);
                }
            }
            ExprKind::Lambda { body, .. } | ExprKind::Try { body } | ExprKind::WaitFor { body, .. } => block(body, out),
            ExprKind::Call { args, .. } | ExprKind::Op { args, .. } | ExprKind::Tuple(args) | ExprKind::List(args) | ExprKind::Array(args) | ExprKind::Concat(args) => {
                for a in args {
                    expr(a, out);
                }
            }
            ExprKind::MemberCall { instance, args, .. } | ExprKind::HandlerCall { instance, args, .. } => {
                expr(instance, out);
                for a in args {
                    expr(a, out);
                }
            }
            ExprKind::UnionToStr { value, arms } => {
                expr(value, out);
                for a in arms {
                    expr(a, out);
                }
            }
            ExprKind::Construct { fields } => {
                for (_, v) in fields {
                    expr(v, out);
                }
            }
            ExprKind::MakeUnion { value, .. } | ExprKind::Rewrap { value, .. } | ExprKind::DropMut { value } | ExprKind::Widen { value } | ExprKind::Present { value } | ExprKind::Spread { value } => expr(value, out),
            _ => {}
        }
    }
    let mut out = HashSet::new();
    block(b, &mut out);
    out
}
