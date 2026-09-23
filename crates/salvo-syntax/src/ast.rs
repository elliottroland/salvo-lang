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
    /// [test-decl] `test "an empty heap pops nothing" { … }` — a test,
    /// named by a string. Legal only in a `<module>.test.sv` companion
    /// [test-file]; expanded into an ordinary exported fn before
    /// resolution (`desugar::expand_tests`), so nothing downstream knows
    /// the form exists.
    Test(TestDecl),
}

/// [test-decl] `test "trim removes both edges" { … }`: a test declaration.
///
/// Named by a **string literal** rather than an identifier — a test name is
/// prose, and there is nothing to call it by, so no identifier is invented
/// (user decision 2026-09-23). Not a function: it takes no parameters,
/// declares no effects and returns nothing, and `export test` is refused —
/// a test is run, never referenced.
#[derive(Clone, Debug, PartialEq)]
pub struct TestDecl {
    /// The `//` comment block directly above the declaration, one entry
    /// per line, `//` and one leading space stripped [doc-comment].
    pub docs: Vec<String>,
    /// The name as written, with no quotes. Interpolation is refused: the
    /// runner enumerates and filters tests without running anything, so a
    /// name has to be knowable statically [test-decl].
    pub name: String,
    /// The span of the name literal — the test's identity for diagnostics,
    /// and (uniquely per test) the span the synthesized fn is keyed by.
    pub name_span: Span,
    pub body: Block,
    pub span: Span,
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
    /// [mod-export] `export`: visible to other modules. Declarations are
    /// module-private by default (user decision 2026-09-18), so this is the
    /// whole of a module's public surface — a name without it can be used
    /// only inside the file that declares it [mod-file].
    pub exported: bool,
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
    /// [mod-export] `export`: visible to other modules. Declarations are
    /// module-private by default (user decision 2026-09-18), so this is the
    /// whole of a module's public surface — a name without it can be used
    /// only inside the file that declares it [mod-file].
    pub exported: bool,
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
    /// [linear-container] Per-parameter `canbe` opt-ins, exactly as a
    /// struct's (`intrinsic type List<T canbe linear>`, user decision
    /// 2026-09-16): an opaque container opts a parameter into holding
    /// **linear** values, and the instantiation is then linear exactly when
    /// that argument is. An opaque type has no fields to inspect, so an opted
    /// parameter is taken to reach one — which is what `List` means by
    /// holding its elements.
    pub generic_canbe: Vec<(Ident, TypeRef)>,
    /// [cmp-carry] The **slot list** in the generics, after the type
    /// parameters: `intrinsic type SortedSet<T, ?cmp: (T, T) -> Int = cmp>`, or a
    /// group spread (`?Ordered<T>`). Each slot carries an identity, not a type, so
    /// they are kept apart from `generics` — arity, `canbe` and substitution are
    /// about types only.
    pub fn_slots: Vec<SlotDecl>,
    /// Auto-qualifiers, e.g. `canbe Mut` [type-canbe-mut]: the type opts
    /// into the language-level `Mut` qualifier (like `struct ... canbe
    /// Mut` [struct-mut]).
    pub auto_qualifiers: Vec<TypeRef>,
    pub alias: Option<Type>,
    pub span: Span,
}

/// `struct Person canbe Mut { name: Str, ... }`, optionally with an
/// obligation clause: `linear struct Lines : Yield<self, Str> canbe Mut { ... }`
/// [group-obligation].
#[derive(Clone, Debug, PartialEq)]
pub struct StructDecl {
    /// The `//` comment block directly above the declaration, one entry
    /// per line, `//` and one leading space stripped [doc-comment].
    pub docs: Vec<String>,
    /// [mod-export] `export`: visible to other modules. Declarations are
    /// module-private by default (user decision 2026-09-18), so this is the
    /// whole of a module's public surface — a name without it can be used
    /// only inside the file that declares it [mod-file].
    pub exported: bool,
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
    pub obligations: Vec<Obligation>,
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

/// One entry of a struct's obligation clause [group-obligation].
#[derive(Clone, Debug, PartialEq)]
pub struct Obligation {
    /// [cmp-auto] `: auto Ordered<self>` — **sugar** for one bodiless
    /// `auto fn` per member of the group, `@`-scoped to this type (user
    /// decisions 2026-09-21, 2026-09-22). Legal only where every member has a
    /// generator (`cmp`, `eq`, `hash`); anywhere else it is an error naming
    /// them, since the word would otherwise promise something no code
    /// provides.
    ///
    /// Without it the clause is only a *promise*, checked at the declaration
    /// [group-obligation]: what satisfies it may be an `auto fn` or an
    /// ordinary one, which is how a type mixes a generated `cmp` with a
    /// hand-written `eq`.
    pub auto: bool,
    /// The group and its type arguments, `self` standing for the declaring
    /// type [group-self].
    pub group: TypeRef,
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
    /// [mod-export] `export`: visible to other modules. Declarations are
    /// module-private by default (user decision 2026-09-18), so this is the
    /// whole of a module's public surface — a name without it can be used
    /// only inside the file that declares it [mod-file].
    pub exported: bool,
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
    /// [cmp-carry] The **slot list** in the generics, after the type parameters:
    /// `qualifier Heap<T, ?cmp: (T, T) -> Int> of List<T>`, or a group spread
    /// (`?Ordered<T>`). The identity a use site writes (`Heap<Person, cmp@Person>`)
    /// fills one.
    pub fn_slots: Vec<SlotDecl>,
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
    /// [mod-export] `export`: visible to other modules. Declarations are
    /// module-private by default (user decision 2026-09-18), so this is the
    /// whole of a module's public surface — a name without it can be used
    /// only inside the file that declares it [mod-file].
    pub exported: bool,
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
    /// [actor-effect-kind] True for `actor effect` — a **actor protocol**
    /// (user decision 2026-09-15, EU-5). The kind is declared on the effect
    /// rather than diagnosed at a binding, because it is a design-time choice:
    /// an actor effect's members give up what cannot cross a seam (kept
    /// parameters, `Mut` parameters, `proj` returns, non-sendable payloads),
    /// and only an actor effect may be bound with `spawn` or `use addr`. A
    /// plain effect is never actor-backed.
    ///
    /// Like `platform`, a flag rather than a shared enum: the two are
    /// independent (a `platform effect` is a host interface, an `actor effect`
    /// a message protocol) and nothing yet is both.
    pub is_actor: bool,
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
    /// [mod-export] `export`: visible to other modules. Declarations are
    /// module-private by default (user decision 2026-09-18), so this is the
    /// whole of a module's public surface — a name without it can be used
    /// only inside the file that declares it [mod-file].
    pub exported: bool,
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
    /// [effect-handler-multi] The effects this handler *implements*, in
    /// declaration order: `handler ManualTime() of Timer, TimerCtl`. Always at
    /// least one, and one is the common case — several is how a handler wears
    /// more than one face over the same state (T-4(a), user decision
    /// 2026-09-17), which a `spawn` answers one addr per and a `use` binds all
    /// of.
    pub of: Vec<Type>,
    /// [actor-mailbox] `mailbox { capacity: 16 }` — the **actor settings slot**
    /// (user decision 2026-09-16). A handler of an `actor effect` states its
    /// mailbox here rather than at every spawn: the author who knows the
    /// protocol's traffic is the one writing the handler, and the bound is then
    /// written once instead of at each binding site.
    ///
    /// Held as the **struct literal it is**: `mailbox` names a compiler-known
    /// slot whose type is std's `Mailbox`, and the braces are that struct's
    /// literal with the type elided, so field names, types, defaults and
    /// diagnostics are the ordinary ones. Its field expressions may read
    /// **constructor parameters only** — the value is needed before the actor
    /// exists, ahead of any state initialiser.
    pub mailbox: Option<Expr>,
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
    /// [mod-export] `export`: visible to other modules. Declarations are
    /// module-private by default (user decision 2026-09-18), so this is the
    /// whole of a module's public surface — a name without it can be used
    /// only inside the file that declares it [mod-file].
    pub exported: bool,
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
    /// [actor-send-fn] `send fn bump(n: Int)`: an **asynchronous** member —
    /// a message, not a call. Sending one enqueues an invocation on the
    /// target actor and returns immediately, so the member answers
    /// nothing: a reply travels as a `Reply<T>` parameter the sender mints
    /// with `replyto`. In the first pass every member of an actor protocol
    /// is one, and the unmarked `fn` spelling stays reserved for the later
    /// call-member sugar (a `-> T` member desugars *to* a `send fn` with a
    /// trailing token). Contextual, like `iter fn`: `send` is not a
    /// reserved word.
    pub is_send: bool,
    pub name: Ident,
    /// [cmp-canonical] `fn cmp@Person(a: Person, b: Person) -> Int`: the
    /// **canonical** implementation of a capability for a type, `@`-scoped to
    /// it (user decision 2026-09-21). An ordinary top-level overload with two
    /// extra properties — it *travels with the type* (importing `Person`
    /// imports it, so the canonical is in scope wherever the type is usable)
    /// and it is the **default selection** for an implicit parameter of the
    /// same name and shape [implicit-resolve]. Declared in the type's own
    /// file, with an `export` matching the type's.
    ///
    /// The same `@` the language already uses to select a scope
    /// ([fn-overload-at]'s `size@core.list`, [effect-at]'s `close@Fs`),
    /// extended from modules and effects to types — on both sides, since
    /// `cmp = cmp@Person` is how a call names one explicitly.
    pub scoped_to: Option<Ident>,
    /// [cmp-auto] `auto fn cmp@Person(a: Person, b: Person) -> Int`: the
    /// **structural** implementation of a capability member for the type it is
    /// `@`-scoped to, written by the compiler (user decisions 2026-09-21,
    /// 2026-09-22). It has no body — each backend lowers it to the host's own
    /// derived comparison, equality or hash (the lowering-split-by-author
    /// rule), which is what keeps everything generated consistent with
    /// everything else generated, by construction.
    ///
    /// Set by the `auto` modifier on a declaration, and by the `auto
    /// Group<self>` obligation sugar, which expands to one such declaration per
    /// member. Nothing downstream distinguishes the two: an `auto fn` is an
    /// ordinary overload for resolution, the duplicate check, mangling and
    /// export.
    pub structural: bool,
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
    /// [actor-spawn-effect] The special `spawn` effect, allowing actor
    /// creation. Lowercase and compiler-owned like `use`: it names a
    /// capability rather than a declared effect, so there is no handler to
    /// resolve and nothing to thread — `fn main() [use, spawn]` is the
    /// typical entry point.
    Spawn(Span),
    /// A named effect, possibly generic: `Random<Int>`. [effect-local]
    /// `local` (contextual, user decision 2026-09-20) accepts a
    /// scope-local binding and disclaims seam rights for it; the default
    /// requires a shareable one.
    Effect(TypeRef),
    /// [effect-local] `local Random<Int>` — the requirement that accepts a
    /// `use local` binding. Kept apart from `Effect` so every existing
    /// match keeps meaning "the shareable default".
    LocalEffect(TypeRef),
}

/// An entry in a function's deduction clause [deduce-syntax], written after
/// the return type behind `=>`:
/// `=> person` (bare — every qualifier the argument has survives),
/// `=> list: Mut` (*exhaustive* — afterwards only `Mut` applies),
/// `=> list: None` (exhaustive and empty — every qualifier stripped),
/// `=> list: -NonEmpty` (*delta* — drop `NonEmpty`, keep the rest),
/// `=> !list` (moved; `list: Never` says the same),
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
    /// `reapplied` holds the ones written `+Q` — the claims this function
    /// **re-establishes** rather than merely preserves [deduce-reapply], which
    /// is what lets a mutator keep a qualifier its own body strips. Trusted,
    /// and only legal in the file declaring the qualifier; the rest of the list
    /// is validated against the body as ever. The caller sees the union: an
    /// exhaustive list means *these and nothing else*, however each got there.
    Exhaustive {
        quals: Vec<TypeRef>,
        reapplied: Vec<TypeRef>,
    },
    /// `=> list: -A -B`: these are dropped, everything else survives.
    Remove(Vec<TypeRef>),
    /// `=> !list` / `=> list: Never`: moved (the caller loses access).
    Moved,
    /// [defer-deduction] `=> defer out`: consumed like `!out`, and the
    /// obligation it carries **may outlive the frame** — parked, stored,
    /// forwarded or captured instead of discharged here (SH-10, user
    /// decision 2026-09-19). An upper bound: a declared `defer` need not be
    /// exercised. The load-bearing consequence is the mixed handler's
    /// rung-4 opt-in [mixed-handler]; elsewhere it is checked documentation.
    Deferred,
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

/// [cmp-carry] Whether a written type argument is a function **identity** rather
/// than a type: the signature's binder (`?cmp`), a selector (`cmp@Person`), or a
/// bare lowercase name — [name-casing] reserves uppercase for types, so a
/// lowercase bare name in a type-argument position is a function.
///
/// Both backends need this: an identity is carried by the checker and by the
/// container's own machinery, never by the emitted type, so it is dropped when a
/// type is rendered. One definition, so the two cannot drop different things.
pub fn is_identity_arg(arg: &Type) -> bool {
    match arg {
        Type::Named { qualifiers, base } => {
            qualifiers.is_empty()
                && base.args.is_empty()
                && (base.binder
                    || base.at.is_some()
                    || base.name.name.starts_with(|c: char| c.is_lowercase()))
        }
        _ => false,
    }
}

/// [cmp-carry] One entry of a declaration's **slot list**: a slot written out,
/// or a `params` group spread into one slot per member (user decision
/// 2026-09-22).
///
/// `qualifier Heap<T, ?Ordered<T>> of List<T>` is sugar for
/// `qualifier Heap<T, ?cmp: (T, T) -> Int>`, exactly as `?Ordered<T>` in a
/// parameter list is sugar for the members as implicit parameters
/// [implicit-group]. One ordered list, because slots are filled positionally and
/// a group's members take the positions where the spread is written.
#[derive(Clone, Debug, PartialEq)]
pub enum SlotDecl {
    /// `?cmp: (T, T) -> Int = cmp`
    One(FnSlot),
    /// `?Ordered<T>` — expanded by the checker, which is where a group's
    /// members are visible.
    Group(TypeRef),
}

/// [cmp-carry] A **fn slot** in a declaration's generics list:
/// `qualifier Heap<T, ?cmp: (T, T) -> Int>`,
/// `intrinsic type SortedSet<T, ?cmp: (T, T) -> Int>`.
///
/// There is no default to write: **the slot's name *is* its default**, because
/// that is how an implicit parameter already works [implicit-resolve] — a `?cmp`
/// nobody writes is resolved by the name `cmp` where it is needed (user decision
/// 2026-09-22).
///
/// The position a function **identity** fills, spelled like the implicit
/// parameter it is resolved as [implicit-param] — so a reader who knows
/// `?cmp: (T, T) -> Int` on a fn knows it here. Slots trail the ordinary
/// type parameters, for the same reason an implicit parameter trails the
/// ordinary ones: a type argument written after one could not be passed.
#[derive(Clone, Debug, PartialEq)]
pub struct FnSlot {
    pub name: Ident,
    /// The fn type an identity must have to fill the slot, over the
    /// declaration's own type parameters.
    pub ty: Type,
    pub span: Span,
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
    /// [cmp-carry] `cmp@Person` in a type-argument position: the selector
    /// naming which `cmp` this identity is — a type (`@Person`, an
    /// `@`-scoped canonical [cmp-canonical]) or a module (`@core.list`,
    /// dotted, [fn-overload-at]). Only a function **identity** carries one;
    /// a type never does, so a selector here is what tells the two apart
    /// without knowing the slot.
    pub at: Option<Ident>,
    /// [cmp-carry] `?cmp`: this argument is the signature's **binder**
    /// rather than a named fn or a type.
    pub binder: bool,
    /// [cmp-binder] `?cmp: cmp2` — the **alias** a binder is introduced under,
    /// when the slot's own name is taken. The destructuring spelling
    /// (`field: variable_name`), for the same reason: the left is what is being
    /// named, the right is the name it gets here. Only meaningful with
    /// `binder`.
    pub alias: Option<Ident>,
    pub span: Span,
}

// --- Source-like rendering (hover, diagnostics) ---

impl fmt::Display for TypeRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // [cmp-carry] An identity reads as it is written: `?cmp`, `cmp@Person`.
        if self.binder {
            write!(f, "?")?;
        }
        write!(f, "{}", self.name.name)?;
        if let Some(alias) = &self.alias {
            write!(f, ": {}", alias.name)?;
        }
        if let Some(at) = &self.at {
            write!(f, "@{}", at.name)?;
        }
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
            EffectRef::LocalEffect(r) => write!(f, "local {r}"),
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
    /// `use HandlerExpr(...)` — register a handler for the current context.
    /// [use-local] `use local HandlerExpr(...)` binds it **scope-local**
    /// (lock-free, unshareable) instead of shareable-by-default (user
    /// decision 2026-09-20).
    /// [with-clause] `use H(...) with D(), addr` supplies `H`'s declared
    /// dependencies from the clause instead of from the scope — private
    /// instances, the self-dependency included (user decision 2026-09-20).
    Use {
        handler: Expr,
        local: bool,
        with_items: Vec<Expr>,
        span: Span,
    },
    /// [fn-rename] `rename fn add2 = add(a: Int, b: Int)` inside a block:
    /// in force from this line to the end of the enclosing scope.
    Rename(RenameDecl),
    /// A bare expression statement.
    Expr(Expr),
}

/// [pick] A qualifier named before an `?:`.
#[derive(Clone, Debug, PartialEq)]
pub struct ElvisPick {
    /// The qualifiers naming the arms to pick.
    pub quals: Vec<TypeRef>,
    /// Every qualifier was `^`-marked, so the picked value reads **without**
    /// them — `^Ok` gives `T` where `Ok` gives `Ok T` [qual-lift].
    pub lift: bool,
    pub span: Span,
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
    /// `expr is ^Qual...` — the *widening* check [qual-lift]: the dual of
    /// `is`. Where a successful `is` narrows the subject (a qualifier added,
    /// a union arm picked), a successful `^` **generalizes** it by removing
    /// the listed qualifiers — `list is ^Mut` reads `list` without `Mut`
    /// afterwards, `outcome is ^Ok` without `Ok`. Boolean-valued, like `is`;
    /// the runtime test is the same one `is` performs.
    Widen {
        subject: Box<Expr>,
        /// The qualifiers to remove (all must be present and droppable).
        quals: Vec<TypeRef>,
        /// [qual-lift] `is ^Ok inner` — the lifted value bound to a fresh
        /// name for the branch (user decision 2026-09-21). Without one the
        /// subject itself reads lifted, as it always did.
        binding: Option<Ident>,
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
    /// [elvis] `subject ?: rhs` — the **optional-or-else** operator (user
    /// decision 2026-09-21, step 4 of the `?` family sequence). The subject
    /// must have a `None` arm; the expression is the subject's non-`None` arms
    /// when it has one of them, and `rhs` otherwise. `rhs` sees `_` bound to
    /// the `None` side, so `x ?: return _` is the same as `x ?: return None`.
    ///
    /// Reserved for `T?`: a *qualifier* is picked by naming it (step 5).
    Elvis {
        subject: Box<Expr>,
        /// [pick] The qualifier picked before the `?:`, if any: `x Ok?: r` picks
        /// the `Ok` arm keeping its tag, `x ^Ok?: r` picks it and **lifts** the
        /// tag (the same `^Q` notation an `is` check uses [qual-lift]). `None`
        /// here is the plain `?:`, which picks the non-`None` arms [elvis].
        pick: Option<ElvisPick>,
        rhs: Box<Expr>,
        span: Span,
    },
    /// [safe-call] `subject?.name` / `subject?.name(args)` — a field read or a
    /// dot-notation call on the **non-`None`** side of an optional (user
    /// decision 2026-09-21, step 4). The result is the member's own type
    /// re-unioned with `None`, so a chain re-tests at each link, as Kotlin's
    /// does.
    ///
    /// Reserved for `T?`, like `?:`. Held as its own node rather than a flag on
    /// `Field`/`Call` because the *whole* postfix step is conditional: the
    /// member is not reached at all when the subject is `None`.
    SafeField {
        /// The receiver, held separately so the checker can strip its `None`
        /// before `inner` is typed.
        base: Box<Expr>,
        /// The **equivalent ordinary access** over the same base: a `Field`, or
        /// a `Call` whose callee is one. Holding it means the whole existing
        /// path types and emits it — field overrides, overload resolution,
        /// effects, diagnostics — with `?.` adding only the conditional and the
        /// `None` arm of the result.
        inner: Box<Expr>,
        span: Span,
    },
    /// [placeholder] `_` — the value the enclosing construct left unnamed.
    /// Legal only inside an `?:` right-hand side today, where it is the
    /// `None` side of the subject.
    Placeholder { span: Span },
    /// [expr-escape] `return expr?` — an **expression** of type `Never`
    /// (user decision 2026-09-21), not a statement. The three escapes are
    /// expressions so that a tail position can hold one without the grammar
    /// naming them: `maybe_t() ?: return _` needs no exception, and a
    /// `Never`-typed escape behaves exactly as a `Never`-returning call like
    /// `throw(m)` already did [type-any-never].
    Return { value: Option<Box<Expr>>, span: Span },
    /// [expr-escape] `break expr?` — type `Never`.
    Break { value: Option<Box<Expr>>, span: Span },
    /// [expr-escape] `continue` — type `Never`.
    Continue { span: Span },
    /// `...expr` — spread in call arguments or struct literals.
    Spread { operand: Box<Expr>, span: Span },
    /// [actor-self-send] `k@self(args)` — a message to **the actor the
    /// enclosing member belongs to**: the selector names the enclosing
    /// handler, as `k@E` names an effect and `k@module` a module's overload.
    ///
    /// A leaf: there is no receiver expression to hold, because the target is
    /// the handler the code is written in. Legal only as a call's callee, and
    /// only inside a handler member — both the checker's rules.
    SelfScoped { name: Ident, span: Span },
    /// [actor-spawn-expr] `spawn Counting(0) with ScriptedDb(f), ddb
    /// on pool(2)` — bind a handler *asynchronously*: the actor. Read left to
    /// right: what to run, what it depends on, where it runs. Its value is the
    /// child's `Addr`.
    ///
    /// The mailbox is **not** here: it is the handler's own
    /// `mailbox { capacity: … }` slot [actor-mailbox], stated once by the
    /// author who knows the protocol rather than at every spawn (user decision
    /// 2026-09-16, replacing the frozen `capacity N` clause).
    ///
    /// `spawn` and the clause words are **contextual** (the `iter fn`
    /// precedent): the form is recognised from `spawn` followed by a name,
    /// so a function called `spawn` keeps working.
    Spawn {
        /// The handler construction — a name plus its constructor
        /// arguments, the shape `use` takes, because a handler is not a
        /// value and only `use`/`spawn` may construct one.
        handler: Box<Expr>,
        /// [with-clause] The `with` clause, empty when it is absent: handler
        /// constructions *or* `Addr` values, supplying the child's declared
        /// dependencies [effect-handler-deps] instead of the spawning
        /// scope's resolution [spawn-inherit]. Arguments evaluate in the
        /// parent and cross the seam; construction happens on the child.
        /// Spelled `with` since 2026-09-20 (user decision, superseding the
        /// clause's original `use`, which shared a word with the statement).
        with_items: Vec<Expr>,

        /// `on POOL` — an ordinary expression. `pool(n)` is a function, not
        /// syntax.
        ///
        /// [main-pool] **Optional**: omitted, the child runs on the pool
        /// current where the spawn is written (in `main`, the main pool; in a
        /// member, the actor's own pool) — FC-3's inheritance rule extended
        /// to `spawn`, which is also how a spawn site *names* the main pool
        /// without new vocabulary.
        pool: Option<Box<Expr>>,
        span: Span,
    },
    /// [actor-replyto] `replyto batch_arrived(id)` — allocate a parked
    /// one-shot continuation targeting a member of the *enclosing handler*,
    /// yielding its linear `Reply<T>` [linear-obligation]. The arguments are
    /// the continuation's *captures*: what the member needs besides the
    /// answer it is waiting for.
    ///
    /// `replyto!` (`gated`) is the same mint plus the gate — bounded
    /// selective receive, at most one outstanding per actor — so the
    /// actor serves nothing else until the answer arrives.
    ReplyTo {
        member: Ident,
        captures: Vec<Expr>,
        /// Written `replyto!`: the gated mint.
        gated: bool,
        /// [task-pool-inherit] `on POOL` — where the continuation runs, for a
        /// mint that targets a free `send fn` [free-send-fn]. Optional: omitted
        /// means the pool current at the mint site, so whoever creates work
        /// pays for it. A mint at a *member* target ignores it — the answer
        /// arrives on the actor's own mailbox — so writing one there is an
        /// error.
        pool: Option<Box<Expr>>,
        span: Span,
    },
    /// [actor-waitfor] `waitfor out: Reply<Int> { counter.total(out) }` —
    /// `main`'s explicit bridge into the asynchronous world, and `main`'s
    /// only source of a token: mints one, requires the block to consume it
    /// (ordinary linearity), blocks the real thread until it is sent to, and
    /// yields what was sent. Legal only in `main`; the program still ends
    /// when `main` returns.
    WaitFor {
        /// The token's name inside the block.
        binding: Ident,
        /// [waitfor-infer] Its type, when written (`waitfor out: Reply<Int>`).
        /// **Optional** since 2026-09-21: omitted, it is inferred from where the
        /// binder is used in the block, and written out only where that is
        /// ambiguous.
        ty: Option<Type>,
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
    /// The `is ...` check, with or without `^`-lifted qualifiers.
    pub check: Vec<TypeRef>,
    pub binding: Option<Ident>,
    /// [qual-lift] Every qualifier in the check was `^`-marked: the same arm
    /// test, but the subject reads *without* them inside the branch.
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
            | Expr::Return { span, .. }
            | Expr::Break { span, .. }
            | Expr::Continue { span, .. }
            | Expr::Elvis { span, .. }
            | Expr::Placeholder { span }
            | Expr::SafeField { span, .. }
            | Expr::SelfScoped { span, .. }
            | Expr::Spawn { span, .. }
            | Expr::ReplyTo { span, .. }
            | Expr::WaitFor { span, .. }
            | Expr::Spread { span, .. }
            | Expr::Error { span } => *span,
            Expr::Ident(ident) => ident.span,
        }
    }
}
