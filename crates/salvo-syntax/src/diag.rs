//! Diagnostics (errors and warnings) with source spans.

use std::sync::atomic::{AtomicBool, Ordering};

use crate::messages::Message;
use crate::span::Span;

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
    /// [diag-id] What the diagnostic is, `label:name` (`crate::messages`);
    /// `None` only for a diagnostic not yet given one (TESTS.md T2).
    pub id: Option<&'static str>,
}

/// [diag-id] What a diagnostic says: a [`Message`], whose id it carries, or
/// — while the diagnostics are given ids (TESTS.md T2) — bare text.
pub trait IntoMessage {
    fn into_message(self) -> (Option<&'static str>, String);
}

impl IntoMessage for Message {
    fn into_message(self) -> (Option<&'static str>, String) {
        (Some(self.id), self.text)
    }
}

/// Bare text, without an id: what a diagnostic not yet given one says
/// (TESTS.md T2). Goes when every diagnostic has one.
impl<T: Into<String>> IntoMessage for T {
    fn into_message(self) -> (Option<&'static str>, String) {
        (None, self.into())
    }
}

/// [diag-id] Whether rendered diagnostics show their ids — the CLI's `-v`.
static SHOW_IDS: AtomicBool = AtomicBool::new(false);

/// [diag-id] Shows ids in every diagnostic rendered from now on (`salvo -v`).
pub fn show_ids(on: bool) {
    SHOW_IDS.store(on, Ordering::Relaxed);
}

/// [diag-id] Whether ids are shown.
pub fn ids_shown() -> bool {
    SHOW_IDS.load(Ordering::Relaxed)
}

/// [diag-id] `error`, or `error[id]` when ids are shown and there is one.
pub fn severity_label(severity: Severity, id: Option<&str>) -> String {
    let sev = match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
    };
    match id.filter(|_| ids_shown()) {
        Some(id) => format!("{sev}[{id}]"),
        None => sev.to_string(),
    }
}

impl Diagnostic {
    pub fn error(message: impl IntoMessage, span: Span) -> Self {
        let (id, message) = message.into_message();
        Diagnostic { severity: Severity::Error, message, span, id }
    }

    pub fn warning(message: impl IntoMessage, span: Span) -> Self {
        let (id, message) = message.into_message();
        Diagnostic { severity: Severity::Warning, message, span, id }
    }

    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }

    /// Renders the diagnostic against its source file, e.g.
    /// `error: unexpected token --> std/core/list.sv:3:7`.
    pub fn render(&self, file_name: &str, source: &str) -> String {
        let (line, col) = crate::span::span_line_col(source, self.span);
        let sev = severity_label(self.severity, self.id);
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
