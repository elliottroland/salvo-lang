//! The IR node set [ir-nodes]: a minimal, decided form of a Salvo program
//! (IR.md §3). Every reference is absolute, every expression carries its
//! type, every narrowing is a binding, and nothing is left for a backend to
//! infer. What a backend adds is representation and idiom (IR.md §1).


pub use salvo_core::param_mode::PassMode;
pub use salvo_core::types::Ty;
pub use salvo_core::ModulePath;
pub use salvo_syntax::Span;

/// The identity of a declaration: its module and its position among the
/// module's items. Unique program-wide; overloads are distinct items.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DeclId {
    pub module: ModulePath,
    pub item: usize,
    /// 0 for the item itself; `i + 1` for the `i`th fn declared *inside* the
    /// item (a qualifier's `qualifies`).
    pub sub: u16,
}

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
}

pub struct Module {
    pub path: ModulePath,
    /// The file this module came from (diagnostics, trap locations).
    pub file_name: String,
    pub decls: Vec<Decl>,
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
    pub factories: Option<salvo_core::abi::Factories>,
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
    pub result_check: Option<salvo_core::abi::BoundaryCheck>,
    /// [platform-factory] the factories of the result and of the `Reply<T>`
    /// payloads a host builds, when a platform handler implements the effect.
    pub factories: Vec<salvo_core::abi::Factories>,
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
    pub span: Span,
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
    /// [canbe-entry] the parameters that may name the same object, as the
    /// clause wrote them (`|` lists desugared to one entry per pair or
    /// subject): symmetric, not transitive. The fourth ownership mark
    /// (IR.md §5).
    pub may_alias: Vec<MayAlias>,
    /// [deduce-field] `p.f: proj(q)`: after the call, parameter `.0` holds a
    /// view of parameter `.1` (indices into `params`).
    pub holds: Vec<(usize, usize)>,
    /// [throw] the message type this fn may throw, when it declares `[Throw<M>]`.
    pub throws: Option<Ty>,
    /// [platform-check] a platform fn's result check.
    pub result_check: Option<salvo_core::abi::BoundaryCheck>,
    /// [platform-factory] a platform fn's result factories.
    pub factories: Option<salvo_core::abi::Factories>,
    pub body: Option<Block>,
    pub span: Span,
}

/// [canbe-entry] One alias relation of a fn's parameters; indices are into
/// `FnDecl.params`.
#[derive(Clone, Debug, PartialEq)]
pub enum MayAlias {
    /// `a canbe d`.
    Params(usize, usize),
    /// `a canbe in lib.tracks`: `param` may be an element of the container
    /// reached from parameter `root` by `path`; two parameters anchored at
    /// the same container may therefore coincide.
    In { param: usize, root: usize, path: Vec<AnchorStep> },
}

/// One step of a `canbe in` anchor path. Fields and tuple elements only: an
/// anchor is part of a signature, so it has no index expression to name.
#[derive(Clone, Debug, PartialEq)]
pub enum AnchorStep {
    Field(String),
    Tuple(usize),
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
    pub check: Option<salvo_core::abi::BoundaryCheck>,
}

#[derive(Clone)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    /// The block's value, when it has one.
    pub value: Option<Box<Expr>>,
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
    Assign { place: Place, value: Expr },
    Expr(Expr),
    Return(Option<Expr>),
    Break,
    Continue,
}

#[derive(Clone, Copy, Debug)]
pub enum Justification {
    /// `Test` node `test` held (in the arm it guards).
    Test { test: NodeId },
    /// Inside arm `arm` of `Switch` `switch`.
    Arm { switch: NodeId, arm: usize },
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
    Read { place: Place, consume: bool },
    Call { target: FnRef, type_args: Vec<Ty>, args: Vec<Expr> },
    /// A call of an interface member through an instance [effect-dispatch].
    MemberCall { instance: Box<Expr>, member: MemberRef, type_args: Vec<Ty>, args: Vec<Expr> },
    /// [ir-op] an operator on scalars the language defines as primitive and
    /// std declares no fn for (`+`, `-`, `*`, `/`, `%`, `and`, `or`, `!`,
    /// unary `-`), and the sign test of an ordering (`Lt`… over the `Int` a
    /// `cmp` call answered, against 0). Comparisons themselves are calls to
    /// `cmp`/`eq`.
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
    // ---- actors (IR.md §6; host primitives until the actor phase) ----
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
    /// A construct the builder does not lower yet: never emitted silently.
    Unsupported(String),
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

