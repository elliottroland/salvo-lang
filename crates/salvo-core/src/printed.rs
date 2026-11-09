//! [ast-print] A program's file printed as the source of its expanded tree
//! (`salvo_syntax::print`), with what only the program knows: a leading
//! comment on each declaration an expansion wrote, saying what produced it
//! and from where, and (for a printing that must parse back) a field
//! default an expansion placed, written out.
//!
//! The resolve dump, the type-check dump and the round trip all print
//! through here, so they show the same text for the same tree.

use salvo_syntax::ast::{Expr, ExpansionKind, FnDecl, Ident, Item, StructDecl, Type};
use salvo_syntax::print::{self, FnSource, Notes, Printed};

use crate::Program;

/// [ast-print] File `file` of `program`, its tree printed: the dumps' form,
/// with what has no spelling marked `#{…}`.
pub fn print_file(program: &Program, file: usize) -> Printed {
    print::print_module_with(&program.modules[file], &ProgramNotes { program, file, inline_defaults: false, restamp: false, verbatim: Default::default() })
}

/// [round-trip] File `file` of `program` printed so it parses back and
/// expands to itself: a stamped fn as the declaration that asked for it,
/// written as it was (what an obligation's `by` or a comptime call stamped
/// left out, the expansion making it again), and a field default a comptime
/// copy reads as its expression.
pub fn print_file_as_source(program: &Program, file: usize) -> Printed {
    print::print_module_with(&program.modules[file], &ProgramNotes { program, file, inline_defaults: true, restamp: true, verbatim: Default::default() })
}

/// [ast-print] File `file` of `program`, its expanded tree printed as source
/// that parses (a field default a comptime copy reads written as its
/// expression), stamped code included: what a test of the printer reads
/// back.
pub fn print_file_expanded_source(program: &Program, file: usize) -> Printed {
    print::print_module_with(&program.modules[file], &ProgramNotes { program, file, inline_defaults: true, restamp: false, verbatim: Default::default() })
}

struct ProgramNotes<'a> {
    program: &'a Program,
    file: usize,
    /// Printing for a parse: field defaults written out.
    inline_defaults: bool,
    /// Printing for an expansion: stamped code as the declaration that
    /// asked for it.
    restamp: bool,
    /// The spans of the declarations written as their text, each once.
    verbatim: std::cell::RefCell<std::collections::HashSet<(u32, u32)>>,
}

impl ProgramNotes<'_> {
    /// `name:line` of `offset` in file `file`.
    fn place(&self, file: usize, offset: u32) -> String {
        let f = &self.program.files[file];
        let (line, _) = salvo_syntax::span::line_col(&f.content, offset);
        format!("{}:{line}", f.name)
    }
}

/// [comptime-hidden] The prefix of the fn a comptime call `[f(x)]` stamps.
pub const HIDDEN_COMPTIME_PREFIX: &str = "__salvo_comptime_";

impl Notes for ProgramNotes<'_> {
    fn fn_note(&self, f: &FnDecl) -> Vec<String> {
        if f.name.name.starts_with(salvo_syntax::desugar::TEST_FN_PREFIX) {
            return vec!["the fn a `test` block is expanded into".to_string()];
        }
        // The `iter fn` expansion documents what it writes; one it left
        // undocumented is said here.
        if let (Some(state), true) = (&f.iter_state_name, f.docs.is_empty()) {
            return vec![format!("written by the expansion of an `iter fn` (its state struct is `{}`)", state.name)];
        }
        let Some(stamp) = &f.stamped else { return Vec::new() };
        // [comptime-inline] An expanded-in-place comptime fn is its own
        // template.
        let Some(tfile) = stamp.template_file else {
            return vec!["expanded in place: a comptime fn, its comptime constructs unrolled".to_string()];
        };
        // Where the template's text is: the body's copied span, in the
        // template's file.
        let template = f
            .body
            .as_ref()
            .filter(|b| b.span.is_expanded() && stamp.expansion(b.span.expn).is_some_and(|e| matches!(e.kind, ExpansionKind::Template)))
            .map(|b| format!(" ({})", self.place(tfile, b.span.start)))
            .unwrap_or_default();
        let site = self.place(self.file, stamp.site.start);
        // `by auto` names a scope, `by to_json` the comptime fn itself.
        let from = if stamp.from == stamp.compfn { String::new() } else { format!(" from `{}`", stamp.from) };
        if f.name.name.starts_with(HIDDEN_COMPTIME_PREFIX) {
            return vec![format!(
                "hidden fn for a comptime call `[{}(…)]` at `{}`, made stamping the `by` at {site}: stamped from comptime fn `{}`{from}{template}",
                stamp.compfn, stamp.at_type, stamp.compfn
            )];
        }
        vec![format!("stamped from comptime fn `{}`{from}{template} at `{}`, for the `by` at {site}", stamp.compfn, stamp.at_type)]
    }

    fn struct_note(&self, s: &StructDecl) -> Vec<String> {
        let _ = s;
        Vec::new()
    }

    fn fn_source(&self, f: &FnDecl) -> FnSource {
        // [comptime-hygiene] [comptime-scope] A stamped body is not source of
        // the file it is in: its names resolve where its template is written,
        // and its unrolled copies are scopes of their own spliced flat. A
        // printing that is expanded again writes the declaration that asked
        // for the stamp, as it was written, and leaves out what an
        // obligation's `by` or a comptime call stamped: the expansion makes
        // them again.
        if !self.restamp || f.stamped.is_none() {
            return FnSource::Tree;
        }
        let source = &self.program.files[self.file].content;
        let Some(stamp) = &f.stamped else { return FnSource::Tree };
        // A `fn … by x` declaration starts before its `by`; what an
        // obligation's `by` stamped is made at the `by` itself.
        let declared = stamp.template_file.is_none() || f.span.start < stamp.site.start;
        if f.name.name.starts_with(HIDDEN_COMPTIME_PREFIX) || !f.span.is_written() || !declared {
            return FnSource::Omit;
        }
        let Some(text) = source.get(f.span.start as usize..f.span.end as usize) else { return FnSource::Omit };
        if !(text.starts_with("fn") || text.starts_with("comptime") || text.starts_with("send")) {
            return FnSource::Omit;
        }
        if !self.verbatim.borrow_mut().insert((f.span.start, f.span.end)) {
            return FnSource::Omit;
        }
        // Its lines after the first, relative to the column it starts at.
        let (_, col) = salvo_syntax::span::line_col(source, f.span.start);
        let indent = " ".repeat(col as usize - 1);
        let lines: Vec<&str> = text.lines().enumerate().map(|(i, l)| if i == 0 { l } else { l.strip_prefix(indent.as_str()).unwrap_or(l) }).collect();
        FnSource::Text(lines.join("\n"))
    }

    fn field_default(&self, owner: &Type, field: &Ident) -> Option<Expr> {
        if !self.inline_defaults {
            return None;
        }
        field_default(self.program, owner, field)
    }
}

/// [comptime-default] The default expression of field `field` of the struct
/// `owner` names, when one struct of the program has that name.
pub fn field_default(program: &Program, owner: &Type, field: &Ident) -> Option<Expr> {
    let Type::Named { base, .. } = owner else { return None };
    let mut found = program.modules.iter().flat_map(|m| m.items.iter()).filter_map(|item| match item {
        Item::Struct(s) if s.name.name == base.name.name => s.fields.iter().find(|f| f.name.name == field.name).and_then(|f| f.default.clone()),
        _ => None,
    });
    let first = found.next()?;
    found.next().is_none().then_some(first)
}
