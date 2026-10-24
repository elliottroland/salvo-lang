//! [iter-fn] Expanding an `iter fn` into ordinary declarations.
//!
//! An `iter fn` is the hand-written half of iteration [iter-protocol] with the
//! boilerplate removed: the author writes the step, and the **iterator struct
//! is generated**.
//!
//! ```text
//! iter fn range(start: Int, end: Int, step: Int) -> Emitted Int | Finished {
//!     state Range {
//!         i: Int = start
//!     }
//!     if step <= 0 || i >= end { return finished() }
//!     i += step
//!     return emitted(i - step)
//! }
//! ```
//!
//! becomes, before anything else in the compiler sees it:
//!
//! ```text
//! struct Range : Yield<self, Int> canbe Mut {
//!     start: Int, end: Int, step: Int,
//!     i: Int
//! }
//!
//! fn range(start: Int, end: Int, step: Int) [] -> Mut Range {
//!     return Mut Range { start: start, end: end, step: step, i: start }
//! }
//!
//! fn next(__p: Mut Range) [] -> Emitted Int | Finished => __p: Mut {
//!     if __p.step <= 0 || __p.i >= __p.end { return finished() }
//!     __p.i += __p.step
//!     return emitted(__p.i - __p.step)
//! }
//! ```
//!
//! The `iter fn` keeps its own **name and parameters** (user decision
//! 2026-09-26): the declaration is the *minter*, and its body is the step. A
//! call `range(0, 10)` mints an iterator of the struct its `state` block names
//! (user decision 2026-10-20), and `for i in range(0, 10)` drives it. The one
//! named `iter` with one parameter is the canonical minter an `: Iter<self,
//! It, T>` obligation asks for [iter-group].
//!
//! The struct holds every parameter — a Copy scalar owned, anything else
//! borrowed whole — so its shape is a function of the declaration's header.
//!
//! Three consequences are the reason it is done *here*, in the syntax crate,
//! rather than as a checker feature:
//!
//! * **Nothing downstream knows the form exists.** Resolution, the checker,
//!   the deduction pass, both emitters and the LSP see an ordinary iterator
//!   struct with an ordinary `next` — the shape they already support — so
//!   `for`, the combinators and `let p = range(3)` all work with no new
//!   machinery.
//! * **The `state` fields get every rule for free**, because they *are* struct
//!   fields: types, mutation, `Mut`, narrowing, deductions.
//! * **"No effects in an initializer" needs no check**: the initializers become
//!   the body of a minter declared `[]`, so an effectful call there is an
//!   ordinary effect error at that call.

use std::collections::BTreeMap;

use crate::ast::*;
use crate::diag::Diagnostic;
use crate::span::Span;

/// Expands every `iter fn` in `module` into its three declarations, in place.
/// Diagnostics are returned rather than thrown: an `iter fn` that cannot be
/// expanded is dropped, so the rest of the module still parses and checks.
pub fn expand_iter_fns(module: &mut Module) -> Vec<Diagnostic> {
    // [fn-attached] The struct-body hoist, run here because this is the one
    // entry point every consumer of a parsed module already calls — so no path
    // can forget it.
    hoist_struct_fns(module);
    let mut diags = Vec::new();
    let mut expanded: Vec<Item> = Vec::with_capacity(module.items.len());
    for item in std::mem::take(&mut module.items) {
        match item {
            Item::Fn(f) if f.is_iter => {
                if let Some(items) = expand(&f, &mut diags) {
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

/// The generated `next`'s parameter name.
const PASS: &str = "__p";

/// How the iterator struct holds one parameter of the `iter fn`: every
/// parameter is held (user decision 2026-10-20, R5).
enum Hold {
    /// Owned, as written: a Copy scalar, whose copy is free [copy-scalar-free].
    Owned,
    /// Borrowed whole: `proj T` [proj-field].
    Whole,
}

fn expand(f: &FnDecl, diags: &mut Vec<Diagnostic>) -> Option<Vec<Item>> {
    let span = f.name.span;
    let mut error = |message: String, span: Span| diags.push(Diagnostic::error(message, span));

    let Some(body) = f.body.clone() else {
        error(
            "an `iter fn` needs a body: it *is* the `next` the compiler would \
             otherwise generate"
                .to_string(),
            span,
        );
        return None;
    };
    if f.params.iter().any(|p| p.implicit) || !f.implicit_groups.is_empty() {
        // Round 6 of the design (a stage over a generic source) needs the
        // generated `next` to carry the forwarded implicits and the obligation
        // match to ignore them; not in this cut.
        error(
            "an `iter fn` cannot take implicit parameters yet: write the \
             iterator struct and its `next` by hand, or take the iterator as a \
             concrete type"
                .to_string(),
            span,
        );
        return None;
    }
    // Every parameter is held by the iterator struct, so each has to be a
    // named type the struct can hold [iter-fn].
    for p in &f.params {
        if p.variadic {
            error(
                format!(
                    "an `iter fn` cannot take a variadic parameter (`...{}`): the \
                     iterator struct holds each parameter as a field",
                    p.name.name
                ),
                p.span,
            );
            return None;
        }
        let Type::Named { qualifiers, .. } = &p.ty else {
            error(
                format!(
                    "an `iter fn`'s parameter `{}` must be a named type — the \
                     iterator struct holds it as a field; wrap an array, tuple or \
                     union in a struct of your own",
                    p.name.name
                ),
                p.span,
            );
            return None;
        };
        if let Some(q) = qualifiers.first() {
            // The parameters are ordinary data the caller keeps: a fresh
            // iterator is minted per call, so advancing never writes through
            // them. `Mut` would promise the opposite.
            error(
                format!(
                    "an `iter fn`'s parameter is read, not advanced: drop the `{}` \
                     from `{}` — the generated iterator struct holds the position, \
                     and each call mints a fresh one",
                    q.name.name, p.name.name
                ),
                p.span,
            );
            return None;
        }
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

    // Names the body may not rebind: the parameters and the state fields all
    // become fields of the iterator struct, and a shadowing local would
    // silently mean something else [iter-fn].
    let mut reserved: Vec<&Ident> = f.params.iter().map(|p| &p.name).collect();
    for (i, field) in f.iter_state.iter().enumerate() {
        if f.params.iter().any(|p| p.name.name == field.name.name) {
            error(
                format!(
                    "`{}` is a parameter's name: a `state` field may not shadow it",
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
                "`{name}` is already a name of `iter fn {}` (a parameter or a \
                 `state` field), and a binding may not reuse it: rename the binding",
                f.name.name
            ),
            *at,
        );
        return None;
    }

    // [iter-fn] Every generated declaration needs a **distinct name span**:
    // the checker's side tables (`fn_refs` → `fn_effects`, deductions, the LSP's
    // definition sites) are keyed by it, so two declarations sharing one span
    // collide and the later wins — which showed up as the generated minter
    // inheriting the `iter fn`'s effect list. Each borrows a different real
    // token of the source, so diagnostics still land somewhere meaningful.
    let mut spans = Spans::new(f.span);
    let struct_span = spans.take();
    let mint_span = spans.take();
    let next_span = span;
    let field_span = spans.take();
    let lit_span = spans.take();
    let ret_span = spans.take();
    // [iter-fn] The struct is named by the `state Name { … }` block (user
    // decision 2026-10-20, R3/R4) and declared beside the fn, so it collides
    // with a same-named type, or another `iter fn`'s state, in that scope.
    // The parser has reported a missing name.
    let pass_name = f.iter_state_name.clone()?;
    let pass_ty = |mutable: bool| Type::Named {
        qualifiers: if mutable {
            vec![type_ref("Mut", span)]
        } else {
            vec![]
        },
        base: TypeRef {
            at: None,
            binder: false,
            established: false,
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
                        established: false,
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

    // [iter-fn] The struct holds **every** parameter (user decision
    // 2026-10-20, R5): a Copy scalar owned, anything else borrowed whole
    // (`proj T`), so the struct's shape is a function of the header and not of
    // what the step happens to read. The borrow makes the iterator a view of
    // the value [proj-field], so the value cannot be written while the
    // iterator lives [proj-infer].
    let state_names: Vec<String> = f
        .iter_state
        .iter()
        .map(|field| field.name.name.clone())
        .collect();
    let holds: Vec<Hold> = f
        .params
        .iter()
        .map(|p| if is_copy_scalar(&p.ty) { Hold::Owned } else { Hold::Whole })
        .collect();

    // 1. the iterator struct: what it holds of each parameter, then the state
    //    fields.
    let mut fields = Vec::new();
    let mut lit_fields: Vec<StructLitField> = Vec::new();
    // Parameter name -> the hidden field holding it whole (or owned).
    let mut held: BTreeMap<String, String> = BTreeMap::new();
    for (p, hold) in f.params.iter().zip(&holds) {
        let ty = if matches!(hold, Hold::Owned) {
            p.ty.clone()
        } else {
            // [proj-field] `proj T`: the iterator projects the value.
            proj_of(&p.ty, field_span)
        };
        fields.push(FieldDecl {
            annotations: Vec::new(),
            docs: vec![format!(
                "The `{}` being iterated — {} [iter-fn].",
                p.name.name,
                if matches!(hold, Hold::Owned) {
                    "a scalar, owned"
                } else {
                    "borrowed, so the iterator is a view of it [proj-field]: \
                     nothing is copied at the mint, and the value cannot be \
                     moved or mutated while the iterator lives"
                }
            )],
            name: Ident {
                name: p.name.name.clone(),
                span: field_span,
            },
            ty: Some(ty),
            canbe_mut: false,
            default: None,
            span: field_span,
        });
        let read_span = spans.take();
        lit_fields.push(StructLitField {
            kind: StructLitFieldKind::Named {
                name: Ident {
                    name: p.name.name.clone(),
                    span: field_span,
                },
                value: Expr::Ident(Ident {
                    name: p.name.name.clone(),
                    span: read_span,
                }),
            },
            span: field_span,
        });
        held.insert(p.name.name.clone(), p.name.name.clone());
    }
    for field in &f.iter_state {
        // [state-infer] An `iter fn`'s state field keeps its written type, or
        // carries `None` for the inferred form (`at = 0`). The initializer
        // moves to the minter's returned struct literal (below), where it is
        // typed with the parameters in scope; the checker infers an untyped
        // field's type from that literal entry and records it at the field's
        // name span — the same channel handler state uses — and the generated
        // struct field, the `next` body's `__p.<field>` reads, and the IR all
        // read it back there. The generated struct field therefore keeps the
        // **original** `state` field-name span, which nothing else may reuse.
        fields.push(FieldDecl {
            annotations: Vec::new(),
            docs: field.docs.clone(),
            name: field.name.clone(),
            ty: field.ty.clone(),
            canbe_mut: field.canbe_mut,
            // The initializer moves to the minter, where it can read the
            // parameters.
            default: None,
            span: field.span,
        });
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
    // [qual-depend] A dependent claim in the element names a parameter
    // (`Emitted (+Idx(list) Int)`): in the obligation the place is the struct's
    // own field (`Idx(self.list)`), and in the generated `next` it is reached
    // through the parameter (`Idx(__p.list)`). A parameter the struct does not
    // hold cannot be claimed about, so a claim on one is held whole.
    let mut ob_elem = elem.clone();
    for p in &f.params {
        if let Some(hidden) = held.get(&p.name.name) {
            rename_value_arg_root(&mut ob_elem, &p.name.name, &format!("self.{hidden}"));
        }
    }
    let pass_struct = StructDecl {
        annotations: Vec::new(),
        fns: Vec::new(),
        // [noremote] An iterator holds a position into a value the frame owns;
        // it is never sent anywhere, so the flag is moot — off.
        noremote: false,
        comptime: false,
        docs: vec![format!(
            "The iterator struct of `iter fn {}`, generated from it [iter-fn].",
            f.name.name
        )],
        // [mod-export] The iterator struct is exported exactly when its `iter
        // fn` is: a `for` over the iterator needs the *type* in scope, so
        // hiding it while exporting the function would make the iterator
        // undrivable from another module.
        exported: f.exported,
        name: pass_name.clone(),
        generic_canbe: Vec::new(),
        fn_slots: Vec::new(),
        opaque: false,
        generics: f.generics.clone(),
        obligations: vec![Obligation {
            by: None,
            group: TypeRef {
                at: None,
                binder: false,
                established: false,
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
                    ob_elem,
                ],
                from: Vec::new(),
                span,
            },
        }],
        auto_qualifiers: vec![type_ref("Mut", struct_span)],
        fields,
        // A generated iterator struct is never itself declared linear (user
        // decision 2026-09-27: no generated discharger); whether it owes
        // follows what it holds, and a linear `state` field is the ordinary
        // [linear-composite] refusal at the struct.
        linear: false,
        span: struct_span,
    };

    // 2. the minter: the `iter fn`'s own name and parameters, minting a fresh
    //    iterator over them. The parameters stay usable — borrowed, not copied
    //    (user decision 2026-09-11): the iterator is a view, linked to them by
    //    the ordinary fate rules [proj-infer], which is what keeps the two
    //    backends agreeing — a write during a drive is refused rather than
    //    differently visible [backend-parity]. An `iter fn` that wants a
    //    snapshot writes one: `state { rows: List<Int> = copy(c.rows) }`
    //    [copy-opt-in].
    let mint_fn = FnDecl {
        docs: {
            let mut docs = f.docs.clone();
            docs.push(String::new());
            docs.push(format!(
                "Mints a fresh `{}` — generated from `iter fn {}` [iter-fn].",
                pass_name.name,
                f.name.name
            ));
            docs
        },
        // [mod-export] Both halves inherit the `iter fn`'s own visibility.
        exported: f.exported,
        intrinsic: false,
        platform: false,
        is_iter: false,
        is_send: false,
        iter_state: vec![],
        iter_state_name: Some(pass_name.clone()),
        name: Ident {
            name: f.name.name.clone(),
            span: mint_span,
        },
        type_name: None,
        scoped_to: None,
        compfn: None,
        by: None,
        stamped: None,
        generics: f.generics.clone(),
        generic_canbe: f.generic_canbe.clone(),
        derived_return: None,
        params: f.params.clone(),
        implicit_groups: vec![],
        effects: Some(vec![]),
        // Every parameter is kept — the struct borrows it — except a Copy
        // scalar, which is stored by value and whose fate is nothing to
        // deduce [copy-scalar-free].
        deductions: Some(
            f.params
                .iter()
                .zip(&holds)
                .filter(|(_, h)| !matches!(h, Hold::Owned))
                .map(|(p, _)| Deduction {
                    target: DeductionTarget::Param { name: p.name.clone(), path: Vec::new() },
                    kind: DeductionKind::KeepAll,
                    span: mint_span,
                })
                .collect(),
        ),
        return_type: Some(pass_ty(true)),
        response: None,
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
            span: mint_span,
        }),
        span: mint_span,
    };

    // 3. `next`: the author's body, with the struct's fields written out.
    let mut next_body = body;
    let mut rewrite = Rewrite {
        params: held.clone(),
        state: state_names,
        spans,
    };
    rewrite.block(&mut next_body);
    // [yield-proj] An `iter fn` emitting borrowed elements names the
    // *parameter* as their source (`Emitted (proj(list) T)`); in the
    // generated `next` the value is reached through the iterator, so the
    // source is the struct parameter — the borrow chains through its `proj`
    // field to the value the caller holds. [qual-depend] A dependent claim
    // about a parameter likewise becomes one about the struct's field.
    let next_return = f.return_type.clone().map(|mut t| {
        for p in &f.params {
            rename_proj_source(&mut t, &p.name.name, PASS);
            if let Some(hidden) = held.get(&p.name.name) {
                rename_value_arg_root(&mut t, &p.name.name, &format!("{PASS}.{hidden}"));
            }
        }
        t
    });
    // A `holds proj(list)` clause on the `iter fn` names a parameter the
    // generated `next` reaches through `__p`.
    let next_deductions: Vec<Deduction> = {
        let mut list = vec![Deduction {
            target: DeductionTarget::Param {
                name: Ident {
                    name: PASS.to_string(),
                    span: next_span,
                },
                path: Vec::new(),
            },
            // Advancing mutates the iterator and hands it back: that is what
            // lets a caller drive it further [iter-drive-in-place].
            kind: DeductionKind::Exhaustive {
                quals: vec![type_ref("Mut", next_span)],
                reapplied: Vec::new(),
            },
            span: next_span,
        }];
        for d in f.deductions.iter().flatten() {
            if let (DeductionTarget::Opaque, DeductionKind::Proj(sources)) = (&d.target, &d.kind) {
                let sources: Vec<Ident> = sources
                    .iter()
                    .map(|s| Ident {
                        name: PASS.to_string(),
                        span: s.span,
                    })
                    .collect();
                if !sources.is_empty() {
                    list.push(Deduction {
                        target: DeductionTarget::Opaque,
                        kind: DeductionKind::Proj(vec![sources[0].clone()]),
                        span: d.span,
                    });
                }
            }
        }
        list
    };
    let next_fn = FnDecl {
        docs: vec![format!(
            "The step of `iter fn {}`, generated from its body [iter-fn].",
            f.name.name
        )],
        // [mod-export] Both halves inherit the `iter fn`'s own visibility.
        exported: f.exported,
        intrinsic: false,
        platform: false,
        is_iter: false,
        is_send: false,
        iter_state: vec![],
        iter_state_name: Some(pass_name.clone()),
        name: Ident {
            name: "next".to_string(),
            span: next_span,
        },
        // [fn-attached] An `iter fn` is never `@`-scoped: the form declares an
        // iterator, and the generated halves inherit its plain name.
        type_name: None,
        scoped_to: None,
        compfn: None,
        by: None,
        stamped: None,
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
            default: None,
            span: next_span,
        }],
        implicit_groups: Vec::new(),
        effects: f.effects.clone(),
        deductions: Some(next_deductions),
        return_type: next_return.clone(),
        response: None,
        constructs: None,
        body: Some(next_body),
        span: f.span,
    };

    Some(vec![
        Item::Struct(pass_struct),
        Item::Fn(mint_fn),
        Item::Fn(next_fn),
    ])
}

fn type_ref(name: &str, span: Span) -> TypeRef {
    TypeRef {
        alias: None,
        at: None,
        binder: false,
        established: false,
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
    /// [test-kind] What the test runs on.
    pub kind: TestKind,
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
            kind: test.kind.clone(),
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
        platform: false,
        is_iter: false,
        iter_state: Vec::new(),
        iter_state_name: None,
        is_send: false,
        name: Ident {
            name: fn_name,
            span,
        },
        type_name: None,
        scoped_to: None,
        compfn: None,
        by: None,
        stamped: None,
        generics: Vec::new(),
        generic_canbe: Vec::new(),
        derived_return: None,
        params: Vec::new(),
        implicit_groups: Vec::new(),
        // [test-body] A test has `main`'s powers for registration: `use` is
        // available without declaring anything, which is what lets a test
        // register a fake (`use MemFs()`) the way an entry point does.
        // [test-actor] An actor test may also spawn: actors are what it tests.
        effects: Some(match test.kind {
            TestKind::Actor { .. } => vec![EffectRef::Use(span), EffectRef::Spawn(span), EffectRef::Effect(throw)],
            TestKind::Plain => vec![EffectRef::Use(span), EffectRef::Effect(throw)],
        }),
        deductions: None,
        return_type: None,
        response: None,
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

/// Rewrites the parameters and the `state` fields into field reads of the
/// iterator parameter.
///
/// Every `Expr` and `Stmt` variant is matched **exhaustively**, so a new
/// variant is a compile error here rather than an unrewritten name that
/// resolves to nothing.
struct Rewrite {
    /// Parameter name -> the field holding it.
    params: BTreeMap<String, String>,
    state: Vec<String>,
    /// Distinct spans for the `__p` bases this rewrite synthesizes, continuing
    /// the declaration's allocation so no two synthesized nodes collide.
    spans: Spans,
}

impl Rewrite {
    /// The struct field a bare name refers to: a parameter held whole, or a
    /// `state` field.
    fn field_of(&mut self, ident: &Ident) -> Option<Expr> {
        let field = if let Some(hidden) = self.params.get(&ident.name) {
            hidden.clone()
        } else if self.state.iter().any(|s| *s == ident.name) {
            ident.name.clone()
        } else {
            return None;
        };
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
                self.expr(target);
                self.expr(value);
            }
            Stmt::Use { handler, .. } => self.expr(handler),
            Stmt::Rename(_) => {}
            Stmt::Expr(expr) => self.expr(expr),
            // [comptime-inline] Gone before this walk runs (a compfn is
            // never an `iter fn`).
            Stmt::Comp(_) => {}
        }
    }

    fn expr(&mut self, expr: &mut Expr) {
        match expr {
            Expr::Ident(ident) => {
                if let Some(replacement) = self.field_of(ident) {
                    *expr = replacement;
                }
            }
            Expr::Int { .. }
            | Expr::Float { .. }
            | Expr::Bool { .. }
            | Expr::Char { .. }
            | Expr::FieldDefault { .. } | Expr::Error { .. } => {}
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
            | Expr::IncDec { operand: base, .. }
            | Expr::Spread { operand: base, .. }
            | Expr::Unary { operand: base, .. } => self.expr(base),
            Expr::SafeNav { base, access, .. } => {
                self.expr(base);
                if let SafeAccess::Call { args, named, .. } = access {
                    for arg in args {
                        self.expr(arg);
                    }
                    for arg in named {
                        self.expr(&mut arg.value);
                    }
                }
            }
            // [assert-fn] Both forms hold ordinary expressions, rewritten like
            // any others so an `iter fn` body may assert.
            Expr::Assert { cond, message, .. } => {
                self.expr(cond);
                if let Some(m) = message {
                    self.expr(m);
                }
            }
            Expr::WhyNot { test, .. } => self.expr(test),
            Expr::Unreachable { message, .. } | Expr::Todo { message, .. } => {
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
            Expr::Tuple { elems, .. } => {
                for elem in elems {
                    self.expr(elem);
                }
            }
            Expr::StructLit { fields, .. } => {
                for field in fields {
                    match &mut field.kind {
                        StructLitFieldKind::Named { value, .. } => self.expr(value),
                        StructLitFieldKind::Spread(value) => self.expr(value),
                        StructLitFieldKind::InlineFor { .. } => {}
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
                LambdaBody::Terminal => {}
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
            Expr::Pipe { lhs, rhs, .. } | Expr::SafePipe { lhs, rhs, .. } => {
                self.expr(lhs);
                self.expr(rhs);
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
            Expr::Use {
                handler,
                with_items,
                ..
            } => {
                self.expr(handler);
                for handler in with_items {
                    self.expr(handler);
                }
            }
            Expr::Project { base, .. } => self.expr(base),
            Expr::Merge { items, .. } => {
                for i in items {
                    self.expr(i);
                }
            }
            // [actor-self-send] A leaf: the selector names the enclosing
            // handler, so there is no sub-expression to rewrite.
            Expr::SelfScoped { .. } | Expr::SelfAddr { .. } => {}
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
                Stmt::Comp(_) => {}
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
                    LambdaBody::Terminal => {}
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
            Expr::Pipe { lhs, rhs, .. } | Expr::SafePipe { lhs, rhs, .. } => {
                walk_expr(lhs, reserved, out);
                walk_expr(rhs, reserved, out);
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
            Expr::Use {
                handler,
                with_items,
                ..
            } => {
                walk_expr(handler, reserved, out);
                for handler in with_items {
                    walk_expr(handler, reserved, out);
                }
            }
            Expr::Project { base, .. } => walk_expr(base, reserved, out),
            Expr::Merge { items, .. } => {
                for i in items {
                    walk_expr(i, reserved, out);
                }
            }
            Expr::ReplyTo { captures, .. } => {
                for capture in captures {
                    walk_expr(capture, reserved, out);
                }
            }
            // [actor-self-send] A leaf: no sub-expressions, and the member
            // name is resolved against the handler rather than a scope.
            Expr::SelfScoped { .. } | Expr::SelfAddr { .. } => {}
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
            | Expr::IncDec { operand: base, .. }
            | Expr::Spread { operand: base, .. }
            | Expr::Unary { operand: base, .. }
            | Expr::Widen { subject: base, .. } => walk_expr(base, reserved, out),
            Expr::SafeNav { base, access, .. } => {
                walk_expr(base, reserved, out);
                if let SafeAccess::Call { args, named, .. } = access {
                    for arg in args {
                        walk_expr(arg, reserved, out);
                    }
                    for arg in named {
                        walk_expr(&arg.value, reserved, out);
                    }
                }
            }
            Expr::Assert { cond, message, .. } => {
                walk_expr(cond, reserved, out);
                if let Some(m) = message {
                    walk_expr(m, reserved, out);
                }
            }
            Expr::WhyNot { test, .. } => walk_expr(test, reserved, out),
            Expr::Unreachable { message, .. } | Expr::Todo { message, .. } => {
                if let Some(m) = message {
                    walk_expr(m, reserved, out);
                }
            }
            Expr::Index { base, index, .. } => {
                walk_expr(base, reserved, out);
                walk_expr(index, reserved, out);
            }
            Expr::Tuple { elems, .. } => {
                for elem in elems {
                    walk_expr(elem, reserved, out);
                }
            }
            Expr::StructLit { fields, .. } => {
                for field in fields {
                    match &field.kind {
                        StructLitFieldKind::Named { value, .. } => {
                            walk_expr(value, reserved, out)
                        }
                        StructLitFieldKind::Spread(value) => walk_expr(value, reserved, out),
                        StructLitFieldKind::InlineFor { .. } => {}
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
            | Expr::FieldDefault { .. } | Expr::Error { .. } => {}
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
    crate::is_copy_scalar_type(ty)
}

/// [yield-proj] Renames the source of every `proj(old)` in `ty` to
/// `new`, in place.
fn rename_proj_source(ty: &mut Type, old: &str, new: &str) {
    fn in_ref(r: &mut TypeRef, old: &str, new: &str) {
        if r.name.name == "proj" || r.name.name == "ref" {
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
        Type::Literal { .. } => {}
        Type::Union { arms, .. } | Type::Tuple { elems: arms, .. } => {
            for a in arms {
                rename_proj_source(a, old, new);
            }
        }
        Type::Array { elem, .. } | Type::Nullable { inner: elem, .. } => {
            rename_proj_source(elem, old, new)
        }
        Type::Spread { inner, .. } => rename_proj_source(inner, old, new),
        Type::Fn { .. } => {}
    }
}

/// [qual-depend] Renames the root of every **place** value argument in `ty`
/// (`Idx(list)`, `Idx(list.items)`) from `old` to `new` — `Idx(__p.list)` in
/// the generated `next`, `Idx(self.list)` in the obligation.
fn rename_value_arg_root(ty: &mut Type, old: &str, new: &str) {
    fn in_ref(r: &mut TypeRef, old: &str, new: &str) {
        for v in &mut r.value_args {
            if let Type::Named { base, .. } = v {
                if base.args.is_empty() && base.at.is_none() && !base.binder {
                    let name = &base.name.name;
                    if name == old {
                        base.name.name = new.to_string();
                    } else if let Some(rest) = name.strip_prefix(old) {
                        if rest.starts_with('.') {
                            base.name.name = format!("{new}{rest}");
                        }
                    }
                }
            }
        }
        for a in &mut r.args {
            rename_value_arg_root(a, old, new);
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
            rename_value_arg_root(base, old, new);
        }
        Type::Literal { .. } => {}
        Type::Union { arms, .. } | Type::Tuple { elems: arms, .. } => {
            for a in arms {
                rename_value_arg_root(a, old, new);
            }
        }
        Type::Array { elem, .. } | Type::Nullable { inner: elem, .. } => {
            rename_value_arg_root(elem, old, new)
        }
        Type::Spread { inner, .. } => rename_value_arg_root(inner, old, new),
        Type::Fn { .. } => {}
    }
}

// ===== [fn-attached] struct-body fns =====

/// [fn-attached] Hoists every fn written inside a struct body to module level,
/// attached to the type and carrying its visibility. Hoisted rather than kept
/// nested because it *is* an ordinary top-level function in every other
/// respect — it overloads, it is called bare or with dot notation, and it
/// takes part in ranking. A `by` declaration [fn-by] and a concrete `compfn`
/// [namespace] Flattens every `namespace X { … }` into dotted top-level
/// declarations before anything else sees the module. A member `Thing` inside
/// `namespace X` becomes a top-level declaration named `X.Thing`; nested
/// namespaces compose (`X.Y.Thing`) [name-dot]. `export namespace X`
/// distributes `export` to every member (D6/D7).
///
/// Runs in `parse_module_deferred`, the chokepoint every parse goes through,
/// so the inline `iter fn`/test/struct-hoist passes and all of resolve, check
/// and the emitters only ever see the dotted names they already understand.
pub fn flatten_namespaces(module: &mut Module) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    let mut out = Vec::with_capacity(module.items.len());
    let mut namespaces = Vec::new();
    for item in std::mem::take(&mut module.items) {
        flatten_item(item, "", false, &mut out, &mut namespaces, &mut diags);
    }
    module.items = out;
    module.namespaces = namespaces;
    diags
}

/// Flattens one item under `prefix` (a dotted namespace path, or `""` at top
/// level). `force_export` is set by an enclosing `export namespace`.
fn flatten_item(
    item: Item,
    prefix: &str,
    force_export: bool,
    out: &mut Vec<Item>,
    namespaces: &mut Vec<String>,
    diags: &mut Vec<Diagnostic>,
) {
    // A name under the current prefix: `X.` + the declaration's own name.
    let under = |name: &str| {
        if prefix.is_empty() {
            name.to_string()
        } else {
            format!("{prefix}.{name}")
        }
    };
    match item {
        Item::Namespace(ns) => {
            // [name-casing] A namespace name is a single capitalized segment;
            // the parser already enforced the capital. A dotted namespace name
            // would come from a malformed parse — guard anyway.
            let inner_prefix = under(&ns.name.name);
            let inner_export = force_export || ns.exported;
            namespaces.push(inner_prefix.clone());
            for inner in ns.items {
                flatten_item(inner, &inner_prefix, inner_export, out, namespaces, diags);
            }
        }
        other => {
            let prefixed = prefix_item_name(other, prefix, force_export, diags);
            if let Some(prefixed) = prefixed {
                out.push(prefixed);
            }
        }
    }
}

/// Rewrites a non-namespace item's name to `prefix.Name` (when `prefix` is
/// non-empty) and applies `force_export`. Items that cannot live in a
/// namespace (imports, renames, refns, tests) are reported when they appear
/// under a prefix, and kept unprefixed so the rest of the file still parses.
fn prefix_item_name(
    item: Item,
    prefix: &str,
    force_export: bool,
    diags: &mut Vec<Diagnostic>,
) -> Option<Item> {
    let under = |name: &str| {
        if prefix.is_empty() {
            name.to_string()
        } else {
            format!("{prefix}.{name}")
        }
    };
    let in_namespace = !prefix.is_empty();
    macro_rules! reject {
        ($span:expr, $what:literal) => {{
            if in_namespace {
                diags.push(Diagnostic::error(
                    concat!(
                        "a `",
                        $what,
                        "` cannot appear inside a `namespace`: it is file- or \
                         module-scoped, not a named declaration [namespace]"
                    )
                    .to_string(),
                    $span,
                ));
            }
        }};
    }
    match item {
        Item::Struct(mut s) => {
            s.name.name = under(&s.name.name);
            s.exported = s.exported || force_export;
            Some(Item::Struct(s))
        }
        Item::Type(mut t) => {
            t.name.name = under(&t.name.name);
            t.exported = t.exported || force_export;
            Some(Item::Type(t))
        }
        Item::Qualifier(mut q) => {
            q.name.name = under(&q.name.name);
            q.exported = q.exported || force_export;
            Some(Item::Qualifier(q))
        }
        Item::Effect(mut e) => {
            e.name.name = under(&e.name.name);
            e.exported = e.exported || force_export;
            Some(Item::Effect(e))
        }
        Item::Handler(mut h) => {
            h.name.name = under(&h.name.name);
            h.exported = h.exported || force_export;
            Some(Item::Handler(h))
        }
        Item::Params(mut g) => {
            g.name.name = under(&g.name.name);
            g.exported = g.exported || force_export;
            Some(Item::Params(g))
        }
        Item::Fn(mut f) => {
            f.name.name = under(&f.name.name);
            f.exported = f.exported || force_export;
            Some(Item::Fn(f))
        }
        Item::Namespace(_) => unreachable!("namespaces are handled by flatten_item"),
        Item::Import(d) => {
            reject!(d.span, "import");
            Some(Item::Import(d))
        }
        Item::Refn(d) => {
            reject!(d.span, "refn");
            Some(Item::Refn(d))
        }
        Item::Rename(d) => {
            reject!(d.span, "rename");
            Some(Item::Rename(d))
        }
        Item::Test(d) => {
            reject!(d.span, "test");
            Some(Item::Test(d))
        }
        Item::EffectAlias(d) => {
            reject!(d.span, "effects alias");
            Some(Item::EffectAlias(d))
        }
    }
}

/// [comptime-bound] in the body travel the same way; the comptime expansion
/// finds them at module level afterwards.
pub fn hoist_struct_fns(module: &mut Module) {
    let mut generated: Vec<Item> = Vec::new();
    for item in &mut module.items {
        let Item::Struct(s) = item else { continue };
        let (exported, name) = (s.exported, s.name.clone());
        for mut f in std::mem::take(&mut s.fns) {
            f.exported = exported;
            f.scoped_to = Some(name.clone());
            generated.push(Item::Fn(f));
        }
    }
    module.items.extend(generated);
}

// ---------------------------------------------------------------------------
// A read-only walk over every statement and expression of a module, for the
// expansions that need to *find* something before they generate ([route-stub]
// looks for `use route_any(…)` and for `protocol<X>()`). Blocks nested in
// expressions are walked too; a declaration's body is walked in item order.

/// Calls `on_stmt` for every statement and `on_expr` for every expression in
/// `module`, outermost first.
pub fn walk_module(
    module: &Module,
    on_stmt: &mut dyn FnMut(&Stmt),
    on_expr: &mut dyn FnMut(&Expr),
) {
    for item in &module.items {
        match item {
            Item::Fn(f) => {
                if let Some(body) = &f.body {
                    walk_block_with(body, on_stmt, on_expr);
                }
            }
            Item::Handler(h) => {
                if let Some(m) = &h.mailbox {
                    walk_expr_with(m, on_stmt, on_expr);
                }
                for field in &h.state {
                    if let Some(d) = &field.default {
                        walk_expr_with(d, on_stmt, on_expr);
                    }
                }
                for f in &h.fns {
                    if let Some(body) = &f.body {
                        walk_block_with(body, on_stmt, on_expr);
                    }
                }
            }
            Item::Struct(s) => {
                for field in &s.fields {
                    if let Some(d) = &field.default {
                        walk_expr_with(d, on_stmt, on_expr);
                    }
                }
                for f in &s.fns {
                    if let Some(body) = &f.body {
                        walk_block_with(body, on_stmt, on_expr);
                    }
                }
            }
            Item::Test(t) => walk_block_with(&t.body, on_stmt, on_expr),
            // [namespace] Flattened in `parse_module_deferred`, so one should
            // not reach here; tolerate it by doing nothing (its members, once
            // flattened, are walked as ordinary items).
            Item::Namespace(_)
            | Item::Import(_)
            | Item::Type(_)
            | Item::Qualifier(_)
            | Item::Effect(_)
            | Item::EffectAlias(_)
            | Item::Params(_)
            | Item::Refn(_)
            | Item::Rename(_) => {}
        }
    }
}

fn walk_block_with(block: &Block, on_stmt: &mut dyn FnMut(&Stmt), on_expr: &mut dyn FnMut(&Expr)) {
    for stmt in &block.stmts {
        on_stmt(stmt);
        match stmt {
            Stmt::Let { value, .. } => walk_expr_with(value, on_stmt, on_expr),
            Stmt::Assign { target, value, .. } => {
                walk_expr_with(target, on_stmt, on_expr);
                walk_expr_with(value, on_stmt, on_expr);
            }
            Stmt::Use { handler, with_items, .. } => {
                walk_expr_with(handler, on_stmt, on_expr);
                for item in with_items {
                    walk_expr_with(item, on_stmt, on_expr);
                }
            }
            Stmt::Rename(_) => {}
            Stmt::Expr(expr) => walk_expr_with(expr, on_stmt, on_expr),
            // [comptime-inline] The comptime forms carry blocks of their own;
            // walked so a read-only scan sees inside a `compfn` too.
            Stmt::Comp(c) => match c {
                CompStmt::For { body, .. } | CompStmt::WhenArms { body, .. } => {
                    walk_block_with(body, on_stmt, on_expr)
                }
                CompStmt::If { then, else_, .. } => {
                    walk_block_with(then, on_stmt, on_expr);
                    if let Some(b) = else_ {
                        walk_block_with(b, on_stmt, on_expr);
                    }
                }
                CompStmt::WhenKind { arms, .. } => {
                    for arm in arms {
                        walk_block_with(&arm.body, on_stmt, on_expr);
                    }
                }
                CompStmt::WhenName { arms, .. } => {
                    for arm in arms {
                        walk_block_with(&arm.body, on_stmt, on_expr);
                    }
                }
                CompStmt::WhenType { arms, .. } => {
                    for arm in arms {
                        walk_block_with(&arm.body, on_stmt, on_expr);
                    }
                }
                CompStmt::Refuse { message, .. } => walk_expr_with(message, on_stmt, on_expr),
            },
        }
    }
}

fn walk_expr_with(expr: &Expr, on_stmt: &mut dyn FnMut(&Stmt), on_expr: &mut dyn FnMut(&Expr)) {
    on_expr(expr);
    let mut e = |x: &Expr| walk_expr_with(x, on_stmt, on_expr);
    match expr {
        Expr::For { iterable, body, else_block, .. } => {
            walk_expr_with(iterable, on_stmt, on_expr);
            walk_block_with(body, on_stmt, on_expr);
            if let Some(b) = else_block {
                walk_block_with(b, on_stmt, on_expr);
            }
        }
        Expr::Lambda { body, .. } => match body {
            LambdaBody::Expr(inner) => walk_expr_with(inner, on_stmt, on_expr),
            LambdaBody::Block(block) => walk_block_with(block, on_stmt, on_expr),
            LambdaBody::Terminal => {}
        },
        Expr::If { branches, else_block, .. } => {
            for (cond, block) in branches {
                walk_expr_with(cond, on_stmt, on_expr);
                walk_block_with(block, on_stmt, on_expr);
            }
            if let Some(b) = else_block {
                walk_block_with(b, on_stmt, on_expr);
            }
        }
        Expr::WhenCond { branches, else_block, .. } => {
            for (cond, block) in branches {
                walk_expr_with(cond, on_stmt, on_expr);
                walk_block_with(block, on_stmt, on_expr);
            }
            walk_block_with(else_block, on_stmt, on_expr);
        }
        Expr::When { subject, branches, .. } => {
            walk_expr_with(subject, on_stmt, on_expr);
            for branch in branches {
                walk_block_with(&branch.body, on_stmt, on_expr);
            }
        }
        Expr::While { cond, body, else_block, .. } => {
            walk_expr_with(cond, on_stmt, on_expr);
            walk_block_with(body, on_stmt, on_expr);
            if let Some(b) = else_block {
                walk_block_with(b, on_stmt, on_expr);
            }
        }
        Expr::Try { body, .. } | Expr::WaitFor { body, .. } => walk_block_with(body, on_stmt, on_expr),
        Expr::Return { value, .. } | Expr::Break { value, .. } => {
            if let Some(v) = value {
                e(v);
            }
        }
        Expr::Elvis { subject, rhs, .. } => {
            e(subject);
            e(rhs);
        }
        Expr::Pipe { lhs, rhs, .. } | Expr::SafePipe { lhs, rhs, .. } => {
            e(lhs);
            e(rhs);
        }
        Expr::Spawn { handler, with_items, pool, .. } => {
            e(handler);
            for h in with_items {
                e(h);
            }
            if let Some(p) = pool {
                e(p);
            }
        }
        Expr::Use { handler, with_items, .. } => {
            e(handler);
            for h in with_items {
                e(h);
            }
        }
        Expr::Project { base, .. } => e(base),
        Expr::Merge { items, .. } => {
            for i in items {
                e(i);
            }
        }
        Expr::ReplyTo { captures, .. } => {
            for c in captures {
                e(c);
            }
        }
        Expr::Call { callee, args, named, .. } => {
            e(callee);
            for a in args {
                e(a);
            }
            for a in named {
                e(&a.value);
            }
        }
        Expr::Binary { lhs, rhs, .. } => {
            e(lhs);
            e(rhs);
        }
        Expr::Field { base, .. }
        | Expr::TupleIndex { base, .. }
        | Expr::IncDec { operand: base, .. }
        | Expr::Spread { operand: base, .. }
        | Expr::Unary { operand: base, .. }
        | Expr::Widen { subject: base, .. }
        | Expr::Is { subject: base, .. } => e(base),
        Expr::SafeNav { base, access, .. } => {
            e(base);
            if let SafeAccess::Call { args, named, .. } = access {
                for a in args {
                    e(a);
                }
                for a in named {
                    e(&a.value);
                }
            }
        }
        Expr::Assert { cond, message, .. } => {
            e(cond);
            if let Some(m) = message {
                e(m);
            }
        }
        Expr::WhyNot { test, .. } => e(test),
        Expr::Unreachable { message, .. } | Expr::Todo { message, .. } => {
            if let Some(m) = message {
                e(m);
            }
        }
        Expr::Index { base, index, .. } => {
            e(base);
            e(index);
        }
        Expr::Tuple { elems, .. } => {
            for x in elems {
                e(x);
            }
        }
        Expr::StructLit { fields, .. } => {
            for field in fields {
                match &field.kind {
                    StructLitFieldKind::Named { value, .. } => e(value),
                    StructLitFieldKind::Spread(value) => e(value),
                    StructLitFieldKind::InlineFor { entries, .. } => {
                        for entry in entries {
                            if let StructLitFieldKind::Named { value, .. } = &entry.kind {
                                e(value);
                            }
                        }
                    }
                }
            }
        }
        Expr::Str { parts, .. } => {
            for part in parts {
                if let StrExprPart::Interp(inner) = part {
                    e(inner);
                }
            }
        }
        Expr::Scoped { base, .. } | Expr::EffectScoped { base, .. } => {
            if let Some(b) = base {
                e(b);
            }
        }
        Expr::Int { .. }
        | Expr::Float { .. }
        | Expr::Bool { .. }
        | Expr::Char { .. }
        | Expr::Ident(_)
        | Expr::Placeholder { .. }
        | Expr::Continue { .. }
        | Expr::SelfScoped { .. }
        | Expr::SelfAddr { .. }
        | Expr::FieldDefault { .. } | Expr::Error { .. } => {}
    }
}
