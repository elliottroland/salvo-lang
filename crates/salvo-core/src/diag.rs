//! Structured diagnostics attributed to files of a [`Program`]
//! [diag-structured].
//!
//! The resolver, checker, and deduction pass report errors as
//! [`FileDiagnostic`]s — `(file, span, severity, message)` — instead of
//! pre-rendered strings, so consumers (the CLI `analyze` command, a future
//! language server) can map them to source locations. Human-readable
//! rendering happens at the boundary via [`FileDiagnostic::render`].
//!
//! [`Program`]: crate::Program

use salvo_syntax::diag::{Diagnostic, IntoMessage, Severity};
use salvo_syntax::Span;

use crate::source::SourceFile;

/// A diagnostic attributed to a source file by index into
/// `Program::files` / `SourceSet::files` [diag-structured].
#[derive(Clone, Debug)]
pub struct FileDiagnostic {
    /// Index into `Program::files`.
    pub file: usize,
    pub severity: Severity,
    pub message: String,
    pub span: Span,
    /// Import paths (`module.Item`) that would bring the unresolved name
    /// into scope [diag-import-suggest]. Rendered as `help:` lines and
    /// offered as LSP quickfixes; empty for most diagnostics.
    pub suggested_imports: Vec<String>,
    /// [node-origin] The declaration the diagnostic was raised in, when it was
    /// raised at a node an expansion copied: its span is then where the
    /// origin rendered it (a field, the `by` site), which is another
    /// declaration's text, so this is what attributes it (`ir::gate`).
    pub decl: Option<crate::DeclId>,
    /// [diag-id] What the diagnostic is, `label:name`; `None` only for one
    /// not yet given an id (TESTS.md T2).
    pub id: Option<&'static str>,
}

impl FileDiagnostic {
    pub fn error(file: usize, span: Span, message: impl IntoMessage) -> Self {
        let (id, message) = message.into_message();
        FileDiagnostic {
            file,
            severity: Severity::Error,
            message: shown(message),
            span,
            suggested_imports: Vec::new(),
            decl: None,
            id,
        }
    }

    /// A diagnostic that reports something the author probably did not
    /// intend without rejecting the program — used where a rule
    /// deliberately degrades instead of failing (a suppressed refinement
    /// conflict [qual-refn]).
    pub fn warning(file: usize, span: Span, message: impl IntoMessage) -> Self {
        let (id, message) = message.into_message();
        FileDiagnostic {
            file,
            severity: Severity::Warning,
            message: shown(message),
            span,
            suggested_imports: Vec::new(),
            decl: None,
            id,
        }
    }

    /// A parse (or per-file expansion) diagnostic, placed in file [file].
    pub fn from_parse(file: usize, d: Diagnostic) -> Self {
        FileDiagnostic {
            file,
            severity: d.severity,
            message: d.message,
            span: d.span,
            suggested_imports: Vec::new(),
            decl: None,
            id: d.id,
        }
    }

    /// [node-origin] Attributes the diagnostic to the declaration it was raised
    /// in (`FileDiagnostic::decl`).
    pub fn in_decl(mut self, decl: Option<crate::DeclId>) -> Self {
        self.decl = decl;
        self
    }

    /// Attaches import suggestions [diag-import-suggest].
    pub fn with_imports(mut self, imports: Vec<String>) -> Self {
        self.suggested_imports = imports;
        self
    }

    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }

    /// Renders against the file list this diagnostic was produced from,
    /// e.g. `error: unknown effect --> main.sv:3:7` with a caret line and
    /// a `help:` line per suggested import [diag-import-suggest].
    pub fn render(&self, files: &[SourceFile]) -> String {
        let f = &files[self.file];
        let mut out = Diagnostic {
            severity: self.severity,
            message: self.message.clone(),
            span: self.span,
            id: self.id,
        }
        .render(&f.name, &f.content);
        for import in &self.suggested_imports {
            out.push_str(&format!("\n  help: add `import {import}`"));
        }
        out
    }
}

/// [diag-std-hidden] Whether a diagnostic located in `file` is hidden: the
/// file is the embedded standard library. A `std = true` tree's files are
/// `is_shadow` (`mark_std_tree`), so the standard library being developed
/// still shows its own.
pub fn hidden_std_file(file: &SourceFile) -> bool {
    file.is_std && !file.is_shadow
}

/// [diag-std-hidden] The one line shown in place of hidden errors, so a
/// program the embedded std does not check with never fails silently. It
/// names no std position: an error there is the compiler's, not the
/// program's.
pub fn std_errors_line(errors: usize) -> String {
    format!(
        "error: the standard library does not check with this program ({errors} error{} in std; \
         this is a compiler bug) [diag-std-hidden]",
        if errors == 1 { "" } else { "s" }
    )
}

/// [diag-std-hidden] Drops from `diags` every diagnostic located in the
/// embedded std, and answers how many of the dropped were errors.
pub fn hide_std_diagnostics(diags: &mut Vec<FileDiagnostic>, files: &[SourceFile]) -> usize {
    let mut errors = 0;
    diags.retain(|d| {
        let hidden = hidden_std_file(&files[d.file]);
        if hidden && d.is_error() {
            errors += 1;
        }
        !hidden
    });
    errors
}

/// [diag-std-hidden] What is shown of `diags`, rendered: every diagnostic not
/// located in the embedded std, then, when errors were hidden, the line
/// saying so ([`std_errors_line`]).
pub fn render_shown<'d>(diags: impl IntoIterator<Item = &'d FileDiagnostic>, files: &[SourceFile]) -> Vec<String> {
    let mut out = Vec::new();
    let mut hidden = 0;
    for d in diags {
        if !hidden_std_file(&files[d.file]) {
            out.push(d.render(files));
        } else if d.is_error() {
            hidden += 1;
        }
    }
    if hidden > 0 {
        out.push(std_errors_line(hidden));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::ModulePath;

    fn file(name: &str, is_std: bool, is_shadow: bool) -> SourceFile {
        SourceFile {
            name: name.to_string(),
            module: ModulePath(vec!["m".to_string()]),
            content: "fn f() [] {\n}\n".to_string(),
            is_std,
            is_shadow,
            is_test: false,
            dependency: None,
        }
    }

    // [diag-std-hidden] The embedded std's diagnostics are dropped and its
    // errors counted; a `std = true` tree's (`is_shadow`) and the project's
    // stay, and the rendering ends with the one line naming no std position.
    #[test]
    fn std_diagnostics_are_hidden_unless_the_tree_is_std() {
        let files = vec![
            file("std/core/list.sv", true, false),
            file("core/list.sv", true, true),
            file("main.sv", false, false),
        ];
        let span = Span::new(0, 2);
        let diags = vec![
            FileDiagnostic::error(0, span, "in the embedded std"),
            FileDiagnostic::warning(0, span, "a warning in the embedded std"),
            FileDiagnostic::error(0, span, "another in the embedded std"),
            FileDiagnostic::error(1, span, "in a std tree"),
            FileDiagnostic::warning(2, span, "in the project"),
        ];
        let rendered = render_shown(&diags, &files);
        assert_eq!(rendered.len(), 3, "{rendered:#?}");
        assert!(rendered[0].contains("in a std tree") && rendered[0].contains("core/list.sv:1:1"), "{rendered:#?}");
        assert!(rendered[1].contains("in the project"), "{rendered:#?}");
        assert_eq!(rendered[2], std_errors_line(2));
        assert!(!rendered[2].contains("std/core"), "the line names no std position");
        assert!(rendered.iter().all(|r| !r.contains("embedded std")), "{rendered:#?}");

        let mut kept = diags.clone();
        assert_eq!(hide_std_diagnostics(&mut kept, &files), 2);
        let messages: Vec<&str> = kept.iter().map(|d| d.message.as_str()).collect();
        assert_eq!(messages, ["in a std tree", "in the project"]);
        // Only warnings hidden: nothing to say.
        let warnings = vec![FileDiagnostic::warning(0, span, "a warning in the embedded std")];
        assert!(render_shown(&warnings, &files).is_empty());
    }
}

/// [decl-id] A message as the user reads it: a type key in it is shown as
/// its written name (`typekey::shown_text`), so a diagnostic built from a
/// key never prints its module separator.
fn shown(message: String) -> String {
    match crate::typekey::shown_text(&message) {
        std::borrow::Cow::Borrowed(_) => message,
        std::borrow::Cow::Owned(s) => s,
    }
}
