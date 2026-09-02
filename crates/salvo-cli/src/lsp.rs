//! `salvo lsp`: a language server speaking LSP over stdio [cli-lsp].
//!
//! The server reuses the `analyze` pipeline ([cli-analyze],
//! `analysis::analyze_sources`) on every document event — the compiler is
//! fast enough at Salvo-project scale that there is no incremental state:
//! open-editor buffers are handed to the pipeline as an overlay and the
//! whole workspace is re-checked.
//!
//! Supported: publish-diagnostics on open/change/close/save (with
//! clearing), and hover showing the checker's type for the smallest
//! expression under the cursor (`Checked::expr_ty` [diag-structured]).
//! Positions are converted between byte offsets (Salvo spans) and UTF-16
//! line/character pairs (the LSP default encoding).

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::path::PathBuf;
use std::process::ExitCode;

use lsp_server::{Connection, Message, Notification, Request, Response};
use lsp_types::notification::{
    DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, DidSaveTextDocument,
    Notification as _, PublishDiagnostics,
};
use lsp_types::request::{CodeActionRequest, HoverRequest, Request as _};
use lsp_types::{
    CodeAction, CodeActionKind, CodeActionOrCommand, CodeActionParams,
    CodeActionProviderCapability, Diagnostic, DiagnosticSeverity,
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    DidSaveTextDocumentParams, Hover, HoverContents, HoverParams, HoverProviderCapability,
    InitializeParams, LanguageString, MarkedString, Position, PublishDiagnosticsParams,
    Range, ServerCapabilities, TextDocumentSyncCapability, TextDocumentSyncKind, TextEdit,
    Url, WorkspaceEdit,
};

use salvo_core::{Checked, FileDiagnostic, FnKey, ParamDeduction, Program, Ty};
use salvo_syntax::ast::Item;
use salvo_syntax::diag::Severity;
use salvo_syntax::Span;

use crate::analysis::{analyze_sources, Analysis};

/// Runs the server until the client disconnects or asks for shutdown.
/// `filter`/`native_ext` select a backend's define files, exactly like
/// `analyze --backend` [cli-analyze].
pub fn run(filter: String, native_ext: String) -> ExitCode {
    match serve(filter, native_ext) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("salvo lsp: {err}");
            ExitCode::FAILURE
        }
    }
}

fn serve(filter: String, native_ext: String) -> Result<(), Box<dyn Error + Sync + Send>> {
    let (connection, io_threads) = Connection::stdio();

    let capabilities = serde_json::to_value(ServerCapabilities {
        // Full-document sync: the analysis re-reads everything anyway.
        text_document_sync: Some(TextDocumentSyncCapability::Kind(
            TextDocumentSyncKind::FULL,
        )),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        // Quickfixes adding suggested imports [diag-import-suggest].
        code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
        ..Default::default()
    })?;
    let init_params: InitializeParams =
        serde_json::from_value(connection.initialize(capabilities)?)?;

    // The workspace root: the client's rootUri, falling back to the
    // current directory.
    #[allow(deprecated)] // rootUri itself is deprecated in LSP, still ubiquitous
    let root = init_params
        .root_uri
        .as_ref()
        .and_then(|uri| uri.to_file_path().ok())
        .unwrap_or_else(|| PathBuf::from("."));
    let root = root.canonicalize().unwrap_or(root);

    let mut server = Server {
        root,
        filter,
        native_ext,
        overlay: HashMap::new(),
        doc_uris: HashMap::new(),
        published: HashSet::new(),
        connection: &connection,
    };
    server.main_loop()?;
    drop(server);
    // The writer thread only terminates once its channel sender — owned
    // by the connection — is dropped; joining first would deadlock.
    drop(connection);
    io_threads.join()?;
    Ok(())
}

struct Server<'c> {
    root: PathBuf,
    filter: String,
    native_ext: String,
    /// Open-editor buffers: canonical absolute path -> contents [cli-lsp].
    overlay: HashMap<PathBuf, String>,
    /// The URI each open document was opened under: diagnostics must be
    /// published against the client's own URI (clients compare exactly;
    /// canonicalization may have rewritten the path, e.g. macOS
    /// `/var` -> `/private/var`).
    doc_uris: HashMap<PathBuf, Url>,
    /// URIs we last published diagnostics for; publishing an empty list
    /// clears them client-side when they disappear.
    published: HashSet<Url>,
    connection: &'c Connection,
}

impl Server<'_> {
    fn main_loop(&mut self) -> Result<(), Box<dyn Error + Sync + Send>> {
        while let Ok(msg) = self.connection.receiver.recv() {
            match msg {
                Message::Request(req) => {
                    if self.connection.handle_shutdown(&req)? {
                        return Ok(());
                    }
                    self.handle_request(req)?;
                }
                Message::Notification(note) => self.handle_notification(note)?,
                Message::Response(_) => {}
            }
        }
        Ok(())
    }

    fn handle_request(&mut self, req: Request) -> Result<(), Box<dyn Error + Sync + Send>> {
        match req.method.as_str() {
            HoverRequest::METHOD => {
                let params: HoverParams = serde_json::from_value(req.params)?;
                let result = self.hover(&params);
                self.respond(Response::new_ok(req.id, result))?;
            }
            CodeActionRequest::METHOD => {
                let params: CodeActionParams = serde_json::from_value(req.params)?;
                let result = self.code_actions(&params);
                self.respond(Response::new_ok(req.id, result))?;
            }
            _ => self.respond(Response::new_err(
                req.id,
                lsp_server::ErrorCode::MethodNotFound as i32,
                format!("unsupported request `{}`", req.method),
            ))?,
        }
        Ok(())
    }

    fn handle_notification(
        &mut self,
        note: Notification,
    ) -> Result<(), Box<dyn Error + Sync + Send>> {
        match note.method.as_str() {
            DidOpenTextDocument::METHOD => {
                let params: DidOpenTextDocumentParams = serde_json::from_value(note.params)?;
                if let Some(path) = file_path(&params.text_document.uri) {
                    self.doc_uris.insert(path.clone(), params.text_document.uri);
                    self.overlay.insert(path, params.text_document.text);
                    self.publish_diagnostics()?;
                }
            }
            DidChangeTextDocument::METHOD => {
                let params: DidChangeTextDocumentParams = serde_json::from_value(note.params)?;
                // Full sync: the last change carries the whole document.
                if let (Some(path), Some(change)) = (
                    file_path(&params.text_document.uri),
                    params.content_changes.into_iter().last(),
                ) {
                    self.doc_uris.insert(path.clone(), params.text_document.uri);
                    self.overlay.insert(path, change.text);
                    self.publish_diagnostics()?;
                }
            }
            DidCloseTextDocument::METHOD => {
                let params: DidCloseTextDocumentParams = serde_json::from_value(note.params)?;
                if let Some(path) = file_path(&params.text_document.uri) {
                    // Back to the on-disk contents.
                    self.overlay.remove(&path);
                    self.doc_uris.remove(&path);
                    self.publish_diagnostics()?;
                }
            }
            DidSaveTextDocument::METHOD => {
                let _params: DidSaveTextDocumentParams = serde_json::from_value(note.params)?;
                self.publish_diagnostics()?;
            }
            _ => {}
        }
        Ok(())
    }

    fn analyze(&self) -> Option<Analysis> {
        match analyze_sources(&self.root, &self.filter, &self.native_ext, &self.overlay) {
            Ok(analysis) => Some(analysis),
            Err(err) => {
                eprintln!("salvo lsp: {err}");
                None
            }
        }
    }

    /// Re-analyzes the workspace and publishes diagnostics per file URI,
    /// clearing files that no longer have any [cli-lsp].
    fn publish_diagnostics(&mut self) -> Result<(), Box<dyn Error + Sync + Send>> {
        let Some(analysis) = self.analyze() else {
            return Ok(());
        };
        // Every open document gets a publish — an explicit empty list
        // tells the client the file was re-checked and is clean, and
        // clears anything it showed before.
        let mut by_uri: HashMap<Url, Vec<Diagnostic>> = self
            .doc_uris
            .values()
            .map(|uri| (uri.clone(), Vec::new()))
            .collect();
        for diag in &analysis.diagnostics {
            let file = &analysis.program.files[diag.file];
            if file.is_std {
                // Embedded std has no on-disk URI; it should be clean.
                eprintln!("salvo lsp: diagnostic in std: {}", diag.message);
                continue;
            }
            // Open documents keep the URI the client opened them under
            // (clients compare URIs exactly).
            let path = self.root.join(&file.name);
            let uri = match self.doc_uris.get(&path) {
                Some(uri) => uri.clone(),
                None => match Url::from_file_path(&path) {
                    Ok(uri) => uri,
                    Err(()) => continue,
                },
            };
            by_uri
                .entry(uri)
                .or_default()
                .push(to_lsp_diagnostic(diag, &file.content));
        }

        let current: HashSet<Url> = by_uri.keys().cloned().collect();
        for uri in self.published.difference(&current) {
            self.send_diagnostics(uri.clone(), Vec::new())?;
        }
        for (uri, diagnostics) in by_uri {
            self.send_diagnostics(uri, diagnostics)?;
        }
        self.published = current;
        Ok(())
    }

    fn send_diagnostics(
        &self,
        uri: Url,
        diagnostics: Vec<Diagnostic>,
    ) -> Result<(), Box<dyn Error + Sync + Send>> {
        let params = PublishDiagnosticsParams {
            uri,
            diagnostics,
            version: None,
        };
        self.connection
            .sender
            .send(Message::Notification(Notification::new(
                PublishDiagnostics::METHOD.to_string(),
                params,
            )))?;
        Ok(())
    }

    /// Hover contents for the position: a fn's full signature when the
    /// cursor is on a fn name ([fn-ref-table] — declaration, callee, or
    /// fn-by-name reference), otherwise the checker's type for the
    /// smallest expression under the cursor (`Checked::expr_ty`); `None`
    /// on unknown types or positions without a typed expression.
    fn hover(&self, params: &HoverParams) -> Option<Hover> {
        let doc = &params.text_document_position_params;
        let path = file_path(&doc.text_document.uri)?;
        let analysis = self.analyze()?;
        let checked = analysis.checked?;

        let file_idx = analysis
            .program
            .files
            .iter()
            .position(|f| !f.is_std && self.root.join(&f.name) == path)?;
        let content = &analysis.program.files[file_idx].content;
        let offset = position_to_offset(content, doc.position);

        // A fn name under the cursor hovers as the full signature,
        // including inferred deductions [fn-ref-table].
        let mut best_ref: Option<(Span, FnKey)> = None;
        for ((file, span), key) in &checked.fn_refs {
            if *file != file_idx {
                continue;
            }
            if span.start <= offset
                && offset < span.end
                && best_ref.is_none_or(|(b, _)| span.len() < b.len())
            {
                best_ref = Some((*span, *key));
            }
        }
        if let Some((span, key)) = best_ref {
            if let Some(signature) = fn_signature(&analysis.program, &checked, key) {
                return Some(Hover {
                    contents: HoverContents::Scalar(MarkedString::LanguageString(
                        LanguageString {
                            language: "salvo".to_string(),
                            value: signature,
                        },
                    )),
                    range: Some(span_to_range(content, span)),
                });
            }
        }

        let mut best: Option<(Span, &Ty)> = None;
        for ((file, span), ty) in &checked.expr_ty {
            if *file != file_idx || matches!(ty, Ty::Unknown) {
                continue;
            }
            if span.start <= offset
                && offset < span.end
                && best.is_none_or(|(b, _)| span.len() < b.len())
            {
                best = Some((*span, ty));
            }
        }
        let (span, ty) = best?;
        // A fate-linked (derived) variable presents its compiler
        // qualifier: the type line carries a bare `ReadOnly`, and the
        // qualifier's parameters (roots, binding sites) follow as
        // on-request detail [fate-link] (progressive disclosure — user
        // decision 2026-09-02).
        if let Some(reads) = checked.fate_reads.get(&(file_idx, span)) {
            let roots: Vec<String> = reads
                .iter()
                .map(|r| {
                    let pos = span_to_range(content, r.bind_span).start;
                    format!(
                        "`{}` (bound at {}:{})",
                        r.root,
                        pos.line + 1,
                        pos.character + 1
                    )
                })
                .collect();
            let detail = format!(
                "Compiler qualifier `ReadOnly` — shares fate with {}. Reads are \
                 free; moving or mutating it is rejected; `copy(...)` makes an \
                 independent value.",
                roots.join(", ")
            );
            return Some(Hover {
                contents: HoverContents::Array(vec![
                    MarkedString::LanguageString(LanguageString {
                        language: "salvo".to_string(),
                        value: format!("ReadOnly {ty}"),
                    }),
                    MarkedString::String(detail),
                ]),
                range: Some(span_to_range(content, span)),
            });
        }
        Some(Hover {
            contents: HoverContents::Scalar(MarkedString::LanguageString(LanguageString {
                language: "salvo".to_string(),
                value: ty.to_string(),
            })),
            range: Some(span_to_range(content, span)),
        })
    }

    /// Quickfix code actions for the import suggestions carried on
    /// diagnostics [diag-import-suggest]: one "Add `import …`" action per
    /// suggested path, inserting the import line after the file's last
    /// existing import (or at the top). The suggestions ride on
    /// `Diagnostic::data`, which the client echoes back in
    /// `context.diagnostics` — no re-analysis needed.
    fn code_actions(&self, params: &CodeActionParams) -> Vec<CodeActionOrCommand> {
        let Some(path) = file_path(&params.text_document.uri) else {
            return Vec::new();
        };
        let content = match self.overlay.get(&path) {
            Some(content) => content.clone(),
            None => match std::fs::read_to_string(&path) {
                Ok(content) => content,
                Err(_) => return Vec::new(),
            },
        };
        let insert = import_insert_position(&content);

        let mut actions = Vec::new();
        for diag in &params.context.diagnostics {
            let imports = diag
                .data
                .as_ref()
                .and_then(|d| d.get("imports"))
                .and_then(|v| v.as_array());
            let Some(imports) = imports else { continue };
            for import in imports.iter().filter_map(|v| v.as_str()) {
                let edit = TextEdit {
                    range: Range::new(insert, insert),
                    new_text: format!("import {import}\n"),
                };
                let changes =
                    HashMap::from([(params.text_document.uri.clone(), vec![edit])]);
                actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                    title: format!("Add `import {import}`"),
                    kind: Some(CodeActionKind::QUICKFIX),
                    diagnostics: Some(vec![diag.clone()]),
                    edit: Some(WorkspaceEdit {
                        changes: Some(changes),
                        ..Default::default()
                    }),
                    ..Default::default()
                }));
            }
        }
        actions
    }

    fn respond(&self, response: Response) -> Result<(), Box<dyn Error + Sync + Send>> {
        self.connection.sender.send(Message::Response(response))?;
        Ok(())
    }
}

/// Where a new `import` line goes: after the last existing top-level
/// import, or at the very top of the file [diag-import-suggest].
fn import_insert_position(content: &str) -> Position {
    let mut line = 0u32;
    for (i, text) in content.lines().enumerate() {
        if text.trim_start().starts_with("import ") {
            line = i as u32 + 1;
        }
    }
    Position::new(line, 0)
}

/// Renders the full source-like signature of a fn declaration, with the
/// *effective* deduction list — inferred/validated (`Checked::deductions`)
/// when available, else as declared — and an explicit return type
/// [fn-ref-table]:
///
/// ```text
/// fn add(a: Int, b: Int) -> [] Int
/// fn greet(person: Person) [Console] -> [person] None
/// fn ok<T>(value: T) -> [] T as Ok
/// ```
fn fn_signature(program: &Program, checked: &Checked, key: FnKey) -> Option<String> {
    let module = program.modules.get(key.file)?;
    let Item::Fn(decl) = module.items.get(key.item)? else {
        return None;
    };

    let mut sig = String::from("fn ");
    sig.push_str(&decl.name.name);
    if !decl.generics.is_empty() {
        let generics: Vec<&str> = decl.generics.iter().map(|g| g.name.as_str()).collect();
        sig.push_str(&format!("<{}>", generics.join(", ")));
    }
    let params: Vec<String> = decl
        .params
        .iter()
        .map(|p| {
            format!(
                "{}{}: {}",
                if p.variadic { "..." } else { "" },
                p.name.name,
                p.ty
            )
        })
        .collect();
    sig.push_str(&format!("({})", params.join(", ")));

    if let Some(effects) = &decl.effects {
        let effects: Vec<String> = effects.iter().map(|e| e.to_string()).collect();
        sig.push_str(&format!(" [{}]", effects.join(", ")));
    }

    sig.push_str(" ->");
    // Deductions: the inferred/validated list when the whole-program pass
    // produced one, else the declared list. Kept params render with their
    // remaining qualifiers (`name:` when a qualified param keeps none);
    // moved params are omitted [deduce-syntax].
    if let Some(deductions) = checked.deductions.get(&key) {
        sig.push_str(&format!(" {}", render_deductions(deductions, decl)));
    } else if let Some(declared) = &decl.deductions {
        let entries: Vec<String> = declared
            .iter()
            .map(|d| {
                let quals: Vec<String> = d.qualifiers.iter().map(|q| q.to_string()).collect();
                if quals.is_empty() {
                    if d.explicit {
                        format!("{}:", d.param.name)
                    } else {
                        d.param.name.clone()
                    }
                } else {
                    format!("{}: {}", d.param.name, quals.join(" "))
                }
            })
            .collect();
        sig.push_str(&format!(" [{}]", entries.join(", ")));
    }

    match &decl.return_type {
        Some(ty) => sig.push_str(&format!(" {ty}")),
        None => sig.push_str(" None"),
    }
    if let Some(cref) = &decl.constructs {
        sig.push_str(&format!(" as {cref}"));
    }
    Some(sig)
}

/// Renders an effective deduction list: kept parameters with their
/// remaining qualifiers, moved parameters omitted [deduce-syntax]. A
/// qualified parameter kept with *no* qualifiers renders in the
/// explicit-empty form (`name:`) to distinguish it from an unqualified
/// bare keep.
fn render_deductions(
    deductions: &[ParamDeduction],
    decl: &salvo_syntax::ast::FnDecl,
) -> String {
    let entries: Vec<String> = deductions
        .iter()
        .filter(|d| d.kept)
        .map(|d| {
            if !d.quals.is_empty() {
                return format!("{}: {}", d.param, d.quals.join(" "));
            }
            let declares_quals = decl
                .params
                .iter()
                .find(|p| p.name.name == d.param)
                .is_some_and(|p| !salvo_core::deduce::declared_quals(&p.ty).is_empty());
            if declares_quals {
                format!("{}:", d.param)
            } else {
                d.param.clone()
            }
        })
        .collect();
    format!("[{}]", entries.join(", "))
}

/// URI -> canonical absolute path (`None` for non-file URIs, which the
/// server ignores). For files that do not exist on disk yet (unsaved
/// buffers), the *parent* is canonicalized so symlinked directories
/// (macOS `/var` -> `/private/var`) still match the workspace root.
fn file_path(uri: &Url) -> Option<PathBuf> {
    let path = uri.to_file_path().ok()?;
    if let Ok(canonical) = path.canonicalize() {
        return Some(canonical);
    }
    let (Some(parent), Some(name)) = (path.parent(), path.file_name()) else {
        return Some(path);
    };
    match parent.canonicalize() {
        Ok(parent) => Some(parent.join(name)),
        Err(_) => Some(path),
    }
}

fn to_lsp_diagnostic(diag: &FileDiagnostic, content: &str) -> Diagnostic {
    Diagnostic {
        range: span_to_range(content, diag.span),
        severity: Some(match diag.severity {
            Severity::Error => DiagnosticSeverity::ERROR,
            Severity::Warning => DiagnosticSeverity::WARNING,
        }),
        source: Some("salvo".to_string()),
        message: diag.message.clone(),
        // Import suggestions ride along for codeAction [diag-import-suggest].
        data: (!diag.suggested_imports.is_empty())
            .then(|| serde_json::json!({ "imports": diag.suggested_imports })),
        ..Default::default()
    }
}

fn span_to_range(content: &str, span: Span) -> Range {
    Range {
        start: offset_to_position(content, span.start),
        end: offset_to_position(content, span.end),
    }
}

/// Byte offset -> LSP position (0-based line, UTF-16 code-unit column).
pub fn offset_to_position(text: &str, offset: u32) -> Position {
    let mut offset = (offset as usize).min(text.len());
    // Clamp to a char boundary (spans always are; belt and braces).
    while offset > 0 && !text.is_char_boundary(offset) {
        offset -= 1;
    }
    let mut line = 0u32;
    let mut line_start = 0usize;
    for (i, b) in text.as_bytes()[..offset].iter().enumerate() {
        if *b == b'\n' {
            line += 1;
            line_start = i + 1;
        }
    }
    let character = text[line_start..offset]
        .chars()
        .map(|c| c.len_utf16() as u32)
        .sum();
    Position { line, character }
}

/// LSP position (0-based line, UTF-16 code-unit column) -> byte offset,
/// clamped to the document / line end.
pub fn position_to_offset(text: &str, position: Position) -> u32 {
    let mut line_start = 0usize;
    if position.line > 0 {
        let mut line = 0u32;
        let mut found = false;
        for (i, b) in text.bytes().enumerate() {
            if b == b'\n' {
                line += 1;
                if line == position.line {
                    line_start = i + 1;
                    found = true;
                    break;
                }
            }
        }
        if !found {
            return text.len() as u32;
        }
    }
    let mut units = 0u32;
    for (i, c) in text[line_start..].char_indices() {
        if c == '\n' || units >= position.character {
            return (line_start + i) as u32;
        }
        units += c.len_utf16() as u32;
    }
    text.len() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    // [cli-lsp] Byte offsets and UTF-16 positions round-trip, including
    // multi-byte (é: 2 bytes / 1 unit) and supplementary-plane characters
    // (🎉: 4 bytes / 2 units).
    #[test]
    fn utf16_position_mapping() {
        let text = "let é = \"🎉\"\nlet x = 1\n";
        // Offset of `x` on line 1: line 0 is 14 bytes + newline.
        let x_off = text.find("x").unwrap() as u32;
        assert_eq!(offset_to_position(text, x_off), Position::new(1, 4));
        assert_eq!(position_to_offset(text, Position::new(1, 4)), x_off);

        // `é` is 1 UTF-16 unit but 2 bytes.
        let quote_off = text.find('"').unwrap() as u32;
        assert_eq!(offset_to_position(text, quote_off), Position::new(0, 8));
        // The emoji counts as 2 UTF-16 units: the closing quote sits at
        // unit 11 (8 + open quote + 2).
        let close_off = text.rfind('"').unwrap() as u32;
        assert_eq!(offset_to_position(text, close_off), Position::new(0, 11));
        assert_eq!(position_to_offset(text, Position::new(0, 11)), close_off);
    }

    #[test]
    fn positions_clamp_to_document_bounds() {
        let text = "ab\ncd";
        assert_eq!(position_to_offset(text, Position::new(0, 99)), 2);
        assert_eq!(position_to_offset(text, Position::new(9, 0)), 5);
        assert_eq!(offset_to_position(text, 99), Position::new(1, 2));
    }
}
