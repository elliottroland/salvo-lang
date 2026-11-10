//! [ast-print] A module's tree printed back as Salvo source.
//!
//! The printer reads a `Module` — as parsed, or after the expansions and
//! desugarings a program's driver runs — and writes the source a reader
//! would have written for it: one declaration per paragraph, four-space
//! indentation, every expression on one line except the blocks it holds.
//! Layout is the printer's, not the file's, and comments inside bodies are
//! not kept (doc comments are: they are part of the tree).
//!
//! It also records where it printed every node whose span is numbered
//! [node-id] (`Printed::nodes`, keyed by the node's file tag and ordinal), so a
//! dump that annotates nodes places its annotations in the printed text by
//! node, not by the node's offset in the file it came from — which is what
//! lets a dump show code an expansion wrote (a comptime stamp's body, a
//! hidden fn), whose spans are its template's.
//!
//! The parser keeps no grouping parentheses, so the printer adds them where
//! precedence needs them (the parser's binding powers); an expression's
//! recorded `outer` range includes them.
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
pub fn print_module_with(module: &Module, notes: &dyn Notes, reading: Reading) -> Printed {
    let mut p = Printer::new(notes, reading);
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
    /// [expr-terminal-scope] The blocks the module wrote as a terminal `...`
    /// (`Module::terminal_bodies`).
    terminal: std::collections::HashSet<Span>,
    /// [expr-terminal-scope] The terminal body the statement being printed
    /// holds, printed after it.
    pending_rest: Option<Block>,
}

impl<'n> Printer<'n> {
    fn new(notes: &'n dyn Notes, reading: Reading) -> Self {
        Printer { out: String::new(), nodes: HashMap::new(), indent: 0, notes, no_struct: false, holes: Vec::new(), inline: false, reading, quiet_origin: false, terminal: Default::default(), pending_rest: None }
    }
}

impl Printer<'_> {
    fn at(&self) -> u32 {
        self.out.len() as u32
    }

    fn push(&mut self, s: &str) {
        self.out.push_str(s);
    }

    fn record(&mut self, span: Span, start: u32, end: u32, outer_start: u32, outer_end: u32) {
        if let Some(k) = node_key(span) {
            self.nodes.entry(k).or_insert(Placed { start, end, outer_start, outer_end });
        }
    }

    fn newline(&mut self) {
        // No trailing spaces on a blank line.
        while self.out.ends_with(' ') {
            self.out.pop();
        }
        self.out.push('\n');
        for _ in 0..self.indent {
            self.out.push_str("    ");
        }
    }

    fn comment_lines(&mut self, lines: &[String]) {
        for l in lines {
            if l.is_empty() {
                self.push("//");
            } else {
                self.push("// ");
                self.push(l);
            }
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
        self.terminal = m.terminal_bodies.iter().copied().collect();
        if !m.docs.is_empty() {
            self.comment_lines(&m.docs);
            self.newline();
        }
        let mut first = true;
        // The imports, as one paragraph.
        for item in &m.items {
            if let Item::Import(i) = item {
                self.import(i);
                self.newline();
                first = false;
            }
        }
        for u in &m.uses {
            let s = self.at();
            self.push("use ");
            self.expr(&u.handler, 0);
            let e = self.at();
            self.record(u.span, s, e, s, e);
            self.newline();
            first = false;
        }
        // [fn-attached] A struct's fns go back in its body.
        let mut attached: HashMap<&str, Vec<&FnDecl>> = HashMap::new();
        let structs: std::collections::HashSet<&str> = m
            .items
            .iter()
            .filter_map(|i| match i {
                Item::Struct(s) => Some(s.name.name.as_str()),
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
        for item in &m.items {
            match item {
                Item::Import(_) => continue,
                Item::Fn(f) if f.scoped_to.as_ref().is_some_and(|o| structs.contains(o.name.as_str())) => continue,
                _ => {}
            }
            if !first {
                self.newline();
            }
            first = false;
            self.namespaced(item, &m.namespaces, &attached);
            self.newline();
        }
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
            Item::Type(t) => self.type_decl(t, strip),
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
                self.comment_lines(&a.docs);
                if a.exported {
                    self.push("export ");
                }
                self.push("effects ");
                self.decl_name(&a.name, strip);
                self.push(" = ");
                self.effect_list(&a.effects);
            }
            Item::Namespace(n) => {
                self.comment_lines(&n.docs);
                if n.exported {
                    self.push("export ");
                }
                self.push("namespace ");
                self.ident(&n.name);
                self.push(" {");
                self.indent += 1;
                for (i, it) in n.items.iter().enumerate() {
                    if i > 0 {
                        self.newline();
                    }
                    self.newline();
                    self.item(it, None, attached);
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

    fn type_decl(&mut self, t: &TypeDecl, strip: Option<usize>) {
        self.comment_lines(&t.docs);
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
        if !t.refns.is_empty() {
            self.push(" {");
            self.indent += 1;
            for r in &t.refns {
                self.newline();
                self.refn(r);
            }
            self.indent -= 1;
            self.newline();
            self.push("}");
        }
        let e = self.at();
        self.record(t.span, s, e, s, e);
    }

    fn struct_decl(&mut self, st: &StructDecl, strip: Option<usize>, fns: &[&FnDecl]) {
        self.comment_lines(&st.docs);
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
        self.indent += 1;
        for f in &st.fields {
            self.newline();
            self.field(f);
        }
        let own: Vec<&FnDecl> = st.fns.iter().chain(fns.iter().copied()).collect();
        for (i, f) in own.iter().enumerate() {
            if i > 0 || !st.fields.is_empty() {
                self.newline();
            }
            self.newline();
            self.fn_decl(f, None, false);
        }
        for r in &st.refns {
            self.newline();
            self.newline();
            self.refn(r);
        }
        self.indent -= 1;
        if st.fields.is_empty() && own.is_empty() && st.refns.is_empty() {
            self.push("}");
        } else {
            self.newline();
            self.push("}");
        }
        let e = self.at();
        self.record(st.span, s, e, s, e);
    }

    fn field(&mut self, f: &FieldDecl) {
        self.comment_lines(&f.docs);
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
        self.comment_lines(&q.docs);
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
            self.indent += 1;
            for f in &q.field_overrides {
                self.newline();
                self.field(f);
            }
            for (i, f) in q.fns.iter().enumerate() {
                if i > 0 || !q.field_overrides.is_empty() {
                    self.newline();
                }
                self.newline();
                self.fn_decl(f, None, true);
            }
            for r in &q.refns {
                self.newline();
                self.newline();
                self.refn(r);
            }
            self.indent -= 1;
            if q.field_overrides.is_empty() && q.fns.is_empty() && q.refns.is_empty() {
                self.push("}");
            } else {
                self.newline();
                self.push("}");
            }
        }
        let e = self.at();
        self.record(q.span, s, e, s, e);
    }

    fn effect(&mut self, ef: &EffectDecl, strip: Option<usize>) {
        self.comment_lines(&ef.docs);
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
        self.indent += 1;
        for (i, f) in ef.fns.iter().enumerate() {
            if i > 0 {
                self.newline();
            }
            self.newline();
            self.fn_decl(f, None, true);
        }
        self.indent -= 1;
        if !ef.fns.is_empty() {
            self.newline();
        }
        self.push("}");
        let e = self.at();
        self.record(ef.span, s, e, s, e);
    }

    fn handler(&mut self, h: &HandlerDecl, strip: Option<usize>) {
        self.comment_lines(&h.docs);
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
            self.params(&h.params, &[]);
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
            self.indent += 1;
            let mut first = true;
            let mut gap = |p: &mut Self| {
                if !first {
                    p.newline();
                }
                first = false;
                p.newline();
            };
            if let Some(Expr::StructLit { fields, span, .. }) = &h.mailbox {
                gap(self);
                let s = self.at();
                self.push("mailbox ");
                self.struct_fields(fields);
                let e = self.at();
                self.record(*span, s, e, s, e);
            }
            if !h.state.is_empty() || h.state_name.is_some() {
                gap(self);
                self.push("state ");
                if let Some(n) = &h.state_name {
                    self.ident(n);
                    self.push(" ");
                }
                self.push("{");
                self.indent += 1;
                for f in &h.state {
                    self.newline();
                    self.field(f);
                }
                self.indent -= 1;
                if !h.state.is_empty() {
                    self.newline();
                }
                self.push("}");
            }
            if let Some(init) = &h.init {
                gap(self);
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
            for f in &h.fns {
                gap(self);
                self.fn_decl(f, None, true);
            }
            self.indent -= 1;
            self.newline();
            self.push("}");
        }
        let e = self.at();
        self.record(h.span, s, e, s, e);
    }

    fn fns_group(&mut self, g: &FnsDecl, strip: Option<usize>) {
        self.comment_lines(&g.docs);
        let s = self.at();
        if g.exported {
            self.push("export ");
        }
        self.push("fns ");
        self.decl_name(&g.name, strip);
        self.generics(&g.generics, &[], &g.effect_generics);
        if !g.deductions.is_empty() {
            self.push(" => ");
            self.list(&g.deductions, ", ", |p, d| p.deduction(d));
        }
        self.push(" {");
        self.indent += 1;
        for f in &g.fns {
            self.newline();
            self.fn_decl(f, None, true);
        }
        self.indent -= 1;
        if !g.fns.is_empty() {
            self.newline();
        }
        self.push("}");
        let e = self.at();
        self.record(g.span, s, e, s, e);
    }

    fn refn(&mut self, r: &RefnDecl) {
        self.comment_lines(&r.docs);
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
        self.comment_lines(&r.docs);
        let s = self.at();
        self.push("rename fn ");
        self.ident(&r.name);
        self.push(" = ");
        self.ident(&r.target);
        self.generics(&r.generics, &[], &[]);
        self.params(&r.params, &[]);
        let e = self.at();
        self.record(r.span, s, e, s, e);
    }

    fn test(&mut self, t: &TestDecl) {
        self.comment_lines(&t.docs);
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
            self.params(&t.params, &[]);
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
                self.comment_lines(&f.docs);
                if f.exported && show_export {
                    self.push("export ");
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
        self.comment_lines(&f.docs);
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
        self.params(&f.params, &f.implicit_groups);
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
            self.list(&own, ", ", |p, d| p.deduction(d));
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
                    self.list(&written, ", ", |p, d| p.deduction(d));
                }
            }
        }
        if let Some(by) = &f.by {
            self.by(by);
        }
        if f.is_iter {
            self.push(" {");
            self.indent += 1;
            self.newline();
            self.push("state ");
            if let Some(n) = &f.iter_state_name {
                self.ident(n);
                self.push(" ");
            }
            self.push("{");
            self.indent += 1;
            for fd in &f.iter_state {
                self.newline();
                self.field(fd);
            }
            self.indent -= 1;
            if !f.iter_state.is_empty() {
                self.newline();
            }
            self.push("}");
            if let Some(b) = &f.body {
                self.stmts(&b.stmts);
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

    fn params(&mut self, params: &[Param], groups: &[TypeRef]) {
        self.push("(");
        let mut first = true;
        for param in params.iter().filter(|p| !p.implicit).chain(params.iter().filter(|p| p.implicit)) {
            if !first {
                self.push(", ");
            }
            first = false;
            self.param(param);
        }
        for g in groups {
            if !first {
                self.push(", ");
            }
            first = false;
            self.push("?");
            self.type_ref(g);
        }
        self.push(")");
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

    fn ty(&mut self, t: &Type) {
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
                if matches!(**base, Type::Literal { .. }) {
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
                        let paren = matches!(a, Type::Fn { .. } | Type::Union { .. });
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
                let paren = matches!(**elem, Type::Fn { .. } | Type::Union { .. })
                    || matches!(&**elem, Type::Named { qualifiers, .. } if !qualifiers.is_empty())
                    || matches!(**elem, Type::QualifiedGroup { .. });
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
            self.ident(&r.name);
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
                self.list(&r.args, ", ", |p, a| p.ty(a));
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
        let s = self.at();
        // [expr-terminal-scope] Written `...`: its statements follow the
        // statement that holds it (`stmts`).
        if self.terminal.contains(&b.span) && !self.inline {
            self.push("...");
            let e = self.at();
            self.record(b.span, s, e, s, e);
            self.pending_rest = Some(b.clone());
            return;
        }
        let saved = std::mem::replace(&mut self.no_struct, false);
        if b.stmts.is_empty() {
            self.push("{}");
        } else if self.inline {
            self.push("{ ");
            self.list(&b.stmts, " ", |p, st| p.stmt(st));
            self.push(" }");
        } else {
            self.push("{");
            self.indent += 1;
            self.stmts(&b.stmts);
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
        // A body printed inside a statement (the `else` after `if c ...`)
        // leaves the statement's own terminal body to the statement.
        let outer = self.pending_rest.take();
        for st in stmts {
            self.newline();
            self.stmt(st);
            // [expr-terminal-scope] The rest of the block a terminal `...`
            // in the statement stands for: the statements after it.
            if let Some(rest) = self.pending_rest.take() {
                self.stmts(&rest.stmts);
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
                self.push("#{a comptime statement}");
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

    /// `e` in a context accepting precedence `min` and tighter.
    fn expr(&mut self, e: &Expr, min: u8) {
        let level = self.level(e);
        let paren = level < min || (self.no_struct && self.holds_struct_lit(e));
        let outer_start = self.at();
        let saved = self.no_struct;
        if paren {
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
        if paren {
            self.push(")");
        }
        self.no_struct = saved;
        let outer_end = self.at();
        self.record(e.span(), start, end, outer_start, outer_end);
        if let Some(sugar) = origin {
            let text = self.source_text(|p| p.sugar(&sugar));
            self.push("#{desugared from `");
            self.push(&text);
            self.push("`}");
        }
    }

    /// Whether `e`'s text begins (or ends) with a struct literal a condition
    /// would read as its block: the literal itself, or a binary or postfix
    /// expression over one.
    fn holds_struct_lit(&self, e: &Expr) -> bool {
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
        let saved = std::mem::replace(&mut self.no_struct, false);
        self.push("(");
        let arg = |p: &mut Self, a: &Expr| p.arg(a);
        self.list(args, ", ", arg);
        for (i, n) in named.iter().enumerate() {
            if i > 0 || !args.is_empty() {
                self.push(", ");
            }
            let s = self.at();
            self.ident(&n.name);
            self.push(" = ");
            arg(self, &n.value);
            let e = self.at();
            self.record(n.span, s, e, s, e);
        }
        self.push(")");
        self.no_struct = saved;
    }

    /// A call argument, or either side of a pair.
    fn arg(&mut self, a: &Expr) {
        // [col-pairs] At the top of an argument a `:` is a pair's, so a
        // re-typing there is parenthesized.
        self.expr(a, if matches!(a, Expr::Retype { .. }) { 1 } else { 0 })
    }

    fn type_args(&mut self, targs: &[Type]) {
        if targs.is_empty() {
            return;
        }
        self.push("<");
        self.list(targs, ", ", |p, t| p.ty(t));
        self.push(">");
    }

    fn opt_message(&mut self, m: &Option<Box<Expr>>) {
        if let Some(m) = m {
            self.expr(m, 0);
        }
    }

    fn expr_inner(&mut self, e: &Expr) {
        match e {
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
                        StrExprPart::Text(t) => {
                            let mut s = String::new();
                            escape_str_into(t, &mut s);
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
                } else {
                    self.ident(id);
                }
            }
            Expr::Field { base, field, grouped, .. } => {
                if *grouped {
                    self.push("(");
                }
                self.expr(base, POSTFIX);
                self.push(".");
                self.ident(field);
                if *grouped {
                    self.push(")");
                }
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
                // bare name means just that.
                if !matches!(module.as_slice(), [m] if m.name == SITE_SELECTOR) {
                    self.push("@");
                    self.list(module, ".", |p, m| p.ident(m));
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
                self.type_args(type_args);
                self.args(args, named);
            }
            Expr::Index { base, index, .. } => {
                self.expr(base, POSTFIX);
                self.push("[");
                self.expr(index, 0);
                self.push("]");
            }
            Expr::Tuple { elems, .. } => {
                let saved = std::mem::replace(&mut self.no_struct, false);
                self.push("(");
                self.list(elems, ", ", |p, x| p.expr(x, 0));
                self.push(")");
                self.no_struct = saved;
            }
            Expr::StructLit { ty, fields, .. } => {
                if let Some(t) = ty {
                    self.ty(t);
                    // `.S` alone is the fieldless literal of a dot-type.
                    if fields.is_empty() && type_text_starts_with_dot(t) {
                        return;
                    }
                    self.push(" ");
                }
                self.struct_fields(fields);
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
            Expr::When { binding, subject, branches, origin, .. } => self.when(binding, subject, branches, *origin),
            Expr::WhenCond { branches, else_block, .. } => {
                self.push("when {");
                self.indent += 1;
                for (c, b) in branches {
                    self.newline();
                    self.condition(c);
                    self.push(" ");
                    self.block(b);
                }
                self.newline();
                self.push("else ");
                self.block(else_block);
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
            Expr::Lambda { params, effects, deductions, ret, body, .. } => {
                self.push("(");
                self.list(params, ", ", |p, lp| {
                    let s = p.at();
                    p.ident(&lp.name);
                    if let Some(t) = &lp.ty {
                        p.push(": ");
                        p.ty(t);
                    }
                    let e = p.at();
                    p.record(lp.span, s, e, s, e);
                });
                self.push(")");
                if let Some(effects) = effects {
                    self.push(" ");
                    self.effect_list(effects);
                }
                if let Some(ds) = deductions {
                    if !ds.is_empty() {
                        self.push(" => ");
                        self.list(ds, ", ", |p, d| p.deduction(d));
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

    fn struct_fields(&mut self, fields: &[StructLitField]) {
        let saved = std::mem::replace(&mut self.no_struct, false);
        if fields.is_empty() {
            self.push("{}");
        } else {
            self.push("{ ");
            self.list(fields, ", ", |p, f| {
                let s = p.at();
                match &f.kind {
                    // [struct-lit-shorthand] `{ x }`: the parser reads it as
                    // `{ x: x }` with the one identifier as both, so the
                    // value's span is the name's. A dump writes it out, so
                    // the value can say what it resolved to.
                    StructLitFieldKind::Named { name, value: Expr::Ident(v) } if v.name == name.name && v.span == name.span && p.reading == Reading::Source => {
                        p.ident(name);
                    }
                    StructLitFieldKind::Named { name, value } => {
                        p.ident(name);
                        p.push(": ");
                        p.expr(value, 0);
                    }
                    StructLitFieldKind::Spread(v) => {
                        p.push("...");
                        p.expr(v, 0);
                    }
                    StructLitFieldKind::InlineFor { binder, .. } => {
                        p.push("#{[...for ");
                        p.push(&binder.name);
                        p.push(" …]}");
                    }
                }
                let e = p.at();
                p.record(f.span, s, e, s, e);
            });
            self.push(" }");
        }
        self.no_struct = saved;
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
            match (&as_qual, i) {
                (Some(what), 0) => self.noted_block(b, what),
                _ => self.block(b),
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

    fn when(&mut self, binding: &Option<Ident>, subject: &Expr, branches: &[WhenBranch], origin: Option<WhenOrigin>) {
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
        for br in branches {
            self.newline();
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
        p.when(binding, subject, branches, origin);
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
/// 2-tuple that starts where its key does (a written one starts at `(`).
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

fn starts_with_minus(e: &Expr) -> bool {
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
