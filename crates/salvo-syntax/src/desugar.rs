//! [iter-fn] Expanding an `iter fn` into ordinary declarations.
//!
//! An `iter fn` is the hand-written half of iteration [iter-protocol] with the
//! boilerplate removed: the author writes the step, and the **iterator struct
//! is generated**.
//!
//! ```text
//! iter fn range(start: Int, end: Int, step: Int) -> Emitted Int | Finished {
//!     state {
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
//! struct __Iter_range_Int_Int_Int : Yield<self, Int> canbe Mut {
//!     start: Int, end: Int, step: Int,
//!     i: Int
//! }
//!
//! fn range(start: Int, end: Int, step: Int) [] -> Mut __Iter_range_Int_Int_Int
//! => start, end, step {
//!     return Mut __Iter_range_Int_Int_Int { start: start, end: end, step: step, i: start }
//! }
//!
//! fn next(__p: Mut __Iter_range_Int_Int_Int) [] -> Emitted Int | Finished => __p: Mut {
//!     if __p.step <= 0 || __p.i >= __p.end { return finished() }
//!     __p.i += __p.step
//!     return emitted(__p.i - __p.step)
//! }
//! ```
//!
//! The `iter fn` keeps its own **name and parameters** (user decision
//! 2026-09-26): the declaration is the *minter*, and its body is the step. A
//! call `range(0, 10)` mints an iterator — its type is the hidden struct, spelled
//! `iter Int` [iter-type] — and `for i in range(0, 10)` drives it. The one named
//! `iter` with one parameter is the canonical minter an `: Iter<self, T>`
//! obligation asks for [iter-group].
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
//!
//! The generated struct is named `__Iter_<fn>_<param types>` — unnameable,
//! since a type reference beginning with `_` is refused [iter-fn] — which is
//! what keeps the first boundary the design rests on: an iterator struct you
//! must *name* is still written by hand.

use std::collections::BTreeMap;

use crate::ast::*;
use crate::diag::Diagnostic;
use crate::span::Span;

/// Expands every `iter fn` in `module` into its three declarations, in place.
/// Diagnostics are returned rather than thrown: an `iter fn` that cannot be
/// expanded is dropped, so the rest of the module still parses and checks.
pub fn expand_iter_fns(module: &mut Module) -> Vec<Diagnostic> {
    let groups = params_decls(module);
    expand_iter_fns_with(module, &[], &groups)
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
    groups: &[ParamsDecl],
) -> Vec<Diagnostic> {
    // [fn-attached] The struct-body hoist, run here because this is the one
    // entry point every consumer of a parsed module already calls — so no path
    // can forget it.
    hoist_struct_fns(module);
    // [iter-type] The placeholder's hidden-generic readings, on every fn —
    // after the struct-body fns are hoisted, so they are covered too.
    hoist_iter_types(module, groups);
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

/// The generated `next`'s parameter name.
const PASS: &str = "__p";

/// The prefix of a generated iterator struct's name. Underscore-leading, so no
/// program can write it [iter-fn].
pub const ITER_STRUCT_PREFIX: &str = "__Iter_";

/// How the generated iterator struct holds one parameter of the `iter fn`.
enum Hold {
    /// Not at all: the body never reads it (a `state` initializer may — that
    /// runs in the minter, where the parameter is in scope).
    Nothing,
    /// Owned, as written: a Copy scalar, whose copy is free [copy-scalar-free].
    Owned,
    /// Borrowed whole: `proj T` [proj-field].
    Whole,
    /// One `proj` field per field the body reads, by (field, hidden name,
    /// field type, docs) — the subject's declaration was visible and the body
    /// only ever reads plain fields of it.
    Fields(Vec<(String, String, Type, Vec<String>)>),
}

fn expand(
    f: &FnDecl,
    structs: &[StructDecl],
    diags: &mut Vec<Diagnostic>,
) -> Option<Vec<Item>> {
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
    // named type the struct can be named after and can hold [iter-fn].
    let mut bases: Vec<String> = Vec::new();
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
        let Type::Named { qualifiers, base } = &p.ty else {
            error(
                format!(
                    "an `iter fn`'s parameter `{}` must be a named type — the \
                     generated iterator struct holds it as a field and is named after \
                     it; wrap an array, tuple or union in a struct of your own",
                    p.name.name
                ),
                p.span,
            );
            return None;
        };
        if let Some(q) = qualifiers.first() {
            if q.name.name == "iter" {
                error(
                    format!(
                        "an `iter fn` cannot take an iterator (`{}: {}`) yet: the \
                         generated `next` would have to drive a generic iterator — \
                         write the iterator struct by hand, or take a concrete \
                         iterator struct",
                        p.name.name,
                        type_display(&p.ty)
                    ),
                    p.span,
                );
                return None;
            }
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
        bases.push(base.name.name.replace('.', ""));
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
                "`{name}` is a field of the generated iterator struct, so a binding \
                 of that name would shadow it: rename the binding"
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
    // Keyed by the fn's name *and* its parameter types, since an `iter fn`
    // overloads like any fn (`iter fn iter(list: List<T>)` beside `iter fn
    // iter(set: Set<T>)`).
    let mut struct_name = format!("{ITER_STRUCT_PREFIX}{}", f.name.name);
    for b in &bases {
        struct_name.push('_');
        struct_name.push_str(b);
    }
    let pass_name = Ident {
        name: struct_name,
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

    // [iter-fn] How much of each parameter the iterator struct has to hold. It
    // exists to carry the position, and a parameter rides along only because
    // the body may read it on any turn — so the cheapest correct answer is
    // looked for first, in three tiers:
    //
    //   1. the body never reads it (a plain counter): hold *nothing*;
    //   2. it only ever reads plain fields of it, and their types are visible:
    //      hold one snapshot field per field read;
    //   3. anything else — the value passed on, an assignment through it, a
    //      generic type (whose field types would need substituting), a
    //      declaration the expansion cannot see: hold it whole. (A single-file
    //      parse sees this file only; the program-level expansion the CLI
    //      drives sees every file's declarations.)
    //
    // All three are observationally identical, because the struct *borrows*: it
    // is a view of the value [proj-field], so the value cannot be written while
    // the iterator lives [proj-infer], and a per-field borrow at the mint says
    // exactly what a whole borrow says. A Copy scalar is simply owned.
    let state_names: Vec<String> = f
        .iter_state
        .iter()
        .map(|field| field.name.name.clone())
        .collect();
    let param_names: Vec<String> = f.params.iter().map(|p| p.name.name.clone()).collect();
    let mut scan = Rewrite {
        params: param_names.iter().map(|n| (n.clone(), n.clone())).collect(),
        state: state_names.clone(),
        // Scan mode synthesizes nothing, so this allocator is never drawn on;
        // the rewrite below gets the live one.
        spans: Spans::new(f.span),
        snapshots: BTreeMap::new(),
        scan: true,
        used_fields: BTreeMap::new(),
        used_whole: Vec::new(),
        assigns_through: Vec::new(),
    };
    let mut scanned = body.clone();
    scan.block(&mut scanned);
    // [qual-depend] A dependent claim in the element names a parameter
    // (`Emitted (+Idx(list) Int)`); the struct must *hold* that parameter for
    // the claim to have a place to name, whether or not the body reads it.
    if let Some(rt) = &f.return_type {
        let mut roots = Vec::new();
        value_arg_roots(rt, &mut roots);
        for root in roots {
            if param_names.contains(&root) && !scan.used_whole.contains(&root) {
                scan.used_whole.push(root);
            }
        }
    }
    let mut holds: Vec<Hold> = Vec::new();
    for p in &f.params {
        let name = &p.name.name;
        if is_copy_scalar(&p.ty) {
            holds.push(Hold::Owned);
            continue;
        }
        let used_fields = scan.used_fields.get(name).cloned().unwrap_or_default();
        let whole = scan.used_whole.contains(name) || scan.assigns_through.contains(name);
        if !whole && used_fields.is_empty() {
            holds.push(Hold::Nothing);
            continue;
        }
        let Type::Named { base, .. } = &p.ty else { unreachable!() };
        let decl = if base.args.is_empty() {
            structs.iter().find(|s| s.name.name == base.name.name)
        } else {
            None
        };
        let mut fields: Vec<(String, String, Type, Vec<String>)> = Vec::new();
        let mut ok = !whole && decl.is_some();
        if ok {
            let decl = decl.unwrap();
            for used in &used_fields {
                let Some(field) = decl.fields.iter().find(|f| f.name.name == *used) else {
                    // Not a field of the parameter: keep it whole, so the
                    // checker reports the real mistake against the real type.
                    ok = false;
                    break;
                };
                // The snapshot keeps the field's own name where that is free,
                // since the generated code is read by people; a clash with a
                // state field or a parameter pushes it into the compiler's
                // namespace instead.
                let hidden = if state_names.iter().any(|s| s == used)
                    || param_names.iter().any(|s| s == used)
                {
                    format!("__s_{used}")
                } else {
                    used.clone()
                };
                fields.push((used.clone(), hidden, field.ty.clone(), field.docs.clone()));
            }
        }
        holds.push(if ok { Hold::Fields(fields) } else { Hold::Whole });
    }
    // Two parameters snapshotting a field of the same name would collide;
    // hold the second whole rather than invent a third namespace.
    {
        let mut seen: Vec<String> = Vec::new();
        for h in holds.iter_mut() {
            if let Hold::Fields(fields) = h {
                if fields.iter().any(|(_, hidden, _, _)| seen.contains(hidden)) {
                    *h = Hold::Whole;
                    continue;
                }
                seen.extend(fields.iter().map(|(_, hidden, _, _)| hidden.clone()));
            }
        }
    }

    // 1. the iterator struct: what it holds of each parameter, then the state
    //    fields.
    let mut fields = Vec::new();
    let mut lit_fields: Vec<StructLitField> = Vec::new();
    // Parameter name -> (subject field -> hidden field), for the rewrite.
    let mut snapshots: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    // Parameter name -> the hidden field holding it whole (or owned).
    let mut held: BTreeMap<String, String> = BTreeMap::new();
    for (p, hold) in f.params.iter().zip(&holds) {
        match hold {
            Hold::Nothing => {}
            Hold::Owned | Hold::Whole => {
                let ty = if matches!(hold, Hold::Owned) {
                    p.ty.clone()
                } else {
                    // [proj-field] `proj T`: the iterator projects the value.
                    proj_of(&p.ty, field_span)
                };
                fields.push(FieldDecl {
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
                    ty,
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
            Hold::Fields(list) => {
                let mut map = BTreeMap::new();
                for (used, hidden, ty, docs) in list {
                    fields.push(FieldDecl {
                        docs: docs.clone(),
                        name: Ident {
                            name: hidden.clone(),
                            span: field_span,
                        },
                        // A per-field snapshot borrows the field too — except a
                        // Copy scalar, whose copy is free and whose borrow would
                        // only cost a deref [copy-scalar-free].
                        ty: if is_copy_scalar(ty) { ty.clone() } else { proj_of(ty, field_span) },
                        canbe_mut: false,
                        default: None,
                        span: field_span,
                    });
                    // A snapshot reads the field once, here at the mint, into a
                    // `proj` field: a borrow of the value's field, costing
                    // nothing. Distinct spans again: two nodes per snapshot
                    // field, and they must not share keys with each other or
                    // with the nodes above.
                    let base_span = spans.take();
                    let read_span = spans.take();
                    lit_fields.push(StructLitField {
                        kind: StructLitFieldKind::Named {
                            name: Ident {
                                name: hidden.clone(),
                                span: read_span,
                            },
                            value: Expr::Field {
                                base: Box::new(Expr::Ident(Ident {
                                    name: p.name.name.clone(),
                                    span: base_span,
                                })),
                                field: Ident {
                                    name: used.clone(),
                                    span: read_span,
                                },
                                span: read_span,
                            },
                        },
                        span: read_span,
                    });
                    map.insert(used.clone(), hidden.clone());
                }
                snapshots.insert(p.name.name.clone(), map);
            }
        }
    }
    for field in &f.iter_state {
        fields.push(FieldDecl {
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
                "Mints a fresh iterator (`iter {}`) — generated from `iter fn {}` [iter-fn].",
                type_display(&elem),
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
        name: Ident {
            name: f.name.name.clone(),
            span: mint_span,
        },
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
        snapshots,
        scan: false,
        used_fields: BTreeMap::new(),
        used_whole: Vec::new(),
        assigns_through: Vec::new(),
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
        name: Ident {
            name: "next".to_string(),
            span: next_span,
        },
        // [fn-attached] An `iter fn` is never `@`-scoped: the form declares an
        // iterator, and the generated halves inherit its plain name.
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
            span: next_span,
        }],
        implicit_groups: Vec::new(),
        effects: f.effects.clone(),
        deductions: Some(next_deductions),
        return_type: next_return.clone(),
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

/// A short rendering of a type for generated documentation.
fn type_display(ty: &Type) -> String {
    format!("{ty}")
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

// ===== [iter-type] the `iter T` placeholder =====

/// [iter-type] Splits `iter T` off a written type: the qualifiers written
/// *before* `iter` (refused by the checker — `Mut iter T` applies `Mut`
/// twice), and the element type — the base with the qualifiers written after
/// `iter`. `None` when the type is not an `iter` placeholder at its top level.
pub fn split_iter_qualifier(ty: &Type) -> Option<(Vec<TypeRef>, Type)> {
    match ty {
        Type::Named { qualifiers, base } => {
            let at = qualifiers.iter().position(|q| q.name.name == "iter")?;
            let before = qualifiers[..at].to_vec();
            let inner = Type::Named {
                qualifiers: qualifiers[at + 1..].to_vec(),
                base: base.clone(),
            };
            Some((before, inner))
        }
        Type::QualifiedGroup {
            qualifiers,
            base,
            span,
        } => {
            let at = qualifiers.iter().position(|q| q.name.name == "iter")?;
            let before = qualifiers[..at].to_vec();
            let after = qualifiers[at + 1..].to_vec();
            let inner = if after.is_empty() {
                (**base).clone()
            } else {
                Type::QualifiedGroup {
                    qualifiers: after,
                    base: base.clone(),
                    span: *span,
                }
            };
            Some((before, inner))
        }
        _ => None,
    }
}

/// [iter-type] Whether a written type mentions the `iter T` placeholder
/// anywhere.
pub fn type_mentions_iter(ty: &Type) -> bool {
    fn in_ref(r: &TypeRef) -> bool {
        r.name.name == "iter" || r.args.iter().any(type_mentions_iter)
    }
    match ty {
        Type::Named { qualifiers, base } => qualifiers.iter().any(in_ref) || in_ref(base),
        Type::QualifiedGroup {
            qualifiers, base, ..
        } => qualifiers.iter().any(in_ref) || type_mentions_iter(base),
        Type::Union { arms, .. } | Type::Tuple { elems: arms, .. } => {
            arms.iter().any(type_mentions_iter)
        }
        Type::Array { elem, .. } | Type::Nullable { inner: elem, .. } => type_mentions_iter(elem),
        Type::Fn { params, ret, .. } => {
            params.iter().any(type_mentions_iter) || type_mentions_iter(ret)
        }
        Type::Literal { .. } => false,
    }
}

/// [iter-group] Whether a `params` group's members mention the `iter T`
/// placeholder — the group then spreads with one **hidden type argument**
/// standing for the iterator struct.
pub fn group_mentions_iter(group: &ParamsDecl) -> bool {
    group.fns.iter().any(|m| {
        m.params.iter().any(|p| type_mentions_iter(&p.ty))
            || m.return_type.as_ref().is_some_and(type_mentions_iter)
    })
}

/// The `params` group declarations of a module, cloned — what a program-level
/// expansion passes to `expand_iter_fns_with` as the *other* files'.
pub fn params_decls(module: &Module) -> Vec<ParamsDecl> {
    module
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Params(g) => Some(g.clone()),
            _ => None,
        })
        .collect()
}

/// The name of the `n`th hidden iterator generic of a fn.
fn hidden_iter_generic(n: usize, span: Span) -> Ident {
    Ident {
        name: format!("__It{n}"),
        span,
    }
}

/// [iter-type] [iter-group] Hoists the `iter T` placeholder out of every
/// top-level fn signature into a **hidden generic** — the reading the
/// language gives it in a parameter position (user decisions 2026-09-26/27),
/// done here so that everything downstream sees an ordinary generic fn:
///
/// * a spread of a group whose members mention `iter T` (`?Iter<C, T>`) gets a
///   fresh type parameter `__ItN` appended to the fn's generics and to the
///   spread's argument list (`?Iter<C, T, __It0>`); the checker substitutes it
///   for the placeholder in every member, so the spread brings in
///   `iter: (C) -> Mut __It0` and `next: (Mut __It0) -> Emitted T | Finished`;
/// * a parameter `it: iter T` becomes `it: Mut __ItN` plus a `?Yield<__ItN, T>`
///   spread — "any iterator struct emitting `T`" — fresh **per occurrence**, so
///   `chain(a: iter T, b: iter T)` takes two different structs.
///
/// Every other position (a return type, a `let` annotation) keeps the
/// placeholder for the checker to fill as a pattern, and the positions that
/// admit neither reading are refused there.
fn hoist_iter_types(module: &mut Module, groups: &[ParamsDecl]) {
    for item in &mut module.items {
        let Item::Fn(f) = item else { continue };
        if f.is_iter {
            // Refused by `expand` for now; hoisting would hide the refusal.
            continue;
        }
        let mut n = 0;
        for g in &mut f.implicit_groups {
            let Some(group) = groups.iter().find(|gr| gr.name.name == g.name.name) else {
                continue;
            };
            if !group_mentions_iter(group) {
                continue;
            }
            let hidden = hidden_iter_generic(n, g.span);
            n += 1;
            f.generics.push(hidden.clone());
            g.args.push(Type::Named {
                qualifiers: Vec::new(),
                base: TypeRef {
                    name: hidden,
                    args: Vec::new(),
                    value_args: Vec::new(),
                    from: Vec::new(),
                    at: None,
                    binder: false,
                    established: false,
                    alias: None,
                    span: g.span,
                },
            });
        }
        let mut spreads: Vec<TypeRef> = Vec::new();
        for p in f.params.iter_mut().filter(|p| !p.implicit) {
            let Some((before, inner)) = split_iter_qualifier(&p.ty) else {
                continue;
            };
            if !before.is_empty() {
                // `Mut iter T`: the checker's duplicate refusal names it.
                continue;
            }
            let hidden = hidden_iter_generic(n, p.span);
            n += 1;
            f.generics.push(hidden.clone());
            let hidden_ty = Type::Named {
                qualifiers: Vec::new(),
                base: TypeRef {
                    name: hidden.clone(),
                    args: Vec::new(),
                    value_args: Vec::new(),
                    from: Vec::new(),
                    at: None,
                    binder: false,
                    established: false,
                    alias: None,
                    span: p.span,
                },
            };
            p.ty = Type::Named {
                qualifiers: vec![type_ref("Mut", p.span)],
                base: TypeRef {
                    name: hidden,
                    args: Vec::new(),
                    value_args: Vec::new(),
                    from: Vec::new(),
                    at: None,
                    binder: false,
                    established: false,
                    alias: None,
                    span: p.span,
                },
            };
            spreads.push(TypeRef {
                name: Ident {
                    name: "Yield".to_string(),
                    span: p.span,
                },
                args: vec![hidden_ty, inner],
                value_args: Vec::new(),
                from: Vec::new(),
                at: None,
                binder: false,
                established: false,
                alias: None,
                span: p.span,
            });
        }
        f.implicit_groups.extend(spreads);
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
        is_send: false,
        name: Ident {
            name: fn_name,
            span,
        },
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
/// iterator parameter — and, in `scan` mode, *classifies* how the body uses
/// each parameter so the struct can hold as little of it as possible.
///
/// One traversal serves both, deliberately: every `Expr` and `Stmt` variant is
/// matched **exhaustively**, so a new variant is a compile error here rather
/// than an unrewritten name that resolves to nothing — and a second walk would
/// have to be kept in step with this one by hand.
struct Rewrite {
    /// Parameter name -> the hidden field holding it whole (or owned). A
    /// parameter held per-field or not at all is absent here. In `scan` mode
    /// every parameter is present, since the scan classifies them all.
    params: BTreeMap<String, String>,
    state: Vec<String>,
    /// Distinct spans for the `__p` bases this rewrite synthesizes, continuing
    /// the declaration's allocation so no two synthesized nodes collide. Unused
    /// in `scan` mode, which rewrites nothing.
    spans: Spans,
    /// Parameter name -> (its field -> the hidden field standing in for it)
    /// [iter-fn]. Only for parameters held per-field.
    snapshots: BTreeMap<String, BTreeMap<String, String>>,
    /// Scanning rather than rewriting: record, change nothing.
    scan: bool,
    /// What the scan found: per parameter, its fields read in the body, in
    /// first-use order.
    used_fields: BTreeMap<String, Vec<String>>,
    /// Parameters the body uses as a *value* — passes on, reads as a whole — so
    /// no per-field snapshot can stand in for them.
    used_whole: Vec<String>,
    /// Parameters the body assigns through. Kept whole so the standing
    /// [struct-mut] refusal fires with its own message, rather than the write
    /// silently landing on a snapshot field of the (mutable) iterator.
    assigns_through: Vec<String>,
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
                if self.scan {
                    if let Some(root) = assign_root(target) {
                        if self.params.contains_key(root) && !self.assigns_through.iter().any(|p| p == root) {
                            self.assigns_through.push(root.to_string());
                        }
                    }
                }
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
        // [iter-fn] `c.field` on the subject is the shape a snapshot can stand
        // in for, and it has to be caught *before* the base is visited — after
        // that the base is no longer the subject.
        if let Expr::Field { base, field, span } = expr {
            if let Expr::Ident(id) = &**base {
                let param = id.name.clone();
                if self.scan {
                    if self.params.contains_key(&param) {
                        let used = self.used_fields.entry(param).or_default();
                        if !used.contains(&field.name) {
                            used.push(field.name.clone());
                        }
                        return;
                    }
                } else if let Some(hidden) =
                    self.snapshots.get(&param).and_then(|m| m.get(&field.name)).cloned()
                {
                    let base_span = self.spans.take();
                    *expr = pass_field(&hidden, *span, base_span);
                    return;
                }
            }
        }
        match expr {
            Expr::Ident(ident) => {
                if self.scan {
                    if self.params.contains_key(&ident.name)
                        && !self.used_whole.contains(&ident.name)
                    {
                        self.used_whole.push(ident.name.clone());
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

/// The root name of an assignment target — `s` in `s`, `s.f`, `s.f[i]`, `s.0`.
fn assign_root(target: &Expr) -> Option<&str> {
    match target {
        Expr::Ident(id) => Some(id.name.as_str()),
        Expr::Field { base, .. } | Expr::TupleIndex { base, .. } | Expr::Index { base, .. } => {
            assign_root(base)
        }
        _ => None,
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
        Type::Literal { .. } => {}
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

/// [qual-depend] The root names of every **place** value argument in `ty`
/// (`list` in `Idx(list)` and `Idx(list.items)`).
fn value_arg_roots(ty: &Type, out: &mut Vec<String>) {
    fn in_ref(r: &TypeRef, out: &mut Vec<String>) {
        for v in &r.value_args {
            if let Type::Named { base, .. } = v {
                if base.args.is_empty() && base.at.is_none() && !base.binder {
                    let root = base.name.name.split('.').next().unwrap_or("").to_string();
                    if root.starts_with(|c: char| c.is_lowercase()) && !out.contains(&root) {
                        out.push(root);
                    }
                }
            }
        }
        for a in &r.args {
            value_arg_roots(a, out);
        }
    }
    match ty {
        Type::Named { qualifiers, base } => {
            for q in qualifiers {
                in_ref(q, out);
            }
            in_ref(base, out);
        }
        Type::QualifiedGroup { qualifiers, base, .. } => {
            for q in qualifiers {
                in_ref(q, out);
            }
            value_arg_roots(base, out);
        }
        Type::Literal { .. } => {}
        Type::Union { arms, .. } | Type::Tuple { elems: arms, .. } => {
            for a in arms {
                value_arg_roots(a, out);
            }
        }
        Type::Array { elem, .. } | Type::Nullable { inner: elem, .. } => value_arg_roots(elem, out),
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
        Type::Fn { .. } => {}
    }
}

// ===== [fn-attached] struct-body fns =====

/// [fn-attached] Hoists every fn written inside a struct body to module level,
/// attached to the type and carrying its visibility. Hoisted rather than kept
/// nested because it *is* an ordinary top-level function in every other
/// respect — it overloads, it is called bare or with dot notation, and it
/// takes part in ranking. A `by` declaration [fn-by] and a concrete `compfn`
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
            Item::Import(_)
            | Item::Type(_)
            | Item::Qualifier(_)
            | Item::Effect(_)
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
                CompStmt::WhenKind { arms, else_, .. } => {
                    for arm in arms {
                        walk_block_with(&arm.body, on_stmt, on_expr);
                    }
                    if let Some(b) = else_ {
                        walk_block_with(b, on_stmt, on_expr);
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
        Expr::SafeField { base, inner, .. } => {
            e(base);
            e(inner);
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
        | Expr::NonNull { operand: base, .. }
        | Expr::IncDec { operand: base, .. }
        | Expr::Spread { operand: base, .. }
        | Expr::Unary { operand: base, .. }
        | Expr::Widen { subject: base, .. }
        | Expr::Is { subject: base, .. } => e(base),
        Expr::Assert { cond, message, .. } => {
            e(cond);
            if let Some(m) = message {
                e(m);
            }
        }
        Expr::Unreachable { message, .. } => {
            if let Some(m) = message {
                e(m);
            }
        }
        Expr::Index { base, index, .. } => {
            e(base);
            e(index);
        }
        Expr::ArrayLit { elems, .. } | Expr::SetLit { elems, .. } | Expr::Tuple { elems, .. } => {
            for x in elems {
                e(x);
            }
        }
        Expr::MapLit { entries, .. } => {
            for (k, v) in entries {
                e(k);
                e(v);
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
        | Expr::Error { .. } => {}
    }
}
