# The compiler's phases, and where a piece of work belongs

This page records what each phase of the compiler is *for*, and the criteria
for deciding which phase a new piece of work goes in. Read it before you add a
fact, an analysis or a diagnostic whose home is not obvious. It complements
[TOUR.md](../../TOUR.md), which says where things *are*. Add to it when a
placement decision teaches something general.

## The phases

| phase | code | what it is for |
|---|---|---|
| parse | `salvo-syntax` (`lexer.rs`, `parser.rs`; `parse_module`) | Text to a tree, per file: the source **as written**, its sugar kept (`ast::Sugar`), so a printer can write it back. No lowering and no numbering. |
| expand (per file) | `salvo-syntax/src/desugar.rs`, `salvo-core/src/expand.rs` (`expand_file`) | The tree as written lowered (`desugar::lower_module`: the namespaces flattened, the sugar as the code it stands for, the struct-body hoist and `iter fn`s, `?:`/`?.`/`?>`), then the tests and the message arms; then numbering. Nothing that needs a name resolved. |
| resolve | `salvo-core/src/resolve.rs`, `scope_ids.rs` | Which declaration every name means: each file's scope, against the program's declaration headers. |
| interfaces | `check.rs`, the interface pass | Each declaration's signature as the program means it: types lowered, effect lists expanded, implicits, default deductions. Reads the declaration's own text and other declarations' interfaces, never a body. |
| comptime | `comptime.rs`, `expand.rs` | Stamps `by` clauses and comptime bodies, between the interfaces (which it reads) and the bodies (which it produces). |
| bodies | `check.rs`, the body pass, then `deduce.rs` | Every fact about the values and operations in a body (see below), then the deductions inferred from the bodies, then the declaration facts that rest on them (`param_mode::record`, `Checked::primitives`). |
| lower | `ir/build/` | The checked program as IR: each node built from the facts the checker recorded. It decides nothing (see below). |
| flow | `ir/flow.rs`, `ir/obligations.rs`, `ir/check.rs` | Every question about one path through a body: moves, poison, which bindings take ownership, linearity. |
| link | `ir/build/actors.rs`, `ir/build/refs.rs`, `deadlock.rs` | What needs the whole program: actor glue, `ref` containers, deadlock. |
| emit | `salvo-backend-*` | The IR spelled in a target language. |

## Placement criteria

### The checker owns facts; the flow pass owns paths

The deciding question is what a fact is **about**, not who reads it. Every
fact the checker records is for the IR in the end, so "only the IR reads it"
says nothing about where it belongs (user decision, 2026-10-30).

- **The checker** answers questions about *values and operations*. What type
  is this value, and what does it claim? What was it derived from (its fate
  links)? What does this call resolve to, and what does it do to its
  arguments (its contract, the paths it mutates, the handles a `NotSame`
  claim proves apart)? A narrowing and an `is` binding's type are facts
  about a value, so they are the checker's, however the IR lowers them.
- **The flow pass** answers questions about *one path through the code*.
  What has happened to this value by this point: is it moved, poisoned,
  owned, a virtual place, iterated by value? Which binding takes ownership?
  These depend on the order of events along a path, which the checker's walk
  does not track (it treats every binding as borrowing, and joins branches by
  union).

The flow pass reads the checker's facts and applies them along each path.
Where the checker walks a loop body twice to finish its facts, it hands over
the final result; the flow pass's own loop fixpoint computes path state the
checker never computes, so the two do not have to agree on anything.

A question often splits across the line. Example: that `keep(p.tags)` moves
data out of `p.tags` is a fact about the operation, so the checker records
it (a consuming read at the projection). That the move makes `for p in ps`
iterate by value depends on what `p` is derived from on that path, so the
flow pass decides it, and the builder lowers the loop from the decision
(the pipeline record D3.5(b)). The IR is built first with every binding
borrow-mode and every loop by reference; the flow pass decides over that
build, and the items it decided something in are built again.

When moving a decision from one phase to another, compare both answers
over the suite with a temporary differential before deleting the old one,
and confirm on a probe that the comparison runs at all.

An event can be the checker's while what it means on a path is the flow
pass's. "This variable is moved here, for a `return`" is a fact about the
operation, and recording it needs types (a Copy scalar's move is a copy, a
`Mut` handle's mutation goes to its roots). Whether that variable is still
derived at that point, and from which bindings, depends on the path, so
the flow pass reads it off the links that stand there (the pipeline record D3.5(b)).
Likewise, whether a binding *may* take ownership is judged where the
binding is (its roots, a lambda around it), so it is the checker's; whether
it *does* is the flow pass's.

When the IR represents something differently from the source, a fact the
checker keeps about the source thing has to travel with the IR node that
stands for it. An `is` binding is the subject's own storage in the IR (a
`Narrow`), so the links the checker gave the binding ride on the `Narrow`;
reading the storage's links instead would answer for the subject.

### The builder lowers; it does not decide

The IR builder reads what the checker recorded and builds nodes from it. When
the builder searches a symbol table by name, matches types to find something,
or inspects a declaration's source to answer a question, it is re-deriving a
decision, and the two copies can disagree. Record the answer where it is
decided instead, keyed by identity (the pipeline record D3):

- effect arguments name their instance's binder (`Instance`), not a type to
  search for;
- calls name their callee's declaration (`Callee`, with the effect's
  `interface` for a member);
- per-declaration answers are facts at the declaration's name
  (`param_modes`, `primitives`, `written_types`).

A fact the lowering needs and does not find is a lowering error, never a
guess or a call made short [backend-never-wrong].

Every checker type enters the IR through one layer (`ir/build/erased.rs`):
erasure happens there and nowhere else in the builder.

### When two phases must agree, they share one definition

Where two phases have to answer the same question, write the answer once and
call it from both, rather than writing it twice and testing that the copies
agree. Examples: union arm identity (`salvo_core::literal`), the effect
member a handler member implements (`effect_member_index`), the handles a
tuple of handles holds (`check::handle_leaves`).

### Core states what Salvo means; a backend spells it

`salvo-core` holds the language's facts: ownership, evaluation order, what
`copy` duplicates, what a call resolves to. `salvo-backend` holds helpers a
backend may use. Each backend decides how to spell a fact in its language.
Before putting something in core, ask whether a garbage-collected backend
with expression blocks would care; if not, it is not a Salvo fact
([core-layers]).

### The parser writes what was written

The parser's tree is the source as written (user decision 2026-11-09,
[syntax-lowered]): a piece of sugar is a node of its own (`ast::Sugar`,
`Stmt::CompoundAssign`), or a spelling the tree keeps beside the node
(`Module::terminal_bodies`), or a trace in the spans (the struct-literal
shorthand, a plural `with` subject). A lowering that needs no name belongs
in the per-file expansion (`desugar::lower_module`, run from `expand_file`
before anything else), not in the parser: a printer, and so a formatter, prints the
parsed tree, and the language server reads the written file. Two rules keep
the move invisible to everything after it: the lowering writes exactly the
tree the parser wrote (spans included: a name a lowering writes several
times gets a span of its own each time, since what later phases record
about a name is keyed by its span), and it runs before numbering. A
diagnostic about how the sugar is written stays the parser's, where it is
cheapest to raise, with its id.

### Interfaces before bodies

An interface reads its own declaration's text and other declarations'
interfaces, never a body. That is what lets a body be checked against
interfaces alone, and what lets an edit to one body leave every other
declaration's facts standing ([decl-interface-pass]). A fact that needs a
body (an inferred deduction, a parameter mode resting on it) is computed
after the bodies, and recomputed every check.
