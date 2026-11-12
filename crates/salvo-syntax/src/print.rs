//! [ast-print] A module's tree printed back as Salvo source.
//!
//! The printer reads a `Module` — as parsed, or after the expansions and
//! desugarings a program's driver runs — and writes the source a reader
//! would have written for it: one declaration per paragraph, four-space
//! indentation, every expression on one line except the blocks it holds.
//! Layout is the printer's, not the file's, but for what the module's trivia
//! keeps: every comment, placed among the nodes by its offset
//! [fmt-comments], and the blank lines written between statements, members
//! and declarations [fmt-blank-lines].
//!
//! It also records where it printed every node whose span is numbered
//! [node-id] (`Printed::nodes`, keyed by the node's file tag and ordinal), so a
//! dump that annotates nodes places its annotations in the printed text by
//! node, not by the node's offset in the file it came from — which is what
//! lets a dump show code an expansion wrote (a comptime stamp's body, a
//! hidden fn), whose spans are its template's.
//!
//! The parser counts the grouping parentheses written [expr-group]; the
//! printer writes them, and adds a pair where precedence needs one (the
//! parser's binding powers); an expression's recorded `outer` range
//! includes them.
//!
//! A module as the parser wrote it (`parse_module`) prints as
//! written [syntax-lowered]: its sugar (`ast::Sugar`, `x += e`), its terminal
//! `...` bodies (`Block::terminal`), its namespaces and its comptime
//! forms, so the text parses back to the same tree — the formatter's half
//! of the work. Once lowered, a piece of sugar prints as what it became, with
//! a comment (or, for an expression in a dump, a mark) giving it as written,
//! found by the trace the lowering leaves (`sugar_origin`,
//! `compound_origin`, `as_qual_name`).
//!
//! What the tree holds with no surface spelling of its own is printed as the
//! surface it came from where the tree says so (a type-named call
//! `Mut_List(…)` to `Mut List(…)`), and otherwise marked `#{…}`. A lowered
//! `?:`, `?.` or `?>` is printed by the [`Reading`] the printing is for: as
//! its operator for source that parses back, as the `when` it became for a
//! dump (user decision 2026-11-09).

use std::collections::HashMap;
use std::fmt::Write;

use crate::ast::*;
use crate::parser::{EFF_TYPE_NAME, SITE_SELECTOR};
use crate::span::Span;
use crate::trivia::{Trivia, TriviaComment};

/// [ast-print] Where a node was printed: its own text `start..end`, and for
/// an expression `outer_start..outer_end` with the parentheses the printer
/// put round it (the same range when there are none).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Placed {
    pub start: u32,
    pub end: u32,
    pub outer_start: u32,
    pub outer_end: u32,
}

/// [ast-print] A node's key in [`Printed::nodes`]: its file's tag and its
/// ordinal there ([node-file] [node-id]).
pub type NodeKey = (u32, u32);

/// The key of the node `span` holds; `None` for a span no numbered node has.
pub fn node_key(span: Span) -> Option<NodeKey> {
    span.is_numbered().then_some((span.file, span.node))
}

/// [ast-print] A printed module: the text, and where each numbered node of
/// it was printed.
#[derive(Default)]
pub struct Printed {
    pub text: String,
    pub nodes: HashMap<NodeKey, Placed>,
}

impl Printed {
    /// Where the node `span` holds was printed.
    pub fn at(&self, span: Span) -> Option<Placed> {
        node_key(span).and_then(|k| self.nodes.get(&k).copied())
    }

    /// The text the node `span` holds was printed as.
    pub fn text_of(&self, span: Span) -> Option<&str> {
        self.at(span).map(|p| &self.text[p.start as usize..p.end as usize])
    }
}

/// [ast-print] What a caller adds to the printing: a leading comment for a
/// declaration (what produced it), and whether a declaration is printed at
/// all.
pub trait Notes {
    /// The comment lines (without `//`) to print above `f`, after its docs.
    fn fn_note(&self, _f: &FnDecl) -> Vec<String> {
        Vec::new()
    }
    /// The comment lines to print above a struct.
    fn struct_note(&self, _s: &StructDecl) -> Vec<String> {
        Vec::new()
    }
    /// How `f` is printed: from its tree, as given text, or not at all.
    fn fn_source(&self, _f: &FnDecl) -> FnSource {
        FnSource::Tree
    }
    /// [comptime-default] The expression a `FieldDefault` stands for, to
    /// print in its place; `None` marks it `#{…}`.
    fn field_default(&self, _owner: &Type, _field: &Ident) -> Option<Expr> {
        None
    }
}

/// [ast-print] How a fn is printed ([`Notes::fn_source`]).
pub enum FnSource {
    /// From its tree.
    Tree,
    /// As this text (its docs and `export` printed before it): the source
    /// it was written as, for a printing that is expanded again.
    Text(String),
    /// Not at all: a printing expanded again makes it anew.
    Omit,
}

/// No notes.
pub struct NoNotes;
impl Notes for NoNotes {}

/// [ast-print] What a printing is for, which decides how a lowered `?:`,
/// `?.` or `?>` [when-lowered] is written (user decision 2026-11-09).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reading {
    /// Source that parses back to the same tree [round-trip]: the operator,
    /// its temporary read as `_` (or not written at all).
    Source,
    /// A dump, for reading what each step decided [dump-resolve]
    /// [dump-type-check]: the `when` the operator became, its binding
    /// temporary, both arms with their heads marked `#{first arm}` and
    /// `#{rest}`, and a comment naming the operator.
    Dump,
}

/// [ast-print] `module` as source, with every node's place recorded.
pub fn print_module(module: &Module) -> Printed {
    print_module_with(module, &NoNotes, Reading::Source)
}

/// [ast-print] [`print_module`] with `notes` adding comments, for `reading`.
pub fn print_module_with<'a>(module: &'a Module, notes: &'a dyn Notes, reading: Reading) -> Printed {
    let mut p = Printer::new(notes, reading);
    if !module.trivia.is_empty() && (!module.trivia.lowered || reading == Reading::Dump) {
        p.trivia = Some(&module.trivia);
        p.file = module.file_tag;
    }
    p.module(module);
    Printed { text: p.out, nodes: p.nodes }
}

/// [ast-print] One expression as source (no node map): for messages and
/// tests.
pub fn expr_text(e: &Expr) -> String {
    let mut p = Printer::new(&NoNotes, Reading::Source);
    p.expr(e, 0);
    p.out
}

/// [ast-print] One type as source.
pub fn type_text(t: &Type) -> String {
    let mut p = Printer::new(&NoNotes, Reading::Source);
    p.ty(t);
    p.out
}

/// [when-lowered] The mark a dump prints for an arm hole: `first arm`,
/// `rest` (inside `#{…}`).
pub fn hole_mark(hole: ArmHole) -> &'static str {
    match hole {
        ArmHole::First => "first arm",
        ArmHole::Rest => "rest",
    }
}

/// [ast-print] A string literal's text, quoted and escaped as Salvo reads it:
/// `\n`, `\t`, `\r`, `\0`, `\\`, `\"`, and `\$` before a `{` (so the text
/// is no interpolation).
pub fn quote_str(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    escape_str_into(text, &mut out);
    out.push('"');
    out
}

/// [ast-print] A string literal's text, escaped as the printer writes it
/// (without the quotes).
pub fn escape_str(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    escape_str_into(text, &mut out);
    out
}

fn escape_str_into(text: &str, out: &mut String) {
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            '\0' => out.push_str("\\0"),
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '$' if chars.peek() == Some(&'{') => out.push_str("\\$"),
            c => out.push(c),
        }
    }
}

/// [ast-print] A char literal, quoted and escaped as Salvo reads it.
pub fn quote_char(c: char) -> String {
    match c {
        '\n' => "'\\n'".into(),
        '\t' => "'\\t'".into(),
        '\r' => "'\\r'".into(),
        '\0' => "'\\0'".into(),
        '\\' => "'\\\\'".into(),
        '\'' => "'\\''".into(),
        c => format!("'{c}'"),
    }
}

/// [ast-print] A floating literal as Salvo writes it: always with a fraction,
/// `f` for a `Float`.
pub fn float_text(value: f64, single: bool) -> String {
    let mut s = if value.is_finite() { format!("{value}") } else { format!("{value:?}") };
    if value.is_finite() && !s.contains('.') {
        s.push_str(".0");
    }
    if single {
        s.push('f');
    }
    s
}

/// [ast-print] The symbol of a binary operator, as written.
pub fn binary_symbol(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::Add => "+",
        BinaryOp::Sub => "-",
        BinaryOp::Mul => "*",
        BinaryOp::Div => "/",
        BinaryOp::Rem => "%",
        BinaryOp::Pow => "**",
        BinaryOp::Eq => "==",
        BinaryOp::NotEq => "!=",
        BinaryOp::Lt => "<",
        BinaryOp::Gt => ">",
        BinaryOp::LtEq => "<=",
        BinaryOp::GtEq => ">=",
        BinaryOp::And => "&&",
        BinaryOp::Or => "||",
    }
}

/// The parser's binding power of a binary operator (`infix_bp`); `**` is
/// tighter than every prefix operator.
fn binary_level(op: BinaryOp) -> u8 {
    match op {
        BinaryOp::Or => 4,
        BinaryOp::And => 6,
        BinaryOp::Eq | BinaryOp::NotEq => 8,
        BinaryOp::Lt | BinaryOp::Gt | BinaryOp::LtEq | BinaryOp::GtEq => 10,
        BinaryOp::Add | BinaryOp::Sub => 14,
        BinaryOp::Mul | BinaryOp::Div | BinaryOp::Rem => 16,
        BinaryOp::Pow => POWER,
    }
}

/// Precedence levels, loosest first: what an expression needs its context
/// to accept without parentheses.
const PIPE: u8 = 2;
const COMPARE: u8 = 10;
const ELVIS: u8 = 12;
const PREFIX: u8 = 18;
const POWER: u8 = 19;
const POSTFIX: u8 = 20;
const PRIMARY: u8 = 21;

/// [fn-type-named] The callee of a type-named call, `Mut_List` for `Mut
/// List(…)`, as written: the key's parts space-joined. `None` for an
/// ordinary callee.
fn type_fn_callee(name: &str) -> Option<String> {
    let first = name.trim_start_matches('.').chars().next()?;
    if !first.is_uppercase() || !name.contains('_') {
        return None;
    }
    Some(name.split('_').collect::<Vec<_>>().join(" "))
}

struct Printer<'n> {
    out: String,
    nodes: HashMap<NodeKey, Placed>,
    indent: usize,
    notes: &'n dyn Notes,
    /// [struct-lit-condition] In a condition, where a struct literal must be
    /// parenthesized.
    no_struct: bool,
    /// [when-lowered] The names a lowered pick, `?.` or `?>` bound for its
    /// subject, printed `_` (or not at all) in the operator they came from.
    holes: Vec<String>,
    /// [interp-nested-str] Inside a `${…}`, which is one line: a block is
    /// written on it.
    inline: bool,
    /// What the printing is for.
    reading: Reading,
    /// [syntax-lowered] Inside an expression a dump has marked with the sugar
    /// it came from: its parts are not marked again.
    quiet_origin: bool,
    /// [expr-terminal-scope] The terminal body the statement being printed
    /// holds, printed after it.
    pending_rest: Option<Block>,
    /// [fmt-comments] The module's trivia, when it has any: its comments
    /// are printed among the nodes, and its blank lines kept.
    trivia: Option<&'n Trivia>,
    /// [node-file] The module's file tag: a node of another file (or of an
    /// expansion) is placed by no comment.
    file: u32,
    /// [fmt-comments] The first comment of `trivia` not yet printed.
    next_comment: usize,
    /// [fmt-comments] The current line already ends in a comment.
    eol: bool,
}

impl<'n> Printer<'n> {
    fn new(notes: &'n dyn Notes, reading: Reading) -> Self {
        Printer {
            out: String::new(),
            nodes: HashMap::new(),
            indent: 0,
            notes,
            no_struct: false,
            holes: Vec::new(),
            inline: false,
            reading,
            quiet_origin: false,
            pending_rest: None,
            trivia: None,
            file: crate::span::NO_FILE,
            next_comment: 0,
            eol: false,
        }
    }
}

/// [fmt-comments] [fmt-blank-lines] Where a run of elements (a body's
/// statements, declarations, members or fields, or a broken list's items)
/// has got to.
#[derive(Default)]
struct Run {
    /// Something of the run is printed: a blank line may come before the
    /// next (never at the start of a body).
    started: bool,
    /// Where the last thing printed ended in the source, when it was
    /// placed there: what the next one's blank line is measured from.
    prev_end: Option<u32>,
    /// The line printed last is a comment of its own line, which a
    /// declaration directly under it would take as its docs.
    comment_last: bool,
    /// A list's run: no blank lines between its items.
    no_blanks: bool,
}

impl<'n> Printer<'n> {
    fn at(&self) -> u32 {
        self.out.len() as u32
    }

    fn push(&mut self, s: &str) {
        self.out.push_str(s);
    }

    fn record(&mut self, span: Span, start: u32, end: u32, outer_start: u32, outer_end: u32) {
        if let Some(k) = node_key(span) {
            // The first printing of a node places it; one around it (an
            // identifier's, inside the expression's parentheses) reaches out
            // to those parentheses [expr-group].
            self.nodes
                .entry(k)
                .and_modify(|p| {
                    if outer_start <= p.outer_start && p.outer_end <= outer_end {
                        p.outer_start = outer_start;
                        p.outer_end = outer_end;
                    }
                })
                .or_insert(Placed { start, end, outer_start, outer_end });
        }
    }

    fn newline(&mut self) {
        // No trailing spaces on a blank line; a comment's own are its text,
        // printed as written [fmt-comments].
        while !self.eol && self.out.ends_with(' ') {
            self.out.pop();
        }
        self.out.push('\n');
        for _ in 0..self.indent {
            self.out.push_str("    ");
        }
        self.eol = false;
    }

    // --- Comments and blank lines [fmt-comments] [fmt-blank-lines] ---

    /// Whether a comment may be placed by `span`: the printing has trivia,
    /// and the node is written in this file (not copied by an expansion, not
    /// made by a pass), outside a one-line `${…}`.
    fn anchored(&self, span: Span) -> bool {
        self.trivia.is_some() && !self.inline && span.is_written() && span.file == self.file
    }

    /// The first comment not printed yet, when it starts before `at`.
    fn comment_before(&self, at: u32) -> Option<&'n TriviaComment> {
        let trivia: &'n Trivia = self.trivia?;
        trivia.comments.get(self.next_comment).filter(|c| c.start < at)
    }

    /// Whether the current line holds nothing but its indentation.
    fn line_is_fresh(&self) -> bool {
        self.out.rsplit('\n').next().is_some_and(|l| l.bytes().all(|b| b == b' '))
    }

    /// Starts a line of its own for what comes next, with one blank line
    /// before it when `blank` (and none at the very start of the text).
    fn fresh_line(&mut self, blank: bool) {
        if self.out.is_empty() {
            return;
        }
        if !self.line_is_fresh() {
            self.newline();
        }
        if blank && !self.out.trim_end_matches(' ').ends_with("\n\n") {
            self.newline();
        }
    }

    /// Prints comment `c` as written: at the end of the line printed last
    /// when it was written after code and that line holds no comment yet,
    /// otherwise on a line of its own (after a blank line when `blank`).
    /// Whether it took a line of its own.
    fn put_comment(&mut self, c: &TriviaComment, blank: bool) -> bool {
        self.next_comment += 1;
        if !c.own_line && !self.eol && !self.line_is_fresh() {
            self.push(" ");
            self.push(&c.text);
            self.eol = true;
            return false;
        }
        self.fresh_line(blank);
        self.push(&c.text);
        self.eol = true;
        true
    }

    /// The comments before `at` not printed yet, each where `put_comment`
    /// puts it; `forced`: a blank line before the first that takes a line.
    fn comments_before(&mut self, run: &mut Run, at: u32, mut forced: bool) -> bool {
        let mut any_own = false;
        while let Some(c) = self.comment_before(at) {
            let blank = run.started && !run.no_blanks && (forced || run.prev_end.is_some_and(|p| self.blank_between(p, c.start)));
            if self.put_comment(c, blank) {
                forced = false;
                any_own = true;
                run.comment_last = true;
                run.started = true;
            }
            run.prev_end = Some(c.end);
        }
        any_own
    }

    fn blank_between(&self, from: u32, to: u32) -> bool {
        self.trivia.is_some_and(|t| t.blank_between(from, to))
    }

    /// Before an element of a run, at `span`: the comments written before
    /// it, then a line of its own, after one blank line where the source
    /// had one or more between it and what came before. `forced(source)`:
    /// the blank line the layout wants whatever was written (`source`: the
    /// written one decides). `takes_docs`: an element a comment directly
    /// above would document, which a comment printed there must not be.
    fn elem_start(&mut self, run: &mut Run, span: Span, takes_docs: bool, forced: impl FnOnce(bool) -> bool) {
        let anchored = self.anchored(span);
        let source = anchored && run.prev_end.is_some();
        let want = run.started && !run.no_blanks && forced(source);
        let mut blank = want;
        if anchored && self.comments_before(run, span.start, want) {
            blank = false;
        }
        if anchored && !run.no_blanks && run.started {
            blank |= run.prev_end.is_some_and(|p| self.blank_between(p, span.start));
        }
        blank |= takes_docs && run.comment_last;
        let blank = blank && run.started && !run.no_blanks;
        if run.started || !self.line_is_fresh() {
            self.fresh_line(blank);
        }
        run.comment_last = false;
        run.started = true;
    }

    /// After an element of a run, at `span`: a comment written after it on
    /// its last line, and one written inside it that no slot of it took.
    fn elem_end(&mut self, run: &mut Run, span: Span) {
        run.started = true;
        if !self.anchored(span) {
            run.prev_end = None;
            return;
        }
        let Some(trivia) = self.trivia else { return };
        let line = trivia.line_of(span.end);
        while let Some(c) = trivia.comments.get(self.next_comment) {
            if !(c.start < span.end || (!c.own_line && trivia.line_of(c.start) == line)) {
                break;
            }
            if self.put_comment(c, false) {
                run.comment_last = true;
            }
        }
        run.prev_end = Some(span.end);
    }

    /// The comments that close a run, written before `close` (a body's `}`,
    /// a list's bracket), each on its line. Whether there were any.
    fn run_close(&mut self, run: &mut Run, close: u32) -> bool {
        self.comments_before(run, close, false)
    }

    /// Whether a comment not printed yet lies in `from..to` outside every
    /// one of `items`: a comment the list itself holds, which breaks it.
    fn list_holds_comment(&self, from: u32, to: u32, items: &[Span]) -> bool {
        let Some(trivia) = self.trivia else { return false };
        if self.inline {
            return false;
        }
        trivia.comments[self.next_comment.min(trivia.comments.len())..]
            .iter()
            .take_while(|c| c.start < to)
            .any(|c| c.start >= from && !items.iter().any(|i| i.start <= c.start && c.start < i.end))
    }

    /// [fmt-comments] A list a comment is written in, one item per line
    /// (after the opening bracket the caller printed; the caller prints the
    /// closing one): each item's comments around it, `,` between the items,
    /// and the comments before `close` on their lines.
    fn broken_list<T>(&mut self, items: &[T], span_of: impl Fn(&T) -> Span, close: u32, mut print: impl FnMut(&mut Self, &T)) {
        self.indent += 1;
        let mut run = Run { no_blanks: true, ..Run::default() };
        for (i, it) in items.iter().enumerate() {
            let sp = span_of(it);
            self.elem_start(&mut run, sp, false, |_| false);
            print(self, it);
            if i + 1 < items.len() {
                self.push(",");
            }
            self.elem_end(&mut run, sp);
        }
        self.run_close(&mut run, close);
        self.indent -= 1;
        self.newline();
    }

    /// The comments not printed yet, at the end of the text: none is lost.
    fn rest_of_comments(&mut self) {
        let mut run = Run { started: true, ..Run::default() };
        self.comments_before(&mut run, u32::MAX, false);
    }

    /// The source offset of the first `close` bracket at or after `offset`,
    /// when the printing has trivia.
    fn close_after(&self, offset: u32, close: char) -> Option<u32> {
        self.trivia.and_then(|t| t.close_after(offset, close))
    }

    fn comment_lines(&mut self, lines: &[String]) {
        for l in lines {
            if l.is_empty() {
                self.push("//");
            } else {
                self.push("// ");
                self.push(l);
            }
            self.eol = true;
            self.newline();
        }
    }

    /// [doc-comment] A declaration's docs: as written, when the printing
    /// has the file's trivia and finds them above `span`; otherwise each
    /// line after `// `.
    fn docs(&mut self, docs: &[String], span: Span) {
        if docs.is_empty() {
            return;
        }
        let written = self.trivia.filter(|_| self.anchored(span)).and_then(|t| t.docs_as_written(docs, span.start, false));
        match written {
            Some(lines) => self.written_lines(&lines),
            None => self.comment_lines(docs),
        }
    }

    /// Comment lines as written, each followed by a newline.
    fn written_lines(&mut self, lines: &[&str]) {
        for l in lines {
            self.push(l);
            self.eol = true;
            self.newline();
        }
    }

    fn ident(&mut self, id: &Ident) {
        let s = self.at();
        self.push(&id.name);
        let e = self.at();
        self.record(id.span, s, e, s, e);
    }

    /// An identifier printed as `text` (its surface spelling).
    fn ident_as(&mut self, id: &Ident, text: &str) {
        let s = self.at();
        self.push(text);
        let e = self.at();
        self.record(id.span, s, e, s, e);
    }

    fn list<T>(&mut self, items: &[T], sep: &str, mut f: impl FnMut(&mut Self, &T)) {
        for (i, x) in items.iter().enumerate() {
            if i > 0 {
                self.push(sep);
            }
            f(self, x);
        }
    }

    // --- Module and items ---

    fn module(&mut self, m: &Module) {
        if !m.docs.is_empty() {
            match self.trivia.and_then(|t| t.docs_as_written(&m.docs, 0, true)) {
                Some(written) => self.written_lines(&written),
                None => self.comment_lines(&m.docs),
            }
            self.newline();
        }
        // [fn-attached] A struct's fns go back in its body, and a named
        // union's [type-union-body].
        let mut attached: HashMap<&str, Vec<&FnDecl>> = HashMap::new();
        let structs: std::collections::HashSet<&str> = m
            .items
            .iter()
            .filter_map(|i| match i {
                Item::Struct(s) => Some(s.name.name.as_str()),
                Item::Type(t) if t.is_named_union() => Some(t.name.name.as_str()),
                _ => None,
            })
            .collect();
        for item in &m.items {
            if let Item::Fn(f) = item {
                if let Some(owner) = &f.scoped_to {
                    if structs.contains(owner.name.as_str()) {
                        attached.entry(owner.name.as_str()).or_default().push(f);
                    }
                }
            }
        }
        let items: Vec<&Item> = m
            .items
            .iter()
            .filter(|item| !matches!(item, Item::Fn(f) if f.scoped_to.as_ref().is_some_and(|o| structs.contains(o.name.as_str()))))
            .collect();
        // The imports the file opens with, then its module-level `use`s; a
        // `use` placed in the source goes where it was written [fmt-comments].
        let leading = items.iter().take_while(|i| matches!(i, Item::Import(_))).count();
        let mut elems: Vec<Top> = items[..leading].iter().map(|i| Top::Item(i)).collect();
        let mut rest: Vec<Top> = items[leading..].iter().map(|i| Top::Item(i)).collect();
        for u in &m.uses {
            if self.anchored(u.span) {
                let at = |e: &Top| self.anchored(e.span()) && e.span().start > u.span.start;
                match elems.iter().position(at) {
                    Some(i) => elems.insert(i, Top::Use(u)),
                    None => match rest.iter().position(at) {
                        Some(i) => rest.insert(i, Top::Use(u)),
                        None => rest.push(Top::Use(u)),
                    },
                }
            } else {
                elems.push(Top::Use(u));
            }
        }
        elems.extend(rest);
        let mut run = Run::default();
        let mut prev: Option<(Top, bool)> = None;
        for e in elems {
            let span = e.span();
            let one_line = self.one_line(&e, &attached);
            self.elem_start(&mut run, span, e.takes_docs(), |source| match prev {
                // [fmt-blank-lines] One blank line between declarations, but
                // for one-line declarations the author wrote on consecutive
                // lines (a group of imports, of aliases).
                Some((_, p)) if source => !(p && one_line),
                // A printing with no source to follow: imports and `use`s
                // are one paragraph.
                Some((p, _)) => !(p.is_import_or_use() && e.is_import_or_use()),
                None => false,
            });
            match e {
                Top::Item(item) => self.namespaced(item, &m.namespaces, &attached),
                Top::Use(u) => {
                    let s = self.at();
                    self.push("use ");
                    self.expr(&u.handler, 0);
                    let e = self.at();
                    self.record(u.span, s, e, s, e);
                }
            }
            self.elem_end(&mut run, span);
            prev = Some((e, one_line));
        }
        self.run_close(&mut run, u32::MAX);
        self.rest_of_comments();
        self.newline();
        while self.out.ends_with('\n') && self.out.ends_with("\n\n") {
            self.out.pop();
        }
        if !self.out.is_empty() && !self.out.ends_with('\n') {
            self.out.push('\n');
        }
    }

    /// [mod-namespace] A flattened declaration (`Outer.Inner.Thing`) back in
    /// the namespaces it was declared in.
    fn namespaced(&mut self, item: &Item, namespaces: &[String], attached: &HashMap<&str, Vec<&FnDecl>>) {
        let name = item_name(item);
        let prefix = name.and_then(|n| {
            namespaces
                .iter()
                .filter(|ns| n.len() > ns.len() && n.starts_with(ns.as_str()) && n.as_bytes()[ns.len()] == b'.')
                .max_by_key(|ns| ns.len())
        });
        let Some(prefix) = prefix else {
            return self.item(item, None, attached);
        };
        let segments: Vec<&str> = prefix.split('.').collect();
        for seg in &segments {
            self.push("namespace ");
            self.push(seg);
            self.push(" {");
            self.indent += 1;
            self.newline();
        }
        self.item(item, Some(prefix.len() + 1), attached);
        for _ in &segments {
            self.indent -= 1;
            self.newline();
            self.push("}");
        }
    }

    /// One item; `strip` cuts a namespace prefix off its name.
    fn item(&mut self, item: &Item, strip: Option<usize>, attached: &HashMap<&str, Vec<&FnDecl>>) {
        match item {
            Item::Import(i) => self.import(i),
            Item::Type(t) => {
                let fns = attached.get(t.name.name.as_str()).cloned().unwrap_or_default();
                self.type_decl(t, strip, &fns)
            }
            Item::Struct(s) => {
                let fns = attached.get(s.name.name.as_str()).cloned().unwrap_or_default();
                self.struct_decl(s, strip, &fns)
            }
            Item::Qualifier(q) => self.qualifier(q, strip),
            Item::Effect(e) => self.effect(e, strip),
            Item::Handler(h) => self.handler(h, strip),
            Item::Fns(g) => self.fns_group(g, strip),
            Item::Fn(f) => self.fn_decl(f, strip, true),
            Item::EffectAlias(a) => {
                self.docs(&a.docs, a.span);
                if a.exported {
                    self.push("export ");
                }
                self.push("effects ");
                self.decl_name(&a.name, strip);
                self.push(" = ");
                self.effect_list(&a.effects);
            }
            Item::Namespace(n) => {
                self.docs(&n.docs, n.span);
                if n.exported {
                    self.push("export ");
                }
                self.push("namespace ");
                self.ident(&n.name);
                // [expr-terminal-scope] `namespace X ...`: its items follow,
                // to the end of the list it is in.
                if n.terminal {
                    self.push(" ...");
                    let mut run = Run { started: true, prev_end: self.anchored(n.name.span).then_some(n.name.span.end), ..Run::default() };
                    self.namespace_items(&mut run, &n.items, attached, true);
                    return;
                }
                self.push(" {");
                self.indent += 1;
                let mut run = Run::default();
                self.namespace_items(&mut run, &n.items, attached, false);
                if self.anchored(n.span) {
                    self.run_close(&mut run, n.span.end.saturating_sub(1));
                }
                self.indent -= 1;
                self.newline();
                self.push("}");
            }
            Item::Refn(r) => self.refn(r),
            Item::Rename(r) => self.rename(r),
            Item::Test(t) => self.test(t),
        }
    }

    /// [mod-namespace] A namespace's declarations, as a run: one blank line
    /// between them, but for consecutive one-line ones written so
    /// [fmt-blank-lines]. `terminal`: after a terminal `...`, where the
    /// first is a paragraph of its own.
    fn namespace_items(&mut self, run: &mut Run, items: &[Item], attached: &HashMap<&str, Vec<&FnDecl>>, terminal: bool) {
        let mut prev: Option<(Top, bool)> = None;
        for it in items {
            let e = Top::Item(it);
            let span = e.span();
            let one_line = self.one_line(&e, attached);
            self.elem_start(run, span, e.takes_docs(), |source| match prev {
                Some((_, p)) if source => !(p && one_line),
                Some((p, _)) => !(p.is_import_or_use() && e.is_import_or_use()),
                None => terminal,
            });
            self.item(it, None, attached);
            self.elem_end(run, span);
            prev = Some((e, one_line));
        }
    }

    /// [fmt-blank-lines] Whether a declaration prints on one line, its docs
    /// aside: no body with anything in it, and no comment inside it.
    fn one_line(&self, e: &Top, attached: &HashMap<&str, Vec<&FnDecl>>) -> bool {
        let span = e.span();
        let commented = self.anchored(span)
            && self.trivia.is_some_and(|t| t.comments[self.next_comment.min(t.comments.len())..].iter().take_while(|c| c.start < span.end).any(|c| c.start >= span.start));
        e.one_line(attached) && !commented
    }

    fn decl_name(&mut self, id: &Ident, strip: Option<usize>) {
        match strip {
            Some(n) if id.name.len() > n => {
                let text = id.name[n..].to_string();
                self.ident_as(id, &text)
            }
            _ => self.ident(id),
        }
    }

    fn import(&mut self, i: &ImportDecl) {
        let s = self.at();
        self.push("import ");
        self.list(&i.path, ".", |p, id| p.ident(id));
        if let Some(a) = &i.alias {
            self.push(" as ");
            self.ident(a);
        }
        let e = self.at();
        self.record(i.span, s, e, s, e);
    }

    /// `<A, B canbe linear, [E]>`.
    fn generics(&mut self, generics: &[Ident], canbe: &[(Ident, TypeRef)], effect_generics: &[Ident]) {
        if generics.is_empty() {
            return;
        }
        self.push("<");
        self.list(generics, ", ", |p, g| {
            if effect_generics.iter().any(|e| e.name == g.name) {
                p.push("[");
                p.ident(g);
                p.push("]");
            } else {
                p.ident(g);
            }
            for (n, q) in canbe {
                if n.name == g.name {
                    p.push(" canbe ");
                    p.type_ref(q);
                }
            }
        });
        self.push(">");
    }

    /// The slot block: `(?cmp: (T, T) -> Int, ?Ordered<T>, map: Map<K, V>)`.
    fn slot_block(&mut self, slots: &[SlotDecl], values: &[ValueSlot]) {
        if slots.is_empty() && values.is_empty() {
            return;
        }
        self.push("(");
        let mut first = true;
        for s in slots {
            if !first {
                self.push(", ");
            }
            first = false;
            self.push("?");
            match s {
                SlotDecl::One(f) => {
                    self.ident(&f.name);
                    self.push(": ");
                    self.ty(&f.ty);
                }
                SlotDecl::Group(g) => self.type_ref(g),
            }
        }
        for v in values {
            if !first {
                self.push(", ");
            }
            first = false;
            self.ident(&v.name);
            self.push(": ");
            self.ty(&v.ty);
        }
        self.push(")");
    }

    fn obligations(&mut self, obligations: &[Obligation]) {
        if obligations.is_empty() {
            return;
        }
        self.push(" : ");
        self.list(obligations, ", ", |p, o| {
            p.type_ref(&o.group);
            if let Some(by) = &o.by {
                p.by(by);
            }
        });
    }

    fn by(&mut self, by: &ByRef) {
        self.push(" by ");
        if matches!(by.at, Some(ByAt::Module(_))) {
            self.push("@");
        }
        self.list(&by.path, ".", |p, id| p.ident(id));
        if let Some(ByAt::Fn(path)) = &by.at {
            self.push("@");
            self.list(path, ".", |p, id| p.ident(id));
        }
    }

    fn canbe(&mut self, quals: &[TypeRef]) {
        if quals.is_empty() {
            return;
        }
        self.push(" canbe ");
        self.list(quals, ", ", |p, q| p.type_ref(q));
    }

    fn type_decl(&mut self, t: &TypeDecl, strip: Option<usize>, fns: &[&FnDecl]) {
        self.docs(&t.docs, t.span);
        let s = self.at();
        if t.exported {
            self.push("export ");
        }
        if t.noremote && !t.platform {
            self.push("noremote ");
        }
        if t.linear {
            self.push("linear ");
        }
        if t.threadsafe {
            self.push("threadsafe ");
        }
        if t.iterable {
            self.push("iterable ");
        }
        if t.platform {
            self.push("platform ");
        }
        if t.intrinsic {
            self.push("intrinsic ");
        }
        if t.comptime {
            self.push("comptime ");
        }
        self.push("type ");
        self.decl_name(&t.name, strip);
        self.generics(&t.generics, &t.generic_canbe, &[]);
        self.slot_block(&t.fn_slots, &[]);
        self.canbe(&t.auto_qualifiers);
        if let Some(a) = &t.alias {
            self.push(" = ");
            self.ty(a);
        }
        self.obligations(&t.obligations);
        // [type-union-body] A named union's fns and refinements, printed as
        // `members` prints a struct body's (in written order, or fns then
        // refinements with no source to follow); an opaque type's body is
        // refinements only.
        let own: Vec<&FnDecl> = t.fns.iter().chain(fns.iter().copied()).collect();
        let mut members: Vec<Member> = own.iter().map(|f| Member::Fn(f, false)).collect();
        members.extend(t.refns.iter().map(Member::Refn));
        let close = t.span.end.saturating_sub(1);
        if !members.is_empty() || (self.anchored(t.span) && t.alias.is_none() && self.comment_before(close).is_some()) {
            self.push(" {");
            self.members(members, t.span);
            self.push("}");
        }
        let e = self.at();
        self.record(t.span, s, e, s, e);
    }

    fn struct_decl(&mut self, st: &StructDecl, strip: Option<usize>, fns: &[&FnDecl]) {
        self.docs(&st.docs, st.span);
        let note = self.notes.struct_note(st);
        self.comment_lines(&note);
        // [actor-message] A message arm is the compiler's, declared again
        // wherever its effect is: shown, but as a comment.
        if let Some(effect) = &st.message_arm {
            self.push(&format!("// the message arm the compiler declares for `{}`: struct {}", effect.name, st.name.name));
            return;
        }
        let s = self.at();
        for a in &st.annotations {
            self.type_ref(a);
            self.push(" ");
        }
        if st.exported {
            self.push("export ");
        }
        if st.noremote {
            self.push("noremote ");
        }
        if st.opaque {
            self.push("opaque ");
        }
        if st.linear {
            self.push("linear ");
        }
        if st.comptime {
            self.push("comptime ");
        }
        self.push("struct ");
        self.decl_name(&st.name, strip);
        self.generics(&st.generics, &st.generic_canbe, &st.effect_generics);
        self.slot_block(&st.fn_slots, &[]);
        self.obligations(&st.obligations);
        self.canbe(&st.auto_qualifiers);
        self.push(" {");
        let mut members: Vec<Member> = st.fields.iter().map(Member::Field).collect();
        members.extend(st.fns.iter().chain(fns.iter().copied()).map(|f| Member::Fn(f, false)));
        members.extend(st.refns.iter().map(Member::Refn));
        self.members(members, st.span);
        self.push("}");
        let e = self.at();
        self.record(st.span, s, e, s, e);
    }

    fn field(&mut self, f: &FieldDecl) {
        self.docs(&f.docs, f.span);
        let s = self.at();
        for a in &f.annotations {
            self.type_ref(a);
            self.push(" ");
        }
        self.ident(&f.name);
        match &f.ty {
            Some(t) => {
                self.push(": ");
                self.ty(t);
                if f.canbe_mut {
                    self.push(" canbe Mut");
                }
                if let Some(d) = &f.default {
                    self.push(" = ");
                    self.expr(d, 0);
                }
            }
            None => {
                self.push(" = ");
                if let Some(d) = &f.default {
                    self.expr(d, 0);
                }
            }
        }
        let e = self.at();
        self.record(f.span, s, e, s, e);
    }

    fn qualifier(&mut self, q: &QualifierDecl, strip: Option<usize>) {
        self.docs(&q.docs, q.span);
        let s = self.at();
        if q.exported {
            self.push("export ");
        }
        if q.intrinsic {
            self.push("intrinsic ");
        }
        if q.comptime {
            self.push("comptime ");
        } else if q.subject == QualSubject::Provenance {
            self.push("provenance ");
        }
        self.push("qualifier ");
        self.decl_name(&q.name, strip);
        self.generics(&q.generics, &q.generic_canbe, &[]);
        self.slot_block(&q.fn_slots, &q.value_slots);
        self.push(" for ");
        self.ty(&q.of);
        if q.has_body {
            self.push(" {");
            let mut members: Vec<Member> = q.field_overrides.iter().map(Member::Field).collect();
            members.extend(q.fns.iter().map(|f| Member::Fn(f, true)));
            members.extend(q.refns.iter().map(Member::Refn));
            self.members(members, q.span);
            self.push("}");
        }
        let e = self.at();
        self.record(q.span, s, e, s, e);
    }

    fn effect(&mut self, ef: &EffectDecl, strip: Option<usize>) {
        self.docs(&ef.docs, ef.span);
        let s = self.at();
        if ef.exported {
            self.push("export ");
        }
        if ef.is_actor {
            self.push("actor ");
        }
        self.push("effect ");
        self.decl_name(&ef.name, strip);
        self.generics(&ef.generics, &[], &[]);
        if !ef.prereqs.is_empty() {
            self.push(" ");
            self.effect_list(&ef.prereqs);
        }
        self.push(" {");
        self.members(ef.fns.iter().map(|f| Member::Fn(f, true)).collect(), ef.span);
        self.push("}");
        let e = self.at();
        self.record(ef.span, s, e, s, e);
    }

    fn handler(&mut self, h: &HandlerDecl, strip: Option<usize>) {
        self.docs(&h.docs, h.span);
        let s = self.at();
        if h.exported {
            self.push("export ");
        }
        if h.intrinsic {
            self.push("intrinsic ");
        }
        if h.threadsafe {
            self.push("threadsafe ");
        }
        if h.platform {
            self.push("platform ");
        }
        self.push("handler ");
        self.decl_name(&h.name, strip);
        self.generics(&h.generics, &[], &[]);
        if !h.params.is_empty() || !h.platform {
            self.params(&h.params, &[], after_generics(h.name.span, &h.generics, &[]));
        }
        if let Some(effects) = &h.effects {
            self.push(" ");
            self.effect_list(effects);
        }
        self.push(" for ");
        for (i, face) in h.of.iter().enumerate() {
            if i > 0 {
                self.push(", ");
            }
            for c in h.of_claims.get(i).map(|v| v.as_slice()).unwrap_or(&[]) {
                self.type_ref(c);
                self.push(" ");
            }
            self.ty(face);
        }
        let body = h.mailbox.is_some() || !h.state.is_empty() || h.state_name.is_some() || h.init.is_some() || !h.fns.is_empty();
        if body {
            self.push(" {");
            let mut members = Vec::new();
            if let Some(m @ Expr::StructLit { .. }) = &h.mailbox {
                members.push(Member::Mailbox(m));
            }
            if !h.state.is_empty() || h.state_name.is_some() {
                members.push(Member::State(h));
            }
            if let Some(init) = &h.init {
                members.push(Member::Init(init));
            }
            members.extend(h.fns.iter().map(|f| Member::Fn(f, true)));
            self.members(members, h.span);
            self.push("}");
        }
        let e = self.at();
        self.record(h.span, s, e, s, e);
    }

    fn fns_group(&mut self, g: &FnsDecl, strip: Option<usize>) {
        self.docs(&g.docs, g.span);
        let s = self.at();
        if g.exported {
            self.push("export ");
        }
        self.push("fns ");
        self.decl_name(&g.name, strip);
        self.generics(&g.generics, &[], &g.effect_generics);
        if !g.deductions.is_empty() {
            self.push(" => ");
            self.deductions(&g.deductions.iter().collect::<Vec<_>>());
        }
        self.push(" {");
        self.members(g.fns.iter().map(|f| Member::Grouped(f)).collect(), g.span);
        self.push("}");
        let e = self.at();
        self.record(g.span, s, e, s, e);
    }

    /// [fmt-comments] [fmt-blank-lines] A declaration's body after its `{`,
    /// up to its `}` (which the caller prints): its members as a run, in the
    /// order they were written when every one was placed in the source, and
    /// the comments closing it. A body with nothing in it is `{}`.
    fn members(&mut self, mut members: Vec<Member>, owner: Span) {
        let close = owner.end.saturating_sub(1);
        let anchored = self.anchored(owner);
        if members.is_empty() && !(anchored && self.comment_before(close).is_some()) {
            return;
        }
        if members.iter().all(|m| self.anchored(m.span(self.trivia))) {
            members.sort_by_key(|m| m.span(self.trivia).start);
        }
        self.indent += 1;
        let mut run = Run::default();
        let mut prev: Option<&Member> = None;
        for m in &members {
            let span = m.span(self.trivia);
            // A printing with no source to follow: a blank line between
            // members, but not between fields, nor between a `fns` group's.
            let default = prev.is_some_and(|p| !matches!((p, m), (Member::Field(_), Member::Field(_)) | (Member::Grouped(_), Member::Grouped(_))));
            self.elem_start(&mut run, span, m.takes_docs(), |source| !source && default);
            self.member(m);
            self.elem_end(&mut run, span);
            prev = Some(m);
        }
        if anchored {
            self.run_close(&mut run, close);
        }
        self.indent -= 1;
        self.newline();
    }

    fn member(&mut self, m: &Member) {
        match m {
            Member::Field(f) => self.field(f),
            Member::Fn(f, export) => self.fn_decl(f, None, *export),
            Member::Grouped(f) => self.fn_decl(f, None, true),
            Member::Refn(r) => self.refn(r),
            Member::Mailbox(Expr::StructLit { fields, span, .. }) => {
                let s = self.at();
                self.push("mailbox ");
                self.struct_fields(fields, Some((span.start, span.end)));
                let e = self.at();
                self.record(*span, s, e, s, e);
            }
            Member::Mailbox(_) => {}
            Member::State(h) => {
                self.push("state ");
                if let Some(n) = &h.state_name {
                    self.ident(n);
                    self.push(" ");
                }
                self.push("{");
                let close = self.state_close(h);
                self.fields_body(&h.state, close);
                self.push("}");
            }
            Member::Init(init) => {
                let note = self.notes.fn_note(init);
                self.comment_lines(&note);
                let s = self.at();
                self.ident(&init.name);
                self.push(" ");
                if let Some(b) = &init.body {
                    self.block(b);
                }
                let e = self.at();
                self.record(init.span, s, e, s, e);
            }
        }
    }

    /// Where a handler's `state { … }` closes, in the source: its `}`.
    fn state_close(&self, h: &HandlerDecl) -> Option<u32> {
        let from = h.state.last().map(|f| f.span.end).or(h.state_name.as_ref().map(|n| n.span.end))?;
        self.anchored(h.span).then(|| self.close_after(from, '}')).flatten()
    }

    /// The fields of a `state { … }` after its `{`, up to its `}`: one per
    /// line, the comments among them kept; `{}` with none.
    fn fields_body(&mut self, fields: &[FieldDecl], close: Option<u32>) {
        let has_comment = close.is_some_and(|c| self.comment_before(c).is_some());
        if fields.is_empty() && !has_comment {
            return;
        }
        self.indent += 1;
        let mut run = Run::default();
        for f in fields {
            self.elem_start(&mut run, f.span, true, |_| false);
            self.field(f);
            self.elem_end(&mut run, f.span);
        }
        if let Some(c) = close {
            self.run_close(&mut run, c);
        }
        self.indent -= 1;
        self.newline();
    }

    fn refn(&mut self, r: &RefnDecl) {
        self.docs(&r.docs, r.span);
        let s = self.at();
        self.push("refn ");
        self.ident(&r.name);
        if let Some(at) = &r.at {
            self.push("@");
            self.ident(at);
        }
        self.generics(&r.generics, &[], &[]);
        self.push("(");
        self.list(&r.params, ", ", |p, param| p.param(param));
        if r.wildcard {
            if !r.params.is_empty() {
                self.push(", ");
            }
            self.push("*");
        }
        self.push(")");
        if let Some(resp) = &r.response {
            self.push(" -> ");
            self.responses(resp);
        }
        if !r.deductions.is_empty() {
            self.push(" => ");
            self.list(&r.deductions, ", ", |p, d| {
                p.ident(&d.param);
                p.push(":");
                for a in &d.add {
                    p.push(" +");
                    p.type_ref(a);
                }
                for a in &d.remove {
                    p.push(" -");
                    p.type_ref(a);
                }
                if !d.preserve.is_empty() {
                    p.push(" preserve");
                    for a in &d.preserve {
                        p.push(" ");
                        p.type_ref(a);
                    }
                }
            });
        }
        let e = self.at();
        self.record(r.span, s, e, s, e);
    }

    fn responses(&mut self, resp: &[ConditionalResponse]) {
        self.list(resp, " && ", |p, c| {
            p.ident(&c.subject);
            p.push(if c.negated { " !is " } else { " is " });
            p.type_ref(&c.qual);
        });
    }

    fn rename(&mut self, r: &RenameDecl) {
        self.docs(&r.docs, r.span);
        let s = self.at();
        self.push("rename fn ");
        self.ident(&r.name);
        self.push(" = ");
        self.ident(&r.target);
        self.generics(&r.generics, &[], &[]);
        self.params(&r.params, &[], after_generics(r.target.span, &r.generics, &[]));
        let e = self.at();
        self.record(r.span, s, e, s, e);
    }

    fn test(&mut self, t: &TestDecl) {
        self.docs(&t.docs, t.span);
        let s = self.at();
        self.push("test ");
        if let Some(k) = &t.kind {
            self.ident(k);
            if !t.args.is_empty() {
                self.push("(");
                self.list(&t.args, ", ", |p, (n, v)| {
                    p.ident(n);
                    p.push(" = ");
                    p.expr(v, 0);
                });
                self.push(")");
            }
            self.push(" ");
        }
        self.push(&quote_str(&t.name));
        if !t.params.is_empty() {
            self.params(&t.params, &[], t.span.at(t.span.start, t.span.start));
        }
        self.push(" ");
        self.block(&t.body);
        let e = self.at();
        self.record(t.span, s, e, s, e);
    }

    /// A fn declaration; `exported` says whether its `export` is printed (a
    /// struct's fn takes the struct's).
    fn fn_decl(&mut self, f: &FnDecl, strip: Option<usize>, show_export: bool) {
        match self.notes.fn_source(f) {
            FnSource::Tree => {}
            FnSource::Omit => return,
            FnSource::Text(text) => {
                self.docs(&f.docs, f.span);
                if f.exported && show_export {
                    self.push("export ");
                }
                // Its comments are in the text.
                if self.anchored(f.span) {
                    while self.comment_before(f.span.end).is_some() {
                        self.next_comment += 1;
                    }
                }
                // The text's own lines (indented relative to its first),
                // at this indentation.
                for (i, line) in text.lines().enumerate() {
                    if i > 0 {
                        self.newline();
                    }
                    self.push(line.trim_end());
                }
                return;
            }
        }
        self.docs(&f.docs, f.span);
        let note = self.notes.fn_note(f);
        self.comment_lines(&note);
        let s = self.at();
        if f.exported && show_export {
            self.push("export ");
        }
        if f.compfn.is_some() {
            self.push("comptime ");
        }
        if f.platform {
            self.push("platform ");
        }
        if f.intrinsic {
            self.push("intrinsic ");
        }
        if f.is_iter {
            self.push("iter ");
        }
        if f.is_send {
            self.push("send ");
        }
        self.push("fn ");
        match &f.type_name {
            // [fn-type-named] `fn Mut List<T>(…)`: the type's own generics.
            Some(t) => {
                let s0 = self.at();
                self.type_fn_name(t, &f.generic_canbe);
                let e0 = self.at();
                self.record(f.name.span, s0, e0, s0, e0);
            }
            None => {
                self.decl_name(&f.name, strip);
                match f.compfn.as_ref().and_then(|c| c.bound.as_ref()) {
                    Some((kind, param)) => {
                        self.push("<");
                        self.ident(param);
                        self.push(" is ");
                        self.push(kind.word());
                        self.push(">");
                    }
                    None => self.generics(&f.generics, &f.generic_canbe, &f.effect_generics),
                }
            }
        }
        let bound = f.compfn.as_ref().and_then(|c| c.bound.as_ref()).map(|(_, p)| p.clone());
        let named = after_generics(f.name.span, &f.generics, &f.generic_canbe);
        self.params(&f.params, &f.implicit_groups, after_generics(named, bound.as_slice(), &[]));
        if let Some(effects) = &f.effects {
            self.push(" ");
            self.effect_list(effects);
        }
        let opaque: Vec<&Ident> = f
            .deductions
            .iter()
            .flatten()
            .filter(|d| matches!(d.target, DeductionTarget::Opaque))
            .filter_map(|d| d.proj_sources())
            .flatten()
            .collect();
        if let Some(resp) = &f.response {
            self.push(" -> ");
            self.responses(resp);
        } else if f.return_type.is_some() || f.constructs.is_some() {
            self.push(" -> ");
            if let Some(c) = &f.constructs {
                self.push("+");
                self.type_ref(c);
                if f.return_type.is_some() {
                    self.push(" ");
                }
            }
            if let Some(t) = &f.return_type {
                self.ty(t);
            }
            if !opaque.is_empty() {
                self.push(" holds proj(");
                self.list(&opaque, ", ", |p, id| p.ident(id));
                self.push(")");
            }
        }
        let own: Vec<&Deduction> = f.deductions.iter().flatten().filter(|d| !matches!(d.target, DeductionTarget::Opaque)).collect();
        if !own.is_empty() {
            self.push(" => ");
            self.deductions(&own);
        } else if f.deductions.as_ref().is_some_and(|d| d.is_empty()) {
            // `=> ` with nothing is no clause: an explicit empty list has no
            // spelling, and means what none does.
        }
        // [fn-contract] `=>[f] …`: a fn-typed parameter's contract.
        for param in &f.params {
            let target = match &param.ty {
                Type::QualifiedGroup { base, .. } => base.as_ref(),
                other => other,
            };
            if let Type::Fn { deductions: Some(ds), .. } = target {
                let written: Vec<&Deduction> = ds.iter().filter(|d| !matches!(d.target, DeductionTarget::Opaque)).collect();
                if !written.is_empty() {
                    self.push(" =>[");
                    self.push(&param.name.name);
                    self.push("] ");
                    self.deductions(&written);
                }
            }
        }
        if let Some(by) = &f.by {
            self.by(by);
        }
        if f.is_iter {
            self.push(" {");
            self.indent += 1;
            // [fmt-comments] The state block is the body's first element,
            // placed by its name (or its first field).
            let mut run = Run::default();
            let at = f.iter_state_name.as_ref().map(|n| n.span).or(f.iter_state.first().map(|fd| fd.span));
            match at {
                Some(at) => self.elem_start(&mut run, at, false, |_| false),
                None => self.newline(),
            }
            self.push("state ");
            if let Some(n) = &f.iter_state_name {
                self.ident(n);
                self.push(" ");
            }
            self.push("{");
            let from = f.iter_state.last().map(|fd| fd.span.end).or(f.iter_state_name.as_ref().map(|n| n.span.end));
            let close = from.filter(|_| self.anchored(f.span)).and_then(|o| self.close_after(o, '}'));
            self.fields_body(&f.iter_state, close);
            self.push("}");
            if let Some(b) = &f.body {
                let mut run = Run { started: true, prev_end: close, ..run };
                self.stmts_run(&mut run, &b.stmts);
                if self.anchored(f.span) {
                    self.run_close(&mut run, f.span.end.saturating_sub(1));
                }
            }
            self.indent -= 1;
            self.newline();
            self.push("}");
        } else if let Some(b) = &f.body {
            self.push(" ");
            self.block(b);
        }
        let e = self.at();
        self.record(f.span, s, e, s, e);
    }

    /// [fn-type-named] `Mut List<T canbe linear>`.
    fn type_fn_name(&mut self, t: &Type, canbe: &[(Ident, TypeRef)]) {
        let Type::Named { qualifiers, base } = t else {
            return self.ty(t);
        };
        for q in qualifiers {
            self.type_ref(q);
            self.push(" ");
        }
        self.ident(&base.name);
        if !base.args.is_empty() {
            self.push("<");
            self.list(&base.args, ", ", |p, a| {
                p.ty(a);
                if let Type::Named { base: b, .. } = a {
                    for (n, q) in canbe {
                        if n.name == b.name.name {
                            p.push(" canbe ");
                            p.type_ref(q);
                        }
                    }
                }
            });
            self.push(">");
        }
    }

    /// A parameter list; `from`: where it starts in the source (the name
    /// before it), so a comment written in it breaks it one parameter per
    /// line [fmt-comments].
    fn params(&mut self, params: &[Param], groups: &[TypeRef], from: Span) {
        self.push("(");
        let items: Vec<ParamItem> = params
            .iter()
            .filter(|p| !p.implicit)
            .chain(params.iter().filter(|p| p.implicit))
            .map(ParamItem::Param)
            .chain(groups.iter().map(ParamItem::Group))
            .collect();
        let spans: Vec<Span> = items.iter().map(ParamItem::span).collect();
        let last = spans.iter().map(|s| s.end).max().unwrap_or(from.end).max(from.end);
        let close = self.anchored(from).then(|| self.close_after(last, ')')).flatten();
        match close.filter(|&c| self.list_holds_comment(from.end, c + 1, &spans)) {
            Some(c) => self.broken_list(&items, ParamItem::span, c, |p, i| p.param_item(i)),
            None => self.list(&items, ", ", |p, i| p.param_item(i)),
        }
        self.push(")");
    }

    fn param_item(&mut self, item: &ParamItem) {
        match item {
            ParamItem::Param(param) => self.param(param),
            ParamItem::Group(g) => {
                self.push("?");
                self.type_ref(g);
            }
        }
    }

    fn param(&mut self, param: &Param) {
        let s = self.at();
        for a in &param.annotations {
            self.type_ref(a);
            self.push(" ");
        }
        if param.variadic {
            self.push("...");
        }
        if param.implicit {
            self.push("?");
        }
        self.ident(&param.name);
        self.push(": ");
        self.ty(&param.ty);
        if let Some(d) = &param.default {
            self.push(" = ");
            self.expr(d, 0);
        }
        let e = self.at();
        self.record(param.span, s, e, s, e);
    }

    fn effect_list(&mut self, effects: &[EffectRef]) {
        self.push("[");
        self.list(effects, ", ", |p, e| p.effect_ref(e));
        self.push("]");
    }

    fn effect_ref(&mut self, e: &EffectRef) {
        match e {
            EffectRef::Use(span) => {
                let s = self.at();
                self.push("use");
                let e = self.at();
                self.record(*span, s, e, s, e);
            }
            EffectRef::Spawn(span) => {
                let s = self.at();
                self.push("spawn");
                let e = self.at();
                self.record(*span, s, e, s, e);
            }
            EffectRef::Effect(r) => self.type_ref(r),
            EffectRef::Claimed { claims, effect } => {
                for c in claims {
                    self.type_ref(c);
                    self.push(" ");
                }
                self.type_ref(effect);
            }
        }
    }

    /// A deduction clause's entries. [implicit-with] A plural subject, `a|b
    /// with hash`, is read as one entry per subject sharing the entry's span
    /// (written separately, each would have its own): such a run is written
    /// back as the one entry.
    fn deductions(&mut self, ds: &[&Deduction]) {
        let mut i = 0;
        while i < ds.len() {
            if i > 0 {
                self.push(", ");
            }
            let d = ds[i];
            let mut run = 1;
            if let (DeductionKind::With { others }, DeductionTarget::Param { path, .. }) = (&d.kind, &d.target) {
                while path.is_empty()
                    && ds.get(i + run).is_some_and(|n| n.span == d.span && matches!((&n.kind, &n.target), (DeductionKind::With { others: o }, DeductionTarget::Param { path: p, .. }) if o == others && p.is_empty()))
                {
                    run += 1;
                }
                if run > 1 {
                    let s = self.at();
                    for (k, e) in ds[i..i + run].iter().enumerate() {
                        if k > 0 {
                            self.push("|");
                        }
                        if let DeductionTarget::Param { name, .. } = &e.target {
                            self.ident(name);
                        }
                    }
                    for o in others {
                        self.push(" with ");
                        self.ident(o);
                    }
                    let e = self.at();
                    self.record(d.span, s, e, s, e);
                }
            }
            if run == 1 {
                self.deduction(d);
            }
            i += run;
        }
    }

    fn deduction(&mut self, d: &Deduction) {
        let s = self.at();
        let target = |p: &mut Self| match &d.target {
            DeductionTarget::Param { name, path } => {
                p.ident(name);
                for f in path {
                    p.push(".");
                    p.ident(f);
                }
            }
            DeductionTarget::Result { path } => {
                for f in path {
                    p.push(".");
                    p.ident(f);
                }
            }
            DeductionTarget::Opaque => {}
        };
        match &d.kind {
            DeductionKind::KeepAll => target(self),
            DeductionKind::Moved => {
                self.push("!");
                target(self);
            }
            DeductionKind::Deferred => {
                self.push("defer ");
                target(self);
            }
            DeductionKind::Exhaustive { quals, reapplied } => {
                target(self);
                self.push(":");
                if quals.is_empty() && reapplied.is_empty() {
                    self.push(" None");
                }
                for q in quals {
                    self.push(" ");
                    self.type_ref(q);
                }
                for q in reapplied {
                    self.push(" +");
                    self.type_ref(q);
                }
            }
            DeductionKind::Remove(quals) => {
                target(self);
                self.push(":");
                for q in quals {
                    self.push(" -");
                    self.type_ref(q);
                }
            }
            DeductionKind::Proj(sources) => {
                target(self);
                self.push(": proj(");
                self.list(sources, ", ", |p, id| p.ident(id));
                self.push(")");
            }
            DeductionKind::Preserve(quals) => {
                target(self);
                self.push(": preserve");
                for q in quals {
                    self.push(" ");
                    self.type_ref(q);
                }
            }
            DeductionKind::With { others } => {
                target(self);
                for o in others {
                    self.push(" with ");
                    self.ident(o);
                }
            }
        }
        let e = self.at();
        self.record(d.span, s, e, s, e);
    }

    // --- Types ---

    /// `t` inside the parentheses written around it [expr-group].
    fn ty(&mut self, t: &Type) {
        let parens = t.parens();
        for _ in 0..parens {
            self.push("(");
        }
        self.ty_inner(t);
        for _ in 0..parens {
            self.push(")");
        }
    }

    fn ty_inner(&mut self, t: &Type) {
        let s = self.at();
        match t {
            Type::Named { qualifiers, base } => {
                // [actor-message] `Message<E>` is written `message[E]`.
                for q in qualifiers {
                    self.type_ref(q);
                    self.push(" ");
                }
                self.type_ref(base);
            }
            Type::QualifiedGroup { qualifiers, base, .. } => {
                for q in qualifiers {
                    self.type_ref(q);
                    self.push(" ");
                }
                if matches!(**base, Type::Literal { .. }) || base.parens() > 0 {
                    self.ty(base);
                } else {
                    self.push("(");
                    self.ty(base);
                    self.push(")");
                }
            }
            Type::Union { arms, .. } => {
                let messages: Option<Vec<&Type>> = arms
                    .iter()
                    .map(|a| match a {
                        Type::Named { qualifiers, base } if qualifiers.is_empty() && base.name.name == MESSAGE_TYPE && base.args.len() == 1 => Some(&base.args[0]),
                        _ => None,
                    })
                    .collect();
                if let Some(effects) = messages.filter(|m| m.len() > 1) {
                    self.push("message[");
                    self.list(&effects, ", ", |p, e| p.ty(e));
                    self.push("]");
                } else {
                    self.list(arms, " | ", |p, a| {
                        let paren = matches!(a, Type::Fn { .. } | Type::Union { .. }) && a.parens() == 0;
                        if paren {
                            p.push("(");
                        }
                        p.ty(a);
                        if paren {
                            p.push(")");
                        }
                    });
                }
            }
            Type::Spread { inner, .. } => {
                self.push("...");
                self.ty(inner);
            }
            Type::Literal { value, .. } => self.push(&value.to_string()),
            Type::Tuple { elems, .. } => {
                self.push("(");
                self.list(elems, ", ", |p, e| p.ty(e));
                self.push(")");
            }
            Type::Array { elem, .. } | Type::Nullable { inner: elem, .. } => {
                let paren = elem.parens() == 0
                    && (matches!(**elem, Type::Fn { .. } | Type::Union { .. })
                        || matches!(&**elem, Type::Named { qualifiers, .. } if !qualifiers.is_empty())
                        || matches!(**elem, Type::QualifiedGroup { .. }));
                if paren {
                    self.push("(");
                }
                self.ty(elem);
                if paren {
                    self.push(")");
                }
                self.push(if matches!(t, Type::Array { .. }) { "[]" } else { "?" });
            }
            Type::Fn { params, param_names, effects, deductions, ret, .. } => {
                self.push("(");
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        self.push(", ");
                    }
                    if let Some(Some(n)) = param_names.get(i) {
                        self.ident(n);
                        self.push(": ");
                    }
                    self.ty(p);
                }
                self.push(")");
                if let Some(effects) = effects {
                    self.push(" ");
                    self.effect_list(effects);
                }
                self.push(" -> ");
                self.ty(ret);
                let opaque: Vec<&Ident> = deductions
                    .iter()
                    .flatten()
                    .filter(|d| matches!(d.target, DeductionTarget::Opaque))
                    .filter_map(|d| d.proj_sources())
                    .flatten()
                    .collect();
                if !opaque.is_empty() {
                    self.push(" holds proj(");
                    self.list(&opaque, ", ", |p, id| p.ident(id));
                    self.push(")");
                }
            }
        }
        let e = self.at();
        self.record(t.span(), s, e, s, e);
    }

    fn type_ref(&mut self, r: &TypeRef) {
        let s = self.at();
        if r.established {
            self.push("+");
        }
        if r.binder {
            self.push("?");
        }
        let name = r.name.name.as_str();
        if name == EFFECT_ROW_TYPE_NAME {
            self.push("[");
            self.list(&r.args, ", ", |p, a| p.ty(a));
            self.push("]");
        } else if name == EFF_TYPE_NAME && !r.binder && r.at.is_none() {
            self.push("eff[");
            self.list(&r.args, ", ", |p, a| p.ty(a));
            self.push("]");
        } else if name == MESSAGE_TYPE && !r.binder && r.at.is_none() && r.args.len() == 1 {
            self.push("message[");
            self.ty(&r.args[0]);
            self.push("]");
        } else {
            match type_splice_text(&r.name.name) {
                // [comptime-inline] `[field.type]`, a ref named `field.type`.
                Some(splice) => self.ident_as(&r.name, &splice),
                None => self.ident(&r.name),
            }
            if let Some(alias) = &r.alias {
                self.push(": ");
                self.ident(alias);
            }
            if let Some(at) = &r.at {
                self.push("@");
                self.ident(at);
            }
            if !r.from.is_empty() {
                self.push("(");
                self.list(&r.from, ", ", |p, id| p.ident(id));
                self.push(")");
            }
            if !r.args.is_empty() {
                self.push("<");
                // [fmt-comments] A comment among the type arguments breaks
                // them one per line.
                let spans: Vec<Span> = r.args.iter().map(Type::span).collect();
                let close = self.anchored(r.span).then(|| self.close_after(spans.iter().map(|s| s.end).max().unwrap_or(r.name.span.end), '>')).flatten();
                match close.filter(|&c| self.list_holds_comment(r.name.span.end, c + 1, &spans)) {
                    Some(c) => self.broken_list(&r.args, Type::span, c, |p, a| p.ty(a)),
                    None => self.list(&r.args, ", ", |p, a| p.ty(a)),
                }
                self.push(">");
            }
            if !r.value_args.is_empty() {
                self.push("(");
                self.list(&r.value_args, ", ", |p, a| p.ty(a));
                self.push(")");
            }
        }
        let e = self.at();
        self.record(r.span, s, e, s, e);
    }

    // --- Blocks and statements ---

    fn block(&mut self, b: &Block) {
        self.block_to(b, b.span.end.saturating_sub(1));
    }

    /// A block whose closing comments are those written before `close`:
    /// its `}`, or for a branch followed by `elif` or `else`, what follows
    /// it [fmt-comments].
    fn block_to(&mut self, b: &Block, close: u32) {
        let s = self.at();
        // [expr-terminal-scope] Written `...`: its statements follow the
        // statement that holds it (`stmts`).
        if b.terminal && !self.inline {
            self.push("...");
            let e = self.at();
            self.record(b.span, s, e, s, e);
            self.pending_rest = Some(b.clone());
            return;
        }
        let saved = std::mem::replace(&mut self.no_struct, false);
        let anchored = self.anchored(b.span);
        if b.stmts.is_empty() && !(anchored && self.comment_before(close).is_some()) {
            self.push("{}");
        } else if self.inline {
            self.push("{ ");
            self.list(&b.stmts, " ", |p, st| p.stmt(st));
            self.push(" }");
        } else {
            self.push("{");
            self.indent += 1;
            let mut run = Run::default();
            self.stmts_run(&mut run, &b.stmts);
            if anchored {
                self.run_close(&mut run, close);
            }
            self.indent -= 1;
            self.newline();
            self.push("}");
        }
        self.no_struct = saved;
        let e = self.at();
        self.record(b.span, s, e, s, e);
    }

    /// The statements of a block, each on a line of its own, and after one
    /// whose construct took a terminal `...` body, that body's.
    fn stmts(&mut self, stmts: &[Stmt]) {
        let mut run = Run { started: true, ..Run::default() };
        self.stmts_run(&mut run, stmts);
    }

    /// The statements of a block as a run [fmt-comments] [fmt-blank-lines].
    fn stmts_run(&mut self, run: &mut Run, stmts: &[Stmt]) {
        // A body printed inside a statement (the `else` after `if c ...`)
        // leaves the statement's own terminal body to the statement.
        let outer = self.pending_rest.take();
        for st in stmts {
            let span = st.span();
            self.elem_start(run, span, matches!(st, Stmt::Rename(_)), |_| false);
            self.stmt(st);
            // [expr-terminal-scope] The rest of the block a terminal `...`
            // in the statement stands for: the statements after it.
            match self.pending_rest.take() {
                Some(rest) => {
                    // The statement ends on the last line of code before
                    // its rest begins.
                    let end = self.trivia.map_or(rest.span.start, |t| t.code_end_before(rest.span.start)).max(span.start);
                    let head = Span { end, ..span };
                    self.elem_end(run, head);
                    self.stmts_run(run, &rest.stmts);
                }
                None => self.elem_end(run, span),
            }
        }
        self.pending_rest = outer;
    }

    fn stmt(&mut self, st: &Stmt) {
        // [syntax-lowered] A statement the sugar lowering wrote, under the
        // sugar it was written as.
        if let (Some(written), false) = (compound_origin(st), self.inline) {
            let text = self.source_text(|p| p.stmt(&written));
            self.push("// desugared from `");
            self.push(&text);
            self.push("`");
            self.newline();
        }
        let s = self.at();
        match st {
            Stmt::Let { pattern, ty, value, span } => {
                self.push("let ");
                self.pattern(pattern);
                if let Some(t) = ty {
                    self.push(": ");
                    self.ty(t);
                }
                self.push(" = ");
                self.expr(value, 0);
                let e = self.at();
                self.record(*span, s, e, s, e);
            }
            Stmt::Assign { target, value, span } => {
                self.expr(target, 1);
                self.push(" = ");
                self.expr(value, 0);
                let e = self.at();
                self.record(*span, s, e, s, e);
            }
            Stmt::Use { handler, with_items, span } => {
                self.push("use ");
                self.expr(handler, 1);
                self.with_items(with_items);
                let e = self.at();
                self.record(*span, s, e, s, e);
            }
            Stmt::Rename(r) => self.rename(r),
            Stmt::Expr(e) => self.expr(e, 0),
            Stmt::Comp(c) => {
                let s = self.at();
                self.comp_stmt(c);
                let e = self.at();
                self.record(c.span(), s, e, s, e);
            }
            Stmt::Retype { target, retype, span } => {
                self.expr(target, 1);
                self.push(" := ");
                self.retype(retype);
                let e = self.at();
                self.record(*span, s, e, s, e);
            }
            Stmt::CompoundAssign { target, op, value, span, .. } => {
                self.expr(target, 1);
                self.push(" ");
                self.push(binary_symbol(*op));
                self.push("= ");
                self.expr(value, 0);
                let e = self.at();
                self.record(*span, s, e, s, e);
            }
        }
    }

    /// What `f` prints for source on one line, by a printer of its own (its
    /// places are not this printing's): the sugar a comment names.
    fn source_text(&self, f: impl FnOnce(&mut Printer<'_>)) -> String {
        let mut p = Printer::new(self.notes, Reading::Source);
        p.inline = true;
        f(&mut p);
        p.out.lines().map(str::trim).collect::<Vec<_>>().join(" ")
    }

    fn with_items(&mut self, items: &[Expr]) {
        if items.is_empty() {
            return;
        }
        self.push(" with ");
        self.list(items, ", ", |p, e| p.expr(e, 1));
    }

    fn retype(&mut self, r: &Retype) {
        let s = self.at();
        match &r.kind {
            RetypeKind::Drop(refs) => self.list(refs, " ", |p, q| {
                p.push("-");
                p.type_ref(q);
            }),
            RetypeKind::Keep(refs) => self.list(refs, " ", |p, q| p.type_ref(q)),
        }
        let e = self.at();
        self.record(r.span, s, e, s, e);
    }

    fn pattern(&mut self, pat: &Pattern) {
        match pat {
            Pattern::Ident(id) => self.ident(id),
            Pattern::Tuple { elems, span } => {
                let s = self.at();
                self.push("(");
                self.list(elems, ", ", |p, e| p.pattern(e));
                self.push(")");
                let e = self.at();
                self.record(*span, s, e, s, e);
            }
            Pattern::Struct { fields, span } => {
                let s = self.at();
                self.push("{ ");
                self.list(fields, ", ", |p, f| {
                    let s = p.at();
                    p.ident(&f.field);
                    if f.binding.name != f.field.name || f.binding.span != f.field.span {
                        p.push(": ");
                        p.ident(&f.binding);
                    }
                    let e = p.at();
                    p.record(f.span, s, e, s, e);
                });
                self.push(" }");
                let e = self.at();
                self.record(*span, s, e, s, e);
            }
        }
    }

    // --- Expressions ---

    /// `e` in a context accepting precedence `min` and tighter: inside the
    /// parentheses written around it [expr-group], or, where none were and
    /// the context needs them, one pair of the printer's.
    fn expr(&mut self, e: &Expr, min: u8) {
        let written = match e {
            // [fn-member-call] A grouped field read is written in parentheses
            // even where no parse counted them.
            Expr::Field { grouped: true, .. } => e.parens().max(1),
            _ => e.parens(),
        };
        let parens = if written > 0 {
            written
        } else {
            let level = self.level(e);
            u8::from(level < min || (self.no_struct && self.holds_struct_lit(e)))
        };
        let outer_start = self.at();
        let saved = self.no_struct;
        for _ in 0..parens {
            self.push("(");
            self.no_struct = false;
        }
        let start = self.at();
        // [syntax-lowered] What the sugar lowering wrote, for a dump: marked
        // with the sugar it was written as, once (not again inside it).
        let origin = match self.reading {
            Reading::Dump if !self.quiet_origin => sugar_origin(e),
            _ => None,
        };
        let quiet = self.quiet_origin || origin.is_some();
        let was_quiet = std::mem::replace(&mut self.quiet_origin, quiet);
        self.expr_inner(e);
        self.quiet_origin = was_quiet;
        let end = self.at();
        for _ in 0..parens {
            self.push(")");
        }
        self.no_struct = saved;
        let outer_end = self.at();
        self.record(e.span(), start, end, outer_start, outer_end);
        if let Some(sugar) = origin {
            let mut text = self.source_text(|p| p.sugar(&sugar));
            if let Sugar::UseOn { .. } = sugar {
                text.insert_str(0, "use ");
            }
            self.push("#{desugared from `");
            self.push(&text);
            self.push("`}");
        }
    }

    /// Whether `e`'s text begins (or ends) with a struct literal a condition
    /// would read as its block: the literal itself, or a binary or postfix
    /// expression over one.
    fn holds_struct_lit(&self, e: &Expr) -> bool {
        if e.parens() > 0 {
            // [expr-group] Written in parentheses: no `{` of it is the block's.
            return false;
        }
        match e {
            Expr::StructLit { .. } => true,
            Expr::Binary { lhs, rhs, .. } => self.holds_struct_lit(lhs) || self.holds_struct_lit(rhs),
            Expr::Field { base, .. } | Expr::TupleIndex { base, .. } | Expr::Index { base, .. } => self.holds_struct_lit(base),
            Expr::Unary { operand, .. } => self.holds_struct_lit(operand),
            Expr::Is { subject, .. } => self.holds_struct_lit(subject),
            _ => false,
        }
    }

    /// The precedence an expression prints at.
    fn level(&self, e: &Expr) -> u8 {
        match e {
            Expr::Lambda { .. } | Expr::Retype { .. } => 0,
            Expr::Pipe { .. } | Expr::SafePipe { .. } => PIPE,
            Expr::When { origin: Some(WhenOrigin::SafePipe { .. }), .. } if self.reading == Reading::Source => PIPE,
            Expr::Binary { op, .. } => binary_level(*op),
            Expr::Is { .. } => COMPARE,
            Expr::Sugar(sugar) => match **sugar {
                Sugar::AsQual { .. } | Sugar::Is { .. } => COMPARE,
                Sugar::Pair { .. } => 0,
                // As a spawn or a mint: its clauses read greedily.
                Sugar::UseOn { .. } | Sugar::TrailingReply { .. } => 1,
            },
            Expr::Elvis { .. } => ELVIS,
            Expr::When { origin: Some(WhenOrigin::Pick), .. } if self.reading == Reading::Source => ELVIS,
            Expr::Unary { .. } | Expr::Spread { .. } => PREFIX,
            Expr::IncDec { prefix: true, .. } => PREFIX,
            Expr::Int { value, .. } if *value < 0 => PREFIX,
            Expr::Float { value, .. } if *value < 0.0 => PREFIX,
            // An escape's value reaches as far as an expression does.
            Expr::Return { value: Some(_), .. } | Expr::Break { value: Some(_), .. } => 1,
            // Spawn, use and replyto read their trailing clauses greedily.
            Expr::Spawn { .. } | Expr::Use { .. } | Expr::ReplyTo { .. } => 1,
            Expr::Field { .. }
            | Expr::TupleIndex { .. }
            | Expr::Scoped { .. }
            | Expr::EffectScoped { .. }
            | Expr::Call { .. }
            | Expr::Index { .. }
            | Expr::SafeNav { .. }
            | Expr::Project { .. }
            | Expr::IncDec { .. }
            | Expr::SelfScoped { .. }
            | Expr::SelfAddr { .. } => POSTFIX,
            Expr::When { origin: Some(WhenOrigin::SafeNav), .. } if self.reading == Reading::Source => POSTFIX,
            // A block-bodied expression is a primary, but one followed by
            // an operator on its closing line reads fine; as a postfix base
            // it needs parentheses (`(if c { a } else { b }).f`).
            Expr::If { .. } | Expr::When { .. } | Expr::WhenCond { .. } | Expr::While { .. } | Expr::For { .. } | Expr::Try { .. } | Expr::WaitFor { .. } => POWER,
            _ => PRIMARY,
        }
    }

    fn args(&mut self, args: &[Expr], named: &[NamedArg]) {
        self.args_in(args, named, None);
    }

    /// A call's arguments; `region`: the source range from its callee to
    /// its `)`, where a comment breaks them one per line [fmt-comments].
    fn args_in(&mut self, args: &[Expr], named: &[NamedArg], region: Option<(u32, u32)>) {
        let saved = std::mem::replace(&mut self.no_struct, false);
        self.push("(");
        let items: Vec<Arg> = args.iter().map(Arg::Pos).chain(named.iter().map(Arg::Named)).collect();
        let spans: Vec<Span> = items.iter().map(Arg::span).collect();
        match region.filter(|&(from, to)| self.list_holds_comment(from, to, &spans)) {
            Some((_, to)) => self.broken_list(&items, Arg::span, to.saturating_sub(1), |p, a| p.arg_item(a)),
            None => self.list(&items, ", ", |p, a| p.arg_item(a)),
        }
        self.push(")");
        self.no_struct = saved;
    }

    fn arg_item(&mut self, a: &Arg) {
        match a {
            Arg::Pos(a) => self.arg(a),
            Arg::Named(n) => {
                let s = self.at();
                self.ident(&n.name);
                self.push(" = ");
                self.arg(&n.value);
                let e = self.at();
                self.record(n.span, s, e, s, e);
            }
        }
    }

    /// A call argument, or either side of a pair.
    fn arg(&mut self, a: &Expr) {
        // [col-pairs] At the top of an argument a `:` is a pair's, so a
        // re-typing there is parenthesized.
        self.expr(a, if matches!(a, Expr::Retype { .. }) { 1 } else { 0 })
    }

    fn type_args(&mut self, targs: &[Type]) {
        self.type_args_in(targs, None);
    }

    /// Type arguments; `from`: the source offset they follow (a callee's
    /// end), so a comment written among them breaks them one per line
    /// [fmt-comments].
    fn type_args_in(&mut self, targs: &[Type], from: Option<Span>) {
        if targs.is_empty() {
            return;
        }
        self.push("<");
        let spans: Vec<Span> = targs.iter().map(Type::span).collect();
        let last = spans.iter().map(|s| s.end).max().unwrap_or(0);
        let close = from.filter(|f| self.anchored(*f)).and_then(|f| self.close_after(last.max(f.end), '>').map(|c| (f.end, c)));
        match close.filter(|&(f, c)| self.list_holds_comment(f, c + 1, &spans)) {
            Some((_, c)) => self.broken_list(targs, Type::span, c, |p, t| p.ty(t)),
            None => self.list(targs, ", ", |p, t| p.ty(t)),
        }
        self.push(">");
    }

    fn opt_message(&mut self, m: &Option<Box<Expr>>) {
        if let Some(m) = m {
            self.expr(m, 0);
        }
    }

    fn expr_inner(&mut self, e: &Expr) {
        match e {
            // [lit-spelling] A literal as written, where it was spelled
            // otherwise than the printer spells its value.
            Expr::Int { spelling: Some(text), .. } | Expr::Float { spelling: Some(text), .. } | Expr::Char { spelling: Some(text), .. } => self.push(text),
            Expr::Int { value, long, .. } => {
                let _ = write!(self.out, "{value}{}", if *long { "L" } else { "" });
            }
            Expr::Float { value, single, .. } => self.push(&float_text(*value, *single)),
            Expr::Bool { value, .. } => self.push(if *value { "true" } else { "false" }),
            Expr::Char { value, .. } => self.push(&quote_char(*value)),
            Expr::Str { parts, .. } => {
                self.push("\"");
                let saved = std::mem::replace(&mut self.no_struct, false);
                for part in parts {
                    match part {
                        StrExprPart::Text { spelling: Some(written), .. } => self.push(written),
                        StrExprPart::Text { text, spelling: None } => {
                            let mut s = String::new();
                            escape_str_into(text, &mut s);
                            self.push(&s);
                        }
                        StrExprPart::Interp(inner) => {
                            self.push("${");
                            let was = std::mem::replace(&mut self.inline, true);
                            self.expr(inner, 0);
                            self.inline = was;
                            self.push("}");
                        }
                    }
                }
                self.no_struct = saved;
                self.push("\"");
            }
            Expr::Ident(id) => {
                if self.holes.iter().any(|h| *h == id.name) {
                    self.ident_as(id, "_");
                } else if let Some(splice) = type_splice_text(&id.name) {
                    // [comptime-arm] `[arm.type](value)`'s callee.
                    self.ident_as(id, &splice);
                } else {
                    self.ident(id);
                }
            }
            Expr::Field { base, field, .. } => {
                self.expr(base, POSTFIX);
                self.push(".");
                self.ident(field);
            }
            Expr::TupleIndex { base, index, .. } => {
                self.expr(base, POSTFIX);
                let _ = write!(self.out, ".{index}");
            }
            Expr::Scoped { base, name, module, .. } => {
                if let Some(b) = base {
                    self.expr(b, POSTFIX);
                    self.push(".");
                }
                self.ident(name);
                // [comptime-hygiene] `name@[site]` in a stamped body is the
                // stamping site's `name`: the file it is printed in, where a
                // bare name means just that. In the template, as written, it
                // is written.
                match module.as_slice() {
                    [m] if m.name == SITE_SELECTOR => {
                        if e.span().is_written() {
                            self.push("@");
                            self.ident_as(m, "[site]");
                        }
                    }
                    _ => {
                        self.push("@");
                        self.list(module, ".", |p, m| p.ident(m));
                    }
                }
            }
            Expr::EffectScoped { base, name, effect, .. } => {
                if let Some(b) = base {
                    self.expr(b, POSTFIX);
                    self.push(".");
                }
                self.ident(name);
                self.push("@");
                self.ident(effect);
            }
            Expr::Call { callee, type_args, args, named, .. } => {
                // [comptime-call] `[to_json@demo(x)]`: the whole call in the
                // brackets, its callee named `[to_json@demo()]`.
                if let Expr::Ident(id) = callee.as_ref() {
                    if let Some(text) = id.name.strip_prefix('[').and_then(|n| n.strip_suffix("()]")) {
                        self.push("[");
                        let text = text.to_string();
                        self.ident_as(id, &text);
                        self.type_args(type_args);
                        self.args(args, named);
                        self.push("]");
                        return;
                    }
                }
                match callee.as_ref() {
                    Expr::Ident(id) if type_fn_callee(&id.name).is_some() => {
                        let s = self.at();
                        let text = type_fn_callee(&id.name).unwrap_or_default();
                        self.ident_as(id, &text);
                        let e = self.at();
                        self.record(callee.span(), s, e, s, e);
                    }
                    other => self.expr(other, POSTFIX),
                }
                self.type_args_in(type_args, Some(callee.span()));
                let from = type_args.last().map_or(callee.span().end, |t| t.span().end);
                self.args_in(args, named, Some((from, e.span().end)));
            }
            Expr::Index { base, index, .. } => {
                self.expr(base, POSTFIX);
                self.push("[");
                self.expr(index, 0);
                self.push("]");
            }
            Expr::Tuple { elems, span } => {
                let saved = std::mem::replace(&mut self.no_struct, false);
                self.push("(");
                let spans: Vec<Span> = elems.iter().map(Expr::span).collect();
                if self.list_holds_comment(span.start, span.end, &spans) {
                    self.broken_list(elems, Expr::span, span.end.saturating_sub(1), |p, x| p.expr(x, 0));
                } else {
                    self.list(elems, ", ", |p, x| p.expr(x, 0));
                }
                self.push(")");
                self.no_struct = saved;
            }
            Expr::StructLit { ty, fields, span } => {
                // [comptime-list-for] A `[...for p in s] { a, b }` head in a
                // call's arguments, carried as a struct literal of the one
                // entry, its values named `_arg`, spanning the head alone.
                if let (None, [StructLitField { kind: StructLitFieldKind::InlineFor { binder, seq, entries }, span: at }]) = (ty, fields.as_slice()) {
                    if at == span {
                        self.list_for_head(binder, seq);
                        let saved = std::mem::replace(&mut self.no_struct, false);
                        self.push("{ ");
                        self.list(entries, ", ", |p, f| match &f.kind {
                            StructLitFieldKind::Named { value, .. } => p.arg(value),
                            _ => p.push("#{an entry}"),
                        });
                        self.push(" }");
                        self.no_struct = saved;
                        return;
                    }
                }
                if let Some(t) = ty {
                    self.ty(t);
                    // `.S` alone is the fieldless literal of a dot-type.
                    if fields.is_empty() && type_text_starts_with_dot(t) {
                        return;
                    }
                    self.push(" ");
                }
                let from = ty.as_ref().map_or(span.start, |t| t.span().end);
                self.struct_fields(fields, Some((from, span.end)));
            }
            Expr::Unary { op, operand, .. } => {
                self.push(match op {
                    UnaryOp::Neg => "-",
                    UnaryOp::Not => "!",
                });
                // `- -x`, `!-x`: a sign after a sign stays apart.
                if matches!(op, UnaryOp::Neg) && starts_with_minus(operand) {
                    self.push(" ");
                }
                self.expr(operand, PREFIX);
            }
            Expr::Binary { op, lhs, rhs, .. } => {
                let level = binary_level(*op);
                if *op == BinaryOp::Pow {
                    self.expr(lhs, POSTFIX);
                    self.push(" ** ");
                    self.expr(rhs, PREFIX);
                } else {
                    self.expr(lhs, level);
                    self.push(" ");
                    self.push(binary_symbol(*op));
                    self.push(" ");
                    self.expr(rhs, level + 1);
                }
            }
            Expr::Is { subject, check, binding, .. } => {
                self.expr(subject, COMPARE);
                self.push(" is ");
                self.list(check, " ", |p, r| p.type_ref(r));
                if let Some(b) = binding {
                    // [expr-as-qual] `as?`'s binder is the compiler's name;
                    // written, it is a binder like any other.
                    self.push(" ");
                    self.ident(b);
                }
            }
            Expr::Retype { subject, retype, .. } => {
                self.expr(subject, 1);
                self.push(": ");
                self.retype(retype);
            }
            Expr::SafeNav { base, access, .. } => {
                self.expr(base, POSTFIX);
                self.push("?.");
                self.safe_access(access);
            }
            Expr::Assert { cond, message, .. } => {
                self.push("assert!(");
                self.expr(cond, 0);
                if let Some(m) = message {
                    self.push(", ");
                    self.expr(m, 0);
                }
                self.push(")");
            }
            // [type-assert] `type!(place: T)`.
            Expr::TypeAssert { place, ty, .. } => {
                self.push("type!(");
                self.expr(place, 1);
                self.push(": ");
                match ty {
                    Some(t) => self.ty(t),
                    // Refused at the parse; a module that parsed has none.
                    None => self.push("?"),
                }
                self.push(")");
            }
            // [type-query] `type?(place)`.
            Expr::TypeQuery { place, .. } => {
                self.push("type?(");
                self.expr(place, 0);
                self.push(")");
            }
            Expr::Unreachable { message, .. } => {
                self.push("unreachable!(");
                self.opt_message(message);
                self.push(")");
            }
            Expr::Todo { message, .. } => {
                self.push("todo!(");
                self.opt_message(message);
                self.push(")");
            }
            Expr::WhyNot { test, .. } => {
                self.push("whynot?(");
                self.expr(test, 0);
                self.push(")");
            }
            Expr::IncDec { operand, down, prefix, .. } => {
                let sym = if *down { "--" } else { "++" };
                if *prefix {
                    self.push(sym);
                    self.expr(operand, PREFIX);
                } else {
                    self.expr(operand, POSTFIX);
                    self.push(sym);
                }
            }
            Expr::If { branches, else_block, .. } => self.if_expr(branches, else_block),
            Expr::When { binding, subject, branches, origin, span } => self.when(binding, subject, branches, *origin, Some(span.end.saturating_sub(1))),
            Expr::WhenCond { branches, else_block, span } => {
                self.push("when {");
                self.indent += 1;
                let mut run = Run::default();
                for (c, b) in branches {
                    let arm = c.span().to(b.span);
                    self.elem_start(&mut run, arm, false, |_| false);
                    self.condition(c);
                    self.push(" ");
                    self.block(b);
                    self.elem_end(&mut run, arm);
                }
                // The `else` arm starts at its keyword, before its block.
                let at = else_block.span;
                self.elem_start(&mut run, at, false, |_| false);
                self.push("else ");
                self.block(else_block);
                self.elem_end(&mut run, at);
                if self.anchored(*span) {
                    self.run_close(&mut run, span.end.saturating_sub(1));
                }
                self.indent -= 1;
                self.newline();
                self.push("}");
            }
            Expr::While { cond, body, else_block, .. } => {
                self.push("while ");
                self.condition(cond);
                self.push(" ");
                self.block(body);
                if let Some(b) = else_block {
                    self.push(" else ");
                    self.block(b);
                }
            }
            Expr::For { pattern, iterable, body, else_block, .. } => {
                self.push("for ");
                self.pattern(pattern);
                self.push(" in ");
                self.condition(iterable);
                self.push(" ");
                self.block(body);
                if let Some(b) = else_block {
                    self.push(" else ");
                    self.block(b);
                }
            }
            Expr::Lambda { params, effects, deductions, ret, body, span } => {
                self.push("(");
                let param = |p: &mut Self, lp: &LambdaParam| {
                    let s = p.at();
                    p.ident(&lp.name);
                    if let Some(t) = &lp.ty {
                        p.push(": ");
                        p.ty(t);
                    }
                    let e = p.at();
                    p.record(lp.span, s, e, s, e);
                };
                // [fmt-comments] A comment among the parameters breaks them
                // one per line.
                let spans: Vec<Span> = params.iter().map(|p| p.span).collect();
                let close = self.anchored(*span).then(|| self.close_after(spans.iter().map(|s| s.end).max().unwrap_or(span.start), ')')).flatten();
                match close.filter(|&c| self.list_holds_comment(span.start, c + 1, &spans)) {
                    Some(c) => self.broken_list(params, |p| p.span, c, param),
                    None => self.list(params, ", ", param),
                }
                self.push(")");
                if let Some(effects) = effects {
                    self.push(" ");
                    self.effect_list(effects);
                }
                if let Some(ds) = deductions {
                    if !ds.is_empty() {
                        self.push(" => ");
                        self.deductions(&ds.iter().collect::<Vec<_>>());
                    }
                }
                self.push(" -> ");
                match body {
                    LambdaBody::Block(b) => {
                        if let Some(t) = ret {
                            self.ty(t);
                            self.push(" ");
                        }
                        self.block(b);
                    }
                    LambdaBody::Expr(x) => {
                        if let Some(t) = ret {
                            // A typed lambda's body is a block.
                            self.ty(t);
                            self.push(" { ");
                            self.expr(x, 0);
                            self.push(" }");
                        } else {
                            // A brace would open a block, and a type before
                            // one would be the lambda's return type.
                            let braced = matches!(**x, Expr::StructLit { .. });
                            self.expr(x, if braced { PRIMARY + 1 } else { 0 });
                        }
                    }
                    LambdaBody::Terminal => self.push("..."),
                }
            }
            Expr::Try { body, .. } => {
                self.push("try ");
                self.block(body);
            }
            Expr::Elvis { subject, rhs, .. } => {
                self.expr(subject, ELVIS);
                self.push(" ?: ");
                self.expr(rhs, ELVIS + 1);
            }
            Expr::Pipe { lhs, rhs, .. } => {
                self.expr(lhs, PIPE);
                self.push(" |> ");
                self.expr(rhs, PIPE + 1);
            }
            Expr::SafePipe { lhs, rhs, .. } => {
                self.expr(lhs, PIPE);
                self.push(" ?> ");
                self.expr(rhs, PIPE + 1);
            }
            Expr::Placeholder { .. } => self.push("_"),
            Expr::Return { value, .. } => {
                self.push("return");
                if let Some(v) = value {
                    self.push(" ");
                    self.expr(v, 0);
                }
            }
            Expr::Break { value, .. } => {
                self.push("break");
                if let Some(v) = value {
                    self.push(" ");
                    self.expr(v, 0);
                }
            }
            Expr::Continue { .. } => self.push("continue"),
            Expr::Spread { operand, .. } => {
                self.push("...");
                self.expr(operand, PREFIX);
            }
            Expr::SelfScoped { name, .. } => {
                self.ident(name);
                self.push("@self");
            }
            Expr::SelfAddr { face, .. } => {
                self.push("self@");
                self.type_ref(face);
            }
            Expr::Spawn { handler, with_items, pool, join, .. } => {
                self.push("spawn ");
                self.expr(handler, 1);
                self.with_items(with_items);
                if let Some(p) = pool {
                    self.push(" on ");
                    self.expr(p, 1);
                }
                if let Some(j) = join {
                    self.push(" in ");
                    self.expr(j, 1);
                }
            }
            Expr::Project { base, effects, .. } => {
                self.expr(base, POSTFIX);
                self.push("[");
                self.list(effects, ", ", |p, r| p.type_ref(r));
                self.push("]");
            }
            Expr::Merge { items, all, .. } => {
                self.push("eff[");
                if *all {
                    self.push("*");
                } else {
                    self.list(items, ", ", |p, x| p.expr(x, 0));
                }
                self.push("]");
            }
            Expr::Use { handler, with_items, .. } => {
                self.push("use ");
                self.expr(handler, 1);
                self.with_items(with_items);
            }
            Expr::ReplyTo { member, module, captures, gated, pool, .. } => {
                self.push(if *gated { "replyto! " } else { "replyto " });
                self.ident(member);
                if let Some(m) = module {
                    self.push("@");
                    self.list(m, ".", |p, id| p.ident(id));
                }
                self.args(captures, &[]);
                if let Some(p) = pool {
                    self.push(" on ");
                    self.expr(p, 1);
                }
            }
            Expr::WaitFor { binding, ty, body, .. } => {
                self.push("waitfor ");
                self.ident(binding);
                if let Some(t) = ty {
                    self.push(": ");
                    self.ty(t);
                }
                self.push(" ");
                self.block(body);
            }
            Expr::FieldDefault { owner, field, .. } => {
                if let Some(d) = self.notes.field_default(owner, field) {
                    return self.expr(&d, PRIMARY);
                }
                self.push("#{the default of ");
                self.ty(owner);
                self.push(".");
                self.ident(field);
                self.push("}");
            }
            Expr::Error { .. } => self.push("#{error}"),
            Expr::Sugar(sugar) => self.sugar(sugar),
        }
    }

    /// [syntax-lowered] A piece of sugar, as written.
    fn sugar(&mut self, sugar: &Sugar) {
        match sugar {
            Sugar::AsQual { subject, check, trap, .. } => {
                self.expr(subject, COMPARE);
                self.push(if *trap { " as! " } else { " as? " });
                self.list(check, " ", |p, r| p.type_ref(r));
            }
            Sugar::Is { subject, checks, binding, not, .. } => {
                self.expr(subject, COMPARE);
                self.push(if not.is_some() { " !is " } else { " is " });
                for (i, check) in checks.iter().enumerate() {
                    if i > 0 {
                        self.push(" | ");
                    }
                    self.list(check, " ", |p, r| p.type_ref(r));
                    if let (0, Some(b)) = (i, binding) {
                        self.push(" ");
                        self.ident(b);
                    }
                }
            }
            Sugar::Pair { key, value, .. } => {
                self.arg(key);
                self.push(": ");
                self.arg(value);
            }
            Sugar::UseOn { handler, with_items, pool, .. } => {
                self.expr(handler, 1);
                self.with_items(with_items);
                self.push(" on ");
                self.expr(pool, 1);
            }
            Sugar::TrailingReply { base, reply, .. } => {
                // A mint in the base would take the `replyto` as its own.
                let min = if matches!(base, Expr::ReplyTo { .. }) { 1 } else { POSTFIX };
                self.expr(base, min);
                self.push(" ");
                self.expr(reply, 1);
            }
        }
    }

    /// [comptime-list-for] `[...for binder in seq] `.
    fn list_for_head(&mut self, binder: &Ident, seq: &CompExpr) {
        self.push("[...for ");
        self.ident(binder);
        self.push(" in ");
        self.comp_expr(seq, 0);
        self.push("] ");
    }

    /// [comptime-inline] A comptime statement, as written.
    fn comp_stmt(&mut self, c: &CompStmt) {
        match c {
            CompStmt::Let { name, value, .. } => {
                self.push("[let ");
                self.ident(name);
                self.push(" = ");
                self.comp_expr(value, 0);
                self.push("]");
            }
            CompStmt::For { binder, seq, body, .. } => {
                self.push("[for ");
                self.ident(binder);
                self.push(" in ");
                self.comp_expr(seq, 0);
                self.push("] ");
                self.block(body);
            }
            CompStmt::If { cond, then, else_, .. } => {
                self.push("[if ");
                self.comp_expr(cond, 0);
                self.push("] ");
                self.block(then);
                if let Some(b) = else_ {
                    self.push(" else ");
                    self.block(b);
                }
            }
            CompStmt::WhenKind { ty, arms, .. } => {
                self.push("[when ");
                self.comp_ty(ty);
                self.push(".kind] {");
                self.indent += 1;
                let mut run = Run::default();
                for arm in arms {
                    self.elem_start(&mut run, arm.span, false, |_| false);
                    let s = self.at();
                    self.push("is ");
                    self.push(arm.kind.word());
                    self.push(" ");
                    self.block(&arm.body);
                    let e = self.at();
                    self.record(arm.span, s, e, s, e);
                    self.elem_end(&mut run, arm.span);
                }
                if self.anchored(c.span()) {
                    self.run_close(&mut run, c.span().end.saturating_sub(1));
                }
                self.indent -= 1;
                self.newline();
                self.push("}");
            }
            CompStmt::WhenType { ty, arms, .. } => {
                self.push("[when ");
                self.comp_ty(ty);
                self.push("] {");
                self.indent += 1;
                let mut run = Run::default();
                for arm in arms {
                    self.elem_start(&mut run, arm.span, false, |_| false);
                    let s = self.at();
                    self.push("is ");
                    self.comp_binders(&arm.binders);
                    self.in_comp_pattern(&arm.pattern);
                    self.push(" ");
                    self.block(&arm.body);
                    let e = self.at();
                    self.record(arm.span, s, e, s, e);
                    self.elem_end(&mut run, arm.span);
                }
                if self.anchored(c.span()) {
                    self.run_close(&mut run, c.span().end.saturating_sub(1));
                }
                self.indent -= 1;
                self.newline();
                self.push("}");
            }
            CompStmt::WhenName { binder, arms, .. } => {
                self.push("[when ");
                self.ident(binder);
                self.push(".name] {");
                self.indent += 1;
                let mut run = Run::default();
                for arm in arms {
                    self.elem_start(&mut run, arm.span, false, |_| false);
                    let s = self.at();
                    self.push("is ");
                    self.list(&arm.lits, " | ", |p, (lit, _)| p.push(&quote_str(lit)));
                    self.push(" ");
                    self.block(&arm.body);
                    let e = self.at();
                    self.record(arm.span, s, e, s, e);
                    self.elem_end(&mut run, arm.span);
                }
                if self.anchored(c.span()) {
                    self.run_close(&mut run, c.span().end.saturating_sub(1));
                }
                self.indent -= 1;
                self.newline();
                self.push("}");
            }
            CompStmt::WhenArms { value, binder, body, .. } => {
                self.push("[when ");
                self.condition(value);
                self.push("] {");
                self.indent += 1;
                self.newline();
                self.push("[");
                self.ident(binder);
                self.push("] ");
                self.block(body);
                self.indent -= 1;
                self.newline();
                self.push("}");
            }
            CompStmt::Refuse { message, .. } => {
                self.push("refuse!(");
                self.expr(message, 0);
                self.push(")");
            }
        }
    }

    /// [comptime-kind] The type a `[when …]` dispatches over: `T`, `Arg<T>`,
    /// `field.type`.
    fn comp_ty(&mut self, t: &CompTy) {
        let s = self.at();
        self.ident(&t.root);
        self.type_args(&t.args);
        if t.via_type {
            self.push(".type");
        }
        let e = self.at();
        self.record(t.span, s, e, s, e);
    }

    /// [comptime-pattern] `<E, k> ` before a pattern.
    fn comp_binders(&mut self, binders: &[Ident]) {
        if binders.is_empty() {
            return;
        }
        self.push("<");
        self.list(binders, ", ", |p, b| p.ident(b));
        self.push("> ");
    }

    /// [comptime-pattern] A type pattern, where `*` is any type.
    fn in_comp_pattern(&mut self, t: &Type) {
        self.ty(t);
    }

    /// [comptime-eval] A compile-time expression in a context accepting
    /// precedence `min` and tighter (the levels of `parse_comp_expr`: `||`
    /// 4, `&&` 6, a comparison or `is` 8, `+ -` 14, `* / %` 16, prefix 18,
    /// a projection or a primary 20; a lambda 0).
    fn comp_expr(&mut self, e: &CompExpr, min: u8) {
        let level = match e {
            CompExpr::Lambda { .. } => 0,
            CompExpr::Binary { op: BinaryOp::Or, .. } => 4,
            CompExpr::Binary { op: BinaryOp::And, .. } => 6,
            CompExpr::Binary { op: BinaryOp::Add | BinaryOp::Sub, .. } => 14,
            CompExpr::Binary { op: BinaryOp::Mul | BinaryOp::Div | BinaryOp::Rem, .. } => 16,
            CompExpr::Binary { .. } | CompExpr::Is { .. } => 8,
            // [comptime-eval] `x !is P` is a `Not` of the test spanning it.
            CompExpr::Not(inner, span) if matches!(**inner, CompExpr::Is { .. }) && inner.span() == *span => 8,
            CompExpr::Not(..) | CompExpr::Neg(..) => 18,
            CompExpr::Int { value, .. } if *value < 0 => 18,
            _ => 20,
        };
        // [expr-group] The parentheses written, or one pair where the
        // context needs it.
        let parens = match e.span().parens {
            0 => u8::from(level < min),
            written => written,
        };
        for _ in 0..parens {
            self.push("(");
        }
        let s = self.at();
        match e {
            CompExpr::Bool(b, _) => self.push(if *b { "true" } else { "false" }),
            CompExpr::Int { value, long, .. } => {
                let _ = write!(self.out, "{value}{}", if *long { "L" } else { "" });
            }
            CompExpr::Char(c, _) => self.push(&quote_char(*c)),
            CompExpr::Str(t, _) => self.push(&quote_str(t)),
            CompExpr::None(_) => self.push("None"),
            CompExpr::Name { id, args, .. } => {
                self.ident(id);
                self.type_args(args);
            }
            CompExpr::Proj { base, field, .. } => {
                self.comp_expr(base, 20);
                self.push(".");
                self.ident(field);
            }
            CompExpr::Call { callee, args, .. } => {
                self.ident(callee);
                self.push("(");
                self.list(args, ", ", |p, a| p.comp_expr(a, 0));
                self.push(")");
            }
            CompExpr::Lambda { params, body, .. } => {
                self.push("(");
                self.list(params, ", ", |p, id| p.ident(id));
                self.push(") -> ");
                self.comp_expr(body, 0);
            }
            CompExpr::Not(inner, span) if matches!(**inner, CompExpr::Is { .. }) && inner.span() == *span => {
                self.comp_is(inner, true);
            }
            CompExpr::Not(inner, _) => {
                self.push("!");
                self.comp_expr(inner, 18);
            }
            CompExpr::Neg(inner, _) => {
                self.push("-");
                if matches!(**inner, CompExpr::Neg(..)) {
                    self.push(" ");
                }
                self.comp_expr(inner, 18);
            }
            CompExpr::Binary { op, lhs, rhs, .. } => {
                let (l, r) = match op {
                    BinaryOp::Or => (4, 6),
                    BinaryOp::And => (6, 8),
                    BinaryOp::Add | BinaryOp::Sub => (14, 16),
                    BinaryOp::Mul | BinaryOp::Div | BinaryOp::Rem => (16, 18),
                    // A comparison does not chain: both sides are sums.
                    _ => (14, 14),
                };
                self.comp_expr(lhs, l);
                self.push(" ");
                self.push(binary_symbol(*op));
                self.push(" ");
                self.comp_expr(rhs, r);
            }
            CompExpr::Is { .. } => self.comp_is(e, false),
        }
        let end = self.at();
        for _ in 0..parens {
            self.push(")");
        }
        self.record(e.span(), s, end, s, end);
    }

    /// [comptime-eval] `subject is <B> pattern` / `subject !is "a" | "b"`.
    fn comp_is(&mut self, e: &CompExpr, negated: bool) {
        let CompExpr::Is { subject, binders, pattern, .. } = e else { return };
        self.comp_expr(subject, 14);
        self.push(if negated { " !is " } else { " is " });
        self.comp_binders(binders);
        match pattern {
            CompPattern::Type(t) => self.in_comp_pattern(t),
            CompPattern::Strs(lits) => self.list(lits, " | ", |p, (lit, _)| p.push(&quote_str(lit))),
        }
    }

    fn safe_access(&mut self, access: &SafeAccess) {
        match access {
            SafeAccess::Field(f) => self.ident(f),
            SafeAccess::Call { field, type_args, args, named } => {
                self.ident(field);
                self.type_args(type_args);
                self.args(args, named);
            }
        }
    }

    /// A struct literal's `{ … }`; `region`: the literal's source range,
    /// where a comment breaks it one field per line [fmt-comments].
    fn struct_fields(&mut self, fields: &[StructLitField], region: Option<(u32, u32)>) {
        let saved = std::mem::replace(&mut self.no_struct, false);
        let spans: Vec<Span> = fields.iter().map(|f| f.span).collect();
        let broken = region.filter(|&(from, to)| self.list_holds_comment(from, to, &spans));
        if let Some((_, to)) = broken {
            self.push("{");
            self.broken_list(fields, |f| f.span, to.saturating_sub(1), |p, f| p.struct_field(f));
            self.push("}");
        } else if fields.is_empty() {
            self.push("{}");
        } else {
            self.push("{ ");
            self.list(fields, ", ", |p, f| p.struct_field(f));
            self.push(" }");
        }
        self.no_struct = saved;
    }

    fn struct_field(&mut self, f: &StructLitField) {
        let s = self.at();
        match &f.kind {
            // [struct-lit-shorthand] `{ x }`: the parser reads it as
            // `{ x: x }` with the one identifier as both, so the
            // value's span is the name's. A dump writes it out, so
            // the value can say what it resolved to.
            StructLitFieldKind::Named { name, value: Expr::Ident(v) } if v.name == name.name && v.span == name.span && self.reading == Reading::Source => {
                self.ident(name);
            }
            StructLitFieldKind::Named { name, value } => {
                self.ident(name);
                self.push(": ");
                self.expr(value, 0);
            }
            StructLitFieldKind::Spread(v) => {
                self.push("...");
                self.expr(v, 0);
            }
            StructLitFieldKind::InlineFor { binder, seq, entries } => {
                self.list_for_head(binder, seq);
                self.struct_fields(entries, None);
            }
        }
        let e = self.at();
        self.record(f.span, s, e, s, e);
    }

    /// A condition: a struct literal there is parenthesized.
    fn condition(&mut self, e: &Expr) {
        let saved = std::mem::replace(&mut self.no_struct, true);
        self.expr(e, 0);
        self.no_struct = saved;
    }

    fn if_expr(&mut self, branches: &[(Expr, Block)], else_block: &Option<Block>) {
        // [expr-as-qual] `x as! Q` is written out as the `if` it is; say so.
        let as_qual = as_qual_name(branches);
        for (i, (c, b)) in branches.iter().enumerate() {
            self.push(if i == 0 { "if " } else { " elif " });
            self.condition(c);
            self.push(" ");
            // [fmt-comments] A comment written between a branch's `}` and
            // the `elif` or `else` after it closes the branch's block.
            let next = branches.get(i + 1).map(|(c, _)| c.span()).or(else_block.as_ref().map(|e| e.span));
            let close = match next {
                Some(n) if self.anchored(n) && self.anchored(b.span) => n.start,
                _ => b.span.end.saturating_sub(1),
            };
            match (&as_qual, i) {
                (Some(what), 0) => self.noted_block(b, what),
                _ => self.block_to(b, close),
            }
        }
        if let Some(b) = else_block {
            self.push(" else ");
            self.block(b);
        }
    }

    /// A block whose first line is a comment.
    fn noted_block(&mut self, b: &Block, note: &str) {
        let s = self.at();
        let saved = std::mem::replace(&mut self.no_struct, false);
        self.push("{");
        self.indent += 1;
        self.newline();
        self.push("// ");
        self.push(note);
        self.stmts(&b.stmts);
        self.indent -= 1;
        self.newline();
        self.push("}");
        self.no_struct = saved;
        let e = self.at();
        self.record(b.span, s, e, s, e);
    }

    /// A `when`; `end`: the source offset of its `}`, before which its
    /// closing comments are [fmt-comments].
    fn when(&mut self, binding: &Option<Ident>, subject: &Expr, branches: &[WhenBranch], origin: Option<WhenOrigin>, end: Option<u32>) {
        // [when-lowered] For source, a lowered `?:`, `?.` or `?>` is written
        // as the operator it was: its arms are holes no `is` can spell. A
        // dump writes the `when`, to show what the lowering made of it.
        let source = self.reading == Reading::Source;
        let note = match origin {
            Some(_) if !source => self.operator_text(binding, subject, branches, origin),
            _ => None,
        };
        if let ([first, rest], Some(origin), true) = (branches, origin, source) {
            fn value(b: &WhenBranch) -> Option<&Expr> {
                match b.body.stmts.as_slice() {
                    [Stmt::Expr(e)] => Some(e),
                    _ => None,
                }
            }
            if let (Some(fv), Some(rv)) = (value(first), value(rest)) {
                match origin {
                    WhenOrigin::Pick => {
                        self.expr(subject, ELVIS);
                        self.push(" ?: ");
                        let hole = binding.as_ref().map(|b| b.name.clone());
                        if let Some(h) = &hole {
                            self.holes.push(h.clone());
                        }
                        self.expr(rv, ELVIS + 1);
                        if hole.is_some() {
                            self.holes.pop();
                        }
                        return;
                    }
                    WhenOrigin::SafePipe { .. } => {
                        self.expr(subject, PIPE);
                        self.push(" ?> ");
                        if let Some(b) = binding {
                            self.holes.push(b.name.clone());
                        }
                        self.expr(fv, PIPE + 1);
                        if binding.is_some() {
                            self.holes.pop();
                        }
                        return;
                    }
                    WhenOrigin::SafeNav => {
                        let link = match fv {
                            Expr::Field { field, .. } => Some((field, None)),
                            Expr::Call { callee, type_args, args, named, .. } => match callee.as_ref() {
                                Expr::Field { field, .. } => Some((field, Some((type_args, args, named)))),
                                _ => None,
                            },
                            _ => None,
                        };
                        if let Some((field, call)) = link {
                            self.expr(subject, POSTFIX);
                            self.push("?.");
                            self.ident(field);
                            if let Some((t, a, n)) = call {
                                self.type_args(t);
                                self.args(a, n);
                            }
                            return;
                        }
                    }
                }
            }
        }
        self.push("when ");
        if let Some(b) = binding {
            self.ident(b);
            self.push(" = ");
        }
        self.condition(subject);
        self.push(" {");
        self.indent += 1;
        if let Some(text) = note {
            self.newline();
            self.push("// desugared from `");
            self.push(&text);
            self.push("`");
        }
        let mut run = Run::default();
        for br in branches {
            self.elem_start(&mut run, br.span, false, |_| false);
            let s = self.at();
            self.push("is ");
            match br.hole {
                Some(hole) => {
                    self.push("#{");
                    self.push(hole_mark(hole));
                    self.push("}");
                }
                None => self.list(&br.check, " ", |p, r| p.type_ref(r)),
            }
            if let Some(b) = &br.binding {
                self.push(" ");
                self.ident(b);
            }
            for alt in &br.or {
                self.push(" | ");
                self.list(alt, " ", |p, r| p.type_ref(r));
            }
            self.push(" ");
            self.block(&br.body);
            let e = self.at();
            self.record(br.span, s, e, s, e);
            self.elem_end(&mut run, br.span);
        }
        if let Some(close) = end.filter(|_| self.anchored(subject.span())) {
            self.run_close(&mut run, close);
        }
        self.indent -= 1;
        self.newline();
        self.push("}");
    }
}

impl Printer<'_> {
    /// [when-lowered] The operator a lowered `when` was written as, on one
    /// line, for a dump's comment over it: the `when` printed for source by
    /// a printer of its own (its places are not this printing's). `None`
    /// when the source printing could not write the operator either.
    fn operator_text(&self, binding: &Option<Ident>, subject: &Expr, branches: &[WhenBranch], origin: Option<WhenOrigin>) -> Option<String> {
        let mut p = Printer::new(self.notes, Reading::Source);
        p.inline = true;
        p.when(binding, subject, branches, origin, None);
        // A block in it the source printing wrote across lines, as one.
        (!p.out.starts_with("when ")).then(|| p.out.lines().map(str::trim).collect::<Vec<_>>().join(" "))
    }
}

/// [expr-as-qual] What an `if` the lowering wrote for `x as! Q` / `x as? Q`
/// came from, by the name it binds (`as_N_v`, which no program writes): the
/// comment over its block, `desugared from `x as! Q``.
fn as_qual_name(branches: &[(Expr, Block)]) -> Option<String> {
    let is_as = |n: &str| n.strip_prefix("as_").and_then(|r| r.strip_suffix("_v")).is_some_and(|d| !d.is_empty() && d.bytes().all(|b| b.is_ascii_digit()));
    let [(cond, body)] = branches else { return None };
    let (subject, check, trap) = match (cond, body.stmts.as_slice()) {
        (Expr::Bool { value: true, .. }, [Stmt::Let { pattern: Pattern::Ident(id), value, .. }, Stmt::Expr(Expr::Assert { cond, .. }), ..]) if is_as(&id.name) => match cond.as_ref() {
            Expr::Is { check, .. } => (value, check, true),
            _ => return None,
        },
        (Expr::Is { binding: Some(b), subject, check, .. }, _) if is_as(&b.name) => (subject.as_ref(), check, false),
        _ => return None,
    };
    let span = subject.span();
    let written = Expr::Sugar(Box::new(Sugar::AsQual { subject: subject.clone(), check: check.clone(), trap, op: span, mark: span, span }));
    let mut p = Printer::new(&NoNotes, Reading::Source);
    p.inline = true;
    p.expr(&written, 0);
    Some(format!("desugared from `{}`", p.out))
}

/// [syntax-lowered] The sugar an expression the lowering wrote came from,
/// by the trace the tree keeps of it: for `x !is Q`, a `!` whose operand
/// starts before it; for `x is A | B`, an `||` whose right test starts where
/// it does (a written test starts at its own subject); for a pair `k: v`, a
/// 2-tuple that starts where its key does (a written one starts at `(`); for
/// `use H() on P`, a spawn that starts where its handler does (a written one
/// starts at `spawn`); for `send(a) replyto k()`, a call (or a mint) whose
/// last argument is a mint that ends where it does (a written one ends at
/// its `)`).
fn sugar_origin(e: &Expr) -> Option<Sugar> {
    match e {
        Expr::Unary { op: UnaryOp::Not, operand, span } if span.start > operand.span().start => {
            let (subject, checks, binding) = match operand.as_ref() {
                Expr::Is { subject, check, binding, .. } => ((**subject).clone(), vec![check.clone()], binding.clone()),
                other => {
                    let (subject, checks) = union_tests(other)?;
                    (subject, checks, None)
                }
            };
            Some(Sugar::Is { subject, checks, binding, not: Some(*span), span: *span })
        }
        Expr::Binary { op: BinaryOp::Or, span, .. } => {
            let (subject, checks) = union_tests(e)?;
            Some(Sugar::Is { subject, checks, binding: None, not: None, span: *span })
        }
        // [col-pairs] A pair's tuple spans its two sides only.
        Expr::Tuple { elems, span } if elems.len() == 2 && elems[0].span().start == span.start => {
            Some(Sugar::Pair { key: elems[0].clone(), value: elems[1].clone(), span: *span })
        }
        // [expr-replyto-flat] A call or a mint whose last argument is a mint
        // and ends where it does: no `)` follows it.
        Expr::Call { callee, type_args, args, named, span } if matches!(args.last(), Some(r @ Expr::ReplyTo { .. }) if r.span().end == span.end) => {
            let mut args = args.clone();
            let reply = args.pop()?;
            // A chain: the mint is itself one, written flat.
            let reply = match sugar_origin(&reply) {
                Some(s @ Sugar::TrailingReply { .. }) => Expr::Sugar(Box::new(s)),
                _ => reply,
            };
            let base = Expr::Call { callee: callee.clone(), type_args: type_args.clone(), args, named: named.clone(), span: *span };
            Some(Sugar::TrailingReply { base, reply, span: *span })
        }
        Expr::ReplyTo { member, module, captures, gated, pool: None, span } if matches!(captures.last(), Some(r @ Expr::ReplyTo { .. }) if r.span().end == span.end) => {
            let mut captures = captures.clone();
            let reply = captures.pop()?;
            // A chain: the mint is itself one, written flat.
            let reply = match sugar_origin(&reply) {
                Some(s @ Sugar::TrailingReply { .. }) => Expr::Sugar(Box::new(s)),
                _ => reply,
            };
            let base = Expr::ReplyTo { member: member.clone(), module: module.clone(), captures, gated: *gated, pool: None, span: *span };
            Some(Sugar::TrailingReply { base, reply, span: *span })
        }
        // [actor-use-addr] A `use … on`'s spawn starts at its handler.
        Expr::Spawn { handler, with_items, pool: Some(pool), join: None, span } if handler.span().start == span.start => {
            Some(Sugar::UseOn { handler: (**handler).clone(), with_items: with_items.clone(), pool: (**pool).clone(), span: *span })
        }
        _ => None,
    }
}

/// [is-union] The subject and the tests of an `||` chain a union `is` was
/// lowered to.
fn union_tests(e: &Expr) -> Option<(Expr, Vec<Vec<TypeRef>>)> {
    let Expr::Binary { op: BinaryOp::Or, lhs, rhs, span } = e else { return None };
    let Expr::Is { subject: last, check, binding: None, span: at } = rhs.as_ref() else { return None };
    if at.start != span.start {
        return None;
    }
    let (subject, mut checks) = match lhs.as_ref() {
        Expr::Is { subject, check, binding: None, .. } => ((**subject).clone(), vec![check.clone()]),
        other => union_tests(other)?,
    };
    if subject != **last {
        return None;
    }
    checks.push(check.clone());
    Some((subject, checks))
}

/// [op-compound] [syntax-lowered] The `x += e` an assignment the sugar
/// lowering wrote came from: its `Binary` spans the whole statement and its
/// left operand is the target itself, which no written `x = x + e` has (its
/// value starts after the `=`).
fn compound_origin(st: &Stmt) -> Option<Stmt> {
    let Stmt::Assign { target, value: Expr::Binary { op, lhs, rhs, span: bspan }, span } = st else { return None };
    (bspan == span && **lhs == *target).then(|| Stmt::CompoundAssign { target: target.clone(), op: *op, op_span: *span, value: (**rhs).clone(), span: *span })
}

/// [comptime-inline] `[field.type]` for a name the parser gave a type splice
/// (`field.type`, which no written name can be); `None` for any other.
fn type_splice_text(name: &str) -> Option<String> {
    let binder = name.strip_suffix(".type")?;
    (binder.starts_with(|c: char| c.is_lowercase()) && !binder.contains('.') && !binder.contains('[')).then(|| format!("[{name}]"))
}

fn starts_with_minus(e: &Expr) -> bool {
    if e.parens() > 0 {
        return false;
    }
    match e {
        Expr::Unary { op: UnaryOp::Neg, .. } => true,
        Expr::Int { value, .. } => *value < 0,
        Expr::Float { value, .. } => *value < 0.0,
        Expr::IncDec { prefix: true, down: true, .. } => true,
        _ => false,
    }
}

fn type_text_starts_with_dot(t: &Type) -> bool {
    matches!(t, Type::Named { qualifiers, base } if qualifiers.is_empty() && base.name.name.starts_with('.'))
}

/// [fmt-blank-lines] An element of a module's (or a namespace's) run.
#[derive(Clone, Copy)]
enum Top<'a> {
    Item(&'a Item),
    Use(&'a ModuleUse),
}

impl Top<'_> {
    fn span(&self) -> Span {
        match self {
            Top::Use(u) => u.span,
            Top::Item(item) => item.span(),
        }
    }

    /// [doc-comment] Whether a comment run directly above it is its docs.
    fn takes_docs(&self) -> bool {
        !matches!(self, Top::Use(_) | Top::Item(Item::Import(_)))
    }

    fn is_import_or_use(&self) -> bool {
        matches!(self, Top::Use(_) | Top::Item(Item::Import(_)))
    }

    /// [fmt-blank-lines] Whether it prints on one line, its docs aside: no
    /// body with anything in it.
    fn one_line(&self, attached: &HashMap<&str, Vec<&FnDecl>>) -> bool {
        let Top::Item(item) = self else { return true };
        let no_fns = |name: &str| attached.get(name).is_none_or(|v| v.is_empty());
        match item {
            Item::Import(_) => true,
            Item::EffectAlias(_) | Item::Rename(_) | Item::Refn(_) => true,
            Item::Type(t) => t.fns.is_empty() && t.refns.is_empty() && no_fns(&t.name.name),
            Item::Struct(st) => {
                st.message_arm.is_none() && st.fields.is_empty() && st.fns.is_empty() && st.refns.is_empty() && no_fns(&st.name.name)
            }
            Item::Fn(f) => f.body.as_ref().is_none_or(|b| b.stmts.is_empty()) && !f.is_iter,
            Item::Effect(e) => e.fns.is_empty(),
            Item::Qualifier(q) => !q.has_body || (q.field_overrides.is_empty() && q.fns.is_empty() && q.refns.is_empty()),
            Item::Fns(g) => g.fns.is_empty(),
            Item::Handler(_) | Item::Namespace(_) | Item::Test(_) => false,
        }
    }
}

/// [fmt-comments] Where a parameter list may start: `name`'s span, reaching
/// to the end of the generics written after it (a comment among those is
/// theirs, not the list's).
fn after_generics(name: Span, generics: &[Ident], canbe: &[(Ident, TypeRef)]) -> Span {
    let end = generics.iter().map(|g| g.span.end).chain(canbe.iter().map(|(_, q)| q.span.end)).fold(name.end, u32::max);
    Span { end, ..name }
}

/// A parameter list's entry: a parameter, or an implicit group `?G`.
enum ParamItem<'a> {
    Param(&'a Param),
    Group(&'a TypeRef),
}

impl ParamItem<'_> {
    fn span(&self) -> Span {
        match self {
            ParamItem::Param(p) => p.span,
            ParamItem::Group(g) => g.span,
        }
    }
}

/// A call argument: positional, or named.
enum Arg<'a> {
    Pos(&'a Expr),
    Named(&'a NamedArg),
}

impl Arg<'_> {
    fn span(&self) -> Span {
        match self {
            Arg::Pos(e) => e.span(),
            Arg::Named(n) => n.span,
        }
    }
}

/// [fmt-comments] A member of a declaration's body.
enum Member<'a> {
    Field(&'a FieldDecl),
    /// A fn, and whether its `export` is printed.
    Fn(&'a FnDecl, bool),
    /// A fn of a `fns` group.
    Grouped(&'a FnDecl),
    Refn(&'a RefnDecl),
    /// A handler's `mailbox { … }`.
    Mailbox(&'a Expr),
    /// A handler's `state { … }`.
    State(&'a HandlerDecl),
    Init(&'a FnDecl),
}

impl Member<'_> {
    fn span(&self, trivia: Option<&Trivia>) -> Span {
        match self {
            Member::Field(f) => f.span,
            Member::Fn(f, _) | Member::Grouped(f) | Member::Init(f) => f.span,
            Member::Refn(r) => r.span,
            Member::Mailbox(m) => m.span(),
            // From its first field (or its name) to its `}`: what the
            // comments around it are placed by.
            Member::State(h) => {
                let first = h.state_name.as_ref().map(|n| n.span).or(h.state.first().map(|f| f.span)).unwrap_or(h.span);
                let last = h.state.last().map(|f| f.span.end).unwrap_or(first.end);
                let end = trivia.and_then(|t| t.close_after(last, '}')).map_or(last, |c| c + 1);
                Span { end, ..first }
            }
        }
    }

    /// [doc-comment] Whether a comment run directly above it is its docs.
    fn takes_docs(&self) -> bool {
        !matches!(self, Member::Mailbox(_) | Member::State(_) | Member::Init(_))
    }
}

/// The name a declaration item declares, for its namespace.
fn item_name(item: &Item) -> Option<&str> {
    Some(match item {
        Item::Type(t) => &t.name.name,
        Item::Struct(s) => &s.name.name,
        Item::Qualifier(q) => &q.name.name,
        Item::Effect(e) => &e.name.name,
        Item::Handler(h) => &h.name.name,
        Item::Fns(g) => &g.name.name,
        Item::Fn(f) => &f.name.name,
        Item::EffectAlias(a) => &a.name.name,
        _ => return None,
    })
}

const MESSAGE_TYPE: &str = crate::known::MESSAGE_TYPE;
