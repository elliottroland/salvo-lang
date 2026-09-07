//! [iter-generator] Planning a `yield` function as a resumable **pass**.
//!
//! A `yield` function is a producer the consumer drives: its body has to be
//! able to stop at a `yield` and continue from there on the next `next`
//! call. Neither target language gives us that for free (the JVM's
//! `iterator { … }` builder captures its environment, and stable Rust has no
//! generators), so the compiler builds the state machine itself — once, here,
//! rather than twice in the emitters. This module turns a body into a
//! [`GeneratorPlan`] the emitters *render*: numbered resume points, the
//! body's locals as fields, one flag per `defer` site, and a release path.
//!
//! The shape is the one the I3 prototype fixed
//! (`experiments/pull-iterators/`, verified against an oracle on both
//! backends): a flat `loop { match state }` dispatch where
//!
//! * every state is a list of [`Step`]s ending in a terminator
//!   ([`Step::Goto`], [`Step::Emit`] or [`Step::Finish`]);
//! * a statement that neither suspends nor jumps out of itself stays a
//!   [`Step::Plain`] and is emitted by the ordinary statement emitter — only
//!   control flow that has to cross a suspension is flattened, so a `while`
//!   with no `yield` in it is one statement to the emitter, brace to brace;
//! * a nested `for` becomes a slot ([`Step::OpenPass`] / [`Step::Drive`]),
//!   because the pass it drives must survive the outer body's suspensions;
//! * a `defer` becomes a flag plus a [`Step::Register`] / [`Step::Discharge`]
//!   pair, which is what makes the release path idempotent — one `close`
//!   after the consumer's loop covers `break` and exhaustion alike.
//!
//! The plan is purely syntactic: it references AST nodes and leaves every
//! type question to the emitters, which have the checker's tables.
//!
//! **States are numbered in the order their code is written**, which is what
//! makes the generated machine readable: steps refer to *labels* while the
//! body is walked, and a label is bound to a state number when its steps are
//! complete. A final pass collapses states that only forward and drops the
//! unreachable.

use std::collections::HashSet;

use salvo_syntax::ast::{Block, Expr, FnDecl, LambdaBody, Param, Pattern, Stmt, Type};
use salvo_syntax::Span;

/// A field of the generated pass struct.
#[derive(Debug)]
pub struct GenField<'p> {
    /// The name as written in the source — the emitters render a read of it
    /// as a field access on the pass.
    pub name: String,
    pub kind: FieldKind<'p>,
}

#[derive(Debug)]
pub enum FieldKind<'p> {
    /// A parameter of the iterator function: captured once, at creation.
    Param(&'p Param),
    /// A body local hoisted out of the body, because it may be live across a
    /// suspension. The annotation is there when written; otherwise the
    /// emitter takes the type of `value` from the checker.
    Local {
        ty: Option<&'p Type>,
        value: &'p Expr,
        span: Span,
    },
    /// The element binding of a flattened `for`: written by [`Step::Drive`]
    /// on every turn, and a field rather than a local because the loop body
    /// may suspend while it is still live.
    Element { subject: &'p Expr, span: Span },
    /// A nested pass driven by a flattened `for`: the state of the producer
    /// this body reads, alive across the outer body's suspensions. (For a
    /// *recursive* producer this field has the pass's own type, which is why
    /// recursion needs a box.)
    Pass { subject: &'p Expr, span: Span },
}

/// One `defer` site at flattened level: a flag on the pass, plus the block to
/// run. The locals the block reads are fields, so nothing is captured.
#[derive(Debug)]
pub struct DeferSite<'p> {
    pub body: &'p Block,
    pub span: Span,
}

/// A state of the machine: the steps to run, in order. The last step is
/// always a terminator.
#[derive(Debug)]
pub struct State<'p> {
    pub steps: Vec<Step<'p>>,
}

#[derive(Debug)]
pub enum Step<'p> {
    /// A statement that neither suspends nor jumps out of itself: emitted by
    /// the ordinary statement emitter, with the body's hoisted names rendered
    /// as fields.
    Plain(&'p Stmt),
    /// Mark a `defer` site pending (its flag becomes true).
    Register(usize),
    /// Run a `defer` site's block if it is pending, and clear its flag.
    Discharge(usize),
    /// Mint the pass a flattened `for` drives into `slot`.
    OpenPass { slot: usize, subject: &'p Expr },
    /// Release the pass in `slot` (running its own pending deferred blocks)
    /// and empty the slot. Idempotent: an empty slot is a no-op.
    ClosePass(usize),
    /// Advance the pass in `slot`. On an element, store it in the `binding`
    /// field and fall through to the following steps (the loop body); on
    /// `Finished`, run `finished` instead.
    Drive {
        slot: usize,
        binding: usize,
        pattern: &'p Pattern,
        /// The subject expression, so the emitter can find the `next`
        /// overload the checker recorded for it (`Checked::for_drivers`).
        subject: &'p Expr,
        finished: Vec<Step<'p>>,
    },
    /// `if cond { <then> }`, falling through to the following steps
    /// otherwise. `then` always ends in a terminator.
    Branch {
        cond: &'p Expr,
        /// Test `!cond` — a loop head asks "is the loop over?".
        negate: bool,
        then: Vec<Step<'p>>,
    },
    /// Continue at another state.
    Goto(usize),
    /// `yield value`: report the element and continue at `resume` next time.
    Emit { value: &'p Expr, resume: usize },
    /// The body is over: report `Finished` from now on.
    Finish,
}

impl Step<'_> {
    /// Whether this step ends its state (control does not fall through).
    pub fn terminates(&self) -> bool {
        matches!(self, Step::Goto(_) | Step::Emit { .. } | Step::Finish)
    }
}

/// The plan for one `yield` function.
#[derive(Debug)]
pub struct GeneratorPlan<'p> {
    pub fields: Vec<GenField<'p>>,
    pub defers: Vec<DeferSite<'p>>,
    pub states: Vec<State<'p>>,
    /// The terminal state: reporting `Finished` from now on.
    pub finished_state: usize,
    /// The release path, run when the *consumer* stops driving the body:
    /// pending deferred blocks latest-first, and any open pass closed. Every
    /// step is flag- or slot-guarded, so running it twice — or after the body
    /// finished on its own — does nothing the second time.
    pub close: Vec<Step<'p>>,
}

impl GeneratorPlan<'_> {
    /// The names the body reads through the pass rather than as locals.
    pub fn field_names(&self) -> HashSet<&str> {
        self.fields.iter().map(|f| f.name.as_str()).collect()
    }

    /// The plan as text, with expressions quoted from `src` — what the
    /// prototype's state numbering is checked against, and the thing to read
    /// when an emitted machine misbehaves.
    pub fn render(&self, src: &str) -> String {
        let mut out = String::new();
        out.push_str("fields:\n");
        for (i, field) in self.fields.iter().enumerate() {
            let kind = match &field.kind {
                FieldKind::Param(_) => "param".to_string(),
                FieldKind::Local { .. } => "local".to_string(),
                FieldKind::Element { .. } => "element".to_string(),
                FieldKind::Pass { .. } => "pass".to_string(),
            };
            out.push_str(&format!("  {i} {} ({kind})\n", field.name));
        }
        out.push_str("defers:\n");
        for (i, site) in self.defers.iter().enumerate() {
            out.push_str(&format!("  {i} {}\n", one_line(slice(src, site.span))));
        }
        out.push_str("states:\n");
        for (i, state) in self.states.iter().enumerate() {
            out.push_str(&format!("  {i}:\n"));
            render_steps(&state.steps, src, 4, &mut out);
        }
        out.push_str("close:\n");
        render_steps(&self.close, src, 4, &mut out);
        out
    }
}

fn render_steps(steps: &[Step<'_>], src: &str, indent: usize, out: &mut String) {
    let pad = " ".repeat(indent);
    for step in steps {
        match step {
            Step::Plain(stmt) => {
                out.push_str(&format!("{pad}plain `{}`\n", one_line(slice(src, stmt_span(stmt)))));
            }
            Step::Register(i) => out.push_str(&format!("{pad}register d{i}\n")),
            Step::Discharge(i) => out.push_str(&format!("{pad}discharge d{i}\n")),
            Step::OpenPass { slot, subject } => out.push_str(&format!(
                "{pad}open pass {slot} = `{}`\n",
                one_line(slice(src, expr_span(subject)))
            )),
            Step::ClosePass(slot) => out.push_str(&format!("{pad}close pass {slot}\n")),
            Step::Drive {
                slot,
                binding,
                finished,
                ..
            } => {
                out.push_str(&format!("{pad}drive pass {slot} -> field {binding}\n"));
                out.push_str(&format!("{pad}finished:\n"));
                render_steps(finished, src, indent + 2, out);
            }
            Step::Branch { cond, negate, then } => {
                let bang = if *negate { "!" } else { "" };
                out.push_str(&format!(
                    "{pad}if {bang}`{}`:\n",
                    one_line(slice(src, expr_span(cond)))
                ));
                render_steps(then, src, indent + 2, out);
            }
            Step::Goto(t) => out.push_str(&format!("{pad}goto {t}\n")),
            Step::Emit { value, resume } => out.push_str(&format!(
                "{pad}emit `{}` resume {resume}\n",
                one_line(slice(src, expr_span(value)))
            )),
            Step::Finish => out.push_str(&format!("{pad}finish\n")),
        }
    }
}

fn slice(src: &str, span: Span) -> &str {
    src.get(span.start as usize..span.end as usize).unwrap_or("?")
}

/// Collapse a multi-line snippet so one step stays one line.
fn one_line(text: &str) -> String {
    let mut out = String::new();
    let mut space = false;
    for ch in text.chars() {
        if ch.is_whitespace() {
            space = true;
            continue;
        }
        if space && !out.is_empty() {
            out.push(' ');
        }
        space = false;
        out.push(ch);
    }
    out
}

/// A construct the state machine cannot represent yet, reported rather than
/// guessed at [backend-never-wrong].
#[derive(Debug, Clone)]
pub struct GenError {
    pub message: String,
    pub span: Span,
}

/// Whether `decl` is an iterator function: it has a body containing a `yield`
/// [fn-iterator].
pub fn is_generator(decl: &FnDecl) -> bool {
    decl.body.as_ref().is_some_and(block_contains_yield)
}

/// Plan `decl`'s body as a pass. `Err` lists every unsupported construct.
pub fn plan_generator(decl: &FnDecl) -> Result<GeneratorPlan<'_>, Vec<GenError>> {
    let Some(body) = &decl.body else {
        return Err(vec![GenError {
            message: "an iterator function needs a body".to_string(),
            span: decl.span,
        }]);
    };

    let mut planner = Planner {
        fields: Vec::new(),
        defers: Vec::new(),
        states: Vec::new(),
        labels: Vec::new(),
        cleanup_order: Vec::new(),
        cleanups: Vec::new(),
        loops: Vec::new(),
        errors: Vec::new(),
    };

    for param in &decl.params {
        planner.fields.push(GenField {
            name: param.name.name.clone(),
            kind: FieldKind::Param(param),
        });
    }
    planner.check_shadowing(body);

    // The terminal state is referenced from `Finish`-free paths (a `Goto`
    // never targets it, but the emitters need its number), and it is
    // committed last so it comes out with the highest number.
    let finished = planner.new_label();

    let entry = planner.new_label();
    let mut cur = Cursor {
        label: entry,
        buf: Vec::new(),
    };
    cur = planner.block(body, cur);
    cur.push(Step::Finish);
    planner.commit(cur);

    planner.commit(Cursor {
        label: finished,
        buf: vec![Step::Finish],
    });

    if !planner.errors.is_empty() {
        return Err(planner.errors);
    }

    let close = planner.close_path();
    let labels: Vec<usize> = planner
        .labels
        .iter()
        .map(|l| l.unwrap_or(usize::MAX))
        .collect();
    let mut states = planner.states;
    for state in &mut states {
        retarget(&mut state.steps, &labels);
    }
    let mut close = close;
    retarget(&mut close, &labels);

    let mut plan = GeneratorPlan {
        fields: planner.fields,
        defers: planner.defers,
        states,
        finished_state: labels[finished],
        close,
    };
    simplify(&mut plan);
    Ok(plan)
}

/// Something to release on the way out, in the order it was entered: the
/// release path is its reverse.
#[derive(Clone, Copy, Debug)]
enum Cleanup {
    Defer(usize),
    Pass(usize),
}

/// One enclosing flattened loop.
struct LoopCtx {
    /// Where `continue` goes: the loop head.
    head: usize,
    /// Where `break` goes: after the loop.
    exit: usize,
    /// How many cleanups were open when the loop was entered — `break`
    /// releases everything above this plus the loop's own pass, `continue`
    /// everything above this.
    floor: usize,
    /// The pass a `for` drives (released by `break`, kept by `continue`).
    own_pass: Option<usize>,
}

/// The steps accumulated for one state, and the label that will name it.
struct Cursor<'p> {
    label: usize,
    buf: Vec<Step<'p>>,
}

impl<'p> Cursor<'p> {
    fn terminated(&self) -> bool {
        self.buf.last().is_some_and(Step::terminates)
    }

    /// Append a step, unless the state is already over — statements after a
    /// `return`/`break` are unreachable, and the checker reports them.
    fn push(&mut self, step: Step<'p>) {
        if !self.terminated() {
            self.buf.push(step);
        }
    }
}

struct Planner<'p> {
    fields: Vec<GenField<'p>>,
    defers: Vec<DeferSite<'p>>,
    states: Vec<State<'p>>,
    /// Label → state number, filled in as each state is committed.
    labels: Vec<Option<usize>>,
    /// Every cleanup the body ever enters, in declaration order.
    cleanup_order: Vec<Cleanup>,
    /// The cleanups open at the point being flattened.
    cleanups: Vec<Cleanup>,
    loops: Vec<LoopCtx>,
    errors: Vec<GenError>,
}

impl<'p> Planner<'p> {
    fn new_label(&mut self) -> usize {
        self.labels.push(None);
        self.labels.len() - 1
    }

    fn commit(&mut self, cur: Cursor<'p>) {
        debug_assert!(self.labels[cur.label].is_none(), "a label is bound once");
        self.labels[cur.label] = Some(self.states.len());
        self.states.push(State { steps: cur.buf });
    }

    fn error(&mut self, message: impl Into<String>, span: Span) {
        self.errors.push(GenError {
            message: message.into(),
            span,
        });
    }

    fn add_field(&mut self, name: &str, kind: FieldKind<'p>) -> usize {
        self.fields.push(GenField {
            name: name.to_string(),
            kind,
        });
        self.fields.len() - 1
    }

    // ------------------------------------------------------------ blocks

    /// Flatten `block`, returning the cursor where control continues. The
    /// block's own `defer`s are discharged at its end.
    fn block(&mut self, block: &'p Block, mut cur: Cursor<'p>) -> Cursor<'p> {
        let floor = self.cleanups.len();
        for stmt in &block.stmts {
            cur = self.stmt(stmt, cur);
        }
        // The end of the block: its deferred blocks run, latest first.
        for cleanup in self.cleanups[floor..].to_vec().into_iter().rev() {
            if let Cleanup::Defer(i) = cleanup {
                cur.push(Step::Discharge(i));
            }
        }
        self.cleanups.truncate(floor);
        cur
    }

    /// Flatten an *inlinable* block into a nested step list: the arm of a
    /// branch, emitted inside the state it is written in. That is what keeps
    /// a `continue` — or a loop head's exit test — from needing a state of
    /// its own [`block_inlinable`].
    fn nested_block(&mut self, block: &'p Block) -> Vec<Step<'p>> {
        let label = self.new_label();
        let cur = Cursor {
            label,
            buf: Vec::new(),
        };
        let cur = self.block(block, cur);
        debug_assert!(
            self.labels[cur.label].is_none(),
            "an inlinable block commits no state"
        );
        cur.buf
    }

    // -------------------------------------------------------- statements

    fn stmt(&mut self, stmt: &'p Stmt, mut cur: Cursor<'p>) -> Cursor<'p> {
        match stmt {
            Stmt::Yield { value, .. } => {
                let resume = self.new_label();
                cur.push(Step::Emit { value, resume });
                self.commit(cur);
                Cursor {
                    label: resume,
                    buf: Vec::new(),
                }
            }
            Stmt::Defer { body, span } => {
                self.defers.push(DeferSite { body, span: *span });
                let idx = self.defers.len() - 1;
                self.cleanup_order.push(Cleanup::Defer(idx));
                self.cleanups.push(Cleanup::Defer(idx));
                cur.push(Step::Register(idx));
                cur
            }
            Stmt::Break { value, span } => {
                if value.is_some() {
                    self.error(
                        "`break` with a value is not supported in an iterator function",
                        *span,
                    );
                }
                match self.loops.last() {
                    Some(l) => {
                        let (floor, exit, own) = (l.floor, l.exit, l.own_pass);
                        self.release(&mut cur, floor, own);
                        cur.push(Step::Goto(exit));
                    }
                    None => self.error("`break` outside a loop", *span),
                }
                cur
            }
            Stmt::Continue { span } => {
                match self.loops.last() {
                    Some(l) => {
                        let (floor, head) = (l.floor, l.head);
                        self.release(&mut cur, floor, None);
                        cur.push(Step::Goto(head));
                    }
                    None => self.error("`continue` outside a loop", *span),
                }
                cur
            }
            Stmt::Return { value, span } => {
                if value.is_some() {
                    // Already an error in the checker; say it here too rather
                    // than plan something wrong.
                    self.error(
                        "`return` with a value is not allowed in an iterator function",
                        *span,
                    );
                }
                self.release(&mut cur, 0, None);
                cur.push(Step::Finish);
                cur
            }
            Stmt::Expr(Expr::While {
                cond,
                body,
                else_block,
                span,
            }) if stmt_needs_flattening(stmt) => {
                if else_block.is_some() {
                    self.error(
                        "a loop with an `else` block cannot suspend yet: the `else` runs \
                         only if the loop never ran, which the pass has nowhere to record",
                        *span,
                    );
                }
                let head = self.new_label();
                let exit = self.new_label();
                cur.push(Step::Goto(head));
                self.commit(cur);

                let mut head_cur = Cursor {
                    label: head,
                    buf: Vec::new(),
                };
                head_cur.push(Step::Branch {
                    cond,
                    negate: true,
                    then: vec![Step::Goto(exit)],
                });
                self.loops.push(LoopCtx {
                    head,
                    exit,
                    floor: self.cleanups.len(),
                    own_pass: None,
                });
                let mut body_cur = self.block(body, head_cur);
                body_cur.push(Step::Goto(head));
                self.commit(body_cur);
                self.loops.pop();

                Cursor {
                    label: exit,
                    buf: Vec::new(),
                }
            }
            Stmt::Expr(Expr::For {
                pattern,
                iterable,
                body,
                else_block,
                span,
            }) if stmt_needs_flattening(stmt) => {
                if else_block.is_some() {
                    self.error(
                        "a loop with an `else` block cannot suspend yet: the `else` runs \
                         only if the loop never ran, which the pass has nowhere to record",
                        *span,
                    );
                }
                let name = match pattern {
                    Pattern::Ident(id) => id.name.clone(),
                    _ => {
                        self.error(
                            "a destructuring `for` binding is not supported in a suspending \
                             loop yet — bind the element and read its parts",
                            *span,
                        );
                        "__elem".to_string()
                    }
                };
                let binding = self.add_field(
                    &name,
                    FieldKind::Element {
                        subject: iterable,
                        span: *span,
                    },
                );
                let slot = self.add_field(
                    &format!("{name}__pass"),
                    FieldKind::Pass {
                        subject: iterable,
                        span: *span,
                    },
                );
                self.cleanup_order.push(Cleanup::Pass(slot));

                let head = self.new_label();
                let exit = self.new_label();
                cur.push(Step::OpenPass {
                    slot,
                    subject: iterable,
                });
                cur.push(Step::Goto(head));
                self.commit(cur);

                let mut head_cur = Cursor {
                    label: head,
                    buf: Vec::new(),
                };
                head_cur.push(Step::Drive {
                    slot,
                    binding,
                    pattern,
                    subject: iterable,
                    finished: vec![Step::ClosePass(slot), Step::Goto(exit)],
                });
                self.cleanups.push(Cleanup::Pass(slot));
                self.loops.push(LoopCtx {
                    head,
                    exit,
                    floor: self.cleanups.len(),
                    own_pass: Some(slot),
                });
                let mut body_cur = self.block(body, head_cur);
                body_cur.push(Step::Goto(head));
                self.commit(body_cur);
                self.loops.pop();
                self.cleanups.pop();

                Cursor {
                    label: exit,
                    buf: Vec::new(),
                }
            }
            Stmt::Expr(Expr::If {
                branches,
                else_block,
                ..
            }) if stmt_needs_flattening(stmt) => {
                if stmt_inlinable(stmt) {
                    // Every arm ends in a jump, so the arms nest inside this
                    // state and the fall-through is "no branch taken".
                    for (cond, blk) in branches {
                        let then = self.nested_block(blk);
                        cur.push(Step::Branch {
                            cond,
                            negate: false,
                            then,
                        });
                    }
                    if let Some(blk) = else_block {
                        for step in self.nested_block(blk) {
                            cur.push(step);
                        }
                    }
                    cur
                } else {
                    self.flatten_if(branches, else_block.as_ref(), cur)
                }
            }
            Stmt::Let {
                pattern,
                ty,
                value,
                span,
            } => {
                if expr_contains_yield(value) {
                    self.error(
                        "a `yield` in a value position is not supported — yield in statement \
                         position and bind the value separately [iter-generator]",
                        *span,
                    );
                } else if expr_escapes(value, 0) {
                    // A loop in *value* position whose body returns or breaks
                    // out of this statement: the machine would have to
                    // produce the loop's value from a state it jumped out of.
                    self.error(
                        "a `return` or a loop exit inside a value-position expression is not \
                         supported in an iterator function yet — write the loop as a statement \
                         [iter-generator]",
                        *span,
                    );
                }
                match pattern {
                    Pattern::Ident(id) => {
                        self.add_field(
                            &id.name,
                            FieldKind::Local {
                                ty: ty.as_ref(),
                                value,
                                span: *span,
                            },
                        );
                    }
                    _ => self.error(
                        "a destructuring `let` is not supported in a suspending block yet — \
                         bind the value and read its parts",
                        *span,
                    ),
                }
                cur.push(Step::Plain(stmt));
                cur
            }
            other => {
                if stmt_needs_flattening(other) {
                    self.error(unsupported_message(other), stmt_span(other));
                }
                cur.push(Step::Plain(other));
                cur
            }
        }
    }

    /// `if`/`elif`/`else` where an arm suspends, or does not end in a jump.
    /// Needs a join state; only reached from a state cursor, since a block
    /// containing such an `if` is not [`block_inlinable`].
    fn flatten_if(
        &mut self,
        branches: &'p [(Expr, Block)],
        else_block: Option<&'p Block>,
        mut cur: Cursor<'p>,
    ) -> Cursor<'p> {
        let join = self.new_label();
        // The arms are flattened only after this state is committed, so
        // states keep coming out in the order their code is written.
        let mut pending: Vec<(&'p Block, usize)> = Vec::new();
        for (cond, blk) in branches {
            if block_inlinable(blk) {
                let mut then = self.nested_block(blk);
                if !then.last().is_some_and(Step::terminates) {
                    then.push(Step::Goto(join));
                }
                cur.push(Step::Branch {
                    cond,
                    negate: false,
                    then,
                });
            } else {
                // The arm needs states of its own: jump to it, and let the
                // fall-through carry on with the following branches.
                let arm = self.new_label();
                cur.push(Step::Branch {
                    cond,
                    negate: false,
                    then: vec![Step::Goto(arm)],
                });
                pending.push((blk, arm));
            }
        }
        match else_block {
            Some(blk) if block_inlinable(blk) => {
                for step in self.nested_block(blk) {
                    cur.push(step);
                }
                cur.push(Step::Goto(join));
            }
            Some(blk) => {
                let arm = self.new_label();
                cur.push(Step::Goto(arm));
                pending.push((blk, arm));
            }
            None => cur.push(Step::Goto(join)),
        }
        self.commit(cur);

        for (blk, arm) in pending {
            let arm_cur = Cursor {
                label: arm,
                buf: Vec::new(),
            };
            let mut arm_cur = self.block(blk, arm_cur);
            arm_cur.push(Step::Goto(join));
            self.commit(arm_cur);
        }

        Cursor {
            label: join,
            buf: Vec::new(),
        }
    }

    /// Release everything open above `floor` — latest first — on the way out
    /// through a `break`, a `continue` or a `return`. `own_pass` is the pass
    /// of the loop being left, which `break` releases and `continue` keeps.
    fn release(&mut self, cur: &mut Cursor<'p>, floor: usize, own_pass: Option<usize>) {
        for cleanup in self.cleanups[floor..].to_vec().into_iter().rev() {
            match cleanup {
                Cleanup::Defer(i) => cur.push(Step::Discharge(i)),
                Cleanup::Pass(slot) => cur.push(Step::ClosePass(slot)),
            }
        }
        if let Some(slot) = own_pass {
            cur.push(Step::ClosePass(slot));
        }
    }

    /// The release path: everything the body ever opens, in reverse
    /// declaration order. Flags and empty slots make it idempotent, so one
    /// call after the consumer's loop covers `break`, `return` and exhaustion
    /// alike.
    fn close_path(&self) -> Vec<Step<'p>> {
        self.cleanup_order
            .iter()
            .rev()
            .map(|c| match c {
                Cleanup::Defer(i) => Step::Discharge(*i),
                Cleanup::Pass(slot) => Step::ClosePass(*slot),
            })
            .collect()
    }

    /// A local that shadows a parameter or another local would collide in the
    /// pass struct, where every hoisted name is one field. Rejected rather
    /// than renamed: the emitters render a name by looking it up in the field
    /// set, so two fields for one name is how wrong code gets emitted.
    fn check_shadowing(&mut self, body: &'p Block) {
        let mut seen: Vec<String> = self.fields.iter().map(|f| f.name.clone()).collect();
        let mut names = Vec::new();
        collect_bindings(body, &mut names);
        for (name, span) in names {
            if seen.contains(&name) {
                self.error(
                    format!(
                        "`{name}` is declared twice in an iterator function: the body's \
                         locals become fields of one pass, so a shadowing name would \
                         collide — rename it"
                    ),
                    span,
                );
            } else {
                seen.push(name);
            }
        }
    }
}

// ------------------------------------------------------------ simplification

/// Collapse states that only forward (`[Goto(t)]`), drop the unreachable, and
/// renumber. Without this the planner's naive shape carries a state per
/// `yield`-at-the-end-of-a-loop-body and per branch join; with it, the plan is
/// the one the prototypes fixed.
fn simplify(plan: &mut GeneratorPlan<'_>) {
    let n = plan.states.len();

    // 1. Forwarding states. State 0 is the entry and stays where it is.
    // A state that only *finishes* forwards to the terminal one: the end of
    // the fn block, a loop exit with nothing after it and the terminal state
    // are all the same state.
    let mut forward: Vec<Option<usize>> = vec![None; n];
    for (i, state) in plan.states.iter().enumerate() {
        if i == 0 || i == plan.finished_state {
            continue;
        }
        match state.steps[..] {
            [Step::Goto(t)] => forward[i] = Some(t),
            [Step::Finish] => forward[i] = Some(plan.finished_state),
            _ => {}
        }
    }
    let mut target: Vec<usize> = (0..n).collect();
    for s in 0..n {
        let mut at = s;
        let mut guard = 0;
        while let Some(t) = forward[at] {
            if t == at || guard > n {
                break;
            }
            at = t;
            guard += 1;
        }
        target[s] = at;
    }
    for state in &mut plan.states {
        retarget(&mut state.steps, &target);
    }
    retarget(&mut plan.close, &target);
    plan.finished_state = target[plan.finished_state];

    // 2. Reachability, from the entry state and the terminal one.
    let mut live = vec![false; n];
    let mut stack = vec![0usize, plan.finished_state];
    while let Some(s) = stack.pop() {
        if live[s] {
            continue;
        }
        live[s] = true;
        collect_targets(&plan.states[s].steps, &mut stack);
    }

    // 3. Renumber, preserving order.
    let mut renumber = vec![usize::MAX; n];
    let mut next = 0;
    for s in 0..n {
        if live[s] {
            renumber[s] = next;
            next += 1;
        }
    }
    let mut states = Vec::with_capacity(next);
    for (s, state) in plan.states.drain(..).enumerate() {
        if live[s] {
            states.push(state);
        }
    }
    plan.states = states;
    for state in &mut plan.states {
        retarget(&mut state.steps, &renumber);
    }
    retarget(&mut plan.close, &renumber);
    plan.finished_state = renumber[plan.finished_state];
}

fn retarget(steps: &mut [Step<'_>], map: &[usize]) {
    for step in steps {
        match step {
            Step::Goto(t) => *t = map[*t],
            Step::Emit { resume, .. } => *resume = map[*resume],
            Step::Branch { then, .. } => retarget(then, map),
            Step::Drive { finished, .. } => retarget(finished, map),
            _ => {}
        }
    }
}

fn collect_targets(steps: &[Step<'_>], out: &mut Vec<usize>) {
    for step in steps {
        match step {
            Step::Goto(t) => out.push(*t),
            Step::Emit { resume, .. } => out.push(*resume),
            Step::Branch { then, .. } => collect_targets(then, out),
            Step::Drive { finished, .. } => collect_targets(finished, out),
            _ => {}
        }
    }
}

// ------------------------------------------------------------- predicates

/// Whether a statement has to be flattened into states: it either suspends
/// (`yield`) or jumps out of itself (`return`, or a `break`/`continue` bound
/// by a loop further out). Everything else stays a [`Step::Plain`].
fn stmt_needs_flattening(stmt: &Stmt) -> bool {
    stmt_escapes(stmt, 0)
}

fn stmt_escapes(stmt: &Stmt, loop_depth: usize) -> bool {
    match stmt {
        Stmt::Yield { .. } | Stmt::Return { .. } => true,
        Stmt::Break { .. } | Stmt::Continue { .. } => loop_depth == 0,
        Stmt::Expr(e) => expr_escapes(e, loop_depth),
        // A deferred block cannot leave its own body [defer-no-escape].
        Stmt::Defer { .. } => false,
        // A *value* can escape too: `let x = while … { … return … }` puts a
        // loop in value position, and everything in it is as much a jump out
        // of this statement as a bare one would be. Missing this is how a
        // `return` leaks into the machine as a target-language `return`.
        Stmt::Let { value, .. } => expr_escapes(value, loop_depth),
        Stmt::Assign { target, value, .. } => {
            expr_escapes(target, loop_depth) || expr_escapes(value, loop_depth)
        }
        Stmt::Use { handler, .. } => expr_escapes(handler, loop_depth),
        Stmt::Rename(_) => false,
    }
}

fn expr_escapes(expr: &Expr, loop_depth: usize) -> bool {
    match expr {
        Expr::If {
            branches,
            else_block,
            ..
        } => {
            branches.iter().any(|(_, b)| block_escapes(b, loop_depth))
                || else_block
                    .as_ref()
                    .is_some_and(|b| block_escapes(b, loop_depth))
        }
        Expr::When { branches, .. } => branches
            .iter()
            .any(|b| block_escapes(&b.body, loop_depth)),
        Expr::WhenCond {
            branches,
            else_block,
            ..
        } => {
            branches.iter().any(|(_, b)| block_escapes(b, loop_depth))
                || block_escapes(else_block, loop_depth)
        }
        Expr::While {
            body, else_block, ..
        }
        | Expr::For {
            body, else_block, ..
        } => {
            block_escapes(body, loop_depth + 1)
                || else_block
                    .as_ref()
                    .is_some_and(|b| block_escapes(b, loop_depth))
        }
        Expr::Try { body, .. } => block_escapes(body, loop_depth),
        _ => false,
    }
}

fn block_escapes(block: &Block, loop_depth: usize) -> bool {
    block.stmts.iter().any(|s| stmt_escapes(s, loop_depth))
}

/// Whether a statement can be planned as *nested* steps inside another
/// state's step list, rather than needing states of its own. A jump
/// (`break`/`continue`/`return`) can; a suspension or a suspending loop
/// cannot; an `if` can when every arm can and every arm ends in a jump — the
/// nested form has no join to fall through to.
fn stmt_inlinable(stmt: &Stmt) -> bool {
    if !stmt_needs_flattening(stmt) {
        // A `Plain` step: one statement to the emitter, brace to brace.
        return true;
    }
    match stmt {
        Stmt::Break { .. } | Stmt::Continue { .. } | Stmt::Return { .. } => true,
        Stmt::Expr(Expr::If {
            branches,
            else_block,
            ..
        }) => {
            branches
                .iter()
                .all(|(_, b)| block_inlinable(b) && block_terminates(b))
                && else_block
                    .as_ref()
                    .is_none_or(|b| block_inlinable(b) && block_terminates(b))
        }
        _ => false,
    }
}

fn block_inlinable(block: &Block) -> bool {
    block.stmts.iter().all(stmt_inlinable)
}

/// Whether every path through a block leaves it (so nothing can fall out of
/// the end).
fn block_terminates(block: &Block) -> bool {
    match block.stmts.last() {
        Some(Stmt::Break { .. }) | Some(Stmt::Continue { .. }) | Some(Stmt::Return { .. }) => true,
        Some(Stmt::Expr(Expr::If {
            branches,
            else_block: Some(els),
            ..
        })) => branches.iter().all(|(_, b)| block_terminates(b)) && block_terminates(els),
        _ => false,
    }
}

/// Why a flattened statement could not be planned. Reached only for shapes
/// the machine does not represent yet: the supported ones are handled before
/// this is asked.
fn unsupported_message(stmt: &Stmt) -> String {
    let what = match stmt {
        Stmt::Expr(Expr::When { .. }) | Stmt::Expr(Expr::WhenCond { .. }) => {
            "a `when` containing a `yield`, a `return` or a loop exit"
        }
        Stmt::Expr(Expr::Try { .. }) => "a `try` block containing a `yield` or a loop exit",
        Stmt::Assign { .. } => "an assignment whose value contains a `yield`",
        Stmt::Use { .. } => "a `use` whose handler contains a `yield`",
        _ => "this construct",
    };
    format!(
        "{what} is not supported in an iterator function yet — restructure it with `if` \
         [iter-generator]"
    )
}

fn stmt_span(stmt: &Stmt) -> Span {
    match stmt {
        Stmt::Let { span, .. }
        | Stmt::Assign { span, .. }
        | Stmt::Return { span, .. }
        | Stmt::Break { span, .. }
        | Stmt::Continue { span }
        | Stmt::Yield { span, .. }
        | Stmt::Use { span, .. }
        | Stmt::Defer { span, .. } => *span,
        Stmt::Rename(r) => r.span,
        Stmt::Expr(e) => expr_span(e),
    }
}

fn expr_span(expr: &Expr) -> Span {
    use Expr::*;
    match expr {
        Int { span, .. }
        | Float { span, .. }
        | Bool { span, .. }
        | Char { span, .. }
        | Str { span, .. }
        | Field { span, .. }
        | TupleIndex { span, .. }
        | Scoped { span, .. }
        | Call { span, .. }
        | Index { span, .. }
        | ArrayLit { span, .. }
        | ArrayInit { span, .. }
        | Tuple { span, .. }
        | StructLit { span, .. }
        | Unary { span, .. }
        | Binary { span, .. }
        | Is { span, .. }
        | Widen { span, .. }
        | NonNull { span, .. }
        | PostIncrement { span, .. }
        | If { span, .. }
        | When { span, .. }
        | WhenCond { span, .. }
        | While { span, .. }
        | For { span, .. }
        | Lambda { span, .. }
        | Try { span, .. }
        | Spread { span, .. }
        | Error { span } => *span,
        Ident(id) => id.span,
    }
}

/// Whether a block contains a `yield` of its own. Lambdas are their own
/// functions and are not descended into; neither are deferred blocks, which
/// cannot contain a `yield` [defer-no-escape].
pub fn block_contains_yield(block: &Block) -> bool {
    block.stmts.iter().any(|stmt| match stmt {
        Stmt::Yield { .. } => true,
        Stmt::Expr(e) => expr_contains_yield(e),
        _ => false,
    })
}

fn expr_contains_yield(expr: &Expr) -> bool {
    match expr {
        Expr::If {
            branches,
            else_block,
            ..
        } => {
            branches.iter().any(|(_, b)| block_contains_yield(b))
                || else_block.as_ref().is_some_and(block_contains_yield)
        }
        Expr::When { branches, .. } => branches.iter().any(|b| block_contains_yield(&b.body)),
        Expr::WhenCond {
            branches,
            else_block,
            ..
        } => {
            branches.iter().any(|(_, b)| block_contains_yield(b))
                || block_contains_yield(else_block)
        }
        Expr::While {
            body, else_block, ..
        }
        | Expr::For {
            body, else_block, ..
        } => block_contains_yield(body) || else_block.as_ref().is_some_and(block_contains_yield),
        Expr::Try { body, .. } => block_contains_yield(body),
        _ => false,
    }
}

/// Every name a body binds, including inside nested blocks and lambda
/// bodies — the shadowing check wants all of them, since the emitters decide
/// "field or local" by name.
fn collect_bindings(block: &Block, out: &mut Vec<(String, Span)>) {
    for stmt in &block.stmts {
        match stmt {
            Stmt::Let { pattern, value, .. } => {
                collect_pattern(pattern, out);
                collect_bindings_expr(value, out);
            }
            Stmt::Assign { value, .. } => collect_bindings_expr(value, out),
            Stmt::Return { value, .. } | Stmt::Break { value, .. } => {
                if let Some(v) = value {
                    collect_bindings_expr(v, out);
                }
            }
            Stmt::Yield { value, .. } => collect_bindings_expr(value, out),
            Stmt::Use { handler, .. } => collect_bindings_expr(handler, out),
            Stmt::Defer { body, .. } => collect_bindings(body, out),
            Stmt::Expr(e) => collect_bindings_expr(e, out),
            Stmt::Continue { .. } | Stmt::Rename(_) => {}
        }
    }
}

fn collect_pattern(pattern: &Pattern, out: &mut Vec<(String, Span)>) {
    match pattern {
        Pattern::Ident(id) => out.push((id.name.clone(), id.span)),
        Pattern::Tuple { elems, .. } => {
            for p in elems {
                collect_pattern(p, out);
            }
        }
        Pattern::Struct { fields, .. } => {
            for f in fields {
                out.push((f.binding.name.clone(), f.binding.span));
            }
        }
    }
}

fn collect_bindings_expr(expr: &Expr, out: &mut Vec<(String, Span)>) {
    match expr {
        Expr::If {
            branches,
            else_block,
            ..
        } => {
            for (c, b) in branches {
                collect_bindings_expr(c, out);
                collect_bindings(b, out);
            }
            if let Some(b) = else_block {
                collect_bindings(b, out);
            }
        }
        Expr::When { branches, .. } => {
            for b in branches {
                collect_bindings(&b.body, out);
            }
        }
        Expr::WhenCond {
            branches,
            else_block,
            ..
        } => {
            for (c, b) in branches {
                collect_bindings_expr(c, out);
                collect_bindings(b, out);
            }
            collect_bindings(else_block, out);
        }
        Expr::While {
            cond,
            body,
            else_block,
            ..
        } => {
            collect_bindings_expr(cond, out);
            collect_bindings(body, out);
            if let Some(b) = else_block {
                collect_bindings(b, out);
            }
        }
        Expr::For {
            pattern,
            iterable,
            body,
            else_block,
            ..
        } => {
            collect_pattern(pattern, out);
            collect_bindings_expr(iterable, out);
            collect_bindings(body, out);
            if let Some(b) = else_block {
                collect_bindings(b, out);
            }
        }
        Expr::Try { body, .. } => collect_bindings(body, out),
        Expr::Lambda { body, .. } => match body {
            LambdaBody::Expr(e) => collect_bindings_expr(e, out),
            LambdaBody::Block(b) => collect_bindings(b, out),
        },
        // An `is` binding names a value for one branch; it is a local like
        // any other as far as collisions go.
        Expr::Is {
            subject, binding, ..
        } => {
            collect_bindings_expr(subject, out);
            if let Some(b) = binding {
                out.push((b.name.clone(), b.span));
            }
        }
        Expr::Call { args, named, .. } => {
            for a in args {
                collect_bindings_expr(a, out);
            }
            for n in named {
                collect_bindings_expr(&n.value, out);
            }
        }
        Expr::Binary { lhs, rhs, .. } => {
            collect_bindings_expr(lhs, out);
            collect_bindings_expr(rhs, out);
        }
        Expr::Unary { operand, .. }
        | Expr::NonNull { operand, .. }
        | Expr::PostIncrement { operand, .. }
        | Expr::Spread { operand, .. } => collect_bindings_expr(operand, out),
        _ => {}
    }
}
