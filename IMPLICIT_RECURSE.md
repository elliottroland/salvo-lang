# Implicit resolution blows up on nested `to_str` (and friends)

Working document. Status: **reproduction + design question open** — awaiting a
language-design DECISION (see "The design question"). The memoization proposal
is recorded at the end so it is not lost.

## What happened

On 2026-10-14 the `salvo` LSP process was found pegging a core (~100–170% CPU,
worker threads included). A `sample` dump (`/tmp/salvo-lsp-sample.txt`, captured
via `LSP_DEBUG.md`) put essentially the whole process — 3499 of 3580 samples —
in one place:

```
lsp::run
  → Server::analyze_for → analyze_for_document_cached → analyze_project
    → check_project → check_program_rounds → check_once → check_module
      → check_fn_item → check_fn → … → check_expr_inner
        → record_text_form → interp_text_form        ← a "${…}" interpolation
          → fill_implicit_list ⇄ resolve_implicit_fn_at   ← nested, deep
            → fn_value_ty → lower_type → lower_type_subst → lower_base_ref
              → type_fn_slots → expand_slots → lower_type → …   ← per level
```

It is **not** idle spin (the earlier LSP investigation that found no idle loop
was right). It is active, very expensive *analysis*: resolving the `?to_str`
implicit of a string interpolation, where the chosen `to_str` has a `?to_str`
implicit of its own, which is resolved in turn, and so on. The LSP re-runs the
whole analysis on every debounce, so a single pathological buffer state keeps a
core busy indefinitely.

Important scoping facts established at the time:

- The stuck process was the **pre-rebuild binary** (launched before that day's
  work). It was analysing whatever was in the editor buffer, not a committed
  file.
- The current binary analyses `demo/demo.sv` in ~0.5s. The collection-
  constructor rename landed that day did **not** introduce this; it is a
  pre-existing property of nested implicit resolution.
- Killing the stale LSP process cleared the CPU; a fresh LSP runs the current
  binary.

## The mechanism

String interpolation `"${x}"` resolves a `to_str` for `x`'s type as an implicit
([interp-text-form] → `record_text_form` → `interp_text_form` →
`fill_implicit_list`). std's collection `to_str`s take a `?to_str` **of their
own** for the element/key/value:

```
// std/core/list.sv
fn to_str<T>(list: List<T>, ?to_str: (x: T) -> Str) [] -> Str
// std/core/map.sv
fn to_str<K, V>(map: Map<K, V>(?hash, ?eq), ?to_str: (k: K) -> Str, ?to_str: (v: V) -> Str) [] -> Str
```

So `to_str` for `List<Int>` wants a `to_str` for `Int`; `to_str` for
`List<List<Int>>` wants a `to_str` for `List<Int>`, which wants one for `Int`;
and so on. This is the designed behaviour: `[implicit-recursive]` (decision
2026-09-28) says a candidate implicit with implicits of its own is a candidate,
and its implicits are filled in turn.

The nested fill goes through `resolve_implicit_fn_at` → `try_nested` →
`fill_implicit_list` (`crates/salvo-core/src/check.rs`). `try_nested` carries
two guards:

- **Depth cap**: `resolve_depth >= 8` → error "nests more than 8 levels deep".
- **Cycle refusal**: the `(FnKey, want-type-as-string)` pair already on
  `resolve_chain` → error "needs itself … a cycle".

Those guards make the recursion **finite**. The blowup is therefore not
unbounded depth — it is **width × per-level cost × repetition**:

1. At each level, `resolve_implicit_fn_at` iterates **every overload** of the
   wanted name (`overloads_of("to_str")`), and for each one calls
   `fn_value_ty` → `lower_type` → `lower_type_subst` → `lower_base_ref` →
   `type_fn_slots` → `expand_slots`. `expand_slots` **clones** the slot table
   (`HashMap<String, String>`), and `lower_type_subst` rebuilds `Vec<Ty>` and
   clones `Ty`s. So one level is already many allocations.
2. Each fitting candidate with its own implicits recurses (`try_nested`).
3. **Nothing is memoized.** Resolving `to_str` for `Int` is redone from scratch
   every time it is reached, at every level and every branch. The same
   `(name, want)` is resolved over and over within a single pass.

With `w` visible `to_str` overloads and depth `d` (≤ 8), the work is on the
order of `w^d` — exponential in the depth cap. A handful of `to_str` overloads
and a type that nests several containers deep is enough to turn one
interpolation into millions of resolutions, i.e. a multi-second analysis pass
that the LSP then repeats.

The cycle guard does not help here because each level has a **different**
wanted type (`List<Int>` → `Int`), so the `(key, want)` pair never repeats; only
the depth cap stops the descent, and the depth cap is exactly what makes the
cost `w^8` rather than infinite.

## Reproduction

Status: **mechanism reproduced; CPU blowup NOT reproducible from std.** Measured
against a prebuilt `target/debug/salvo` on `tmp/recurse/salvo` (scratch,
gitignored) to avoid cargo contention.

What reproduces the *exact recursive chain* from the sample — a generic,
unresolvable element so every level recurses and the leaf fails:

```
fn show<T>(xs: List<List<List<List<List<List<T>>>>>>) [Console] -> None => xs {
    println("${xs}")
}
```

yields precisely the sample's nesting:

```
no `to_str` fits `?to_str: (List<List<List<List<List<T>>>>>) -> Str` …
  needs implicits of its own, and no `to_str` fits `?to_str: (List<List<List<List<T>>>>) -> Str` …
    … → `?to_str: (T) -> Str`: the `to_str` in scope is `(Int) -> Str`, and the position needs `(T) -> Str`
```

But every attempt stayed **fast (~0.46–0.54s)**, and analyze time is **flat
across nesting depth 4→10**. The reason is decisive: with the std surface there
is **exactly one** fitting-and-recursing `to_str` per level
(`to_str<T>(List<T>, ?to_str)`), so the descent is **linear** in depth, and the
depth-8 cap (or a clean bottom at a concrete `Int`) ends it: `1^d`, not `w^d`.

### Hypotheses tried and ruled out (all ~0.46–0.54s)

1. Concrete `List<…<Int>>` nested 4–10 deep — linear, flat.
2. Nested `Map<Str, Map<Str, …>>` to the cap — Map's key implicit resolves at
   once, the value implicit recurses once per level: still width 1.
3. A recursive user type whose `to_str` takes **no** implicit — no recursion.
4. Open-`T` nested lists (the chain above) — recurses to the cap, fails, but
   width 1.
5. Open-`K,V` nested `Map` — errors early (a `Map` key must be hashable).
6. `List` whose element is a **union of containers**
   (`List<List<Int> | Map<…> | Set<…>>`) — no `to_str` fits a union position,
   fails fast.
7. **Two** fitting `to_str<T>(List<T>, ?to_str)` overloads (user + std),
   open-`T` leaf, depth 8 — the two same-shape candidates collapse to one
   near-miss chain; the resolver does **not** explore them as independent
   recursing branches. Width stays effectively 1 (0.52s).
8. Nested `Map`/`Set` keyed on a `: Hashed<self> by auto, Ordered<self> by auto`
   struct (comptime obligation expansion per level) — errors early.
9. A **growing** nested `to_str` (`to_str<T>(x: T, ?to_str: (List<T>) -> Str)`,
   whose nested want is *larger* than its input, hypothesis (ii) below) — the
   want oscillates `(List<T>) → (T) → (List<T>)` under generic binding rather
   than growing monotonically, bottoms out, stays fast.

### Conclusion

The `w^d` blowup is **not reachable from the std surface** — std gives width
`w = 1` at each level, and same-shape user overloads collapse rather than
branch. The live editor buffer that pegged the CPU had something these
synthetic cases do not. Leading remaining hypotheses, in order of suspicion:

- **(i) A mid-edit `Unknown`/ambiguous element** where candidate selection
  genuinely could not settle to one *and the near-miss machinery re-explored*
  rather than collapsing — i.e. a shape unlike case 7, where the candidates are
  distinct enough to each recurse. We could not construct one; it may need a
  partial/ill-typed buffer the LSP sees between keystrokes.
- **(ii) A type that *grows* per level** (a qualifier/alias that wraps the
  element in something larger each step), so the `(key, want-string)` cycle
  guard never fires, depth reaches 8, and per-level `expand_slots`/`Ty::clone`
  cost rises with the growing type — super-linear even at width 1. **Tried
  (case 9) and ruled out** for the obvious construction: under generic binding
  the want oscillates rather than growing, so it bottoms out. A
  non-generic growing wrapper (a chain of distinct aliases) was not tried and
  remains the live form of this hypothesis.
- **(iii) Many *distinct* visible `to_str` overloads** (a large program, not a
  2-line scratch) so each level lowers a long candidate list via
  `fn_value_ty → lower_type → expand_slots`; linear per level but with a large
  constant, multiplied by depth. Plausible in a real workspace, invisible in a
  minimal file.

### Checked against all real Salvo code (2026-10-14)

Both repositories hold all existing Salvo code. Every committed project analyses
in **0.47–0.56s** against the current binary — none reproduces the blowup:

- `demo/` 0.51s; `bench/scheduler` 0.47s; `bench/collections` 0.48s;
- `examples/{actors,cluster,collections,aws_s3,files,iteration,linearity,qualifiers}`
  0.47–0.56s;
- `../salvo-noticeboard/src` 0.51s (3 user files; it is work-in-progress — 1
  error, 6 warnings — and uses the `codec`/`stream` generic surface).

Notes from the noticeboard:
- Its interpolations are of `Str`/`Path`/placeholder `${_}` — not nested
  collections. Interpolating its `Notice` struct (no `to_str`) fails fast (one
  near-miss), ~0.46s.
- `std/stream.sv`'s `write_value<T>(…, ?encode: (T) -> Bytes)` /
  `read_value<T>(…, ?decode: (Bytes) -> T?)` take recursive implicits *shaped*
  like `?to_str`, but their fillers `encode<T>`/`decode<T>` (`std/codec.sv`)
  are **single intrinsics taking no implicits**, so that path is width 1 and
  does not recurse. So the codec surface is not a second source of the blowup.

This makes the mid-edit hypothesis the strongest: the pegged buffer was an
**unsaved, transient editor state** (the noticeboard is actively being written —
`todo!()`, `unreachable!()`, `?:`/`??:`/`^Ok` operators mid-construction) whose
inferred types during a keystroke-to-keystroke window produced the wide/ambiguous
`to_str` search that no saved file reproduces. It is gone with the buffer.

**The reliable next step is to capture the offending buffer**, not to keep
guessing: next time the LSP pegs, save the exact editor source (and the
`SALVO_LSP_TRACE` log). **This now happens automatically** — the server times
every analysis pass and dumps the buffer of any pass over a threshold (default
2000ms) to a single overwritten file (`salvo-lsp-slow.sv` + `.txt` in the OS
temp dir); see `LSP_DEBUG.md` §0 and `[lsp-slow-dump]` in `crates/salvo-cli/src/lsp.rs`.
(All *saved* Salvo code has been checked and none reproduces it —
see "Checked against all real Salvo code" above — so the trigger is an unsaved
mid-edit state the dump-on-slow hook is the only way to catch.)

To re-run the chain repro:

```
mkdir -p tmp/recurse/salvo && cat > tmp/recurse/salvo/main.sv <<'EOF'
fn show<T>(xs: List<List<List<List<List<List<T>>>>>>) [Console] -> None => xs {
    println("${xs}")
}
fn main() [use] { use StdConsole() }
EOF
time ./target/debug/salvo analyze --src tmp/recurse
```

## The design question

The reproduction narrows this. With the std surface, each level has width
`w = 1`, so cost is linear and the depth-8 cap is almost never reached in
practice. The CPU blowup needs **`w > 1`** — several candidates that both *fit*
and *recurse* at a level — and/or an **unsettled element type** that makes the
checker explore the full candidate set at every level before failing. So the
question splits:

**Is the fix purely implementation performance, or is there a surface the
language should change?** A DECISION for the user, because the options differ in
observable behaviour:

- **(A) Pure performance.** The set of programs that type-check is unchanged;
  the exponential ones are just slow. Fix: memoize resolution within a pass
  (below). No language-design content.

- **(B) The depth-8 cap is a language-visible contract.** A program nesting
  `to_str`-bearing containers 9 deep gets "nests more than 8 levels deep"
  today. Keeping that error makes the cap part of the language; making depth
  cheap (memoization) might let us raise/remove it, which changes which
  programs check. The cap's status is a language-design call.

- **(C) `[implicit-recursive]` is too permissive about *width*.** The real
  blow-up fuel is multiple fitting-and-recursing candidates, or an open element
  type that defeats single-candidate selection. Options that change the
  surface: forbid an *ambiguous* nested implicit from recursing at all (fail
  fast instead of exploring every branch); require container `to_str` to take
  the element renderer as an ordinary argument rather than a recursing
  implicit; or cap *width* (refuse when >1 candidate recurses) rather than
  depth.

Open sub-questions, now informed by the repro:

1. **Capture the real `w`.** We could not reproduce `w > 1` from std. Next time
   the LSP pegs, save the buffer. The design choice between (A), (B), (C)
   depends on whether real `w` comes from (i) legitimate multiple user
   overloads, (ii) `Map`'s two implicits compounding, or (iii) an `Unknown`
   element type during a mid-edit — only (iii) argues for a surface change (C),
   since (i)/(ii) are legitimate programs that just need to check fast (A).
2. **Does failure short-circuit?** The costly sample ended in failure
   (`T` unresolved at the leaf). Confirm whether a *failing* nested resolution
   re-explores candidates (near-miss bookkeeping, `[type-unknown-lenient]`)
   more than a succeeding one — if so, the LSP's mid-edit (often ill-typed)
   buffers are the worst case, which argues for fast-failing (C) over just
   memoizing (A).
3. **`to_str` vs `eq`/`cmp`/`hash`.** Do the other implicit groups recurse and
   branch the same way, or is `to_str` the only interpolation-triggered one hot
   on every keystroke?

## Proposal on the table: memoize resolution within a pass

Recorded so it is not lost; **not yet decided**.

`resolve_implicit_fn_at(name, at, want)` is a pure function of its inputs plus
the current scope/visible-overload set, which is constant within a single
`check_once` pass. Cache its result (`Result<(FnKey, FnId), ImplicitMiss>` plus
the `last_nested` it would set) keyed by `(name, at, want-as-key)` for the
duration of a pass, and clear the cache between passes.

- Collapses the `w^d` blowup to roughly `w × (distinct wanted types)` per pass:
  `to_str` for `Int` is resolved once and reused everywhere it recurs.
- Behaviour-preserving **iff** resolution really is a function of
  `(name, at, want)` and nothing pass-local mutates between the first and later
  lookups of the same key. Needs an audit of `resolve_implicit_fn_at` for
  hidden state (it reads `self.resolve_as_call`, `self.own_implicits`, the
  rename stack — the cache key may need to include those, or the cache must be
  scoped to where they are stable).
- Interacts with the depth cap: if a cached entry was itself computed under a
  depth-8 cutoff, replaying it at a shallower depth is fine (the result is the
  same fn + nested fill); replaying a *failure* that was only a failure because
  of the cap would be wrong, so cap-induced errors must not be cached. This is
  the subtle part.
- Does **not** by itself answer the design question (B)/(C); it is the fix for
  (A), and it may make (B) moot by making a higher cap cheap.

Alternatives noted, not developed: lower the depth cap (changes behaviour),
widen the cycle guard to catch "structurally shrinking but string-distinct"
chains, or move container element rendering off the implicit mechanism.
