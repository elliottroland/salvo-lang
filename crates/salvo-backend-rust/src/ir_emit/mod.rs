//! [rs-ir] The Rust emitter: it reads the IR (IR record) and nothing of the AST
//! but the intrinsic tables keyed off a declaration.
//!
//! What it decides is representation and ownership idiom: `UnionN` enums and
//! `Option` for unions, references for `proj` values and lent parameters,
//! clones where a non-consuming read needs its own value, the handle struct
//! of an effect, and fully qualified paths (`crate::core_list::get`), so it
//! writes no `use` items.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use salvo_core::types::Ty;
use salvo_core::{ModulePath, Program};
use salvo_ir::{Decl, DeclId, FnDecl, ImplDecl, InterfaceDecl, Module as IrModule, Program as IrProgram, StructDecl};

use crate::emit::{
    generate_scheduler_file, generate_seq_file, generate_strings_file, generate_time_file, generate_unions_file,
    host_mod_name, module_mod_names, relative_path, rs_ident, CRATE_ATTRS,
};
use crate::EmittedFile;

mod actors;
mod body;
mod decls;
mod paths;
mod skeleton;



pub fn emit_program_ir(
    program: &Program,
    entry: Option<&ModulePath>,
    abi: bool,
) -> Result<(Vec<EmittedFile>, Vec<String>), Vec<String>> {
    run(program, entry, abi, false)
}

/// [platform-tree] The host implementation skeletons of the project's own
/// modules (a dependency's host files are the dependency's to ship).
pub fn platform_skeletons_ir(program: &Program, entry: Option<&ModulePath>) -> Result<Vec<EmittedFile>, Vec<String>> {
    run(program, entry, true, true).map(|(files, _)| files)
}

fn run(
    program: &Program,
    entry: Option<&ModulePath>,
    abi: bool,
    skeletons: bool,
) -> Result<(Vec<EmittedFile>, Vec<String>), Vec<String>> {
    let (_erased_program, erased, mut checked, warnings) = salvo_backend::driver::check_for_emission(program)?;
    let (symbols, resolution) = salvo_backend::driver::resolve_for_emission(program, &mut checked);
    let reach = salvo_backend::driver::reach(program, &resolution, &symbols, &checked, abi);
    let wanted: HashSet<&ModulePath> = reach.emitted.iter().copied().collect();
    let (ir, build_errors) = salvo_ir::build_program(program, &symbols, &resolution, &checked, Some(&wanted));
    if !build_errors.is_empty() {
        return Err(build_errors);
    }
    PLAIN_EFFECTS.with(|p| *p.borrow_mut() = symbols.effects.iter().filter(|(_, e)| !e.is_actor).map(|(k, _)| k.to_string()).collect());
    let (mod_names, mut used) = module_mod_names(&reach.emitted);
    // [rs-crate] The crate root: the module declaring `fn main` with a body,
    // the driver's choice winning.
    let declares_main = |m: &IrModule| m.decls.iter().any(|d| matches!(d, Decl::Fn(f) if f.name == "main" && f.body.is_some()));
    let root_module: Option<ModulePath> = ir
        .modules
        .iter()
        .find(|m| declares_main(m) && entry.is_some_and(|e| *e == m.path))
        .or_else(|| ir.modules.iter().find(|m| declares_main(m)))
        .map(|m| m.path.clone());
    let mut prefixes: HashMap<ModulePath, String> = HashMap::new();
    for (m, name) in &mod_names {
        let p = if Some(m) == root_module.as_ref() { "crate::".to_string() } else { format!("crate::{}::", rs_ident(name)) };
        prefixes.insert(m.clone(), p);
    }
    let fn_names = salvo_core::naming::FnNames::compute(program);
    let mut shared = Shared {
        program,
        symbols: &symbols,
        erased: &erased,
        abi,
        ir: &ir,
        fn_names: &fn_names,
        prefixes,
        union_sizes: BTreeSet::new(),
        needs_str: false,
        needs_seq: false,
        errors: fn_names.errors.clone(),
        decls: HashMap::new(),
        platform_effects: reach.platform_effects.clone(),
        platform_hosts: BTreeSet::new(),
        borrowing: HashSet::new(),
        locs: HashSet::new(),
        statics: HashMap::new(),
        paths: HashMap::new(),
        path_enums: Vec::new(),
        path_pairs: Vec::new(),
        needs_walk: false,
    };
    for m in &ir.modules {
        for d in &m.decls {
            shared.decls.insert(d.id().clone(), d);
            if let Decl::Static(st) = d {
                shared.statics.entry(m.path.clone()).or_default().insert(st.local.0.clone());
            }
        }
    }
    // [rs-proj-struct] A struct holding a `proj` field (or a struct that
    // does) carries its source's lifetime.
    loop {
        let mut grew = false;
        for m in &ir.modules {
            for d in &m.decls {
                if let Decl::Struct(s) = d {
                    let key = shared.key_of(&s.id, &s.name);
                    if shared.borrowing.contains(&key) {
                        continue;
                    }
                    if s.fields.iter().any(|f| shared.holds_proj(&f.ty)) {
                        shared.borrowing.insert(key);
                        grew = true;
                    }
                }
            }
        }
        if !grew {
            break;
        }
    }
    shared.compute_locs();
    shared.compute_paths();
    if skeletons {
        let mut files = Vec::new();
        for m in &ir.modules {
            let Some(file) = program.files.iter().find(|f| f.module == m.path) else { continue };
            if file.is_std || file.dependency.is_some() {
                continue;
            }
            let own_path = shared.prefix(&m.path).trim_end_matches("::").to_string();
            let mut em = ModuleEmitter::new(&mut shared, m);
            let Some((body, paths)) = em.skeleton(&own_path) else { continue };
            // [rs-platform-host] The host file is a module of the same crate:
            // the names its signatures mention are in scope through these.
            let mut uses: BTreeSet<String> = BTreeSet::new();
            uses.insert(format!("use {own_path}::*;"));
            for path in &paths {
                uses.insert(format!("use {path}::*;"));
            }
            if !shared.union_sizes.is_empty() {
                uses.insert("use crate::unions::*;".to_string());
            }
            let preamble = format!("\n{}\n", uses.into_iter().collect::<Vec<_>>().join("\n"));
            let module = &m.path;
            files.push(EmittedFile {
                rel_path: salvo_core::host_rel_path(module, "rs"),
                content: format!(
                    "// Host implementation of the platform declarations of Salvo module `{module}`.\n//\n\
                     // Generated once by `salvo platform generate`; the compiler never writes\n\
                     // this file again — it is yours. Nothing here is checked by Salvo: rustc\n\
                     // checks it, against the traits the backend generates from the\n\
                     // `platform handler` declarations.\n{preamble}{body}"
                ),
            });
        }
        let errors = std::mem::take(&mut shared.errors);
        return if errors.is_empty() { Ok((files, Vec::new())) } else { Err(errors) };
    }
    let mut files = Vec::new();
    for m in &ir.modules {
        let keep: Option<&BTreeSet<String>> = if abi && !reach.abi_full.contains(&m.path) { reach.closure.as_ref() } else { None };
        let text = ModuleEmitter::new(&mut shared, m).module_with(keep);
        let mut rel_path = std::path::PathBuf::new();
        for part in &m.path.0 {
            rel_path.push(part);
        }
        rel_path.set_extension(if abi { "sv.rs" } else { "rs" });
        files.push(EmittedFile { rel_path, content: text });
    }
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
    let (needs_scheduler, needs_wire) = (features.scheduler, features.wire);
    let union_sizes = shared.union_sizes.clone();
    if !union_sizes.is_empty() {
        if abi {
            files.push(EmittedFile { rel_path: "unions.rs".into(), content: generate_unions_file(&union_sizes, needs_wire) });
        } else {
            let mut root = String::from("// Generated by the Salvo compiler: enums for union types, one file per arity.\n");
            for &n in &union_sizes {
                root.push_str(&format!("mod union{n};\npub use union{n}::*;\n"));
                files.push(EmittedFile {
                    rel_path: format!("unions/union{n}.rs").into(),
                    content: generate_unions_file(&BTreeSet::from([n]), needs_wire),
                });
            }
            files.push(EmittedFile { rel_path: "unions/mod.rs".into(), content: root });
        }
    }
    if needs_wire {
        files.push(EmittedFile { rel_path: "wire.rs".into(), content: include_str!("../../runtime/wire.rs").to_string() });
    }
    let needs_hoststreams = reach.emitted.iter().any(|m| m.0 == ["runtime", "streams"])
        && (!abi || reach.abi_full.iter().any(|m| m.0 == ["runtime", "streams"]));
    let needs_time = features.time();
    let mut errors = std::mem::take(&mut shared.errors);
    // A construct rendered twice (a retried borrow, a hoist) reports once.
    let mut seen_errors = HashSet::new();
    errors.retain(|e| seen_errors.insert(e.clone()));
    if needs_scheduler && !salvo_core::runtime_module(program).is_some_and(|m| reach.emitted.contains(m)) {
        errors.push("internal: this program uses actors but the runtime module (`std/runtime.sv`) was not reached [runtime-sched]".to_string());
    }
    let (needs_str, needs_seq) = (shared.needs_str, shared.needs_seq);
    if needs_str {
        files.push(EmittedFile { rel_path: "strings.rs".into(), content: generate_strings_file() });
    }
    if needs_seq {
        files.push(EmittedFile { rel_path: "seq.rs".into(), content: generate_seq_file() });
    }
    if needs_scheduler {
        files.push(EmittedFile { rel_path: "scheduler.rs".into(), content: generate_scheduler_file() });
    }
    if needs_hoststreams {
        files.push(EmittedFile { rel_path: "hoststreams.rs".into(), content: include_str!("../../runtime/hoststreams.rs").to_string() });
    }
    if needs_time {
        files.push(EmittedFile { rel_path: "hosttime.rs".into(), content: generate_time_file() });
    }
    // [backend-companion] Companions, copied verbatim and mounted.
    let mut companion_mods: Vec<(String, std::path::PathBuf)> = Vec::new();
    for comp in &program.companions {
        if abi
            && !(comp.platform
                && (reach.abi_full.contains(&comp.module)
                    || (reach.abi_modules.contains(&comp.module) && !salvo_backend::emit_util::is_project_module(program, &comp.module))))
        {
            continue;
        }
        if !abi && !reach.reachable.contains(&comp.module) {
            continue;
        }
        if !abi && files.iter().any(|f| f.rel_path == comp.rel_path) {
            errors.push(format!(
                "companion file `{}` collides with the generated file of module `{}`: a companion cannot replace a module Salvo emits",
                comp.rel_path.display(),
                comp.module
            ));
            continue;
        }
        let mut name = if comp.platform { host_mod_name(&comp.module) } else { comp.module.0.join("_") };
        while !used.insert(name.clone()) {
            name.push('_');
        }
        let rel_path = if abi {
            let mut p = std::path::PathBuf::from("salvo").join(&comp.rel_path);
            p.set_extension("sv.rs");
            p
        } else {
            comp.rel_path.clone()
        };
        companion_mods.push((name, rel_path.clone()));
        files.push(EmittedFile { rel_path, content: comp.content.clone() });
    }
    if abi {
        for unit in program.units() {
            let module = &unit.file.module;
            if !salvo_backend::emit_util::is_project_module(program, module) || salvo_core::platform_declarations(unit.ast).is_empty() {
                continue;
            }
            let mut name = host_mod_name(module);
            while !used.insert(name.clone()) {
                name.push('_');
            }
            let rel = salvo_core::host_rel_path(module, "rs");
            let rel = rel.strip_prefix(salvo_core::PLATFORM_DIR).unwrap_or(&rel).to_path_buf();
            companion_mods.push((name, rel));
        }
    }
    for module in &shared.platform_hosts {
        if abi || salvo_core::host_file(&program.companions, module).is_some() {
            continue;
        }
        let what: String = program.units().filter(|u| u.file.module == *module).map(|u| salvo_core::platform_declarations(u.ast)).collect::<Vec<_>>().join(", ");
        errors.push(salvo_core::missing_handler_host_error(&what, module, &salvo_core::host_rel_path(module, "rs")));
    }
    // Crate-root assembly [rs-crate].
    if !files.is_empty() {
        let root_rel: std::path::PathBuf = match &root_module {
            _ if abi => "lib.sv.rs".into(),
            Some(module) => {
                let mut p = std::path::PathBuf::new();
                for part in &module.0 {
                    p.push(part);
                }
                p.set_extension("rs");
                p
            }
            None => "lib.rs".into(),
        };
        let root_dir = root_rel.parent().map(|p| p.to_path_buf()).unwrap_or_default();
        let mut header = CRATE_ATTRS.to_string();
        let mut mounts: Vec<(String, std::path::PathBuf)> = Vec::new();
        let abi_root_body = if abi {
            root_module
                .as_ref()
                .and_then(|m| {
                    let mut p = std::path::PathBuf::new();
                    for part in &m.0 {
                        p.push(part);
                    }
                    p.set_extension("sv.rs");
                    let i = files.iter().position(|f| f.rel_path == p)?;
                    Some(files.remove(i).content)
                })
                .unwrap_or_default()
        } else {
            String::new()
        };
        let runtime = |name: &str| -> std::path::PathBuf {
            if abi {
                format!("salvo/{name}.sv.rs").into()
            } else {
                format!("{name}.rs").into()
            }
        };
        if !union_sizes.is_empty() {
            mounts.push(("unions".into(), if abi { runtime("unions") } else { "unions/mod.rs".into() }));
        }
        if needs_str {
            mounts.push(("strings".into(), runtime("strings")));
        }
        if needs_seq {
            mounts.push(("seq".into(), runtime("seq")));
        }
        if needs_scheduler {
            mounts.push(("scheduler".into(), runtime("scheduler")));
        }
        if needs_hoststreams {
            mounts.push(("hoststreams".into(), runtime("hoststreams")));
        }
        if needs_time {
            mounts.push(("hosttime".into(), runtime("hosttime")));
        }
        if needs_wire {
            mounts.push(("wire".into(), runtime("wire")));
        }
        for (module, name) in &mod_names {
            if Some(module) == root_module.as_ref() {
                continue;
            }
            let mut p = std::path::PathBuf::new();
            for part in &module.0 {
                p.push(part);
            }
            p.set_extension(if abi { "sv.rs" } else { "rs" });
            mounts.push((name.clone(), p));
        }
        mounts.extend(companion_mods);
        for (name, path) in mounts {
            let rel = relative_path(&root_dir, &path);
            header.push_str(&format!("#[path = \"{rel}\"]\npub mod {};\n", rs_ident(&name)));
        }
        header.push('\n');
        if abi {
            files.push(EmittedFile { rel_path: root_rel, content: format!("{header}{abi_root_body}") });
            for f in &mut files {
                let name = f.rel_path.to_string_lossy().to_string();
                if !name.ends_with(".sv.rs") {
                    let stem = f.rel_path.file_stem().unwrap_or_default().to_string_lossy().to_string();
                    f.rel_path = runtime(&stem);
                }
            }
            return if errors.is_empty() { Ok((files, warnings)) } else { Err(errors) };
        }
        match files.iter_mut().find(|f| f.rel_path == root_rel) {
            Some(root_file) => root_file.content = format!("{header}{}", root_file.content),
            None => files.push(EmittedFile { rel_path: root_rel, content: header }),
        }
    }
    if errors.is_empty() {
        salvo_backend::emit_util::no_duplicate_paths(&files)?;
        Ok((files, warnings))
    } else {
        Err(errors)
    }
}

/// Program-wide state the module emitters share.
pub(crate) struct Shared<'p> {
    pub program: &'p Program,
    pub symbols: &'p salvo_core::Symbols<'p>,
    pub erased: &'p salvo_core::Erased,
    pub abi: bool,
    pub ir: &'p IrProgram,
    pub fn_names: &'p salvo_core::naming::FnNames,
    /// Per module: the path prefix of its items (`crate::core_list::`).
    pub prefixes: HashMap<ModulePath, String>,
    pub union_sizes: BTreeSet<usize>,
    pub needs_str: bool,
    pub needs_seq: bool,
    pub errors: Vec<String>,
    pub decls: HashMap<DeclId, &'p Decl>,
    pub platform_effects: BTreeMap<String, salvo_core::PlatformEffect>,
    pub platform_hosts: BTreeSet<ModulePath>,
    /// [rs-proj-struct] struct keys carrying a lifetime.
    pub borrowing: HashSet<String>,
    /// [rs-loc] fns that get a locator variant (`f__loc`).
    pub locs: HashSet<DeclId>,
    /// [mod-use] Per module, its statics' locals.
    pub statics: HashMap<ModulePath, HashSet<String>>,
    /// [rs-path] Path types by (container, element).
    pub paths: HashMap<(String, String), Result<paths::PathTy, String>>,
    /// [rs-path] The generated path enums, declared with their home module.
    pub path_enums: Vec<paths::PathTy>,
    /// [rs-path] The concrete (container, element) pairs, for `Walk` impls.
    pub path_pairs: Vec<(Ty, Ty, paths::PathTy)>,
    /// [rs-path] Whether some fn is generic over a handle's container.
    pub needs_walk: bool,
}

impl<'p> Shared<'p> {
    /// The symbol key of a declaration: the key the checker's types use.
    pub fn key_of(&self, id: &DeclId, name: &str) -> String {
        for (k, m) in &self.symbols.key_modules {
            if **m == id.module && salvo_core::typekey::plain(k) == name {
                return k.to_string();
            }
        }
        name.to_string()
    }

    /// Whether a value of this type holds a borrow (`proj`/`ref` anywhere
    /// but under a fn type, or a borrowing struct).
    pub fn holds_proj(&self, t: &Ty) -> bool {
        match t {
            Ty::Qualified { quals, base } => (quals.iter().any(|q| salvo_core::types::is_proj_name(&q.name)) && !is_copy_ty(base)) || self.holds_proj(base),
            Ty::Named { name, args } => self.borrowing.contains(name) || args.iter().any(|a| self.holds_proj(a)),
            Ty::Union(arms) | Ty::Tuple(arms) => arms.iter().any(|a| self.holds_proj(a)),
            Ty::Array(e) => self.holds_proj(e),
            _ => false,
        }
    }

    pub fn interface(&self, key: &str) -> Option<&'p InterfaceDecl> {
        let all = || self.ir.modules.iter().flat_map(|m| &m.decls);
        all().find_map(|d| match d {
            Decl::Interface(i) if self.key_of(&i.id, &i.name) == key => Some(i),
            _ => None,
        })
        .or_else(|| {
            if self.symbols.effects.contains_key(key) || !self.symbols.key_modules.contains_key(key) {
                all().find_map(|d| match d {
                    Decl::Interface(i) if i.name == salvo_core::typekey::plain(key) => Some(i),
                    _ => None,
                })
            } else {
                None
            }
        })
    }

    /// [actor-msg] The generated enum a type key names, and its module.
    pub fn generated_enum(&self, key: &str) -> Option<(&'p salvo_ir::EnumDecl, ModulePath)> {
        let module = salvo_core::typekey::module_of(key)?;
        self.ir.modules.iter().filter(|m| m.path.0.join(".") == module).flat_map(|m| &m.decls).find_map(|d| match d {
            Decl::Enum(e) if salvo_ir::enum_key(&e.name, &e.id.module) == key => Some((e, e.id.module.clone())),
            _ => None,
        })
    }

    pub fn interface_by_id(&self, id: &DeclId) -> Option<&'p InterfaceDecl> {
        match self.decls.get(id) {
            Some(Decl::Interface(i)) => Some(i),
            _ => None,
        }
    }

    pub fn impl_decl(&self, key: &str) -> Option<&'p ImplDecl> {
        if !self.symbols.handlers.contains_key(key) {
            return None;
        }
        let all = || self.ir.modules.iter().flat_map(|m| &m.decls);
        all().find_map(|d| match d {
            Decl::Impl(h) if self.key_of(&h.id, &h.name) == key => Some(h),
            _ => None,
        })
        .or_else(|| {
            all().find_map(|d| match d {
                Decl::Impl(h) if h.name == salvo_core::typekey::plain(key) => Some(h),
                _ => None,
            })
        })
    }

    pub fn fn_decl(&self, id: &DeclId) -> Option<&'p FnDecl> {
        match self.decls.get(id) {
            Some(Decl::Fn(f)) => Some(f),
            _ => None,
        }
    }

    pub fn struct_decl(&self, key: &str) -> Option<&'p StructDecl> {
        self.ir.modules.iter().flat_map(|m| &m.decls).find_map(|d| match d {
            Decl::Struct(s) if self.key_of(&s.id, &s.name) == key || s.name == key => Some(s),
            _ => None,
        })
    }

    /// The AST of a fn reference (intrinsic lowering keys off it).
    pub fn ast_fn(&self, id: &DeclId) -> Option<&'p salvo_syntax::ast::FnDecl> {
        let fi = self.program.files.iter().position(|f| f.module == id.module)?;
        match self.program.modules[fi].items.get(id.item)? {
            salvo_syntax::ast::Item::Fn(f) if id.sub == 0 => Some(f),
            salvo_syntax::ast::Item::Qualifier(q) if id.sub > 0 => q.fns.get(id.sub as usize - 1),
            _ => None,
        }
    }

    /// A type with a top-level alias expanded (a fn-type alias decides how
    /// a parameter is passed).
    pub fn unalias(&self, t: &Ty) -> Ty {
        let (quals, base) = match t {
            Ty::Qualified { quals, base } => (quals.clone(), (**base).clone()),
            other => (Vec::new(), other.clone()),
        };
        if let Ty::Named { name, args } = &base {
            if let Some(alias) = self.symbols.type_aliases.get(name.as_str()) {
                if let Some(target) = alias.alias.as_ref() {
                    let subst: HashMap<String, Ty> = alias.generics.iter().map(|g| g.name.clone()).zip(args.iter().cloned()).collect();
                    if let Some(expanded) = salvo_core::approx_ty(target, &subst) {
                        return self.unalias(&salvo_ir::build::erase(&expanded)).qualify(quals);
                    }
                }
            }
        }
        t.clone()
    }

    pub fn unalias_params(&self, ps: &[salvo_ir::Param]) -> Vec<salvo_ir::Param> {
        ps.iter().map(|p| salvo_ir::Param { ty: self.unalias(&p.ty), ..p.clone() }).collect()
    }

    /// [rs-loc] Whether a fn type answers a mutable lend (a callback that
    /// lends mutably is passed as its locator).
    pub fn fn_ty_lends_mut(&self, t: &Ty) -> bool {
        match self.unalias(t).strip_quals() {
            Ty::Fn { ret, .. } => {
                let v = ret.strip_quals().without_none();
                (is_ref(ret) && is_mut(ret)) || (is_ref(&v) && is_mut(&v))
            }
            _ => false,
        }
    }

    /// [rs-path] Whether a fn mints a mutable handle: its result is a `ref`
    /// carrying `Mut` (or the optional of one).
    pub fn is_mint(&self, f: &FnDecl) -> bool {
        let v = f.ret.strip_quals().without_none();
        (is_ref(&f.ret) && is_mut(&f.ret)) || (is_ref(&v) && is_mut(&v))
    }

    /// [rs-loc] A call that lends mutably from a fn whose result is a read
    /// projection: the callee and the index of the parameter it lends from.
    pub fn mut_lend(&self, e: &salvo_ir::Expr) -> Option<(DeclId, usize)> {
        let salvo_ir::ExprKind::Call { target: salvo_ir::FnRef::Decl(id), .. } = &e.kind else { return None };
        let lends_mut = |t: &Ty| is_ref(t) && is_mut(t);
        let value = |t: &Ty| -> Ty { t.strip_quals().without_none() };
        if !lends_mut(&e.ty) && !lends_mut(&value(&e.ty)) {
            return None;
        }
        let f = self.fn_decl(id)?;
        if lends_mut(&f.ret) || lends_mut(&value(&f.ret)) {
            return None;
        }
        Some((id.clone(), self.lend_param(f)?))
    }

    /// [rs-proj-generic] The type variables whose `proj` is the variable
    /// itself: no parameter the result can borrow from holds one, so the
    /// borrow is the instantiation's (`filter` over a pass yields `T`s).
    pub fn plain_proj_vars(&self, tps: &[salvo_ir::TypeParam], params: &[salvo_ir::Param], borrows: &[usize]) -> HashSet<String> {
        let sources: Vec<&salvo_ir::Param> = if borrows.is_empty() {
            params.iter().filter(|p| p.mode != salvo_ir::PassMode::Moved && !matches!(self.unalias(&p.ty).strip_quals(), Ty::Fn { .. })).collect()
        } else {
            borrows.iter().filter_map(|i| params.get(*i)).collect()
        };
        tps.iter().map(|t| t.name.clone()).filter(|v| !sources.iter().any(|p| mentions_var(&p.ty, v))).collect()
    }

    /// [runtime-kept-fn] [platform-fn-value] A fn-typed parameter of a
    /// platform fn that the host keeps: `Some(boxed, once)` — boxed and
    /// `Send + 'static` in the runtime, a plain `fn` pointer elsewhere.
    pub fn kept_param(&self, f: &FnDecl, i: usize) -> Option<(bool, bool)> {
        if f.kind != salvo_ir::FnKind::Platform || i < f.effect_params {
            return None;
        }
        let af = self.ast_fn(&f.id)?;
        let ap = af.params.iter().filter(|p| !p.implicit).nth(i - f.effect_params)?;
        let once = matches!(&ap.ty, salvo_syntax::ast::Type::QualifiedGroup { qualifiers, base, .. } if qualifiers.iter().any(|q| q.name.name == "once") && matches!(base.as_ref(), salvo_syntax::ast::Type::Fn { .. }));
        if !(matches!(ap.ty, salvo_syntax::ast::Type::Fn { .. }) || once) || !salvo_core::check::platform_keeps_param(af, ap) {
            return None;
        }
        let file = self.program.files.iter().find(|x| x.module == f.id.module)?;
        let runtime = file.is_std && file.module.0.first().is_some_and(|m| m == salvo_core::STD_INTERNAL);
        if !runtime && !matches!(ap.ty, salvo_syntax::ast::Type::Fn { .. }) {
            return None;
        }
        Some((runtime, once))
    }

    /// [once-fn] Whether a fn's `i`th parameter is a `once` fn.
    pub fn once_param(&self, f: &FnDecl, i: usize) -> bool {
        if i < f.effect_params {
            return false;
        }
        let Some(af) = self.ast_fn(&f.id) else { return false };
        let Some(ap) = af.params.iter().filter(|p| !p.implicit).nth(i - f.effect_params) else { return false };
        matches!(&ap.ty, salvo_syntax::ast::Type::QualifiedGroup { qualifiers, base, .. } if qualifiers.iter().any(|q| q.name.name == "once") && matches!(base.as_ref(), salvo_syntax::ast::Type::Fn { .. }))
    }

    /// The parameter a fn's result borrows from.
    pub fn lend_param(&self, f: &FnDecl) -> Option<usize> {
        if let Some(i) = f.borrows.first() {
            return Some(*i);
        }
        let mut names = Vec::new();
        decls::proj_sources(&f.ret, &mut names);
        if let Some(i) = names.first().and_then(|n| f.params.iter().position(|p| &p.local.0 == n)) {
            return Some(i);
        }
        // No source named: the first borrowed parameter.
        f.params.iter().enumerate().skip(f.effect_params).find(|(_, p)| p.mode != salvo_ir::PassMode::Moved && !is_copy_ty(&p.ty)).map(|(i, _)| i)
    }

    /// [rs-loc] An effect member whose result lends mutably from a
    /// parameter, on an interface rendered as a plain trait: it has a
    /// locator face (`m__loc`) beside its natural one, for a handle that
    /// outlives a read of the container.
    pub fn member_lends_mut(&self, iface: &InterfaceDecl, m: &salvo_ir::Member) -> bool {
        if iface.actor || self.platform_effects.contains_key(&iface.name) || m.send {
            return false;
        }
        let v = m.ret.strip_quals().without_none();
        ((is_ref(&m.ret) && is_mut(&m.ret)) || (is_ref(&v) && is_mut(&v))) && self.member_lend_param(m).is_some()
    }

    /// The parameter (an index into the member's own) a lending member's
    /// result borrows from: the one `proj(…)` names, else the first borrowed.
    pub fn member_lend_param(&self, m: &salvo_ir::Member) -> Option<usize> {
        let mut names = Vec::new();
        decls::proj_sources(&m.ret, &mut names);
        if let Some(i) = names.first().and_then(|n| m.params.iter().position(|p| &p.local.0 == n)) {
            return Some(i);
        }
        m.params.iter().position(|p| p.mode != salvo_ir::PassMode::Moved && !is_copy_ty(&p.ty) && !matches!(self.unalias(&p.ty).strip_quals(), Ty::Fn { .. }))
    }

    /// [rs-loc] Every fn some mutable lend reaches, and every fn those
    /// forward their result from.
    fn compute_locs(&mut self) {
        let mut work: Vec<DeclId> = Vec::new();
        let mut bodies: Vec<&'p salvo_ir::Block> = Vec::new();
        for m in &self.ir.modules {
            for d in &m.decls {
                match d {
                    Decl::Fn(f) => bodies.extend(f.body.as_ref()),
                    Decl::Impl(h) => {
                        for f in h.members.iter().chain(h.init.as_ref()) {
                            bodies.extend(f.body.as_ref());
                        }
                    }
                    _ => {}
                }
            }
        }
        // Every declared mutable lender: a handle binds its position.
        for m in &self.ir.modules {
            for d in &m.decls {
                if let Decl::Fn(f) = d {
                    let v = f.ret.strip_quals().without_none();
                    if f.body.is_some() && ((is_ref(&f.ret) && is_mut(&f.ret)) || (is_ref(&v) && is_mut(&v))) {
                        work.push(f.id.clone());
                    }
                }
            }
        }
        for b in bodies {
            body::walk_block(b, &mut |_| {}, &mut |e| {
                if let Some((id, _)) = self.mut_lend(e) {
                    work.push(id);
                }
                if let salvo_ir::ExprKind::FnValue(salvo_ir::FnRef::Decl(id)) = &e.kind {
                    if self.fn_ty_lends_mut(&e.ty) {
                        work.push(id.clone());
                    }
                }
            });
        }
        // A lending member's body forwards its result like a lender's does.
        for m in &self.ir.modules {
            for d in &m.decls {
                let Decl::Impl(h) = d else { continue };
                for face in &h.faces {
                    let Ty::Named { name: fname, .. } = face.strip_quals() else { continue };
                    let Some(iface) = self.interface(fname) else { continue };
                    for im in iface.members.iter().filter(|im| self.member_lends_mut(iface, im)) {
                        for f in h.members.iter().filter(|f| f.name == im.name) {
                            if let Some(b) = &f.body {
                                self.forwarded_callees(b, &mut work);
                            }
                        }
                    }
                }
            }
        }
        while let Some(id) = work.pop() {
            if !self.locs.insert(id.clone()) {
                continue;
            }
            let Some(f) = self.fn_decl(&id) else { continue };
            let Some(b) = &f.body else { continue };
            self.forwarded_callees(b, &mut work);
        }
    }

    /// The fns whose result a body hands back as its own.
    fn forwarded_callees(&self, b: &'p salvo_ir::Block, work: &mut Vec<DeclId>) {
        let mut rets: Vec<&salvo_ir::Expr> = Vec::new();
        fn collect<'b>(b: &'b salvo_ir::Block, out: &mut Vec<&'b salvo_ir::Expr>) {
            for s in &b.stmts {
                match s {
                    salvo_ir::Stmt::Return(Some(e)) => out.push(e),
                    salvo_ir::Stmt::Loop { body, .. } | salvo_ir::Stmt::ForEach { body, .. } => collect(body, out),
                    salvo_ir::Stmt::Expr(salvo_ir::Expr { kind: salvo_ir::ExprKind::Branch { arms, otherwise, .. }, .. }) => {
                        for (_, a) in arms {
                            collect(a, out);
                        }
                        if let Some(o) = otherwise {
                            collect(o, out);
                        }
                    }
                    _ => {}
                }
            }
            if let Some(v) = &b.value {
                out.push(v);
            }
        }
        collect(b, &mut rets);
        for mut e in rets {
            while let salvo_ir::ExprKind::Present { value } = &e.kind {
                e = value;
            }
            if let salvo_ir::ExprKind::Call { target: salvo_ir::FnRef::Decl(g), .. } = &e.kind {
                if self.fn_decl(g).is_some_and(|g| g.kind != salvo_ir::FnKind::Intrinsic) {
                    work.push(g.clone());
                }
            }
        }
    }

    pub fn prefix(&self, m: &ModulePath) -> String {
        self.prefixes.get(m).cloned().unwrap_or_else(|| "crate::".to_string())
    }
}

/// One `use` per foreign item a module's text names, unambiguous and not
/// declared here.
fn host_imports(body: &str) -> String {
    let mut found: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let bytes = body.as_bytes();
    let mut i = 0;
    while let Some(at) = body[i..].find("crate::") {
        let start = i + at;
        i = start + 7;
        if start > 0 && (bytes[start - 1].is_ascii_alphanumeric() || bytes[start - 1] == b'_' || bytes[start - 1] == b':') {
            continue;
        }
        let rest = &body[i..];
        let end = rest.find(|c: char| !(c.is_alphanumeric() || c == '_' || c == ':')).unwrap_or(rest.len());
        let path = rest[..end].trim_end_matches(':');
        let parts: Vec<&str> = path.split("::").collect();
        if parts.len() != 2 {
            continue;
        }
        let (m, name) = (parts[0], parts[1]);
        if name.is_empty() || name.starts_with("__") || m.starts_with("__") || m.starts_with("platform_") || matches!(m, "unions" | "scheduler" | "wire" | "seq" | "strings" | "hosttime" | "hoststreams") {
            continue;
        }
        if !name.chars().next().is_some_and(|c| c.is_alphabetic()) {
            continue;
        }
        found.entry(name.to_string()).or_default().insert(m.to_string());
    }
    let declared = |n: &str| -> bool {
        ["fn", "struct", "trait", "enum", "type", "const"].iter().any(|k| body.contains(&format!("pub {k} {n}<")) || body.contains(&format!("pub {k} {n}(")) || body.contains(&format!("pub {k} {n} ")) || body.contains(&format!("pub {k} {n}:")))
            || body.contains(&format!("::{n};\n")) && body.contains(&format!("pub use crate::"))
    };
    let mut out = String::new();
    for (name, mods) in found {
        if mods.len() != 1 || declared(&name) {
            continue;
        }
        let m = mods.into_iter().next().unwrap();
        out.push_str(&format!("use crate::{m}::{name};\n"));
    }
    if !out.is_empty() {
        out.push('\n');
    }
    out
}

pub(crate) fn mentions_var(t: &Ty, v: &str) -> bool {
    match t {
        Ty::Var(x) => x == v,
        Ty::Named { name, args } => (args.is_empty() && name == v) || args.iter().any(|a| mentions_var(a, v)),
        Ty::Qualified { base, .. } => mentions_var(base, v),
        Ty::Union(a) | Ty::Tuple(a) => a.iter().any(|x| mentions_var(x, v)),
        Ty::Array(e) => mentions_var(e, v),
        Ty::Fn { params, ret, .. } => params.iter().any(|x| mentions_var(x, v)) || mentions_var(ret, v),
        _ => false,
    }
}

/// [rs-proj-generic] `proj T`/`ref T` over a type variable, as a
/// container's element, is `T`: the instantiation carries the borrow.
pub(crate) fn strip_nested_var_proj_in(t: &Ty, nested: bool, generics: &HashSet<String>) -> Ty {
    let strip_nested_var_proj = |t: &Ty, n: bool| strip_nested_var_proj_in(t, n, generics);
    match t {
        Ty::Qualified { quals, base } => {
            let b = strip_nested_var_proj(base, nested);
            let var = matches!(&b, Ty::Var(_)) || matches!(&b, Ty::Named { name, args } if args.is_empty() && generics.contains(name));
            let q: Vec<salvo_core::types::Qual> = quals.iter().filter(|q| !(nested && var && salvo_core::types::is_proj_name(&q.name))).cloned().collect();
            b.qualify(q)
        }
        Ty::Named { name, args } => Ty::Named { name: name.clone(), args: args.iter().map(|a| strip_nested_var_proj(a, true)).collect() },
        Ty::Union(a) => Ty::Union(a.iter().map(|x| strip_nested_var_proj(x, nested)).collect()),
        Ty::Tuple(a) => Ty::Tuple(a.iter().map(|x| strip_nested_var_proj(x, true)).collect()),
        Ty::Array(e) => Ty::Array(Box::new(strip_nested_var_proj(e, true))),
        other => other.clone(),
    }
}

/// `t` with `proj`/`ref` dropped over the variables in `vars`.
pub(crate) fn strip_plain_proj(t: &Ty, vars: &HashSet<String>) -> Ty {
    match t {
        Ty::Qualified { quals, base } => {
            let b = strip_plain_proj(base, vars);
            let plain = match &b {
                Ty::Var(v) => vars.contains(v),
                Ty::Named { name, args } => args.is_empty() && vars.contains(name),
                _ => false,
            };
            let q: Vec<salvo_core::types::Qual> = quals.iter().filter(|q| !(plain && salvo_core::types::is_proj_name(&q.name))).cloned().collect();
            b.qualify(q)
        }
        Ty::Named { name, args } => Ty::Named { name: name.clone(), args: args.iter().map(|a| strip_plain_proj(a, vars)).collect() },
        Ty::Union(a) => Ty::Union(a.iter().map(|x| strip_plain_proj(x, vars)).collect()),
        Ty::Tuple(a) => Ty::Tuple(a.iter().map(|x| strip_plain_proj(x, vars)).collect()),
        Ty::Array(e) => Ty::Array(Box::new(strip_plain_proj(e, vars))),
        other => other.clone(),
    }
}

thread_local! {
    /// [monitor-handler] The plain effects of the program being emitted: an
    /// `Addr` of one is its handle, not a scheduler index.
    static PLAIN_EFFECTS: std::cell::RefCell<HashSet<String>> = std::cell::RefCell::new(HashSet::new());
}

pub(crate) fn is_copy_ty(t: &Ty) -> bool {
    match t.strip_quals() {
        Ty::Named { name, args } if args.is_empty() => matches!(name.as_str(), "Int" | "Long" | "Float" | "Double" | "Bool" | "Char" | "Byte" | "Pool"),
        // [runtime-handles] an actor's addr is a scheduler index.
        Ty::Named { name, args } if name == "Addr" => !matches!(args.first().map(|a| a.strip_quals()), Some(Ty::Named { name: e, .. }) if PLAIN_EFFECTS.with(|p| p.borrow().contains(e))),
        _ => false,
    }
}

/// Whether `t` carries a projection qualifier at its top level — either
/// flavour, `proj` or `ref` [proj-type] [ref-handle]. Most call sites ask
/// only "is this a borrow of a source?", for which both answer yes; the
/// handle-specific sites use [`is_ref`] instead.
pub(crate) fn is_proj(t: &Ty) -> bool {
    t.quals().iter().any(|q| salvo_core::types::is_proj_name(&q.name)) && !is_copy_ty(t)
}

/// Whether `t` is a **`ref` handle** at its top level — the container-named
/// projection that may be a mutable element handle when it also carries
/// `Mut` [ref-handle]. A plain `proj` is a read-only view's borrow and
/// never a handle; only `ref` is.
pub(crate) fn is_ref(t: &Ty) -> bool {
    t.quals().iter().any(|q| q.name == "ref") && !is_copy_ty(t)
}

pub(crate) fn is_mut(t: &Ty) -> bool {
    t.quals().iter().any(|q| q.name == "Mut")
}

/// [ref-handle] Whether a borrow of `t` is a **mutable** borrow: `t`
/// carries `Mut` and is not a plain read-only `proj` (where the `Mut` only
/// rides along as part of the element type, rendered `&T`). A `ref` carrying
/// `Mut` is the mutable handle.
pub(crate) fn is_mut_borrow(t: &Ty) -> bool {
    is_mut(t) && (!is_proj(t) || is_ref(t))
}

/// One module's emitter.
pub(crate) struct ModuleEmitter<'a, 'p> {
    pub s: &'a mut Shared<'p>,
    pub module: &'a IrModule,
    pub out: String,
    pub f: body::FnState,
}

impl<'a, 'p> ModuleEmitter<'a, 'p> {
    fn new(s: &'a mut Shared<'p>, module: &'a IrModule) -> Self {
        ModuleEmitter { s, module, out: String::new(), f: body::FnState::default() }
    }

    pub fn error(&mut self, msg: impl Into<String>) {
        self.s.errors.push(format!("{}: {}", self.module.file_name, msg.into()));
    }

    fn module_with(mut self, keep: Option<&BTreeSet<String>>) -> String {
        let kept = |d: &Decl| -> bool {
            let Some(keep) = keep else { return true };
            match d {
                Decl::Struct(x) => keep.contains(&x.name),
                Decl::Union(x) => keep.contains(&x.name),
                Decl::Interface(x) => keep.contains(&x.name),
                Decl::Impl(h) => h.platform,
                Decl::Fn(_) | Decl::Static(_) | Decl::PlatformType(_) => false,
                // A message enum travels with its interface.
                Decl::Enum(e) => matches!(e.kind, salvo_ir::EnumKind::Message { .. }) && keep.contains(e.name.trim_start_matches("__Msg_")),
            }
        };
        let mut body = String::new();
        for d in &self.module.decls {
            if !kept(d) {
                continue;
            }
            self.out.clear();
            self.decl(d);
            if !self.out.is_empty() {
                body.push_str(&self.out);
            }
        }
        // [rs-imports] The generated code names everything in full; these
        // `use`s are for the host files, which `use crate::<module>::*` and
        // name what the module's declarations mention unqualified.
        // [rs-path] The path enums of the handles into this module's types.
        if keep.is_none() {
            body.push_str(&self.path_enums());
            body.push_str(&self.walk_impls());
        }
        format!("{}{body}", host_imports(&body))
    }

    // ------------------------------------------------------------- names --

    /// The Rust path of a fn declaration.
    pub fn fn_path(&mut self, id: &DeclId) -> String {
        let name = self.fn_rust_name(id);
        format!("{}{name}", self.s.prefix(&id.module))
    }

    pub fn fn_rust_name(&self, id: &DeclId) -> String {
        match self.s.ast_fn(id) {
            Some(f) if id.sub == 0 => rs_ident(self.s.fn_names.get(f).unwrap_or(&f.name.name)),
            Some(f) => {
                let fi = self.s.program.files.iter().position(|x| x.module == id.module).unwrap();
                let q = match &self.s.program.modules[fi].items[id.item] {
                    salvo_syntax::ast::Item::Qualifier(q) => match salvo_core::refine::of_base(&q.of, &q.generics) {
                        Some(subject) => format!("{}__{}", q.name.name.replace('.', ""), subject.replace('.', "")),
                        None => q.name.name.replace('.', ""),
                    },
                    _ => String::new(),
                };
                rs_ident(&format!("{q}_{}", f.name.name))
            }
            None => rs_ident(&self.s.ir.ref_name(id)),
        }
    }

    /// A type's Rust path by its symbol key.
    pub fn type_path(&self, key: &str) -> String {
        let plain = salvo_core::typekey::plain(key);
        match self.s.symbols.key_modules.get(key) {
            Some(m) => format!("{}{}", self.s.prefix(m), rs_ident(plain)),
            // [actor-msg] A generated enum is keyed by its module.
            None => match self.s.generated_enum(key) {
                Some((_, m)) => format!("{}{}", self.s.prefix(&m), rs_ident(plain)),
                None => rs_ident(plain),
            },
        }
    }
}
