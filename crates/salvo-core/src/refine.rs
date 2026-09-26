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
//!   [qual-refn-ambiguous] — no error, just a less useful function, plus a
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
    /// The module that file belongs to — the **place** a call names to pick
    /// this statement over a disagreeing one [qual-refn-ambiguous].
    pub module: String,
}

/// One refinement's own statement about a parameter, with the **place** that
/// made it: `f@place` picks by place [qual-refn-at], and an ambiguity names the
/// places that disagree [qual-refn-ambiguous], so the merged view below cannot
/// answer either question on its own.
#[derive(Clone, Debug, PartialEq)]
pub struct RefnStatement {
    pub module: String,
    pub add: Vec<String>,
    pub remove: Vec<String>,
    pub preserve: Vec<String>,
}

/// The refinements that apply to one parameter of one call
/// [qual-refn-ambiguous]: the merged additions and removals, the
/// refinements they came from, and — when applicable refinements
/// disagree — the qualifier names that cannot co-apply. A conflict leaves
/// `add` and `remove` empty: none of them apply.
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
    /// Empty unless the applicable refinements conflict.
    pub conflict: Vec<String>,
    /// What each contributing refinement said, and where it was written
    /// [qual-refn-at] [qual-refn-ambiguous]. Kept even when the merged view is
    /// suppressed by a conflict: a call that names a place still applies that
    /// place's statement.
    pub stated: Vec<RefnStatement>,
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

    /// Every group written about one parameter of such a call — several when
    /// refinements differ in what they require of the argument
    /// [qual-refn-narrow].
    pub fn for_param_all(&self, file: usize, callee: FnKey, param: &str) -> Vec<&RefnGroup> {
        self.for_call(file, callee)
            .iter()
            .filter(|g| g.param == param)
            .collect()
    }

    /// The **unconditional** group for one parameter: what a reader of a
    /// signature is told without knowing the argument [qual-refn-docs], and
    /// what the deduction pass uses.
    pub fn for_param(&self, file: usize, callee: FnKey, param: &str) -> Option<&RefnGroup> {
        self.for_param_all(file, callee, param)
            .into_iter()
            .find(|g| g.requires.is_empty())
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
    entries: Vec<(String, Vec<String>, Vec<String>, Vec<String>)>,
    /// [qual-refn-narrow] Per parameter, the qualifiers an argument must
    /// already carry for this refinement to apply.
    requires: Vec<(String, Vec<String>)>,
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
    // grouped per (callee, parameter) [qual-refn-ambiguous].
    let mut groups: HashMap<(usize, FnKey), Vec<RefnGroup>> = HashMap::new();
    // [qual-refn-ambiguous] Same-place disagreements already reported, keyed by
    // the refinement and the parameter: the visibility loop below runs per
    // *file*, and the declaration's error belongs to the declaration.
    let mut same_place_reported: HashSet<(usize, u32, String)> = HashSet::new();
    for (file_idx, (file, _)) in program.files.iter().zip(&program.modules).enumerate() {
        let scope = &resolution.scopes[file_idx];
        // One contribution per (callee, param, refinement), tagged with
        // whether it came from a top-level `refn`.
        // [qual-refn-narrow] Keyed by the precondition as well as the
        // parameter: two refinements that require different things of the
        // argument are two groups, each applying only where it holds.
        let mut contributions: HashMap<
            (FnKey, String, Vec<String>),
            Vec<(bool, RefnSource, Vec<String>, Vec<String>, Vec<String>)>,
        > = HashMap::new();
        for res in &resolved {
            let visible = match res.qualifier {
                // A qualifier's refinements are in scope wherever the
                // qualifier is [qual-refn-scope] — the *same*
                // declaration, not merely one of that name.
                Some(q) => scope
                    .qualifiers
                    .get(q.name.name.as_str())
                    .is_some_and(|ds| ds.iter().any(|d| std::ptr::eq(*d, q))),
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
                module: program.files[res.file].module.to_string(),
            };
            for (param, add, remove, preserve) in &res.entries {
                let mut requires: Vec<String> = res
                    .requires
                    .iter()
                    .find(|(p, _)| p == param)
                    .map(|(_, qs)| qs.clone())
                    .unwrap_or_default();
                requires.sort();
                contributions
                    .entry((res.callee, param.clone(), requires))
                    .or_default()
                    .push((
                        res.qualifier.is_none(),
                        source.clone(),
                        add.clone(),
                        remove.clone(),
                        preserve.clone(),
                    ));
            }
        }
        for ((callee, param, requires), mut list) in contributions {
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
                preserve: Vec::new(),
                sources: Vec::new(),
                conflict: Vec::new(),
                stated: Vec::new(),
                requires,
            };
            for (_, source, add, remove, preserve) in list {
                group.stated.push(RefnStatement {
                    module: source.module.clone(),
                    add: add.clone(),
                    remove: remove.clone(),
                    preserve: preserve.clone(),
                });
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
                // [qual-preserve] Preservation cannot conflict: keeping a
                // claim alive and any other statement about the parameter
                // are about different values.
                for q in preserve {
                    if !group.preserve.contains(&q) {
                        group.preserve.push(q);
                    }
                }
                if !group.sources.contains(&source) {
                    group.sources.push(source);
                }
            }
            // [qual-refn-ambiguous] Suppress a group whose members
            // disagree. Two additions conflict when the qualifiers could
            // not have been written together [qual-with]; an addition and
            // a removal of the same qualifier conflict outright. Never
            // is applied then — the caller can still test by hand, or
            // reconcile with a top-level `refn`. Judged in *this* file's
            // scope, since that is where the refinements are visible.
            let mut conflict: Vec<String> = Vec::new();
            for (i, a) in group.add.iter().enumerate() {
                for b in group.add.iter().skip(i + 1) {
                    // [qual-overload] `with` names a qualifier, so
                    // same-named declarations share their compatibility list
                    // and any of them answers this.
                    let compatible = match (
                        scope.qualifiers.get(a.as_str()).and_then(|ds| ds.first()),
                        scope.qualifiers.get(b.as_str()).and_then(|ds| ds.first()),
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
                // [qual-refn-ambiguous] A disagreement a selector cannot
                // separate is the refinements' own error, reported where they
                // are written: `f@place` picks a *place*, so two incompatible
                // statements made by **one** place leave the caller no way to
                // choose. Reported once per pair, not once per file that sees
                // it — the file loop would otherwise repeat it.
                // Which places actually stated a conflicting claim: a group's
                // sources include refinements that agree, and reporting those
                // would name the wrong declarations.
                let guilty: Vec<String> = group
                    .stated
                    .iter()
                    .filter(|st| st.add.iter().any(|q| conflict.contains(q)))
                    .map(|st| st.module.clone())
                    .collect();
                let one_place = guilty.len() > 1 && guilty.iter().all(|m| *m == guilty[0]);
                let mut places: Vec<(String, usize, Span)> = group
                    .sources
                    .iter()
                    .map(|s| (s.module.clone(), s.file, s.span))
                    .collect();
                places.sort_by(|a, b| {
                    (&a.0, a.1, a.2.start, a.2.end).cmp(&(&b.0, b.1, b.2.start, b.2.end))
                });
                places.dedup_by(|a, b| a.0 == b.0 && a.1 == b.1 && a.2 == b.2);
                places.retain(|(m, ..)| guilty.contains(m));
                if one_place {
                    for (_, file, span) in &places {
                        let key = (*file, span.start, group.param.clone());
                        if !same_place_reported.insert(key) {
                            continue;
                        }
                        errors.push(FileDiagnostic::error(
                            *file,
                            *span,
                            format!(
                                "this refinement and another in `{}` disagree about \
                                 `{}`: they establish `{}`, which one value cannot \
                                 carry (neither qualifier declares `with` the \
                                 other). A call cannot choose between two \
                                 statements made by the same place, so one of them \
                                 has to go — or the qualifiers have to declare \
                                 `with` each other",
                                guilty[0],
                                group.param,
                                conflict.join("` and `")
                            ),
                        ));
                    }
                }
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
        list.sort_by(|a, b| (&a.param, &a.requires).cmp(&(&b.param, &b.requires)));
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
    // [qual-refn-narrow] Matching ignores the *qualifiers* a refinement writes
    // on its parameters and compares the types under them: that is what lets a
    // refinement narrow a parameter (`list: NonEmpty Mut List<T>` refining
    // `list: Mut List<T>`) and state a claim it only *keeps*. The declaration's
    // own qualifiers must still all be written — they are part of the overload
    // being named — and anything beyond them is the precondition.
    let want_base = base_signature(&decl.params, &generics);
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
            if base_signature(&c.decl.params, &cg) != want_base {
                return false;
            }
            // Every qualifier the declaration writes must appear on the
            // refinement's parameter too, or the refinement is about a
            // different position than it looks.
            c.decl.params.len() == decl.params.len()
                && c.decl.params.iter().zip(&decl.params).all(|(cp, rp)| {
                    let written = qual_names(&rp.ty);
                    qual_names(&cp.ty).iter().all(|q| written.contains(q))
                })
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

    let mut entries: Vec<(String, Vec<String>, Vec<String>, Vec<String>)> = Vec::new();
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
        if e.add.is_empty() && e.remove.is_empty() && e.preserve.is_empty() {
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
                    format!(
                        "this refinement both establishes and invalidates `{q}` for \
                         `{param}`"
                    ),
                ));
            }
        }
        if !add.is_empty() || !remove.is_empty() || !preserve.is_empty() {
            entries.push((param.to_string(), add, remove, preserve));
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
        requires: preconditions(&decl.params, &entry.decl.params),
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
                format!(
                    "`{q}` is not a dependent qualifier [qual-depend]: `preserve` \
                     speaks about claims other values hold, which only a \
                     qualifier with value slots can be"
                ),
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
                    format!(
                        "no value slot of `{q}` accepts a `{}`: `preserve` names \
                         the parameter the claims depend on",
                        param.ty
                    ),
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
    // [deduce-syntax] Unmentioned is kept; only a written move consumes.
    !list.iter().any(|d| {
        d.param_name().is_some_and(|n| n.name == param) && matches!(d.kind, DeductionKind::Moved | DeductionKind::Deferred)
    })
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

/// A type with its own qualifiers removed — what [qual-refn-narrow] matches
/// on, so a refinement may write a *narrower* parameter than the declaration
/// it refines.
fn unqualified(ty: &Type) -> Type {
    match ty {
        Type::Named { base, .. } => Type::Named {
            qualifiers: Vec::new(),
            base: base.clone(),
        },
        Type::QualifiedGroup { base, .. } => (**base).clone(),
        other => other.clone(),
    }
}

/// [qual-refn-match] [qual-refn-narrow] The signature a refinement matches on:
/// parameter names and types with every qualifier stripped. Qualifiers are
/// compared separately — the declaration's are required to be present (they
/// are part of what is being refined) and the extras are the precondition.
fn base_signature(params: &[Param], generics: &[&str]) -> Vec<String> {
    params
        .iter()
        .map(|p| {
            format!(
                "{}{}{}: {}",
                if p.variadic { "..." } else { "" },
                if p.implicit { "?" } else { "" },
                p.name.name,
                normalize(&unqualified(&p.ty), generics)
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
