//! [platform-tree] The host implementation skeletons, from the IR: one file
//! per user module declaring platform types, fns or handlers, whose
//! signatures are the ones the generated wrappers and adapters call — so the
//! two cannot disagree.

use std::collections::BTreeSet;

use salvo_core::types::Ty;
use salvo_ir::{Decl, FnKind, ImplDecl, PlatformTypeDecl};

use super::{decls::subst_ty, ModuleEmitter};
use crate::emit::{platform_trait_name, rs_ident};

impl<'a, 'p> ModuleEmitter<'a, 'p> {
    /// The skeleton file's body and the module paths its signatures need in
    /// scope beyond its own; `None` when the module declares nothing for the
    /// host.
    pub(super) fn skeleton(&mut self, own_path: &str) -> Option<(String, BTreeSet<String>)> {
        let module = self.module;
        let decls: Vec<&'a Decl> = module.decls.iter().collect();
        let mut body = String::new();
        let mut paths: BTreeSet<String> = BTreeSet::new();
        let mut any = false;
        // [platform-type] One struct per platform type, named after it.
        for d in &decls {
            if let Decl::PlatformType(t) = d {
                if t.platform {
                    any = true;
                    let code = self.platform_type_skeleton(t);
                    body.push_str(&code);
                }
            }
        }
        // [platform-fn] One function per platform fn: the wrapper's own
        // signature under the real name, with a stub body.
        for d in &decls {
            if let Decl::Fn(f) = d {
                if f.kind == FnKind::Platform {
                    any = true;
                    let wrapper = self.fn_decl(f, None, 0);
                    let Some(open) = wrapper.find(" {\n") else { continue };
                    let start = wrapper.find("pub fn ").unwrap_or(0);
                    let header = &wrapper[start..open];
                    let after = header["pub fn ".len()..].find(|c: char| !(c.is_alphanumeric() || c == '_')).map(|i| i + "pub fn ".len()).unwrap_or(header.len());
                    // The wrapper's `mut` bindings are its own business.
                    let mut header = format!("pub fn {}{}", rs_ident(&f.name), &header[after..]).replace("(mut ", "(").replace(", mut ", ", ");
                    // [platform-never] The host's fn answering `Never` is `-> !`.
                    if matches!(f.ret, Ty::Never) && !header.contains(" -> ") {
                        header.push_str(" -> !");
                    }
                    body.push_str(&format!("\n{header} {{\n    todo!(\"implement {}\")\n}}\n", f.name));
                }
            }
        }
        // [platform-handler] One struct per platform handler, named after the
        // handler itself: the `use` site constructs *this* struct through
        // `::new(…)`. The trait is the ordinary effect's host face, which may
        // live in another module (std's, typically).
        for d in &decls {
            if let Decl::Impl(h) = d {
                if h.platform {
                    any = true;
                    if let Some(path) = self.host_handler_skeleton(h, &mut body) {
                        if path != own_path {
                            paths.insert(path);
                        }
                    }
                }
            }
        }
        any.then_some((body, paths))
    }

    fn platform_type_skeleton(&mut self, t: &PlatformTypeDecl) -> String {
        let name = rs_ident(&t.name);
        let (derive, what) = if t.linear {
            ("", "owned by one holder at a time; must be `Send`")
        } else if t.threadsafe {
            ("#[derive(Clone)]\n", "copied as a handle and used from several threads at once: `Clone` (cheaply — an `Arc` inside), `Send` and `Sync`")
        } else {
            ("#[derive(Clone)]\n", "copied as a handle: `Clone` (cheaply — an `Arc` inside) and `Send`")
        };
        let sfx = salvo_core::naming::each_suffix(self.s.program, &self.module.path, &t.name);
        let generic = !t.type_params.is_empty();
        // [platform-iterable] The host's loop: what the contract assertion
        // beside the re-export checks.
        let each = |this: &mut Self, ty: &str, gens: &str| -> String {
            if !t.iterable {
                return String::new();
            }
            let elem = match &t.iter_elem {
                Some(e) => this.ty(&subst_ty(e, &Default::default())),
                None => "()".to_string(),
            };
            let mut out = format!(
                "\n// [platform-iterable] What a `for` over a `{}` loops over.\n\
                 pub fn each{sfx}{gens}(x: &{ty}) -> impl Iterator<Item = {}> + '_ {{\n    todo!(\"implement each\");\n    #[allow(unreachable_code)]\n    std::iter::empty()\n}}\n",
                t.name,
                if generic { format!("&{elem}") } else { elem.clone() }
            );
            if generic {
                out.push_str(&format!(
                    "\npub fn each{sfx}_mut{gens}(x: &mut {ty}) -> impl Iterator<Item = &mut {elem}> + '_ {{\n    todo!(\"implement each_mut\");\n    #[allow(unreachable_code)]\n    std::iter::empty()\n}}\n\
                     \npub fn into_each{sfx}{gens}(x: {ty}) -> impl Iterator<Item = {elem}> {{\n    todo!(\"implement into_each\");\n    #[allow(unreachable_code)]\n    std::iter::empty()\n}}\n"
                ));
            }
            out
        };
        if !generic {
            let each = each(self, &name, "");
            return format!("\n// `platform type {}`: {what} [platform-type].\n{derive}pub struct {name} {{\n}}\n{each}", t.name);
        }
        // [platform-generic] A type parameter is opaque: the host may store
        // and move one, so the skeleton carries it as a phantom.
        let ps: Vec<String> = t.type_params.iter().map(|g| g.name.clone()).collect();
        let bounded = ps.iter().map(|p| format!("{p}: Send + 'static")).collect::<Vec<_>>().join(", ");
        let each = each(self, &format!("{name}<{}>", ps.join(", ")), &format!("<{bounded}>"));
        format!(
            "\n// `platform type {}`: {what}; its type parameters are opaque [platform-type].\n\
             {derive}pub struct {name}<{bounded}> {{\n    _of: std::marker::PhantomData<fn() -> ({},)>,\n}}\n{each}",
            t.name,
            ps.join(", ")
        )
    }

    /// The host struct of a platform handler and its trait impl; answers
    /// the path of the module declaring the effect.
    fn host_handler_skeleton(&mut self, h: &ImplDecl, out: &mut String) -> Option<String> {
        let Some(Ty::Named { name: face, .. }) = h.faces.first().map(|f| f.strip_quals().clone()) else { return None };
        let Some(iface) = self.s.interface(&face) else {
            self.error(format!("platform handler `{}` implements `{face}`, which is not a declared effect", h.name));
            return None;
        };
        let prefix = self.type_prefix(&face);
        let effect_path = prefix.trim_end_matches("::").to_string();
        let name = rs_ident(&h.name);
        // [threadsafe-platform] The contract, printed where the implementer
        // signs it.
        out.push_str(&if h.threadsafe {
            format!(
                "\n// `threadsafe platform handler {}` — THE CONTRACT YOU ARE SIGNING:\n\
                 // every member below is safe to run concurrently with every other, so\n\
                 // any mutable state has its own synchronization (`Mutex`, `RwLock`,\n\
                 // atomics). Today the compiler still serializes the instance behind\n\
                 // the effect's handle, so the receivers are `&mut self` and the\n\
                 // struct must be `Send`; the contract is what lets a later emission\n\
                 // drop the lock [threadsafe-platform].\n\
                 pub struct {name} {{\n",
                h.name
            )
        } else {
            format!(
                "\n// `platform handler {}` — the compiler SERIALIZES this instance: every\n\
                 // member runs under one lock on both backends, so the receivers are\n\
                 // `&mut self` and plain fields are fine; the struct must be `Send`.\n\
                 // If the host synchronizes internally, declare it `threadsafe platform\n\
                 // handler` in Salvo to say so [threadsafe-platform].\n\
                 pub struct {name} {{\n",
                h.name
            )
        });
        let ps: Vec<(String, String)> = h.ctor_params.iter().map(|p| (rs_ident(&p.local.0), self.ty(&p.ty))).collect();
        for (n, t) in &ps {
            out.push_str(&format!("    {n}: {t},\n"));
        }
        out.push_str("}\n");
        let ctor: Vec<String> = ps.iter().map(|(n, t)| format!("{n}: {t}")).collect();
        out.push_str(&format!("\nimpl {name} {{\n    pub fn new({}) -> Self {{\n        Self {{", ctor.join(", ")));
        if ps.is_empty() {
            out.push_str(" }\n    }\n}\n");
        } else {
            out.push('\n');
            for (n, _) in &ps {
                out.push_str(&format!("            {n},\n"));
            }
            out.push_str("        }\n    }\n}\n");
        }
        // [threadsafe-platform] [rs-handle] Which trait, and which receiver.
        let (trait_name, receiver) = if h.threadsafe { (platform_trait_name(&rs_ident(&iface.name), true), "&self") } else { (platform_trait_name(&rs_ident(&iface.name), false), "&mut self") };
        out.push_str(&format!("\nimpl {prefix}{trait_name} for {name} {{\n"));
        let ast = self.s.symbols.effects.get(face.as_str()).copied();
        for (i, m) in iface.members.iter().enumerate() {
            if !m.type_params.is_empty() {
                continue;
            }
            let sig = self.member_sig(m).replace("&RECV", receiver);
            let sig = if matches!(m.ret, Ty::Never) { format!("{sig} -> !") } else { sig };
            if let Some(f) = ast.and_then(|e| e.fns.get(i)).filter(|f| f.name.name == m.name) {
                out.push_str(&salvo_core::reply_contract_comment(f));
            }
            out.push_str(&format!("    {sig} {{\n        todo!(\"implement {}.{}\")\n    }}\n", iface.name, m.name));
        }
        out.push_str("}\n");
        Some(effect_path)
    }
}
