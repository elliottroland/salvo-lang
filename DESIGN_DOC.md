# DESIGN_DOC.md — the working-document shape

A bootstrap template for the design working documents this repository uses to
open a phase. If you are about to write one — a read-ahead option space for an
upcoming phase, or the design pass for a **DECISION** in ROADMAP.md — read this
first, then follow the skeleton in §2. The live instance is **CONCURRENCY.md**
(phase 5, `OPEN`); when this template is ambiguous, copy what it does. Two
retired instances show the whole life cycle: OBLIGATIONS.md (phase 3) and
FILE_SYSTEM.md (phase 4), both **deleted** once their phase landed, with their
decided outcomes in COMPLETED.md's decision log.

## 1. What a working document is, and is for

A working document is written **before** a phase is built, so the
language-design calls the phase forces are *chosen* rather than discovered
mid-build. It lays out each open question as options with trade-offs and a
recommendation — and **the calls are the user's** (AGENTS.md's first invariant:
any choice that shapes the language surface or its semantics is presented to the
user with options, trade-offs, and a recommendation, and the outcome is recorded
in COMPLETED.md as a user decision). The document is analysis; it does not decide.

Two consequences of that charter:

- **It is temporary.** Once the user has made the calls, the outcomes are folded
  into COMPLETED.md's decision log and the specs, the corresponding ROADMAP.md
  **DECISION**s become plans, and **the document is deleted** — its history lives
  in the log, not in a directory of superseded design docs. This is the charter
  OBLIGATIONS.md had (deleted once phase 3 landed), that FILE_SYSTEM.md
  followed (deleted once phase 4 landed, 2026-09-14), and that CONCURRENCY.md
  carries. A working document left lying around after its phase is a
  stale-syntax hazard, exactly what AGENTS.md warns against.
- **A read-only session writes one but does not propagate it.** These documents
  are often written in a session restricted to just the new file(s). In that case
  the eventual edits to ROADMAP.md / COMPLETED.md / the specs are *owed, not done*
  — say so explicitly in the header (see the propagation-owed note in §2), so the
  next session knows the outcomes have not landed anywhere yet.

## 2. The skeleton

Follow this section order. CONCURRENCY.md uses the label prefix `C-`
(FILE_SYSTEM.md used `FS-`); pick a short stable prefix for your phase and use
it for every decision section.

### Header block

- **Title line:** `# <Topic> — the phase-N option space (working document)`.
- **`Status:` line:** `OPEN` (nothing decided yet — CONCURRENCY.md) or `DECIDED`
  (with the date and a pointer to the section holding the decided list — how
  FILE_SYSTEM.md ended up). Update it as rounds land; keep superseded sections with a
  pointer to what replaced them, for the argument trail.
- **Provenance paragraph:** who wrote it, when, under what phase, and while what
  else was in flight. State plainly that it lays out the design in
  options/trade-offs/recommendations style "and the calls are the user's."
- **Propagation-owed note** (when written read-only): name the files whose
  updates are deferred — COMPLETED.md's decision log, the ROADMAP.md
  **DECISION**s that become plans, the new LANGUAGE_SPEC.md / backend-spec rules
  and their fresh labels — and state that until the user decides, nothing has
  propagated. Include the delete-when-decided charter here.
- **`Sources:` paragraph:** cite the ROADMAP.md sections, COMPLETED.md log
  entries, LANGUAGE.md / LANGUAGE_SPEC.md / backend-spec sections, and the rule
  **labels** you rely on — and the user's stated intent. **Verify every label
  exists before citing it** (`grep -rn '\[label-name\]' *.md`); a cited label
  that no longer exists is a bug (AGENTS.md).

### §0. The stated intent

Restate the user's sketch, **numbered**, "so the decisions below can be judged
against it." If the sketch is tentative, say so, and say how it is being used
(to *guide* the options and their cost, not to fix answers). Close §0 by naming
which points fit the recorded direction outright and which meet a rule or an
open **DECISION** — those meetings become your numbered sections.

### §1. Fixed points — already decided, inherited here

Bullets, each tied to a **rule label** (`[linear-obligation]`, `[rs-fn-field]`,
…) or a decision-log entry. These are what the phase builds on and does not
reopen: settled decisions, things the implementation already does, and the open
**DECISION**s named as such (a fixed point can be "this is still open, and here
is exactly why"). **Flag where a §0 point collides with a fixed point** — that
collision is the reason the decision section exists.

### §2. What other languages teach

A `###` subsection per language or family. Each: the pattern, any unique
variation worth stealing, and **which §0 point or decision it informs**. Group
by family (FILE_SYSTEM.md grouped by API lineage; CONCURRENCY.md groups by
concurrency family) and, when the choice must encode into the backends, add a
subsection for **what Kotlin and Rust do**, since every decision must land in
both. Verify claims against primary documentation; web research is expected.

### §3…N. The decision sections

One numbered section per decision, stable-labelled (`FS-1`, `C-1`, …). Each
section:

- states the question and which §0 point and/or ROADMAP **DECISION** it folds;
- lists **at least two options**, each with **cost and trade-offs**, and
  **at least one shape-changing alternative** — an option that changes the
  overall shape but might be better for reasons the sketch did not consider;
- ends with a **recommendation explicitly marked as the user's call.**

Order the sections so the load-bearing decision (the one others depend on) comes
first, or state the dependency order explicitly near the table (CONCURRENCY.md
notes "C-5 first").

### Final: decisions-pending table

A summary table in ROADMAP.md style: one row per decision, its label, the
question, the §0 point / ROADMAP **DECISION** it answers, and the one-line
recommendation. State the load-bearing order. This table is what a session
reads first, so keep it faithful to the sections above.

## 3. Conventions checklist

- [ ] Title, `Status:`, provenance, propagation-owed note (if read-only),
      `Sources:` — all present in the header.
- [ ] Every cited rule label verified to exist (`grep`); backend-prefixed labels
      (`kt-`, `rs-`) cited only as representational facts, not as core rules a
      core crate would reference.
- [ ] §0 numbered and judged-against-able; tentative intent marked as such.
- [ ] §1 fixed points each carry a label or a log reference; §0 collisions
      flagged.
- [ ] §2 informs each entry back to a §0 point / decision; backends covered when
      relevant.
- [ ] Every decision section: ≥2 options, cost/trade-offs, ≥1 shape-changing
      alternative, recommendation marked as the user's call.
- [ ] Decisions-pending table present, with load-bearing order.
- [ ] Delete-when-decided charter stated; only the intended file(s) written in a
      read-only session.

## 4. Worked instances

- **FILE_SYSTEM.md** — phase 4 (the filesystem, on an IO stream design).
  **Retired 2026-09-14** with the phase, so it is a life-cycle example rather
  than a readable one: it ran through six rounds of user decisions as a
  `DECIDED` document — superseded sections kept with pointers for the argument
  trail, a final section holding the decided list, the agreed implementation
  sequence and the propagation owed — and was deleted when the last item
  landed. Its `FS-`/`O-` decisions live in COMPLETED.md's decision log, which
  is where the `§`-references left behind in code comments now read against.
- **CONCURRENCY.md** — phase 5 (threading and concurrency, the OTP model). Shows
  an `OPEN` document at the start of a phase: tentative intent, fixed points that
  include four open ROADMAP **DECISION**s, a cross-language survey with a
  backends subsection, and eight decision sections (`C-1`…`C-8`) each folding a
  sketch point and/or a **DECISION**.
