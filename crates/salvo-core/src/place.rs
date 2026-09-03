//! Places: the keys of flow analysis [flow-place].
//!
//! A *place* names a storage location reachable from a local variable: the
//! variable itself (`h`), or a projection out of it (`h.field`,
//! `h.a.b`). Flow facts — narrowed type today [is-narrowing], ownership
//! later — are keyed by place rather than by variable name, so a fact
//! about `h.field` can be recorded, invalidated, and merged independently
//! of `h`.
//!
//! The two relations the analysis needs are *prefix* (does an event on
//! this place reach that one?) and *overlap* (could the two name the same
//! storage?). Both are conservative: when a projection step cannot be
//! resolved statically — an array index — it may alias any element, so it
//! overlaps every element step at that position.

use std::fmt;

use salvo_syntax::ast::Expr;

/// One projection step of a place path.
///
/// [`Proj::Field`] and [`Proj::Index`] steps narrow
/// ([`Place::narrowable`], user decision on P1a) — both name one storage
/// location statically. [`Proj::Element`] does not, and exists so
/// place-based *ownership* (roadmap L5) can name array elements too.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Proj {
    /// `.name` — a struct field.
    Field(String),
    /// A constant element index: a tuple position, written `t.0`
    /// [expr-tuple-index]. Statically identified, so it aliases only the
    /// same index.
    Index(usize),
    /// An element at a statically unknown index (`arr[i]`). May alias any
    /// element of its base.
    Element,
}

impl Proj {
    /// Whether two steps out of the *same* base may name the same storage.
    /// Conservative for unknown indices: `arr[i]` may hit any element.
    pub fn may_alias(&self, other: &Proj) -> bool {
        match (self, other) {
            (Proj::Field(a), Proj::Field(b)) => a == b,
            (Proj::Index(a), Proj::Index(b)) => a == b,
            // An unknown index may land on any element position; a field
            // and an element never share storage.
            (Proj::Element, Proj::Element | Proj::Index(_))
            | (Proj::Index(_), Proj::Element) => true,
            (Proj::Field(_), _) | (_, Proj::Field(_)) => false,
        }
    }
}

impl fmt::Display for Proj {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Proj::Field(name) => write!(f, ".{name}"),
            Proj::Index(i) => write!(f, ".{i}"),
            Proj::Element => write!(f, "[?]"),
        }
    }
}

/// A place: a local variable root plus the projection path out of it. The
/// root is a *name* because that is what the checker's scope frames are
/// keyed by; a place with an empty path is the variable itself.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Place {
    pub root: String,
    pub path: Vec<Proj>,
}

impl Place {
    /// The place of a whole variable.
    pub fn root(name: impl Into<String>) -> Place {
        Place {
            root: name.into(),
            path: Vec::new(),
        }
    }

    /// The place an expression denotes, or `None` when the expression is
    /// not a place rooted in a variable (a call result, a literal, an
    /// operator application — nothing the analysis can track).
    pub fn of_expr(expr: &Expr) -> Option<Place> {
        match expr {
            Expr::Ident(id) => Some(Place::root(id.name.clone())),
            Expr::Field { base, field, .. } => {
                let mut place = Place::of_expr(base)?;
                place.path.push(Proj::Field(field.name.clone()));
                Some(place)
            }
            Expr::TupleIndex { base, index, .. } => {
                let mut place = Place::of_expr(base)?;
                place.path.push(Proj::Index(*index));
                Some(place)
            }
            Expr::Index { base, .. } => {
                let mut place = Place::of_expr(base)?;
                // Salvo has no constant-index projection syntax, so every
                // `Index` expression is an array subscript with a
                // statically unknown index.
                place.path.push(Proj::Element);
                Some(place)
            }
            _ => None,
        }
    }

    /// Whether this place is the whole variable (no projection).
    pub fn is_root(&self) -> bool {
        self.path.is_empty()
    }

    /// Whether flow *narrowing* may be keyed by this place: field and
    /// constant-index steps (P1a), in any combination — `p.pair.0.name`
    /// names exactly one location. An `Element` step never does (`arr[i]`
    /// cannot be told from `arr[j]`), so a path containing one does not
    /// narrow.
    pub fn narrowable(&self) -> bool {
        self.path
            .iter()
            .all(|p| matches!(p, Proj::Field(_) | Proj::Index(_)))
    }

    /// Whether an event on `self` reaches `other`: same root, and `self`'s
    /// path is a (may-alias) prefix of `other`'s. Reflexive — a place is a
    /// prefix of itself — so mutating `h.a` invalidates facts about `h.a`
    /// and `h.a.b`, and mutating `h` invalidates facts about every place
    /// rooted in it.
    pub fn is_prefix_of(&self, other: &Place) -> bool {
        self.root == other.root
            && self.path.len() <= other.path.len()
            && self
                .path
                .iter()
                .zip(&other.path)
                .all(|(a, b)| a.may_alias(b))
    }

    /// Whether the two places could name the same storage: either is a
    /// prefix of the other. Writing `h.a` overlaps a fact about `h.a.b`
    /// (it replaces the whole subtree) and a fact about `h` (it mutates
    /// part of it), but not one about `h.c`.
    pub fn overlaps(&self, other: &Place) -> bool {
        self.is_prefix_of(other) || other.is_prefix_of(self)
    }
}

impl fmt::Display for Place {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.root)?;
        for p in &self.path {
            write!(f, "{p}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn place(root: &str, path: &[Proj]) -> Place {
        Place {
            root: root.to_string(),
            path: path.to_vec(),
        }
    }

    fn field(name: &str) -> Proj {
        Proj::Field(name.to_string())
    }

    /// [flow-place] A place is a prefix of itself and of anything below
    /// it; siblings are unrelated.
    #[test]
    fn prefix_is_reflexive_and_downward() {
        let h = place("h", &[]);
        let ha = place("h", &[field("a")]);
        let hab = place("h", &[field("a"), field("b")]);
        let hc = place("h", &[field("c")]);

        assert!(h.is_prefix_of(&h));
        assert!(h.is_prefix_of(&hab));
        assert!(ha.is_prefix_of(&hab));
        assert!(!hab.is_prefix_of(&ha));
        assert!(!ha.is_prefix_of(&hc));
        assert!(!hc.is_prefix_of(&ha));
    }

    /// [flow-place] Different roots never relate, even with equal paths.
    #[test]
    fn different_roots_never_relate() {
        let h = place("h", &[field("a")]);
        let g = place("g", &[field("a")]);
        assert!(!h.is_prefix_of(&g));
        assert!(!h.overlaps(&g));
    }

    /// [flow-place] Overlap is symmetric and holds in both directions of
    /// the prefix relation.
    #[test]
    fn overlap_is_symmetric() {
        let ha = place("h", &[field("a")]);
        let hab = place("h", &[field("a"), field("b")]);
        let hc = place("h", &[field("c")]);

        assert!(ha.overlaps(&hab));
        assert!(hab.overlaps(&ha));
        assert!(!ha.overlaps(&hc));
    }

    /// [flow-place] An unknown array index may alias any element, so
    /// `arr[?]` overlaps every element place — including a constant one.
    #[test]
    fn unknown_index_aliases_every_element() {
        let elem = place("arr", &[Proj::Element]);
        let zero = place("arr", &[Proj::Index(0)]);
        let one = place("arr", &[Proj::Index(1)]);

        assert!(elem.overlaps(&zero));
        assert!(elem.overlaps(&one));
        assert!(elem.overlaps(&elem));
        // Constant indices are distinguishable from each other.
        assert!(!zero.overlaps(&one));
        // A field never shares storage with an element.
        assert!(!elem.overlaps(&place("arr", &[field("a")])));
    }

    /// [flow-place] Field chains and constant indices narrow (P1a);
    /// unknown-index element steps do not.
    #[test]
    fn narrowable_is_statically_identified_paths() {
        assert!(place("h", &[]).narrowable());
        assert!(place("h", &[field("a"), field("b")]).narrowable());
        assert!(place("h", &[Proj::Index(0)]).narrowable());
        assert!(place("h", &[field("a"), Proj::Index(1), field("b")]).narrowable());
        assert!(!place("h", &[Proj::Element]).narrowable());
        assert!(!place("h", &[field("a"), Proj::Element]).narrowable());
    }
}
