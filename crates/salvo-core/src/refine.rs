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
//!   [qual-refn] — no error, just a less useful function, plus a
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
use salvo_syntax::messages;
use salvo_syntax::Span;

use crate::diag::FileDiagnostic;
use crate::program::Program;
use crate::decl_id::DeclId;
use crate::resolve::Resolution;

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
    /// The module that file belongs to — the **place** a call names to pick
    /// this statement over a disagreeing one [qual-refn].
    pub module: String,
}

/// The refinements that apply to one parameter of one call: the merged
/// additions, removals and preservations, plus the refinements they came
/// from (for merged docs [qual-refn-docs]). All matching refinements apply
/// (user decision 2026-10-12) — a `+Q`/`-Q` contradiction is refused at the
/// declaration [qual-refn], so there is no call-site conflict to resolve.
#[derive(Clone, Debug, PartialEq)]
pub struct RefnGroup {
    pub param: String,
    pub add: Vec<String>,
    pub remove: Vec<String>,
    /// [qual-preserve] The dependent claims [qual-depend] this call keeps
    /// alive on *other* values that depend on this parameter — the
    /// opt-back from conservative cross-value stripping.
    pub preserve: Vec<String>,
    pub sources: Vec<RefnSource>,
    /// [qual-refn-narrow] The qualifiers the argument must **already** carry
    /// for this group to apply: the refinement wrote a narrower parameter than
    /// the declaration it refines, so what it states is *conditional*. Empty
    /// for an unconditional refinement, which is the common case.
    pub requires: Vec<String>,
}

impl RefnGroup {
    /// Whether the group changes anything at a call site.
    pub fn is_empty(&self) -> bool {
        self.add.is_empty() && self.remove.is_empty() && self.preserve.is_empty()
    }
}

/// Every refinement that applies, keyed by *where the call is written* and
/// *which overload it resolves to* — refinement visibility is per file
/// [qual-refn-scope], so the same callee refines differently in two files.
#[derive(Clone, Debug, Default)]
pub struct Refinements {
    groups: HashMap<(usize, DeclId), Vec<RefnGroup>>,
    /// [deduce-conditional] Refn-attached Bool-conditional responses (6c),
    /// per (file, callee): each entry is (callee param position, negated,
    /// qualifier name). A matched Bool fn gains the conditional at call sites.
    responses: HashMap<(usize, DeclId), Vec<(usize, bool, String)>>,
    /// Validation diagnostics from `collect`, replayed by the checker so
    /// they land in the same output as everything else [diag-structured].
    pub errors: Vec<FileDiagnostic>,
}

impl Refinements {
    /// The refinement groups applying to a call to `callee` written in
    /// `file` (empty when there are none).
    pub fn for_call(&self, file: usize, callee: DeclId) -> &[RefnGroup] {
        self.groups
            .get(&(file, callee))
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// [deduce-conditional] The refn-attached Bool-conditional responses for
    /// a call to `callee` written in `file`: (param position, negated, qual).
    pub fn responses_for(&self, file: usize, callee: DeclId) -> &[(usize, bool, String)] {
        self.responses
            .get(&(file, callee))
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Every group written about one parameter of such a call — several when
    /// refinements differ in what they require of the argument
    /// [qual-refn-narrow].
    pub fn for_param_all(&self, file: usize, callee: DeclId, param: &str) -> Vec<&RefnGroup> {
        self.for_call(file, callee)
            .iter()
            .filter(|g| g.param == param)
            .collect()
    }

    /// The **unconditional** group for one parameter: what a reader of a
    /// signature is told without knowing the argument [qual-refn-docs], and
    /// what the deduction pass uses.
    pub fn for_param(&self, file: usize, callee: DeclId, param: &str) -> Option<&RefnGroup> {
        self.for_param_all(file, callee, param)
            .into_iter()
            .find(|g| g.requires.is_empty())
    }

    pub fn is_empty(&self) -> bool {
        self.groups.is_empty()
    }
}

/// One validated refinement, before visibility is applied.
struct Resolved<'p> {
    decl: &'p RefnDecl,
    /// The declaring qualifier, or `None` for a top-level `refn`.
    qualifier: Option<&'p QualifierDecl>,
    /// The file the refinement is *written* in.
    file: usize,
    /// [qual-refn-match] The overloads it refines — one-to-many (user
    /// decision 2026-10-12): every in-scope overload of the name whose
    /// parameters the refn's gate accepts.
    callees: Vec<DeclId>,
    /// Entries as (refn-param *position*, add, remove, preserve): the
    /// position maps to each callee's parameter at that slot
    /// [qual-refn-match].
    entries: Vec<(usize, Vec<String>, Vec<String>, Vec<String>)>,
    /// [deduce-conditional] The refn's Bool-conditional response (6c), as
    /// (refn-param position, negated, qualifier name).
    response: Vec<(usize, bool, String)>,
    /// [qual-refn-narrow] Per refn-param *position*, the qualifiers an
    /// argument must already carry for this refinement to apply.
    requires: Vec<(usize, Vec<String>)>,
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
    // grouped per (callee, parameter). All matching refinements apply — a
    // `+Q`/`-Q` contradiction is a declaration error [qual-refn], so there is
    // no call-site conflict to resolve.
    let mut groups: HashMap<(usize, DeclId), Vec<RefnGroup>> = HashMap::new();
    let mut response_index: HashMap<(usize, DeclId), Vec<(usize, bool, String)>> = HashMap::new();
    for (file_idx, (file, _)) in program.files.iter().zip(&program.modules).enumerate() {
        let scope = &resolution.scopes[file_idx];
        // One contribution per (callee, param, precondition).
        // [qual-refn-narrow] Keyed by the precondition too: two refinements
        // that require different things of the argument are two groups, each
        // applying only where it holds.
        let mut contributions: HashMap<
            (DeclId, String, Vec<String>),
            Vec<(RefnSource, Vec<String>, Vec<String>, Vec<String>)>,
        > = HashMap::new();
        for res in &resolved {
            let visible = match res.qualifier {
                // A qualifier's refinements are in scope wherever the
                // qualifier is [qual-refn-scope] — the *same* declaration.
                Some(q) => scope
                    .qualifiers
                    .get(q.name.name.as_str())
                    .is_some_and(|ds| ds.iter().any(|d| std::ptr::eq(*d, q))),
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
                module: program.files[res.file].module.to_string(),
            };
            // [qual-refn-match] One refn now refines *many* overloads: index
            // its statements under each matched callee, translating the
            // refn-param position to that callee's parameter name.
            for callee in &res.callees {
                let Some(cdecl) = fn_decl(program, resolution, *callee) else { continue };
                let cfixed: Vec<&Param> = cdecl.params.iter().filter(|p| !p.implicit).collect();
                for (pos, add, remove, preserve) in &res.entries {
                    let Some(cp) = cfixed.get(*pos) else { continue };
                    let param = cp.name.name.clone();
                    let mut requires: Vec<String> = res
                        .requires
                        .iter()
                        .find(|(p, _)| p == pos)
                        .map(|(_, qs)| qs.clone())
                        .unwrap_or_default();
                    requires.sort();
                    contributions
                        .entry((*callee, param, requires))
                        .or_default()
                        .push((
                            source.clone(),
                            add.clone(),
                            remove.clone(),
                            preserve.clone(),
                        ));
                }
                // [deduce-conditional] A refn-attached Bool response (6c):
                // index it under the callee, by that callee's param position.
                for (pos, negated, qual) in &res.response {
                    if cfixed.get(*pos).is_some() {
                        response_index
                            .entry((file_idx, *callee))
                            .or_default()
                            .push((*pos, *negated, qual.clone()));
                    }
                }
            }
        }
        for ((callee, param, requires), list) in contributions {
            let mut group = RefnGroup {
                param,
                add: Vec::new(),
                remove: Vec::new(),
                preserve: Vec::new(),
                sources: Vec::new(),
                requires,
            };
            for (source, add, remove, preserve) in list {
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
                for q in preserve {
                    if !group.preserve.contains(&q) {
                        group.preserve.push(q);
                    }
                }
                if !group.sources.contains(&source) {
                    group.sources.push(source);
                }
            }
            if !group.is_empty() {
                groups.entry((file_idx, callee)).or_default().push(group);
            }
        }
    }
    // Deterministic order per call: the table is read for diagnostics and
    // for hover, and a hash-map traversal built it.
    for list in groups.values_mut() {
        list.sort_by(|a, b| (&a.param, &a.requires).cmp(&(&b.param, &b.requires)));
    }
    groups.retain(|_, v| !v.is_empty());

    Refinements { groups, responses: response_index, errors }
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
    // A refinement with type parameters **of its own** refines a fn over
    // *those* (`refn add<T>(list: Mut List<T>, …)` inside `Idx<C>`), so they
    // take the leading positions; the qualifier's follow, for a signature
    // that mentions one.
    let mut generics: Vec<&str> = Vec::new();
    generics.extend(decl.generics.iter().map(|g| g.name.as_str()));
    if let Some(q) = qualifier {
        generics.extend(q.generics.iter().map(|g| g.name.as_str()));
    }
    let want = signature(&decl.params, &generics);
    let _ = &want;

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
            messages::qual_refn_match_unknown_type(name, &decl.name.name),
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
            messages::qual_refn_match_no_function(name),
        ));
        return None;
    }
    // [qual-refn-match] One-to-many positional base-type matching (user
    // decision 2026-10-12). The refn's parameters are a *gate*: a candidate
    // overload of the name matches when, position by position, the refn's
    // parameter base type is a super- or subtype of the candidate's base
    // type (all qualifiers, including `Mut`, stripped). A trailing `*`
    // matches every remaining parameter. Implicit parameters take no
    // position. An `@module` selector on the refn restricts the candidate
    // set to that module's overloads.
    let matched: Vec<&crate::resolve::FnEntry<'p>> = candidates
        .iter()
        .filter(|c| {
            if let Some(at) = &decl.at {
                if !crate::source::ModulePath::text_matches_suffix(
                    &c.module.to_string(),
                    &at.name,
                ) {
                    return false;
                }
            }
            let cfixed: Vec<&Param> = c.decl.params.iter().filter(|p| !p.implicit).collect();
            // Without `*`, arity must match the refn's spelled params; with
            // `*`, the candidate must have at least that many.
            if decl.wildcard {
                if cfixed.len() < decl.params.len() {
                    return false;
                }
            } else if cfixed.len() != decl.params.len() {
                return false;
            }
            decl.params.iter().zip(&cfixed).all(|(rp, cp)| {
                base_super_or_sub(&rp.ty, &cp.ty, &generics)
            })
        })
        .collect();
    let mut seen = HashSet::new();
    let callees: Vec<DeclId> = matched
        .iter()
        .filter(|c| seen.insert(c.key))
        .map(|c| c.key)
        .collect();
    if callees.is_empty() {
        let shapes: Vec<String> =
            candidates.iter().map(|c| written_shape(name, c.decl)).collect();
        errors.push(FileDiagnostic::error(
            file_idx,
            decl.name.span,
            messages::qual_refn_match_no_overload(name, &shapes.join(", ")),
        ));
        return None;
    }
    // A representative candidate for per-parameter validation below (every
    // matched overload shares the refn's parameter *names* via position).
    let entry = *matched[0];

    let mut entries: Vec<(usize, Vec<String>, Vec<String>, Vec<String>)> = Vec::new();
    let mut seen: HashSet<&str> = HashSet::new();
    for e in &decl.deductions {
        let param = e.param.name.as_str();
        if !seen.insert(param) {
            errors.push(FileDiagnostic::error(
                file_idx,
                e.span,
                messages::qual_refn_duplicate_param(param),
            ));
            continue;
        }
        // The deduction names one of the *refn's* parameters; its position
        // maps to each matched overload's parameter at that slot.
        let Some(pos) = decl.params.iter().position(|p| p.name.name == param) else {
            errors.push(FileDiagnostic::error(
                file_idx,
                e.span,
                messages::qual_refn_unknown_param(param),
            ));
            continue;
        };
        // The matched overload's parameter at that position — what the
        // qualifier's `of` type is checked against. Non-implicit slots only.
        let Some(p) = entry.decl.params.iter().filter(|p| !p.implicit).nth(pos) else {
            continue;
        };
        if e.add.is_empty() && e.remove.is_empty() && e.preserve.is_empty() {
            errors.push(FileDiagnostic::error(
                file_idx,
                e.span,
                messages::qual_refn_states_nothing(param),
            ));
            continue;
        }
        // A refinement only ever speaks about a parameter the call gives
        // back: nothing is known about a moved one afterwards. Checked
        // against a *written* list only — an inferred one is not settled
        // yet at this point, and a moved parameter's group simply never
        // fires (the call-site loop applies refinements on the kept path).
        if let Some(list) = &entry.decl.deductions {
            if !kept_in(list, &p.name.name) {
                errors.push(FileDiagnostic::error(
                    file_idx,
                    e.span,
                    messages::qual_refn_moved_param(name, param),
                ));
                continue;
            }
        }
        let mut add: Vec<String> = Vec::new();
        let mut remove: Vec<String> = Vec::new();
        let mut preserve: Vec<String> = Vec::new();
        for (refs, out, verb) in [
            (&e.add, &mut add, "establish"),
            (&e.remove, &mut remove, "invalidate"),
            (&e.preserve, &mut preserve, "preserve"),
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
                    messages::qual_refn_entry_contradiction(q, param),
                ));
            }
        }
        if !add.is_empty() || !remove.is_empty() || !preserve.is_empty() {
            entries.push((pos, add, remove, preserve));
        }
    }
    if entries.is_empty() && decl.response.is_none() {
        return None;
    }
    // [qual-refn] A declaration-time contradiction: the refn both `+Q` and
    // `-Q` for one parameter position and qualifier is refused here (user
    // decision 2026-10-12), replacing the old call-site conflict suppression.
    for (pos, add, remove, _) in &entries {
        for q in add {
            if remove.contains(q) {
                errors.push(FileDiagnostic::error(
                    file_idx,
                    decl.name.span,
                    messages::qual_refn_position_contradiction(q, pos + 1),
                ));
            }
        }
    }
    // [qual-refn-narrow] Preconditions, by refn-param position.
    let requires: Vec<(usize, Vec<String>)> = preconditions(&decl.params, &entry.decl.params)
        .into_iter()
        .filter_map(|(pname, qs)| {
            decl.params.iter().position(|p| p.name.name == pname).map(|pos| (pos, qs))
        })
        .collect();
    // [deduce-conditional] Resolve the refn's response clause (6c) to
    // (refn-param position, negated, qual-name).
    let response: Vec<(usize, bool, String)> = decl
        .response
        .iter()
        .flatten()
        .filter_map(|r| {
            decl.params
                .iter()
                .position(|p| p.name.name == r.subject.name)
                .map(|pos| (pos, r.negated, r.qual.name.name.clone()))
        })
        .collect();
    Some(Resolved {
        decl,
        qualifier,
        file: file_idx,
        callees,
        entries,
        response,
        requires,
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
                messages::qual_refn_foreign_qualifier(&owner.name.name, verb, q),
            ));
            return None;
        }
    }
    // [qual-overload] Several qualifiers may share this name over different
    // subject types, and the refined parameter says which one the refinement
    // is about — matched here by *base name*, which is all this pass can see
    // without the checker's unification (the same comparison the `of` check
    // below makes). With one candidate the parameter is not consulted, so a
    // generic `of` still resolves.
    let candidates = scope.qualifiers.get(q).map(|v| v.as_slice()).unwrap_or(&[]);
    let qd = match candidates {
        [] => None,
        [one] => Some(*one),
        many => param_base(&param.ty).and_then(|base| {
            many.iter().copied().find(|d| {
                of_base(&d.of, &d.generics).map(|of| of == base).unwrap_or(true)
            })
        }),
    };
    let Some(qd) = qd else {
        errors.push(FileDiagnostic::error(
            file_idx,
            r.span,
            messages::qual_refn_unknown_qualifier(q),
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
            messages::qual_subject_compiler_qualifier(q, verb),
        ));
        return None;
    }
    if qd.subject == QualSubject::Provenance {
        errors.push(FileDiagnostic::error(
            file_idx,
            r.span,
            messages::qual_subject_provenance(q),
        ));
        return None;
    }
    // The claim has to be about the parameter's type: the qualifier's `of`
    // type must accept it [qual-of]. Compared by base name, which is what
    // this pass can see without the checker's unification — a generic `of`
    // (`qualifier Ok<T> of T`) accepts anything.
    //
    // [qual-preserve] A `preserve` entry reads the other way round: the
    // parameter is not the claim's *subject* but the value it depends on,
    // so it must fit one of the qualifier's **value slots** — and only a
    // dependent qualifier has any claims held by other values to keep.
    if verb == "preserve" {
        if qd.value_slots.is_empty() {
            errors.push(FileDiagnostic::error(
                file_idx,
                r.span,
                messages::qual_depend_preserve_not_dependent(q),
            ));
            return None;
        }
        if let Some(base) = param_base(&param.ty) {
            let fits = qd
                .value_slots
                .iter()
                .any(|v| of_base(&v.ty, &qd.generics).map(|b| b == base).unwrap_or(true));
            if !fits {
                errors.push(FileDiagnostic::error(
                    file_idx,
                    r.span,
                    messages::qual_preserve_no_value_slot(q, &param.ty),
                ));
                return None;
            }
        }
        return Some(q.to_string());
    }
    if let (Some(of), Some(base)) = (of_base(&qd.of, &qd.generics), param_base(&param.ty)) {
        if of != base {
            errors.push(FileDiagnostic::error(
                file_idx,
                r.span,
                messages::qual_of_wrong_subject(q, &qd.of, &base, &decl.name.name, &param.name.name, &param.ty),
            ));
            return None;
        }
    }
    Some(q.to_string())
}

/// Whether a written deduction list gives `param` back to the caller
/// [deduce-syntax].
fn kept_in(list: &[Deduction], param: &str) -> bool {
    // [deduce-syntax] Unmentioned is kept; only a written move consumes.
    !list.iter().any(|d| {
        d.param_name().is_some_and(|n| n.name == param) && matches!(d.kind, DeductionKind::Moved | DeductionKind::Deferred)
    })
}

/// [qual-refn-match] Whether two written parameter types match for the
/// refinement *gate*: their **base** types (every qualifier stripped,
/// including `Mut`) are such that one is a super- or subtype of the other.
/// Approximate and syntactic — the pre-pass has no checker to lower types —
/// which is all the gate needs: the strict work is the call site's
/// [qual-refn] (`arg <: refn-param`). Handles base-name equality, `Any` as
/// the top on either side, optional (`T?`) widening either way, and a type
/// parameter (which the refn may use to mean "anything of that shape").
fn base_super_or_sub(a: &Type, b: &Type, generics: &[&str]) -> bool {
    let a = strip_all_quals(a);
    let b = strip_all_quals(b);
    base_fits(a, b, generics) || base_fits(b, a, generics)
}

/// Whether `sub`'s base fits where `sup`'s is expected (one direction).
fn base_fits(sub: &Type, sup: &Type, generics: &[&str]) -> bool {
    // A bare type parameter accepts anything (the refn means "any such").
    let is_var = |t: &Type| matches!(t, Type::Named { base, qualifiers }
        if qualifiers.is_empty() && base.args.is_empty() && generics.contains(&base.name.name.as_str()));
    if is_var(sub) || is_var(sup) {
        return true;
    }
    match (sub, sup) {
        // `Any` is the top: anything fits `Any`.
        (_, Type::Named { base, .. }) if base.name.name == "Any" => true,
        // `T` fits `T?` (optional widening).
        (_, Type::Nullable { inner, .. }) => base_fits(sub, inner, generics),
        (Type::Nullable { inner, .. }, _) => base_fits(inner, sup, generics),
        (
            Type::Named { base: ba, .. },
            Type::Named { base: bb, .. },
        ) => ba.name.name == bb.name.name,
        // Same-kind structural types match as a gate (the strict check is
        // the call site): fn ↔ fn, tuple ↔ tuple, array ↔ array, literal
        // ↔ literal, union ↔ union.
        (Type::Fn { .. }, Type::Fn { .. }) => true,
        (Type::Tuple { .. }, Type::Tuple { .. }) => true,
        (Type::Array { .. }, Type::Array { .. }) => true,
        (Type::Literal { .. }, Type::Literal { .. }) => true,
        (Type::Union { .. }, _) | (_, Type::Union { .. }) => true,
        _ => false,
    }
}

/// The `FnDecl` a `DeclId` points at.
fn fn_decl<'p>(program: &'p Program, resolution: &Resolution<'p>, key: DeclId) -> Option<&'p salvo_syntax::ast::FnDecl> {
    resolution.decls.fn_decl(&program.modules, key)
}

/// A written type with *every* qualifier stripped at the top level — both
/// the `Named` qualifier list and a `QualifiedGroup`'s — leaving the base.
fn strip_all_quals(ty: &Type) -> &Type {
    match ty {
        Type::QualifiedGroup { base, .. } => strip_all_quals(base),
        other => other,
    }
}

/// Every *base* type name a written type mentions (qualifier positions
/// excluded: `Mut` and friends are the language's, not declarations).
fn base_names<'t>(ty: &'t Type, out: &mut Vec<&'t Ident>) {
    match ty {
        Type::Literal { .. } => {}
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
        Type::Spread { inner, .. } => base_names(inner, out),
        Type::Fn { params, ret, .. } => {
            params.iter().for_each(|p| base_names(p, out));
            base_names(ret, out);
        }
    }
}


/// A candidate rendered the way a `refn` or a `rename` has to **write** it:
/// its own type parameters in front of the parameter list, because that is the
/// spelling the matcher accepts (`min<T>(set: SortedSet<T>)`). Printing the
/// list without them offers a shape that does not match, which is a diagnostic
/// that sends the reader into the trap it is warning about.
fn written_shape(name: &str, decl: &salvo_syntax::ast::FnDecl) -> String {
    let mut gs: Vec<&str> = decl.generics.iter().map(|g| g.name.as_str()).collect();
    for (g, _) in &decl.generic_canbe {
        if !gs.contains(&g.name.as_str()) {
            gs.push(g.name.as_str());
        }
    }
    let generics = if gs.is_empty() {
        String::new()
    } else {
        format!("<{}>", gs.join(", "))
    };
    let ps: Vec<String> = decl
        .params
        .iter()
        .map(|p| format!("{}: {}", p.name.name, p.ty))
        .collect();
    format!("`{name}{generics}({})`", ps.join(", "))
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
    // A declaration may be in scope twice — a module's annex sees its own
    // module's fns, and the same fns again as the module it tests — so the
    // match is by declaration, not by entry.
    let mut matched: Vec<&crate::resolve::FnEntry<'p>> = candidates
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
    let mut seen = std::collections::HashSet::new();
    matched.retain(|c| seen.insert(c.key));
    match matched.as_slice() {
        [one] => Ok(**one),
        _ => Err(candidates
            .iter()
            .map(|c| written_shape(name, c.decl))
            .collect()),
    }
}

/// [fn-overload-return] A return type as part of *overload identity*,
/// spelled like a parameter type (`None` when none is written). Two
/// declarations of one name with the same parameter types are an overload
/// set when this differs, and a duplicate when it agrees.
pub(crate) fn return_type_signature(ret: Option<&Type>, generics: &[&str]) -> String {
    ret.map(|t| normalize(t, generics)).unwrap_or_else(|| "None".to_string())
}

/// [fn-overload-duplicate] A parameter list as *overload identity*: the
/// types only, with type parameters positional and the `...`/`?` markers
/// kept. With the return type ([`return_type_signature`]) it is what makes
/// two declarations of one name a duplicate: nothing at a call site could
/// ever tell them apart [fn-overload-return].
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

/// [qual-refn-narrow] The qualifiers a refinement's parameter carries beyond
/// the ones the refined declaration's own parameter has: the refinement's
/// **precondition**, one entry per parameter that adds any.
///
/// `refn swap(list: NonEmpty Mut List<T>, …)` refining
/// `swap(list: Mut List<T>, …)` reads "when the list is already `NonEmpty`" —
/// which is the only way to state a claim that is *kept* rather than
/// established, since exchanging two elements of an empty list does not make
/// it non-empty (user correction 2026-09-23).
fn preconditions(refn: &[Param], callee: &[Param]) -> Vec<(String, Vec<String>)> {
    let mut out = Vec::new();
    for (r, c) in refn.iter().zip(callee) {
        let have = qual_names(&c.ty);
        let extra: Vec<String> = qual_names(&r.ty)
            .into_iter()
            .filter(|q| !have.contains(q))
            .collect();
        if !extra.is_empty() {
            out.push((r.name.name.clone(), extra));
        }
    }
    out
}

/// The qualifier names written on a type, outermost first.
fn qual_names(ty: &Type) -> Vec<String> {
    match ty {
        Type::Named { qualifiers, .. } => {
            qualifiers.iter().map(|q| q.name.name.clone()).collect()
        }
        Type::QualifiedGroup { qualifiers, .. } => {
            qualifiers.iter().map(|q| q.name.name.clone()).collect()
        }
        _ => Vec::new(),
    }
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
        Type::Literal { value, .. } => value.to_string(),
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
        Type::Spread { inner, .. } => format!("...{}", normalize(inner, generics)),
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
/// [qual-overload] The base name of a qualifier's `of` type — the *subject*
/// that distinguishes same-named qualifiers. `None` for a generic `of`
/// (`qualifier Ok<T> of T`), which accepts any subject and so cannot be
/// told apart by one. Syntactic on purpose: it is what a pass without the
/// checker's unification can see, and both backends need the same answer.
pub fn of_base(of: &Type, generics: &[Ident]) -> Option<String> {
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
