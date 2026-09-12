//! Semantic type representation for the Salvo checker.
//!
//! `Ty` is the checker's view of a type: aliases expanded, `T?` desugared to
//! `T | None`, unions flattened and deduplicated, qualifiers attached as a
//! sorted set. Structural equality (`PartialEq`/`Hash`) is meaningful and is
//! used for union arm identity.

use std::collections::HashSet;
use std::fmt;

/// A qualifier applied to a type, e.g. `Ok` in `Ok Int` or `Mut` in
/// `Mut List<T>`. Generic qualifier arguments are rarely written explicitly
/// (`Ok Str` implies `Ok<Str>`), so `args` is usually empty.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Qual {
    pub name: String,
    pub args: Vec<Ty>,
    /// [fn-effects] This "qualifier" names an **effect**, not a qualifier
    /// declaration. The spelling was how a producer *type* declared the
    /// effects its driving performs (user decision 2026-09-07, when a
    /// producer was an `Iter<T>` value); with the reduction to `next` a
    /// producer is a struct whose `next` is an ordinary fn, so writing an
    /// effect in qualifier position is now *refused* at declarations,
    /// naming the replacement — the `next`'s own effect list. The flag
    /// stays because the refused type still lowers (one mistake, one
    /// diagnostic) and the `^`-widening diagnostic still explains why such
    /// a claim could not drop.
    ///
    /// It rides in `Qual` because the spelling, the display and the erasure
    /// are a qualifier's, and it carries a flag because the *rules* are not:
    /// the variance is inverted (fewer effects fits where more are
    /// expected), it never drops, and nothing tests it at run time. A name
    /// alone could not tell the two apart here — a qualifier's name is
    /// arbitrary and so is an effect's.
    pub effect: bool,
}

impl Qual {
    pub fn plain(name: impl Into<String>, args: Vec<Ty>) -> Self {
        Qual {
            name: name.into(),
            args,
            effect: false,
        }
    }

    /// An effect claim in qualifier position [fn-effects].
    pub fn effect(name: impl Into<String>, args: Vec<Ty>) -> Self {
        Qual {
            name: name.into(),
            args,
            effect: true,
        }
    }

    /// Why this qualifier may not be dropped from a type, if it may not.
    pub fn drop_block(&self) -> Option<&'static str> {
        if self.effect {
            // [fn-effects] Dropping the claim would let a producer that
            // performs effects into a position that supplies none.
            return Some(
                "an effect on a producer restricts rather than refines: dropping it \
                 would let a producer that performs effects be driven where none can \
                 be supplied",
            );
        }
        qual_drop_block(&self.name)
    }
}

/// What a call does to an argument's known qualifiers [deduce-syntax].
/// All three reduce to a removal set at the call site, computed against
/// the qualifiers the *argument* actually carries — which is what makes
/// the exhaustive form sound: it drops qualifiers the callee never
/// declared and therefore cannot have preserved.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum QualEffect {
    /// Bare `[p]`: nothing is stripped. Only sound for a parameter the
    /// body cannot invalidate — i.e. does not mutate.
    KeepAll,
    /// `[p: A B]` / `[p:]`: afterwards *exactly* these apply.
    Exhaustive(Vec<String>),
    /// `[p: -A -B]`: these are dropped, everything else survives.
    Remove(Vec<String>),
}

impl QualEffect {
    /// The qualifiers to strip from an argument that currently carries
    /// `have`.
    /// `is_provenance` exempts provenance qualifiers [qual-subject]: the
    /// stripping rule is sound only for claims about *contents*, so a
    /// claim about where the handle came from survives any call. Callers
    /// pass a predicate because only the checker knows the declarations.
    pub fn removal_set(
        &self,
        have: &[String],
        is_provenance: impl Fn(&str) -> bool,
    ) -> HashSet<String> {
        let removed: HashSet<String> = match self {
            QualEffect::KeepAll => HashSet::new(),
            QualEffect::Exhaustive(keep) => have
                .iter()
                .filter(|q| !keep.contains(q))
                .cloned()
                .collect(),
            QualEffect::Remove(drop) => {
                drop.iter().filter(|q| have.contains(q)).cloned().collect()
            }
        };
        removed
            .into_iter()
            .filter(|q| !is_provenance(q))
            .collect()
    }

    /// The qualifiers a caller may still assume, given a parameter's
    /// declared set — for signature rendering and body validation.
    pub fn kept_quals(&self, declared: &[String]) -> Vec<String> {
        match self {
            QualEffect::KeepAll => declared.to_vec(),
            QualEffect::Exhaustive(keep) => keep.clone(),
            QualEffect::Remove(drop) => declared
                .iter()
                .filter(|q| !drop.contains(q))
                .cloned()
                .collect(),
        }
    }
}

/// One parameter of a fn type's *contract* [fn-contract]: whether a call
/// through the fn value keeps the argument, which qualifier names stay
/// known, and whether the declared parameter type grants mutation
/// (`Mut`). An fn type without a written contract keeps everything (the
/// default).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FnParamContract {
    pub name: Option<String>,
    pub kept: bool,
    /// What a call through this fn value does to the argument's
    /// qualifiers [deduce-syntax].
    pub effect: QualEffect,
    pub mutable: bool,
    /// [proj-infer] `[p: Proj]` written on the fn type: a call's result
    /// holds a borrow of this argument.
    pub lent: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Ty {
    /// A nominal type: `Str`, `List<Int>`, a struct, an effect, ...
    Named { name: String, args: Vec<Ty> },
    /// A qualified type: `Ok Int`, `Mut NonEmpty List<T>`. Invariants:
    /// `quals` is non-empty and sorted by name; `base` is never `Qualified`.
    Qualified { quals: Vec<Qual>, base: Box<Ty> },
    /// A union: at least two arms, no arm is itself a union, arms are
    /// deduplicated [type-union], declaration order preserved (arm order
    /// is the wrapper arm identity for codegen [union-arm-identity]).
    Union(Vec<Ty>),
    Tuple(Vec<Ty>),
    Array(Box<Ty>),
    /// A function/lambda type; `contract` carries the written per-param
    /// deduction facts [fn-contract] (`None` = keeps everything).
    Fn {
        params: Vec<Ty>,
        ret: Box<Ty>,
        contract: Option<Vec<FnParamContract>>,
        /// The effects a *call* of this fn value performs [fn-effects]:
        /// declared on the type (`(s: Str) [Logger] -> Str`) or inferred
        /// from a lambda's body. The value carries no capability — the
        /// caller supplies these at each call — so a fn value may be
        /// stored and passed freely; only *calling* it needs them in
        /// scope.
        effects: Vec<Ty>,
    },
    /// A generic type parameter in scope, e.g. `T`.
    Var(String),
    Any,
    /// The type of `return`/`break`/`continue`; subtype of everything
    /// [type-any-nothing].
    Nothing,
    /// An unknown/unchecked type. Compatible with everything; produced when
    /// the checker cannot determine a type. Never an error by itself
    /// [type-unknown-lenient].
    Unknown,
}

impl Ty {
    pub fn named(name: impl Into<String>) -> Ty {
        Ty::Named {
            name: name.into(),
            args: Vec::new(),
        }
    }

    pub fn none() -> Ty {
        Ty::named("None")
    }

    pub fn is_none_ty(&self) -> bool {
        matches!(self, Ty::Named { name, args } if name == "None" && args.is_empty())
    }

    /// Whether this is `Bool` — qualifiers ignored, since a claim about a
    /// boolean is still a boolean [cond-bool].
    pub fn is_bool(&self) -> bool {
        matches!(self.strip_quals(), Ty::Named { name, args } if name == "Bool" && args.is_empty())
    }

    pub fn is_unknown(&self) -> bool {
        matches!(self, Ty::Unknown)
    }

    /// The type without its qualifiers.
    pub fn strip_quals(&self) -> &Ty {
        match self {
            Ty::Qualified { base, .. } => base,
            other => other,
        }
    }

    /// [proj-type] The type with a *top-level* `Proj` removed (nested ones —
    /// inside a union arm or a type argument — stay): what a kept, non-`Mut`
    /// position reads through a projection.
    pub fn strip_top_proj(&self) -> Ty {
        let mut names = HashSet::new();
        names.insert("Proj".to_string());
        self.clone().remove_quals(&names)
    }

    /// [proj-type] Whether the type is a projection at its top level.
    pub fn is_proj(&self) -> bool {
        self.quals().iter().any(|q| q.name == "Proj")
    }

    pub fn quals(&self) -> &[Qual] {
        match self {
            Ty::Qualified { quals, .. } => quals,
            _ => &[],
        }
    }

    /// [fn-effects] The effect claims this producer type carries, as
    /// effect types, in the **canonical order**: sorted, because the
    /// generated trait is per effect *set* and `qualify` normalizes
    /// qualifier order anyway, so two producers written the two ways must
    /// agree on their machine's parameter order.
    pub fn effect_claims(&self) -> Vec<Ty> {
        let mut out: Vec<Ty> = self
            .quals()
            .iter()
            .filter(|q| q.effect)
            .map(|q| Ty::Named {
                name: q.name.clone(),
                args: q.args.clone(),
            })
            .collect();
        out.sort_by_key(|t| t.to_string());
        out
    }

    /// Applies additional qualifiers to a type (merging and re-sorting).
    pub fn qualify(self, mut new_quals: Vec<Qual>) -> Ty {
        if new_quals.is_empty() {
            return self;
        }
        let (mut quals, base) = match self {
            Ty::Qualified { quals, base } => (quals, *base),
            other => (Vec::new(), other),
        };
        quals.append(&mut new_quals);
        // [copy-scalar-free] `Proj Int` is `Int`: a borrowed Copy scalar is
        // the value itself on both backends, so the qualifier is erased
        // rather than carried (and `Proj Proj X` dedups to `Proj X` below
        // [proj-type]).
        if is_copy_scalar(&base) {
            quals.retain(|q| q.name != "Proj");
        }
        quals.sort_by(|a, b| a.name.cmp(&b.name));
        quals.dedup();
        if quals.is_empty() {
            return base;
        }
        Ty::Qualified {
            quals,
            base: Box::new(base),
        }
    }

    /// The type with the named qualifiers removed (a no-op when none
    /// match); collapses to the base type when no qualifiers remain
    /// [deduce-consume].
    pub fn remove_quals(self, names: &HashSet<String>) -> Ty {
        match self {
            Ty::Qualified { quals, base } => {
                let quals: Vec<Qual> = quals
                    .into_iter()
                    .filter(|q| !names.contains(&q.name))
                    .collect();
                if quals.is_empty() {
                    *base
                } else {
                    Ty::Qualified { quals, base }
                }
            }
            other => other,
        }
    }

    /// Builds a normalized union: flattens nested unions, drops `Nothing`
    /// arms, deduplicates (keeping first occurrence). Returns the single arm
    /// directly when only one remains.
    pub fn union_of(arms: Vec<Ty>) -> Ty {
        let mut flat: Vec<Ty> = Vec::new();
        let push = |ty: Ty, flat: &mut Vec<Ty>| {
            if ty == Ty::Nothing || flat.contains(&ty) {
                return;
            }
            flat.push(ty);
        };
        for arm in arms {
            match arm {
                Ty::Union(inner) => {
                    for a in inner {
                        push(a, &mut flat);
                    }
                }
                other => push(other, &mut flat),
            }
        }
        match flat.len() {
            0 => Ty::Nothing,
            1 => flat.pop().unwrap(),
            _ => Ty::Union(flat),
        }
    }

    /// The arms of this type viewed as a union (a non-union type is a
    /// single-arm union).
    pub fn arms(&self) -> &[Ty] {
        match self {
            Ty::Union(arms) => arms,
            other => std::slice::from_ref(other),
        }
    }

    /// The non-`None` arms of this type viewed as a union.
    pub fn value_arms(&self) -> Vec<&Ty> {
        self.arms().iter().filter(|a| !a.is_none_ty()).collect()
    }

    /// Whether the union includes a `None` arm (i.e. is nullable).
    pub fn has_none_arm(&self) -> bool {
        self.arms().iter().any(|a| a.is_none_ty())
    }

    /// Whether this type is represented as a sealed union wrapper in the
    /// backend: two or more non-`None` arms.
    pub fn is_wrapper_union(&self) -> bool {
        matches!(self, Ty::Union(_)) && self.value_arms().len() >= 2
    }

    /// Removes `None` arms (the type of `x!`).
    pub fn without_none(&self) -> Ty {
        match self {
            Ty::Union(arms) => {
                Ty::union_of(arms.iter().filter(|a| !a.is_none_ty()).cloned().collect())
            }
            other => other.clone(),
        }
    }
}

/// The subtype relation. `Unknown` is compatible in both directions so that
/// unchecked code never produces cascading errors.
pub fn is_subtype(a: &Ty, b: &Ty) -> bool {
    if a == b || a.is_unknown() || b.is_unknown() {
        return true;
    }
    match (a, b) {
        (Ty::Nothing, _) => true,
        (_, Ty::Any) => true,
        // A union is a subtype when every arm is.
        (Ty::Union(arms), _) => arms.iter().all(|arm| is_subtype(arm, b)),
        // A qualified union group (`Ok (A | B)`) matches an identical union
        // arm, or may drop its group qualifiers (checked before the any-arm
        // rule below, which would compare the whole group against arms)
        // [qual-group].
        (Ty::Qualified { quals, base }, _) if matches!(**base, Ty::Union(_)) => {
            if let Ty::Union(arms) = b {
                if arms.iter().any(|arm| a == arm) {
                    return true;
                }
            }
            // [proj-type] The group may drop its qualifiers only if none is a
            // never-drop one: `Proj (A | B)` is a borrow of the union, not
            // the union.
            if quals.iter().any(|q| q.drop_block().is_some()) {
                // …unless the target is the same group minus droppable
                // extras, handled by the qualified/qualified arm below.
                if let Ty::Qualified { .. } = b {
                    // fall through to the general arm
                } else {
                    return false;
                }
            } else {
                return is_subtype(base, b);
            }
            match b {
                Ty::Qualified { quals: qb, base: bb } => {
                    is_subtype(base, bb)
                        && qb.iter().all(|q| quals.contains(q))
                        && quals
                            .iter()
                            .filter(|q| q.drop_block().is_some())
                            .all(|q| qb.contains(q))
                }
                _ => false,
            }
        }
        // A non-union is a subtype of a union when it fits some arm.
        (_, Ty::Union(arms)) => arms.iter().any(|arm| is_subtype(a, arm)),
        // [proj-type] `X <: Proj X`: an owned value fits a projected position
        // (the borrow of an owned value is a borrow), never the reverse. The
        // value's own top-level `Proj`, if any, is matched by the
        // qualified/qualified arm; here `a` has none.
        (_, Ty::Qualified { quals, .. })
            if quals.iter().any(|q| q.name == "Proj")
                && !a.quals().iter().any(|q| q.name == "Proj") =>
        {
            let mut names = HashSet::new();
            names.insert("Proj".to_string());
            is_subtype(a, &b.clone().remove_quals(&names))
        }
        (
            Ty::Qualified { quals: qa, base: ba },
            Ty::Qualified { quals: qb, base: bb },
        ) => {
            // `Q (A | B)` against a value whose qualifier list *flattened*
            // [qual-group]: applying `Q` to an already-qualified value
            // (`emitted(ok("x"))`) appends to one flat list, so
            // `Q (A | B)` and `Q A` are indistinguishable by shape.
            // Split the group's own qualifiers off the value's list and let
            // the remainder try the union — which is what the value really
            // is once `Q` is accounted for.
            (is_subtype(ba, bb) || nested_group_remainder(qa, qb, ba, bb).is_some())
                && qb
                    .iter()
                    .all(|q| q.name == "Once" || q.effect || qa.contains(q))
                // [proj-type] A never-drop qualifier the value carries
                // (`Proj`, `Linear`) must be expected too: `Emitted (Proj
                // Str)` is not an `Emitted Str` — the borrow is inside.
                // `Proj` on a Copy scalar is free [copy-scalar-free].
                && qa
                    .iter()
                    .filter(|q| !q.effect && q.drop_block().is_some() && q.name != "Once")
                    .filter(|q| !(q.name == "Proj" && is_copy_scalar(ba)))
                    .all(|q| qb.contains(q))
                // [fn-effects] An effect claim runs the *other* way, like
                // [fn-effects] on a fn type: every effect the supplied
                // producer performs must be one the position expects, and a
                // producer performing fewer fits a position expecting more.
                && qa
                    .iter()
                    .filter(|q| q.effect)
                    .all(|q| qb.contains(q))
        }
        // [once-fn] INVERTED subtyping, flagged for future review
        // (user decision 2026-09-02): `Once` *restricts* (usable at
        // most once) instead of refining, so a plain value may be used
        // where a `Once` one is expected — the opposite direction of
        // every other qualifier. Generalized 2026-09-07 from fn types to
        // any base: an un-driven value of a `canbe Once` type fits where
        // a `Once` one is wanted (the `Iter<T>` factory this was built
        // for went with the reduction to `next`), since promising to use
        // something at most once demands less than being able to use it
        // repeatedly. Never the reverse — `Once` never drops [qual-widen].
        //
        // Deliberately unconditional on the base, unlike the *position*
        // rule: whether `Once` may be **written** on a type needs the
        // declaration (`canbe Once`), which lives in the checker's scope
        // and not here. An `Once` on a base that never opted in has
        // already been reported, so accepting it in this direction costs
        // nothing and keeps one mistake to one diagnostic.
        // [fn-effects] The same shape, for the same reason: a producer that
        // performs *no* effects fits a position that expects some.
        (_, Ty::Qualified { quals, base })
            if quals.iter().all(|q| q.name == "Once" || q.effect)
                && quals.iter().any(|q| q.name == "Once" || q.effect) =>
        {
            is_subtype(a, base)
        }
        // `Qual T <: T` — except the qualifiers that may never be dropped
        // ([qual-widen]'s single exclusion list: `Once`, `Linear`,
        // `Proj`). [copy-scalar-free] `Proj` on a Copy scalar is the one
        // exception: a borrowed `Int` is the number itself on both
        // backends, so `Proj Int <: Int`.
        (Ty::Qualified { quals, base }, _) => {
            quals
                .iter()
                .all(|q| q.drop_block().is_none() || (q.name == "Proj" && is_copy_scalar(base)))
                && is_subtype(base, b)
        }
        (Ty::Named { name: na, args: aa }, Ty::Named { name: nb, args: ab }) => {
            na == nb
                && aa.len() == ab.len()
                && aa.iter().zip(ab).all(|(x, y)| compatible(x, y))
        }
        (Ty::Array(x), Ty::Array(y)) => compatible(x, y),
        (Ty::Tuple(xs), Ty::Tuple(ys)) => {
            xs.len() == ys.len() && xs.iter().zip(ys).all(|(x, y)| is_subtype(x, y))
        }
        (
            Ty::Fn {
                params: pa,
                ret: ra,
                contract: ca,
                effects: ea,
            },
            Ty::Fn {
                params: pb,
                ret: rb,
                contract: cb,
                effects: eb,
            },
        ) => {
            pa.len() == pb.len()
                && pa.iter().zip(pb).all(|(x, y)| compatible(x, y))
                && is_subtype(ra, rb)
                && contract_fits(ca.as_deref(), cb.as_deref(), pa.len())
                // [fn-effects] A fn value that performs *fewer* effects fits
                // where more are expected — the caller supplies what it
                // declared and the value simply does not use all of it. The
                // reverse would let a value perform an effect its call site
                // cannot provide.
                && ea.iter().all(|e| eb.contains(e))
        }
        _ => false,
    }
}

/// [qual-group] The *inner* arm a flattened nested group fits, if any.
///
/// `Ty::Qualified` holds a flat, sorted qualifier list and its base is never
/// itself `Qualified`, so applying a qualifier to an already-qualified value
/// appends: `emitted(ok("x"))` is `Qualified { quals: [Emitted, Ok], base:
/// Str }`, which is shape-identical to "two qualifiers on a `Str`" and not
/// `Emitted` applied to `Ok Str`. When the *expected* type is a qualified
/// union group `Q (A | B)`, this reads the value the other way round: take
/// the group's qualifiers off the value's list and ask whether what is left
/// fits one arm of the union. Exactly one arm must fit — an ambiguity is not
/// resolvable from a flat list, so it is left to the caller to report.
///
/// Returns the matching arm's index. The value is physically a bare `A` at
/// that point, so the caller must wrap it into the inner union before the
/// outer one; `nested_group_arm` is the entry point emitters' coercions go
/// through.
fn nested_group_remainder(qa: &[Qual], qb: &[Qual], ba: &Ty, bb: &Ty) -> Option<usize> {
    if !bb.is_wrapper_union() || matches!(ba, Ty::Union(_)) {
        // A wrapper union is the case that has a physical inner arm to wrap
        // into. Restricting to it keeps this rule and the coercion
        // `nested_group_arm` records in exact agreement — a shape the
        // emitters could not wrap is an error rather than wrong output
        // [backend-never-wrong].
        return None;
    }
    // Match by *name*: a qualifier's generic arguments are rarely written
    // (`Ok Str` implies `Ok<Str>`), so the group's `Q` and the value's `Q`
    // need not carry identical args to be the same claim.
    let rest: Vec<Qual> = qa
        .iter()
        .filter(|q| !qb.iter().any(|g| g.name == q.name))
        .cloned()
        .collect();
    if rest.len() == qa.len() {
        // None of the group's qualifiers is on the value — nothing was
        // flattened, so this is not the nested reading.
        return None;
    }
    let remainder = ba.clone().qualify(rest);
    let arms = bb.value_arms();
    let mut found = None;
    for (i, arm) in arms.iter().enumerate() {
        if is_subtype(&remainder, arm) {
            if found.is_some() {
                return None;
            }
            found = Some(i);
        }
    }
    found
}

/// [qual-group] The inner union and arm index a value must be wrapped into
/// before it is wrapped into a qualified group arm `Q (A | B)`, when the
/// value's qualifier list flattened (`emitted(ok("x"))` for an expected
/// `Emitted (Ok Str | Err Str)`). `None` when no inner wrap is needed —
/// including when the value is already typed as the inner union or as the
/// group itself.
pub fn nested_group_arm(value: &Ty, group: &Ty) -> Option<(Ty, usize)> {
    let Ty::Qualified { quals: qb, base: bb } = group else {
        return None;
    };
    if !bb.is_wrapper_union() {
        return None;
    }
    let Ty::Qualified { quals: qa, base: ba } = value else {
        return None;
    };
    let arm = nested_group_remainder(qa, qb, ba, bb)?;
    Some(((**bb).clone(), arm))
}

/// Whether a fn value with contract `a` may be used where contract `b`
/// is expected [fn-contract]. INVERTED direction like `Once` [once-fn]:
/// a fn that *keeps* its argument fits where a *consuming* one is
/// expected (the caller merely over-estimates the damage), never the
/// reverse. `None` = keeps everything.
pub fn contract_fits(
    a: Option<&[FnParamContract]>,
    b: Option<&[FnParamContract]>,
    arity: usize,
) -> bool {
    let keeps_all = |c: Option<&[FnParamContract]>, i: usize| -> (bool, bool) {
        // (kept, mutable) per position; default keeps, not mutable-add.
        match c {
            None => (true, false),
            Some(list) => list
                .get(i)
                .map(|e| (e.kept, e.mutable))
                .unwrap_or((true, false)),
        }
    };
    (0..arity).all(|i| {
        let (a_kept, _a_mut) = keeps_all(a, i);
        let (b_kept, b_mut) = keeps_all(b, i);
        // Expected kept => supplied must keep. Expected consuming =>
        // anything fits. Expected mutable grants permission; a supplied
        // fn that mutates needs the expectation to grant it.
        let (_, a_mut) = keeps_all(a, i);
        (!b_kept || a_kept) && (!a_mut || b_mut || !b_kept)
    })
}

/// Whether `Once` may be written on this base type *without* an opt-in
/// [once-fn]: function types, where using a value means calling it. (The
/// other built-in — the `Iter<T>` factory, where using a value meant
/// driving it — went with the reduction to `next`.)
///
/// A type of one's own reaches the same place by declaring `canbe Once`,
/// which needs the declaration and therefore lives in the checker
/// (`has_auto_once`) — this predicate is only the built-in half. Widening it
/// to every type is roadmap D6, to be designed together with D7.
pub fn once_position(ty: &Ty) -> bool {
    matches!(ty, Ty::Fn { .. })
}

/// [qual-widen]. The single exclusion list: `is_subtype`'s `Qual T <: T`
/// rule and the `^` widening check both read it, so the two cannot drift as
/// intrinsic qualifiers are added (user decision 2026-09-05).
///
/// Everything else is droppable: dropping a *claim* only loses knowledge
/// (`NonEmpty`, `Ok`, provenance), and dropping a *permission* only loses
/// permission (`Mut`, and `Cell` when it arrives).
pub fn qual_drop_block(name: &str) -> Option<&'static str> {
    match name {
        // [once-fn] A once-callable fn is not a many-callable fn.
        "Once" => Some(
            "`Once` restricts rather than refines: dropping it would make a              once-callable value callable again",
        ),
        // [linear-obligation] Declared on the type, never written at a use
        // site, so there is nothing to remove — and removing it would drop
        // a use obligation.
        "Linear" => Some(
            "linearity is declared on the type, not applied at a use site, and              it carries a use obligation that cannot be dropped",
        ),
        // [readonly-return] The value is borrowed from somewhere else.
        "Proj" => Some(
            "`Proj` marks a value derived from another: dropping it would              claim ownership the value does not have",
        ),
        _ => None,
    }
}

/// Invariant compatibility (used for generic arguments).
/// [copy-scalar-free] A bare native scalar, whose copy is free and whose
/// projection is therefore the value itself.
pub fn is_copy_scalar(ty: &Ty) -> bool {
    matches!(
        ty,
        Ty::Named { name, args }
            if args.is_empty()
                && matches!(
                    name.as_str(),
                    "Int" | "Long" | "Float" | "Double" | "Bool" | "Char" | "Byte"
                )
    )
}

pub fn compatible(a: &Ty, b: &Ty) -> bool {
    is_subtype(a, b) && is_subtype(b, a)
}

/// [fn-overload-rank] How two parameter *patterns* (the un-substituted
/// declared parameter types of two overload candidates) compare in
/// **specificity**: `Greater` means `a` is more specific — it says more
/// about the value it accepts.
///
/// The whole ladder, and no more (user decisions 2026-09-06/07):
///
/// 1. **A type variable knows nothing**, so it is the least specific thing
///    a parameter can say — structurally, so `List<Int>` beats `List<T>`.
/// 2. **Broader accepts less specifically**: a union's arms compare as
///    *sets*, so `Int` (one arm) beats `Int | Str` beats `Int | Str | Bool`,
///    and `Int` beats `Int?`. `Any` is the broadest type there is, so it is
///    always the least specific.
/// 3. **A qualifier says more**: qualifier *sets* compare by inclusion, so
///    `Mut NonEmpty List<T>` beats `Mut List<T>` beats `List<T>`. The
///    *kind* of qualifier deliberately does not matter — ranking `Mut`
///    against `NonEmpty` would ask the caller to know more than what is in
///    front of them — so `Mut List<T>` and `NonEmpty List<T>` are
///    unrankable.
///
/// `None` is "these two cannot be ranked", which is what makes this a
/// partial order: an un-rankable best set is an *error* naming the remedies
/// rather than a coin flip [fn-overload-ambiguous]. Two criteria pulling in
/// opposite directions is unrankable too (a more specific base with a
/// smaller qualifier set), for the same reason.
pub fn spec_cmp(a: &Ty, b: &Ty) -> Option<std::cmp::Ordering> {
    use std::cmp::Ordering::*;
    if a == b {
        return Some(Equal);
    }
    match (a, b) {
        // 1. A type variable knows nothing; anything else knows something.
        (Ty::Var(_), Ty::Var(_)) => Some(Equal),
        (Ty::Var(_), _) => Some(Less),
        (_, Ty::Var(_)) => Some(Greater),
        // 2. `Any` accepts everything, so nothing is broader.
        (Ty::Any, Ty::Any) => Some(Equal),
        (Ty::Any, _) => Some(Less),
        (_, Ty::Any) => Some(Greater),
        // 3. Qualifier sets by inclusion, together with the bases. The
        // *larger* qualifier set says more, so the sets go in swapped:
        // `set_cmp` ranks the smaller set higher. Except `Proj`, which
        // inverts [proj-type]: `X <: Proj X`, so a `Proj` position accepts
        // owned values *too* — it accepts more, so it says less, the way a
        // union says less than one of its arms. It is compared as its own
        // dimension, so `Proj Mut Str` vs `Str` (more qualifiers, less
        // ownership) is unrankable rather than a guess.
        (Ty::Qualified { .. }, _) | (_, Ty::Qualified { .. }) => {
            let (mut qa, mut qb) = (qual_names(a), qual_names(b));
            let pa = qa.iter().position(|q| *q == "Proj").map(|i| qa.remove(i)).is_some();
            let pb = qb.iter().position(|q| *q == "Proj").map(|i| qb.remove(i)).is_some();
            let proj_dim = match (pa, pb) {
                (false, true) => Greater,
                (true, false) => Less,
                _ => Equal,
            };
            combine(
                [
                    Some(proj_dim),
                    set_cmp(&qb, &qa),
                    spec_cmp(a.strip_quals(), b.strip_quals()),
                ]
                .into_iter(),
            )
        }
        // 4. Unions by arm inclusion — including the single-arm case, which
        // is how an arm beats the union it belongs to.
        (Ty::Union(_), _) | (_, Ty::Union(_)) => {
            let (aa, ba) = (a.arms(), b.arms());
            set_cmp(aa, ba)
        }
        (Ty::Named { name: na, args: aa }, Ty::Named { name: nb, args: ab })
            if na == nb && aa.len() == ab.len() =>
        {
            combine(aa.iter().zip(ab).map(|(x, y)| spec_cmp(x, y)))
        }
        (Ty::Array(x), Ty::Array(y)) => spec_cmp(x, y),
        (Ty::Tuple(xs), Ty::Tuple(ys)) if xs.len() == ys.len() => {
            combine(xs.iter().zip(ys).map(|(x, y)| spec_cmp(x, y)))
        }
        (
            Ty::Fn {
                params: pa,
                ret: ra,
                ..
            },
            Ty::Fn {
                params: pb,
                ret: rb,
                ..
            },
        ) if pa.len() == pb.len() => combine(
            pa.iter()
                .zip(pb)
                .map(|(x, y)| spec_cmp(x, y))
                .chain(std::iter::once(spec_cmp(ra, rb))),
        ),
        _ => None,
    }
}

/// The qualifier names of a type, sorted (`Ty::Qualified` keeps them
/// sorted, so this is just a projection).
fn qual_names(ty: &Ty) -> Vec<&str> {
    ty.quals().iter().map(|q| q.name.as_str()).collect()
}

/// [fn-overload-rank] Set inclusion as a specificity comparison: the
/// **smaller** set is the more specific statement — fewer arms accepted,
/// or more qualifiers demanded, depending on which side calls this. Both
/// callers pass the set whose *shrinking* means "says more" first, so
/// `Greater` always means "a is more specific".
///
/// Sets that neither contain the other are unrankable, which is what makes
/// `Int | Str` and `Int | Bool` an ambiguity rather than a guess.
fn set_cmp<T: PartialEq>(a: &[T], b: &[T]) -> Option<std::cmp::Ordering> {
    use std::cmp::Ordering::*;
    let a_in_b = a.iter().all(|x| b.contains(x));
    let b_in_a = b.iter().all(|x| a.contains(x));
    match (a_in_b, b_in_a) {
        (true, true) => Some(Equal),
        // `a` is a subset: it accepts fewer things, so it says more.
        (true, false) => Some(Greater),
        (false, true) => Some(Less),
        (false, false) => None,
    }
}

/// Folds child comparisons into one: all `Equal` is `Equal`, a consistent
/// direction wins, and a disagreement (or any unrankable child) is `None`.
fn combine(items: impl Iterator<Item = Option<std::cmp::Ordering>>) -> Option<std::cmp::Ordering> {
    use std::cmp::Ordering::*;
    let mut acc = Equal;
    for item in items {
        match item? {
            Equal => {}
            dir if acc == Equal => acc = dir,
            dir if dir == acc => {}
            _ => return None,
        }
    }
    Some(acc)
}

/// [fn-overload-rank] One overload candidate as the ranking sees it: its
/// declared parameter patterns, one per *argument slot*, and whether the
/// slots came from a variadic parameter.
#[derive(Clone, Debug)]
pub struct RankedCandidate {
    pub patterns: Vec<Ty>,
    /// True when the candidate collects some of these slots with `...xs`.
    pub variadic: bool,
}

/// [fn-overload-rank] How two candidates compare: per **argument slot**,
/// and a candidate wins only by being at least as specific everywhere and
/// strictly more specific somewhere. A sum of per-slot scores was the old
/// rule and is deliberately gone: it let one argument's gain pay for
/// another's loss, which is the definition of a guess.
///
/// The last word is arity shape: with the slots otherwise equal, a
/// **fixed** parameter list beats a variadic one — `list()` picks the
/// no-argument overload over `list(...elems)`, which is what lets an
/// "empty" case be an overload rather than a special form.
pub fn rank_cmp(a: &RankedCandidate, b: &RankedCandidate) -> Option<std::cmp::Ordering> {
    use std::cmp::Ordering::*;
    if a.patterns.len() != b.patterns.len() {
        return None;
    }
    let slots = combine(
        a.patterns
            .iter()
            .zip(&b.patterns)
            .map(|(x, y)| spec_cmp(x, y)),
    )?;
    if slots != Equal {
        return Some(slots);
    }
    match (a.variadic, b.variadic) {
        (false, true) => Some(Greater),
        (true, false) => Some(Less),
        _ => Some(Equal),
    }
}

/// [fn-overload-rank] Whether `a` is strictly more specific than `b`.
pub fn spec_dominates(a: &RankedCandidate, b: &RankedCandidate) -> bool {
    rank_cmp(a, b) == Some(std::cmp::Ordering::Greater)
}

/// [fn-overload-rank] The index of the unique candidate that dominates
/// every other, if there is one. `None` means the set has no single most
/// specific member — which is an ambiguity, never a pick
/// [fn-overload-ambiguous].
pub fn most_specific(candidates: &[RankedCandidate]) -> Option<usize> {
    if candidates.len() == 1 {
        return Some(0);
    }
    (0..candidates.len()).find(|&i| {
        (0..candidates.len())
            .all(|j| i == j || spec_dominates(&candidates[i], &candidates[j]))
    })
}

impl fmt::Display for Qual {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)?;
        fmt_args(f, &self.args)
    }
}

impl fmt::Display for Ty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Ty::Named { name, args } => {
                write!(f, "{name}")?;
                fmt_args(f, args)
            }
            Ty::Qualified { quals, base } => {
                // [proj-type] `Proj` reads first: it says what the value *is*
                // (a borrow); the rest say what is known about it.
                for q in quals.iter().filter(|q| q.name == "Proj") {
                    write!(f, "{q} ")?;
                }
                for q in quals.iter().filter(|q| q.name != "Proj") {
                    write!(f, "{q} ")?;
                }
                if matches!(**base, Ty::Union(_)) {
                    write!(f, "({base})")
                } else {
                    write!(f, "{base}")
                }
            }
            Ty::Union(arms) => {
                // `T | None` prints as `T?`.
                let value: Vec<&Ty> = arms.iter().filter(|a| !a.is_none_ty()).collect();
                if value.len() == 1 && arms.len() == 2 {
                    return write!(f, "{}?", value[0]);
                }
                for (i, arm) in arms.iter().enumerate() {
                    if i > 0 {
                        write!(f, " | ")?;
                    }
                    write!(f, "{arm}")?;
                }
                Ok(())
            }
            Ty::Tuple(elems) => {
                write!(f, "(")?;
                for (i, e) in elems.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{e}")?;
                }
                write!(f, ")")
            }
            Ty::Array(elem) => write!(f, "{elem}[]"),
            Ty::Fn {
                params,
                ret,
                effects,
                ..
            } => {
                write!(f, "(")?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{p}")?;
                }
                write!(f, ")")?;
                // [fn-effects] Part of the type, so it is part of how the
                // type reads in a diagnostic.
                if !effects.is_empty() {
                    let names: Vec<String> = effects.iter().map(|e| e.to_string()).collect();
                    write!(f, " [{}]", names.join(", "))?;
                }
                write!(f, " -> {ret}")
            }
            Ty::Var(name) => write!(f, "{name}"),
            Ty::Any => write!(f, "Any"),
            Ty::Nothing => write!(f, "Nothing"),
            Ty::Unknown => write!(f, "?"),
        }
    }
}

fn fmt_args(f: &mut fmt::Formatter<'_>, args: &[Ty]) -> fmt::Result {
    if args.is_empty() {
        return Ok(());
    }
    write!(f, "<")?;
    for (i, a) in args.iter().enumerate() {
        if i > 0 {
            write!(f, ", ")?;
        }
        write!(f, "{a}")?;
    }
    write!(f, ">")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(base: Ty) -> Ty {
        base.qualify(vec![Qual {
            effect: false,
            name: "Ok".into(),
            args: vec![],
        }])
    }

    fn err(base: Ty) -> Ty {
        base.qualify(vec![Qual {
            effect: false,
            name: "Err".into(),
            args: vec![],
        }])
    }

    #[test]
    fn union_normalization() {
        let u = Ty::union_of(vec![
            Ty::named("Str"),
            Ty::union_of(vec![Ty::named("Str"), Ty::named("Int")]),
            Ty::Nothing,
        ]);
        assert_eq!(u, Ty::Union(vec![Ty::named("Str"), Ty::named("Int")]));
        assert_eq!(Ty::union_of(vec![Ty::named("Str"), Ty::named("Str")]), Ty::named("Str"));
    }

    #[test]
    fn subtyping_rules() {
        let ok_int = ok(Ty::named("Int"));
        let err_str = err(Ty::named("Str"));
        let result = Ty::union_of(vec![ok_int.clone(), err_str.clone()]);
        // Qual T <: T
        assert!(is_subtype(&ok_int, &Ty::named("Int")));
        assert!(!is_subtype(&Ty::named("Int"), &ok_int));
        // arm <: union
        assert!(is_subtype(&ok_int, &result));
        assert!(is_subtype(&result, &result));
        // Nothing <: T <: Any
        assert!(is_subtype(&Ty::Nothing, &ok_int));
        assert!(is_subtype(&result, &Ty::Any));
        // subset union <: union
        let sub = Ty::union_of(vec![err_str.clone(), ok_int.clone()]);
        assert!(is_subtype(&sub, &result));
    }

    #[test]
    fn display_forms() {
        let opt = Ty::union_of(vec![Ty::named("Str"), Ty::none()]);
        assert_eq!(opt.to_string(), "Str?");
        let res = Ty::union_of(vec![ok(Ty::named("Int")), err(Ty::named("Str"))]);
        assert_eq!(res.to_string(), "Ok Int | Err Str");
    }

    #[test]
    fn wrapper_union_detection() {
        let opt = Ty::union_of(vec![Ty::named("Str"), Ty::none()]);
        assert!(!opt.is_wrapper_union());
        let res = Ty::union_of(vec![Ty::named("Str"), Ty::named("Int"), Ty::none()]);
        assert!(res.is_wrapper_union());
        assert!(res.has_none_arm());
        assert_eq!(res.value_arms().len(), 2);
    }

    /// [fn-overload-rank] The specificity ladder, rung by rung: a type
    /// variable is the least specific thing a parameter can say, `Any` is the
    /// broadest type, a narrower union says more, and more qualifiers say
    /// more.
    #[test]
    fn specificity_ladder() {
        use std::cmp::Ordering::*;
        let var = Ty::Var("T".into());
        let int = Ty::named("Int");
        let str_ = Ty::named("Str");
        // 1. A type variable knows nothing.
        assert_eq!(spec_cmp(&int, &var), Some(Greater));
        assert_eq!(spec_cmp(&var, &int), Some(Less));
        assert_eq!(spec_cmp(&var, &Ty::Var("U".into())), Some(Equal));
        let list = |arg: Ty| Ty::Named {
            name: "List".into(),
            args: vec![arg],
        };
        assert_eq!(spec_cmp(&list(int.clone()), &list(var.clone())), Some(Greater));
        // 2. `Any` accepts everything, so nothing is broader.
        assert_eq!(spec_cmp(&int, &Ty::Any), Some(Greater));
        assert_eq!(spec_cmp(&Ty::Any, &var), Some(Greater), "a variable is vaguer still");
        // 3. Unions by arm inclusion: an arm beats its union beats a broader
        // one, and `T` beats `T?`.
        let both = Ty::union_of(vec![int.clone(), str_.clone()]);
        let three = Ty::union_of(vec![int.clone(), str_.clone(), Ty::named("Bool")]);
        assert_eq!(spec_cmp(&int, &both), Some(Greater));
        assert_eq!(spec_cmp(&both, &three), Some(Greater));
        assert_eq!(spec_cmp(&three, &int), Some(Less));
        let opt = Ty::union_of(vec![int.clone(), Ty::none()]);
        assert_eq!(spec_cmp(&int, &opt), Some(Greater));
        // Same size, neither a subset: unrankable.
        let other = Ty::union_of(vec![int.clone(), Ty::named("Bool")]);
        assert_eq!(spec_cmp(&both, &other), None);
        // 4. Qualifier sets by inclusion, kind ignored.
        let mut_list = list(int.clone()).qualify(vec![Qual {
            effect: false,
            name: "Mut".into(),
            args: vec![],
        }]);
        let mut_ne_list = mut_list.clone().qualify(vec![Qual {
            effect: false,
            name: "NonEmpty".into(),
            args: vec![],
        }]);
        let ne_list = list(int.clone()).qualify(vec![Qual {
            effect: false,
            name: "NonEmpty".into(),
            args: vec![],
        }]);
        assert_eq!(spec_cmp(&mut_list, &list(int.clone())), Some(Greater));
        assert_eq!(spec_cmp(&mut_ne_list, &mut_list), Some(Greater));
        // Different single qualifiers: the *kind* does not rank, so this is
        // an ambiguity for the caller to settle with a rename.
        assert_eq!(spec_cmp(&mut_list, &ne_list), None);
        // Criteria pulling opposite ways are unrankable: more qualifiers but
        // a vaguer base.
        let ne_generic = list(var.clone()).qualify(vec![Qual {
            effect: false,
            name: "NonEmpty".into(),
            args: vec![],
        }]);
        assert_eq!(spec_cmp(&ne_generic, &list(int.clone())), None);
        // Agreeing criteria compose, though.
        assert_eq!(spec_cmp(&ne_list, &list(var.clone())), Some(Greater));
        // 5. [proj-type] `Proj` inverts: `X <: Proj X`, so a `Proj`
        // position accepts owned values too — an owned position says more.
        let proj = |ty: Ty| {
            ty.qualify(vec![Qual {
                effect: false,
                name: "Proj".into(),
                args: vec![],
            }])
        };
        assert_eq!(spec_cmp(&str_, &proj(str_.clone())), Some(Greater));
        assert_eq!(spec_cmp(&proj(str_.clone()), &str_), Some(Less));
        // Structurally too: `List<Str>` beats `List<Proj Str>`.
        assert_eq!(
            spec_cmp(&list(str_.clone()), &list(proj(str_.clone()))),
            Some(Greater)
        );
        // Its own dimension: more qualifiers but less ownership is
        // unrankable, not a guess.
        let proj_mut_str = proj(str_.clone()).qualify(vec![Qual {
            effect: false,
            name: "Mut".into(),
            args: vec![],
        }]);
        assert_eq!(spec_cmp(&proj_mut_str, &str_), None);
        // Agreeing dimensions compose: `Mut Str` beats `Proj Mut Str`.
        let mut_str = str_.clone().qualify(vec![Qual {
            effect: false,
            name: "Mut".into(),
            args: vec![],
        }]);
        assert_eq!(spec_cmp(&mut_str, &proj_mut_str), Some(Greater));
    }

    fn ranked(patterns: Vec<Ty>) -> RankedCandidate {
        RankedCandidate {
            patterns,
            variadic: false,
        }
    }

    /// [fn-overload-rank] Candidates compare **per argument slot**, and a
    /// winner has to be at least as specific everywhere: one argument's gain
    /// never pays for another's loss.
    #[test]
    fn ranking_is_per_slot() {
        let var = Ty::Var("T".into());
        let int = Ty::named("Int");
        let concrete = ranked(vec![int.clone(), int.clone()]);
        let half = ranked(vec![var.clone(), int.clone()]);
        assert!(spec_dominates(&concrete, &half));
        assert!(!spec_dominates(&half, &concrete));
        // Mutually unrankable: each is more specific in one slot.
        let a = ranked(vec![int.clone(), var.clone()]);
        let b = ranked(vec![var.clone(), int.clone()]);
        assert!(!spec_dominates(&a, &b) && !spec_dominates(&b, &a));
        assert_eq!(most_specific(&[a, b]), None, "no winner is an ambiguity");
        assert_eq!(most_specific(&[concrete.clone(), half]), Some(0));
        assert_eq!(most_specific(&[concrete]), Some(0));
    }

    /// [fn-overload-rank] With the slots equal, a **fixed** parameter list
    /// beats a variadic one — which is what lets `list()` pick the
    /// no-argument overload over `list(...elems)`.
    #[test]
    fn fixed_beats_variadic() {
        let empty_fixed = RankedCandidate {
            patterns: vec![],
            variadic: false,
        };
        let empty_variadic = RankedCandidate {
            patterns: vec![],
            variadic: true,
        };
        assert!(spec_dominates(&empty_fixed, &empty_variadic));
        assert!(!spec_dominates(&empty_variadic, &empty_fixed));
        assert_eq!(most_specific(&[empty_variadic, empty_fixed]), Some(1));
    }
    /// [fn-effects] The two rules an effect claim does *not* share with an
    /// ordinary qualifier: it is never dropped, and its variance is inverted —
    /// a producer performing fewer effects fits where more are expected.
    #[test]
    fn an_effect_claim_never_drops_and_inverts() {
        let iter = Ty::Named {
            name: "Iter".to_string(),
            args: vec![Ty::Named {
                name: "Int".to_string(),
                args: vec![],
            }],
        };
        let claimed = iter.clone().qualify(vec![Qual::effect("Logger", vec![])]);
        assert!(is_subtype(&iter, &claimed), "pure fits a claiming position");
        assert!(!is_subtype(&claimed, &iter), "a claim does not drop");
    }
}

