//! [kt-ir] [platform-tree] [kt-platform-host] `salvo platform generate` over
//! the IR: the host implementation skeleton of every module of the project
//! that declares platform items — one class per platform type, one fn per
//! platform fn (the wrapper's own signature, under the real name), one class
//! per platform handler implementing the host-facing interface.

use std::collections::{BTreeSet, HashMap, HashSet};

use crate::EmittedFile;
use salvo_core::types::Ty;
use salvo_core::{ModulePath, Program};
use salvo_ir::{Decl, FnKind, Module as IrModule};

use super::{host_package, kt_local, ModuleEmitter, Shared};
use crate::emit::{kotlin_package, kt_ident};

pub fn platform_skeletons_ir(program: &Program) -> Result<Vec<EmittedFile>, Vec<String>> {
    // Errors only, never warnings [qual-refn-ambiguous]: the shared front.
    let (symbols, resolution, checked) = salvo_backend::driver::check_for_skeletons(program)?;
    let erased = salvo_core::erased_generics(program);
    // [manifest-deps] A dependency's host files are the dependency's to
    // ship, beside its own sources — not skeletons under this project.
    let wanted: HashSet<&ModulePath> = program
        .units()
        .filter(|u| !u.file.is_std && u.file.dependency.is_none() && !salvo_core::platform_declarations(u.ast).is_empty() || has_platform_type(u.ast))
        .filter(|u| !u.file.is_std && u.file.dependency.is_none())
        .map(|u| &u.file.module)
        .collect();
    if wanted.is_empty() {
        return Ok(Vec::new());
    }
    let (ir, build_errors) = salvo_ir::build_program(program, &symbols, &resolution, &checked, None);
    if !build_errors.is_empty() {
        return Err(build_errors);
    }
    let fn_names = salvo_core::naming::FnNames::compute(program);
    let mut shared = Shared {
        program,
        symbols: &symbols,
        erased: &erased,
        abi: true,
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
        }
    }
    let mut files = Vec::new();
    for m in &ir.modules {
        if !wanted.contains(&m.path) {
            continue;
        }
        let before = shared.union_sizes.len();
        let mut em = ModuleEmitter::new(&mut shared, m);
        let (body, imports) = em.skeleton(m);
        let mentions_root = em.s.union_sizes.len() != before;
        if body.is_empty() {
            continue;
        }
        let mut all_imports: BTreeSet<String> = BTreeSet::new();
        all_imports.insert(format!("import {}.*", kotlin_package(&m.path)));
        all_imports.extend(imports);
        // [kt-platform-host] The union wrappers a fallible member's result
        // lowers to live in the root `salvo` package.
        if mentions_root {
            all_imports.insert("import salvo.*".to_string());
        }
        let module = &m.path;
        let mut content = format!(
            "// Host implementation of the platform declarations of Salvo module \
             `{module}`.\n//\n// Generated once by `salvo platform generate`; the \
             compiler never writes\n// this file again — it is yours. Nothing here \
             is checked by Salvo: the\n// Kotlin compiler checks it, against the \
             interfaces the backend generates\n// from the `platform handler` \
             declarations.\npackage {}\n\n",
            host_package(module)
        );
        for import in &all_imports {
            content.push_str(import);
            content.push('\n');
        }
        content.push_str(&body);
        files.push(EmittedFile { rel_path: salvo_core::host_rel_path(module, "kt"), content });
    }
    if shared.errors.is_empty() {
        Ok(files)
    } else {
        Err(shared.errors)
    }
}

fn has_platform_type(ast: &salvo_syntax::ast::Module) -> bool {
    ast.items.iter().any(|i| matches!(i, salvo_syntax::ast::Item::Type(t) if t.platform))
}

impl<'a, 'p> ModuleEmitter<'a, 'p> {
    /// The skeleton body of a module, and the imports its handlers need (the
    /// package of each effect a platform handler implements).
    fn skeleton(&mut self, m: &IrModule) -> (String, BTreeSet<String>) {
        let mut imports = BTreeSet::new();
        let saved = std::mem::take(&mut self.out);
        // [platform-type] One class per platform type, named after it.
        for d in &m.decls {
            if let Decl::PlatformType(t) = d {
                if t.platform {
                    self.platform_type_skeleton(t);
                }
            }
        }
        // [platform-fn] One function per platform fn, with the real name.
        for d in &m.decls {
            if let Decl::Fn(f) = d {
                if f.kind == FnKind::Platform {
                    let tps = self.type_params(&f.type_params);
                    let ps = self.params(&f.params);
                    let ret = self.ret_ty(&f.ret);
                    self.out.push_str(&format!("fun{tps} {}({ps}){ret} {{\n    TODO(\"implement {}\")\n}}\n", kt_ident(&f.name), f.name));
                }
            }
        }
        // [platform-handler] One class per platform handler, named after the
        // handler itself: the `use` site constructs *this* class.
        for d in &m.decls {
            let Decl::Impl(h) = d else { continue };
            if !h.platform {
                continue;
            }
            let Some(Ty::Named { name: face_key, .. }) = h.faces.first().map(|f| f.strip_quals().clone()) else { continue };
            let plain = salvo_core::typekey::plain(&face_key).to_string();
            let Some(iface) = self.interface_by_name(&plain) else {
                self.error(format!("platform handler `{}` implements `{plain}`, which is not a declared effect", h.name));
                continue;
            };
            if iface.id.module != m.path {
                imports.insert(format!("import {}.*", kotlin_package(&iface.id.module)));
            }
            let ctor = if h.ctor_params.is_empty() {
                String::new()
            } else {
                let ps: Vec<String> = h.ctor_params.iter().map(|p| format!("private val {}: {}", kt_local(&p.local), self.ty(&p.ty))).collect();
                format!("({})", ps.join(", "))
            };
            // [threadsafe-platform] [kt-platform-handler] The contract, printed
            // where the implementer signs it.
            let contract = if h.threadsafe {
                format!(
                    "\n// `threadsafe platform handler {}` — THE CONTRACT YOU ARE SIGNING:\n\
                     // this instance is shared across every thread of the program with NO\n\
                     // lock around it. Every member below may run concurrently with every\n\
                     // other, so any mutable state needs its own synchronization\n\
                     // (`ConcurrentHashMap`, atomics, `synchronized` blocks of your own). If\n\
                     // the host cannot promise that, delete `threadsafe` from the Salvo\n\
                     // declaration: the compiler then serializes the instance for you\n\
                     // [threadsafe-platform].\n",
                    h.name
                )
            } else {
                format!(
                    "\n// `platform handler {}` — the compiler SERIALIZES this instance: every\n\
                     // member runs under one `synchronized` monitor on both backends, so\n\
                     // plain fields are fine. If the host synchronizes internally and wants\n\
                     // to run concurrently, declare it `threadsafe platform handler` in\n\
                     // Salvo and regenerate [threadsafe-platform].\n",
                    h.name
                )
            };
            self.out.push_str(&format!("{contract}class {}{ctor} : {}Platform {{\n", kt_ident(&h.name), iface.name));
            let effect_ast = self.s.symbols.effects.get(face_key.as_str()).copied();
            for (i, mem) in iface.members.iter().enumerate() {
                let mtps = self.type_params(&mem.type_params);
                let ps = self.params(&mem.params);
                let ret = self.ret_ty(&mem.ret);
                if let Some(f) = effect_ast.and_then(|e| e.fns.get(i)) {
                    self.out.push_str(&salvo_core::reply_contract_comment(f));
                }
                self.out.push_str(&format!(
                    "    override fun{mtps} {}({ps}){ret} {{\n        TODO(\"implement {}.{}\")\n    }}\n",
                    kt_ident(&mem.emitted_name),
                    iface.name,
                    mem.name
                ));
            }
            self.out.push_str("}\n");
        }
        let body = std::mem::replace(&mut self.out, saved);
        (body, imports)
    }

    /// [platform-type] The host's class for a platform type, and the `each`
    /// of an iterable one [platform-iterable].
    fn platform_type_skeleton(&mut self, t: &salvo_ir::PlatformTypeDecl) {
        let file_idx = self.s.program.files.iter().position(|f| f.module == t.id.module);
        let ast = file_idx.and_then(|fi| match self.s.program.modules[fi].items.get(t.id.item) {
            Some(salvo_syntax::ast::Item::Type(td)) => Some(td),
            _ => None,
        });
        let threadsafe = ast.is_some_and(|td| td.threadsafe);
        let what = if t.linear {
            "owned by one holder at a time"
        } else if threadsafe {
            "copied as a handle (a copy shares this object) and used from several threads at once, so it must synchronize itself"
        } else {
            "copied as a handle: a copy shares this object"
        };
        let params = self.type_params(&t.type_params);
        self.out.push_str(&format!("\n// `platform type {}`: {what} [platform-type].\nclass {}{params} {{\n}}\n", t.name, t.name));
        if let Some(td) = ast.filter(|td| td.iterable) {
            let elem = td
                .obligations
                .iter()
                .find(|o| o.group.name.name == "Iter")
                .and_then(|o| o.group.args.get(1))
                .map(|a| {
                    let fi = file_idx.unwrap_or(0);
                    let ctx_ty = salvo_core::wire::approx_ty(a, &HashMap::new()).unwrap_or(Ty::Unknown);
                    let _ = fi;
                    if ctx_ty.is_unknown() { "Any".to_string() } else { self.ty(&ctx_ty) }
                })
                .unwrap_or_else(|| "Any".to_string());
            let fn_params = if params.is_empty() { String::new() } else { format!("{params} ") };
            let each = salvo_core::case::camel(&format!("each{}", salvo_core::naming::each_suffix(self.s.program, &t.id.module, &t.name)));
            self.out.push_str(&format!(
                "\n// [platform-iterable] What a `for` over a `{}` loops over.\nfun {fn_params}{each}(x: {}{params}): Iterable<{elem}> = TODO(\"implement each\")\n",
                t.name, t.name
            ));
        }
    }
}
