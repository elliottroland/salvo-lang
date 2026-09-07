//! Qualifier refinements [qual-refn].
//!
//! A deduction list is written by the function's author, so it can only
//! promise what that author knows. `add(list: Mut List<T>, elem: T)` mutates
//! its list, and [deduce-syntax] therefore forbids it from promising a
//! caller's `NonEmpty` back — even though appending to a list can never
//! empty it. The function cannot state the fact: it has never heard of
//! `NonEmpty`.
//!
//! A refinement lets the *claim's owner* state it instead:
//!
//! ```text
//! qualifier NonEmpty<T> of List<T> {
//!     fn qualifies(list: List<T>) -> Bool { return list.size() > 0 }
//!
//!     // Adding to a list makes it non-empty.
//!     refn add(list: Mut List<T>, elem: T) -> [list: +NonEmpty]
//! }
//! ```
//!
//! What that buys, and what it costs:
//!
//! * A refinement changes nothing about what the function *does* — it has
//!   no body, no effects and no return type, and its deduction entries can
//!   only add and remove state qualifiers [qual-refn]. So it is invisible
//!   to both backends: qualifiers are erased [qual-erasure], and the
//!   capability qualifiers that are *not* erased (`Mut` above all) may not
//!   be named.
//! * It is **trusted**, exactly like `-> T as Q` [qual-ctor-fn]: nothing
//!   proves that `add` establishes `NonEmpty`. The qualifier author owns
//!   the claim's meaning, which is the whole reason this is the right party
//!   to ask.
//! * It travels with its qualifier [qual-refn-scope]: opting into
//!   `NonEmpty` opts into what `NonEmpty` knows. A top-level `refn` is
//!   module-scoped and not importable.
//! * Two qualifiers can disagree, and then **neither applies**
//!   [qual-refn-conflict] — no error, just a less useful function, plus a
//!   warning at the call site so the silence is discoverable.
//!
//! This module resolves each refinement to the overload it refines
//! [qual-refn-match], validates it, and computes the per-file table the
//! checker and the deduction pass read.

use std::collections::{HashMap, HashSet};

use salvo_syntax::ast::{
    Deduction, DeductionKind, Ident, Item, Param, QualSubject, QualifierDecl, RefnDecl,
    Type, TypeRef,
};
use salvo_syntax::Span;

use crate::diag::FileDiagnostic;
use crate::program::Program;
use crate::resolve::{FnKey, Resolution};

/// Where one refinement of a group came from, for diagnostics and for the
/// merged documentation [qual-refn-docs].
#[derive(Clone, Debug, PartialEq)]
pub struct RefnSource {
    /// The qualifier that declared it, or `None` for a top-level `refn`.
    pub qualifier: Option<String>,
    /// The refinement's own doc comment [doc-comment].
    pub docs: Vec<String>,
    /// Where the refinement is written (file index and the span of its
    /// name), so tooling can point at it.
    pub file: usize,
    pub span: Span,
}

/// The refinements that apply to one parameter of one call
/// [qual-refn-conflict]: the merged additions and removals, the
/// refinements they came from, and — when applicable refinements
/// disagree — the qualifier names that cannot co-apply. A conflict leaves
/// `add` and `remove` empty: none of them apply.
#[derive(Clone, Debug, PartialEq)]
pub struct RefnGroup {
    pub param: String,
    pub add: Vec<String>,
    pub remove: Vec<String>,
    pub sources: Vec<RefnSource>,
    /// Empty unless the applicable refinements conflict.
    pub conflict: Vec<String>,
}

impl RefnGroup {
    /// Whether the group changes anything at a call site.
    pub fn is_empty(&self) -> bool {
        self.add.is_empty() && self.remove.is_empty()
    }
}

/// Every refinement that applies, keyed by *where the call is written* and
/// *which overload it resolves to* — refinement visibility is per file
/// [qual-refn-scope], so the same callee refines differently in two files.
#[derive(Clone, Debug, Default)]
pub struct Refinements {
    groups: HashMap<(usize, FnKey), Vec<RefnGroup>>,
    /// Validation diagnostics from `collect`, replayed by the checker so
    /// they land in the same output as everything else [diag-structured].
    pub errors: Vec<FileDiagnostic>,
}

impl Refinements {
    /// The refinement groups applying to a call to `callee` written in
    /// `file` (empty when there are none).
    pub fn for_call(&self, file: usize, callee: FnKey) -> &[RefnGroup] {
        self.groups
            .get(&(file, callee))
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// The group applying to one parameter of such a call.
    pub fn for_param(&self, file: usize, callee: FnKey, param: &str) -> Option<&RefnGroup> {
        self.for_call(file, callee).iter().find(|g| g.param == param)
    }

    pub fn is_empty(&self) -> bool {
        self.groups.is_empty()
    }
}

/// Whether two qualifiers may apply to one value [qual-with]. Shared with
/// the checker's declaration-site validation so the two cannot drift: a
/// refinement conflict is *precisely* "these two could not have been
/// written together".
pub fn quals_compatible(a: &QualifierDecl, b: &QualifierDecl) -> bool {
    if a.name.name == b.name.name {
        return true;
    }
    // [qual-subject] Provenance composes with everything; so do the
    // compiler's own qualifiers.
    if a.subject == QualSubject::Provenance || b.subject == QualSubject::Provenance {
        return true;
    }
    if a.intrinsic || b.intrinsic {
        return true;
    }
    a.with.iter().any(|w| w.name.name == b.name.name)
        || b.with.iter().any(|w| w.name.name == a.name.name)
}

/// One validated refinement, before visibility is applied.
struct Resolved<'p> {
    decl: &'p RefnDecl,
    /// The declaring qualifier, or `None` for a top-level `refn`.
    qualifier: Option<&'p QualifierDecl>,
    /// The file the refinement is *written* in.
    file: usize,
    /// The overload it refines [qual-refn-match].
    callee: FnKey,
    entries: Vec<(String, Vec<String>, Vec<String>)>,
}

/// Resolves, validates and indexes every refinement in the program.
pub fn collect<'p>(program: &'p Program, resolution: &Resolution<'p>) -> Refinements {
    let mut errors: Vec<FileDiagnostic> = Vec::new();
    let mut resolved: Vec<Resolved<'p>> = Vec::new();

    for (file_idx, ast) in program.modules.iter().enumerate() {
        for item in &ast.items {
            match item {
                Item::Refn(r) => {
                    if let Some(res) =
                        resolve_refn(resolution, file_idx, r, None, &mut errors)
                    {
                        resolved.push(res);
                    }
                }
                Item::Qualifier(q) => {
                    for r in &q.refns {
                        if let Some(res) =
                            resolve_refn(resolution, file_idx, r, Some(q), &mut errors)
                        {
                            resolved.push(res);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    // Visibility, then merging: for each file, which refinements apply,
    // grouped per (callee, parameter) [qual-refn-conflict].
    let mut groups: HashMap<(usize, FnKey), Vec<RefnGroup>> = HashMap::new();
    for (file_idx, (file, _)) in program.files.iter().zip(&program.modules).enumerate() {
        let scope = &resolution.scopes[file_idx];
        // One contribution per (callee, param, refinement), tagged with
        // whether it came from a top-level `refn`.
        let mut contributions: HashMap<(FnKey, String), Vec<(bool, RefnSource, Vec<String>, Vec<String>)>> =
            HashMap::new();
        for res in &resolved {
            let visible = match res.qualifier {
                // A qualifier's refinements are in scope wherever the
                // qualifier is [qual-refn-scope] — the *same*
                // declaration, not merely one of that name.
                Some(q) => scope
                    .qualifiers
                    .get(q.name.name.as_str())
                    .is_some_and(|d| std::ptr::eq(*d, q)),
                // A top-level refinement is module-scoped and not
                // importable.
                None => program.files[res.file].module == file.module,
            };
            if !visible {
                continue;
            }
            let source = RefnSource {
                qualifier: res.qualifier.map(|q| q.name.name.clone()),
                docs: res.decl.docs.clone(),
                file: res.file,
                span: res.decl.name.span,
            };
            for (param, add, remove) in &res.entries {
                contributions
                    .entry((res.callee, param.clone()))
                    .or_default()
                    .push((
                        res.qualifier.is_none(),
                        source.clone(),
                        add.clone(),
                        remove.clone(),
                    ));
            }
        }
        for ((callee, param), mut list) in contributions {
            // [qual-refn-reconcile] A top-level `refn` *replaces* the
            // qualifiers' own refinements for that parameter. That is what
            // makes reconciling a conflict possible at all: joining them
            // would keep the disagreement, so the consumer's own module
            // has the last word — the same precedence own-module
            // declarations have over imported ones [mod-collision].
            if list.iter().any(|(top, ..)| *top) {
                list.retain(|(top, ..)| *top);
            }
            let mut group = RefnGroup {
                param,
                add: Vec::new(),
                remove: Vec::new(),
                sources: Vec::new(),
                conflict: Vec::new(),
            };
            for (_, source, add, remove) in list {
                for q in add {
                    if !group.add.contains(&q) {
                        group.add.push(q);
                    }
                }
                for q in remove {
                    if !group.remove.contains(&q) {
                        group.remove.push(q);
                    }
                }
                if !group.sources.contains(&source) {
                    group.sources.push(source);
                }
            }
            // [qual-refn-conflict] Suppress a group whose members
            // disagree. Two additions conflict when the qualifiers could
            // not have been written together [qual-with]; an addition and
            // a removal of the same qualifier conflict outright. Nothing
            // is applied then — the caller can still test by hand, or
            // reconcile with a top-level `refn`. Judged in *this* file's
            // scope, since that is where the refinements are visible.
            let mut conflict: Vec<String> = Vec::new();
            for (i, a) in group.add.iter().enumerate() {
                for b in group.add.iter().skip(i + 1) {
                    let compatible = match (
                        scope.qualifiers.get(a.as_str()),
                        scope.qualifiers.get(b.as_str()),
                    ) {
                        (Some(x), Some(y)) => quals_compatible(x, y),
                        // A qualifier this file cannot see cannot be
                        // judged; assume compatible rather than
                        // suppressing on missing information.
                        _ => true,
                    };
                    if !compatible {
                        conflict.push(a.clone());
                        conflict.push(b.clone());
                    }
                }
            }
            for q in &group.add {
                if group.remove.contains(q) {
                    conflict.push(q.clone());
                }
            }
            conflict.sort();
            conflict.dedup();
            if !conflict.is_empty() {
                group.add.clear();
                group.remove.clear();
                group.conflict = conflict;
            }
            if !group.is_empty() || !group.conflict.is_empty() {
                groups.entry((file_idx, callee)).or_default().push(group);
            }
        }
    }
    // Deterministic order per call: the table is read for diagnostics and
    // for hover, and a hash-map traversal built it.
    for list in groups.values_mut() {
        list.sort_by(|a, b| a.param.cmp(&b.param));
    }
    groups.retain(|_, v| !v.is_empty());

    Refinements { groups, errors }
}

/// Resolves one refinement to the overload it refines and validates it
/// [qual-refn-match].
fn resolve_refn<'p>(
    resolution: &Resolution<'p>,
    file_idx: usize,
    decl: &'p RefnDecl,
    qualifier: Option<&'p QualifierDecl>,
    errors: &mut Vec<FileDiagnostic>,
) -> Option<Resolved<'p>> {
    let scope = &resolution.scopes[file_idx];
    let name = decl.name.name.as_str();
    // Type parameters in scope for the refinement's signature: the
    // qualifier's first (a refinement inside `NonEmpty<T>` writes that
    // `T`), then the refinement's own. Matching is positional, so a
    // refinement and the fn it refines need not agree on the *names*
    // [qual-refn-match].
    let mut generics: Vec<&str> = Vec::new();
    if let Some(q) = qualifier {
        generics.extend(q.generics.iter().map(|g| g.name.as_str()));
    }
    generics.extend(decl.generics.iter().map(|g| g.name.as_str()));
    let want = signature(&decl.params, &generics);

    // A name that is neither a type parameter nor a visible type is the
    // most likely reason a refinement fails to match — a top-level `refn`
    // has no qualifier to borrow `T` from — and reporting it as a shape
    // mismatch would send the author looking at the wrong thing.
    let mut unbound: Vec<&Ident> = Vec::new();
    for p in &decl.params {
        base_names(&p.ty, &mut unbound);
    }
    let mut reported: HashSet<&str> = HashSet::new();
    let mut any_unbound = false;
    for id in unbound {
        let name = id.name.as_str();
        if generics.contains(&name)
            || scope.structs.contains_key(name)
            || scope.opaque_types.contains_key(name)
            || scope.type_aliases.contains_key(name)
        {
            continue;
        }
        any_unbound = true;
        if !reported.insert(name) {
            continue;
        }
        errors.push(FileDiagnostic::error(
            file_idx,
            id.span,
            format!(
                "unknown type `{name}` in this refinement's parameter list: if it \
                 is a type parameter of the function being refined, declare it \
                 here too (`refn {}<{name}>(...)`)",
                decl.name.name
            ),
        ));
    }
    if any_unbound {
        return None;
    }

    let candidates = scope.fns.get(name).map(|v| v.as_slice()).unwrap_or(&[]);
    if candidates.is_empty() {
        errors.push(FileDiagnostic::error(
            file_idx,
            decl.name.span,
            format!(
                "no function `{name}` is visible here, so there is nothing to \
                 refine: a refinement names an existing function and repeats its \
                 parameter list to pick one overload"
            ),
        ));
        return None;
    }
    let matched: Vec<&crate::resolve::FnEntry<'p>> = candidates
        .iter()
        .filter(|c| {
            let mut cg: Vec<&str> = c.decl.generics.iter().map(|g| g.name.as_str()).collect();
            // A `canbe` clause names a type parameter too.
            for (g, _) in &c.decl.generic_canbe {
                if !cg.contains(&g.name.as_str()) {
                    cg.push(g.name.as_str());
                }
            }
            signature(&c.decl.params, &cg) == want
        })
        .collect();
    let entry = match matched.as_slice() {
        [one] => *one,
        [] => {
            let shapes: Vec<String> = candidates
                .iter()
                .map(|c| {
                    let params: Vec<String> = c
                        .decl
                        .params
                        .iter()
                        .map(|p| format!("{}: {}", p.name.name, p.ty))
                        .collect();
                    format!("`{name}({})`", params.join(", "))
                })
                .collect();
            errors.push(FileDiagnostic::error(
                file_idx,
                decl.name.span,
                format!(
                    "this refinement matches no `{name}` in scope: the parameter \
                     list must repeat one overload's parameters exactly — same \
                     names, same types (type parameters match by position). In \
                     scope: {}",
                    shapes.join(", ")
                ),
            ));
            return None;
        }
        _ => {
            errors.push(FileDiagnostic::error(
                file_idx,
                decl.name.span,
                format!("this refinement matches more than one `{name}` in scope"),
            ));
            return None;
        }
    };

    let mut entries: Vec<(String, Vec<String>, Vec<String>)> = Vec::new();
    let mut seen: HashSet<&str> = HashSet::new();
    for e in &decl.deductions {
        let param = e.param.name.as_str();
        if !seen.insert(param) {
            errors.push(FileDiagnostic::error(
                file_idx,
                e.span,
                format!("`{param}` appears twice in this refinement's deductions"),
            ));
            continue;
        }
        let Some(p) = entry.decl.params.iter().find(|p| p.name.name == param) else {
            errors.push(FileDiagnostic::error(
                file_idx,
                e.span,
                format!("`{name}` has no parameter `{param}`"),
            ));
            continue;
        };
        if e.add.is_empty() && e.remove.is_empty() {
            errors.push(FileDiagnostic::error(
                file_idx,
                e.span,
                format!(
                    "`{param}` states nothing: write `+Qual` if the call \
                     establishes a qualifier, `-Qual` if it invalidates one"
                ),
            ));
            continue;
        }
        // A refinement only ever speaks about a parameter the call gives
        // back: nothing is known about a moved one afterwards. Checked
        // against a *written* list only — an inferred one is not settled
        // yet at this point, and a moved parameter's group simply never
        // fires (the call-site loop applies refinements on the kept path).
        if let Some(list) = &entry.decl.deductions {
            if !kept_in(list, param) {
                errors.push(FileDiagnostic::error(
                    file_idx,
                    e.span,
                    format!(
                        "`{name}` moves `{param}`, so a caller knows nothing about \
                         it afterwards — there is nothing to refine"
                    ),
                ));
                continue;
            }
        }
        let mut add: Vec<String> = Vec::new();
        let mut remove: Vec<String> = Vec::new();
        for (refs, out, verb) in [
            (&e.add, &mut add, "establish"),
            (&e.remove, &mut remove, "invalidate"),
        ] {
            for r in refs {
                if let Some(q) = check_refined_qual(
                    file_idx, decl, qualifier, scope, r, p, verb, errors,
                ) {
                    if !out.contains(&q) {
                        out.push(q);
                    }
                }
            }
        }
        for q in &add {
            if remove.contains(q) {
                errors.push(FileDiagnostic::error(
                    file_idx,
                    e.span,
                    format!(
                        "this refinement both establishes and invalidates `{q}` for \
                         `{param}`"
                    ),
                ));
            }
        }
        if !add.is_empty() || !remove.is_empty() {
            entries.push((param.to_string(), add, remove));
        }
    }
    if entries.is_empty() {
        return None;
    }
    Some(Resolved {
        decl,
        qualifier,
        file: file_idx,
        callee: entry.key,
        entries,
    })
}

/// Validates one qualifier named by a refinement entry, returning its name
/// when it may be used [qual-refn].
#[allow(clippy::too_many_arguments)]
fn check_refined_qual(
    file_idx: usize,
    decl: &RefnDecl,
    qualifier: Option<&QualifierDecl>,
    scope: &crate::resolve::ModuleScope<'_>,
    r: &TypeRef,
    param: &Param,
    verb: &str,
    errors: &mut Vec<FileDiagnostic>,
) -> Option<String> {
    let q = r.name.name.as_str();
    // A refinement inside a qualifier may only speak about *its own*
    // claim [qual-refn]. That is what makes "opting into a qualifier opts
    // into its refinements" honest — a qualifier cannot restate someone
    // else's claim — and it is why conflicts reduce to two qualifiers
    // that cannot co-apply.
    if let Some(owner) = qualifier {
        if q != owner.name.name {
            errors.push(FileDiagnostic::error(
                file_idx,
                r.span,
                format!(
                    "a refinement declared by `{}` can only {verb} `{}`, not \
                     `{q}`: a qualifier states what happens to *its own* claim. \
                     Write a top-level `refn` to reconcile several qualifiers in \
                     your own module",
                    owner.name.name, owner.name.name
                ),
            ));
            return None;
        }
    }
    let Some(qd) = scope.qualifiers.get(q) else {
        errors.push(FileDiagnostic::error(
            file_idx,
            r.span,
            format!("unknown qualifier `{q}`"),
        ));
        return None;
    };
    // Only *state* qualifiers [qual-subject]. Provenance cannot be
    // invalidated (so there is nothing to re-establish) and is minted by
    // constructors; the compiler's own qualifiers carry representation
    // choices and flow rules — `Mut` is not even erased — so a refinement
    // must not hand one out.
    if qd.intrinsic {
        errors.push(FileDiagnostic::error(
            file_idx,
            r.span,
            format!(
                "`{q}` is one of the compiler's own qualifiers: a refinement may \
                 only {verb} a state qualifier, since it changes what is *known* \
                 about a value and not what may be done with it"
            ),
        ));
        return None;
    }
    if qd.subject == QualSubject::Provenance {
        errors.push(FileDiagnostic::error(
            file_idx,
            r.span,
            format!(
                "`{q}` is a provenance qualifier, which no call can invalidate \
                 [qual-subject] — so there is nothing for a refinement to say \
                 about it; only state qualifiers can be refined"
            ),
        ));
        return None;
    }
    // The claim has to be about the parameter's type: the qualifier's `of`
    // type must accept it [qual-of]. Compared by base name, which is what
    // this pass can see without the checker's unification — a generic `of`
    // (`qualifier Ok<T> of T`) accepts anything.
    if let (Some(of), Some(base)) = (of_base(&qd.of, &qd.generics), param_base(&param.ty)) {
        if of != base {
            errors.push(FileDiagnostic::error(
                file_idx,
                r.span,
                format!(
                    "`{q}` applies to `{}`, not to `{}`: `{}` is declared `{}: {}`",
                    qd.of, base, decl.name.name, param.name.name, param.ty
                ),
            ));
            return None;
        }
    }
    Some(q.to_string())
}

/// Whether a written deduction list gives `param` back to the caller
/// [deduce-syntax].
fn kept_in(list: &[Deduction], param: &str) -> bool {
    list.iter()
        .any(|d| d.param.name == param && !matches!(d.kind, DeductionKind::Moved))
}

/// Every *base* type name a written type mentions (qualifier positions
/// excluded: `Mut` and friends are the language's, not declarations).
fn base_names<'t>(ty: &'t Type, out: &mut Vec<&'t Ident>) {
    match ty {
        Type::Named { base, .. } => {
            out.push(&base.name);
            for a in &base.args {
                base_names(a, out);
            }
        }
        Type::QualifiedGroup { base, .. } => base_names(base, out),
        Type::Union { arms, .. } => arms.iter().for_each(|a| base_names(a, out)),
        Type::Tuple { elems, .. } => elems.iter().for_each(|e| base_names(e, out)),
        Type::Array { elem, .. } => base_names(elem, out),
        Type::Nullable { inner, .. } => base_names(inner, out),
        Type::Fn { params, ret, .. } => {
            params.iter().for_each(|p| base_names(p, out));
            base_names(ret, out);
        }
    }
}

/// A parameter list rendered for overload matching [qual-refn-match]: each
/// parameter's name, flags and type, with type parameters replaced by their
/// *position*, so a refinement need not use the same names.
/// [fn-rename] [qual-refn-match] Which visible overload a written parameter
/// list names: same parameter names, same types, type parameters matched by
/// position. The one matcher for both mechanisms that name an overload — a
/// refinement and a rename — so the two cannot drift.
///
/// `Err` carries the visible candidates' shapes, for the diagnostic.
pub(crate) fn match_overload<'p>(
    scope: &crate::resolve::ModuleScope<'p>,
    name: &str,
    generics: &[salvo_syntax::ast::Ident],
    params: &[Param],
) -> Result<crate::resolve::FnEntry<'p>, Vec<String>> {
    let gnames: Vec<&str> = generics.iter().map(|g| g.name.as_str()).collect();
    let want = signature(params, &gnames);
    let candidates = scope.fns.get(name).map(|v| v.as_slice()).unwrap_or(&[]);
    let matched: Vec<&crate::resolve::FnEntry<'p>> = candidates
        .iter()
        .filter(|c| {
            let mut cg: Vec<&str> = c.decl.generics.iter().map(|g| g.name.as_str()).collect();
            for (g, _) in &c.decl.generic_canbe {
                if !cg.contains(&g.name.as_str()) {
                    cg.push(g.name.as_str());
                }
            }
            signature(&c.decl.params, &cg) == want
        })
        .collect();
    match matched.as_slice() {
        [one] => Ok(**one),
        _ => Err(candidates
            .iter()
            .map(|c| {
                let ps: Vec<String> = c
                    .decl
                    .params
                    .iter()
                    .map(|p| format!("{}: {}", p.name.name, p.ty))
                    .collect();
                format!("`{name}({})`", ps.join(", "))
            })
            .collect()),
    }
}

/// [fn-overload-duplicate] A parameter list as *overload identity*: the
/// types only, with type parameters positional and the `...`/`?` markers
/// kept. Two declarations of one name that agree here are duplicates —
/// nothing at a call site could ever tell them apart, since parameter names
/// and return types take no part in selection [fn-overload-rank].
pub(crate) fn param_type_signature(params: &[Param], generics: &[&str]) -> Vec<String> {
    params
        .iter()
        .map(|p| {
            format!(
                "{}{}{}",
                if p.variadic { "..." } else { "" },
                if p.implicit { "?" } else { "" },
                normalize(&p.ty, generics)
            )
        })
        .collect()
}

fn signature(params: &[Param], generics: &[&str]) -> Vec<String> {
    params
        .iter()
        .map(|p| {
            format!(
                "{}{}{}: {}",
                if p.variadic { "..." } else { "" },
                if p.implicit { "?" } else { "" },
                p.name.name,
                normalize(&p.ty, generics)
            )
        })
        .collect()
}

/// Renders a written type with generic parameter names replaced by their
/// position in `generics` (`#0`, `#1`, …).
fn normalize(ty: &Type, generics: &[&str]) -> String {
    let name = |id: &Ident| match generics.iter().position(|g| *g == id.name) {
        Some(i) => format!("#{i}"),
        None => id.name.clone(),
    };
    let type_ref = |r: &TypeRef| {
        let mut s = name(&r.name);
        if !r.args.is_empty() {
            let args: Vec<String> = r.args.iter().map(|a| normalize(a, generics)).collect();
            s.push('<');
            s.push_str(&args.join(", "));
            s.push('>');
        }
        s
    };
    match ty {
        Type::Named { qualifiers, base } => {
            let mut s = String::new();
            for q in qualifiers {
                s.push_str(&type_ref(q));
                s.push(' ');
            }
            s.push_str(&type_ref(base));
            s
        }
        Type::QualifiedGroup {
            qualifiers, base, ..
        } => {
            let mut s = String::new();
            for q in qualifiers {
                s.push_str(&type_ref(q));
                s.push(' ');
            }
            format!("{s}({})", normalize(base, generics))
        }
        Type::Union { arms, .. } => arms
            .iter()
            .map(|a| normalize(a, generics))
            .collect::<Vec<_>>()
            .join(" | "),
        Type::Tuple { elems, .. } => format!(
            "({})",
            elems
                .iter()
                .map(|e| normalize(e, generics))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Type::Array { elem, .. } => format!("{}[]", normalize(elem, generics)),
        Type::Nullable { inner, .. } => format!("{}?", normalize(inner, generics)),
        Type::Fn {
            params,
            effects,
            ret,
            ..
        } => {
            let ps: Vec<String> = params.iter().map(|p| normalize(p, generics)).collect();
            let eff = match effects {
                Some(list) => format!(
                    " [{}]",
                    list.iter().map(|e| e.to_string()).collect::<Vec<_>>().join(", ")
                ),
                None => String::new(),
            };
            format!("({}){eff} -> {}", ps.join(", "), normalize(ret, generics))
        }
    }
}

/// The base type name a qualifier's `of` type demands, or `None` when it is
/// a bare type parameter (which accepts anything).
fn of_base(of: &Type, generics: &[Ident]) -> Option<String> {
    let Type::Named { base, .. } = of else {
        return None;
    };
    if generics.iter().any(|g| g.name == base.name.name) {
        return None;
    }
    Some(base.name.name.clone())
}

/// The base type name of a parameter's type, or `None` for shapes a
/// qualifier cannot apply to anyway [qual-union-arm].
fn param_base(ty: &Type) -> Option<String> {
    match ty {
        Type::Named { base, .. } => Some(base.name.name.clone()),
        _ => None,
    }
}
