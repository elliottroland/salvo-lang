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
    pub quals: Vec<String>,
    pub mutable: bool,
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

    pub fn quals(&self) -> &[Qual] {
        match self {
            Ty::Qualified { quals, .. } => quals,
            _ => &[],
        }
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
        quals.sort_by(|a, b| a.name.cmp(&b.name));
        quals.dedup();
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
        (Ty::Qualified { base, .. }, _) if matches!(**base, Ty::Union(_)) => {
            if let Ty::Union(arms) = b {
                if arms.iter().any(|arm| a == arm) {
                    return true;
                }
            }
            is_subtype(base, b)
        }
        // A non-union is a subtype of a union when it fits some arm.
        (_, Ty::Union(arms)) => arms.iter().any(|arm| is_subtype(a, arm)),
        (
            Ty::Qualified { quals: qa, base: ba },
            Ty::Qualified { quals: qb, base: bb },
        ) => {
            is_subtype(ba, bb)
                && qb
                    .iter()
                    .all(|q| q.name == "Once" || qa.contains(q))
        }
        // [once-fn] INVERTED subtyping, flagged for future review
        // (user decision 2026-09-02): `Once` *restricts* (callable at
        // most once) instead of refining, so a plain fn may be used
        // where a `Once` fn is expected — the opposite direction of
        // every other qualifier.
        (Ty::Fn { .. }, Ty::Qualified { quals, base })
            if quals.iter().any(|q| q.name == "Once")
                && matches!(**base, Ty::Fn { .. }) =>
        {
            is_subtype(a, base)
        }
        // `Qual T <: T` — except `Once`, which may never be dropped
        // (a once-callable fn is not a many-callable fn) [once-fn].
        (Ty::Qualified { quals, base }, _) => {
            !quals.iter().any(|q| q.name == "Once") && is_subtype(base, b)
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
            Ty::Fn { params: pa, ret: ra, contract: ca },
            Ty::Fn { params: pb, ret: rb, contract: cb },
        ) => {
            pa.len() == pb.len()
                && pa.iter().zip(pb).all(|(x, y)| compatible(x, y))
                && is_subtype(ra, rb)
                && contract_fits(ca.as_deref(), cb.as_deref(), pa.len())
        }
        _ => false,
    }
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

/// Invariant compatibility (used for generic arguments).
pub fn compatible(a: &Ty, b: &Ty) -> bool {
    is_subtype(a, b) && is_subtype(b, a)
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
                for q in quals {
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
            Ty::Fn { params, ret, .. } => {
                write!(f, "(")?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{p}")?;
                }
                write!(f, ") -> {ret}")
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
            name: "Ok".into(),
            args: vec![],
        }])
    }

    fn err(base: Ty) -> Ty {
        base.qualify(vec![Qual {
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
}
