//! Diagnostics (errors and warnings) with source spans.

use crate::span::{line_col, Span};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Clone, Debug)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    pub span: Span,
}

impl Diagnostic {
    pub fn error(message: impl Into<String>, span: Span) -> Self {
        Diagnostic {
            severity: Severity::Error,
            message: message.into(),
            span,
        }
    }

    pub fn warning(message: impl Into<String>, span: Span) -> Self {
        Diagnostic {
            severity: Severity::Warning,
            message: message.into(),
            span,
        }
    }

    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }

    /// Renders the diagnostic against its source file, e.g.
    /// `error: unexpected token --> std/core/list.sv:3:7`.
    pub fn render(&self, file_name: &str, source: &str) -> String {
        let (line, col) = line_col(source, self.span.start);
        let sev = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        let mut out = format!("{sev}: {} --> {file_name}:{line}:{col}", self.message);
        if let Some(line_text) = source.lines().nth(line as usize - 1) {
            out.push_str(&format!("\n  {line} | {line_text}"));
            let pad = " ".repeat(line.to_string().len() + col as usize + 2);
            let underline_len = (self.span.len().max(1) as usize)
                .min(line_text.len().saturating_sub(col as usize - 1).max(1));
            out.push_str(&format!("\n  {pad}{}", "^".repeat(underline_len)));
        }
        out
    }
}
