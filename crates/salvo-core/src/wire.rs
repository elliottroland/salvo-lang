//! [wire-format] [noremote] [protocol-hash] What may cross a machine, and the
//! two compile-time facts a wire needs: whether a type has a wire form, and
//! the canonical hash of an actor protocol.
//!
//! Step ② of the network sequence (user decisions 2026-09-26; ROADMAP.md
//! section 2). **Serializable by default, `noremote` the opt-out**: a type has
//! a wire form unless it is, or transitively holds, something process-local —
//! a `noremote` declaration, a function value, a `proj` view. The predicate is
//! *shared* between the checker (which refuses `encode`/`decode` of a blocked
//! type at the call, and — from step ⑤ — `attach<E>` of a protocol with a
//! blocked payload) and both emitters (which generate a codec for exactly the
//! structs that pass), so the two can never disagree about which types have a
//! form [backend-never-wrong].
//!
//! The encoding itself is fixed here in prose and implemented twice, in each
//! backend's `wire` runtime, byte for byte — that is what lets a Kotlin node
//! and a Rust node share a group:
//!
//! * `Bool` one byte (0/1); `Byte` one byte; `Int` four bytes big-endian
//!   two's complement; `Long` eight; `Float` four (IEEE-754 bits); `Double`
//!   eight; `Char` four (the code point); `Str` a `u32` byte length then
//!   UTF-8; `Bytes` a `u32` length then the bytes; `None` nothing.
//! * An optional (`T?`, or a union with a `None` arm) is one byte — 0 absent,
//!   1 present — then the payload.
//! * A union of *n* non-`None` arms is one byte holding the arm's index over
//!   the **declared** arms in declaration order (the same positional
//!   identity the checker and emitters already share), then the payload.
//! * A struct is its fields in declaration order; a tuple its components;
//!   qualifiers erase [qual-erasure].
//! * A `List`/array is a `u32` count then the elements.
//! * Actor messages (`__Msg_E`) are a union of the effect's `send fn`
//!   members in declaration order, each variant its parameters in order.

use std::collections::HashMap;

use salvo_syntax::ast::{EffectDecl, Type};

use crate::program::Symbols;
use crate::types::{Qual, Ty};

/// Why a type has no wire form. `Later` is step ②'s own cut: the type *will*
/// have one, but not until the step that gives it a routable identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WireBlock {
    /// A declaration marked `noremote`, or one of the process-local
    /// intrinsics — named, so the diagnostic can point at it.
    NoRemote(String),
    /// A function value: shared, not owned, and no representation.
    FnValue,
    /// A `proj` view: a borrow of a value the sender still owns.
    Proj,
    /// A keyed container (`Set`, `Map` and their sorted forms): its identity
    /// capabilities (`hash`/`eq`, or `cmp`) are resolved by the checker per
    /// type and would have to be threaded into the decoder. Recorded cut.
    Keyed(String),
    /// A generic parameter: the form is the instantiation's, and `encode`
    /// inside a generic body has nothing to dispatch on yet.
    Generic(String),
}

impl WireBlock {
    /// The diagnostic's account of the block, phrased so the remedy reads off
    /// it.
    pub fn describe(&self) -> String {
        match self {
            WireBlock::NoRemote(name) => {
                format!("`{name}` is declared `noremote`, so it has no wire form")
            }
            WireBlock::FnValue => {
                "it holds a function value, which is shared rather than owned and has no \
                 wire form"
                    .to_string()
            }
            WireBlock::Proj => {
                "it holds a `proj` view, which borrows the sender's value".to_string()
            }
            WireBlock::Keyed(name) => format!(
                "`{name}` is a keyed container, whose identity capabilities are not yet \
                 carried on the wire — send a `List` of its entries"
            ),
            WireBlock::Generic(name) => format!(
                "`{name}` is a generic parameter, and a wire form is the instantiation's \
                 — encode at a concrete type"
            ),
        }
    }
}

/// [noremote] Whether `ty` is, or transitively holds, something without a
/// wire form — and which thing, for the diagnostic. `None` means the type
/// has a codec on both backends.
pub fn wire_blocker(symbols: &Symbols<'_>, ty: &Ty) -> Option<WireBlock> {
    wire_blocker_at(symbols, ty, 0)
}

fn wire_blocker_at(symbols: &Symbols<'_>, ty: &Ty, depth: usize) -> Option<WireBlock> {
    if depth > 8 {
        return None;
    }
    match ty {
        Ty::Fn { .. } => Some(WireBlock::FnValue),
        Ty::Var(name) => Some(WireBlock::Generic(name.clone())),
        Ty::Qualified { quals, base } => {
            if quals.iter().any(|q: &Qual| q.name == "proj") {
                return Some(WireBlock::Proj);
            }
            wire_blocker_at(symbols, base, depth + 1)
        }
        Ty::Union(arms) | Ty::Tuple(arms) => arms
            .iter()
            .find_map(|a| wire_blocker_at(symbols, a, depth + 1)),
        Ty::Array(elem) => wire_blocker_at(symbols, elem, depth + 1),
        Ty::Named { name, args } => {
            match name.as_str() {
                "Bool" | "Byte" | "Int" | "Long" | "Float" | "Double" | "Char" | "Str"
                | "Bytes" | "None" => return None,
                "List" => return args.iter().find_map(|a| wire_blocker_at(symbols, a, depth + 1)),
                "Set" | "Map" | "SortedSet" | "SortedMap" => {
                    return Some(WireBlock::Keyed(name.clone()))
                }
                // [addr-routable] An addr crosses as its routable identity —
                // when the protocol behind it has a wire form, since a proxy
                // is only good for sends that can be framed; a reply token
                // crosses when its answer can.
                "Addr" if args.len() == 1 => {
                    let Ty::Named { name: effect, .. } = args[0].strip_quals() else {
                        return None;
                    };
                    let Some(e) = symbols.effects.get(effect.as_str()) else {
                        return None;
                    };
                    let empty = HashMap::new();
                    for f in e.fns.iter().filter(|f| f.is_send) {
                        for p in f.params.iter().filter(|p| !p.implicit) {
                            if let Some(t) = approx_ty(&p.ty, &empty) {
                                if let Some(b) = wire_blocker_at(symbols, &t, depth + 1) {
                                    return Some(match b {
                                        WireBlock::NoRemote(inner) => WireBlock::NoRemote(format!(
                                            "Addr<{effect}> ({effect}.{} takes {inner})",
                                            f.name.name
                                        )),
                                        other => other,
                                    });
                                }
                            }
                        }
                    }
                    return None;
                }
                "Reply" => return args.iter().find_map(|a| wire_blocker_at(symbols, a, depth + 1)),
                _ => {}
            }
            if let Some(decl) = symbols.intrinsic_types.get(name.as_str()) {
                if decl.noremote {
                    return Some(WireBlock::NoRemote(name.clone()));
                }
                // An intrinsic type without a codec in the runtimes is a
                // gap the emitters report by name; the predicate trusts the
                // declaration.
                return args.iter().find_map(|a| wire_blocker_at(symbols, a, depth + 1));
            }
            if let Some(alias) = symbols.type_aliases.get(name.as_str()) {
                // [comptime-fields] A compile-time type has no runtime value,
                // let alone a wire form.
                if alias.noremote || alias.comptime {
                    return Some(WireBlock::NoRemote(name.clone()));
                }
                let subst: HashMap<String, Ty> = alias
                    .generics
                    .iter()
                    .map(|g| g.name.clone())
                    .zip(args.iter().cloned())
                    .collect();
                if let Some(def) = &alias.alias {
                    if let Some(lowered) = approx_ty(def, &subst) {
                        return wire_blocker_at(symbols, &lowered, depth + 1);
                    }
                }
                return None;
            }
            if let Some(a) = args.iter().find_map(|a| wire_blocker_at(symbols, a, depth + 1)) {
                return Some(a);
            }
            let Some(decl) = symbols.structs.get(name.as_str()) else {
                return None;
            };
            if decl.noremote || decl.comptime {
                return Some(WireBlock::NoRemote(name.clone()));
            }
            let subst: HashMap<String, Ty> = decl
                .generics
                .iter()
                .map(|g| g.name.clone())
                .zip(args.iter().cloned())
                .collect();
            for field in &decl.fields {
                let Some(field_ty) = approx_ty(&field.ty, &subst) else {
                    continue;
                };
                // A generic field left unsubstituted (the struct checked at
                // its declaration rather than at an instantiation) is the
                // instantiation's business, not a block.
                if let Ty::Var(_) = field_ty {
                    continue;
                }
                if let Some(b) = wire_blocker_at(symbols, &field_ty, depth + 1) {
                    return Some(match b {
                        WireBlock::NoRemote(inner) => {
                            WireBlock::NoRemote(format!("{name}.{} ({inner})", field.name.name))
                        }
                        other => other,
                    });
                }
            }
            None
        }
        _ => None,
    }
}

/// Whether a struct **declaration** has a wire form, judged with its generic
/// parameters left free: what the emitters ask before generating a codec for
/// it (a generic struct's codec is conditional on its arguments).
pub fn struct_has_wire_form(symbols: &Symbols<'_>, name: &str) -> bool {
    let Some(decl) = symbols.structs.get(name) else {
        return false;
    };
    if decl.noremote {
        return false;
    }
    let ty = Ty::Named {
        name: name.to_string(),
        args: decl.generics.iter().map(|g| Ty::Var(g.name.clone())).collect(),
    };
    wire_blocker(symbols, &ty).is_none()
}

/// A written type lowered structurally, with a struct's generic parameters
/// substituted from `subst`. The checker's `lower_type` is the real thing;
/// this is enough for a predicate over declared shapes (and was the Kotlin
/// emitter's private helper until the wire needed it in three places).
pub fn approx_ty(t: &Type, subst: &HashMap<String, Ty>) -> Option<Ty> {
    match t {
        Type::Named { qualifiers, base } => {
            if qualifiers.is_empty() && base.args.is_empty() {
                if let Some(ty) = subst.get(&base.name.name) {
                    return Some(ty.clone());
                }
            }
            let args: Option<Vec<Ty>> = base.args.iter().map(|a| approx_ty(a, subst)).collect();
            let named = Ty::Named {
                name: base.name.name.clone(),
                args: args?,
            };
            let quals: Vec<Qual> = qualifiers
                .iter()
                .map(|q| Qual {
                    effect: false,
                    name: q.name.name.clone(),
                    args: Vec::new(),
                })
                .collect();
            Some(named.qualify(quals))
        }
        Type::QualifiedGroup { qualifiers, base, .. } => {
            let inner = approx_ty(base, subst)?;
            let quals: Vec<Qual> = qualifiers
                .iter()
                .map(|q| Qual {
                    effect: false,
                    name: q.name.name.clone(),
                    args: Vec::new(),
                })
                .collect();
            Some(inner.qualify(quals))
        }
        Type::Union { arms, .. } => {
            let arms: Option<Vec<Ty>> = arms.iter().map(|a| approx_ty(a, subst)).collect();
            Some(Ty::Union(arms?))
        }
        Type::Tuple { elems, .. } => {
            let elems: Option<Vec<Ty>> = elems.iter().map(|e| approx_ty(e, subst)).collect();
            Some(Ty::Tuple(elems?))
        }
        Type::Array { elem, .. } => Some(Ty::Array(Box::new(approx_ty(elem, subst)?))),
        Type::Nullable { inner, .. } => {
            Some(Ty::Union(vec![approx_ty(inner, subst)?, Ty::none()]))
        }
        Type::Fn { .. } => Some(Ty::Fn {
            contract: None,
            effects: Vec::new(),
            params: Vec::new(),
            ret: Box::new(Ty::Unknown),
        }),
    }
}

// ------------------------------------------------------------- protocol ----

/// [protocol-hash] The canonical form of an actor protocol: every `send fn`
/// member in declaration order, each as its name and its parameters' types
/// in order, with every struct **expanded to its fields** (so a renamed
/// struct is the same protocol and a reordered field is not — the encoding
/// is positional), unions as their declared arms, qualifiers erased. Two
/// nodes may talk on `E` exactly when their canonical forms agree; the hash
/// is exchanged in the handshake and compared at `attach`/`join` (steps
/// ④/⑤), never at decode.
/// [protocol-hash] Whether every payload of an actor effect's `send fn`
/// members has a wire form — the predicate under "this protocol has a hash",
/// shared by both emitters and the lock file [protocol-lock].
pub fn effect_has_wire_form(symbols: &Symbols<'_>, e: &EffectDecl) -> bool {
    let empty = HashMap::new();
    e.fns.iter().filter(|f| f.is_send).all(|f| {
        f.params
            .iter()
            .filter(|p| !p.implicit)
            .all(|p| approx_ty(&p.ty, &empty).is_some_and(|t| wire_blocker(symbols, &t).is_none()))
    })
}

pub fn protocol_canonical(symbols: &Symbols<'_>, effect: &EffectDecl) -> String {
    let mut out = String::new();
    for f in &effect.fns {
        if !f.is_send {
            continue;
        }
        out.push_str(&f.name.name);
        out.push('(');
        let mut first = true;
        for p in &f.params {
            if p.implicit {
                continue;
            }
            if !first {
                out.push(',');
            }
            first = false;
            match approx_ty(&p.ty, &HashMap::new()) {
                Some(ty) => canonical_ty(symbols, &ty, &mut out, 0),
                None => out.push('?'),
            }
        }
        out.push_str(");");
    }
    out
}

fn canonical_ty(symbols: &Symbols<'_>, ty: &Ty, out: &mut String, depth: usize) {
    if depth > 8 {
        out.push_str("...");
        return;
    }
    match ty {
        Ty::Qualified { base, .. } => canonical_ty(symbols, base, out, depth),
        Ty::Union(arms) => {
            out.push('(');
            for (i, a) in arms.iter().enumerate() {
                if i > 0 {
                    out.push('|');
                }
                canonical_ty(symbols, a, out, depth + 1);
            }
            out.push(')');
        }
        Ty::Tuple(elems) => {
            out.push('<');
            for (i, e) in elems.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                canonical_ty(symbols, e, out, depth + 1);
            }
            out.push('>');
        }
        Ty::Array(elem) => {
            out.push('[');
            canonical_ty(symbols, elem, out, depth + 1);
            out.push(']');
        }
        Ty::Fn { .. } => out.push_str("fn"),
        Ty::Var(v) => out.push_str(v),
        Ty::Named { name, args } => {
            if let Some(decl) = symbols.structs.get(name.as_str()) {
                let subst: HashMap<String, Ty> = decl
                    .generics
                    .iter()
                    .map(|g| g.name.clone())
                    .zip(args.iter().cloned())
                    .collect();
                out.push('{');
                for (i, field) in decl.fields.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    match approx_ty(&field.ty, &subst) {
                        Some(t) => canonical_ty(symbols, &t, out, depth + 1),
                        None => out.push('?'),
                    }
                }
                out.push('}');
                return;
            }
            if let Some(alias) = symbols.type_aliases.get(name.as_str()) {
                let subst: HashMap<String, Ty> = alias
                    .generics
                    .iter()
                    .map(|g| g.name.clone())
                    .zip(args.iter().cloned())
                    .collect();
                if let Some(def) = &alias.alias {
                    if let Some(t) = approx_ty(def, &subst) {
                        canonical_ty(symbols, &t, out, depth + 1);
                        return;
                    }
                }
            }
            out.push_str(name);
            if !args.is_empty() {
                out.push('<');
                for (i, a) in args.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    canonical_ty(symbols, a, out, depth + 1);
                }
                out.push('>');
            }
        }
        other => out.push_str(&format!("{other}")),
    }
}

/// [protocol-hash] FNV-1a 64 over the canonical form, rendered as sixteen
/// lowercase hex digits. Computed by the compiler, so the two backends carry
/// the same constant without either runtime hashing anything.
pub fn protocol_hash(canonical: &str) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in canonical.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{h:016x}")
}
