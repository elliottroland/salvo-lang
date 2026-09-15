//! Abstract syntax tree for Salvo.
//!
//! The AST is deliberately close to the surface syntax; desugaring (e.g.
//! `T?` -> `T | None`) happens during lowering in `salvo-core`.

use std::fmt;

use crate::span::Span;

#[derive(Clone, Debug, PartialEq)]
pub struct Ident {
    pub name: String,
    pub span: Span,
}

/// A parsed source file.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Module {
    pub items: Vec<Item>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Item {
    Import(ImportDecl),
    Type(TypeDecl),
    Struct(StructDecl),
    Qualifier(QualifierDecl),
    Effect(EffectDecl),
    Handler(HandlerDecl),
    Params(ParamsDecl),
    Fn(FnDecl),
    /// A top-level `refn` [qual-refn]: a refinement written by the
    /// *consumer* rather than by a qualifier, which is how two
    /// conflicting refinements are reconciled in one's own module.
    Refn(RefnDecl),
    /// `rename fn add2 = add(a: Int, b: Int)` [fn-rename].
    Rename(RenameDecl),
}

/// `refn add(list: Mut List<T>, elem: T) -> [list: +NonEmpty]`
/// [qual-refn]: additional deductions for a function *someone else*
/// declared, stated by the qualifier whose claim they are about.
///
/// The syntax is deliberately narrower than a `fn`'s: a refinement cannot
/// declare effects or a return type (it does not change what the function
/// *does*, only what is known afterwards), and its deduction entries can
/// only add or remove state qualifiers — never decide whether a parameter
/// is kept. The parameter list is there to pick one overload
/// [qual-refn-match], so it carries types *and* names.
#[derive(Clone, Debug, PartialEq)]
pub struct RefnDecl {
    /// The `//` comment block directly above the declaration, one entry
    /// per line, `//` and one leading space stripped [doc-comment]. Merged
    /// into the refined function's documentation wherever the refinement
    /// applies [qual-refn-docs].
    pub docs: Vec<String>,
    /// The name of the function being refined.
    pub name: Ident,
    /// Type parameters written on the refinement itself. A refinement in a
    /// qualifier body also sees the *qualifier's* parameters, which come
    /// first when matching an overload [qual-refn-match].
    pub generics: Vec<Ident>,
    pub params: Vec<Param>,
    pub deductions: Vec<RefnDeduction>,
    pub span: Span,
}

/// [fn-rename] `rename fn add2 = add(a: Int | Str, b: Int | Str)`: a
/// scope-local name for **one** overload, which from that point on stops
/// answering to its own name (user decision 2026-09-07). Not an alias: the
/// renamed overload leaves the `add` candidate set for the rest of the
/// scope, which is what makes it a way *out* of an ambiguity rather than a
/// second way in.
///
/// The parameter list identifies the overload and nothing else — same names,
/// same types, type parameters positional, as `refn` matches [qual-refn-match].
/// Effects, deductions and a return type may not be written: none of them
/// takes part in selecting an overload, so writing one could only be wrong.
#[derive(Clone, Debug, PartialEq)]
pub struct RenameDecl {
    /// The `//` comment block above the declaration [doc-comment].
    pub docs: Vec<String>,
    /// The new name.
    pub name: Ident,
    /// The overload's own name.
    pub target: Ident,
    /// Type parameters written on the rename, matched positionally against
    /// the declaration's.
    pub generics: Vec<Ident>,
    pub params: Vec<Param>,
    pub span: Span,
}

/// One entry of a refinement's deduction list [qual-refn]:
/// `[list: +NonEmpty]`, `[list: -Sorted]`, or both.
///
/// A separate type from [`Deduction`] on purpose: a refinement can *only*
/// add and remove, so the restriction is structural rather than a check on
/// a shape that could express more.
#[derive(Clone, Debug, PartialEq)]
pub struct RefnDeduction {
    pub param: Ident,
    /// `+Q`: the call establishes `Q`.
    pub add: Vec<TypeRef>,
    /// `-Q`: the call invalidates `Q`.
    pub remove: Vec<TypeRef>,
    pub span: Span,
}

/// `params Field<T> { fn add(a: T, b: T) -> T ... }` [implicit-group]: a
/// named bundle of implicit parameters, written once and spread into a
/// signature as `?Field<T>`.
///
/// Not a struct and never a value: the members are *parameters* after
/// expansion, which is what keeps a group free of any runtime
/// representation — nothing is boxed, and neither backend needs to know
/// groups exist. Declaring it as a struct of fn-typed fields would need
/// `Box<dyn Fn>` fields on Rust (`impl Trait` is illegal in a field type)
/// and a way to call a fn-typed field, which dot-notation [fn-dot] already
/// spells otherwise.
#[derive(Clone, Debug, PartialEq)]
pub struct ParamsDecl {
    /// The `//` comment block directly above the declaration, one entry
    /// per line, `//` and one leading space stripped [doc-comment].
    pub docs: Vec<String>,
    pub name: Ident,
    pub generics: Vec<Ident>,
    /// Member signatures: bodiless, like an effect's [effect-decl].
    pub fns: Vec<FnDecl>,
    pub span: Span,
}

/// `import path.to.item (as alias)?`
#[derive(Clone, Debug, PartialEq)]
pub struct ImportDecl {
    pub path: Vec<Ident>,
    pub alias: Option<Ident>,
    pub span: Span,
}

/// `intrinsic type Str`, `intrinsic type List<T> canbe Mut`, or a type alias
/// `type Result<S, T> = Ok S | Err T`.
#[derive(Clone, Debug, PartialEq)]
pub struct TypeDecl {
    /// The `//` comment block directly above the declaration, one entry
    /// per line, `//` and one leading space stripped [doc-comment].
    pub docs: Vec<String>,
    /// `intrinsic`: declared by the standard library and implemented inside
    /// the compiler — every backend must lower every one of them
    /// [backend-intrinsic] [intrinsic-fn] [intrinsic-std-only]. A flag
    /// rather than an enum: it is the only backing modifier there is, now
    /// that `external`/`define` are gone (user decision 2026-09-05).
    pub intrinsic: bool,
    /// [linear-group] `linear intrinsic type Reply<T>` — the exactly-once
    /// obligation on an **opaque** type (user decision 2026-09-15, taken for
    /// the first linear type whose representation belongs to the backend: a
    /// reply token is a scheduler handle, so there is nothing to make a
    /// `linear struct` out of). The struct modifier's rules apply unchanged —
    /// the declaring file must contain a discharger, and every value owes.
    /// Only an `intrinsic type` may carry it, never an alias.
    pub linear: bool,
    pub name: Ident,
    pub generics: Vec<Ident>,
    /// Auto-qualifiers, e.g. `canbe Mut` [type-canbe-mut]: the type opts
    /// into the language-level `Mut` qualifier (like `struct ... canbe
    /// Mut` [struct-mut]).
    pub auto_qualifiers: Vec<TypeRef>,
    pub alias: Option<Type>,
    pub span: Span,
}

/// `struct Person canbe Mut { name: Str, ... }`, optionally with an
/// obligation clause: `struct Lines : Linear, Yield<Str> canbe Mut { ... }`
/// [group-obligation].
#[derive(Clone, Debug, PartialEq)]
pub struct StructDecl {
    /// The `//` comment block directly above the declaration, one entry
    /// per line, `//` and one leading space stripped [doc-comment].
    pub docs: Vec<String>,
    pub name: Ident,
    pub generics: Vec<Ident>,
    /// Per-type-parameter opt-ins, `<T canbe linear>` [linear-generics]:
    /// a **conditional container** — the struct is linear exactly when the
    /// instantiation puts a linear type in a field the parameter reaches
    /// (user decision 2026-09-12).
    pub generic_canbe: Vec<(Ident, TypeRef)>,
    /// Obligations [group-obligation]: `params` groups this type promises
    /// to satisfy, written `: Group<Args>` after the generics and before
    /// `canbe`. Each member of each named group must have a matching
    /// visible fn, with `self` in the argument list standing for this type —
    /// checked at this declaration, not at a use site.
    pub obligations: Vec<TypeRef>,
    /// Auto-qualifiers, e.g. `canbe Mut`.
    pub auto_qualifiers: Vec<TypeRef>,
    pub fields: Vec<FieldDecl>,
    /// [linear-group] `linear struct X` — the exactly-once obligation,
    /// declared as a modifier (user decision 2026-09-12; replaces the
    /// `: Linear<self>` group entry). A generic struct with a
    /// `canbe linear` parameter is *conditionally* linear without the
    /// modifier; the modifier is for leaf types and concrete linear
    /// fields.
    pub linear: bool,
    pub span: Span,
}

/// A struct field, optionally with a default value.
#[derive(Clone, Debug, PartialEq)]
pub struct FieldDecl {
    /// The `//` comment block directly above the declaration, one entry
    /// per line, `//` and one leading space stripped [doc-comment].
    pub docs: Vec<String>,
    pub name: Ident,
    pub ty: Type,
    pub default: Option<Expr>,
    pub span: Span,
}

/// What a qualifier's claim is *about* [qual-subject]: the value's
/// contents, or where the handle came from. The subject decides whether a
/// mutating call may strip it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum QualSubject {
    /// A claim about the contents (`NonEmpty`, `Sorted`): a call that
    /// mutates the value may invalidate it, so it is stripped unless the
    /// callee's deduction list keeps it [deduce-syntax].
    #[default]
    State,
    /// A claim about the handle's origin (`Authenticated`,
    /// `Environment.Id`): content-independent, so mutation cannot
    /// invalidate it and it survives stripping. Mint-only — a provenance
    /// qualifier has no body, since no inspection of the bits can
    /// establish it.
    Provenance,
}

/// `qualifier Name<G> of Type with Other { field-overrides fns }`
#[derive(Clone, Debug, PartialEq)]
pub struct QualifierDecl {
    /// The `//` comment block directly above the declaration, one entry
    /// per line, `//` and one leading space stripped [doc-comment].
    pub docs: Vec<String>,
    /// `intrinsic`: declared by the standard library and implemented inside
    /// the compiler — every backend must lower every one of them
    /// [backend-intrinsic] [intrinsic-fn] [intrinsic-std-only]. A flag
    /// rather than an enum: it is the only backing modifier there is, now
    /// that `external`/`define` are gone (user decision 2026-09-05).
    pub intrinsic: bool,
    /// State (default) or provenance [qual-subject].
    pub subject: QualSubject,
    pub name: Ident,
    pub generics: Vec<Ident>,
    pub of: Type,
    /// Compatible qualifiers, e.g. `with Surname`.
    pub with: Vec<TypeRef>,
    /// Field type overrides for predicate qualifiers on structs.
    pub field_overrides: Vec<FieldDecl>,
    /// Functions defined in the qualifier body (e.g. `qualifies`).
    pub fns: Vec<FnDecl>,
    /// Refinements of *other* functions [qual-refn]: extra deductions this
    /// qualifier claims for functions it does not own, in scope wherever
    /// the qualifier is.
    pub refns: Vec<RefnDecl>,
    /// True when the declaration had a `{ ... }` body (predicate qualifier).
    pub has_body: bool,
    pub span: Span,
}

/// `effect Console { fn println(...) }`, or `platform effect Telemetry
/// { ... }` — an effect whose handler the *host* provides
/// [platform-effect].
#[derive(Clone, Debug, PartialEq)]
pub struct EffectDecl {
    /// The `//` comment block directly above the declaration, one entry
    /// per line, `//` and one leading space stripped [doc-comment].
    pub docs: Vec<String>,
    /// True for `platform effect` [platform-effect]: the members are
    /// implemented by the *host* in the target language, so the compiler
    /// generates the interface and the instance arrives from outside the
    /// Salvo program — there is nothing to `use`.
    ///
    /// A dedicated flag rather than a shared backing enum, because `platform`
    /// applies to nothing but an effect and `intrinsic`/`external` never
    /// apply to one: the two sets are disjoint, so keeping them apart makes
    /// the invariant structural.
    pub platform: bool,
    pub name: Ident,
    pub generics: Vec<Ident>,
    pub fns: Vec<FnDecl>,
    pub span: Span,
}

/// `handler CyclicRandom<T>(values: T[]) of Random<T> { state fns }`,
/// `intrinsic handler StdOutConsole of Console`, or `platform handler
/// HostRawFs of RawFs` [platform-handler].
#[derive(Clone, Debug, PartialEq)]
pub struct HandlerDecl {
    /// The `//` comment block directly above the declaration, one entry
    /// per line, `//` and one leading space stripped [doc-comment].
    pub docs: Vec<String>,
    /// `intrinsic`: declared by the standard library and implemented inside
    /// the compiler — every backend must lower every one of them
    /// [backend-intrinsic] [intrinsic-fn] [intrinsic-std-only]. A flag
    /// rather than an enum: it is the only backing modifier there is, now
    /// that `external`/`define` are gone (user decision 2026-09-05).
    pub intrinsic: bool,
    /// [platform-handler] `platform handler HostRawFs of RawFs`: a handler
    /// of an *ordinary* Salvo effect whose implementation is a class the
    /// host supplies — a companion in the `platform/` tree of the module
    /// that declares it [platform-tree]. Bodyless in Salvo, and unlike a
    /// `platform effect` it is registered with `use` like any handler: the
    /// generated code constructs the host class.
    ///
    /// A flag beside `intrinsic` rather than a shared enum, for the reason
    /// the effect's flag is one: the two never combine — `intrinsic` means
    /// *the compiler* implements the members, `platform` means the *build*
    /// does — and the grammar admits only one of them.
    pub platform: bool,
    pub name: Ident,
    pub generics: Vec<Ident>,
    /// Constructor parameters, e.g. `(values: T[])`.
    pub params: Vec<Param>,
    /// The effects this handler *depends on*, written like a fn's
    /// ([effect-handler-deps]): `handler Stamped [Logger, Clock] of Logger`.
    /// They are supplied by the compiler at the `use` site, so they are
    /// unnamed — the members reach them by calling their members, exactly as
    /// any other code does. `None` when the list is absent, as on a fn.
    pub effects: Option<Vec<EffectRef>>,
    pub of: Type,
    /// State fields with initializers, e.g. `i: Int = 0`.
    pub state: Vec<FieldDecl>,
    pub fns: Vec<FnDecl>,
    pub span: Span,
}

/// A function declaration or signature.
///
/// `fn name<G>(params) [effects] -> [deductions] return_type (as Qualifier)? { body }`
#[derive(Clone, Debug, PartialEq)]
pub struct FnDecl {
    /// The `//` comment block directly above the declaration, one entry
    /// per line, `//` and one leading space stripped [doc-comment].
    pub docs: Vec<String>,
    /// `intrinsic`: declared by the standard library and implemented inside
    /// the compiler — every backend must lower every one of them
    /// [backend-intrinsic] [intrinsic-fn] [intrinsic-std-only]. A flag
    /// rather than an enum: it is the only backing modifier there is, now
    /// that `external`/`define` are gone (user decision 2026-09-05).
    pub intrinsic: bool,
    /// [iter-fn] `iter fn next(c: Countdown) -> Emitted T | Finished`: a
    /// hand-written `next` whose **pass struct is generated**. The subject is
    /// ordinary data; the pass's own fields are declared in the `state { … }`
    /// block below and initialized once per pass. Named after the `iter` it
    /// generates — that function is what a combinator or a `for` reaches it
    /// through. Desugared away before the checker ever sees it
    /// (`desugar::expand_iter_fns`), into a hidden struct, that `iter`, and this
    /// body as an ordinary `next` — so nothing downstream knows the form exists.
    pub is_iter: bool,
    /// [iter-fn] The `state { … }` block's fields, in declaration order. Each
    /// carries an annotation and an initializer, exactly like a handler's state
    /// [effect-handler]; the initializer may read the subject and runs when the
    /// pass is minted.
    pub iter_state: Vec<FieldDecl>,
    /// [async-send-fn] `send fn bump(n: Int)`: an **asynchronous** member —
    /// a message, not a call. Sending one enqueues an invocation on the
    /// target process and returns immediately, so the member answers
    /// nothing: a reply travels as a `Reply<T>` parameter the sender mints
    /// with `replyto`. In the first pass every member of a process protocol
    /// is one, and the unmarked `fn` spelling stays reserved for the later
    /// call-member sugar (a `-> T` member desugars *to* a `send fn` with a
    /// trailing token). Contextual, like `iter fn`: `send` is not a
    /// reserved word.
    pub is_send: bool,
    pub name: Ident,
    pub generics: Vec<Ident>,
    /// Per-type-parameter opt-ins: `<T canbe linear>` [linear-generics].
    pub generic_canbe: Vec<(Ident, TypeRef)>,
    /// `-> proj[from: param] T`: the returned value is derived from
    /// (borrows) the named kept parameter [readonly-return].
    pub derived_return: Option<Ident>,
    pub params: Vec<Param>,
    /// `?Field<T>` spreads [implicit-group]: a *declaration-side* shorthand
    /// for one implicit parameter per member of a `params` group. There is
    /// no binder — the members become ordinary implicit parameters, so
    /// resolution, forwarding and call-site override all key on a member's
    /// own name and type. The checker publishes the expansion (written
    /// implicits first, then each group's members in declaration order) as
    /// the one ordered list both emitters render.
    pub implicit_groups: Vec<TypeRef>,
    /// `None` means unspecified (pure); `Some(vec![])` means explicit `[]`.
    pub effects: Option<Vec<EffectRef>>,
    /// `None` means unspecified (inferred); `Some(vec![])` means explicit `[]`.
    pub deductions: Option<Vec<Deduction>>,
    /// `None` means the function returns `None` (the unit type).
    pub return_type: Option<Type>,
    /// `-> T as Qualifier` marks a constructive-qualifier constructor: the
    /// body returns plain `T` values which gain the qualifier by
    /// construction; callers see `Qualifier T`.
    pub constructs: Option<TypeRef>,
    /// `None` for signatures (`intrinsic fn`, effect members).
    pub body: Option<Block>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Param {
    pub name: Ident,
    pub ty: Type,
    /// True for `...args: T[]`.
    pub variadic: bool,
    /// True for `?cmp: (T, T) -> Int` [implicit-param]: the caller need not
    /// pass it. A call site fills it by resolving the parameter's *name* at
    /// the parameter's *type* — an ordinary overload query, only against a
    /// type instead of an argument list — or by forwarding an implicit of
    /// the same name and type from the enclosing fn [implicit-forward].
    pub implicit: bool,
    pub span: Span,
}

/// An entry in a function's effect list: `[Random<Int>, Console, use]`.
#[derive(Clone, Debug, PartialEq)]
pub enum EffectRef {
    /// The special `use` effect, allowing handler registration.
    Use(Span),
    /// [async-spawn-effect] The special `spawn` effect, allowing process
    /// creation. Lowercase and compiler-owned like `use`: it names a
    /// capability rather than a declared effect, so there is no handler to
    /// resolve and nothing to thread — `fn main() [use, spawn]` is the
    /// typical entry point.
    Spawn(Span),
    /// A named effect, possibly generic: `Random<Int>`.
    Effect(TypeRef),
}

/// An entry in a function's deduction clause [deduce-syntax], written after
/// the return type behind `=>`:
/// `=> person` (bare — every qualifier the argument has survives),
/// `=> list: Mut` (*exhaustive* — afterwards only `Mut` applies),
/// `=> list: None` (exhaustive and empty — every qualifier stripped),
/// `=> list: -NonEmpty` (*delta* — drop `NonEmpty`, keep the rest),
/// `=> !list` (moved; `list: Nothing` says the same),
/// `=> .items: proj[from: list]` (the result's field projects `list`),
/// `=> v.items: proj[from: other]` (a parameter's field is re-pointed), and
/// `=> proj[from: c]` (opaque: the result holds a borrow of `c`)
/// [proj-infer]. A parameter the clause does not mention is *inferred*
/// from the body; a bodiless declaration must mention every parameter
/// except Copy scalars.
#[derive(Clone, Debug, PartialEq)]
pub struct Deduction {
    pub target: DeductionTarget,
    pub kind: DeductionKind,
    pub span: Span,
}

impl Deduction {
    /// The parameter a *plain* entry is about — `elem`, `!elem`,
    /// `elem: Qual` — an entry that decides a parameter's keptness and
    /// afterwards-qualifiers. `None` for a projection entry, a result path,
    /// or a parameter *field* path.
    pub fn param_name(&self) -> Option<&Ident> {
        match (&self.target, &self.kind) {
            (DeductionTarget::Param { name, path }, kind)
                if path.is_empty() && !matches!(kind, DeductionKind::Proj(_)) =>
            {
                Some(name)
            }
            _ => None,
        }
    }

    /// The sources of a projection entry, if it is one.
    pub fn proj_sources(&self) -> Option<&[Ident]> {
        match &self.kind {
            DeductionKind::Proj(sources) => Some(sources),
            _ => None,
        }
    }
}

/// The subject of a deduction entry [deduce-syntax].
#[derive(Clone, Debug, PartialEq)]
pub enum DeductionTarget {
    /// A parameter (`elem`), or a field path under one (`v.items`).
    Param { name: Ident, path: Vec<Ident> },
    /// A field path of the result (`.items`).
    Result { path: Vec<Ident> },
    /// The result as a whole, opaquely: a bare `proj[from: c]` says the
    /// result *holds* a borrow of `c` somewhere inside [proj-infer].
    Opaque,
}

/// The polarity of one deduction entry [deduce-syntax]. A written entry is
/// either exhaustive (plain qualifier names) or a delta (`-`-prefixed
/// names); mixing them in one entry is an error.
#[derive(Clone, Debug, PartialEq)]
pub enum DeductionKind {
    /// Bare `=> list`: the parameter is kept and *nothing* is stripped.
    KeepAll,
    /// `=> list: A B` / `=> list: None`: afterwards exactly these apply.
    Exhaustive(Vec<TypeRef>),
    /// `=> list: -A -B`: these are dropped, everything else survives.
    Remove(Vec<TypeRef>),
    /// `=> !list` / `=> list: Nothing`: moved (the caller loses access).
    Moved,
    /// `proj[from: a, b]`: a projection of the named parameters — of the
    /// entry's target (a result path, a parameter, a parameter's field) or,
    /// with no target, held somewhere inside the result [proj-infer].
    Proj(Vec<Ident>),
}

// --- Types ---

/// A type expression as written in source.
#[derive(Clone, Debug, PartialEq)]
pub enum Type {
    /// A (possibly qualified, possibly generic) named type: `Str`,
    /// `List<Int>`, `Mut NonEmpty List<T>`. The last element of `path` is the
    /// base type; the preceding elements are qualifiers.
    Named {
        qualifiers: Vec<TypeRef>,
        base: TypeRef,
    },
    /// Qualifiers applied to a parenthesized type:
    /// `Ok (Ok Str | Err Int)`.
    QualifiedGroup {
        qualifiers: Vec<TypeRef>,
        base: Box<Type>,
        span: Span,
    },
    /// `A | B | C`
    Union { arms: Vec<Type>, span: Span },
    /// `(A, B, C)`
    Tuple { elems: Vec<Type>, span: Span },
    /// `T[]`
    Array { elem: Box<Type>, span: Span },
    /// `T?` — sugar for `T | None`.
    Nullable { inner: Box<Type>, span: Span },
    /// `(S) -> T` or `(S) [E] -> T` — a function/lambda type. Parameters
    /// may be *named* (`(v: List<Int>) -> [v] Int`), which lets a
    /// deduction list state the fn value's contract [fn-contract]; an
    /// unannotated fn type keeps everything (the default contract).
    Fn {
        params: Vec<Type>,
        /// Parallel to `params`: the optional parameter names.
        param_names: Vec<Option<Ident>>,
        effects: Option<Vec<EffectRef>>,
        /// `-> [deductions] R` inside the fn type [fn-contract].
        deductions: Option<Vec<Deduction>>,
        ret: Box<Type>,
        span: Span,
    },
}

impl Type {
    pub fn span(&self) -> Span {
        match self {
            Type::Named { qualifiers, base } => qualifiers
                .first()
                .map(|q| q.span.to(base.span))
                .unwrap_or(base.span),
            Type::Union { span, .. }
            | Type::Tuple { span, .. }
            | Type::Array { span, .. }
            | Type::Nullable { span, .. }
            | Type::Fn { span, .. }
            | Type::QualifiedGroup { span, .. } => *span,
        }
    }
}

/// A reference to a named type or qualifier, with optional generic arguments:
/// `Str`, `List<Int>`, `Ok<T>`.
#[derive(Clone, Debug, PartialEq)]
pub struct TypeRef {
    pub name: Ident,
    pub args: Vec<Type>,
    /// [proj-anywhere] `proj[from: a, b]`: for the `proj` qualifier, the
    /// kept parameters the value borrows from (several when a projection is
    /// joined across branches). Only `proj` carries them, wherever a type
    /// does — a return, a union arm (`(proj[from: xs] T)?`). Empty on a
    /// field or parameter (the source is the value's, not the type's).
    pub from: Vec<Ident>,
    pub span: Span,
}

// --- Source-like rendering (hover, diagnostics) ---

impl fmt::Display for TypeRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name.name)?;
        if !self.from.is_empty() {
            let names: Vec<&str> = self.from.iter().map(|i| i.name.as_str()).collect();
            write!(f, "[from: {}]", names.join(", "))?;
        }
        if !self.args.is_empty() {
            let args: Vec<String> = self.args.iter().map(|a| a.to_string()).collect();
            write!(f, "<{}>", args.join(", "))?;
        }
        Ok(())
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Named { qualifiers, base } => {
                for q in qualifiers {
                    write!(f, "{q} ")?;
                }
                write!(f, "{base}")
            }
            Type::QualifiedGroup {
                qualifiers, base, ..
            } => {
                for q in qualifiers {
                    write!(f, "{q} ")?;
                }
                write!(f, "({base})")
            }
            Type::Union { arms, .. } => {
                let arms: Vec<String> = arms.iter().map(|a| a.to_string()).collect();
                write!(f, "{}", arms.join(" | "))
            }
            Type::Tuple { elems, .. } => {
                let elems: Vec<String> = elems.iter().map(|e| e.to_string()).collect();
                write!(f, "({})", elems.join(", "))
            }
            Type::Array { elem, .. } => write!(f, "{elem}[]"),
            Type::Nullable { inner, .. } => write!(f, "{inner}?"),
            Type::Fn {
                params,
                effects,
                ret,
                ..
            } => {
                let params: Vec<String> = params.iter().map(|p| p.to_string()).collect();
                write!(f, "({})", params.join(", "))?;
                if let Some(effects) = effects {
                    let effects: Vec<String> =
                        effects.iter().map(|e| e.to_string()).collect();
                    write!(f, " [{}]", effects.join(", "))?;
                }
                write!(f, " -> {ret}")
            }
        }
    }
}

impl fmt::Display for EffectRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EffectRef::Use(_) => write!(f, "use"),
            EffectRef::Spawn(_) => write!(f, "spawn"),
            EffectRef::Effect(r) => write!(f, "{r}"),
        }
    }
}

// --- Statements and expressions ---

#[derive(Clone, Debug, PartialEq)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Stmt {
    /// `let pattern (: Type)? = expr`
    Let {
        pattern: Pattern,
        ty: Option<Type>,
        value: Expr,
        span: Span,
    },
    /// `target = expr` (assignment to a variable or field)
    Assign {
        target: Expr,
        value: Expr,
        span: Span,
    },
    /// `return expr?`
    Return { value: Option<Expr>, span: Span },
    /// `break expr?`
    Break { value: Option<Expr>, span: Span },
    /// `continue`
    Continue { span: Span },
    /// `use HandlerExpr(...)` — register a handler for the current context.
    Use { handler: Expr, span: Span },
    /// [fn-rename] `rename fn add2 = add(a: Int, b: Int)` inside a block:
    /// in force from this line to the end of the enclosing scope.
    Rename(RenameDecl),
    /// A bare expression statement.
    Expr(Expr),
}

/// Destructuring patterns for `let`.
#[derive(Clone, Debug, PartialEq)]
pub enum Pattern {
    Ident(Ident),
    /// `let (a, b, c) = ...`
    Tuple { elems: Vec<Pattern>, span: Span },
    /// `let {name, age: their_age} = ...`
    Struct {
        fields: Vec<StructPatternField>,
        span: Span,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct StructPatternField {
    /// Field name in the struct.
    pub field: Ident,
    /// Variable to bind it to (same as `field` for shorthand).
    pub binding: Ident,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    /// Integer literal [lit-numeric]: `Int`, or `Long` with the `L`
    /// suffix (`long: true`).
    Int { value: i64, long: bool, span: Span },
    /// Floating-point literal [lit-numeric]: `Double`, or `Float` with
    /// the `f` suffix (`single: true`).
    Float { value: f64, single: bool, span: Span },
    /// Boolean literal.
    Bool { value: bool, span: Span },
    /// Character literal.
    Char { value: char, span: Span },
    /// String literal with interpolation parts.
    Str { parts: Vec<StrExprPart>, span: Span },
    /// A variable or type-name reference.
    Ident(Ident),
    /// `expr.field`
    Field {
        base: Box<Expr>,
        field: Ident,
        span: Span,
    },
    /// `expr.0` — a tuple element by constant index [expr-tuple-index].
    /// Distinct from `Index`: the position is known statically, so it
    /// names one element rather than an unknown one.
    TupleIndex {
        base: Box<Expr>,
        index: usize,
        span: Span,
    },
    /// `callee(args)` — including dot-notation `list.size()` which is kept
    /// as a `Field` callee and normalized later.
    /// [fn-overload-at] `add@core.list(x)`, or `xs.add@core.list(x)` in dot
    /// form: the *module* whose overload is meant, written where scope
    /// precedence would otherwise choose [fn-overload-scope]. Also valid as
    /// a value (`describe@main` passed to a higher-order function), which is
    /// how an overloaded name is disambiguated in a non-call position.
    Scoped {
        /// The dot-notation receiver, when written as `base.name@module(..)`.
        base: Option<Box<Expr>>,
        name: Ident,
        /// The module path, as written (`core.list`).
        module: Vec<Ident>,
        span: Span,
    },
    /// [effect-at] `close@Fs(s)`, or `s.close@Fs()` in dot form: the
    /// *effect* whose member is meant, written where two effects declare
    /// the same member name [effect-member-overload]. The effect is named
    /// bare; a generic instance is pinned with the call's type arguments
    /// (`next_random@Random<Int>()`), which keep their
    /// [effect-disambiguation] meaning.
    EffectScoped {
        /// The dot-notation receiver, when written as `base.name@Effect(..)`.
        base: Option<Box<Expr>>,
        name: Ident,
        /// The effect's name, as written (capitalized — a lowercase name
        /// after `@` is a module path).
        effect: Ident,
        span: Span,
    },
    Call {
        callee: Box<Expr>,
        /// Explicit generic args: `next_random<Int>()`.
        type_args: Vec<Type>,
        args: Vec<Arg>,
        /// `sort(xs, cmp = my_cmp)` [implicit-override]: an implicit
        /// parameter supplied by name instead of resolved. Named arguments
        /// exist for exactly this — Salvo has no general named-argument
        /// form — so a name that matches no implicit parameter of the
        /// callee is an error.
        named: Vec<NamedArg>,
        span: Span,
    },
    /// `expr[index]`
    Index {
        base: Box<Expr>,
        index: Box<Expr>,
        span: Span,
    },
    /// `[1, 2, 3]` — a **List** literal [col-literal] (an array only where
    /// the position expects one, i.e. a variadic).
    ArrayLit { elems: Vec<Expr>, span: Span },
    /// `{1, 2, 3}` — a Set literal [col-literal].
    SetLit { elems: Vec<Expr>, span: Span },
    /// `{"a": 1, "b": 2}` — a Map literal [col-literal]. Keys are
    /// expressions, so an *identifier* key is not writable here: `{x: 1}` is
    /// a bare struct literal, which came first and stays.
    MapLit {
        entries: Vec<(Expr, Expr)>,
        span: Span,
    },
    /// `(a, b, c)` — tuple literal (a single-element paren is just grouping).
    Tuple { elems: Vec<Expr>, span: Span },
    /// `Person {name: "R", ...other, age: 36}` or `Mut Person {...p}` or a
    /// bare `{name: ...}` when the type is contextually known.
    StructLit {
        /// Type as written (`None` for a bare `{...}` literal).
        ty: Option<Type>,
        fields: Vec<StructLitField>,
        span: Span,
    },
    /// Unary operators.
    Unary {
        op: UnaryOp,
        operand: Box<Expr>,
        span: Span,
    },
    /// Binary operators.
    Binary {
        op: BinaryOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
        span: Span,
    },
    /// `expr is TypeRefSeq binding?` — type/qualifier check with optional
    /// value binding (`if x is Str s { ... }`).
    Is {
        subject: Box<Expr>,
        /// The checked type: a sequence of qualifiers ending in an optional
        /// base type, e.g. `Str`, `Err Str`, `Surname`.
        check: Vec<TypeRef>,
        binding: Option<Ident>,
        span: Span,
    },
    /// `expr ^ Qual...` — the *widening* check [qual-widen]: the dual of
    /// `is`. Where a successful `is` narrows the subject (a qualifier added,
    /// a union arm picked), a successful `^` **generalizes** it by removing
    /// the listed qualifiers — `list ^ Mut` reads `list` without `Mut`
    /// afterwards, `outcome ^ Ok` without `Ok`. Boolean-valued, like `is`;
    /// the runtime test is the same one `is` performs.
    Widen {
        subject: Box<Expr>,
        /// The qualifiers to remove (all must be present and droppable).
        quals: Vec<TypeRef>,
        span: Span,
    },
    /// `expr!` — non-null assertion.
    NonNull { operand: Box<Expr>, span: Span },
    /// `i++` — postfix increment.
    /// [inc-dec] `i++`, `++i`, `i--`, `--i`: a step of one on a place, in
    /// either fixity. One node rather than four variants — every consumer
    /// but the emitters treats them identically (a mutation of the operand),
    /// so the distinction belongs in fields.
    IncDec {
        operand: Box<Expr>,
        /// Which way the step goes.
        down: bool,
        /// Prefix (`++i`) rather than postfix (`i++`): the difference is the
        /// *value* the expression has, not what it does to the operand.
        prefix: bool,
        span: Span,
    },
    /// `if cond { } elif cond { } else { }`
    If {
        branches: Vec<(Expr, Block)>,
        else_block: Option<Block>,
        span: Span,
    },
    /// `when subject { is X { } is Y { } }`
    When {
        subject: Box<Expr>,
        branches: Vec<WhenBranch>,
        span: Span,
    },
    /// `when { cond { } cond { } else { } }` — the subject-less form
    /// [when-condition]: a condition chain whose `else` is mandatory, so
    /// the expression is exhaustive without one of its branches ever
    /// contributing `None` to the value (which is what separates it from
    /// `if`/`elif`/`else`).
    WhenCond {
        branches: Vec<(Expr, Block)>,
        else_block: Block,
        span: Span,
    },
    /// `while cond { } else { }`
    While {
        cond: Box<Expr>,
        body: Block,
        else_block: Option<Block>,
        span: Span,
    },
    /// `for pat in iterable { } else { }`
    For {
        pattern: Pattern,
        iterable: Box<Expr>,
        body: Block,
        else_block: Option<Block>,
        span: Span,
    },
    /// `i -> expr`, `(a, b) -> { ... }`, `{ i: Int -> 0 }`
    Lambda {
        params: Vec<LambdaParam>,
        body: LambdaBody,
        span: Span,
    },
    /// `try { ... }` — the abort delimiter [try]. A compiler intrinsic
    /// rather than an effect: the block's value becomes the `Ok T` arm of
    /// the outcome `Ok T | Aborted M`, and an `abort` performed inside it
    /// becomes the `Aborted M` arm.
    Try { body: Block, span: Span },
    /// `...expr` — spread in call arguments or struct literals.
    Spread { operand: Box<Expr>, span: Span },
    /// [async-spawn-expr] `spawn Counting(0) use ScriptedDb(f), ddb
    /// capacity 16 on pool(2)` — bind a handler *asynchronously*: the
    /// process. Read left to right: what to run, what it depends on, how
    /// deep its queue is, where it runs. Its value is the child's `Pid`.
    ///
    /// `spawn` and the three clause words are **contextual** (the `iter fn`
    /// precedent): the form is recognised from `spawn` followed by a name,
    /// so a function called `spawn` keeps working.
    Spawn {
        /// The handler construction — a name plus its constructor
        /// arguments, the shape `use` takes, because a handler is not a
        /// value and only `use`/`spawn` may construct one.
        handler: Box<Expr>,
        /// The spawn-site `use` clause, empty when it is absent: handler
        /// constructions *or* `Pid` values, supplying the child's declared
        /// dependencies [effect-handler-deps]. Arguments evaluate in the
        /// parent and cross the seam; construction happens on the child.
        uses: Vec<Expr>,
        /// `capacity N` — this instance's mailbox bound. Explicit and
        /// required, with no default: it is a property of the instance, so
        /// it sits at the spawn site rather than on the handler.
        capacity: Box<Expr>,
        /// `on POOL` — an ordinary expression. `pool(n)` is a function, not
        /// syntax.
        pool: Box<Expr>,
        span: Span,
    },
    /// [async-replyto] `replyto batch_arrived(id)` — allocate a parked
    /// one-shot continuation targeting a member of the *enclosing handler*,
    /// yielding its linear `Reply<T>` [linear-obligation]. The arguments are
    /// the continuation's *captures*: what the member needs besides the
    /// answer it is waiting for.
    ///
    /// `replyto!` (`gated`) is the same mint plus the gate — bounded
    /// selective receive, at most one outstanding per process — so the
    /// process serves nothing else until the answer arrives.
    ReplyTo {
        member: Ident,
        captures: Vec<Expr>,
        /// Written `replyto!`: the gated mint.
        gated: bool,
        span: Span,
    },
    /// [async-waitfor] `waitfor out: Reply<Int> { counter.total(out) }` —
    /// `main`'s explicit bridge into the asynchronous world, and `main`'s
    /// only source of a token: mints one, requires the block to consume it
    /// (ordinary linearity), blocks the real thread until it is sent to, and
    /// yields what was sent. Legal only in `main`; the program still ends
    /// when `main` returns.
    WaitFor {
        /// The token's name inside the block.
        binding: Ident,
        /// Its declared type, written out (`Reply<Int>`).
        ty: Type,
        body: Block,
        span: Span,
    },
    /// Placeholder produced on parse errors so parsing can continue.
    Error { span: Span },
}

#[derive(Clone, Debug, PartialEq)]
pub enum StrExprPart {
    Text(String),
    Interp(Box<Expr>),
}

/// A call argument (possibly spread).
pub type Arg = Expr;

/// One `name = value` argument [implicit-override]: an implicit parameter
/// supplied at the call site rather than resolved.
#[derive(Clone, Debug, PartialEq)]
pub struct NamedArg {
    pub name: Ident,
    pub value: Expr,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StructLitField {
    pub kind: StructLitFieldKind,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum StructLitFieldKind {
    /// `name: expr`
    Named { name: Ident, value: Expr },
    /// `...expr`
    Spread(Expr),
}

#[derive(Clone, Debug, PartialEq)]
pub struct WhenBranch {
    /// The `is ...` / `^ ...` check.
    pub check: Vec<TypeRef>,
    pub binding: Option<Ident>,
    /// The branch head was `^` rather than `is` [qual-widen]: the same arm
    /// test, but the subject reads *without* the listed qualifiers inside
    /// the branch.
    pub widen: bool,
    pub body: Block,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LambdaParam {
    pub name: Ident,
    pub ty: Option<Type>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum LambdaBody {
    /// `i -> "${i}"` — the expression is the value; no `return` required.
    Expr(Box<Expr>),
    /// `i -> { return "${i}" }` — a full block; requires `return`.
    Block(Block),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnaryOp {
    Neg, // -x
    Not, // !x
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Eq,
    NotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    And,
    Or,
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::Int { span, .. }
            | Expr::Float { span, .. }
            | Expr::Bool { span, .. }
            | Expr::Char { span, .. }
            | Expr::Str { span, .. }
            | Expr::Field { span, .. }
            | Expr::TupleIndex { span, .. }
            | Expr::Scoped { span, .. }
            | Expr::EffectScoped { span, .. }
            | Expr::Call { span, .. }
            | Expr::Index { span, .. }
            | Expr::ArrayLit { span, .. }
            | Expr::SetLit { span, .. }
            | Expr::MapLit { span, .. }
            | Expr::Tuple { span, .. }
            | Expr::StructLit { span, .. }
            | Expr::Unary { span, .. }
            | Expr::Binary { span, .. }
            | Expr::Is { span, .. }
            | Expr::Widen { span, .. }
            | Expr::NonNull { span, .. }
            | Expr::IncDec { span, .. }
            | Expr::If { span, .. }
            | Expr::When { span, .. }
            | Expr::WhenCond { span, .. }
            | Expr::While { span, .. }
            | Expr::For { span, .. }
            | Expr::Lambda { span, .. }
            | Expr::Try { span, .. }
            | Expr::Spawn { span, .. }
            | Expr::ReplyTo { span, .. }
            | Expr::WaitFor { span, .. }
            | Expr::Spread { span, .. }
            | Expr::Error { span } => *span,
            Expr::Ident(ident) => ident.span,
        }
    }
}
