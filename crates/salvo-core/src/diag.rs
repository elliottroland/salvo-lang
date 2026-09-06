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

use salvo_syntax::diag::{Diagnostic, Severity};
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
}

impl FileDiagnostic {
    pub fn error(file: usize, span: Span, message: impl Into<String>) -> Self {
        FileDiagnostic {
            file,
            severity: Severity::Error,
            message: message.into(),
            span,
            suggested_imports: Vec::new(),
        }
    }

    /// A diagnostic that reports something the author probably did not
    /// intend without rejecting the program — used where a rule
    /// deliberately degrades instead of failing (a suppressed refinement
    /// conflict [qual-refn-conflict]).
    pub fn warning(file: usize, span: Span, message: impl Into<String>) -> Self {
        FileDiagnostic {
            file,
            severity: Severity::Warning,
            message: message.into(),
            span,
            suggested_imports: Vec::new(),
        }
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
        }
        .render(&f.name, &f.content);
        for import in &self.suggested_imports {
            out.push_str(&format!("\n  help: add `import {import}`"));
        }
        out
    }
}
