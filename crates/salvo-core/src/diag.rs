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
}

impl FileDiagnostic {
    pub fn error(file: usize, span: Span, message: impl Into<String>) -> Self {
        FileDiagnostic {
            file,
            severity: Severity::Error,
            message: message.into(),
            span,
        }
    }

    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }

    /// Renders against the file list this diagnostic was produced from,
    /// e.g. `error: unknown effect --> main.sv:3:7` with a caret line.
    pub fn render(&self, files: &[SourceFile]) -> String {
        let f = &files[self.file];
        Diagnostic {
            severity: self.severity,
            message: self.message.clone(),
            span: self.span,
        }
        .render(&f.name, &f.content)
    }
}
