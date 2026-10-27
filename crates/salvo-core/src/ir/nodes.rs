//! The IR node set [ir-nodes]: a minimal, decided form of a Salvo program
//! (IR record §3). Every reference is absolute, every expression carries its
//! type, every narrowing is a binding, and nothing is left for a backend to
//! infer. What a backend adds is representation and idiom (IR record §1).


pub use crate::param_mode::PassMode;
pub use crate::types::Ty;
pub use crate::ModulePath;
pub use salvo_syntax::Span;

/// [decl-id] The identity of a declaration: the checker's, so a reference
/// is the declaration the checker resolved.
pub use crate::decl_id::DeclId;

/// A local variable, unique within its fn body.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Local(pub String);

/// A node id, unique within a fn body; what a `Narrow` names as its reason.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NodeId(pub u32);

pub struct Program {
    pub modules: Vec<Module>,
    /// The dump name of every declaration, program-wide.
    pub names: std::collections::HashMap<DeclId, String>,
    /// [fate-move-mode] The flow pass's decision the bodies were built from.
    pub decision: Decision,
}

pub struct Module {
    pub path: ModulePath,
    /// The file this module came from (diagnostics, trap locations).
    pub file_name: String,
    pub decls: Vec<Decl>,
    /// [namespace] The dotted paths of every `namespace` declared in this
    /// file (`AttributeValue`, `Outer`, `Outer.Inner`), carried from the AST.
    /// A backend that nests namespaces (Kotlin) reads this to build the
    /// nesting tree, since a flattened dotted name alone cannot say whether a
    /// prefix was a `namespace` or a dot-named struct (they may share a name).
    pub namespaces: Vec<String>,
}

pub enum Decl {
    Struct(StructDecl),
    Union(UnionDecl),
    Interface(InterfaceDecl),
    Impl(ImplDecl),
    Fn(FnDecl),
    PlatformType(PlatformTypeDecl),
    /// [mod-use] a module-level `use`: an instance every fn of the module
    /// reads as a local, constructed once (when, is the backend's).
    Static(StaticDecl),
    /// [actor-msg] A generated enum with named variants: an actor
    /// interface's messages, a handler's private messages or continuations.
    /// The builder writes it (IR record §6); the backend picks the layout.
    Enum(EnumDecl),
}

/// What an [`EnumDecl`] stands for.
#[derive(Clone, Debug, PartialEq)]
pub enum EnumKind {
    /// `__Msg_E`: one variant per `send` member of the actor interface.
    Message { interface: DeclId },
    /// `__Priv_H`: `Init`, and one variant per `send fn` no face declares.
    Private { handler: DeclId },
    /// `__Cont_H`: one variant per member that parks on a reply, holding
    /// what the member had captured when it parked.
    Continuation { handler: DeclId },
}

pub struct EnumDecl {
    pub id: DeclId,
    pub name: String,
    pub kind: EnumKind,
    pub variants: Vec<Variant>,
    /// [protocol-hash] the canonical hash of a message enum with a wire form.
    pub protocol_hash: Option<String>,
    pub span: Span,
}

pub struct Variant {
    /// The variant's name (`Bump` for member `bump`; `Init` for the init).
    pub name: String,
    /// The member this variant carries: the emitted name for a message, the
    /// source name for a private or continuation variant; `None` for `Init`.
    pub member: Option<String>,
    /// The payload: the member's parameter names and types.
    pub fields: Vec<(String, Ty)>,
}

pub struct StaticDecl {
    pub id: DeclId,
    pub local: Local,
    pub ty: Ty,
    /// The construction, and the instances it binds (one per face).
    pub stmts: Vec<Stmt>,
    pub span: Span,
}

impl Decl {
    pub fn id(&self) -> &DeclId {
        match self {
            Decl::Struct(d) => &d.id,
            Decl::Union(d) => &d.id,
            Decl::Interface(d) => &d.id,
            Decl::Impl(d) => &d.id,
            Decl::Fn(d) => &d.id,
            Decl::PlatformType(d) => &d.id,
            Decl::Static(d) => &d.id,
            Decl::Enum(d) => &d.id,
        }
    }
    pub fn name(&self) -> &str {
        match self {
            Decl::Struct(d) => &d.name,
            Decl::Union(d) => &d.name,
            Decl::Interface(d) => &d.name,
            Decl::Impl(d) => &d.name,
            Decl::Fn(d) => &d.name,
            Decl::PlatformType(d) => &d.name,
            Decl::Static(d) => &d.local.0,
            Decl::Enum(d) => &d.name,
        }
    }
}

pub struct TypeParam {
    pub name: String,
    /// [canbe-linear] the parameter may be instantiated at a linear type.
    pub canbe_linear: bool,
}

pub struct StructDecl {
    pub id: DeclId,
    pub name: String,
    pub exported: bool,
    pub type_params: Vec<TypeParam>,
    pub fields: Vec<Field>,
    pub linear: bool,
    pub opaque: bool,
    /// [type-canbe-mut] values of this struct may be `Mut`.
    pub canbe_mut: bool,
    /// [wire-format] every field has a wire form.
    pub has_wire_form: bool,
    pub span: Span,
}

pub struct Field {
    pub name: String,
    pub ty: Ty,
    /// [field-canbe-mut] `Mut` when the struct value is `Mut`.
    pub canbe_mut: bool,
    pub default: Option<Expr>,
}

/// A named union type (`type Either = A | B`). Arms in identity order
/// [union-arm-identity]; a `None` arm is recorded as `has_none`.
pub struct UnionDecl {
    pub id: DeclId,
    pub name: String,
    pub exported: bool,
    pub type_params: Vec<TypeParam>,
    pub ty: Ty,
    /// [platform-factory] the factories a host builds this union with, when
    /// a platform signature reaches it.
    pub factories: Option<crate::abi::Factories>,
    pub span: Span,
}

/// An effect: the interface its handlers implement.
pub struct InterfaceDecl {
    pub id: DeclId,
    pub name: String,
    pub exported: bool,
    pub type_params: Vec<TypeParam>,
    /// [actor-effect-kind] members are messages.
    pub actor: bool,
    /// [protocol-hash] the hash of an actor effect's canonical form, when
    /// every message has a wire form; what two nodes compare before talking.
    pub protocol_hash: Option<String>,
    /// [effect-prereq] effects every member needs in scope.
    pub prereqs: Vec<Ty>,
    pub members: Vec<Member>,
    pub span: Span,
}

/// An effect member's signature. The effect's prerequisites
/// [effect-prereq] are not parameters: a handler of the effect depends on
/// them.
pub struct Member {
    pub name: String,
    /// The emitted name when the effect overloads the name
    /// [effect-member-overload]; equal to `name` otherwise.
    pub emitted_name: String,
    pub type_params: Vec<TypeParam>,
    pub params: Vec<Param>,
    pub ret: Ty,
    /// [actor-send-fn] a message: asynchronous, answers nothing.
    pub send: bool,
    /// [platform-check] what a platform handler's answer is checked for.
    pub result_check: Option<crate::abi::BoundaryCheck>,
    /// [platform-factory] the factories of the result and of the `Reply<T>`
    /// payloads a host builds, when a platform handler implements the effect.
    pub factories: Vec<crate::abi::Factories>,
    pub span: Span,
}

/// A handler: an implementation of one or more interfaces.
pub struct ImplDecl {
    pub id: DeclId,
    pub name: String,
    pub exported: bool,
    pub type_params: Vec<TypeParam>,
    /// [effect-handler-multi] the interfaces implemented, declaration order.
    pub faces: Vec<Ty>,
    /// [effect-any] parallel to `faces`: a router over any instance.
    pub face_any: Vec<bool>,
    pub ctor_params: Vec<Param>,
    /// [effect-handler-deps] the instances a constructor is given, in
    /// declaration order, as trailing constructor parameters.
    pub deps: Vec<Ty>,
    pub state: Vec<Field>,
    /// [actor-mailbox] the mailbox capacity, when declared.
    pub mailbox: Option<Expr>,
    /// [handler-init] run once after construction.
    pub init: Option<FnDecl>,
    pub members: Vec<FnDecl>,
    /// [effect-handle] a shared instance is entered one member at a time.
    pub stateful: bool,
    pub platform: bool,
    pub threadsafe: bool,
    pub intrinsic: bool,
    /// [actor-dispatch] For an actor handler: the generated function that
    /// delivers each message enum it serves to the handler's members.
    pub dispatch: Vec<Dispatch>,
    /// [actor-use-addr] The generated `__Stub_E`: the effect implemented by
    /// sending each member to an address. Not an actor itself.
    pub stub: bool,
    pub span: Span,
}

/// [actor-dispatch] `func(handler, msg)` delivers a `message` to the handler.
pub struct Dispatch {
    /// The [`EnumDecl`] it dispatches.
    pub message: DeclId,
    /// The generated [`FnDecl`], in the handler's module.
    pub func: DeclId,
}

pub struct PlatformTypeDecl {
    pub id: DeclId,
    pub name: String,
    pub exported: bool,
    pub type_params: Vec<TypeParam>,
    pub linear: bool,
    pub canbe_mut: bool,
    /// [platform-type] the host defines it (an `intrinsic type` is the
    /// backend's own).
    pub platform: bool,
    /// [platform-type] `threadsafe`: shared across threads at once.
    pub threadsafe: bool,
    /// [platform-slots] it keeps fn values by identity (so it is no key).
    pub slots: bool,
    /// [platform-iterable] a `for` loops over it, yielding `iter_elem`.
    pub iterable: bool,
    pub iter_elem: Option<Ty>,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FnKind {
    Plain,
    /// [intrinsic-fn] no body; the backend lowers it from its own table.
    Intrinsic,
    /// [platform-fn] no body; the host implements it.
    Platform,
    /// A member body of an `Impl`: `faces` says which interface members it
    /// implements.
    Member { faces: usize },
    /// A `qualifies` fn of a predicate qualifier [qual-predicate].
    Qualifies,
}

pub struct FnDecl {
    pub id: DeclId,
    pub name: String,
    pub exported: bool,
    pub kind: FnKind,
    /// [actor-send-fn] a `send fn`: a free one is a task, a handler's one a
    /// message (private when no face declares it [actor-private-send]).
    pub send: bool,
    pub type_params: Vec<TypeParam>,
    /// Effects first (as `Param`s of interface type), then the declared
    /// parameters, then the implicit ones [implicit-param].
    pub params: Vec<Param>,
    /// How many leading `params` are effect instances.
    pub effect_params: usize,
    /// How many trailing `params` are implicits.
    pub implicit_params: usize,
    pub ret: Ty,
    /// [proj-infer] indices into `params` the result holds a view of.
    pub borrows: Vec<usize>,
    /// [deduce-field] `p.f: proj(q)`: after the call, parameter `.0` holds a
    /// view of parameter `.1` (indices into `params`).
    pub holds: Vec<(usize, usize)>,
    /// [ref-anchor] `a: ref(c) Mut T`: parameter `.0` is a handle into the
    /// container parameter `.1` (indices into `params`). Two parameters
    /// anchored at one container may alias; a backend with ownership
    /// passes the container once and each handle as a position in it.
    pub ref_anchors: Vec<(usize, usize)>,
    /// [throw] the message type this fn may throw, when it declares `[Throw<M>]`.
    pub throws: Option<Ty>,
    /// [platform-check] a platform fn's result check.
    pub result_check: Option<crate::abi::BoundaryCheck>,
    /// [platform-factory] a platform fn's result factories.
    pub factories: Option<crate::abi::Factories>,
    pub body: Option<Block>,
    pub span: Span,
    /// [fate-poison] Flow-only facts for `ir::flow`; no emitter reads them
    /// and the dump does not show them.
    pub fate: FateFacts,
}

/// [fate-poison] What the IR flow pass needs to poison a derived value and
/// name a move-mode binding, beyond the nodes themselves — from the
/// checker's tables, keyed by what the IR carries. Flow-only.
#[derive(Clone, Default)]
pub struct FateFacts {
    /// A borrow-mode binding's fate links, by its statement's `NodeId` (a
    /// `Let`, an `Alias`, a `ForEach`): the roots it shares fate with, at
    /// the projection it came from [fate-link] [fate-field-disjoint].
    pub binds: std::collections::HashMap<u32, Vec<crate::check::FlowLink>>,
    /// The same for an assignment of a whole local, by the assigned value's
    /// span (an `Assign` has no id).
    pub assigns: std::collections::HashMap<Span, Vec<crate::check::FlowLink>>,
    /// [fate-move-mode] A move-mode binding, by its statement's `NodeId`:
    /// its name, and the links it took ownership through — each root is
    /// consumed at the binding (named by it), what was derived from it is
    /// poisoned.
    pub bound: std::collections::HashMap<u32, (String, Vec<crate::check::FlowLink>)>,
    /// The same for a move-mode assignment, by the assigned value's span.
    pub assign_bound: std::collections::HashMap<Span, (String, Vec<crate::check::FlowLink>)>,
    /// Mutation events on a root, by the span of the IR expression they
    /// happen at (a call, an iterated subject): applied after it.
    pub mutations: std::collections::HashMap<Span, Vec<crate::check::FlowMutation>>,
    /// Mutation events of an assignment through a projection, by the
    /// assigned value's span: applied before the value (the checker's
    /// order — the target, then the value).
    pub assign_mutations: std::collections::HashMap<Span, Vec<crate::check::FlowMutation>>,
    /// [linear-obligation] Every written assignment of a whole local, by
    /// the assigned value's span: the statement's span (where "assigning
    /// drops the linear value" points). A builder-made assignment (a loop's
    /// result, a pick) has none.
    pub assign_spans: std::collections::HashMap<Span, Span>,
    /// [flow-use-after-move] An assignment through a projection, by the
    /// assigned value's span: the span of the target's root as written
    /// (`p` in `p.hp = 5`), where a write into a moved or poisoned value
    /// is reported.
    pub assign_targets: std::collections::HashMap<Span, Span>,
    /// [linear-obligation] A written loop's span, by its `Loop`/`ForEach`
    /// id: where a join over its exits points.
    pub loop_spans: std::collections::HashMap<u32, Span>,
    /// [linear-state] The fn's name, where "cannot end this member" points.
    pub name_span: Span,
    /// [linear-state] In a handler member: the state fields holding
    /// obligations, which the member must leave whole.
    pub linear_state: Vec<String>,
    /// [fate-move-mode] The bindings (by their stamp, the binding's span)
    /// that may take ownership — the checker's judgment where the binding
    /// is (`Checked::bind_facts`): the flow pass decides which do.
    pub eligible: std::collections::HashSet<Span>,
    /// [fate-move-mode] Per binding stamp, the `for` loops (by iterable
    /// span) whose binding is one of its roots: taking ownership makes them
    /// iterate by value.
    pub origins: std::collections::HashMap<Span, Vec<Span>>,
    /// [fate-move-mode] [fate-derived-readonly] Moves and mutations of a
    /// derived local, by the span of the IR expression they happen at: the
    /// local, the action, and the S1 error should its binding stay
    /// borrow-mode.
    pub uses: std::collections::HashMap<Span, Vec<DerivedUseAt>>,
    /// The same for a mutation by an assignment through a projection, by
    /// the assigned value's span.
    pub assign_uses: std::collections::HashMap<Span, Vec<DerivedUseAt>>,
    /// [deduce-field] The bindings a mutation event at a span may leave as
    /// virtual places (`Checked::virtual_candidates`), by the event's span.
    pub virtual_at: std::collections::HashMap<Span, Vec<VirtualAt>>,
}

/// [fate-move-mode] A move or mutation of a derived local (`FateFacts::uses`).
#[derive(Clone, Debug)]
pub struct DerivedUseAt {
    /// The local, as the IR names it.
    pub local: String,
    pub action: String,
    pub message: String,
    /// Where the checker placed the use: where the S1 error points.
    pub at: Span,
    /// The bindings the local's own links were made by (the checker's, at
    /// the use: an `is` binding is the same storage as its subject in the
    /// IR, so its link is not a local's here).
    pub own: Vec<Span>,
}

/// [deduce-field] A binding a mutation event may leave as a virtual place:
/// its stamp, the place, and — for a fields-declaring call's survivor — the
/// local that must still be live there.
#[derive(Clone, Debug)]
pub struct VirtualAt {
    pub stamp: Span,
    pub place: String,
    pub local: Option<String>,
}

/// [fate-move-mode] What the IR flow pass decided over a program's first
/// build (`flow::decide`), which the builder then builds from: the
/// bindings that take ownership (by stamp), the statements and loops the
/// builder lowers as moves (`modes`: those bindings, and the loops they
/// make iterate by value), and the bindings rendered as virtual places
/// [deduce-field]. Keyed as the checker's tables are [node-id].
#[derive(Clone, Default, Debug)]
pub struct Decision {
    pub decided: std::collections::HashSet<crate::check::Key>,
    pub modes: std::collections::HashSet<crate::check::Key>,
    pub virtual_binds: std::collections::HashMap<crate::check::Key, String>,
}

#[derive(Clone)]
pub struct Param {
    pub local: Local,
    pub ty: Ty,
    pub mode: PassMode,
    /// [fn-variadic] the trailing list of the rest of the arguments.
    pub variadic: bool,
    /// [platform-check] for a `Reply<T>` of a platform-handled member: what
    /// the host's answer on it is checked for.
    pub check: Option<crate::abi::BoundaryCheck>,
    /// The parameter's source span (the AST parameter it lowers from, or the
    /// nearest source construct for a synthetic one). Not shown in the dump.
    pub span: Span,
}

#[derive(Clone)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    /// The block's value, when it has one.
    pub value: Option<Box<Expr>>,
    /// The block's source span: where its scope ends for a "still owed at end
    /// of scope" error. A synthetic block takes the nearest source span.
    /// Not shown in the dump.
    pub span: Span,
}

#[derive(Clone)]
pub enum Stmt {
    /// [ir-loop] unconditional; left by `break`.
    Loop { id: NodeId, body: Block },
    /// [iter-for-native] iteration over an intrinsic container (a list, an
    /// array, a `Str`), which the backends iterate natively; the only loop
    /// form with a subject.
    ForEach { id: NodeId, local: Local, ty: Ty, iterable: Expr, body: Block },
    Let { id: NodeId, local: Local, ty: Ty, value: Expr },
    /// [ir-alias] [deduce-field] `local` names `place`: every read and write
    /// of it is one of the place (a field handle that survives a write to
    /// another field of its root).
    Alias { id: NodeId, local: Local, ty: Ty, place: Place },
    /// [ir-narrow] a new local of the narrowed type, justified by a test.
    Narrow { id: NodeId, local: Local, ty: Ty, from: Place, from_ty: Ty, because: Justification },
    /// [actor-dispatch] The payload of a variant of an enum value, bound to
    /// locals (one per field, in order); valid in the arm that tested the
    /// variant.
    Unpack { id: NodeId, locals: Vec<(Local, Ty)>, from: Place, from_ty: Ty, variant: usize },
    Assign { place: Place, value: Expr },
    Expr(Expr),
    /// The value, and the `return`'s source span (where a flow diagnostic about
    /// leaving here points; not shown in the dump).
    Return(Option<Expr>, Span),
    Break,
    Continue,
    /// [ir-obligation] A directive recording a change to the set of linear
    /// obligations the enclosing fn currently owes. It emits no code — the
    /// backends skip it — but it is the ledger the IR-level linear check
    /// (`ir::check`) reads, and it is dumped so `salvo ir` shows the
    /// reasoning. See IR_CHECKER.md §3 (D3).
    Obligation(Obligation),
}

/// [ir-obligation] One line of the obligation ledger: the four points at
/// which a fn's outstanding-obligation set changes.
#[derive(Clone)]
pub struct Obligation {
    pub kind: ObligationKind,
    /// The obligations this transition gains, releases, or discharges.
    pub refs: Vec<ObligationRef>,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObligationKind {
    /// At the top of a fn body: obligations entering from linear parameters
    /// (concrete linear, or a `canbe linear` generic parameter).
    GainParams,
    /// After a binding whose value (or named arms of it) is linear: the
    /// obligation enters the scope from the new value — including a value
    /// received from a call (its linear arms named).
    Gain,
    /// After a value is moved out: the obligations that are no longer ours.
    Release,
    /// An obligation settled in place, without moving the value (consumed
    /// where it lives, or narrowed to a non-linear arm on every path).
    Discharge,
}

/// [ir-obligation] One obligation named by a directive: the place that holds
/// (or held) the linear value, and — when it is a union — which of its
/// runtime arms [union-arm-identity] carry the obligation. `arms` is `None`
/// for a value that is linear as a whole (a linear leaf such as a `Checked`
/// or a struct), `Some(indices)` for a union some of whose arms are linear.
#[derive(Clone)]
pub struct ObligationRef {
    pub place: Place,
    pub arms: Option<Vec<usize>>,
    /// [linear-group] When the value is a linear struct with linear *fields*,
    /// their names: the container is settled by decomposition once every one
    /// has been moved out ([linear-container]). Empty for a linear leaf or a
    /// union.
    pub container_fields: Vec<String>,
    /// The name of the linear type (a leaf, or a union's first linear arm),
    /// for naming its dischargers in a leak diagnostic. `None` when no single
    /// name applies.
    pub ty_name: Option<String>,
    /// [linear-group] For a union, each linear arm's runtime index paired with
    /// its arm type's display name — so a leak names the arm (`the
    /// `Checked<FsError>` arm`) instead of a bare index. Parallel to `arms`;
    /// empty for a linear leaf.
    pub arm_names: Vec<(usize, String)>,
}

#[derive(Clone, Copy, Debug)]
pub enum Justification {
    /// `Test` node `test` held (in the arm it guards).
    Test { test: NodeId },
    /// Inside arm `arm` of `Switch` `switch`.
    Arm { switch: NodeId, arm: usize },
    /// [safe-nav] Inside the present arm of a `?.`'s `Switch` `switch`: the
    /// receiver tested non-`None`, so the link reads it narrowed. A distinct
    /// justification (rather than a plain `Arm`) is the hint a backend uses to
    /// reconstruct the chain as a native `a?.b` (Kotlin) or an
    /// `.and_then`/`.map` combinator chain (Rust).
    SafeNav { switch: NodeId },
    /// Inside arm `arm` of `Branch` `branch`, whose condition tested it.
    Cond { branch: NodeId, arm: usize },
    /// After `branch`'s arm `arm` left (returned, broke, threw): the test
    /// failed, so the subject is what remains.
    After { branch: NodeId, arm: usize },
    /// [ir-loop] `Loop` `loop_` ended with its value assigned on every path
    /// out (the checker's totality), so the result local is not `None`.
    LoopValue { loop_: NodeId },
    /// [qual-field-override] A qualifier the subject carries refines the
    /// field read: the claim is the proof.
    Claim,
}

/// A storage location: a local and the steps into it.
#[derive(Clone)]
pub struct Place {
    pub root: Local,
    pub steps: Vec<Step>,
}

#[derive(Clone)]
pub enum Step {
    Field(String),
    Tuple(usize),
    Index(Box<Expr>),
}

#[derive(Clone)]
pub struct Expr {
    pub ty: Ty,
    pub span: Span,
    pub kind: ExprKind,
}

#[derive(Clone)]
pub enum ExprKind {
    Int(i64),
    Long(i64),
    Float(f64),
    Double(f64),
    Bool(bool),
    Char(char),
    Str(String),
    /// The unit value: `None` where the slot's type is `None` [type-none-unit].
    Unit,
    /// The absent arm of an optional.
    MakeNone,
    /// [ir-read] a read of a place; `consume` ends the value's life here.
    /// [flow-use-after-move] `moved`: the checker consumed the place's root
    /// wholly here, which the flow pass takes as the move whatever
    /// `consume` says — a fn value handed to a consuming parameter (which
    /// `param_mode` lends), a projection whose move took its root. Flow-only:
    /// no emitter reads it.
    Read { place: Place, consume: bool, moved: bool },
    /// [flow-use-after-move] `moved`: the call consumes its `FnRef::Local`
    /// target (a `once` fn called [once-fn]) — flow-only, like `Read`'s.
    Call { target: FnRef, type_args: Vec<Ty>, args: Vec<Expr>, moved: bool },
    /// A call of an interface member through an instance [effect-dispatch].
    MemberCall { instance: Box<Expr>, member: MemberRef, type_args: Vec<Ty>, args: Vec<Expr> },
    /// [ir-op] the host's operator on basic types [type-basic]: arithmetic,
    /// logic, ordering and equality. An operator on any other type (`Str`
    /// included) is a `Call` of the `cmp`/`eq` it resolved to; an ordering
    /// is then the `Lt`… sign test of that call's `Int` against 0.
    Op { op: Op, args: Vec<Expr> },
    /// A struct or handler instance.
    Construct { fields: Vec<(String, Expr)> },
    /// A value placed into arm `arm` of the union `ty` [union-arm-identity].
    MakeUnion { arm: usize, value: Box<Expr> },
    /// [rewrap] a value moved between two union representations.
    Rewrap { from: Ty, value: Box<Expr> },
    /// [str-drop-mut] a `Mut` value used where the plain type is required.
    DropMut { value: Box<Expr> },
    /// [ir-coerce] a value placed into an optional's present arm (`ty` is
    /// the optional): Rust's `Some`, nothing on Kotlin.
    Present { value: Box<Expr> },
    /// [proj-opt-slot] a union with borrowed arms read as the owned union
    /// `ty`, at a kept position: `arms` pairs each value arm of `ty`, in
    /// order, with the arm of the value that lands there (`None`: no arm
    /// does). The pair says what the arm does — a handle read as its element
    /// (`ref Mut A` → `Mut A`), a borrow read as the value (`proj Mut A` →
    /// `A`), an owned arm lent (`B` → `B`).
    BorrowAs { value: Box<Expr>, arms: Vec<BorrowArm> },
    /// [op-promote] a numeric operand widened within its class to `ty`
    /// (`Int` to `Long`, `Float` to `Double`).
    Widen { value: Box<Expr> },
    Tuple(Vec<Expr>),
    /// A list literal, `ty` says `List<T>` or `Mut List<T>`.
    List(Vec<Expr>),
    /// An array literal; an element may be a `Spread` of another array
    /// [fn-variadic].
    Array(Vec<Expr>),
    /// [fn-variadic] `...xs` among an array literal's elements: the array's
    /// elements, in place. Only inside `Array`.
    Spread { value: Box<Expr> },
    /// String concatenation; every part is a `Str` [interp-to-str].
    Concat(Vec<Expr>),
    /// [ir-branch] subjectless: ordered conditions.
    Branch { id: NodeId, arms: Vec<(Expr, Block)>, otherwise: Option<Block> },
    /// [ir-switch] on a subject: one arm per test.
    Switch { id: NodeId, subject: Box<Expr>, arms: Vec<SwitchArm> },
    /// [ir-test] `subject is …` as a condition: `Bool`. Narrowing inside the
    /// branch it guards is a `Narrow` justified by that branch's arm; a
    /// conjunct after a binding test reads the binding inside a `Branch` on
    /// the test [is-bind-once].
    Test { id: NodeId, subject: Box<Expr>, test: ArmTest },
    Lambda { params: Vec<Param>, ret: Ty, body: Block, captures: Vec<Capture> },
    /// A fn used as a value.
    FnValue(FnRef),
    Try { body: Block },
    Throw { message: Box<Expr> },
    /// A conjunction of predicate-qualifier checks on a subject
    /// [qual-predicate]: calls to `qualifies` fns.
    /// [assert-trap] `at` is the Salvo location (`module:line:col`) the
    /// trap names; a written `message` replaces the default text.
    Assert { cond: Box<Expr>, message: Option<Box<Expr>>, at: String },
    /// [assert-trap] A point the checker proved unreachable (`x!` on `None`,
    /// a `when` with no arm left): a trap naming `at`, with `message` as the
    /// text (`value is absent`).
    Unreachable { message: Option<Box<Expr>>, at: String },
    // ---- actors (IR record §6; host primitives until the actor phase) ----
    Spawn { handler: Box<Expr>, deps: Vec<Expr>, pool: Option<Box<Expr>>, join: Option<Box<Expr>>, effects: Vec<Ty> },
    /// A send to an actor through an addr.
    Send { addr: Box<Expr>, member: MemberRef, args: Vec<Expr> },
    /// [actor-replyto] a reply token that delivers to a member of the
    /// enclosing handler, or to a free `send fn` run as a task, with the
    /// captures as its leading arguments and the answer as its last.
    ReplyTo { target: ReplyTarget, captures: Vec<Expr>, gated: bool, pool: Option<Box<Expr>> },
    /// [actor-waitfor] bind a fresh reply token, run the body, wait.
    WaitFor { local: Local, token_ty: Ty, body: Block },
    /// The enclosing actor's own address.
    SelfAddr,
    /// [effect-handle] A handler instance bound as an effect's instance:
    /// the shared handle, entered one member at a time, which is how every
    /// `use` binds (Kotlin `__Mon_E`, Rust `__Handle_E`). `ty` is the face.
    Handle { instance: Box<Expr> },
    /// [actor-use-addr] The instance of the effect `ty` behind an address:
    /// every member a send. `use addr` and a `with addr` item.
    AddrInstance { addr: Box<Expr> },
    /// [actor-self-send] a message to the enclosing actor.
    SelfSend { member: String, args: Vec<Expr> },
    /// [actor-dispatch] A call of a member of a handler *instance* (not
    /// through a handle): a face's member, or one of the handler's own
    /// (`init`, a private `send fn`).
    HandlerCall { instance: Box<Expr>, member: HandlerMember, args: Vec<Expr> },
    /// [interp-union] The text of a union value: the function of the arm it
    /// holds is called on its payload. `arms[i]` is a value of type
    /// `(arm i) -> Str`; the backend owns the dispatch (`UnionN`'s `to_str`).
    UnionToStr { value: Box<Expr>, arms: Vec<Expr> },
    /// A construct the builder does not lower yet: never emitted silently.
    Unsupported(String),
}

/// What a [`ExprKind::HandlerCall`] calls.
#[derive(Clone, Debug, PartialEq)]
pub enum HandlerMember {
    /// A member of an interface the handler implements.
    Face(MemberRef),
    /// A member of the handler no face declares, by name.
    Own(String),
}

#[derive(Clone)]
pub enum ReplyTarget {
    /// A `send` member of the enclosing handler, by name.
    Member(String),
    /// [task-mint] a free `send fn`, with the effect instances it inherits.
    Task { target: DeclId, effects: Vec<Expr> },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    And,
    Or,
    Not,
    Neg,
    Lt,
    Gt,
    LtEq,
    GtEq,
    Eq,
    NotEq,
}

/// [proj-opt-slot] One arm of a [`ExprKind::BorrowAs`]: what the value's arm
/// `from` is read as, `to`.
#[derive(Clone, Debug, PartialEq)]
pub struct BorrowArm {
    pub from: Option<Ty>,
    pub to: Ty,
}

#[derive(Clone)]
pub struct SwitchArm {
    pub test: ArmTest,
    pub body: Block,
}

#[derive(Clone, Debug)]
pub enum ArmTest {
    /// The subject is in union arm `arm` (runtime index).
    Arm(usize),
    /// The subject is one of several arms.
    Arms(Vec<usize>),
    /// The subject is `None`.
    None,
    /// [type-literal] The subject is in one of these arms, each possibly
    /// under a value condition (literals collapsed into a base's arm).
    Lit(Vec<LitArm>),
    Else,
}

/// One arm of a literal test: the runtime arm, and the values on it the
/// test accepts (`negate`: refuses); no values means the whole arm.
#[derive(Clone, Debug, PartialEq)]
pub struct LitArm {
    pub arm: usize,
    pub negate: bool,
    pub lits: Vec<Lit>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Lit {
    Str(String),
    Int(i64),
    Long(i64),
    Bool(bool),
}

#[derive(Clone)]
pub struct Capture {
    pub local: Local,
    pub consumed: bool,
    pub mutable: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum FnRef {
    /// Any top-level fn declaration, intrinsic or not.
    Decl(DeclId),
    /// A fn-typed local (a parameter, a let, an implicit).
    Local(Local),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct MemberRef {
    pub interface: DeclId,
    pub index: usize,
}

/// What the builder could not lower, per module.
#[derive(Default)]
pub struct Diagnostics {
    pub errors: Vec<String>,
}

