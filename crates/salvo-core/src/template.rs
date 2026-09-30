//! [host-splice] Platform templates: host code with Salvo in it.
//!
//! `platform/app/entry.sv.kt` (and `.sv.rs`) implements the bodiless
//! declarations of `app/entry.sv`. The file is ordinary Kotlin (Rust) in
//! which a `` `…` `` marker is Salvo the compiler renders, and inside a
//! marker `@name` / `@{…}` is host code again. ```` `` ```` is a literal
//! backtick. Strings and comments are host text and are not scanned.
//!
//! Three markers declare structure, each followed by a braced host body:
//!
//! * `` `fn name(params) -> R` { … } `` implements a bodiless Salvo fn — at
//!   file level a free fn, inside a handler one of its members;
//! * `` `platform handler H(params) of E` { … } `` implements a platform
//!   handler: its body is the class body (Kotlin) or the trait impl (Rust);
//! * `` `struct H` { name: T = init, … } `` adds host fields to handler `H`'s
//!   state, initialised in order after the Salvo state.
//!
//! Every other marker is a hole: a type, a value, `e : T`, a declaration
//! `name : T`, or an assignment `name = e`.
//!
//! [`apply`] attaches what it finds to the declarations of the module's AST
//! (`FnDecl.host`, `HandlerDecl.host`/`host_fields`/`spliced`,
//! `Module.host`), where the checker and both emitters already know how to
//! treat host bodies. Each template's text is appended to its module's
//! [`SourceFile`] as an [`Appendix`] so diagnostics name the template.

use std::path::PathBuf;

use salvo_syntax::ast::{self, FnDecl, HandlerDecl, HostBlock, HostBlockPart, HostField, Item, Module};
use salvo_syntax::Span;

use crate::source::{ModulePath, SourceFile};
use crate::FileDiagnostic;

/// One platform template, as loaded.
#[derive(Clone, Debug)]
pub struct TemplateFile {
    pub rel_path: PathBuf,
    /// The Salvo module it implements.
    pub module: ModulePath,
    /// `kotlin` or `rust`.
    pub lang: String,
    pub content: String,
    /// The name diagnostics show.
    pub name: String,
}

/// The backend a template's extension names.
pub fn template_lang(ext: &str) -> Option<&'static str> {
    match ext {
        "kt" => Some("kotlin"),
        "rs" => Some("rust"),
        _ => None,
    }
}

/// A template's text, carried by its module's source file so a span inside it
/// renders against the template.
#[derive(Clone, Debug)]
pub struct Appendix {
    pub base: u32,
    pub name: String,
    pub content: String,
}

// ================================================================ scanning ===

#[derive(Clone, Debug)]
enum Node {
    Text(String),
    Marker { source: String, offset: u32, in_return: bool },
    Decl { header: String, offset: u32, body: Vec<Node>, span: Span },
}

struct Scanner<'a> {
    chars: Vec<(usize, char)>,
    pos: usize,
    src: &'a str,
    lang: &'a str,
    base: u32,
    errors: Vec<(Span, String)>,
}

impl<'a> Scanner<'a> {
    fn peek(&self, n: usize) -> Option<char> {
        self.chars.get(self.pos + n).map(|&(_, c)| c)
    }
    fn offset(&self) -> u32 {
        self.base + self.chars.get(self.pos).map(|&(i, _)| i as u32).unwrap_or(self.src.len() as u32)
    }
    fn at_str(&self, s: &str) -> bool {
        s.chars().enumerate().all(|(i, c)| self.peek(i) == Some(c))
    }

    /// Copies a string, comment or character literal whole into [text].
    fn skip_host_literal(&mut self, text: &mut String) -> bool {
        let c = match self.peek(0) {
            Some(c) => c,
            None => return false,
        };
        let copy_until = |sc: &mut Self, text: &mut String, end: &str| {
            while sc.pos < sc.chars.len() && !sc.at_str(end) {
                if sc.peek(0) == Some('\\') && end.starts_with('"') {
                    text.push('\\');
                    sc.pos += 1;
                }
                if let Some(c) = sc.peek(0) {
                    text.push(c);
                    sc.pos += 1;
                }
            }
            for _ in 0..end.chars().count() {
                if let Some(c) = sc.peek(0) {
                    text.push(c);
                    sc.pos += 1;
                }
            }
        };
        if self.at_str("//") {
            while let Some(c) = self.peek(0) {
                if c == '\n' {
                    break;
                }
                text.push(c);
                self.pos += 1;
            }
            return true;
        }
        if self.at_str("/*") {
            let mut depth = 0usize;
            while self.pos < self.chars.len() {
                if self.at_str("/*") {
                    depth += 1;
                    text.push_str("/*");
                    self.pos += 2;
                } else if self.at_str("*/") {
                    depth -= 1;
                    text.push_str("*/");
                    self.pos += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    text.push(self.peek(0).unwrap());
                    self.pos += 1;
                }
            }
            return true;
        }
        if self.lang == "kotlin" && self.at_str("\"\"\"") {
            text.push_str("\"\"\"");
            self.pos += 3;
            copy_until(self, text, "\"\"\"");
            return true;
        }
        if self.lang == "rust" && (c == 'r' || c == 'b') && !self.prev_ident() {
            // r"…", r#"…"#, br"…", b"…"
            let mut i = 0;
            if self.peek(i) == Some('b') {
                i += 1;
            }
            let raw = self.peek(i) == Some('r');
            if raw {
                i += 1;
            }
            let mut hashes = 0;
            while raw && self.peek(i + hashes) == Some('#') {
                hashes += 1;
            }
            if (raw || i > 0) && self.peek(i + hashes) == Some('"') {
                for _ in 0..i + hashes + 1 {
                    text.push(self.peek(0).unwrap());
                    self.pos += 1;
                }
                let end = format!("\"{}", "#".repeat(hashes));
                if raw {
                    while self.pos < self.chars.len() && !self.at_str(&end) {
                        text.push(self.peek(0).unwrap());
                        self.pos += 1;
                    }
                    for _ in 0..end.len() {
                        if let Some(c) = self.peek(0) {
                            text.push(c);
                            self.pos += 1;
                        }
                    }
                } else {
                    copy_until(self, text, "\"");
                }
                return true;
            }
        }
        if c == '"' {
            text.push('"');
            self.pos += 1;
            copy_until(self, text, "\"");
            return true;
        }
        if c == '\'' {
            // A character literal — or, in Rust, a lifetime (`'a`), which is
            // copied as ordinary text.
            let is_char = self.peek(1) == Some('\\') || self.peek(2) == Some('\'');
            if is_char {
                text.push('\'');
                self.pos += 1;
                while let Some(c) = self.peek(0) {
                    text.push(c);
                    self.pos += 1;
                    if c == '\\' {
                        if let Some(n) = self.peek(0) {
                            text.push(n);
                            self.pos += 1;
                        }
                        continue;
                    }
                    if c == '\'' {
                        break;
                    }
                }
                return true;
            }
        }
        false
    }

    fn prev_ident(&self) -> bool {
        self.pos > 0 && {
            let c = self.chars[self.pos - 1].1;
            c.is_alphanumeric() || c == '_'
        }
    }

    /// Nodes up to the `}` closing the current body (or the end of file at
    /// the top). [depth] counts host braces opened inside this body.
    fn nodes(&mut self, top: bool) -> Vec<Node> {
        let mut out = Vec::new();
        let mut text = String::new();
        let mut depth = 0usize;
        let flush = |text: &mut String, out: &mut Vec<Node>| {
            if !text.is_empty() {
                out.push(Node::Text(std::mem::take(text)));
            }
        };
        while let Some(c) = self.peek(0) {
            if self.skip_host_literal(&mut text) {
                continue;
            }
            match c {
                '`' if self.peek(1) == Some('`') => {
                    text.push('`');
                    self.pos += 2;
                }
                '`' => {
                    let start = self.offset();
                    self.pos += 1;
                    let offset = self.offset();
                    let mut source = String::new();
                    loop {
                        match self.peek(0) {
                            None => {
                                self.errors.push((Span::new(start, self.offset()), "unterminated `…` marker".into()));
                                break;
                            }
                            Some('`') => {
                                self.pos += 1;
                                break;
                            }
                            Some(c) => {
                                source.push(c);
                                self.pos += 1;
                            }
                        }
                    }
                    let head = source.trim_start();
                    let is_decl = ["fn ", "platform handler ", "threadsafe platform handler ", "struct "]
                        .iter()
                        .any(|k| head.starts_with(k));
                    if is_decl {
                        // Its body: the next `{`, past whitespace.
                        let mut ws = String::new();
                        while let Some(c) = self.peek(0) {
                            if c.is_whitespace() {
                                ws.push(c);
                                self.pos += 1;
                            } else {
                                break;
                            }
                        }
                        if self.peek(0) != Some('{') {
                            self.errors.push((
                                Span::new(start, self.offset()),
                                format!("`{}` declares something, so a `{{` body must follow it", head.trim()),
                            ));
                            text.push_str(&ws);
                            continue;
                        }
                        self.pos += 1;
                        flush(&mut text, &mut out);
                        let body = self.nodes(false);
                        let end = self.offset();
                        out.push(Node::Decl { header: source, offset, body, span: Span::new(start, end) });
                    } else {
                        // The whole of a `return`'s expression: host `return`
                        // before it, the statement's end after it.
                        let in_return = {
                            let t = text.trim_end();
                            let after = (0..).map(|i| self.peek(i)).find(|c| !matches!(c, Some(' ') | Some('\t')));
                            t.ends_with("return")
                                && !t[..t.len() - 6].ends_with(|c: char| c.is_alphanumeric() || c == '_')
                                && matches!(after, Some(None) | Some(Some('\n')) | Some(Some(';')) | Some(Some(',')) | Some(Some('}')) | Some(Some('\r')))
                        };
                        flush(&mut text, &mut out);
                        out.push(Node::Marker { source, offset, in_return });
                    }
                }
                '{' => {
                    depth += 1;
                    text.push('{');
                    self.pos += 1;
                }
                '}' => {
                    self.pos += 1;
                    if depth == 0 && !top {
                        flush(&mut text, &mut out);
                        return out;
                    }
                    depth = depth.saturating_sub(1);
                    text.push('}');
                }
                c => {
                    text.push(c);
                    self.pos += 1;
                }
            }
        }
        if !top {
            self.errors.push((Span::new(self.offset(), self.offset()), "unclosed `{` after a declaring marker".into()));
        }
        flush(&mut text, &mut out);
        out
    }
}

// ================================================================== apply ===

/// [host-splice] [cli-platform] The skeleton of `module`'s platform template
/// in `lang`, as `salvo platform generate` writes it: a declaring marker, with
/// its full signature and a `TODO`/`todo!` body, for every platform handler
/// (each member of the effect it implements) and every bodiless fn that has
/// no `lang` implementation yet. `modules` is the whole program, where the
/// handled effect is looked up (std's, often). `None` when nothing is
/// missing.
pub fn skeleton(module: &Module, modules: &[Module], lang: &str) -> Option<String> {
    let todo = |what: &str| match lang {
        "kotlin" => format!("TODO(\"implement {what}\")"),
        _ => format!("todo!(\"implement {what}\")"),
    };
    let header = |f: &FnDecl| {
        let ret = f.return_type.as_ref().map(|t| format!(" -> {t}")).unwrap_or_default();
        format!("fn {}{}{ret}", f.name.name, params_text(&f.params))
    };
    let has = |blocks: &[HostBlock]| blocks.iter().any(|b| b.lang == lang);
    let mut parts: Vec<String> = Vec::new();
    for item in &module.items {
        match item {
            Item::Fn(f) if f.body.is_none() && f.by.is_none() && !f.intrinsic && !has(&f.host) => {
                parts.push(format!("`{}` {{\n    {}\n}}\n", header(f), todo(&f.name.name)));
            }
            Item::Handler(h) if h.platform => {
                let written = h.fns.iter().any(|f| has(&f.host)) || has(&h.host) || h.host_fields.iter().any(|f| f.lang == lang);
                if written {
                    continue;
                }
                let effect = h.of.first().and_then(|t| match t {
                    ast::Type::Named { base, .. } => Some(base.name.name.as_str()),
                    _ => None,
                });
                // The handler's own module first: an effect declared there
                // wins over a same-named one elsewhere (std has a `Clock`).
                let members: Vec<&FnDecl> = std::iter::once(module)
                    .chain(modules.iter())
                    .flat_map(|m| &m.items)
                    .find_map(|i| match i {
                        Item::Effect(e) if Some(e.name.name.as_str()) == effect => Some(e.fns.iter().collect()),
                        _ => None,
                    })
                    .unwrap_or_default();
                let of = h.of.iter().map(|t| t.to_string()).collect::<Vec<_>>().join(", ");
                let mut out = format!(
                    "`{}platform handler {}{} of {of}` {{\n",
                    if h.threadsafe { "threadsafe " } else { "" },
                    h.name.name,
                    if h.params.is_empty() { String::new() } else { params_text(&h.params) },
                );
                for (i, f) in members.iter().enumerate() {
                    if i > 0 {
                        out.push('\n');
                    }
                    out.push_str(&crate::platform::reply_contract_comment(f));
                    out.push_str(&format!(
                        "    `{}` {{\n        {}\n    }}\n",
                        header(f),
                        todo(&format!("{}.{}", effect.unwrap_or_default(), f.name.name))
                    ));
                }
                out.push_str("}\n");
                parts.push(out);
            }
            _ => {}
        }
    }
    if parts.is_empty() {
        return None;
    }
    let host = if lang == "kotlin" { "Kotlin" } else { "Rust" };
    Some(format!(
        "// The {host} implementation of this module's bodiless declarations: a platform\n\
         // template [host-splice]. Anything between backticks is Salvo, rendered by the\n\
         // compiler; the rest is {host}. Written once by `salvo platform generate`,\n\
         // never overwritten.\n\n{}",
        parts.join("\n")
    ))
}

/// [host-splice] Where a template's markers are: the Salvo text of each,
/// between its backticks, as offsets into [content] — a declaring marker's
/// header included, and the markers inside its body. Everything else is host
/// text. The language server answers inside these spans and nowhere else.
pub fn marker_spans(content: &str, lang: &str) -> Vec<Span> {
    fn walk(nodes: &[Node], out: &mut Vec<Span>) {
        for n in nodes {
            match n {
                Node::Text(_) => {}
                Node::Marker { source, offset, .. } => out.push(Span::new(*offset, offset + source.len() as u32)),
                Node::Decl { header, offset, body, .. } => {
                    out.push(Span::new(*offset, offset + header.len() as u32));
                    walk(body, out);
                }
            }
        }
    }
    let mut scanner = Scanner {
        chars: content.char_indices().collect(),
        pos: 0,
        src: content,
        lang,
        base: 0,
        errors: Vec::new(),
    };
    let nodes = scanner.nodes(true);
    let mut out = Vec::new();
    walk(&nodes, &mut out);
    out
}

/// [host-splice] A declaring marker's header, parsed as the Salvo it names,
/// with spans as offsets into the template. It repeats the signature of the
/// `.sv` declaration it implements, which is how the language server answers
/// in it: a position in the header is the matching position of the
/// declaration.
#[derive(Clone, Debug)]
pub enum Header {
    /// `` `fn name(params) -> R` ``: a free fn, or — with `handler` the
    /// enclosing `` `platform handler H …` ``'s name — a member of `H`'s
    /// effect.
    Fn { decl: FnDecl, handler: Option<String> },
    /// `` `platform handler H(params) of E` ``.
    Handler(HandlerDecl),
    /// `` `struct H` ``: handler `H`'s host fields.
    Struct(ast::Ident),
}

/// [host-splice] Every declaring marker's header in a template, parsed; one
/// that does not parse is left out (the analysis reports it).
pub fn headers(content: &str, lang: &str) -> Vec<Header> {
    fn walk(nodes: &[Node], handler: Option<&str>, out: &mut Vec<Header>) {
        for n in nodes {
            let Node::Decl { header, offset, body, .. } = n else { continue };
            let head = header.trim_start();
            let at = offset + (header.len() - head.len()) as u32;
            if let Some(name) = head.strip_prefix("struct ") {
                let lead = (name.len() - name.trim_start().len()) as u32;
                let name = name.trim();
                let start = at + "struct ".len() as u32 + lead;
                out.push(Header::Struct(ast::Ident {
                    name: name.to_string(),
                    span: Span::new(start, start + name.len() as u32),
                }));
                continue;
            }
            match parse_header(head, at).and_then(|m| m.items.into_iter().next()) {
                Some(Item::Handler(h)) => {
                    let name = h.name.name.clone();
                    out.push(Header::Handler(h));
                    walk(body, Some(&name), out);
                }
                Some(Item::Fn(decl)) => out.push(Header::Fn { decl, handler: handler.map(str::to_string) }),
                _ => {}
            }
        }
    }
    let mut scanner = Scanner {
        chars: content.char_indices().collect(),
        pos: 0,
        src: content,
        lang,
        base: 0,
        errors: Vec::new(),
    };
    let nodes = scanner.nodes(true);
    let mut out = Vec::new();
    walk(&nodes, None, &mut out);
    out
}

fn marker_part(source: &str, offset: u32, in_return: bool, diags: &mut Vec<(Span, String)>) -> HostBlockPart {
    let (mut hole, errs) = salvo_syntax::parser::parse_template_marker(source, offset);
    hole.in_return = in_return;
    for e in errs {
        if e.is_error() {
            diags.push((e.span, e.message));
        }
    }
    HostBlockPart::Hole(hole)
}

/// Flattens a body into host parts; a nested declaration is an error here.
fn parts(nodes: &[Node], diags: &mut Vec<(Span, String)>) -> Vec<HostBlockPart> {
    let mut out = Vec::new();
    for n in nodes {
        match n {
            Node::Text(t) => out.push(HostBlockPart::Text(t.clone())),
            Node::Marker { source, offset, in_return } => out.push(marker_part(source, *offset, *in_return, diags)),
            Node::Decl { header, span, .. } => {
                diags.push((*span, format!("`{}` cannot be declared here", header.trim())));
            }
        }
    }
    out
}

/// Parses a declaring marker's header as the Salvo it names, its spans at
/// [offset] (the source is padded rather than re-spanned).
fn parse_header(src: &str, offset: u32) -> Option<ast::Module> {
    let padded = format!("{}{src}", " ".repeat(offset as usize));
    let (module, diags) = salvo_syntax::parse_module(&padded);
    if diags.iter().any(|d| d.is_error()) {
        return None;
    }
    Some(module)
}

fn sig(params: &[ast::Param], ret: Option<&ast::Type>) -> String {
    let ps: Vec<String> = params.iter().map(|p| format!("{}: {}", p.name.name, p.ty)).collect();
    format!("({}) -> {}", ps.join(", "), ret.map(|t| t.to_string()).unwrap_or_else(|| "None".to_string()))
}

/// The effect member a handler member implements, by name and parameters —
/// looked up in the handler's own module (`own`) first, so an effect declared
/// there wins over a same-named one elsewhere (std declares a `Clock`).
fn effect_member<'m>(modules: &'m [Module], own: usize, effect: &str, name: &str, params: &[ast::Param]) -> Option<&'m FnDecl> {
    for m in modules.get(own).into_iter().chain(modules.iter()) {
        for item in &m.items {
            if let Item::Effect(e) = item {
                if e.name.name == effect {
                    let mut cands = e.fns.iter().filter(|f| f.name.name == name);
                    let all: Vec<&FnDecl> = cands.by_ref().collect();
                    if all.len() == 1 {
                        return Some(all[0]);
                    }
                    return all.into_iter().find(|f| sig(&f.params, None) == sig(params, None));
                }
            }
        }
    }
    None
}

/// Attaches every template to its module, appending its text to the module's
/// source file for diagnostics. Returns the errors found.
pub fn apply(templates: &[TemplateFile], files: &mut [SourceFile], modules: &mut [Module]) -> Vec<FileDiagnostic> {
    let mut out = Vec::new();
    for t in templates {
        let Some(fi) = files.iter().position(|f| f.module == t.module) else {
            // Nothing to attach to: the module's diagnostics cannot carry it.
            out.push(FileDiagnostic::error(
                0,
                Span::new(0, 0),
                format!(
                    "`{}` implements module `{}`, which has no `.sv` file [host-splice]",
                    t.name, t.module
                ),
            ));
            continue;
        };
        let base = files[fi].content.len() as u32
            + 1
            + files[fi].appendix.iter().map(|a| a.content.len() as u32 + 1).sum::<u32>();
        files[fi].appendix.push(Appendix { base, name: t.name.clone(), content: t.content.clone() });
        let mut scanner = Scanner {
            chars: t.content.char_indices().collect(),
            pos: 0,
            src: &t.content,
            lang: &t.lang,
            base,
            errors: Vec::new(),
        };
        let nodes = scanner.nodes(true);
        let mut diags = scanner.errors;
        let block = |parts: Vec<HostBlockPart>, span: Span| HostBlock { lang: t.lang.clone(), parts, span };
        let file_span = Span::new(base, base + t.content.len() as u32);
        let mut file_parts = Vec::new();
        for node in &nodes {
            match node {
                Node::Text(_) | Node::Marker { .. } => file_parts.extend(parts(std::slice::from_ref(node), &mut diags)),
                Node::Decl { header, offset, body, span } => {
                    attach_decl(header, *offset, body, *span, &t.lang, fi, modules, &mut diags);
                }
            }
        }
        if file_parts.iter().any(|p| !matches!(p, HostBlockPart::Text(s) if s.trim().is_empty())) {
            modules[fi].host.push(block(file_parts, file_span));
        }
        for (span, msg) in diags {
            out.push(FileDiagnostic::error(fi, span, format!("{msg} [host-splice]")));
        }
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn attach_decl(
    header: &str,
    offset: u32,
    body: &[Node],
    span: Span,
    lang: &str,
    fi: usize,
    modules: &mut [Module],
    diags: &mut Vec<(Span, String)>,
) {
    let head = header.trim_start();
    let lead = (header.len() - head.len()) as u32;
    let host_block = |parts: Vec<HostBlockPart>| HostBlock { lang: lang.to_string(), parts, span };
    if let Some(name) = head.strip_prefix("struct ") {
        let name = name.trim();
        let fields = host_fields(body, lang, diags);
        let found = modules[fi].items.iter_mut().find_map(|i| match i {
            Item::Handler(h) if h.name.name == name && h.platform => Some(h),
            _ => None,
        });
        match found {
            Some(h) => h.host_fields.extend(fields),
            None => diags.push((span, format!("`struct {name}` names no platform handler of this module"))),
        }
        return;
    }
    // A handler: `platform handler H(p) of E` — parse it as the declaration.
    if head.contains("platform handler ") {
        let Some(parsed) = parse_header(head, offset + lead) else {
            diags.push((span, format!("`{}` is not a platform handler declaration", head.trim())));
            return;
        };
        let Some(Item::Handler(written)) = parsed.items.into_iter().next() else { return };
        let snapshot: Vec<Module> = modules.to_vec();
        let Some(h) = modules[fi].items.iter_mut().find_map(|i| match i {
            Item::Handler(h) if h.name.name == written.name.name => Some(h),
            _ => None,
        }) else {
            diags.push((span, format!("the template implements `platform handler {}`, which its `.sv` file does not declare", written.name.name)));
            return;
        };
        if !h.platform {
            diags.push((span, format!("`{}` is not a platform handler: only a platform handler has a template", h.name.name)));
            return;
        }
        let of = |x: &HandlerDecl| x.of.iter().map(|t| t.to_string()).collect::<Vec<_>>();
        if sig(&written.params, None) != sig(&h.params, None) || of(&written) != of(h) {
            diags.push((
                span,
                format!(
                    "the template's `platform handler {}{} of {}` does not match the declaration's `{}{} of {}`",
                    written.name.name,
                    params_text(&written.params),
                    of(&written).join(", "),
                    h.name.name,
                    params_text(&h.params),
                    of(h).join(", ")
                ),
            ));
        }
        h.spliced = true;
        let effect = h.of.first().and_then(|t| match t {
            ast::Type::Named { base, .. } => Some(base.name.name.clone()),
            _ => None,
        });
        let mut level = Vec::new();
        for node in body {
            match node {
                Node::Decl { header, offset, body, span } if header.trim_start().starts_with("fn ") => {
                    let head = header.trim_start();
                    let lead = (header.len() - head.len()) as u32;
                    let Some(parsed) = parse_header(head, offset + lead) else {
                        diags.push((*span, format!("`{}` is not a fn signature", head.trim())));
                        continue;
                    };
                    let Some(Item::Fn(mut f)) = parsed.items.into_iter().next() else { continue };
                    let Some(member) = effect.as_deref().and_then(|e| effect_member(&snapshot, fi, e, &f.name.name, &f.params)) else {
                        diags.push((*span, format!(
                            "`fn {}` implements no member of `{}`",
                            f.name.name,
                            effect.clone().unwrap_or_default()
                        )));
                        continue;
                    };
                    if sig(&f.params, f.return_type.as_ref()) != sig(&member.params, member.return_type.as_ref()) {
                        diags.push((*span, format!(
                            "the template's `fn {}{}` does not match the member's `{}`",
                            f.name.name,
                            sig(&f.params, f.return_type.as_ref()),
                            sig(&member.params, member.return_type.as_ref())
                        )));
                    }
                    let block = HostBlock { lang: lang.to_string(), parts: parts(body, diags), span: *span };
                    match h.fns.iter_mut().find(|g| g.name.name == f.name.name && sig(&g.params, None) == sig(&f.params, None)) {
                        Some(existing) => existing.host.push(block),
                        None => {
                            // The member's contract is the effect's.
                            f.deductions = member.deductions.clone();
                            f.effects = None;
                            f.host = vec![block];
                            f.body = None;
                            h.fns.push(f);
                        }
                    }
                }
                other => level.extend(parts(std::slice::from_ref(other), diags)),
            }
        }
        if level.iter().any(|p| !matches!(p, HostBlockPart::Text(s) if s.trim().is_empty())) {
            h.host.push(host_block(level));
        }
        return;
    }
    // A free fn.
    let Some(parsed) = parse_header(head, offset + lead) else {
        diags.push((span, format!("`{}` is not a fn signature", head.trim())));
        return;
    };
    let Some(Item::Fn(written)) = parsed.items.into_iter().next() else { return };
    let target = modules[fi].items.iter_mut().find_map(|i| match i {
        Item::Fn(f) if f.name.name == written.name.name && f.body.is_none() && !f.intrinsic
            && (sig(&f.params, None) == sig(&written.params, None)) => Some(f),
        _ => None,
    });
    let Some(f) = target else {
        diags.push((span, format!(
            "the template implements `fn {}{}`, which its `.sv` file does not declare without a body",
            written.name.name,
            params_text(&written.params)
        )));
        return;
    };
    if sig(&f.params, f.return_type.as_ref()) != sig(&written.params, written.return_type.as_ref()) {
        diags.push((span, format!(
            "the template's `fn {}{}` does not match the declaration's `{}`",
            written.name.name,
            sig(&written.params, written.return_type.as_ref()),
            sig(&f.params, f.return_type.as_ref())
        )));
    }
    if f.host.iter().any(|b| b.lang == lang) {
        diags.push((span, format!("`fn {}` has two {lang} implementations", f.name.name)));
    }
    f.host.push(host_block(parts(body, diags)));
}

fn params_text(params: &[ast::Param]) -> String {
    let ps: Vec<String> = params.iter().map(|p| format!("{}: {}", p.name.name, p.ty)).collect();
    format!("({})", ps.join(", "))
}

/// `name: HostType = init` entries, one per line or comma, at the body's top
/// level; the type and the initialiser may hold markers.
fn host_fields(body: &[Node], lang: &str, diags: &mut Vec<(Span, String)>) -> Vec<HostField> {
    let all = parts(body, diags);
    // Split into entries at top-level `,` / newline in text parts.
    let mut entries: Vec<Vec<HostBlockPart>> = vec![Vec::new()];
    let mut depth = 0i32;
    for p in all {
        match p {
            HostBlockPart::Text(t) => {
                let mut cur = String::new();
                for c in t.chars() {
                    match c {
                        '(' | '[' | '{' | '<' => depth += 1,
                        ')' | ']' | '}' | '>' => depth -= 1,
                        _ => {}
                    }
                    if depth == 0 && (c == ',' || c == '\n') {
                        if !cur.is_empty() {
                            entries.last_mut().unwrap().push(HostBlockPart::Text(std::mem::take(&mut cur)));
                        }
                        entries.push(Vec::new());
                    } else {
                        cur.push(c);
                    }
                }
                if !cur.is_empty() {
                    entries.last_mut().unwrap().push(HostBlockPart::Text(cur));
                }
            }
            hole => entries.last_mut().unwrap().push(hole),
        }
    }
    let mut out = Vec::new();
    for entry in entries {
        if entry.iter().all(|p| matches!(p, HostBlockPart::Text(t) if t.trim().is_empty() || t.trim().starts_with("//"))) {
            continue;
        }
        // name `:` ty `=` init
        let Some(HostBlockPart::Text(first)) = entry.first().cloned() else {
            diags.push((Span::new(0, 0), "a host field starts with its name".into()));
            continue;
        };
        let Some((name, rest)) = first.split_once(':') else {
            diags.push((Span::new(0, 0), format!("a host field is `name: Type = init`, found `{}`", first.trim())));
            continue;
        };
        let mut ty = vec![HostBlockPart::Text(rest.to_string())];
        let mut init = Vec::new();
        let mut in_init = false;
        for p in entry.into_iter().skip(1) {
            if in_init {
                init.push(p);
                continue;
            }
            match p {
                HostBlockPart::Text(t) if t.contains('=') => {
                    let (a, b) = t.split_once('=').unwrap();
                    ty.push(HostBlockPart::Text(a.to_string()));
                    init.push(HostBlockPart::Text(b.to_string()));
                    in_init = true;
                }
                other => ty.push(other),
            }
        }
        if !in_init {
            // `=` may sit in the first text part.
            if let Some(HostBlockPart::Text(t)) = ty.first().cloned() {
                if let Some((a, b)) = t.split_once('=') {
                    ty[0] = HostBlockPart::Text(a.to_string());
                    let mut rest = vec![HostBlockPart::Text(b.to_string())];
                    rest.extend(ty.drain(1..));
                    init = rest;
                    in_init = true;
                }
            }
        }
        if !in_init {
            diags.push((Span::new(0, 0), format!("host field `{}` needs an initialiser: `= …`", name.trim())));
            continue;
        }
        out.push(HostField { lang: lang.to_string(), name: name.trim().to_string(), ty, init });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // [host-splice] The spans the language server answers in are the
    // scanner's markers: not a string, a comment or a doubled backtick; a
    // declaring marker's header and the markers of its body.
    #[test]
    fn marker_spans_are_the_scanners_markers() {
        let src = "// `not` a marker\nval s = \"`nor` this\"\nval x = ``in``\n`fn f(a: Int) -> Int` {\n    return `a`\n}\n";
        let texts: Vec<&str> = marker_spans(src, "kotlin")
            .into_iter()
            .map(|s| &src[s.start as usize..s.end as usize])
            .collect();
        assert_eq!(texts, ["fn f(a: Int) -> Int", "a"]);
    }

    // [host-splice] Headers parse with their template offsets, a handler's
    // members knowing their handler.
    #[test]
    fn headers_carry_template_offsets() {
        let src = "`struct H` {\n}\n`platform handler H(n: Int) of E` {\n    `fn m(x: Int) -> Int` { `x` }\n}\n";
        let hs = headers(src, "rust");
        assert_eq!(hs.len(), 3, "{hs:?}");
        let text = |s: Span| &src[s.start as usize..s.end as usize];
        match &hs[0] {
            Header::Struct(name) => assert_eq!(text(name.span), "H"),
            other => panic!("{other:?}"),
        }
        match &hs[1] {
            Header::Handler(h) => assert_eq!(text(h.params[0].ty.span()), "Int"),
            other => panic!("{other:?}"),
        }
        match &hs[2] {
            Header::Fn { decl, handler } => {
                assert_eq!(handler.as_deref(), Some("H"));
                assert_eq!(text(decl.name.span), "m");
            }
            other => panic!("{other:?}"),
        }
    }
}
