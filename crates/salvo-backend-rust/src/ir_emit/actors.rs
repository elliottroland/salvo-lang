//! [rs-actor] Actors over the IR: the message enum, codec, protocol constant
//! and addr stub per actor interface; the continuation and private enums and
//! the `SalvoActor` body per actor impl; and the scheduler primitives
//! (IR.md §6, §9 decision 2: the dispatch body and codecs are the
//! backend's for now).

use salvo_core::types::Ty;
use salvo_ir::{Decl, Expr, ExprKind, FnDecl, ImplDecl, InterfaceDecl, Local, ReplyTarget};

use super::body::rs_local;
use super::{is_copy_ty, ModuleEmitter};
use crate::emit::{rs_ident, stateful_trait_name, stateless_trait_name};

pub(crate) fn variant(member: &str) -> String {
    salvo_backend::emit_util::msg_variant_name(member)
}

impl<'a, 'p> ModuleEmitter<'a, 'p> {
    /// A declaration-relative path in the module of `id`.
    pub fn path_in(&self, m: &salvo_core::ModulePath, item: &str) -> String {
        format!("{}{item}", self.s.prefix(m))
    }

    pub fn iface_by_id(&self, id: &salvo_ir::DeclId) -> Option<&'p InterfaceDecl> {
        match self.s.decls.get(id) {
            Some(Decl::Interface(i)) => Some(i),
            _ => None,
        }
    }

    /// The actor interfaces an impl serves, in face order.
    pub fn actor_faces(&self, h: &ImplDecl) -> Vec<&'p InterfaceDecl> {
        h.faces
            .iter()
            .filter_map(|f| match f.strip_quals() {
                Ty::Named { name, .. } => self.s.interface(name),
                _ => None,
            })
            .filter(|i| i.actor)
            .collect()
    }

    /// [mixed-handler] Every face a plain effect, and a `send fn` member.
    pub fn is_mixed(&self, h: &ImplDecl) -> bool {
        !h.faces.is_empty() && h.members.iter().any(|m| m.send) && self.actor_faces(h).is_empty()
    }

    /// [actor-private-send] The impl's `send fn`s no face declares.
    pub fn private_sends<'h>(h: &'h ImplDecl, faces: &[&InterfaceDecl]) -> Vec<&'h FnDecl> {
        h.members.iter().filter(|m| m.send && !faces.iter().any(|f| f.members.iter().any(|fm| fm.name == m.name))).collect()
    }

    fn declared_params(m: &FnDecl) -> Vec<&salvo_ir::Param> {
        m.params.iter().skip(m.effect_params).take(m.params.len() - m.effect_params - m.implicit_params).collect()
    }

    /// The send members that can be resumed: those with a declared
    /// parameter (the answer is the last).
    pub fn parking<'h>(&self, h: &'h ImplDecl, faces: &[&InterfaceDecl]) -> Vec<&'h FnDecl> {
        let mut out = Vec::new();
        for face in faces {
            for fm in face.members.iter().filter(|m| m.send) {
                if let Some(m) = h.members.iter().find(|m| m.name == fm.name) {
                    if !Self::declared_params(m).is_empty() {
                        out.push(m);
                    }
                }
            }
        }
        for m in Self::private_sends(h, faces) {
            if !Self::declared_params(m).is_empty() {
                out.push(m);
            }
        }
        out
    }

    /// [actor-msg] A generated enum: its variants, and — for a message enum
    /// with a wire form — the codec and the protocol constant (the codec is
    /// the backend's, IR.md §9 decision 2).
    pub fn enum_decl(&mut self, e: &salvo_ir::EnumDecl) {
        let name = rs_ident(&e.name);
        match &e.kind {
            salvo_ir::EnumKind::Message { interface } => {
                // A generic protocol is refused where its stub is rendered.
                let Some(i) = self.iface_by_id(interface) else { return };
                if !i.type_params.is_empty() && !self.s.erased.is_erased(&i.name) {
                    return;
                }
            }
            // Nothing parks: nothing to resume into.
            salvo_ir::EnumKind::Continuation { .. } if e.variants.is_empty() => return,
            _ => {}
        }
        let mut out = format!("\npub enum {name} {{\n");
        for v in &e.variants {
            let payload: Vec<String> = v.fields.iter().map(|(_, t)| self.ty(t)).collect();
            if payload.is_empty() {
                out.push_str(&format!("    {},\n", v.name));
            } else {
                out.push_str(&format!("    {}({}),\n", v.name, payload.join(", ")));
            }
        }
        out.push_str("}\n");
        if let (salvo_ir::EnumKind::Message { interface }, Some(hash)) = (&e.kind, &e.protocol_hash) {
            let msg = &name;
            let mut enc = String::new();
            let mut dec = String::new();
            for (tag, v) in e.variants.iter().enumerate() {
                let vn = &v.name;
                let ps: Vec<String> = (0..v.fields.len()).map(|k| format!("__p{k}")).collect();
                if ps.is_empty() {
                    enc.push_str(&format!("            {msg}::{vn} => out.push({tag}),\n"));
                    dec.push_str(&format!("            {tag} => Some({msg}::{vn}),\n"));
                } else {
                    enc.push_str(&format!("            {msg}::{vn}({}) => {{\n                out.push({tag});\n", ps.join(", ")));
                    for p in &ps {
                        enc.push_str(&format!("                crate::wire::__Wire::__enc({p}, out);\n"));
                    }
                    enc.push_str("            }\n");
                    let ds: Vec<&str> = ps.iter().map(|_| "crate::wire::__Wire::__dec(r)?").collect();
                    dec.push_str(&format!("            {tag} => Some({msg}::{vn}({})),\n", ds.join(", ")));
                }
            }
            out.push_str(&format!(
                "\nimpl crate::wire::__Wire for {msg} {{\n    fn __enc(&self, out: &mut Vec<u8>) {{\n        match self {{\n{enc}        }}\n    }}\n    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {{\n        match r.u8()? {{\n{dec}            _ => None,\n        }}\n    }}\n}}\n"
            ));
            let iname = self.iface_by_id(interface).map(|i| i.name.clone()).unwrap_or_default();
            out.push_str(&format!("\n/// [protocol-hash] The canonical hash of `{iname}`.\npub const __PROTO_{}: &str = \"{hash}\";\n", rs_ident(&iname)));
        }
        self.out.push_str(&out);
    }

    /// [rs-actor] The actor parts of an interface: message enum, codec and
    /// hash constant, and the addr stub.
    pub fn actor_interface(&mut self, i: &InterfaceDecl) {
        let sends: Vec<&salvo_ir::Member> = i.members.iter().filter(|m| m.send).collect();
        if sends.is_empty() {
            return;
        }
        if !i.type_params.is_empty() && !self.s.erased.is_erased(&i.name) {
            self.error(format!("effect `{}` is generic, which the rust backend cannot make an actor protocol of yet", i.name));
            return;
        }
        let name = rs_ident(&i.name);
        let msg = format!("__Msg_{name}");
        let mut out = String::new();
        // [actor-use-addr] The stub: the effect implemented by sending.
        let stub = format!("__Stub_{name}");
        out.push_str(&format!(
            "\npub struct {stub} {{\n    addr: usize,\n}}\n\nimpl {stub} {{\n    pub fn new(addr: usize) -> Self {{\n        Self {{ addr }}\n    }}\n}}\n\nimpl {} for {stub} {{\n",
            stateless_trait_name(&i.name)
        ));
        for m in &sends {
            let mut ps = Vec::new();
            for p in &m.params {
                let t = self.ty(&p.ty);
                ps.push(format!(", {}: {t}", rs_ident(&p.local.0)));
            }
            let args: Vec<String> = m.params.iter().map(|p| rs_ident(&p.local.0)).collect();
            let built = if args.is_empty() { format!("{msg}::{}", variant(&m.emitted_name)) } else { format!("{msg}::{}({})", variant(&m.emitted_name), args.join(", ")) };
            let call = self.send_call(i, "self.addr", &built);
            out.push_str(&format!("    fn {}(&self{}) {{\n        {call};\n    }}\n", rs_ident(&m.emitted_name), ps.join("")));
        }
        out.push_str("}\n");
        self.out.push_str(&out);
    }

    /// [addr-routable] A send: the typed wire path when the protocol has a
    /// wire form, the plain enqueue otherwise.
    pub fn send_call(&mut self, i: &InterfaceDecl, target: &str, built: &str) -> String {
        if i.protocol_hash.is_some() {
            let proto = self.path_in(&i.id.module, &format!("__PROTO_{}", rs_ident(&i.name)));
            format!("crate::scheduler::salvo_send_wire({target}, {built}, {proto})")
        } else {
            format!("crate::scheduler::salvo_send({target}, std::boxed::Box::new({built}))")
        }
    }

    /// The generated fields of an actor impl, and their initializers.
    pub fn actor_fields(&mut self, h: &ImplDecl, faces: &[&InterfaceDecl]) -> (Vec<(String, String)>, Vec<String>) {
        let cap = h.mailbox.as_ref().and_then(|m| match &m.kind {
            ExprKind::Construct { fields } => fields.iter().find(|(n, _)| n == "capacity").map(|(_, v)| v.clone()),
            _ => None,
        });
        let cap_code = match cap {
            Some(e) => self.value(&e, 3),
            None => "0".to_string(),
        };
        let mut fields = vec![("pub __mailbox_capacity".to_string(), "i32".to_string()), ("__addr".to_string(), "Option<usize>".to_string())];
        let mut inits = vec![format!("__mailbox_capacity: {cap_code}"), "__addr: None".to_string()];
        if !self.parking(h, faces).is_empty() {
            fields.push(("__parked".to_string(), format!("std::collections::HashMap<u64, __Cont_{}>", rs_ident(&h.name))));
            inits.push("__parked: std::collections::HashMap::new()".to_string());
        }
        (fields, inits)
    }

    /// [actor-dispatch] The path of the generated function delivering the
    /// message enum `which` selects to `h`'s members.
    fn dispatch_fn(&mut self, h: &ImplDecl, which: impl Fn(&salvo_ir::EnumKind) -> bool) -> Option<String> {
        let id = h.dispatch.iter().find(|d| matches!(self.s.decls.get(&d.message), Some(Decl::Enum(e)) if which(&e.kind)))?.func.clone();
        Some(self.fn_path(&id))
    }

    /// [rs-actor] `__Actor_H` beside an actor impl (its enums and dispatch
    /// functions are the builder's).
    pub fn actor_body(&mut self, h: &ImplDecl, faces: &[&'p InterfaceDecl]) -> String {
        let hn = rs_ident(&h.name);
        let privs = Self::private_sends(h, faces);
        let parking = self.parking(h, faces);
        let cont = format!("__Cont_{hn}");
        let priv_enum = format!("__Priv_{hn}");
        let mut out = String::new();
        let has_priv = h.init.is_some() || !privs.is_empty();
        let actor = format!("__Actor_{hn}");
        out.push_str(&format!("\npub struct {actor} {{\n    handler: {hn},\n}}\n\nimpl {actor} {{\n    pub fn new(handler: {hn}) -> Self {{\n        Self {{ handler }}\n    }}\n"));
        let single = faces.len() == 1 && !has_priv;
        // One dispatcher per face, one for the private messages: the
        // functions the builder generated [actor-dispatch].
        let mut dispatch_names: Vec<(String, String)> = Vec::new();
        for face in faces {
            let msg = self.path_in(&face.id.module, &format!("__Msg_{}", rs_ident(&face.name)));
            match self.dispatch_fn(h, |k| matches!(k, salvo_ir::EnumKind::Message { interface } if *interface == face.id)) {
                Some(f) => dispatch_names.push((msg, f)),
                None => self.error(format!("handler `{}` has no dispatcher for `{}`", h.name, face.name)),
            }
        }
        let priv_fn = if has_priv { self.dispatch_fn(h, |k| matches!(k, salvo_ir::EnumKind::Private { handler } if *handler == h.id)) } else { None };
        out.push_str("}\n");
        // handle
        let mut handle = String::new();
        if single {
            let (msg, d) = &dispatch_names[0];
            handle.push_str(&format!("        let msg = *msg.downcast::<{msg}>().expect(\"message of this protocol\");\n        {d}(&mut self.handler, msg);\n"));
        } else {
            for (msg, d) in &dispatch_names {
                handle.push_str(&format!("        let msg = match msg.downcast::<{msg}>() {{\n            Ok(__m) => return {d}(&mut self.handler, *__m),\n            Err(__m) => __m,\n        }};\n"));
            }
            if let Some(pf) = &priv_fn {
                handle.push_str(&format!("        let msg = match msg.downcast::<{priv_enum}>() {{\n            Ok(__m) => return {pf}(&mut self.handler, *__m),\n            Err(__m) => __m,\n        }};\n"));
            }
            handle.push_str("        let _ = msg;\n        unreachable!(\"a message of one of this actor's protocols\")\n");
        }
        out.push_str(&format!(
            "\nimpl crate::scheduler::SalvoActor for {actor} {{\n    fn handle(&mut self, _ctx: &crate::scheduler::SalvoCtx, msg: crate::scheduler::SalvoMsg) {{\n        self.handler.__addr = Some(_ctx.addr);\n{handle}    }}\n"
        ));
        if parking.is_empty() {
            out.push_str("\n    fn resume(&mut self, _ctx: &crate::scheduler::SalvoCtx, _slot: u64, _value: crate::scheduler::SalvoMsg) {\n        unreachable!(\"this protocol has no continuation targets\")\n    }\n");
        } else {
            let mut resume = String::new();
            let mut decode = String::new();
            for m in &parking {
                let declared = Self::declared_params(m);
                let caps: Vec<String> = declared[..declared.len() - 1].iter().map(|p| rs_ident(&p.local.0)).collect();
                let bind = if caps.is_empty() { String::new() } else { format!("({})", caps.join(", ")) };
                let last = declared[declared.len() - 1];
                let at = self.ty(&last.ty);
                let mut args = caps.clone();
                args.push(format!("*value.downcast::<{at}>().expect(\"the awaited answer\")"));
                // The message it rebuilds: the face's, or the private one.
                let face = faces.iter().find(|f| f.members.iter().any(|fm| fm.name == m.name && fm.send));
                let call = match face {
                    Some(face) => {
                        let fm = face.members.iter().find(|fm| fm.name == m.name).unwrap();
                        let (msg, d) = dispatch_names.iter().find(|(msg, _)| msg.ends_with(&format!("__Msg_{}", rs_ident(&face.name)))).cloned().unwrap();
                        format!("{d}(&mut self.handler, {msg}::{}({}))", variant(&fm.emitted_name), args.join(", "))
                    }
                    None => format!("{}(&mut self.handler, {priv_enum}::{}({}))", priv_fn.clone().unwrap_or_default(), variant(&m.name), args.join(", ")),
                };
                resume.push_str(&format!("            {cont}::{}{bind} => {call},\n", variant(&m.name)));
                let dec = self.reply_decoder(&last.ty);
                decode.push_str(&format!("            {cont}::{}{{ .. }} => {dec}(payload),\n", variant(&m.name)));
            }
            out.push_str(&format!(
                "\n    fn resume(&mut self, _ctx: &crate::scheduler::SalvoCtx, slot: u64, value: crate::scheduler::SalvoMsg) {{\n        self.handler.__addr = Some(_ctx.addr);\n        let Some(__cont) = self.handler.__parked.remove(&slot) else {{\n            return; // a reply whose continuation is gone: nothing to run\n        }};\n        match __cont {{\n{resume}        }}\n    }}\n\n    fn decode_reply(&self, slot: u64, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {{\n        match self.handler.__parked.get(&slot)? {{\n{decode}        }}\n    }}\n"
            ));
        }
        out.push_str("}\n");
        // [addr-routable] The frame decoder, by protocol hash.
        let mut arms = String::new();
        for face in faces {
            if face.protocol_hash.is_some() {
                let msg = self.path_in(&face.id.module, &format!("__Msg_{}", rs_ident(&face.name)));
                let proto = self.path_in(&face.id.module, &format!("__PROTO_{}", rs_ident(&face.name)));
                arms.push_str(&format!("    if proto == {proto} {{\n        return crate::wire::salvo_decode::<{msg}>(payload).map(|__m| std::boxed::Box::new(__m) as crate::scheduler::SalvoMsg);\n    }}\n"));
            }
        }
        if arms.is_empty() {
            out.push_str(&format!("\npub const __DECODE_{hn}: Option<crate::scheduler::MsgDecoder> = None;\n"));
        } else {
            out.push_str(&format!(
                "\npub const __DECODE_{hn}: Option<crate::scheduler::MsgDecoder> = Some(__decode_msg_{hn});\nfn __decode_msg_{hn}(proto: &str, payload: &[u8]) -> Option<crate::scheduler::SalvoMsg> {{\n{arms}    None\n}}\n"
            ));
        }
        out
    }

    /// [addr-routable] The `ReplyDecoder` for an answer of type `t`.
    pub fn reply_decoder(&mut self, t: &Ty) -> String {
        let t = t.strip_quals().clone();
        if salvo_core::wire_blocker(self.s.symbols, &t).is_some() || super::body::foreign_var(&t, &self.f.generics) {
            return "(|_| None)".to_string();
        }
        let rs = self.ty(&t);
        format!("(|__b: &[u8]| crate::wire::salvo_decode::<{rs}>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg))")
    }

    // ------------------------------------------------- the primitives --

    pub fn spawn(&mut self, handler: &Expr, pool: Option<&Expr>, join: Option<&Expr>, indent: usize) -> String {
        let Ty::Named { name, .. } = handler.ty.strip_quals().clone() else {
            self.error("`spawn` of something that is not a handler");
            return "()".to_string();
        };
        let Some(decl) = self.s.impl_decl(&name) else {
            self.error(format!("`spawn` of an unknown handler `{name}`"));
            return "()".to_string();
        };
        let faces = self.actor_faces(decl);
        if faces.is_empty() && self.is_mixed(decl) {
            // [mixed-handler] [rs-mixed] constructor arguments evaluated once,
            // cloned into the servant and moved into the façade.
            let ExprKind::Construct { fields } = &handler.kind else {
                self.error("`spawn` of a mixed handler needs its construction");
                return "()".to_string();
            };
            let mut lets = String::new();
            let mut servant_args = Vec::new();
            let mut fac_fields = vec!["__addr: __a".to_string()];
            for (i, p) in decl.ctor_params.iter().enumerate() {
                let Some((_, v)) = fields.iter().find(|(n, _)| *n == p.local.0) else { continue };
                let code = self.value_into(v, &p.ty, indent);
                lets.push_str(&format!("let __c{i} = {code}; "));
                servant_args.push(format!("__c{i}.clone()"));
                fac_fields.push(format!("{}: __c{i}", rs_ident(&p.local.0)));
            }
            for i in 0..decl.deps.len() {
                if let Some((_, v)) = fields.iter().find(|(n, _)| *n == format!("__dep{i}")) {
                    servant_args.push(self.value(v, indent));
                }
            }
            let pool_code = match pool {
                Some(p) => self.value(p, indent),
                None => "crate::scheduler::salvo_current_pool()".to_string(),
            };
            let m = decl.id.module.clone();
            let hn = rs_ident(&decl.name);
            let ctor = self.path_in(&m, &hn);
            let actor = self.path_in(&m, &format!("__Actor_{hn}"));
            let fac = self.path_in(&m, &format!("__Fac_{hn}"));
            let face = match decl.faces.first().map(|f| f.strip_quals()) {
                Some(Ty::Named { name, .. }) => self.type_path(name),
                _ => String::new(),
            };
            let init = if decl.init.is_some() {
                format!("crate::scheduler::salvo_send(__a, std::boxed::Box::new({}::Init)); ", self.path_in(&m, &format!("__Priv_{hn}")))
            } else {
                String::new()
            };
            return format!(
                "({{ {lets}let __h = {ctor}::new({}); let __cap = __h.__mailbox_capacity; let __a = crate::scheduler::salvo_spawn({pool_code}, __cap as usize, std::boxed::Box::new({actor}::new(__h)), None); {init}{face}::shared({fac} {{ {} }}) }})",
                servant_args.join(", "),
                fac_fields.join(", ")
            );
        }
        if faces.is_empty() {
            // [monitor-handler] the instance shared behind its face's handle.
            if decl.faces.len() != 1 {
                self.error(format!("handler `{}` implements several plain effects, and a shared instance behind several faces is not supported yet", decl.name));
                return "()".to_string();
            }
            let face = match decl.faces[0].strip_quals() {
                Ty::Named { name, .. } => self.type_path(name),
                _ => String::new(),
            };
            let h = self.value(handler, indent);
            let mk = if decl.stateful { "locked" } else { "shared" };
            return format!("{face}::{mk}({h})");
        }
        if !decl.type_params.is_empty() && !self.s.erased.is_erased(&decl.name) {
            self.error(format!("spawning generic handler `{}` is not supported yet", decl.name));
            return "()".to_string();
        }
        let h = self.value(handler, indent);
        let pool_code = match pool {
            Some(p) => self.value(p, indent),
            None => "crate::scheduler::salvo_current_pool()".to_string(),
        };
        let m = decl.id.module.clone();
        let hn = rs_ident(&decl.name);
        let actor = self.path_in(&m, &format!("__Actor_{hn}"));
        let dec = self.path_in(&m, &format!("__DECODE_{hn}"));
        let init = if decl.init.is_some() {
            format!("crate::scheduler::salvo_send(__a, std::boxed::Box::new({}::Init)); ", self.path_in(&m, &format!("__Priv_{hn}")))
        } else {
            String::new()
        };
        let joined = match join {
            Some(g) => {
                let Some(group) = self.s.interface("ActorGroup") else {
                    self.error("`spawn … in group` needs std `net`");
                    return "()".to_string();
                };
                let Some(jm) = group.members.iter().find(|m| m.name == "join") else {
                    self.error("`ActorGroup` has no `join` member");
                    return "()".to_string();
                };
                let g_code = self.value(g, indent);
                let built = format!("{}::{}(__a)", self.path_in(&group.id.module, &format!("__Msg_{}", rs_ident(&group.name))), variant(&jm.emitted_name));
                format!("{}; ", self.send_call(group, &g_code, &built))
            }
            None => String::new(),
        };
        let result = if faces.len() > 1 { format!("({})", vec!["__a"; faces.len()].join(", ")) } else { "__a".to_string() };
        format!("({{ let __h = {h}; let __cap = __h.__mailbox_capacity; let __a = crate::scheduler::salvo_spawn({pool_code}, __cap as usize, std::boxed::Box::new({actor}::new(__h)), {dec}); {init}{joined}{result} }})")
    }

    pub fn send(&mut self, addr: &Expr, member: &salvo_ir::MemberRef, args: &[Expr], indent: usize) -> String {
        let Some(iface) = self.iface_by_id(&member.interface) else {
            self.error("a send to an unknown interface");
            return "()".to_string();
        };
        let m = &iface.members[member.index];
        let a = self.value(addr, indent);
        let mut ps = Vec::new();
        for (p, x) in m.params.iter().zip(args) {
            ps.push(self.value_into(x, &p.ty, indent));
        }
        let msg = self.path_in(&iface.id.module, &format!("__Msg_{}", rs_ident(&iface.name)));
        let built = if ps.is_empty() { format!("{msg}::{}", variant(&m.emitted_name)) } else { format!("{msg}::{}({})", variant(&m.emitted_name), ps.join(", ")) };
        self.send_call(iface, &a, &built)
    }

    pub fn addr_instance(&mut self, e: &Expr, addr: &Expr, indent: usize) -> String {
        let a = self.value(addr, indent);
        let Ty::Named { name, .. } = e.ty.strip_quals() else {
            self.error("an addr instance of a non-effect type");
            return a;
        };
        match self.s.interface(name) {
            Some(i) if i.actor => {
                let face = self.type_path(name);
                let stub = self.path_in(&i.id.module, &format!("__Stub_{}", rs_ident(&i.name)));
                format!("{face}::shared({stub}::new({a}))")
            }
            Some(_) => a,
            None => {
                self.error(format!("`{name}` is not an effect, so an addr is not an instance of it"));
                a
            }
        }
    }

    pub fn replyto(&mut self, target: &ReplyTarget, captures: &[Expr], gated: bool, pool: Option<&Expr>, indent: usize) -> String {
        match target {
            ReplyTarget::Member(member) => {
                let Some(h) = self.f.current_impl.clone() else {
                    self.error("`replyto` outside a handler");
                    return "()".to_string();
                };
                let Some(decl) = self.s.impl_decl(&h) else { return "()".to_string() };
                let Some(m) = decl.members.iter().find(|m| m.name == *member) else {
                    self.error(format!("`replyto {member}`: no such member"));
                    return "()".to_string();
                };
                let declared = Self::declared_params(m);
                let mut caps = Vec::new();
                for (c, p) in captures.iter().zip(&declared) {
                    caps.push(self.value_into(c, &p.ty, indent));
                }
                let cont = format!("__Cont_{}", rs_ident(&decl.name));
                let built = if caps.is_empty() { format!("{cont}::{}", variant(member)) } else { format!("{cont}::{}({})", variant(member), caps.join(", ")) };
                let mint = if gated { "salvo_mint_gated" } else { "salvo_mint" };
                format!("{{ let (__r, __s) = crate::scheduler::{mint}(self.__addr.expect(\"a parking handler runs as an actor\")); self.__parked.insert(__s, {built}); __r }}")
            }
            ReplyTarget::Task { target, effects } => {
                let Some(decl) = self.s.fn_decl(target) else {
                    self.error("`replyto` of an unknown task");
                    return "()".to_string();
                };
                let declared: Vec<salvo_ir::Param> = decl.params.iter().skip(decl.effect_params).cloned().collect();
                let Some(last) = declared.last().cloned() else {
                    self.error("a task target has no answer parameter");
                    return "()".to_string();
                };
                let payload = self.ty(&last.ty);
                let decoder = self.reply_decoder(&last.ty);
                let name = self.fn_path(target);
                let mut lets = String::new();
                let mut args = Vec::new();
                for (i, e) in effects.iter().enumerate() {
                    let code = self.value(e, indent);
                    lets.push_str(&format!("let __e{i} = {code}; "));
                    args.push(format!("&__e{i}"));
                }
                for (i, c) in captures.iter().enumerate() {
                    let slot = declared.get(i).map(|p| p.ty.clone()).unwrap_or_else(|| c.ty.clone());
                    let code = self.value_into(c, &slot, indent);
                    lets.push_str(&format!("let __c{i} = {code}; "));
                    args.push(format!("__c{i}"));
                }
                args.push(format!("*__v.downcast::<{payload}>().expect(\"the awaited answer\")"));
                let pool_code = match pool {
                    Some(p) => self.value(p, indent),
                    None => "crate::scheduler::salvo_current_pool()".to_string(),
                };
                format!("({{ {lets}crate::scheduler::salvo_mint_task({pool_code}, std::boxed::Box::new(move |__v| {name}({})), {decoder}) }})", args.join(", "))
            }
        }
    }

    pub fn waitfor(&mut self, local: &Local, token_ty: &Ty, body: &salvo_ir::Block, indent: usize) -> String {
        let pad = "    ".repeat(indent + 1);
        let close = "    ".repeat(indent);
        let payload_ty = match token_ty.strip_quals() {
            Ty::Named { name, args } if name == "Reply" && args.len() == 1 => args[0].clone(),
            _ => {
                self.error("`waitfor` binds a `Reply<T>`");
                return "()".to_string();
            }
        };
        let payload = self.ty(&payload_ty);
        let decoder = self.reply_decoder(&payload_ty);
        let n = rs_local(&local.0);
        self.f.kinds.insert(local.0.clone(), super::body::Kind::Owned);
        self.f.tys.insert(local.0.clone(), token_ty.clone());
        let stmts = self.block_stmts_pub(body, indent + 1);
        format!(
            "{{\n{pad}let (mut {n}, __wid) = crate::scheduler::salvo_waiter();\n{pad}crate::scheduler::salvo_waiter_decoder(__wid, {decoder});\n{stmts}{pad}*crate::scheduler::salvo_wait(__wid).downcast::<{payload}>().expect(\"the awaited answer\")\n{close}}}"
        )
    }

    pub fn self_send(&mut self, member: &str, args: &[Expr], indent: usize) -> String {
        let Some(hname) = self.f.current_impl.clone() else {
            self.error("`@self` send outside an actor");
            return "()".to_string();
        };
        let Some(h) = self.s.impl_decl(&hname) else { return "()".to_string() };
        let faces = self.actor_faces(h);
        let decl = h.members.iter().find(|m| m.name == member);
        let ps: Vec<salvo_ir::Param> = decl.map(|m| Self::declared_params(m).into_iter().cloned().collect()).unwrap_or_default();
        let mut a = Vec::new();
        for (i, x) in args.iter().enumerate() {
            let slot = ps.get(i).map(|p| p.ty.clone()).unwrap_or_else(|| x.ty.clone());
            a.push(self.value_into(x, &slot, indent));
        }
        let names: Vec<String> = (0..a.len()).map(|i| format!("__s{i}")).collect();
        let lets: String = a.iter().zip(&names).map(|(c, n)| format!("let {n} = {c}; ")).collect();
        if let Some(face) = faces.iter().find(|f| f.members.iter().any(|m| m.name == member)) {
            let fm = face.members.iter().find(|m| m.name == member).unwrap();
            let msg = self.path_in(&face.id.module, &format!("__Msg_{}", rs_ident(&face.name)));
            let built = if names.is_empty() { format!("{msg}::{}", variant(&fm.emitted_name)) } else { format!("{msg}::{}({})", variant(&fm.emitted_name), names.join(", ")) };
            let send = self.send_call(face, "__a", &built);
            let tr = self.path_in(&face.id.module, &if h.stateful { stateful_trait_name(&face.name) } else { stateless_trait_name(&face.name) });
            let mut inline_args = vec!["self".to_string()];
            inline_args.extend(names.iter().cloned());
            return format!("{{ {lets}match self.__addr {{ Some(__a) => {send}, None => {tr}::{}({}) }} }}", rs_ident(&fm.emitted_name), inline_args.join(", "));
        }
        let priv_enum = self.path_in(&h.id.module, &format!("__Priv_{}", rs_ident(&h.name)));
        let built = if names.is_empty() { format!("{priv_enum}::{}", variant(member)) } else { format!("{priv_enum}::{}({})", variant(member), names.join(", ")) };
        if self.f.in_facade {
            return format!("{{ {lets}crate::scheduler::salvo_send(self.__addr, std::boxed::Box::new({built})) }}");
        }
        format!("{{ {lets}match self.__addr {{ Some(__a) => crate::scheduler::salvo_send(__a, std::boxed::Box::new({built})), None => self.{}({}) }} }}", rs_ident(member), names.join(", "))
    }

    /// [protocol-hash] `salvo_set_protocols` at the top of `main`.
    pub fn protocol_prelude(&mut self) -> String {
        let mut ifaces: Vec<&InterfaceDecl> = self
            .s
            .ir
            .modules
            .iter()
            .flat_map(|m| &m.decls)
            .filter_map(|d| match d {
                Decl::Interface(i) if i.actor && i.protocol_hash.is_some() && i.members.iter().any(|m| m.send) => Some(i),
                _ => None,
            })
            .collect();
        ifaces.sort_by(|a, b| a.name.cmp(&b.name));
        let entries: Vec<String> = ifaces
            .iter()
            .map(|i| format!("(\"{}\".to_string(), {}.to_string())", i.name, self.path_in(&i.id.module, &format!("__PROTO_{}", rs_ident(&i.name)))))
            .collect();
        if entries.is_empty() {
            return String::new();
        }
        format!("    crate::scheduler::salvo_set_protocols(vec![{}]);\n", entries.join(", "))
    }

    /// The actor and wire intrinsics; `None` when `name` is not one.
    pub fn actor_intrinsic(&mut self, e: &Expr, name: &str, recv: Option<&str>, type_args: &[Ty], args: &[Expr], indent: usize) -> Option<String> {
        match (name, args.len()) {
            ("encode", 1) => {
                if matches!(args[0].kind, ExprKind::Read { .. }) {
                    let v = self.raw(&args[0], indent);
                    return Some(format!("crate::wire::salvo_encode(&{v})"));
                }
                // A built value is typed first: its union's other arms are
                // not inferable from it.
                let t = self.ty(&args[0].ty);
                let v = self.value(&args[0], indent);
                Some(format!("{{ let __enc: {t} = {v}; crate::wire::salvo_encode(&__enc) }}"))
            }
            ("decode", 1) => {
                let t = type_args.first().cloned().unwrap_or_else(|| e.ty.strip_quals().without_none());
                let rs = self.ty(&t);
                let v = self.raw(&args[0], indent);
                Some(format!("crate::wire::salvo_decode::<{rs}>(&{v})"))
            }
            ("key_hash", 1) => {
                let k = self.raw(&args[0], indent);
                Some(format!("crate::scheduler::salvo_key_hash(&crate::wire::salvo_encode(&{k}))"))
            }
            ("send", 2) if recv == Some("Reply") => {
                let payload = match args[0].ty.strip_quals() {
                    Ty::Named { name, args: t } if name == "Reply" && t.len() == 1 => t[0].strip_quals().clone(),
                    _ => return None,
                };
                let tok = self.value(&args[0], indent);
                let val = self.value_into(&args[1], &payload, indent);
                let rs = self.ty(&payload);
                if salvo_core::wire_blocker(self.s.symbols, &payload).is_none() && !super::body::foreign_var(&payload, &self.f.generics) {
                    Some(format!("crate::scheduler::salvo_reply_wire::<{rs}>({tok}, {val})"))
                } else {
                    Some(format!("({tok}).send(std::boxed::Box::<{rs}>::new({val}))"))
                }
            }
            ("pool", 2) if recv == Some("Int") => {
                let a = self.value(&args[0], indent);
                let b = self.value(&args[1], indent);
                let faults = self.s.interface("Faults")?;
                let msg = self.path_in(&faults.id.module, "__Msg_Faults");
                let fault = self.type_path("Fault");
                Some(format!("crate::scheduler::salvo_pool_with_sink((({a}) as usize), Some(((({b}) as usize), |__reason| std::boxed::Box::new({msg}::Faulted({fault} {{ reason: __reason }})))))"))
            }
            ("protocol", 0) => {
                let effect = match e.ty.strip_quals() {
                    Ty::Named { name, args } if name == "Protocol" && args.len() == 1 => match args[0].strip_quals() {
                        Ty::Named { name, .. } => Some(name.clone()),
                        _ => None,
                    },
                    _ => None,
                };
                let iface = effect.and_then(|n| self.s.interface(&n)).filter(|i| i.protocol_hash.is_some());
                let Some(iface) = iface else {
                    self.error("`protocol<E>`: `E` has no wire form, so it cannot be a group's protocol");
                    return Some("()".to_string());
                };
                let proto = self.path_in(&iface.id.module, &format!("__PROTO_{}", rs_ident(&iface.name)));
                let st = self.type_path("Protocol");
                Some(format!("{st} {{ name: \"{}\".to_string(), hash: {proto}.to_string() }}", iface.name))
            }
            ("watch_control", 2) => {
                let channel = self.raw(&args[0], indent);
                let sink = self.raw(&args[1], indent);
                let h = self.f.current_impl.clone()?;
                let decl = self.s.impl_decl(&h)?;
                let control = decl.members.iter().find(|m| m.name == "control")?;
                let from = control.params.get(control.effect_params).map(|p| p.ty.clone()).unwrap_or(Ty::Unknown);
                let nid = match from.strip_quals() {
                    Ty::Named { name, .. } => self.type_path(name),
                    _ => "()".to_string(),
                };
                let priv_enum = self.path_in(&decl.id.module, &format!("__Priv_{}", rs_ident(&decl.name)));
                Some(format!("crate::scheduler::salvo_watch_control(({channel}).clone(), ({sink}).clone(), |__n, __d| std::boxed::Box::new({priv_enum}::Control({nid} {{ id: __n as i64 }}, __d)))"))
            }
            _ => {
                let _ = is_copy_ty(&e.ty);
                None
            }
        }
    }
}
