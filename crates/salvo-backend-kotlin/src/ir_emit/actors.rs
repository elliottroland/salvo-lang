//! [kt-ir] [kt-actor] Actors over the IR: the wire codecs per type, the
//! message classes and codec per actor interface, the dispatch body per actor
//! impl, and the scheduler primitives (IR.md §6, §9 decision 2: the codecs
//! and the dispatch body are the backend's for now).

use std::collections::HashMap;

use salvo_core::types::Ty;
use salvo_ir::{Decl, Expr, ExprKind, FnDecl, ImplDecl, InterfaceDecl, ReplyTarget, StructDecl};

use super::{kt_local, ModuleEmitter};
use crate::emit::{kotlin_package, kt_ident};

impl<'a, 'p> ModuleEmitter<'a, 'p> {
    /// [kt-wire] The codec expression of a type.
    pub(super) fn codec(&mut self, ty: &Ty) -> String {
        if salvo_core::literal::mentions_lit(ty) {
            return self.codec(&salvo_core::literal::collapse_ty(ty));
        }
        match ty {
            Ty::Qualified { quals, base } => {
                if quals.iter().any(|q| q.name == "Mut") {
                    if let Ty::Named { name, args } = base.as_ref() {
                        if name == "List" && args.len() == 1 {
                            let inner = self.codec(&args[0]);
                            return format!("salvo.MutListCodec({inner})");
                        }
                        if name == "Str" {
                            return "salvo.MutStrCodec".to_string();
                        }
                    }
                }
                self.codec(base)
            }
            Ty::Var(name) => format!("__c_{name}"),
            Ty::Array(elem) => format!("salvo.ListCodec({})", self.codec(elem)),
            Ty::Tuple(elems) => {
                let cs: Vec<String> = elems.iter().map(|e| self.codec(e)).collect();
                match cs.len() {
                    2 => format!("salvo.PairCodec({})", cs.join(", ")),
                    3 => format!("salvo.TripleCodec({})", cs.join(", ")),
                    n => {
                        self.error(format!("no wire codec for a {n}-tuple"));
                        "salvo.UnitCodec".to_string()
                    }
                }
            }
            Ty::Union(_) => {
                let arms: Vec<Ty> = ty.value_arms().into_iter().cloned().collect();
                let inner = match arms.len() {
                    0 => "salvo.UnitCodec".to_string(),
                    1 => self.codec(&arms[0]),
                    n => {
                        let cs: Vec<String> = arms.iter().map(|a| self.codec(a)).collect();
                        format!("salvo.Union{n}Codec({})", cs.join(", "))
                    }
                };
                if ty.has_none_arm() { format!("salvo.OptCodec({inner})") } else { inner }
            }
            Ty::Named { name, args } => match name.as_str() {
                "Bool" => "salvo.BoolCodec".into(),
                "Byte" => "salvo.ByteCodec".into(),
                "Int" => "salvo.IntCodec".into(),
                "Long" => "salvo.LongCodec".into(),
                "Float" => "salvo.FloatCodec".into(),
                "Double" => "salvo.DoubleCodec".into(),
                "Char" => "salvo.CharCodec".into(),
                "Str" => "salvo.StrCodec".into(),
                "Bytes" => "salvo.BytesCodec".into(),
                "None" => "salvo.UnitCodec".into(),
                "Addr" => "salvo.AddrCodec".into(),
                "Reply" => "salvo.ReplyCodec".into(),
                "List" if args.len() == 1 => format!("salvo.ListCodec({})", self.codec(&args[0])),
                _ => {
                    if let Some(alias) = self.s.symbols.type_aliases.get(name.as_str()).copied() {
                        let subst: HashMap<String, Ty> = alias.generics.iter().map(|g| g.name.clone()).zip(args.iter().cloned()).collect();
                        if let Some(def) = &alias.alias {
                            if let Some(t) = salvo_core::approx_ty(def, &subst) {
                                return self.codec(&t);
                            }
                        }
                    }
                    if self.s.symbols.structs.contains_key(name.as_str()) {
                        let codec = self.struct_codec_name(name);
                        // [effect-generic-decl] an erased struct's codec is an object.
                        if args.is_empty() || self.s.erased.is_erased(salvo_core::typekey::plain(name)) {
                            return codec;
                        }
                        let cs: Vec<String> = args.iter().map(|a| self.codec(a)).collect();
                        return format!("{codec}({})", cs.join(", "));
                    }
                    self.error(format!("no wire codec for `{name}`"));
                    "salvo.UnitCodec".to_string()
                }
            },
            _ => "salvo.UnitCodec".to_string(),
        }
    }

    fn struct_codec_name(&self, key: &str) -> String {
        let plain = salvo_core::typekey::plain(key).replace('.', "_");
        match self.s.symbols.key_modules.get(key) {
            Some(m) if **m != self.module.path => format!("{}.__Codec_{plain}", kotlin_package(m)),
            _ => format!("__Codec_{plain}"),
        }
    }

    pub(super) fn reply_decoder(&mut self, ty: &Ty) -> String {
        if salvo_core::wire_blocker(self.s.symbols, ty.strip_quals()).is_some() {
            return "{ _: ByteArray -> Pair(false, null) }".to_string();
        }
        let codec = self.codec(ty);
        format!("{{ __b: ByteArray -> salvo.salvoDecodeChecked(salvo.SalvoBytes(__b), {codec}) }}")
    }

    /// [kt-wire] `object __Codec_S : WireCodec<S>` for a struct with a wire form.
    pub(super) fn struct_codec(&mut self, s: &StructDecl) {
        let codec_name = format!("__Codec_{}", s.name.replace('.', "_"));
        let mut enc = String::new();
        let mut dec: Vec<String> = Vec::new();
        for f in &s.fields {
            let c = self.codec(&f.ty);
            enc.push_str(&format!("        {c}.enc(v.{}, out)\n", kt_ident(&f.name)));
            dec.push(format!("{c}.dec(inp)"));
        }
        let ctor = dec.join(", ");
        let name = &s.name;
        if self.decl_type_params(&s.name, &s.type_params).is_empty() {
            self.out.push_str(&format!(
                "\nobject {codec_name} : salvo.WireCodec<{name}> {{\n    override fun enc(v: {name}, out: salvo.WireOut) {{\n{enc}    }}\n    override fun dec(inp: salvo.WireIn): {name} = {name}({ctor})\n}}\n"
            ));
        } else {
            let params: Vec<String> = s.type_params.iter().map(|t| t.name.clone()).collect();
            let cps: Vec<String> = params.iter().map(|g| format!("private val __c_{g}: salvo.WireCodec<{g}>")).collect();
            let ty = format!("{name}<{}>", params.join(", "));
            self.out.push_str(&format!(
                "\nclass {codec_name}<{}>({}) : salvo.WireCodec<{ty}> {{\n    override fun enc(v: {ty}, out: salvo.WireOut) {{\n{enc}    }}\n    override fun dec(inp: salvo.WireIn): {ty} = {name}({ctor})\n}}\n",
                params.join(", "),
                cps.join(", ")
            ));
        }
    }

    /// Whether the effect behind an actor interface has a wire form.
    fn interface_has_wire(&self, i: &InterfaceDecl) -> bool {
        i.protocol_hash.is_some()
    }

    /// [kt-actor] The message classes, their codec, the protocol constant and
    /// the addr stub of an actor interface.
    pub(super) fn actor_interface(&mut self, i: &InterfaceDecl) {
        let sends: Vec<&salvo_ir::Member> = i.members.iter().filter(|m| m.send).collect();
        if sends.is_empty() {
            return;
        }
        if !self.decl_type_params(&i.name, &i.type_params).is_empty() {
            self.error(format!("actor effect `{}` is generic over values, which the IR emitter cannot make a protocol of yet", i.name));
            return;
        }
        let msg = format!("__Msg_{}", i.name);
        self.out.push_str(&format!("\nsealed class {msg} {{\n"));
        for m in &sends {
            let payload: Vec<String> = m.params.iter().map(|p| format!("val {}: {}", kt_local(&p.local), self.ty(&p.ty))).collect();
            self.out.push_str(&format!("    class {}({}) : {msg}()\n", variant(&m.emitted_name), payload.join(", ")));
        }
        self.out.push_str("}\n");
        if !self.interface_has_wire(i) {
            return;
        }
        let mut enc = String::new();
        let mut dec = String::new();
        for (tag, m) in sends.iter().enumerate() {
            let v = variant(&m.emitted_name);
            let mut encs = vec![format!("out.u8({tag})")];
            let mut decs = Vec::new();
            for p in &m.params {
                let c = self.codec(&p.ty);
                encs.push(format!("{c}.enc(v.{}, out)", kt_local(&p.local)));
                decs.push(format!("{c}.dec(inp)"));
            }
            enc.push_str(&format!("            is {msg}.{v} -> {{ {} }}\n", encs.join("; ")));
            dec.push_str(&format!("            {tag} -> {msg}.{v}({})\n", decs.join(", ")));
        }
        self.out.push_str(&format!(
            "\nobject __Codec_{msg} : salvo.WireCodec<{msg}> {{\n    override fun enc(v: {msg}, out: salvo.WireOut) {{\n        when (v) {{\n{enc}        }}\n    }}\n    override fun dec(inp: salvo.WireIn): {msg} = when (inp.u8()) {{\n{dec}        else -> throw salvo.WireError()\n    }}\n}}\n"
        ));
        if let Some(hash) = i.protocol_hash.clone() {
            self.out.push_str(&format!("\nconst val __PROTO_{}: String = \"{hash}\"\n", i.name));
        }
        // [actor-use-addr] the stub: the interface implemented by sending.
        self.out.push_str(&format!("\nclass __Stub_{0}(private val addr: Int) : {0} {{\n", i.name));
        for m in &sends {
            let ps = self.params(&m.params);
            let names: Vec<String> = m.params.iter().map(|p| kt_local(&p.local)).collect();
            self.out.push_str(&format!(
                "    override fun {}({ps}) {{\n        salvo.SalvoSched.sendWire(addr, {msg}.{}({}), __PROTO_{}, __Codec_{msg})\n    }}\n",
                kt_ident(&m.emitted_name),
                variant(&m.emitted_name),
                names.join(", "),
                i.name
            ));
        }
        self.out.push_str("}\n");
    }

    /// The actor interfaces an impl serves, in face order. Empty for a
    /// handler of plain effects; a handler mixing the two kinds is refused by
    /// the caller ([mixed-handler] is not rendered over the IR yet).
    pub(super) fn actor_faces(&self, h: &ImplDecl) -> Vec<&'p InterfaceDecl> {
        h.faces
            .iter()
            .filter_map(|f| match f.strip_quals() {
                Ty::Named { name, .. } => self.interface_by_name(&salvo_core::typekey::plain(name)),
                _ => None,
            })
            .filter(|i| i.actor)
            .collect()
    }

    pub(super) fn interface_by_name(&self, plain: &str) -> Option<&'p InterfaceDecl> {
        self.s.ir.modules.iter().flat_map(|m| &m.decls).find_map(|d| match d {
            Decl::Interface(i) if i.name == plain => Some(i),
            _ => None,
        })
    }

    /// [actor-private-send] The impl's `send fn`s no face declares.
    pub(super) fn private_sends<'h>(h: &'h ImplDecl, faces: &[&InterfaceDecl]) -> Vec<&'h FnDecl> {
        h.members.iter().filter(|m| m.send && !faces.iter().any(|f| f.members.iter().any(|fm| fm.name == m.name))).collect()
    }

    /// Whether the impl needs a private message class: an `init` or a
    /// private `send fn`.
    pub(super) fn has_private(h: &ImplDecl, faces: &[&InterfaceDecl]) -> bool {
        h.init.is_some() || !Self::private_sends(h, faces).is_empty()
    }

    /// [kt-actor] The fields an actor impl carries, inside its class body.
    pub(super) fn actor_fields(&mut self, h: &ImplDecl) -> String {
        let cap_expr = h.mailbox.as_ref().and_then(|m| match &m.kind {
            ExprKind::Construct { fields } => fields.iter().find(|(n, _)| n == "capacity").map(|(_, v)| v.clone()),
            _ => None,
        });
        let cap = match cap_expr {
            Some(e) => self.expr(&e, 1),
            None => "0".to_string(),
        };
        format!(
            "    val __mailboxCapacity: Int = {cap}\n    var __addr: Int? = null\n    val __parked: MutableMap<Long, __Cont_{}> = mutableMapOf()\n",
            h.name
        )
    }

    /// The declared (non-effect, non-implicit) parameters of a member.
    fn declared_params<'f>(m: &'f FnDecl) -> Vec<&'f salvo_ir::Param> {
        m.params.iter().skip(m.effect_params).take(m.params.len() - m.effect_params - m.implicit_params).collect()
    }

    /// [kt-actor] `__Cont_H`, `__Priv_H` and `__Actor_H` beside an actor impl.
    pub(super) fn actor_body(&mut self, h: &ImplDecl, faces: &[&'p InterfaceDecl]) {
        let privs = Self::private_sends(h, faces);
        let cont = format!("__Cont_{}", h.name);
        let priv_cls = format!("__Priv_{}", h.name);
        // Every send member, face or private, that parks: those with at
        // least one declared parameter (the answer is the last).
        let mut parking: Vec<&FnDecl> = Vec::new();
        for face in faces {
            for fm in face.members.iter().filter(|m| m.send) {
                if let Some(m) = h.members.iter().find(|m| m.name == fm.name) {
                    if !Self::declared_params(m).is_empty() {
                        parking.push(m);
                    }
                }
            }
        }
        for m in &privs {
            if !Self::declared_params(m).is_empty() {
                parking.push(m);
            }
        }
        self.out.push_str(&format!("\nsealed class {cont} {{\n"));
        for m in &parking {
            let declared = Self::declared_params(m);
            let caps: Vec<String> = declared.iter().take(declared.len() - 1).map(|p| format!("val {}: {}", kt_local(&p.local), self.ty(&p.ty))).collect();
            self.out.push_str(&format!("    class {}({}) : {cont}()\n", variant(&m.name), caps.join(", ")));
        }
        self.out.push_str("}\n");
        let has_priv = Self::has_private(h, faces);
        if has_priv {
            self.out.push_str(&format!("\nsealed class {priv_cls} {{\n"));
            if h.init.is_some() {
                self.out.push_str(&format!("    object Init : {priv_cls}()\n"));
            }
            for m in &privs {
                let ps: Vec<String> = Self::declared_params(m).iter().map(|p| format!("val {}: {}", kt_local(&p.local), self.ty(&p.ty))).collect();
                self.out.push_str(&format!("    class {}({}) : {priv_cls}()\n", variant(&m.name), ps.join(", ")));
            }
            self.out.push_str("}\n");
        }
        // Dispatch: one fn per face, one for the private messages.
        let mut dispatchers = String::new();
        let mut handle_arms = String::new();
        for face in faces {
            let msg = self.msg_class(face);
            let fname = format!("__dispatch{}", face.name.replace('.', "_"));
            let mut arms = String::new();
            for fm in face.members.iter().filter(|m| m.send) {
                let names: Vec<String> = fm.params.iter().map(|p| format!("m.{}", kt_local(&p.local))).collect();
                arms.push_str(&format!("            is {msg}.{} -> handler.{}({})\n", variant(&fm.emitted_name), kt_ident(&fm.emitted_name), names.join(", ")));
            }
            dispatchers.push_str(&format!("\n    private fun {fname}(m: {msg}) {{\n        when (m) {{\n{arms}        }}\n    }}\n"));
            handle_arms.push_str(&format!("            is {msg} -> {fname}(msg)\n"));
        }
        if has_priv {
            let mut arms = String::new();
            if h.init.is_some() {
                arms.push_str(&format!("            is {priv_cls}.Init -> handler.init()\n"));
            }
            for m in &privs {
                let names: Vec<String> = Self::declared_params(m).iter().map(|p| format!("m.{}", kt_local(&p.local))).collect();
                arms.push_str(&format!("            is {priv_cls}.{} -> handler.{}({})\n", variant(&m.name), kt_ident(&m.name), names.join(", ")));
            }
            dispatchers.push_str(&format!("\n    private fun __dispatchPriv(m: {priv_cls}) {{\n        when (m) {{\n{arms}        }}\n    }}\n"));
            handle_arms.push_str(&format!("            is {priv_cls} -> __dispatchPriv(msg)\n"));
        }
        let handle = if faces.len() == 1 && !has_priv {
            let msg = self.msg_class(faces[0]);
            format!("        __dispatch{}(msg as {msg})\n", faces[0].name.replace('.', "_"))
        } else {
            format!("        when (msg) {{\n{handle_arms}            else -> error(\"a message of one of this actor's protocols\")\n        }}\n")
        };
        let mut resume = String::new();
        let mut decode = String::new();
        for m in &parking {
            let v = variant(&m.name);
            let kt = self.impl_member_name_of(h, &m.name, Some(m));
            let declared = Self::declared_params(m);
            let mut res: Vec<String> = declared.iter().take(declared.len() - 1).map(|p| format!("c.{}", kt_local(&p.local))).collect();
            let last = declared[declared.len() - 1];
            let t = self.ty(&last.ty);
            res.push(format!("value as {t}"));
            let dec = self.reply_decoder(&last.ty);
            decode.push_str(&format!("            is {cont}.{v} -> ({dec})(payload)\n"));
            resume.push_str(&format!("            is {cont}.{v} -> handler.{kt}({})\n", res.join(", ")));
        }
        let mut proto_arms = String::new();
        for face in faces {
            if self.interface_has_wire(face) {
                let msg = self.msg_class(face);
                let proto = self.proto_const(face);
                proto_arms.push_str(&format!(
                    "                {proto} -> salvo.salvoDecode(salvo.SalvoBytes(payload), {})?.let {{ Pair(true, it) }} ?: Pair(false, null)\n",
                    self.msg_codec(face)
                ));
                let _ = msg;
            }
        }
        self.out.push_str(&format!(
            "\nclass __Actor_{h}(private val handler: {h}) : salvo.SalvoActor {{\n    \
             override fun handle(ctx: salvo.SalvoCtx, msg: Any?) {{\n        handler.__addr = ctx.addr\n{handle}    }}\n{dispatchers}\n    \
             override fun resume(ctx: salvo.SalvoCtx, slot: Long, value: Any?) {{\n        handler.__addr = ctx.addr\n        val c = handler.__parked.remove(slot) ?: return\n        when (c) {{\n{resume}{resume_else}        }}\n    }}\n\n    \
             @Suppress(\"REDUNDANT_ELSE_IN_WHEN\")\n    override fun decodeReply(slot: Long, payload: ByteArray): Pair<Boolean, Any?> {{\n        val c = handler.__parked[slot] ?: return Pair(false, null)\n        return when (c) {{\n{decode}            else -> Pair(false, null)\n        }}\n    }}\n\n    \
             companion object {{\n        val __DECODE: ((String, ByteArray) -> Pair<Boolean, Any?>)? = {{ proto, payload ->\n            when (proto) {{\n{proto_arms}                else -> Pair(false, null)\n            }}\n        }}\n    }}\n}}\n",
            h = h.name,
            resume_else = if parking.is_empty() { "            else -> {}\n" } else { "" }
        ));
    }

    /// [monitor-handler] [kt-monitor] `__Mon_E`: the effect implemented by
    /// locking a shared instance and delegating; emitted for every interface
    /// so a shared `spawn` of a plain-effect handler has its wrapper.
    pub(super) fn monitor_stub(&mut self, i: &InterfaceDecl) {
        let tps = self.decl_type_params(&i.name, &i.type_params);
        let targs = tps.clone();
        let name = &i.name;
        self.out.push_str(&format!(
            "\nclass __Mon_{name}{tps}(\n    private val inner: {name}{targs},\n    private val lock: java.util.concurrent.locks.ReentrantLock = java.util.concurrent.locks.ReentrantLock(),\n) : {name}{targs} {{\n"
        ));
        for m in &i.members {
            let mtps = self.type_params(&m.type_params);
            let ps = self.params(&m.params);
            let ret = self.ret_ty(&m.ret);
            let args: Vec<String> = m.params.iter().map(|p| kt_local(&p.local)).collect();
            let kt = kt_ident(&m.emitted_name);
            // [kt-monitor-reentry] re-entering the handle on its own thread
            // would deadlock on Rust, so it traps here [backend-parity].
            self.out.push_str(&format!(
                "    override fun {mtps}{kt}({ps}){ret} {{\n        check(!lock.isHeldByCurrentThread) {{ \"salvo: a handler's lock was entered again through its own handle, which on Rust would deadlock [monitor-handler]\" }}\n        lock.lock()\n        try {{ {}inner.{kt}({}) }} finally {{ lock.unlock() }}\n    }}\n",
                if ret.is_empty() { "" } else { "return " },
                args.join(", ")
            ));
        }
        self.out.push_str("}\n");
    }

    /// The package prefix of a declaration in another module.
    pub(super) fn pkg_prefix(&self, module: &salvo_core::ModulePath) -> String {
        if *module == self.module.path { String::new() } else { format!("{}.", kotlin_package(module)) }
    }

    /// `__Msg_E`, qualified from another module.
    pub(super) fn msg_class(&self, i: &InterfaceDecl) -> String {
        format!("{}__Msg_{}", self.pkg_prefix(&i.id.module), i.name)
    }

    pub(super) fn msg_codec(&self, i: &InterfaceDecl) -> String {
        format!("{}__Codec___Msg_{}", self.pkg_prefix(&i.id.module), i.name)
    }

    pub(super) fn proto_const(&self, i: &InterfaceDecl) -> String {
        format!("{}__PROTO_{}", self.pkg_prefix(&i.id.module), i.name)
    }

    // ------------------------------------------------- the primitives --

    /// [actor-self-send] A message to the enclosing actor: the face's class
    /// for a face member, `__Priv_H` for a private one; inline when the
    /// handler runs synchronously (no addr).
    pub(super) fn self_send(&mut self, member: &str, args: &[Expr], indent: usize) -> String {
        let a = self.exprs(args, indent).join(", ");
        let Some(hname) = self.current_impl.clone() else {
            self.error("`@self` send outside an actor");
            return "TODO()".to_string();
        };
        let Some(h) = self.s.ir.modules.iter().flat_map(|m| &m.decls).find_map(|d| match d {
            Decl::Impl(h) if h.name == hname => Some(h),
            _ => None,
        }) else {
            self.error(format!("internal: no impl `{hname}` for a self-send"));
            return "TODO()".to_string();
        };
        let faces = self.actor_faces(h);
        let kt = self.impl_member_name(h, member);
        if let Some(face) = faces.iter().find(|f| f.members.iter().any(|m| m.name == member)) {
            let fm = face.members.iter().find(|m| m.name == member).unwrap();
            let msg = self.msg_class(face);
            return format!(
                "run {{ val __a = __addr; if (__a != null) salvo.SalvoSched.send(__a, {msg}.{}({a})) else this.{kt}({a}) }}",
                variant(&fm.emitted_name)
            );
        }
        let built = if a.is_empty() { format!("__Priv_{hname}.{}", variant(member)) } else { format!("__Priv_{hname}.{}({a})", variant(member)) };
        if self.in_facade {
            // [kt-mixed] the façade's addr is the servant's, always set.
            return format!("salvo.SalvoSched.send(__addr, {built})");
        }
        format!("run {{ val __a = __addr; if (__a != null) salvo.SalvoSched.send(__a, {built}) else this.{kt}({a}) }}")
    }

    /// [kt-actor] `spawn H(…) [on p] [in g]`: build the instance, spawn the
    /// actor around it, send `Init` first [handler-init], join the group,
    /// answer one addr per face [effect-handler-multi].
    pub(super) fn spawn(&mut self, e: &Expr, handler: &Expr, pool: Option<&Expr>, join: Option<&Expr>, indent: usize) -> String {
        let Ty::Named { name, .. } = handler.ty.strip_quals() else {
            self.error("`spawn` of something that is not a handler");
            return "TODO()".to_string();
        };
        let plain = salvo_core::typekey::plain(name).to_string();
        let Some(decl) = self.s.ir.modules.iter().flat_map(|m| &m.decls).find_map(|d| match d {
            Decl::Impl(h) if h.name == plain => Some(h),
            _ => None,
        }) else {
            self.error(format!("`spawn` of an unknown handler `{plain}`"));
            return "TODO()".to_string();
        };
        let faces = self.actor_faces(decl);
        if faces.is_empty() && self.is_mixed(decl) {
            // [mixed-handler] [kt-mixed] the mixed spawn: constructor
            // arguments evaluated once, shared by servant and façade.
            let ExprKind::Construct { fields } = &handler.kind else {
                self.error("`spawn` of a mixed handler needs its construction");
                return "TODO()".to_string();
            };
            let mut lets = String::new();
            let mut all: Vec<String> = Vec::new();
            let mut fac: Vec<String> = vec!["__a".to_string()];
            for (i, (fname, v)) in fields.iter().enumerate() {
                let code = self.expr(v, indent);
                lets.push_str(&format!("val __c{i} = {code}; "));
                all.push(format!("__c{i}"));
                if !fname.starts_with("__dep") {
                    fac.push(format!("__c{i}"));
                }
            }
            let pool_code = match pool {
                Some(p) => self.expr(p, indent),
                None => "salvo.SalvoSched.currentPool()".to_string(),
            };
            let ctor = self.construct_ty(&handler.ty);
            let actor = self.handler_path(name, &format!("__Actor_{plain}"));
            let facade = self.handler_path(name, &format!("__Fac_{plain}"));
            let init = if decl.init.is_some() { format!("salvo.SalvoSched.send(__a, {}.Init); ", self.handler_path(name, &format!("__Priv_{plain}"))) } else { String::new() };
            return format!(
                "run {{ {lets}val __h = {ctor}({}); val __a = salvo.SalvoSched.spawn({pool_code}, __h.__mailboxCapacity, {actor}(__h), {actor}.__DECODE); {init}{facade}({}) }}",
                all.join(", "),
                fac.join(", ")
            );
        }
        if faces.is_empty() {
            // [monitor-handler] a plain-effect spawn: the instance shared
            // behind the effect's monitor.
            if decl.faces.len() != 1 {
                self.error(format!("handler `{plain}` implements several plain effects, and a shared instance behind several faces is not supported yet"));
                return "TODO()".to_string();
            }
            let h = self.expr(handler, indent);
            let face = self.ty(&decl.faces[0]);
            return format!("{}({h})", self.monitor_for(&face));
        }
        if faces.len() != decl.faces.len() {
            self.error(format!("handler `{plain}` mixes actor and plain effects, which the IR emitter does not render yet"));
            return "TODO()".to_string();
        }
        let h = self.expr(handler, indent);
        let pool_code = match pool {
            Some(p) => self.expr(p, indent),
            None => "salvo.SalvoSched.currentPool()".to_string(),
        };
        let actor = self.handler_path(name, &format!("__Actor_{plain}"));
        let init = if decl.init.is_some() {
            let priv_cls = self.handler_path(name, &format!("__Priv_{plain}"));
            format!("salvo.SalvoSched.send(__a, {priv_cls}.Init); ")
        } else {
            String::new()
        };
        let joined = match join {
            Some(g) => {
                let Some(group) = self.interface_by_name("ActorGroup") else {
                    self.error("`spawn … in group` needs std `net`");
                    return "TODO()".to_string();
                };
                let Some(idx) = group.members.iter().position(|m| m.name == "join") else {
                    self.error("`ActorGroup` has no `join` member");
                    return "TODO()".to_string();
                };
                let g_code = self.expr(g, indent);
                format!(
                    "salvo.SalvoSched.sendWire({g_code}, {}.{}(__a), {}, {}); ",
                    self.msg_class(group),
                    variant(&group.members[idx].emitted_name),
                    self.proto_const(group),
                    self.msg_codec(group)
                )
            }
            None => String::new(),
        };
        let result = match faces.len() {
            1 => "__a".to_string(),
            2 => "Pair(__a, __a)".to_string(),
            3 => "Triple(__a, __a, __a)".to_string(),
            n => {
                self.error(format!("handler `{plain}` implements {n} effects, and the kotlin backend renders a spawn's addr tuple as `Pair`/`Triple` — at most three faces"));
                return "TODO()".to_string();
            }
        };
        let _ = e;
        format!("run {{ val __h = {h}; val __a = salvo.SalvoSched.spawn({pool_code}, __h.__mailboxCapacity, {actor}(__h), {actor}.__DECODE); {init}{joined}{result} }}")
    }

    fn handler_path(&self, key: &str, class: &str) -> String {
        match self.s.symbols.key_modules.get(key) {
            Some(m) if **m != self.module.path => format!("{}.{class}", kotlin_package(m)),
            _ => class.to_string(),
        }
    }

    /// [kt-monitor] `__Mon_E` for a rendered effect type `E<…>`.
    pub(super) fn monitor_for(&self, rendered: &str) -> String {
        match rendered.split_once('<') {
            Some((base, rest)) => match base.rsplit_once('.') {
                Some((pkg, n)) => format!("{pkg}.__Mon_{n}<{rest}"),
                None => format!("__Mon_{base}<{rest}"),
            },
            None => match rendered.rsplit_once('.') {
                Some((pkg, n)) => format!("{pkg}.__Mon_{n}"),
                None => format!("__Mon_{rendered}"),
            },
        }
    }

    pub(super) fn send(&mut self, addr: &Expr, member: &salvo_ir::MemberRef, args: &[Expr], indent: usize) -> String {
        let a = self.expr(addr, indent);
        let iface = self.s.ir.modules.iter().flat_map(|m| &m.decls).find_map(|d| match d {
            Decl::Interface(i) if i.id == member.interface => Some(i),
            _ => None,
        });
        let Some(iface) = iface else {
            self.error("send to an unknown interface");
            return "TODO()".to_string();
        };
        let m = &iface.members[member.index];
        let args_code = self.exprs(args, indent);
        let pkg = if iface.id.module == self.module.path { String::new() } else { format!("{}.", kotlin_package(&iface.id.module)) };
        format!(
            "salvo.SalvoSched.sendWire({a}, {pkg}__Msg_{0}.{1}({2}), {pkg}__PROTO_{0}, {pkg}__Codec___Msg_{0})",
            iface.name,
            variant(&m.emitted_name),
            args_code.join(", ")
        )
    }

    pub(super) fn replyto(&mut self, target: &ReplyTarget, captures: &[Expr], gated: bool, pool: Option<&Expr>, indent: usize) -> String {
        match target {
            ReplyTarget::Member(member) => {
                let Some(h) = self.current_impl.clone() else {
                    self.error("`replyto` outside a handler");
                    return "TODO()".to_string();
                };
                let caps = self.exprs(captures, indent);
                let mint = if gated { "mintGated" } else { "mint" };
                format!(
                    "run {{ val (__r, __s) = salvo.SalvoSched.{mint}(__addr!!); __parked[__s] = __Cont_{h}.{}({}); __r }}",
                    variant(member),
                    caps.join(", ")
                )
            }
            ReplyTarget::Task { target, effects } => {
                let Some(decl) = self.s.ir.modules.iter().flat_map(|m| &m.decls).find_map(|d| match d {
                    Decl::Fn(f) if &f.id == target => Some(f),
                    _ => None,
                }) else {
                    self.error("`replyto` of an unknown task");
                    return "TODO()".to_string();
                };
                let declared: Vec<&salvo_ir::Param> = decl.params.iter().skip(decl.effect_params).collect();
                let Some(last) = declared.last() else {
                    self.error("a task target has no answer parameter");
                    return "TODO()".to_string();
                };
                let payload = self.ty(&last.ty);
                let decoder = self.reply_decoder(&last.ty);
                let name = self.fn_name(target);
                let mut lets = String::new();
                let mut args = Vec::new();
                for (i, e) in effects.iter().enumerate() {
                    let code = self.expr(e, indent);
                    lets.push_str(&format!("val __e{i} = {code}; "));
                    args.push(format!("__e{i}"));
                }
                for (i, c) in captures.iter().enumerate() {
                    let code = self.expr(c, indent);
                    lets.push_str(&format!("val __c{i} = {code}; "));
                    args.push(format!("__c{i}"));
                }
                args.push(format!("__v as {payload}"));
                let pool_code = match pool {
                    Some(p) => self.expr(p, indent),
                    None => "salvo.SalvoSched.currentPool()".to_string(),
                };
                format!("run {{ {lets}salvo.SalvoSched.mintTask({pool_code}, {decoder}) {{ __v -> {name}({}) }} }}", args.join(", "))
            }
        }
    }

    pub(super) fn waitfor(&mut self, local: &salvo_ir::Local, token_ty: &Ty, body: &salvo_ir::Block, indent: usize) -> String {
        let pad = "    ".repeat(indent + 1);
        let close = "    ".repeat(indent);
        let payload_ty = match token_ty.strip_quals() {
            Ty::Named { name, args } if name == "Reply" && args.len() == 1 => args[0].clone(),
            _ => {
                self.error("`waitfor` binds a `Reply<T>`");
                return "TODO()".to_string();
            }
        };
        let payload = self.ty(&payload_ty);
        let decoder = self.reply_decoder(&payload_ty);
        let saved = std::mem::take(&mut self.out);
        self.block_stmts(body, indent + 1);
        let stmts = std::mem::replace(&mut self.out, saved);
        format!(
            "run {{\n{pad}val ({}, __wid) = salvo.SalvoSched.waiter()\n{pad}salvo.SalvoSched.waiterDecoder(__wid, {decoder})\n{stmts}{pad}salvo.SalvoSched.awaitReply(__wid) as {payload}\n{close}}}",
            kt_local(local)
        )
    }

    /// [protocol-hash] `setProtocols` at the top of `main`.
    pub(super) fn protocol_prelude(&mut self) -> String {
        let mut pairs: Vec<String> = Vec::new();
        let mut ifaces: Vec<&InterfaceDecl> = self.s.ir.modules.iter().flat_map(|m| &m.decls).filter_map(|d| match d {
            Decl::Interface(i) if i.actor && i.protocol_hash.is_some() => Some(i),
            _ => None,
        }).collect();
        ifaces.sort_by(|a, b| a.name.cmp(&b.name));
        for iface in ifaces {
            let name = &iface.name;
            pairs.push(format!("Pair(\"{name}\", {}.__PROTO_{name})", kotlin_package(&iface.id.module)));
        }
        if pairs.is_empty() {
            return String::new();
        }
        format!("    salvo.SalvoSched.setProtocols(listOf({}))\n", pairs.join(", "))
    }
}

/// A message or continuation class: the member's name, capitalized.
pub(super) fn variant(member: &str) -> String {
    salvo_backend::emit_util::msg_variant_name(member)
}
