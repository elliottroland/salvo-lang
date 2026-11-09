//! [ast-print] A program's file printed as the source of its expanded tree
//! (`salvo_syntax::print`), with what only the program knows: a leading
//! comment on each declaration an expansion wrote, saying what produced it
//! and from where, and (for a printing that must parse back) a field
//! default an expansion placed, written out.
//!
//! The resolve dump, the type-check dump and the round trip all print
//! through here, so they show the same text for the same tree.

use salvo_syntax::ast::{Expr, ExpansionKind, FnDecl, Ident, Item, StructDecl, Type};
use salvo_syntax::print::{self, Notes, Printed};

use crate::Program;

/// [ast-print] File `file` of `program`, its tree printed: the dumps' form,
/// with what has no spelling marked `#{…}`.
pub fn print_file(program: &Program, file: usize) -> Printed {
    print::print_module_with(&program.modules[file], &ProgramNotes { program, file, inline_defaults: false })
}

/// [round-trip] File `file` of `program` printed so it parses back: a field
/// default a comptime copy reads is written as its expression.
pub fn print_file_as_source(program: &Program, file: usize) -> Printed {
    print::print_module_with(&program.modules[file], &ProgramNotes { program, file, inline_defaults: true })
}

struct ProgramNotes<'a> {
    program: &'a Program,
    file: usize,
    inline_defaults: bool,
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
        if let Some(state) = &f.iter_state_name {
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
        if f.name.name.starts_with(HIDDEN_COMPTIME_PREFIX) {
            return vec![format!(
                "hidden fn for a comptime call `[{}(…)]` at `{}`: stamped from comptime fn `{}` of `{}`{template}, for the stamp at {site}",
                stamp.compfn, stamp.at_type, stamp.compfn, stamp.from
            )];
        }
        vec![format!("stamped from comptime fn `{}` of `{}`{template} at `{}`, for the `by` at {site}", stamp.compfn, stamp.from, stamp.at_type)]
    }

    fn struct_note(&self, s: &StructDecl) -> Vec<String> {
        let _ = s;
        Vec::new()
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
