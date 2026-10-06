//! Copy plans [copy-plan]: how `copy(x)` duplicates a value of a type, as a
//! tree a backend renders.
//!
//! A backend whose values share structure (a garbage-collected one) needs
//! to know, per type, which parts a Salvo operation can still mutate: those
//! are copied, the rest are shared. That is a fact about Salvo values, not
//! about the target, so core decides it and a backend only spells it. A
//! backend whose copy is always deep (Rust's `clone`) ignores the plan.

use std::collections::HashMap;

use crate::program::Symbols;
use crate::types::{Qual, Ty};

/// How one type's values are copied.
#[derive(Clone, Debug, PartialEq)]
pub enum CopyPlan {
    /// No Salvo operation can mutate any part of the value: the copy is the
    /// value itself.
    Identity,
    /// A `Mut Str`: a new buffer over the same characters.
    StrBuilder,
    /// A value platform type with a `Mut` kind: the host's own `copy`.
    Platform(String),
    /// A list or deque. `elem` is how one element copies; `Identity` leaves
    /// the container copy shallow.
    Elements { deque: bool, mutable: bool, elem: Box<CopyPlan> },
    /// A struct, deeply: a copy of the struct with each listed field (by
    /// Salvo name) replaced by its own copy. Fields that copy as themselves
    /// are not listed.
    Struct { name: String, fields: Vec<(String, CopyPlan)> },
    /// An array whose elements are immutable.
    Array,
    /// A non-generic struct reached again inside its own copy (a list of
    /// itself): the copy calls the struct's own copy fn, which a backend
    /// generates from `copy_plan` of the struct. `mutable` is whether this
    /// value is `Mut`.
    Recur { name: String, mutable: bool },
}

/// The plan for a value of type `ty`, or `None` where no backend can copy
/// the shape correctly (a shallow copy would alias mutable parts): a generic
/// struct reached again inside its own copy, a mutable array of mutable
/// elements.
pub fn copy_plan(symbols: &Symbols<'_>, ty: &Ty) -> Option<CopyPlan> {
    plan(symbols, ty, &mut Vec::new())
}

fn plan(symbols: &Symbols<'_>, ty: &Ty, copying: &mut Vec<String>) -> Option<CopyPlan> {
    if immutable(symbols, ty, &mut Vec::new()) {
        return Some(CopyPlan::Identity);
    }
    let has_mut = ty.quals().iter().any(|q| q.name == "Mut");
    match ty.strip_quals() {
        Ty::Named { name, .. } if has_mut && name == "Str" => Some(CopyPlan::StrBuilder),
        // [platform-value-type] A value platform type copies through its
        // host's `copy`, plain or `Mut`: a host may use one class for both.
        // Element-free ones only; a collection also copies mutable elements.
        Ty::Named { name, args } if args.is_empty() && mut_platform_value(symbols, name) => {
            Some(CopyPlan::Platform(name.clone()))
        }
        // [col-deque] One class serves `Deque` and `Mut Deque`, so even a
        // plain deque may be an object someone else mutates.
        Ty::Named { name, args } if (name == "Deque" || name == "List") && args.len() == 1 => {
            let elem = plan(symbols, &args[0], copying)?;
            Some(CopyPlan::Elements { deque: name == "Deque", mutable: has_mut, elem: Box::new(elem) })
        }
        Ty::Named { name, args } if symbols.structs.contains_key(name.as_str()) => {
            let s = symbols.structs[name.as_str()];
            if s.fields.is_empty() {
                return Some(CopyPlan::Identity);
            }
            // A struct reached again inside its own copy would need a
            // recursive copy fn: refused, not looped on.
            // The entry keeps the outer struct's own (keyed) name: the written
            // field type names it plainly, and a plain name may be another
            // module's struct.
            if let Some(outer) = copying.iter().find(|n| crate::typekey::plain(n) == crate::typekey::plain(name)) {
                return s.generics.is_empty().then(|| CopyPlan::Recur { name: outer.clone(), mutable: has_mut });
            }
            copying.push(name.clone());
            let out = struct_plan(symbols, s, args, has_mut, copying);
            copying.pop();
            out
        }
        Ty::Array(elem) if immutable(symbols, elem, &mut Vec::new()) => Some(CopyPlan::Array),
        _ => None,
    }
}

fn struct_plan(
    symbols: &Symbols<'_>,
    s: &salvo_syntax::ast::StructDecl,
    targs: &[Ty],
    has_mut: bool,
    copying: &mut Vec<String>,
) -> Option<CopyPlan> {
    let name = s.name.name.clone();
    let subst: HashMap<String, Ty> =
        s.generics.iter().map(|g| g.name.clone()).zip(targs.iter().cloned()).collect();
    let mut fields = Vec::new();
    for field in &s.fields {
        let fty = crate::wire::approx_ty(&field.ty, &subst)?;
        // A field is typed as the value has it: a `canbe Mut` field is `Mut`
        // in a `Mut` struct [field-canbe-mut].
        let fty = if field.canbe_mut && has_mut { fty.qualify(vec![Qual::plain("Mut", Vec::new())]) } else { fty };
        if immutable(symbols, &fty, &mut vec![name.clone()]) {
            continue;
        }
        fields.push((field.name.name.clone(), plan(symbols, &fty, copying)?));
    }
    Some(CopyPlan::Struct { name, fields })
}

fn mut_platform_value(symbols: &Symbols<'_>, name: &str) -> bool {
    symbols
        .intrinsic_types
        .get(name)
        .is_some_and(|t| t.platform && !t.linear && t.auto_qualifiers.iter().any(|q| q.name.name == "Mut"))
}

/// Whether no Salvo operation can mutate any part of a value of this type —
/// the condition under which sharing is a correct `copy`. Conservative:
/// anything unknown is mutable. `visiting` breaks cycles (a cycle through
/// immutable spines stays immutable).
pub fn immutable(symbols: &Symbols<'_>, ty: &Ty, visiting: &mut Vec<String>) -> bool {
    match ty {
        Ty::ValueRef { .. } | Ty::ConstInt(_) | Ty::Lit(_) => true,
        Ty::Qualified { quals, base } => !quals.iter().any(|q| q.name == "Mut") && immutable(symbols, base, visiting),
        Ty::Named { name, args } => match name.as_str() {
            "Byte" | "Int" | "Long" | "Float" | "Double" | "Char" | "Bool" | "Str" | "None" => true,
            // [actor-types] An addr or a pool is a scheduler index: copying
            // one is the reference itself.
            "Addr" | "Pool" => true,
            // [platform-type] A copy of a platform handle shares the host
            // object by definition, when nothing reachable through it is
            // mutable. A platform *collection* is immutable only if its type
            // arguments are, and a `Deque` never is [col-deque].
            _ if symbols.intrinsic_types.get(name.as_str()).is_some_and(|t| t.platform) => {
                name != "Deque" && args.iter().all(|a| immutable(symbols, a, visiting))
            }
            _ => {
                // A type alias is what it names.
                if let Some(alias) = symbols.type_aliases.get(name.as_str()).and_then(|d| d.alias.as_ref()) {
                    if visiting.iter().any(|v| v == name) {
                        return true;
                    }
                    visiting.push(name.clone());
                    let ok = crate::wire::approx_ty(alias, &HashMap::new()).is_some_and(|t| immutable(symbols, &t, visiting));
                    visiting.pop();
                    return ok;
                }
                let Some(s) = symbols.structs.get(name.as_str()) else { return false };
                if visiting.iter().any(|v| v == name) {
                    return true;
                }
                // A struct value without `Mut` cannot have fields assigned
                // [struct-mut]; its fields must still be immutable themselves.
                visiting.push(name.clone());
                let subst: HashMap<String, Ty> =
                    s.generics.iter().map(|g| g.name.clone()).zip(args.iter().cloned()).collect();
                let ok = s.fields.iter().all(|field| match crate::wire::approx_ty(&field.ty, &subst) {
                    Some(t) => immutable(symbols, &t, visiting),
                    None => false,
                });
                visiting.pop();
                ok
            }
        },
        Ty::Union(arms) => arms.iter().all(|a| immutable(symbols, a, visiting)),
        Ty::Tuple(elems) => elems.iter().all(|e| immutable(symbols, e, visiting)),
        // Arrays are index-assignable without `Mut`.
        Ty::Array(_) => false,
        // Function values are opaque and immutable.
        Ty::Fn { .. } => true,
        // An identity is not a value, so no value of this "type" exists.
        Ty::FnName(_) => false,
        Ty::Var(_) | Ty::Any | Ty::Never | Ty::Unknown => false,
    }
}
