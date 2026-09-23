//! [iter-fn] Expanding an `iter fn` into ordinary declarations.
//!
//! An `iter fn` is the hand-written half of iteration [iter-protocol] with the
//! boilerplate removed: the author writes the `next`, and the **pass struct is
//! generated**.
//!
//! ```text
//! struct Countdown { from: Int }
//!
//! iter fn next(c: Countdown) -> Emitted Int | Finished {
//!     state {
//!         at: Int = c.from
//!     }
//!     if at <= 0 { return finished() }
//!     at = at - 1
//!     return emitted(at + 1)
//! }
//! ```
//!
//! becomes, before anything else in the compiler sees it:
//!
//! ```text
//! struct __Pass_Countdown : Yield<self, Int> canbe Mut {
//!     __subject: proj Countdown,
//!     at: Int
//! }
//!
//! fn iter(c: Countdown) [] -> [c] Mut __Pass_Countdown {
//!     return Mut __Pass_Countdown { __subject: c, at: c.from }
//! }
//!
//! fn next(__p: Mut __Pass_Countdown) [] -> [__p: Mut] Emitted Int | Finished {
//!     if __p.at <= 0 { return finished() }
//!     __p.at = __p.at - 1
//!     return emitted(__p.at + 1)
//! }
//! ```
//!
//! Three consequences are the reason it is done *here*, in the syntax crate,
//! rather than as a checker feature:
//!
//! * **Nothing downstream knows the form exists.** Resolution, the checker,
//!   the deduction pass, both emitters and the LSP see an ordinary pass struct
//!   with an ordinary `next` — the shape they already support — so `for`,
//!   the combinators and `let p = iter(c)` all work with no new machinery.
//! * **The `state` fields get every rule for free**, because they *are* struct
//!   fields: types, mutation, `Mut`, narrowing, deductions.
//! * **"No effects in an initializer" needs no check**: the initializers become
//!   the body of an `iter` declared `[]`, so an effectful call there is an
//!   ordinary effect error at that call.
//!
//! The generated struct is named `__Pass_<Subject>` — unnameable, since a type
//! reference beginning with `_` is refused [iter-fn] — which is what keeps the
//! first boundary the design rests on: a pass you must *name* is still written
//! by hand.

use std::collections::BTreeMap;

use crate::ast::*;
use crate::diag::Diagnostic;
use crate::span::Span;

/// Expands every `iter fn` in `module` into its three declarations, in place.
/// Diagnostics are returned rather than thrown: an `iter fn` that cannot be
/// expanded is dropped, so the rest of the module still parses and checks.
pub fn expand_iter_fns(module: &mut Module) -> Vec<Diagnostic> {
    expand_iter_fns_with(module, &[])
}

/// The struct declarations of a module, cloned — what a program-level
/// expansion passes to `expand_iter_fns_with` as the *other* files'
/// declarations.
pub fn struct_decls(module: &Module) -> Vec<StructDecl> {
    module
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Struct(s) => Some(s.clone()),
            _ => None,
        })
        .collect()
}

/// `expand_iter_fns`, with the struct declarations of the *rest of the
/// program* available for the per-field snapshot [iter-fn]. The module's own
/// declarations are looked up first, so a local name always wins; with the
/// extra table a subject declared in another file gets the per-field
/// snapshot instead of falling back to holding the whole value (the roadmap
/// "mutable origins, option (e)" residue). Generic subjects still hold the
/// whole value — their field types would need substituting.
pub fn expand_iter_fns_with(
    module: &mut Module,
    extern_structs: &[StructDecl],
) -> Vec<Diagnostic> {
    // [cmp-auto] The other desugaring, run here because this is the one
    // entry point every consumer of a parsed module already calls — so no path
    // can forget it. It needs no declarations but the module's own: a `default`
    // obligation and the type it is written on are the same declaration.
    expand_auto_obligations(module);
    let mut diags = Vec::new();
    // The subject's own declaration: that is what makes a *per-field*
    // snapshot possible, since a generated field needs the field's written
    // type and the desugaring has no type information of its own. Local
    // declarations first — `.find` takes the first hit.
    let mut structs: Vec<StructDecl> = struct_decls(module);
    structs.extend(extern_structs.iter().cloned());
    let mut expanded: Vec<Item> = Vec::with_capacity(module.items.len());
    for item in std::mem::take(&mut module.items) {
        match item {
            Item::Fn(f) if f.is_iter => {
                if let Some(items) = expand(&f, &structs, &mut diags) {
                    expanded.extend(items);
                }
            }
            other => expanded.push(other),
        }
    }
    module.items = expanded;
    diags
}

/// Distinct spans for synthesized nodes, taken one byte at a time from inside
/// the declaration they came from.
///
/// This is not cosmetic. The checker's side tables are keyed by span —
/// `expr_ty`, `coerce`, `call_fn`, and `fn_refs` (which is how the emitters find
/// a fn's effect list) — so two synthesized nodes sharing one span silently
/// overwrite each other's entry. It cost an afternoon: the generated `iter`
/// inherited the `iter fn`'s `[Console]` because their name spans were equal,
/// and then a `copy` argument was typed as the struct literal that shared its
/// span. Every span here still points *inside* the declaration, so a diagnostic
/// that somehow escapes lands in the right place.
struct Spans {
    next: u32,
    end: u32,
}

impl Spans {
    fn new(decl: Span) -> Self {
        Spans {
            next: decl.start,
            end: decl.end,
        }
    }

    /// The next unused byte of the declaration, as a one-byte span. Falls back
    /// to the declaration's start when a very short declaration runs out —
    /// which cannot happen for anything the grammar accepts (`iter fn next(x: T)`
    /// is already longer than the node count), but a wrap is better than a
    /// panic.
    fn take(&mut self) -> Span {
        if self.next + 1 >= self.end {
            self.next = self.end.saturating_sub(1);
        }
        let at = self.next;
        self.next += 1;
        Span::new(at, at + 1)
    }
}

/// The pass struct's field holding the subject. Unwritable, like the struct.
const SUBJECT: &str = "__subject";
/// The generated `next`'s parameter name.
const PASS: &str = "__p";

fn expand(
    f: &FnDecl,
    structs: &[StructDecl],
    diags: &mut Vec<Diagnostic>,
) -> Option<Vec<Item>> {
    let span = f.name.span;
    let mut error = |message: String, span: Span| diags.push(Diagnostic::error(message, span));

    // The name is the obligation's member name, as for a `yield fn`: `for`
    // reads a declaration, and an `iter fn` called anything else answers to
    // nothing [iter-protocol].
    if f.name.name != "next" {
        error(
            format!(
                "an `iter fn` must be called `next`: it is the member of the \
                 `Yield` obligation that `for` drives, and `{}` answers to \
                 nothing",
                f.name.name
            ),
            span,
        );
        return None;
    }
    if f.params.len() != 1 {
        error(
            format!(
                "an `iter fn` takes exactly one parameter — the subject it \
                 iterates (found {})",
                f.params.len()
            ),
            span,
        );
        return None;
    }
    let Some(body) = f.body.clone() else {
        error(
            "an `iter fn` needs a body: it *is* the `next` the compiler would \
             otherwise generate"
                .to_string(),
            span,
        );
        return None;
    };
    let subject = &f.params[0];
    // The subject is ordinary data the caller keeps: a fresh pass is minted per
    // drive, so advancing never writes through it. `Mut` would promise the
    // opposite.
    let Type::Named { qualifiers, base } = &subject.ty else {
        error(
            "an `iter fn`'s subject must be a named type — the generated pass \
             is named after it; wrap an array, tuple or union in a struct of \
             your own"
                .to_string(),
            subject.span,
        );
        return None;
    };
    if let Some(q) = qualifiers.first() {
        error(
            format!(
                "an `iter fn`'s subject is read, not advanced: drop the `{}` \
                 from `{}` — the generated pass holds the position, and each \
                 drive mints a fresh one",
                q.name.name, subject.name.name
            ),
            subject.span,
        );
        return None;
    }

    // The element type comes from the declared result, which must be the
    // obligation's own shape.
    let elem = element_type(f.return_type.as_ref()).or_else(|| {
        error(
            "an `iter fn` returns `Emitted T | Finished`: it reports either an \
             element or the end of the sequence"
                .to_string(),
            f.return_type
                .as_ref()
                .map(|t| t.span())
                .unwrap_or(span),
        );
        None
    })?;

    // Names the body may not rebind: the subject and the state fields all
    // become fields of the pass, and a shadowing local would silently mean
    // something else [iter-fn].
    let mut reserved: Vec<&Ident> = vec![&subject.name];
    for (i, field) in f.iter_state.iter().enumerate() {
        if field.name.name == subject.name.name {
            error(
                format!(
                    "`{}` is the subject's name: a `state` field may not shadow it",
                    field.name.name
                ),
                field.span,
            );
            return None;
        }
        if f.iter_state[..i]
            .iter()
            .any(|prev| prev.name.name == field.name.name)
        {
            error(
                format!("`state` field `{}` is declared twice", field.name.name),
                field.span,
            );
            return None;
        }
        reserved.push(&field.name);
    }
    let mut shadowed = Vec::new();
    collect_shadowing(&body, &reserved, &mut shadowed);
    if let Some((name, at)) = shadowed.first() {
        error(
            format!(
                "`{name}` is a field of the generated pass, so a binding of \
                 that name would shadow it: rename the binding"
            ),
            *at,
        );
        return None;
    }

    // [iter-fn] Every generated declaration needs a **distinct name span**:
    // the checker's side tables (`fn_refs` → `fn_effects`, deductions, the LSP's
    // definition sites) are keyed by it, so two declarations sharing one span
    // collide and the later wins — which showed up as the generated `iter`
    // inheriting the `iter fn`'s effect list. Each borrows a different real
    // token of the source, so diagnostics still land somewhere meaningful.
    let mut spans = Spans::new(f.span);
    let struct_span = spans.take();
    let iter_span = spans.take();
    let next_span = span;
    let subject_field_span = spans.take();
    let subject_read_span = spans.take();
    let lit_span = spans.take();
    let ret_span = spans.take();
    let pass_name = Ident {
        name: format!("__Pass_{}", base.name.name.replace('.', "")),
        span: struct_span,
    };
    let pass_ty = |mutable: bool| Type::Named {
        qualifiers: if mutable {
            vec![type_ref("Mut", span)]
        } else {
            vec![]
        },
        base: TypeRef {
            at: None,
            binder: false,
            alias: None,
            value_args: Vec::new(),
            name: pass_name.clone(),
            args: f
                .generics
                .iter()
                .map(|g| Type::Named {
                    qualifiers: vec![],
                    base: TypeRef {
                        at: None,
                        binder: false,
                        alias: None,
                        name: g.clone(),
                        args: vec![],
            value_args: Vec::new(),
                        from: Vec::new(),
                        span: g.span,
                    },
                })
                .collect(),
            from: Vec::new(),
            span,
        },
    };

    // [iter-fn] How much of the subject the pass has to hold. A pass exists to
    // carry the position, and the subject rides along only because the body may
    // read it on any turn — so the cheapest correct answer is looked for first,
    // in three tiers:
    //
    //   1. the body never reads the subject (a plain counter): hold *nothing*;
    //   2. it only ever reads plain fields of it, and their types are visible:
    //      hold one snapshot field per field read;
    //   3. anything else — the subject passed on as a value, an assignment
    //      through it, a generic subject (whose field types would need
    //      substituting), a declaration the expansion cannot see: hold it
    //      whole. (A single-file parse sees this file only; the program-level
    //      expansion the CLI drives sees every file's declarations.)
    //
    // All three are observationally identical, because the pass *borrows*: it
    // is a view of the subject [proj-field], so the subject cannot be written
    // while the pass lives [proj-infer], and a per-field borrow at the mint says
    // exactly what a whole borrow says. (Until 2026-09-11 the mint copied, which
    // was the phase's last hidden copy; an `iter fn` that wants a snapshot now
    // writes `copy(...)` in a `state` initializer.)
    let state_names: Vec<String> = f
        .iter_state
        .iter()
        .map(|field| field.name.name.clone())
        .collect();
    let mut scan = Rewrite {
        subject: subject.name.name.clone(),
        state: state_names.clone(),
        // Scan mode synthesizes nothing, so this allocator is never drawn on;
        // the rewrite below gets the live one.
        spans: Spans::new(f.span),
        snapshots: BTreeMap::new(),
        scan: true,
        used_fields: Vec::new(),
        used_whole: false,
        assigns_through: false,
    };
    let mut scanned = body.clone();
    scan.block(&mut scanned);
    let subject_decl = if base.args.is_empty() {
        structs.iter().find(|s| s.name.name == base.name.name)
    } else {
        None
    };
    let mut snapshots: BTreeMap<String, String> = BTreeMap::new();
    let mut snapshot_fields: Vec<(String, Type, Vec<String>)> = Vec::new();
    let mut whole = scan.used_whole || scan.assigns_through;
    if !whole {
        for used in &scan.used_fields {
            let Some(decl) = subject_decl else {
                whole = true;
                break;
            };
            let Some(field) = decl.fields.iter().find(|f| f.name.name == *used) else {
                // Not a field of the subject: keep it whole, so the checker
                // reports the real mistake against the real type.
                whole = true;
                break;
            };
            // The snapshot keeps the field's own name where that is free, since
            // the generated code is read by people; a `state` field of the same
            // name pushes it into the compiler's namespace instead.
            let pass_name = if state_names.iter().any(|s| *s == *used) {
                format!("__s_{used}")
            } else {
                used.clone()
            };
            snapshots.insert(used.clone(), pass_name.clone());
            snapshot_fields.push((pass_name, field.ty.clone(), field.docs.clone()));
        }
    }
    if whole {
        snapshots.clear();
        snapshot_fields.clear();
    }

    // 1. the pass struct: what it holds of the subject, then the state fields.
    let mut fields = Vec::new();
    if whole {
        fields.push(FieldDecl {
            docs: vec![
                "The value being iterated — borrowed, so the pass is a view of it \
                 [proj-field]: nothing is copied at the mint [copy-opt-in], and the \
                 subject cannot be moved or mutated while the pass lives."
                    .to_string(),
            ],
            name: Ident {
                name: SUBJECT.to_string(),
                span: subject_field_span,
            },
            // [proj-field] `proj Subject`: the pass projects the subject.
            ty: proj_of(&subject.ty, subject_field_span),
            default: None,
            span: subject_field_span,
        });
    }
    for (name, ty, docs) in &snapshot_fields {
        fields.push(FieldDecl {
            docs: docs.clone(),
            name: Ident {
                name: name.clone(),
                span: subject_field_span,
            },
            // A per-field snapshot borrows the field too — except a Copy
            // scalar, whose copy is free and whose borrow would only cost a
            // deref [copy-scalar-free].
            ty: if is_copy_scalar(ty) { ty.clone() } else { proj_of(ty, subject_field_span) },
            default: None,
            span: subject_field_span,
        });
    }
    for field in &f.iter_state {
        fields.push(FieldDecl {
            docs: field.docs.clone(),
            name: field.name.clone(),
            ty: field.ty.clone(),
            // The initializer moves to `iter`, where it can read the subject.
            default: None,
            span: field.span,
        });
    }
    let pass_struct = StructDecl {
        docs: vec![format!(
            "The pass over `{}`, generated from its `iter fn next` [iter-fn].",
            base.name.name
        )],
        // [mod-export] The pass type is exported exactly when its `iter fn`
        // is: a `for` over the pass needs the *type* in scope, so hiding it
        // while exporting the function would make the iterator undrivable
        // from another module.
        exported: f.exported,
        name: pass_name.clone(),
        generic_canbe: Vec::new(),
        generics: f.generics.clone(),
        obligations: vec![Obligation {
            auto: false,
            group: TypeRef {
                at: None,
                binder: false,
                alias: None,
                value_args: Vec::new(),
                name: Ident {
                    name: "Yield".to_string(),
                    span: struct_span,
                },
                args: vec![
                    Type::Named {
                        qualifiers: vec![],
                        base: type_ref("self", span),
                    },
                    elem.clone(),
                ],
                from: Vec::new(),
                span,
            },
        }],
        auto_qualifiers: vec![type_ref("Mut", struct_span)],
        fields,
        // A generated pass is never itself declared linear; whether it owes
        // follows its subject (conditional containers, phase 3 step 6).
        linear: false,
        span: struct_span,
    };

    // 2. `iter`: mints a fresh pass over the subject, which stays usable — it
    //    is borrowed, not copied (user decision 2026-09-11): the pass is a view,
    //    linked to the subject by the ordinary fate rules [proj-infer], which is
    //    what keeps the two backends agreeing — a write to the subject during a
    //    drive is refused rather than differently visible [backend-parity]. An
    //    `iter fn` that wants a snapshot writes one: `state { rows: List<Int> =
    //    copy(c.rows) }` [copy-opt-in].
    let mut lit_fields: Vec<StructLitField> = Vec::new();
    if whole {
        lit_fields.push(StructLitField {
            kind: StructLitFieldKind::Named {
                name: Ident {
                    name: SUBJECT.to_string(),
                    span: subject_field_span,
                },
                value: Expr::Ident(Ident {
                    name: subject.name.name.clone(),
                    span: subject_read_span,
                }),
            },
            span: subject_field_span,
        });
    }
    // A snapshot reads the field once, here at the mint, into a `proj` field:
    // a borrow of the subject's field, costing nothing.
    for (field_name, pass_name) in &snapshots {
        // Distinct spans again: two nodes per snapshot field, and they must not
        // share keys with each other or with the nodes above.
        let base_span = spans.take();
        let read_span = spans.take();
        lit_fields.push(StructLitField {
            kind: StructLitFieldKind::Named {
                name: Ident {
                    name: pass_name.clone(),
                    span: read_span,
                },
                value: Expr::Field {
                    base: Box::new(Expr::Ident(Ident {
                        name: subject.name.name.clone(),
                        span: base_span,
                    })),
                    field: Ident {
                        name: field_name.clone(),
                        span: read_span,
                    },
                    span: read_span,
                },
            },
            span: read_span,
        });
    }
    for field in &f.iter_state {
        lit_fields.push(StructLitField {
            kind: StructLitFieldKind::Named {
                name: field.name.clone(),
                value: field
                    .default
                    .clone()
                    .unwrap_or(Expr::Error { span: field.span }),
            },
            span: field.span,
        });
    }
    let iter_fn = FnDecl {
        docs: vec![format!(
            "A fresh pass over [{}] [iter-pass], generated from its \
             `iter fn next` [iter-fn].",
            subject.name.name
        )],
        // [mod-export] Both halves inherit the `iter fn`'s own visibility.
        exported: f.exported,
        intrinsic: false,
        is_iter: false,
        is_send: false,
        iter_state: vec![],
        name: Ident {
            name: "iter".to_string(),
            span: iter_span,
        },
        scoped_to: None,
        structural: false,
        generics: f.generics.clone(),
        generic_canbe: f.generic_canbe.clone(),
        derived_return: None,
        params: vec![Param {
            name: subject.name.clone(),
            ty: subject.ty.clone(),
            variadic: false,
            implicit: false,
            span: subject.span,
        }],
        implicit_groups: vec![],
        effects: Some(vec![]),
        deductions: Some(vec![Deduction {
            target: DeductionTarget::Param { name: subject.name.clone(), path: Vec::new() },
            kind: DeductionKind::KeepAll,
            span: iter_span,
        }]),
        return_type: Some(pass_ty(true)),
        constructs: None,
        body: Some(Block {
            stmts: vec![Stmt::Expr(Expr::Return {
                value: Some(Box::new(Expr::StructLit {
                    ty: Some(pass_ty(true)),
                    fields: lit_fields,
                    span: lit_span,
                })),
                span: ret_span,
            })],
            span: iter_span,
        }),
        span: iter_span,
    };

    // 3. `next`: the author's body, with the pass's fields written out.
    let mut next_body = body;
    let mut rewrite = Rewrite {
        subject: subject.name.name.clone(),
        state: state_names,
        spans,
        snapshots,
        scan: false,
        used_fields: Vec::new(),
        used_whole: false,
        assigns_through: false,
    };
    rewrite.block(&mut next_body);
    // [yield-proj] An `iter fn` emitting borrowed elements names the
    // *subject* as their source (`Emitted (proj(b) T)`); in the
    // generated `next` the subject is reached through the pass, so the
    // source is the pass parameter — the borrow chains through its `proj`
    // field to the subject the caller holds.
    let next_return = f.return_type.clone().map(|mut t| {
        rename_proj_source(&mut t, &subject.name.name, PASS);
        t
    });
    let next_fn = FnDecl {
        docs: f.docs.clone(),
        // [mod-export] Both halves inherit the `iter fn`'s own visibility.
        exported: f.exported,
        intrinsic: false,
        is_iter: false,
        is_send: false,
        iter_state: vec![],
        name: f.name.clone(),
        // [cmp-canonical] An `iter fn` is never `@`-scoped: the form declares
        // a pass, and the generated halves inherit its plain name.
        scoped_to: None,
        structural: false,
        generics: f.generics.clone(),
        generic_canbe: f.generic_canbe.clone(),
        derived_return: next_return.as_ref().and_then(crate::parser::first_proj_source),
        params: vec![Param {
            name: Ident {
                name: PASS.to_string(),
                span: next_span,
            },
            ty: pass_ty(true),
            variadic: false,
            implicit: false,
            span: next_span,
        }],
        implicit_groups: f.implicit_groups.clone(),
        effects: f.effects.clone(),
        // Advancing mutates the pass and hands it back: that is what lets a
        // caller drive it further [iter-drive-in-place].
        deductions: Some(vec![Deduction {
            target: DeductionTarget::Param {
                name: Ident {
                    name: PASS.to_string(),
                    span: next_span,
                },
                path: Vec::new(),
            },
            kind: DeductionKind::Exhaustive {
                quals: vec![type_ref("Mut", next_span)],
                reapplied: Vec::new(),
            },
            span: next_span,
        }]),
        return_type: next_return.clone(),
        constructs: None,
        body: Some(next_body),
        span: f.span,
    };

    Some(vec![
        Item::Struct(pass_struct),
        Item::Fn(iter_fn),
        Item::Fn(next_fn),
    ])
}

fn type_ref(name: &str, span: Span) -> TypeRef {
    TypeRef {
        alias: None,
        at: None,
        binder: false,
        value_args: Vec::new(),
        name: Ident {
            name: name.to_string(),
            span,
        },
        args: vec![],
        from: Vec::new(),
        span,
    }
}

// ===== [test-decl] tests =====

/// The prefix of a synthesized test function [test-run]. Underscore-leading,
/// so it cannot collide with a name a program writes and reads as generated
/// wherever it surfaces.
pub const TEST_FN_PREFIX: &str = "__salvo_test_";

/// One expanded test: what the runner needs to call it and what the report
/// calls it [test-run].
#[derive(Clone, Debug, PartialEq)]
pub struct ExpandedTest {
    /// The name as written in the `test "…"` declaration.
    pub name: String,
    /// The synthesized function the harness calls — exported, so the
    /// generated harness module can reach it [test-run].
    pub fn_name: String,
    /// The span of the name literal, for diagnostics.
    pub span: Span,
}

/// [test-decl] [test-run] Expands every `test "name" { … }` in `module` into an
/// **exported, parameterless fn** declaring `[use, Throw<Failure>]`, and
/// returns one [`ExpandedTest`] per test in declaration order.
///
/// Done here, in the syntax crate and before resolution, for the reason the
/// `iter fn` expansion is: nothing downstream learns the form exists. A test
/// body is then an ordinary fn body, so effects, linearity, narrowing,
/// deductions and both emitters need no test-shaped rules at all — the
/// implicit powers of a test [test-body] *are* the declared effect list of the
/// fn it becomes (`use` for handler registration, `Throw<Failure>` for the
/// assertion channel), and the harness reads a failure off an ordinary `try`.
///
/// `mangled` distinguishes one module's tests from another's in the flat
/// function namespace the harness calls into: the module path with `.`
/// replaced by `_`.
///
/// Diagnostics: two tests with the same name in one file are an error at the
/// second [test-unique], and the duplicate is dropped.
pub fn expand_tests(module: &mut Module, mangled: &str) -> (Vec<ExpandedTest>, Vec<Diagnostic>) {
    let mut tests = Vec::new();
    let mut diags = Vec::new();
    let mut expanded: Vec<Item> = Vec::with_capacity(module.items.len());
    for item in std::mem::take(&mut module.items) {
        let Item::Test(test) = item else {
            expanded.push(item);
            continue;
        };
        // [test-unique] Names identify a test to the runner (`--list`, the
        // filter, the report), so two tests cannot share one.
        if let Some(prior) = tests.iter().find(|t: &&ExpandedTest| t.name == test.name) {
            let _ = prior;
            diags.push(Diagnostic::error(
                format!(
                    "a test called `{}` is already declared in this file: test names \
                     identify a test to the runner, so they have to differ \
                     [test-unique]",
                    test.name
                ),
                test.name_span,
            ));
            continue;
        }
        let fn_name = format!("{TEST_FN_PREFIX}{mangled}_{}", tests.len());
        let span = test.name_span;
        tests.push(ExpandedTest {
            name: test.name.clone(),
            fn_name: fn_name.clone(),
            span,
        });
        expanded.push(Item::Fn(test_fn(test, fn_name)));
    }
    module.items = expanded;
    (tests, diags)
}

/// The fn a `test` block becomes [test-run].
fn test_fn(test: TestDecl, fn_name: String) -> FnDecl {
    let span = test.name_span;
    // `Throw<Failure>`: the failure channel [test-fail]. `Failure` resolves
    // in a test file because `std.test` is implicitly available there
    // [test-implicit-import].
    let mut throw = type_ref("Throw", span);
    throw.args = vec![Type::Named {
        qualifiers: Vec::new(),
        base: type_ref("Failure", span),
    }];
    FnDecl {
        docs: test.docs,
        // Exported so the generated harness module can call it, and for no
        // other reason: the `test` declaration itself refuses `export`
        // [test-decl].
        exported: true,
        intrinsic: false,
        is_iter: false,
        iter_state: Vec::new(),
        is_send: false,
        name: Ident {
            name: fn_name,
            span,
        },
        scoped_to: None,
        structural: false,
        generics: Vec::new(),
        generic_canbe: Vec::new(),
        derived_return: None,
        params: Vec::new(),
        implicit_groups: Vec::new(),
        // [test-body] A test has `main`'s powers for registration: `use` is
        // available without declaring anything, which is what lets a test
        // register a fake (`use MemFs()`) the way an entry point does.
        effects: Some(vec![EffectRef::Use(span), EffectRef::Effect(throw)]),
        deductions: None,
        return_type: None,
        constructs: None,
        body: Some(test.body),
        span: test.span,
    }
}

/// The `T` of a declared `Emitted T | Finished`, in either arm order.
fn element_type(ty: Option<&Type>) -> Option<Type> {
    let Some(Type::Union { arms, .. }) = ty else {
        return None;
    };
    if arms.len() != 2 {
        return None;
    }
    let mut emitted = None;
    let mut finished = false;
    for arm in arms {
        match arm {
            Type::Named { qualifiers, base } if qualifiers.is_empty() && base.name.name == "Finished" => {
                finished = true;
            }
            Type::Named { qualifiers, base } if qualifiers.len() == 1 && qualifiers[0].name.name == "Emitted" => {
                emitted = Some(Type::Named {
                    qualifiers: vec![],
                    base: base.clone(),
                });
            }
            Type::QualifiedGroup {
                qualifiers, base, ..
            } if qualifiers.len() == 1 && qualifiers[0].name.name == "Emitted" => {
                emitted = Some((**base).clone());
            }
            _ => return None,
        }
    }
    if finished {
        emitted
    } else {
        None
    }
}

// ===================== the body rewrite =====================

/// Rewrites the subject and the `state` fields into field reads of the pass
/// parameter — and, in `scan` mode, *classifies* how the body uses the subject
/// so the pass can hold as little of it as possible.
///
/// One traversal serves both, deliberately: every `Expr` and `Stmt` variant is
/// matched **exhaustively**, so a new variant is a compile error here rather
/// than an unrewritten name that resolves to nothing — and a second walk would
/// have to be kept in step with this one by hand.
struct Rewrite {
    subject: String,
    state: Vec<String>,
    /// Distinct spans for the `__p` bases this rewrite synthesizes, continuing
    /// the declaration's allocation so no two synthesized nodes collide. Unused
    /// in `scan` mode, which rewrites nothing.
    spans: Spans,
    /// Subject field -> the pass field standing in for it [iter-fn]. Empty when
    /// the whole subject is kept (or when the body never reads it).
    snapshots: BTreeMap<String, String>,
    /// Scanning rather than rewriting: record, change nothing.
    scan: bool,
    /// What the scan found: the subject's fields read in the body, in first-use
    /// order.
    used_fields: Vec<String>,
    /// The body uses the subject as a *value* — passes it on, reads it as a
    /// whole — so no per-field snapshot can stand in for it.
    used_whole: bool,
    /// The body assigns through the subject. Kept whole so the standing
    /// [struct-mut] refusal fires with its own message, rather than the write
    /// silently landing on a snapshot field of the (mutable) pass.
    assigns_through: bool,
}

impl Rewrite {
    /// The pass field a bare name refers to: the subject itself, or a `state`
    /// field.
    fn field_of(&mut self, ident: &Ident) -> Option<Expr> {
        let field = if ident.name == self.subject {
            SUBJECT
        } else if self.state.iter().any(|s| *s == ident.name) {
            &ident.name
        } else {
            return None;
        }
        .to_string();
        let base_span = self.spans.take();
        Some(pass_field(&field, ident.span, base_span))
    }

    fn block(&mut self, block: &mut Block) {
        for stmt in &mut block.stmts {
            self.stmt(stmt);
        }
    }

    fn stmt(&mut self, stmt: &mut Stmt) {
        match stmt {
            Stmt::Let { value, .. } => self.expr(value),
            Stmt::Assign { target, value, .. } => {
                if self.scan && root_is(target, &self.subject) {
                    self.assigns_through = true;
                }
                self.expr(target);
                self.expr(value);
            }
            Stmt::Use { handler, .. } => self.expr(handler),
            Stmt::Rename(_) => {}
            Stmt::Expr(expr) => self.expr(expr),
        }
    }

    fn expr(&mut self, expr: &mut Expr) {
        // [iter-fn] `c.field` on the subject is the shape a snapshot can stand
        // in for, and it has to be caught *before* the base is visited — after
        // that the base is no longer the subject.
        if let Expr::Field { base, field, span } = expr {
            if matches!(&**base, Expr::Ident(id) if id.name == self.subject) {
                if self.scan {
                    if !self.used_fields.contains(&field.name) {
                        self.used_fields.push(field.name.clone());
                    }
                    return;
                }
                if let Some(pass_name) = self.snapshots.get(&field.name).cloned() {
                    let base_span = self.spans.take();
                    *expr = pass_field(&pass_name, *span, base_span);
                    return;
                }
            }
        }
        match expr {
            Expr::Ident(ident) => {
                if self.scan {
                    if ident.name == self.subject {
                        self.used_whole = true;
                    }
                    return;
                }
                if let Some(replacement) = self.field_of(ident) {
                    *expr = replacement;
                }
            }
            Expr::Int { .. }
            | Expr::Float { .. }
            | Expr::Bool { .. }
            | Expr::Char { .. }
            | Expr::Error { .. } => {}
            Expr::Str { parts, .. } => {
                for part in parts {
                    match part {
                        StrExprPart::Text(_) => {}
                        StrExprPart::Interp(inner) => self.expr(inner),
                    }
                }
            }
            Expr::Field { base, .. }
            | Expr::TupleIndex { base, .. }
            | Expr::NonNull { operand: base, .. }
            | Expr::IncDec { operand: base, .. }
            | Expr::Spread { operand: base, .. }
            | Expr::Unary { operand: base, .. } => self.expr(base),
            // [assert-fn] Both forms hold ordinary expressions, rewritten like
            // any others so an `iter fn` body may assert.
            Expr::Assert { cond, message, .. } => {
                self.expr(cond);
                if let Some(m) = message {
                    self.expr(m);
                }
            }
            Expr::Unreachable { message, .. } => {
                if let Some(m) = message {
                    self.expr(m);
                }
            }
            Expr::Scoped { base, .. } | Expr::EffectScoped { base, .. } => {
                if let Some(base) = base {
                    self.expr(base);
                }
            }
            Expr::Call {
                callee,
                args,
                named,
                ..
            } => {
                self.expr(callee);
                for arg in args {
                    self.expr(arg);
                }
                for arg in named {
                    self.expr(&mut arg.value);
                }
            }
            Expr::Index { base, index, .. } => {
                self.expr(base);
                self.expr(index);
            }
            Expr::ArrayLit { elems, .. }
            | Expr::SetLit { elems, .. }
            | Expr::Tuple { elems, .. } => {
                for elem in elems {
                    self.expr(elem);
                }
            }
            // [col-literal] A map literal's keys and values are both
            // ordinary expressions.
            Expr::MapLit { entries, .. } => {
                for (k, v) in entries {
                    self.expr(k);
                    self.expr(v);
                }
            }
            Expr::StructLit { fields, .. } => {
                for field in fields {
                    match &mut field.kind {
                        StructLitFieldKind::Named { value, .. } => self.expr(value),
                        StructLitFieldKind::Spread(value) => self.expr(value),
                    }
                }
            }
            Expr::Binary { lhs, rhs, .. } => {
                self.expr(lhs);
                self.expr(rhs);
            }
            Expr::Is { subject, .. } | Expr::Widen { subject, .. } => self.expr(subject),
            Expr::If {
                branches,
                else_block,
                ..
            } => {
                for (cond, block) in branches {
                    self.expr(cond);
                    self.block(block);
                }
                if let Some(block) = else_block {
                    self.block(block);
                }
            }
            Expr::WhenCond {
                branches,
                else_block,
                ..
            } => {
                for (cond, block) in branches {
                    self.expr(cond);
                    self.block(block);
                }
                self.block(else_block);
            }
            Expr::When {
                subject, branches, ..
            } => {
                self.expr(subject);
                for branch in branches {
                    self.block(&mut branch.body);
                }
            }
            Expr::While {
                cond,
                body,
                else_block,
                ..
            } => {
                self.expr(cond);
                self.block(body);
                if let Some(block) = else_block {
                    self.block(block);
                }
            }
            Expr::For {
                iterable,
                body,
                else_block,
                ..
            } => {
                self.expr(iterable);
                self.block(body);
                if let Some(block) = else_block {
                    self.block(block);
                }
            }
            Expr::Lambda { body, .. } => match body {
                LambdaBody::Expr(inner) => self.expr(inner),
                LambdaBody::Block(block) => self.block(block),
            },
            Expr::Try { body, .. } => self.block(body),
            // [expr-escape] The escapes carry an optional value expression.
            Expr::Return { value, .. } | Expr::Break { value, .. } => {
                if let Some(value) = value {
                    self.expr(value);
                }
            }
            Expr::Continue { .. } => {}
            // [elvis] Both sides are ordinary expressions.
            Expr::Elvis { subject, rhs, .. } => {
                self.expr(subject);
                self.expr(rhs);
            }
            Expr::SafeField { base, inner, .. } => {
                self.expr(base);
                self.expr(inner);
            }
            Expr::Placeholder { .. } => {}
            // [actor-spawn-expr] Every clause is an ordinary expression, so
            // an `iter fn` subject read inside one rewrites like any other.
            Expr::Spawn {
                handler,
                with_items,
                pool,
                ..
            } => {
                self.expr(handler);
                for handler in with_items {
                    self.expr(handler);
                }
                if let Some(pool) = pool {
                    self.expr(pool);
                }
            }
            // [actor-self-send] A leaf: the selector names the enclosing
            // handler, so there is no sub-expression to rewrite.
            Expr::SelfScoped { .. } => {}
            // [actor-replyto] The captures are expressions; the member name
            // is not one.
            Expr::ReplyTo { captures, .. } => {
                for capture in captures {
                    self.expr(capture);
                }
            }
            // [actor-waitfor] An ordinary block.
            Expr::WaitFor { body, .. } => self.block(body),
        }
    }
}

/// A field read of the pass parameter: `__p.<name>`.
///
/// The **base** takes a span of its own, never the original identifier's. The
/// checker's side tables are keyed by span, so a base sharing the read's span
/// answers to that read's narrowing — and a backend that unwraps a narrowing
/// physically then unwraps `__p` itself before reaching the field (a narrowed
/// `state` slot emitted `__p.as_mut().unwrap().inner…`, which rustc rejected).
/// The `Field` node keeps the original span, since that is what a diagnostic
/// about this read should point at [iter-fn].
fn pass_field(name: &str, span: Span, base_span: Span) -> Expr {
    Expr::Field {
        base: Box::new(Expr::Ident(Ident {
            name: PASS.to_string(),
            span: base_span,
        })),
        field: Ident {
            name: name.to_string(),
            span,
        },
        span,
    }
}

/// Whether an assignment target's root is `name` — `s`, `s.f`, `s.f[i]`, `s.0`.
fn root_is(target: &Expr, name: &str) -> bool {
    match target {
        Expr::Ident(id) => id.name == name,
        Expr::Field { base, .. } | Expr::TupleIndex { base, .. } | Expr::Index { base, .. } => {
            root_is(base, name)
        }
        _ => false,
    }
}

/// Bindings in `body` whose name is one of `reserved` — the subject or a
/// `state` field. Reported rather than renamed: a shadowing binding would
/// silently mean something other than the field it hides.
fn collect_shadowing(body: &Block, reserved: &[&Ident], out: &mut Vec<(String, Span)>) {
    fn pattern(pat: &Pattern, reserved: &[&Ident], out: &mut Vec<(String, Span)>) {
        match pat {
            Pattern::Ident(ident) => {
                if reserved.iter().any(|r| r.name == ident.name) {
                    out.push((ident.name.clone(), ident.span));
                }
            }
            Pattern::Tuple { elems, .. } => {
                for elem in elems {
                    pattern(elem, reserved, out);
                }
            }
            Pattern::Struct { fields, .. } => {
                for field in fields {
                    if reserved.iter().any(|r| r.name == field.binding.name) {
                        out.push((field.binding.name.clone(), field.binding.span));
                    }
                }
            }
        }
    }

    fn walk_block(block: &Block, reserved: &[&Ident], out: &mut Vec<(String, Span)>) {
        for stmt in &block.stmts {
            match stmt {
                Stmt::Let { pattern: pat, value, .. } => {
                    pattern(pat, reserved, out);
                    walk_expr(value, reserved, out);
                }
                Stmt::Assign { target, value, .. } => {
                    walk_expr(target, reserved, out);
                    walk_expr(value, reserved, out);
                }
                Stmt::Use { handler, .. } => walk_expr(handler, reserved, out),
                Stmt::Rename(_) => {}
                Stmt::Expr(expr) => walk_expr(expr, reserved, out),
            }
        }
    }

    fn walk_expr(expr: &Expr, reserved: &[&Ident], out: &mut Vec<(String, Span)>) {
        match expr {
            Expr::Is { binding, .. } => {
                if let Some(binding) = binding {
                    if reserved.iter().any(|r| r.name == binding.name) {
                        out.push((binding.name.clone(), binding.span));
                    }
                }
            }
            Expr::For { pattern: pat, body, else_block, .. } => {
                pattern(pat, reserved, out);
                walk_block(body, reserved, out);
                if let Some(block) = else_block {
                    walk_block(block, reserved, out);
                }
            }
            Expr::Lambda { params, body, .. } => {
                for param in params {
                    if reserved.iter().any(|r| r.name == param.name.name) {
                        out.push((param.name.name.clone(), param.name.span));
                    }
                }
                match body {
                    LambdaBody::Expr(inner) => walk_expr(inner, reserved, out),
                    LambdaBody::Block(block) => walk_block(block, reserved, out),
                }
            }
            Expr::If { branches, else_block, .. } => {
                for (cond, block) in branches {
                    walk_expr(cond, reserved, out);
                    walk_block(block, reserved, out);
                }
                if let Some(block) = else_block {
                    walk_block(block, reserved, out);
                }
            }
            Expr::WhenCond { branches, else_block, .. } => {
                for (cond, block) in branches {
                    walk_expr(cond, reserved, out);
                    walk_block(block, reserved, out);
                }
                walk_block(else_block, reserved, out);
            }
            Expr::When { subject, branches, .. } => {
                walk_expr(subject, reserved, out);
                for branch in branches {
                    if let Some(binding) = &branch.binding {
                        if reserved.iter().any(|r| r.name == binding.name) {
                            out.push((binding.name.clone(), binding.span));
                        }
                    }
                    walk_block(&branch.body, reserved, out);
                }
            }
            Expr::While { cond, body, else_block, .. } => {
                walk_expr(cond, reserved, out);
                walk_block(body, reserved, out);
                if let Some(block) = else_block {
                    walk_block(block, reserved, out);
                }
            }
            Expr::Try { body, .. } => walk_block(body, reserved, out),
            Expr::Return { value, .. } | Expr::Break { value, .. } => {
                if let Some(value) = value {
                    walk_expr(value, reserved, out);
                }
            }
            Expr::Continue { .. } => {}
            Expr::Elvis { subject, rhs, .. } => {
                walk_expr(subject, reserved, out);
                walk_expr(rhs, reserved, out);
            }
            Expr::SafeField { base, inner, .. } => {
                walk_expr(base, reserved, out);
                walk_expr(inner, reserved, out);
            }
            Expr::Placeholder { .. } => {}
            // [actor-spawn-expr] [actor-replyto] [actor-waitfor] The
            // asynchronous forms hold ordinary expressions and blocks.
            Expr::Spawn {
                handler,
                with_items,
                pool,
                ..
            } => {
                walk_expr(handler, reserved, out);
                for handler in with_items {
                    walk_expr(handler, reserved, out);
                }
                if let Some(pool) = pool {
                    walk_expr(pool, reserved, out);
                }
            }
            Expr::ReplyTo { captures, .. } => {
                for capture in captures {
                    walk_expr(capture, reserved, out);
                }
            }
            // [actor-self-send] A leaf: no sub-expressions, and the member
            // name is resolved against the handler rather than a scope.
            Expr::SelfScoped { .. } => {}
            Expr::WaitFor { body, .. } => walk_block(body, reserved, out),
            Expr::Call { callee, args, named, .. } => {
                walk_expr(callee, reserved, out);
                for arg in args {
                    walk_expr(arg, reserved, out);
                }
                for arg in named {
                    walk_expr(&arg.value, reserved, out);
                }
            }
            Expr::Binary { lhs, rhs, .. } => {
                walk_expr(lhs, reserved, out);
                walk_expr(rhs, reserved, out);
            }
            Expr::Field { base, .. }
            | Expr::TupleIndex { base, .. }
            | Expr::NonNull { operand: base, .. }
            | Expr::IncDec { operand: base, .. }
            | Expr::Spread { operand: base, .. }
            | Expr::Unary { operand: base, .. }
            | Expr::Widen { subject: base, .. } => walk_expr(base, reserved, out),
            Expr::Assert { cond, message, .. } => {
                walk_expr(cond, reserved, out);
                if let Some(m) = message {
                    walk_expr(m, reserved, out);
                }
            }
            Expr::Unreachable { message, .. } => {
                if let Some(m) = message {
                    walk_expr(m, reserved, out);
                }
            }
            Expr::Index { base, index, .. } => {
                walk_expr(base, reserved, out);
                walk_expr(index, reserved, out);
            }
            Expr::ArrayLit { elems, .. }
            | Expr::SetLit { elems, .. }
            | Expr::Tuple { elems, .. } => {
                for elem in elems {
                    walk_expr(elem, reserved, out);
                }
            }
            // [col-literal] Keys and values are ordinary expressions.
            Expr::MapLit { entries, .. } => {
                for (k, v) in entries {
                    walk_expr(k, reserved, out);
                    walk_expr(v, reserved, out);
                }
            }
            Expr::StructLit { fields, .. } => {
                for field in fields {
                    match &field.kind {
                        StructLitFieldKind::Named { value, .. } => {
                            walk_expr(value, reserved, out)
                        }
                        StructLitFieldKind::Spread(value) => walk_expr(value, reserved, out),
                    }
                }
            }
            Expr::Str { parts, .. } => {
                for part in parts {
                    if let StrExprPart::Interp(inner) = part {
                        walk_expr(inner, reserved, out);
                    }
                }
            }
            Expr::Scoped { base, .. } | Expr::EffectScoped { base, .. } => {
                if let Some(base) = base {
                    walk_expr(base, reserved, out);
                }
            }
            Expr::Int { .. }
            | Expr::Float { .. }
            | Expr::Bool { .. }
            | Expr::Char { .. }
            | Expr::Ident(_)
            | Expr::Error { .. } => {}
        }
    }

    walk_block(body, reserved, out);
}

/// [proj-field] `ty` under a `proj` qualifier: the pass struct's borrowed
/// field type.
fn proj_of(ty: &Type, span: Span) -> Type {
    match ty {
        Type::Named { qualifiers, base } => {
            let mut qs = vec![type_ref("proj", span)];
            qs.extend(qualifiers.iter().cloned());
            Type::Named { qualifiers: qs, base: base.clone() }
        }
        other => Type::QualifiedGroup {
            qualifiers: vec![type_ref("proj", span)],
            base: Box::new(other.clone()),
            span,
        },
    }
}

/// [copy-scalar-free] A bare native scalar, whose copy is free.
fn is_copy_scalar(ty: &Type) -> bool {
    matches!(
        ty,
        Type::Named { qualifiers, base }
            if qualifiers.is_empty()
                && base.args.is_empty()
                && matches!(
                    base.name.name.as_str(),
                    "Int" | "Long" | "Float" | "Double" | "Bool" | "Char" | "Byte"
                )
    )
}

/// [yield-proj] Renames the source of every `proj(old)` in `ty` to
/// `new`, in place.
fn rename_proj_source(ty: &mut Type, old: &str, new: &str) {
    fn in_ref(r: &mut TypeRef, old: &str, new: &str) {
        if r.name.name == "proj" {
            for from in &mut r.from {
                if from.name == old {
                    from.name = new.to_string();
                }
            }
        }
        for a in &mut r.args {
            rename_proj_source(a, old, new);
        }
    }
    match ty {
        Type::Named { qualifiers, base } => {
            for q in qualifiers {
                in_ref(q, old, new);
            }
            in_ref(base, old, new);
        }
        Type::QualifiedGroup { qualifiers, base, .. } => {
            for q in qualifiers {
                in_ref(q, old, new);
            }
            rename_proj_source(base, old, new);
        }
        Type::Union { arms, .. } | Type::Tuple { elems: arms, .. } => {
            for a in arms {
                rename_proj_source(a, old, new);
            }
        }
        Type::Array { elem, .. } | Type::Nullable { inner: elem, .. } => {
            rename_proj_source(elem, old, new)
        }
        Type::Fn { .. } => {}
    }
}

// ===== [cmp-auto] `auto` obligations and `auto fn` =====

/// [cmp-auto] The capability groups the compiler can generate, and the members
/// each one asks for — the expansion of `: auto Group<self>` into one bodiless
/// `auto fn` per member.
///
/// It duplicates what `core.compare` declares, and has to: this pass runs per
/// module, before any cross-module visibility exists, so the group's own member
/// list is not available here. **Keep the two in step** — the checker reads this
/// same table for its "can the compiler generate this member" test, so the
/// compiler cannot disagree with itself, only with std.
///
/// `Hashed` **brings `eq` with it** (user decision 2026-09-22): a hash
/// container buckets by `hash` and confirms by `eq`, so the pair is the unit and
/// a `hash` without its `eq` is useless. `Ordered` does **not**: no sorted
/// container consults equality — both hosts collapse by the comparator — so an
/// `eq` there would be a member nothing reads.
const AUTO_GROUPS: [(&str, &[&str]); 3] = [
    ("Ordered", &["cmp"]),
    ("Eq", &["eq"]),
    ("Hashed", &["hash", "eq"]),
];

/// [cmp-auto] The members `auto Group<self>` generates, if the compiler has a
/// generator for that group at all.
pub fn auto_members(group: &str) -> Option<&'static [&'static str]> {
    AUTO_GROUPS
        .iter()
        .find(|(g, _)| *g == group)
        .map(|(_, ms)| *ms)
}

/// [cmp-auto] Whether the compiler can write this member's body — the whole
/// list, which is what an `auto fn` of another name is refused against.
pub fn is_auto_member(member: &str) -> bool {
    AUTO_GROUPS.iter().any(|(_, ms)| ms.contains(&member))
}

/// [cmp-auto] Every member the compiler can generate, for a diagnostic that
/// names them.
pub fn auto_member_names() -> Vec<&'static str> {
    let mut out: Vec<&'static str> = Vec::new();
    for (_, ms) in AUTO_GROUPS {
        for m in ms {
            if !out.contains(m) {
                out.push(m);
            }
        }
    }
    out
}

/// [cmp-auto] Expands every `: auto Group<self>` into the `auto fn`
/// declarations it is sugar for: one `@`-scoped fn per member
/// [cmp-canonical], bodiless and marked `structural`, which each backend lowers
/// to the host's own derived comparison, equality or hash.
///
/// Done here, beside the `iter fn` expansion and for the same three reasons:
/// nothing downstream learns the form exists (the expansions are ordinary
/// overloads, so resolution, implicit filling, the canonical rules and the
/// duplicate check all apply unchanged), a hand-written member colliding with a
/// generated one *is* the ordinary duplicate error, and the emitters need no
/// notion of an obligation clause — only of a structural fn.
///
/// The written `auto fn` form needs no expansion at all: it *is* what this
/// produces, which is why the clause is sugar rather than a second mechanism.
///
/// An `auto` on a group with no generator expands to nothing; the checker
/// reports it, naming the members it can write.
pub fn expand_auto_obligations(module: &mut Module) {
    let mut generated: Vec<Item> = Vec::new();
    for item in &module.items {
        let Item::Struct(s) = item else { continue };
        let mut members: Vec<&str> = Vec::new();
        for ob in &s.obligations {
            if !ob.auto {
                continue;
            }
            // `self` is what the generator writes the signature over; an
            // `auto` naming someone else's type is the checker's error.
            if !ob
                .group
                .args
                .iter()
                .all(|a| matches!(a, Type::Named { base, .. } if base.name.name == "self"))
            {
                continue;
            }
            // [cmp-auto] Two groups asking for the same member ask for one
            // declaration, not two: `auto Eq<self>` beside `auto Hashed<self>`
            // is one `eq`, the same merge an overlapping spread gets.
            if let Some(ms) = auto_members(&ob.group.name.name) {
                for m in ms {
                    if !members.contains(m) {
                        members.push(m);
                    }
                }
            }
        }
        for member in members {
            generated.push(Item::Fn(structural_member(s, member)));
        }
    }
    module.items.extend(generated);
}

/// One generated canonical: `fn cmp@Point(a: Point, b: Point) [] -> Int => a, b`.
///
/// Spans point at the struct's own name, so a duplicate or an ineligible field
/// is reported where the `default` was written rather than at an invisible
/// declaration.
fn structural_member(s: &StructDecl, member: &str) -> FnDecl {
    let span = s.name.span;
    let self_ty = Type::Named {
        qualifiers: vec![],
        base: TypeRef {
            at: None,
            binder: false,
            alias: None,
            value_args: Vec::new(),
            name: s.name.clone(),
            args: s
                .generics
                .iter()
                .map(|g| Type::Named {
                    qualifiers: vec![],
                    base: type_ref(&g.name, span),
                })
                .collect(),
            from: Vec::new(),
            span,
        },
    };
    let param = |name: &str| Param {
        name: Ident {
            name: name.to_string(),
            span,
        },
        ty: self_ty.clone(),
        variadic: false,
        implicit: false,
        span,
    };
    let keep = |name: &str| Deduction {
        target: DeductionTarget::Param {
            name: Ident {
                name: name.to_string(),
                span,
            },
            path: Vec::new(),
        },
        kind: DeductionKind::KeepAll,
        span,
    };
    // Every member *keeps* its parameters: comparing or hashing a value reads
    // it [fn-contract], which is also the contract the group's member declares.
    let (params, deductions, ret) = match member {
        "hash" => (
            vec![param("value")],
            vec![keep("value")],
            Some("Long"),
        ),
        "eq" => (
            vec![param("a"), param("b")],
            vec![keep("a"), keep("b")],
            Some("Bool"),
        ),
        _ => (
            vec![param("a"), param("b")],
            vec![keep("a"), keep("b")],
            Some("Int"),
        ),
    };
    FnDecl {
        docs: vec![format!(
            "The structural `{member}` for [{}], generated from its `auto` \
             obligation [cmp-auto].",
            s.name.name
        )],
        // [mod-export] A generated canonical carries the struct's own
        // visibility, which is also what [cmp-canonical]'s export-match rule
        // demands of a hand-written one.
        exported: s.exported,
        intrinsic: false,
        is_iter: false,
        is_send: false,
        iter_state: vec![],
        name: Ident {
            name: member.to_string(),
            span,
        },
        scoped_to: Some(s.name.clone()),
        structural: true,
        generics: s.generics.clone(),
        generic_canbe: Vec::new(),
        derived_return: None,
        params,
        implicit_groups: vec![],
        effects: Some(vec![]),
        deductions: Some(deductions),
        return_type: ret.map(|r| Type::Named {
            qualifiers: vec![],
            base: type_ref(r, span),
        }),
        constructs: None,
        body: None,
        span,
    }
}
