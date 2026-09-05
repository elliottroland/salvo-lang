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
    Fn(FnDecl),
    DefineFn(DefineFn),
    DefineType(DefineType),
    DefineHandler(DefineHandler),
}

/// Visibility/backing modifier on declarations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackingMod {
    /// `intrinsic`: declared by the standard library and implemented inside
    /// the compiler — every backend must lower every one of them
    /// [backend-intrinsic] [intrinsic-fn].
    Intrinsic,
    /// `external`: implemented via `define` templates in backend files.
    External,
}

/// `import path.to.item (as alias)?`
#[derive(Clone, Debug, PartialEq)]
pub struct ImportDecl {
    pub path: Vec<Ident>,
    pub alias: Option<Ident>,
    pub span: Span,
}

/// `intrinsic type Str`, `external type List<T> canbe Mut`, or a type alias
/// `type Result<S, T> = Ok S | Err T`.
#[derive(Clone, Debug, PartialEq)]
pub struct TypeDecl {
    /// The `//` comment block directly above the declaration, one entry
    /// per line, `//` and one leading space stripped [doc-comment].
    pub docs: Vec<String>,
    pub backing: Option<BackingMod>,
    pub name: Ident,
    pub generics: Vec<Ident>,
    /// Auto-qualifiers, e.g. `canbe Mut` [type-canbe-mut]: the type opts
    /// into the language-level `Mut` qualifier (like `struct ... canbe
    /// Mut` [struct-mut]).
    pub auto_qualifiers: Vec<TypeRef>,
    pub alias: Option<Type>,
    pub span: Span,
}

/// `struct Person canbe Mut { name: Str, ... }`
#[derive(Clone, Debug, PartialEq)]
pub struct StructDecl {
    /// The `//` comment block directly above the declaration, one entry
    /// per line, `//` and one leading space stripped [doc-comment].
    pub docs: Vec<String>,
    pub name: Ident,
    pub generics: Vec<Ident>,
    /// Auto-qualifiers, e.g. `canbe Mut`.
    pub auto_qualifiers: Vec<TypeRef>,
    pub fields: Vec<FieldDecl>,
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
    pub backing: Option<BackingMod>,
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
    /// A dedicated flag rather than a [`BackingMod`], because `platform`
    /// applies to nothing but an effect and `intrinsic`/`external` never
    /// apply to one: the two sets are disjoint, so keeping them apart makes
    /// the invariant structural.
    pub platform: bool,
    pub name: Ident,
    pub generics: Vec<Ident>,
    pub fns: Vec<FnDecl>,
    pub span: Span,
}

/// `handler CyclicRandom<T>(values: T[]) of Random<T> { state fns }`
/// or `external handler StdOutConsole of Console`.
#[derive(Clone, Debug, PartialEq)]
pub struct HandlerDecl {
    /// The `//` comment block directly above the declaration, one entry
    /// per line, `//` and one leading space stripped [doc-comment].
    pub docs: Vec<String>,
    pub backing: Option<BackingMod>,
    pub name: Ident,
    pub generics: Vec<Ident>,
    /// Constructor parameters, e.g. `(values: T[])`.
    pub params: Vec<Param>,
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
    pub backing: Option<BackingMod>,
    pub name: Ident,
    pub generics: Vec<Ident>,
    /// Per-type-parameter opt-ins: `<T canbe Linear>` [linear-generics].
    pub generic_canbe: Vec<(Ident, TypeRef)>,
    /// `-> ReadOnly[from: param] T`: the returned value is derived from
    /// (borrows) the named kept parameter [readonly-return].
    pub derived_return: Option<Ident>,
    pub params: Vec<Param>,
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
    /// `None` for signatures (`external fn`, effect members).
    pub body: Option<Block>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Param {
    pub name: Ident,
    pub ty: Type,
    /// True for `...args: T[]`.
    pub variadic: bool,
    pub span: Span,
}

/// An entry in a function's effect list: `[Random<Int>, Console, use]`.
#[derive(Clone, Debug, PartialEq)]
pub enum EffectRef {
    /// The special `use` effect, allowing handler registration.
    Use(Span),
    /// A named effect, possibly generic: `Random<Int>`.
    Effect(TypeRef),
}

/// An entry in a function's deduction list [deduce-syntax]:
/// `[person]` (bare — every qualifier the argument has survives),
/// `[list: Mut]` (*exhaustive* — afterwards only `Mut` applies),
/// `[list:]` (exhaustive and empty — every qualifier stripped),
/// `[list: -NonEmpty]` (*delta* — drop `NonEmpty`, keep the rest), and
/// `[list: Nothing]` (moved, like omitting the entry).
#[derive(Clone, Debug, PartialEq)]
pub struct Deduction {
    pub param: Ident,
    pub kind: DeductionKind,
    pub span: Span,
}

/// The polarity of one deduction entry [deduce-syntax]. A written entry is
/// either exhaustive (plain qualifier names) or a delta (`-`-prefixed
/// names); mixing them in one entry is an error.
#[derive(Clone, Debug, PartialEq)]
pub enum DeductionKind {
    /// Bare `[list]`: the parameter is kept and *nothing* is stripped.
    KeepAll,
    /// `[list: A B]` / `[list:]`: afterwards exactly these apply.
    Exhaustive(Vec<TypeRef>),
    /// `[list: -A -B]`: these are dropped, everything else survives.
    Remove(Vec<TypeRef>),
    /// `[list: Nothing]`: moved (the caller loses access).
    Moved,
}

// --- define templates (backend files) ---

/// `define fn chars(str: Str) -> Char[] { imports: `` ... `` inline: `` ... `` }`
#[derive(Clone, Debug, PartialEq)]
pub struct DefineFn {
    pub sig: FnDecl,
    pub body: DefineBody,
    pub span: Span,
}

/// `define type LinkedList<T> { inline: `` ... `` }`
#[derive(Clone, Debug, PartialEq)]
pub struct DefineType {
    pub name: Ident,
    pub generics: Vec<Ident>,
    pub body: DefineBody,
    pub span: Span,
}

/// `define handler StdOutConsole of Console { define fn ... }`
#[derive(Clone, Debug, PartialEq)]
pub struct DefineHandler {
    pub name: Ident,
    pub generics: Vec<Ident>,
    pub of: Type,
    pub fns: Vec<DefineFn>,
    pub span: Span,
}

/// The `imports:`/`inline:` sections of a `define` block.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct DefineBody {
    pub imports: Option<Template>,
    pub inline: Option<Template>,
    /// `Mut inline:` — the template used instead of `inline:` when the
    /// type is qualified with `Mut` [type-canbe-mut] (e.g. Kotlin maps
    /// `Mut List<T>` to `MutableList<T>`). Only meaningful on
    /// `define type` for types declared `canbe Mut`.
    pub mut_inline: Option<Template>,
}

/// A backtick template, split into literal text and `${...}` interpolations.
#[derive(Clone, Debug, PartialEq)]
pub struct Template {
    pub parts: Vec<TemplatePart>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TemplatePart {
    Text(String),
    /// `${name}` — a parameter or generic to interpolate.
    Interp(Ident),
    /// `${...name}` — a variadic parameter spliced as comma-separated values.
    InterpVariadic(Ident),
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
    pub span: Span,
}

// --- Source-like rendering (hover, diagnostics) ---

impl fmt::Display for TypeRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name.name)?;
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
    /// `yield expr`
    Yield { value: Expr, span: Span },
    /// `use HandlerExpr(...)` — register a handler for the current context.
    Use { handler: Expr, span: Span },
    /// `defer { ... }` — run the block when the enclosing block ends
    /// [defer]. Its meaning is *splice at exit*: the body runs at every
    /// exit of the enclosing block (the end of the block, and each
    /// `return`/`break`/`continue` that leaves it), latest `defer` first.
    Defer { body: Block, span: Span },    /// A bare expression statement.
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
    Call {
        callee: Box<Expr>,
        /// Explicit generic args: `next_random<Int>()`.
        type_args: Vec<Type>,
        args: Vec<Arg>,
        span: Span,
    },
    /// `expr[index]`
    Index {
        base: Box<Expr>,
        index: Box<Expr>,
        span: Span,
    },
    /// `[1, 2, 3]`
    ArrayLit { elems: Vec<Expr>, span: Span },
    /// `Int[5] { i: Int -> 0 }` — sized array construction with an
    /// element-initializer lambda.
    ArrayInit {
        elem_type: TypeRef,
        size: Box<Expr>,
        init: Box<Expr>,
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
    PostIncrement { operand: Box<Expr>, span: Span },
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
            | Expr::Call { span, .. }
            | Expr::Index { span, .. }
            | Expr::ArrayLit { span, .. }
            | Expr::ArrayInit { span, .. }
            | Expr::Tuple { span, .. }
            | Expr::StructLit { span, .. }
            | Expr::Unary { span, .. }
            | Expr::Binary { span, .. }
            | Expr::Is { span, .. }
            | Expr::Widen { span, .. }
            | Expr::NonNull { span, .. }
            | Expr::PostIncrement { span, .. }
            | Expr::If { span, .. }
            | Expr::When { span, .. }
            | Expr::WhenCond { span, .. }
            | Expr::While { span, .. }
            | Expr::For { span, .. }
            | Expr::Lambda { span, .. }
            | Expr::Try { span, .. }
            | Expr::Spread { span, .. }
            | Expr::Error { span } => *span,
            Expr::Ident(ident) => ident.span,
        }
    }
}
