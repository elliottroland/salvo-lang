# IR design: a minimal, decided form of a Salvo program

Design document, written 2026-10-06 after the TOUR.md conversation. Its
decisions were made by the user the same day (§9 records them); the rest is the
design those decisions fix. When the work lands, this file retires into COMPLETED.md
(design record), LANGUAGE_SPEC.md (rules) and ROADMAP.md (what is left), per
AGENTS.md.

Contents

1. Principles
2. What the IR builder absorbs (construct → node)
3. The node set
4. References
5. Ownership marks
6. Effects, handlers and actors
7. Generated-by-type code
8. The text form, with the TOUR.md program dumped
9. Open decisions
10. Transition plan

---

## 1. Principles

**The IR is where guessing stops.** Every question the checker can answer is
answered in the tree, as a field of the node that needs it. No side table keyed
by span survives past the IR builder. An emitter that reaches a node it cannot
render reports an error naming the node; it has no "plain output" fallback
([backend-never-wrong]).

**The IR states Salvo facts; a backend reconstructs its idiom** ([core-layers]).
The IR says "branch on subject `x`: arm `None` → …, else bind `v` → …";
Kotlin recognizes the shape and writes `x ?: …`. The IR says "this read
consumes the value"; Rust writes a move and Kotlin writes nothing. The IR never
says `clone`, `run {}`, `&mut`, `import`, `?:`.

**Narrowing is a binding.** A successful test introduces a *new* local of the
narrowed type, justified by the test (`narrow#1[check#1] x2: Str = x`). A
backend may materialize it (Java: a cast after `instanceof`), alias it
(Kotlin: smart cast, the name is reused), or unwrap it (Rust: `x.u1()`). The
checker's `repr_ty`/`expr_ty` split, the binding-versus-non-binding `is`, and
escape analysis all become this one thing: a binding with a reason.

**Effects are values.** An effect instance is a parameter like any other; a
member call is a call through that parameter. The effect *declaration* stays
(a backend emits an interface for it), the handler declaration stays (a
class), but nothing about *which* instance or *which* member overload is left
to find.

**Printable.** `salvo ir` dumps a module. The dump is the checker's golden
test, independent of either backend, and the bisect point when the backends
disagree.

**Generics stay.** Both targets have generics; monomorphizing would multiply
output and lose the user's structure. A generic fn is one IR declaration with
type parameters, and a call carries its type arguments explicitly.

## 2. What the IR builder absorbs

Each Salvo construct, and the IR it becomes. "Decided" means the builder
reads the checker's answer and writes it into the node.

| Salvo | IR | decided by |
|---|---|---|
| `if`/`elif`/`else`, `when` without subject | `Branch` (subjectless): ordered `(condition, block)` arms, optional else | — |
| `when x { is A … }`, `x is A`, `x ?: e`, `x?.f`, `x!` | `Switch` (subject): arms over the subject's arms | `expr_ty`, `repr_ty`, union arm identity |
| `x is Idx(xs)`, predicate qualifier tests | a `Call` to the `qualifies` fn, used as a condition | `predicate_tests` |
| narrowing (`x` after `is`) | `Narrow` binding, justified by a `Switch` arm or a `Branch` condition | `repr_ty`, flow |
| `for x in e { } else { }` | the mint (`iter(e)` when needed), then a `Loop` whose body begins with `let __step = next(..)` and a `Switch` on `Emitted \| Finished` that breaks on `Finished` | `for_drivers`, `resolved_fn_keys` |
| `while c { } else { }` | `Loop { body }` whose body begins `branch if not c { break }`; the value, the `else` and the ran-flag as explicit locals and a `Branch` after the loop, each only when present | `while-value` join |
| `while x is T (n) { }` | `Loop` whose body begins with a `let` of the subject (evaluated once per iteration) and a `Switch` that breaks on the failing arm; the binding is a `Narrow` | `is-bind-once` |
| `f(x)`, `x.f(a)`, `f@m(x)`, `cmp@Person(a, b)` (the `cmp` declared in `Person`'s body or by its `: Ordered<self>` obligation) | `Call { target: Ref, type_args, args }` | `call_fn`, `dot_calls`, `call_type_args` |
| `println(x)` (effect member) | `MemberCall { instance: Expr, member: MemberRef, args }` | `member_calls`, `effect_calls` |
| `a < b`, `a == b`, `a != b` | `Call` to the resolved `cmp`/`eq`, **always**: on `Int` that is `core.compare::cmp(Int, Int)`, a declaration marked `intrinsic`, from which the backend reconstructs `<` | `comparisons` (`CompareVia::Call`, or `Implicit` for a forwarded `?cmp`) |
| `a + b`, `a and b`, `!x`, `-x` on scalars | `Call` to the operator's fn declaration (`core.basic::add(Int, Int)`, intrinsic), the same way | `expr_ty` |
| `"${a}/${n}"` | `Concat([…])` of `Str`-typed expressions: a literal, or a `Call` to the resolved `to_str` for each `${…}` | `interp_to_str`, `interp_implicit` |
| `?cmp`, `?hash`, `?to_str` at a call | ordinary trailing arguments: a `Ref` to a fn, a `Lambda` adapter, or a forwarded parameter | `implicit_args` (recursive) |
| implicit parameters on a declaration | ordinary trailing parameters | `implicit_params` |
| effect list `[Console, Fs]` on a fn | leading parameters of interface type | `fn_effects`, `call_effects` |
| `use H(args)` | `Let` of a `Construct` (handler) + the instance binding | `use_effects`, `use_deps` |
| a value placed into a union arm | `MakeUnion { ty, arm, value }` | `Coercion::WrapUnion` |
| `None` as unit vs absent | `Unit` vs `MakeNone { ty }` | `Coercion::NoneUnit`, `WrapOption` |
| `copy(x)` | `Call` to `core.basic::copy` with the type's plan attached | `copyplan` |
| comptime, `by auto`, stamps | ordinary declarations (already expanded before checking) | `comptime` |
| `iter fn` | ordinary struct + minter + `next` (already desugared in the syntax crate) | `desugar` |
| qualifiers on types (`Mut`, `NonEmpty`, `Idx(xs)`) | erased, except `Mut` where a backend needs it (kept as a type flag) | `erase` |
| deductions (`=> !p`, `=> p: Mut`) | parameter annotations `moved \| lent \| lent_mut`, result annotation `borrows [p…]` | `param_mode`, `lends` |
| `try { … }`, `throw(m)` | `Try { body }`, `Throw { message }` | `throw_sites` |
| `spawn H(…)`, `send`, `replyto`, `waitfor` | see §6 | actor tables |
| `assert`, `unreachable` | `Assert`, `Unreachable` | — |
| `import`, `export`, `rename` | nothing: references are absolute, visibility is checked | `resolve` |

Absent from the IR: `Scoped`, `EffectScoped`, `SafeField`, `Placeholder`,
`Elvis`, `WhenCond`, `Widen`, `Is`, `IncDec`, `Spread` (a variadic call's
tail is a list argument), `Rename`, `Comp`, `Import`, `Refn`, `Params`, `Test`
(a test is a fn the harness calls), and every qualifier type.

## 3. The node set

A sketch of the Rust types; names are provisional.

```rust
pub struct Module { pub path: ModulePath, pub decls: Vec<Decl> }

pub enum Decl {
    Struct(StructDecl),        // fields, type params, `opaque`, `linear`, `canbe Mut`
    Union(UnionDecl),          // a named union type: arms in identity order
    Interface(InterfaceDecl),  // an effect: members (signatures), `actor`, `platform`
    Impl(ImplDecl),            // a handler: ctor params, state, deps, member bodies, faces
    Fn(FnDecl),
    PlatformType(…),           // a host type, no body
}

pub struct FnDecl {
    pub id: DeclId,
    pub type_params: Vec<TypeParam>,         // with their bounds, if any
    pub params: Vec<Param>,                  // effects first, then declared, then implicits
    pub ret: Ty,
    pub borrows: Vec<usize>,                 // params the result holds a view of
    pub body: Option<Block>,                 // None: platform or intrinsic
    pub kind: FnKind,                        // Plain | Intrinsic | Platform | Member(…)
}
pub struct Param { pub name: Local, pub ty: Ty, pub mode: PassMode /* Moved | Lent | LentMut */ }

pub struct Block { pub stmts: Vec<Stmt>, pub value: Option<Expr> }

pub enum Stmt {
    Loop { body: Block },                                  // unconditional; left by `break`
    Let { local: Local, ty: Ty, value: Expr },
    Narrow { local: Local, ty: Ty, from: Local, because: Justification },
    Assign { place: Place, value: Expr },
    Expr(Expr),
    Return(Option<Expr>),
    Break(Option<Expr>), Continue,
}

pub enum Justification { Arm { switch: NodeId, arm: usize }, Cond { branch: NodeId, arm: usize } }

pub enum Expr {
    // every variant carries `ty: Ty` and `span: Span`
    Lit(Lit),
    Read { place: Place, consume: bool },                 // §5
    Call { target: FnRef, type_args: Vec<Ty>, args: Vec<Expr> },
    MemberCall { instance: Box<Expr>, member: MemberRef, args: Vec<Expr> },
    Construct { ty: Ty, fields: Vec<(Field, Expr)> },     // struct literal, handler ctor
    MakeUnion { arm: usize, value: Box<Expr> },           // ty is the union
    MakeNone, Unit,
    Tuple(Vec<Expr>), List(Vec<Expr>), Array(Vec<Expr>),
    Concat(Vec<Expr>),                                    // every part is a Str
    Branch { arms: Vec<(Expr, Block)>, otherwise: Option<Block> },
    Switch { subject: Box<Expr>, arms: Vec<SwitchArm> },  // ty is the join of the arm values
    Lambda { params: Vec<Param>, body: Block, captures: Vec<Local> },
    FnValue(FnRef),                                       // a fn used as a value
    Try { body: Block }, Throw { message: Box<Expr> },
    Assert { cond: Box<Expr>, message: Option<String> }, Unreachable,
    Spawn(…), Send(…), Reply(…), WaitFor(…),             // §6
}

pub struct SwitchArm { pub test: ArmTest, pub body: Block }
pub enum ArmTest { Arm(usize), None, Lit(Lit), Else }
```

Notes on the choices:

* **`Block.value`.** A block in value position ends in an expression. Both
  branch forms are expressions; a backend without expression-`if` lowers to a
  result variable (a reconstruction, in `salvo-backend`).
* **One loop** (user decision 2026-10-06). `Loop { body }` is a statement, has
  no condition and no value, and is left by `break`. `while c` is a loop
  whose body begins `branch if not c { break }`; `for` and `while x is T`
  are loops whose body begins with a `let` of the step or subject and a
  `switch` that breaks on one arm. A loop's value, its `else` and the
  "ran" flag become explicit locals and a `branch` after the loop, **each only
  when the program has it** (a value-position loop, a written `else`).
  Backends recognize the leading shape to print `while`, `while let`,
  `loop { break v }`; the dump stays readable because the first statement
  says which kind of loop it is. A condition slot was considered and rejected:
  it would give the IR two spellings of "loop until" for the one
  reconstruction it saved.
* **`Place`** is a local followed by field/index steps, the same as core's
  `place.rs`. A read of a place is the only way to use a variable.
* **`Narrow`** is a statement so that the justification is visible where the
  binding is introduced, and so a backend that aliases (Kotlin) can drop the
  statement and rename uses.
* **Types** are the checker's `Ty`, minus qualifiers, with one flag kept:
  `Mut` on a type a backend represents differently (Kotlin's `Mut Str` is a
  `StringBuilder`). Every `Let`, `Param`, `Expr` carries its type.
* **Spans** are kept on nodes for the one thing generated code says about the
  source: trap locations (`"salvo: value is absent at main.sv:12"`) and the
  LSP's mapping. Nothing is looked *up* by span.

## 4. References

A reference is absolute and never a name alone.

```rust
pub struct DeclId { pub module: ModulePath, pub item: usize }   // stable per build
pub enum FnRef {
    Decl(DeclId),                                 // any top-level fn, intrinsic or not
    Member { interface: DeclId, index: usize },   // only via MemberCall
    Local(Local),                                 // a fn-typed parameter or let
}
pub struct MemberRef { pub interface: DeclId, pub index: usize }
```

Text form: `module::name(param types)`, with the parameter types written only
when the module overloads the name: `core.compare::cmp(Int, Int)`,
`core.list::add(Mut List<T>, T)`, `main::bumped`. That is readable and stable
across runs; the `DeclId` behind it is the module and item index.

An intrinsic is not a kind of reference. `core.compare::cmp(Int, Int)` points
at a declaration that happens to say `intrinsic`; the backend looks the
declaration up, sees the flag, and lowers it from its own table
(`intrinsics.rs`, keyed as now by name and receiver type) instead of calling
it. The same goes for `platform`: the reference is ordinary, the declaration
says how it is provided.

What this buys: **no imports in the IR**. A backend walks a module's
references, collects the foreign declarations, and writes whatever its target
needs (Rust `use`, Kotlin `import`, Java fully-qualified names since it has
no aliasing). Name clashes are the backend's to resolve: `naming.rs`
(`FnNames`, the overload mangling) becomes a helper in `salvo-backend` that
turns a `FnRef` into a target identifier, shared by Rust and Kotlin.

## 5. Ownership marks

Three facts, all Salvo's, none a target's:

1. **Per parameter**, `PassMode`: `Moved`, `Lent`, `LentMut` (today's
   `param_mode.rs`, unchanged).
2. **Per fn result**, `borrows: Vec<usize>`: the parameters the result holds a
   view of (`proj`; today's `lends.rs`).
3. **Per read**, `consume: bool`: this read ends the value's life here. True
   for a `Moved` argument's place, a `return` of a local, a linear value
   handed over, a `state` field taken, a projection moved out of a consumed
   root — the union of `linear_moves`, `state_takes`, `moved_projections`,
   move-mode `binding_modes` and the argument/return positions whose mode
   says so.

The principle: **the IR marks consumption, never duplication.** A backend
with ownership (Rust) reads `consume: false` on a non-scalar read whose root
it holds as a borrow and writes `.clone()`; one without (Kotlin) ignores all
three. Whether the *root* is a borrow in the output is the backend's own
state (today's `BindKind`), and stays there.

Evaluation order is left to right and is the IR's statement/argument order.
The constraint "a read before a mutable lend of the same place sees the old
value" is implied by that order; Rust's hoist is a reconstruction of it, in
`salvo-backend`, from the modes (`LentMut` argument after a read of the same
place).

## 6. Effects, handlers and actors

**Effects as values.** A fn declaring `[Console]` has a leading parameter
`console: Console` of interface type. A call to it passes the instance the
checker resolved (`call_effects`). A member call is `MemberCall { instance,
member, args }`. `use StdOutConsole()` is a `Let` of a `Construct` whose type
is the handler, and the instance in scope for the rest of the block.

**Declarations kept.** `Interface` (the effect): its members' signatures, with
modes; whether it is `actor`; its prerequisites as leading parameters of each
member. `Impl` (the handler): constructor parameters, state fields, the
handlers it depends on (as constructor-time parameters), member bodies, and
the faces it implements. Platform handlers and platform fns are declarations
with no body and a `platform` flag; the backend binds them to host code
exactly as now.

**Actors.** The scheduler is Salvo (`std/runtime`), so `spawn`, `send`,
`replyto` and `waitfor` are mostly calls into it. What each emitter generates
today, per actor effect, is three declarations: the message union (one arm
per `send` member, carrying its arguments), the dispatch fn (decode a message,
call the member, answer the reply), and the wire codec with its protocol hash.
Proposal: **the IR builder generates these as ordinary IR declarations** (a
`Union`, a `Fn` with a `Switch` over the message, a `Fn` that encodes), so the
backends render structs, unions and fns and nothing actor-specific. `Spawn`,
`Send`, `Reply`, `WaitFor` stay as IR nodes only where the target needs a
primitive the std scheduler cannot express in Salvo (the host thread handoff);
the aim is for that list to be short, and it is the part of this design with
the least evidence behind it. Scheduled as its own phase (§10).

## 7. Generated-by-type code

Code derived from a type, not written by the user:

| what | today | proposal |
|---|---|---|
| wire codecs (`__enc`/`__dec`, `__Codec_S`) | each emitter | IR fns generated per type with a wire form |
| `copy` of a struct | Rust `clone`, Kotlin renders `CopyPlan` | an IR fn per struct that needs one, from `copyplan`; a scalar/identity copy is the read itself |
| `to_str`/`eq`/`cmp`/`hash` `by auto` | stamped Salvo fns (already) | unchanged |
| the `UnionN` wrapper types | each emitter | **backend**: it is a representation (Rust enum, Kotlin sealed class, Java could use records) |
| optional (`T?`) representation | each emitter | **backend** |
| `Display` for Rust's `format!` | Rust | gone: `Concat` names the `to_str` |

Rule: if the generated code is *behaviour* (what bytes, which fields are
copied), the builder writes it as IR; if it is *representation* (how a union
is laid out), the backend owns it.

## 8. The text form, with the TOUR.md program

The dump is for inspection and golden tests, not for parsing back. One
declaration per paragraph; nodes that matter for reconstruction carry an id
(`bind#`, `check#`, `branch#`, `switch#`, `narrow#`) so a justification can
name its reason. Types after `:`; modes before a parameter; `!` marks a
consuming read.

The TOUR.md sample (`tmp/tour/main.sv`), as the builder would dump it:

```
module main

struct Box canbe Mut { n: Int, tag: Str }

fn main::bumped(lent_mut b: Box) -> Int {
  assign b.n = call core.basic::add(Int, Int)(read b.n, 1)
  return read b.n
}

fn main::label(lent s: Str, lent n: Int) -> Str {
  return concat(call core.basic::to_str(Str)(read s), "/", call core.basic::to_str(Int)(read n))
}

fn main::consume(moved items: List<Int>) -> Int {
  return call core.list::size(read items)                 // size is `platform fn`; lent
}

struct __Iter_countdown_Int canbe Mut { from: Int, at: Int }

fn main::countdown(lent from: Int) -> __Iter_countdown_Int {
  return construct __Iter_countdown_Int { from: read from, at: read from }
}

fn main::next#1(lent_mut __p: __Iter_countdown_Int) -> Emitted Int | Finished {
  branch#1 if call core.basic::le(Int, Int)(call core.compare::cmp(Int, Int)(read __p.at, 0), 0) {
    return make_union[Emitted Int | Finished]#2(call core.iterator::finished())
  }
  assign __p.at = call core.basic::sub(Int, Int)(read __p.at, 1)
  return make_union[Emitted Int | Finished]#1(call core.iterator::emitted<Int>(call core.basic::add(Int, Int)(read __p.at, 1)))
}

fn main::main() -> None {
  bind#1 let console: core.console::Console = construct core.console::StdOutConsole {}
  bind#2 let b: Mut Box = construct Box { n: 1, tag: "t" }
  member core.console::Console.println(read console, concat("1. ",
      call core.basic::to_str(Int)(read b.n), " ",
      call core.basic::to_str(Int)(call main::bumped(read b))))
  member Console.println(read console, concat("2. ",
      call core.basic::to_str(Str)(call main::label(read b.tag, call main::bumped(read b)))))
  bind#3 let xs: List<Int> = call core.list::list_of<Int>([3, 2, 1])
  bind#4 let twin: List<Int> = call core.basic::copy<List<Int>>[plan: identity](read xs)
  member Console.println(read console, concat("3. ",
      call core.basic::to_str(Int)(call main::consume(!read xs)), " ",
      call core.basic::to_str(Int)(call core.list::size(read twin))))
  bind#5 let __pass: Mut __Iter_countdown_Int = call main::countdown(3)
  loop#1 {
    bind#6 let __step: Emitted Int | Finished = call main::next#1(read __pass)
    switch#1 read __step {
      arm#1 (Emitted Int) {
        narrow#1[switch#1.arm#1] x: Int = __step
        member Console.println(read console, concat("4. ", call core.basic::to_str(Int)(read x)))
      }
      arm#2 (Finished) { break }
    }
  }
}
```

Things to notice:

* `bumped(read b)`: a `lent_mut` parameter, so the argument is `read b` with
  no consume mark; Rust spells `&mut b`, and *reconstructs* the hoist of the
  earlier `read b.n` from the two reads of `b` in one argument list.
* `consume(!read xs)`: the one consuming read. Rust moves; Kotlin passes the
  reference.
* `copy` carries its plan; Kotlin reads `identity` and writes `xs`, Rust
  ignores it and writes `.clone()`.
* The `for` is gone: a loop, a `next` call, a `switch` on the step, and a
  `narrow` for `x`. Kotlin can alias `x` to `__step.value`; Rust writes a
  `while let`.
* `next#1`: `next` is overloaded across std, so the reference says which.
* `at <= 0` is a `cmp` call then a test of its result, both references to
  declarations marked `intrinsic`; Rust and Kotlin reconstruct `<=` from the
  pair. No operator node and no intrinsic node exist in the IR, so one rule
  covers `Int` and `Person`, and one rule covers an intrinsic and a fn with a
  body: the reference is the same, the declaration differs.
* Every `Mut` that survives is on a *type* (`Mut Box`, `Mut __Iter…`), kept
  for backends that represent it; every other qualifier is gone.

## 9. Decisions (user, 2026-10-06)

1. **Effects as values: the instance's type** is the interface type itself
   (`console: Console`); how an instance is represented (Rust's handle struct,
   Kotlin's interface) is the backend's.
2. **Actors** (revised 2026-10-06 on the evidence of the Kotlin port, user
   decision): a hybrid. The message union and the dispatch fn are behaviour
   and become IR declarations the builder generates; the **codecs stay in the
   backend** as per-type `__Codec_*` classes, since a struct's codec must agree
   byte for byte with the `UnionN` codec, which is a representation choice the
   backend owns; `Spawn`/`Send`/`ReplyTo`/`WaitFor` stay nodes, since they
   bottom out in the scheduler's host primitives. To be revisited if a neater
   split appears.
3. **Generated-by-type code** (§7): behaviour (codecs, `copy`) is IR the
   builder writes; representation (`UnionN`, optionals) stays in the backend.
4. **A `salvo-ir` crate** holds the node types, the builder from `Program +
   Checked`, the dump and `salvo ir`; `salvo-backend` holds the reconstruction
   helpers over it. Core stays the checker.
5. **Generics stay** in the IR; a call carries its type arguments.
6. **`Mut` survives as a type flag**, the one qualifier that does, because a
   backend may represent a `Mut` type differently (Kotlin's `StringBuilder`).
7. **Transition order**: builder and dump first, then Kotlin, then Rust, then
   actors (§10).
8. **Loops**: one unconditional `Loop` statement; everything else lowered,
   value locals only where the program has a value or an `else` (§3).
9. **References**: a `DeclId` (module, item index) for every declaration,
   intrinsic or not; the declaration's own flags say how a backend provides it.
   Interpolation parts and operators are ordinary calls to the resolved fn.

Confirmed: nothing here changes the language. Treating `x is Person p` and
`x is Person` as one narrowing binding is an implementation unification.

## 9a. Built so far (2026-10-06)

Step 1 of §10 is built: the `salvo-ir` crate (`ir.rs` nodes, `build/` the
builder, `dump.rs`), `salvo ir [--all] [--module m]`, a corpus test that
builds every inline program of both backends' codegen tests (about 350, none
`Unsupported`), and golden snapshots of every example's IR
(`crates/salvo-ir/tests/snapshots`). Decisions the build forced, all
provisional until reviewed:

* **`Op` exists after all.** `+ - * / %`, `and`/`or`, `!`, unary `-` have no
  declaration in std to reference, so they are an `Op` node on scalars; `<`
  and friends are `Op::Lt…` over the `Int` a `cmp` call answered, against 0
  (the sign test). Comparisons themselves are calls. The alternative — std
  declaring `intrinsic fn add(Int, Int)` etc. — makes `add(1, 2)` callable,
  a surface change, so it was not taken without asking.
* **`Test` node.** `x is T` as a *condition* (inside `if`/`while`, in an
  `and` chain) is `Test { subject, test }` of type `Bool`; the narrowing it
  justifies is a `Narrow` with `Justification::Test`. `Switch` is for the
  subject forms (`when`, `?:`, `?.`, `!`, `for`).
* **`ForEach`** for intrinsic containers (a list, an array, a `Str`), which
  the checker records no driver for [iter-for-native]: the one loop form
  with a subject. A `List<T>` has a Salvo `iter`/`next` pair, so this could
  become the pass form; left as is because both backends iterate natively.
* **`Static`** for a module-level `use`: an instance every fn of the module
  reads as a local.
* **Block expressions** are a `Branch` with one `true` arm (`x!`, `?:`, a
  projection of a call result, `x++` as a value); the IR has no block node.
* A checker table was added for the builder: `Checked::written_types`, the
  lowered type of every written type by span, so declarations copy the
  checker's types rather than lowering again.

### Step 2, so far

`crates/salvo-backend-kotlin/src/ir_emit/` (`mod.rs`, `actors.rs`; about
2,000 lines) renders the IR to Kotlin, selected by `SALVO_KOTLIN_IR=1` (the
`Backend` and the codegen tests both honour it). It decides representation
only: `UnionN` wrappers, `T?`, `StringBuilder`, fully qualified references (so
it writes no imports), `when` for `Switch`, `while (true)` for `Loop`,
anonymous `fun` for a lambda, `run {}` for a block in value position, the
platform adapter classes (`__Platform_E`, `__Platform_H`), `try`/`catch
(ThrowSignal)` for `Try`, and the actor phase of §9 decision 2: the wire
codecs per type, `__Msg_E`/`__Codec___Msg_E`/`__PROTO_E`/`__Stub_E` per actor
interface, `__Mon_E` per interface, `__Cont_H`/`__Priv_H`/`__Actor_H` per
actor impl (several faces, private sends, `init` as the first message),
`__Fac_H` for a mixed handler, and `Spawn`/`Send`/`ReplyTo`/`WaitFor`/
`Handle`/`AddrInstance` as scheduler calls. With it, **every example** prints
its `expected.txt`, the whole std compiles, all 33 CLI run tests pass, and
the Kotlin codegen compile-and-run shards that reach kotlinc pass (two shards
still stop at textual assertions on the AST emitter's spelling — `val x`
versus a qualified name — which the flip to the IR path revises). The AST
path is still the default.

What the port added to the IR, all backend-neutral:

* `Handle { instance }` [effect-handle]: a `use` binds one handle per face
  over one instance (`__Mon_E` on Kotlin, `__Handle_E` on Rust); a module
  `use` is one `Static` for the instance and one per face.
* `AddrInstance { addr }` [actor-use-addr]: the instance behind an addr —
  `use addr`, and a dependency given an addr. A plain effect's addr *is* its
  handle [monitor-handler].
* `Widen { value }` [op-promote]: a numeric operand widened within its class;
  a literal is retyped instead (`0` against a `Long` is `0L`) [lit-adopt].
* `Spread { value }` inside an `Array` literal [fn-variadic].
* `ArmTest::Lit(Vec<LitArm>)`: per runtime arm, the literal values accepted
  on it [type-literal].
* `Justification::LoopValue`: a value loop's result local is `T?` while it
  runs and read narrowed after it [ir-loop].
* `InterfaceDecl.protocol_hash`, `FnDecl.send`, `PlatformTypeDecl.platform`.
* `ReplyTarget::Task` for a free `send fn` [task-mint].
* A tagged `None` arm (`Ok None`) keeps its tag through erasure: it is the
  unit value arm, not the absent one [type-none-unit].
* Interface members carry only their written parameters: a prerequisite
  [effect-prereq] is a handler's dependency, not a member's parameter.
* **Condition narrowings are scoped by nesting** [ir-narrow]: a narrowing a
  later `elif` condition or the right operand of `&&`/`||` depends on makes
  the rest of the chain a nested `Branch` in the preceding `else` (or the
  operand a `Branch` on the left), so no narrowing runs where its test did
  not hold. A test's subject is read *as stored* (no alias); an assignment
  through a narrowed name writes the storage and ends the narrowing.
* `Checked::written_types` is keyed by the AST node's address, not `(file,
  span)`: a foreign declaration's types are lowered under the referencing
  file, where spans collide. The IR is built from the checked program itself;
  effect-generic erasure [effect-generic-decl] is applied where a type is
  rendered.

## 10. Transition plan

The e2e suite (1712 tests, same program compiled and run on both backends,
stdout compared) is the oracle. Golden snapshots of emitted code will all
change and are not the check; the IR dump gets its own goldens.

1. **`salvo-ir` with builder and dump, beside the current path.** No emitter
   changes. `salvo ir --src …` dumps every reached module. Snapshot the dump
   for the corpus and the examples. This is where the node set gets tested
   against the whole language: every construct the builder cannot absorb is
   found here, with no backend involved.
2. **Port Kotlin to read the IR.** Kotlin is smaller (9.7k lines) and has no
   ownership rendering. Both Kotlin paths exist until the IR path passes every
   e2e test; then the AST path is deleted. The reconstruction helpers
   (imports from references, elvis detection, value-position lowering) go in
   `salvo-backend` as they are written.
3. **Port Rust.** The ownership reconstruction (`BindKind`, clone insertion,
   hoists, `__loc` twins, element handles) is rewritten against `consume`
   marks and `PassMode`. This is the largest step; the e2e suite and the
   hoist/narrowing codegen tests are the checks.
4. **Actors and generated-by-type code as IR** (decisions 2 and 3).
5. **Retire** the span-keyed tables the emitters no longer read, the
   `salvo-backend` walkers the IR made unnecessary, and this document.

Each step is a series of commits; each commit leaves the suite green. The
timelog will say what each cost.
