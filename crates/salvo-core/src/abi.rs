//! [platform-abi] What the host project of a platform root needs to see: the
//! declarations reached from the project's platform signatures (user decision
//! 2026-10-01, ABI.md D4 (a)).
//!
//! An implementation file compiles in the host project against generated
//! declaration files — the same definitions the build emits, with nothing
//! else of the program. This module computes *which* declarations: starting
//! from every `platform handler` (the effect it implements, its constructor
//! parameters) and every `platform fn` (its parameters and result) of the
//! project's own modules, it follows types — struct fields, type aliases,
//! union arms, effect members' signatures — to a fixed point. Each backend
//! emits exactly these, plus the runtime files they need.

use std::collections::BTreeSet;

use salvo_syntax::ast::{EffectRef, FnDecl, Item, Type};

use crate::program::{Program, Symbols};

/// The names of the structs, type aliases and effects the platform surface
/// reaches. Empty when the project declares nothing platform.
pub fn platform_closure(program: &Program, symbols: &Symbols<'_>) -> BTreeSet<String> {
    closure(program, symbols, false)
}

/// [platform-factory] The same over **every** module's platform declarations,
/// std's and dependencies' included: the build emits factories for all of
/// them, since every implementation file it compiles may call one.
pub fn platform_closure_all(program: &Program, symbols: &Symbols<'_>) -> BTreeSet<String> {
    closure(program, symbols, true)
}

fn closure(program: &Program, symbols: &Symbols<'_>, all: bool) -> BTreeSet<String> {
    let mut walk = Walk { symbols, seen: BTreeSet::new(), queue: Vec::new() };
    for unit in program.units() {
        // The project's own modules — std's too when the tree being built is
        // std (its files shadow the embedded copy [std-shadow]).
        if !all && ((unit.file.is_std && !unit.file.is_shadow) || unit.file.dependency.is_some()) {
            continue;
        }
        for item in &unit.ast.items {
            match item {
                Item::Handler(h) if h.platform => {
                    for t in &h.of {
                        walk.ty(t);
                    }
                    for p in &h.params {
                        walk.ty(&p.ty);
                    }
                }
                Item::Fn(f) if f.platform => walk.signature(f),
                _ => {}
            }
        }
    }
    while let Some(name) = walk.queue.pop() {
        if let Some(s) = symbols.structs.get(name.as_str()) {
            for field in &s.fields {
                walk.ty(&field.ty);
            }
            // [name-dot] A namespacing struct carries its dot-named members.
            let prefix = format!("{name}.");
            for other in symbols.structs.keys().filter(|k| k.starts_with(&prefix)) {
                walk.name(other);
            }
        } else if let Some(t) = symbols.type_aliases.get(name.as_str()) {
            if let Some(alias) = &t.alias {
                walk.ty(alias);
            }
            let prefix = format!("{name}.");
            for other in symbols.structs.keys().filter(|k| k.starts_with(&prefix)) {
                walk.name(other);
            }
        } else if let Some(e) = symbols.effects.get(name.as_str()) {
            for f in &e.fns {
                walk.signature(f);
            }
            for p in &e.prereqs {
                walk.effect_ref(p);
            }
        }
    }
    walk.seen
}

struct Walk<'s, 'p> {
    symbols: &'s Symbols<'p>,
    seen: BTreeSet<String>,
    queue: Vec<String>,
}

impl Walk<'_, '_> {
    fn name(&mut self, name: &str) {
        let known = self.symbols.structs.contains_key(name)
            || self.symbols.type_aliases.contains_key(name)
            || self.symbols.effects.contains_key(name);
        if known && self.seen.insert(name.to_string()) {
            self.queue.push(name.to_string());
        }
    }

    fn signature(&mut self, f: &FnDecl) {
        for p in &f.params {
            self.ty(&p.ty);
        }
        if let Some(r) = &f.return_type {
            self.ty(r);
        }
    }

    fn effect_ref(&mut self, e: &EffectRef) {
        if let EffectRef::Effect(r) | EffectRef::AnyEffect(r) = e {
            self.name(&r.name.name);
            for a in &r.args {
                self.ty(a);
            }
        }
    }

    fn ty(&mut self, t: &Type) {
        match t {
            Type::Named { qualifiers, base } => {
                for q in qualifiers {
                    for a in &q.args {
                        self.ty(a);
                    }
                }
                self.name(&base.name.name);
                for a in &base.args {
                    self.ty(a);
                }
            }
            Type::QualifiedGroup { base, .. } => self.ty(base),
            Type::Union { arms, .. } | Type::Tuple { elems: arms, .. } => {
                for a in arms {
                    self.ty(a);
                }
            }
            Type::Array { elem, .. } | Type::Nullable { inner: elem, .. } => self.ty(elem),
            Type::Fn { params, ret, .. } => {
                for p in params {
                    self.ty(p);
                }
                self.ty(ret);
            }
            Type::Literal { .. } => {}
        }
    }
}

/// [platform-abi] [platform-check] What an adapter or a platform fn's wrapper
/// checks of one value crossing from host to Salvo (user decisions
/// 2026-10-01, ABI.md D7, D10 C3): exactly what the Salvo type promises and
/// the host type cannot say. Computed by the checker over the *runtime*
/// shape ([union-arm-identity]: a base's literals are one arm), so both
/// backends render the same plan; a value that needs no check has no plan.
#[derive(Clone, Debug, PartialEq)]
pub enum BoundaryCheck {
    /// A closed literal arm: the value is one of these.
    OneOf(Vec<salvo_syntax::ast::TypeLit>),
    /// State qualifiers, each checked by running its `qualifies`: the
    /// qualifier's name and declaring module, then the check of the value
    /// under them, if any.
    /// `ty` is the value's runtime type, for a backend that must cast to it.
    Qualifies {
        quals: Vec<(String, crate::source::ModulePath)>,
        ty: crate::types::Ty,
        inner: Option<Box<BoundaryCheck>>,
    },
    /// Every element of a `List`, `Set` or array.
    Elems(Box<BoundaryCheck>),
    /// Every entry of a `Map`: keys, values.
    Entries(Option<Box<BoundaryCheck>>, Option<Box<BoundaryCheck>>),
    /// A value that may be `None`: the check applies otherwise.
    Nullable(Box<BoundaryCheck>),
    /// A runtime union of `arity` value arms: the check of each arm that has
    /// one, with the arm's runtime type.
    Union { arity: usize, arms: Vec<(usize, crate::types::Ty, BoundaryCheck)> },
    /// A struct's fields that need checking.
    Struct { name: String, fields: Vec<(String, BoundaryCheck)> },
    /// A tuple's elements that need checking, by position.
    Tuple { arity: usize, elems: Vec<(usize, BoundaryCheck)> },
}

impl BoundaryCheck {
    /// Whether the check walks a collection, whose cost grows with what the
    /// host returned — what the declaration's warning names (D10 C3).
    pub fn walks(&self) -> bool {
        match self {
            BoundaryCheck::Elems(_) | BoundaryCheck::Entries(..) => true,
            BoundaryCheck::OneOf(_) => false,
            BoundaryCheck::Qualifies { inner, .. } => inner.as_ref().is_some_and(|c| c.walks()),
            BoundaryCheck::Nullable(c) => c.walks(),
            BoundaryCheck::Union { arms, .. } => arms.iter().any(|(_, _, c)| c.walks()),
            BoundaryCheck::Struct { fields, .. } => fields.iter().any(|(_, c)| c.walks()),
            BoundaryCheck::Tuple { elems, .. } => elems.iter().any(|(_, c)| c.walks()),
        }
    }
}

/// [platform-factory] One factory of a union at the boundary (user decisions
/// 2026-10-01, ABI.md D5): builds runtime arm `arm` from a `param`.
#[derive(Clone, Debug, PartialEq)]
pub struct Factory {
    /// Snake case; each backend spells it in its own case.
    pub name: String,
    /// The runtime arm, 0-based.
    pub arm: usize,
    /// The arm's runtime type: what the factory takes.
    pub param: crate::types::Ty,
    /// A literal arm's factory checks its argument (one `str(value)`, D5).
    pub check: Option<BoundaryCheck>,
}

/// [platform-factory] The factories of one union: a named one's (`FsErrors`
/// on Kotlin, `impl FsError` on Rust) or a signature's anonymous one (an
/// object named after the fn or member, `ReadToStr`).
#[derive(Clone, Debug, PartialEq)]
pub struct Factories {
    /// The runtime value arms.
    pub arity: usize,
    /// The union admits `None`.
    pub nullable: bool,
    /// The runtime type the factories return.
    pub union: crate::types::Ty,
    pub arms: Vec<Factory>,
    /// Names two or more arms would share, which get no factory (D5).
    pub dropped: Vec<String>,
}

/// [platform-factory] The snake-case name of a factory for a runtime arm of
/// type `ty`: a qualified arm by its qualifier (`Ok Int` → `ok`), a struct or
/// a named union by its name (`NotFound` → `not_found`), a base type by its
/// own (`Str` → `str`, `List<T>` → `list`). `None` for an arm with no name.
pub fn factory_name(ty: &crate::types::Ty) -> Option<String> {
    use crate::types::Ty;
    match ty {
        Ty::Qualified { quals, base } => quals
            .iter()
            .find(|q| !q.effect && q.name != "Mut")
            .map(|q| snake(&q.name))
            .or_else(|| factory_name(base)),
        Ty::Named { name, .. } => Some(snake(name.rsplit('.').next().unwrap_or(name))),
        Ty::Tuple(_) => Some("tuple".to_string()),
        Ty::Array(_) => Some("array".to_string()),
        Ty::Lit(l) => Some(snake(l.base())),
        _ => None,
    }
}

/// `NotFound` → `not_found`, `Str` → `str`, `ok` → `ok`.
pub fn snake(name: &str) -> String {
    let mut out = String::new();
    for (i, c) in name.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 && !out.ends_with('_') {
                out.push('_');
            }
            out.extend(c.to_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

/// `read_to_str` → `ReadToStr`: the object of a signature's factories.
pub fn upper_camel(name: &str) -> String {
    name.split('_')
        .filter(|p| !p.is_empty())
        .map(|p| {
            let mut c = p.chars();
            match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                None => String::new(),
            }
        })
        .collect()
}
