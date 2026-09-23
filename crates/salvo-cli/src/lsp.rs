//! `salvo lsp`: a language server speaking LSP over stdio [cli-lsp].
//!
//! The server reuses the `analyze` pipeline ([cli-analyze],
//! `analysis::analyze_sources`) on every document event — the compiler is
//! fast enough at Salvo-project scale that there is no incremental state:
//! open-editor buffers are handed to the pipeline as an overlay and the
//! whole workspace is re-checked.
//!
//! Supported: publish-diagnostics on open/change/close/save (with
//! clearing), hover showing the checker's type for the smallest
//! expression under the cursor (`Checked::expr_ty` [diag-structured]),
//! and go-to-definition [lsp-definition].
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
use lsp_types::request::{
    CodeActionRequest, GotoDefinition, HoverRequest, Request as _,
};
use lsp_types::{
    CodeAction, CodeActionKind, CodeActionOrCommand, CodeActionParams,
    CodeActionProviderCapability, Diagnostic, DiagnosticSeverity,
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    DidSaveTextDocumentParams, GotoDefinitionParams, GotoDefinitionResponse, Hover,
    HoverContents, HoverParams, HoverProviderCapability, InitializeParams,
    Location, MarkupContent, MarkupKind, OneOf, Position, PublishDiagnosticsParams, Range,
    ServerCapabilities, TextDocumentSyncCapability, TextDocumentSyncKind, TextEdit, Url,
    WorkspaceEdit,
};

use salvo_core::{
    Checked, DefSite, FileDiagnostic, FnKey, ParamDeduction, Program, QualEffect, Ty,
};
use salvo_syntax::ast::{FnDecl, Item, QualSubject};
use salvo_syntax::diag::Severity;
use salvo_syntax::Span;

use crate::analysis::{analyze_sources, Analysis};
use crate::docs;

/// Runs the server until the client disconnects or asks for shutdown.
/// Backend-neutral, like `salvo analyze` [cli-analyze]: nothing a backend
/// selects participates in checking.
pub fn run() -> ExitCode {
    match serve() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("salvo lsp: {err}");
            ExitCode::FAILURE
        }
    }
}

fn serve() -> Result<(), Box<dyn Error + Sync + Send>> {
    let (connection, io_threads) = Connection::stdio();

    let capabilities = serde_json::to_value(ServerCapabilities {
        // Full-document sync: the analysis re-reads everything anyway.
        text_document_sync: Some(TextDocumentSyncCapability::Kind(
            TextDocumentSyncKind::FULL,
        )),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        // Go-to-definition [lsp-definition].
        definition_provider: Some(OneOf::Left(true)),
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
            GotoDefinition::METHOD => {
                let params: GotoDefinitionParams = serde_json::from_value(req.params)?;
                let result = self.goto_definition(&params);
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
        match analyze_sources(&self.root, "", &self.overlay) {
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

    /// Hover contents for the position, as markdown: a `salvo` code block
    /// with the declaration or type, plus the doc comment of the
    /// declaration under the cursor [doc-comment].
    ///
    /// * a fn name — the full signature with inferred deductions
    ///   ([fn-ref-table]) and its docs;
    /// * any other declared name — its declaration line and docs, with a
    ///   **Fields** section for a struct [doc-struct-fields];
    /// * anything else — the checker's type for the smallest expression
    ///   under the cursor (`Checked::expr_ty`), which for a variable is
    ///   the type *known at that point*, qualifiers included
    ///   [doc-hover-narrowed].
    fn hover(&self, params: &HoverParams) -> Option<Hover> {
        let doc = &params.text_document_position_params;
        let path = file_path(&doc.text_document.uri)?;
        let analysis = self.analyze()?;
        let checked = analysis.checked.as_ref()?;

        let file_idx = analysis
            .program
            .files
            .iter()
            .position(|f| !f.is_std && self.root.join(&f.name) == path)?;
        let content = &analysis.program.files[file_idx].content;
        let offset = position_to_offset(content, doc.position);
        let link = self.doc_linker(&analysis.program);

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
                let decl = match analysis.program.modules[key.file].items.get(key.item) {
                    Some(Item::Fn(decl)) => Some(decl),
                    _ => None,
                };
                let docs = decl.and_then(|decl| {
                    let source = &analysis.program.files.get(key.file)?.content;
                    let scope = docs::fn_scope(
                        decl,
                        key.file,
                        source,
                        &analysis.program.modules,
                        &link,
                    );
                    docs::render(&decl.docs, &scope)
                });
                // [qual-refn-docs] The refinements that apply *here* — in
                // the file the cursor is in, since that is what decides
                // which qualifiers are in scope — with their own docs
                // merged in. A refinement is written somewhere else
                // entirely, so this is the only place a reader can find it.
                let refinements = decl.and_then(|decl| {
                    let groups = checked.refinements.for_call(file_idx, key);
                    let source = &analysis.program.files.get(key.file)?.content;
                    let scope = docs::fn_scope(
                        decl,
                        key.file,
                        source,
                        &analysis.program.modules,
                        &link,
                    );
                    docs::refinement_section(groups, &scope)
                });
                // [lsp-fn-origin] Which module the *resolved* overload came
                // from. With overloading by scope ladder [fn-overload-scope]
                // this is load-bearing rather than decoration: `size` may be
                // std's, an import's, or this module's, and the signature
                // alone does not say which won.
                let origin = self.origin_section(&analysis, key.file, file_idx);
                // [lsp-hover-overloads] The rest of the overload set, so the
                // resolved signature reads as *a choice* rather than as the
                // only candidate.
                let others = decl.and_then(|d| {
                    self.overload_section(
                        &analysis,
                        file_idx,
                        &d.name.name,
                        DefSite {
                            file: key.file,
                            span: d.name.span,
                        },
                    )
                });
                return Some(markdown_hover(
                    docs::hover_markdown(&signature, &[docs, refinements, origin, others]),
                    span_to_range(content, span),
                ));
            }
        }

        // Any other declared name: the declaration under the cursor, or
        // the declaration a name refers to [lsp-definition].
        if let Some(hover) = self.declaration_hover(&analysis, &checked, file_idx, offset, &link) {
            return Some(hover);
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
        // qualifier: the type line carries a bare `proj`, and the
        // qualifier's parameters (roots, binding sites) follow as
        // on-request detail [fate-link] (progressive disclosure — user
        // decision 2026-09-02).
        if let Some(reads) = checked.fate_reads.get(&(file_idx, span)) {
            let roots: Vec<String> = reads
                .iter()
                .map(|r| {
                    let pos = span_to_range(content, r.bind_span).start;
                    // [fate-field-disjoint] Name the storage actually
                    // shared: a link to `p.name` is not a link to all of
                    // `p`, and the hover must not overstate it.
                    let place = match &r.path {
                        Some(path) => format!("{}{}", r.root, path),
                        None => r.root.clone(),
                    };
                    format!(
                        "`{}` (bound at {}:{})",
                        place,
                        pos.line + 1,
                        pos.character + 1
                    )
                })
                .collect();
            let detail = format!(
                "Compiler qualifier `proj` — shares fate with {}. Reads are \
                 free; moving or mutating it is rejected; `copy(...)` makes an \
                 independent value.",
                roots.join(", ")
            );
            // [proj-type] The prefix is skipped when the *type* already leads
            // with a `proj` — a `get` result is `(proj(list) T)?`, and
            // the fate link and the declared borrow are the same claim, so
            // prefixing one anyway read `proj proj T?` (the heap demo's item 3,
            // fixed 2026-09-22). The detail line still names the roots, which
            // is the part the type does not carry.
            let line = if ty.presents_proj() {
                ty.to_string()
            } else {
                format!("proj {ty}")
            };
            return Some(markdown_hover(
                docs::hover_markdown(&line, &[Some(detail)]),
                span_to_range(content, span),
            ));
        }
        // The variable's type as narrowed at this position, with its
        // qualifiers; `repr_ty` holds the declared type whenever flow
        // analysis narrowed it, which is the fact worth showing next to
        // it [doc-hover-narrowed].
        let declared = checked
            .repr_ty
            .get(&(file_idx, span))
            .filter(|d| **d != *ty)
            .map(|d| {
                format!(
                    "Declared as `{d}` — narrowed here by an `is` test or a `when` arm.",
                )
            });
        Some(markdown_hover(
            docs::hover_markdown(&ty.to_string(), &[declared]),
            span_to_range(content, span),
        ))
    }


    /// [lsp-fn-origin] Where a declaration came from, as a hover section: the
    /// module, and what kind of place that is — the standard library, another
    /// file of the program, or the file being edited. The module path is what
    /// an `@module` selector would name [fn-overload-at], so a reader can act
    /// on it directly.
    fn origin_section(
        &self,
        analysis: &Analysis,
        decl_file: usize,
        here: usize,
    ) -> Option<String> {
        let file = analysis.program.files.get(decl_file)?;
        let module = file.module.to_string();
        if file.is_std {
            return Some(format!("From `{module}` — the standard library."));
        }
        if decl_file == here {
            return Some("Declared in this file.".to_string());
        }
        let same_module = analysis
            .program
            .files
            .get(here)
            .is_some_and(|h| h.module == file.module);
        Some(if same_module {
            format!("From `{module}` — this module, in `{}`.", file.name)
        } else {
            format!("From `{module}`.")
        })
    }

    /// A markdown link target for a definition site, or `None` for the
    /// embedded std (not on disk, so nothing to link to).
    fn doc_linker<'a>(
        &'a self,
        program: &'a Program,
    ) -> impl Fn(usize, Span) -> Option<String> + 'a {
        move |file: usize, span: Span| {
            let target = program.files.get(file)?;
            if target.is_std {
                return None;
            }
            let target_path = self.root.join(&target.name);
            let uri = match self.doc_uris.get(&target_path) {
                Some(uri) => uri.clone(),
                None => Url::from_file_path(&target_path).ok()?,
            };
            let pos = span_to_range(&target.content, span).start;
            Some(format!("{uri}#L{},{}", pos.line + 1, pos.character + 1))
        }
    }

    /// Hover for a declared name: the declaration the cursor sits on, or
    /// the one a reference points at [lsp-definition]. Structs add a
    /// **Fields** section [doc-struct-fields].
    fn declaration_hover(
        &self,
        analysis: &Analysis,
        checked: &Checked,
        file_idx: usize,
        offset: u32,
        link: &dyn Fn(usize, Span) -> Option<String>,
    ) -> Option<Hover> {
        let content = &analysis.program.files[file_idx].content;
        // The name the cursor is on: a reference to a declaration
        // (`def_refs`) or to a field (`field_refs`), else a declaration's
        // own name span in this file.
        let mut site: Option<(Span, DefSite)> = None;
        for ((file, span), target) in checked.def_refs.iter().chain(&checked.field_refs) {
            if *file != file_idx {
                continue;
            }
            if span.start <= offset
                && offset < span.end
                && site.is_none_or(|(b, _)| span.len() < b.len())
            {
                site = Some((*span, *target));
            }
        }
        let (span, target) = match site {
            Some(found) => found,
            None => {
                let items = &analysis.program.modules.get(file_idx)?.items;
                let hit = |s: Span| s.start <= offset && offset < s.end;
                // [doc-comment] The cursor may be on an `import`'s item name
                // (`import shapes.Point`), which is where a reader looks to
                // learn what was imported. The name is not a *reference* the
                // checker recorded, so it is resolved here by finding the
                // declaration it names (user request 2026-09-11).
                match import_target(items, &hit, analysis) {
                    Some(found) => found,
                    None => {
                        let name_span = decl_name_span_at(items, &hit)?;
                        (
                            name_span,
                            DefSite {
                                file: file_idx,
                                span: name_span,
                            },
                        )
                    }
                }
            }
        };

        let items = &analysis.program.modules.get(target.file)?.items;
        let found = decl_at(items, &|s: Span| s == target.span)?;
        // [lsp-hover-overloads] Read before the rendering match below consumes
        // `found`: the name is what the other declarations are looked up by.
        let decl_name: Option<String> = match &found {
            DeclAt::Member { decl, .. } => Some(decl.name.name.clone()),
            DeclAt::Field { decl, .. } => Some(decl.name.name.clone()),
            DeclAt::Struct(s) => Some(s.name.name.clone()),
            DeclAt::Qualifier(q) => Some(q.name.name.clone()),
            DeclAt::Effect(e) => Some(e.name.name.clone()),
            DeclAt::Params(g) => Some(g.name.name.clone()),
            DeclAt::Handler(h) => Some(h.name.name.clone()),
            DeclAt::Type(t) => Some(t.name.name.clone()),
        };
        let source = &analysis.program.files.get(target.file)?.content;
        let scope = |locals: Vec<(String, Span)>| docs::DocScope {
            locals,
            file: target.file,
            source,
            modules: &analysis.program.modules,
            link,
        };
        let (signature, body, fields) = match found {
            DeclAt::Struct(s) => {
                let scope = docs::struct_scope(
                    s,
                    target.file,
                    source,
                    &analysis.program.modules,
                    link,
                );
                let fields = docs::field_section(&s.fields, &scope);
                (struct_signature(s), docs::render(&s.docs, &scope), fields)
            }
            DeclAt::Qualifier(q) => {
                // Members (`qualifies`, constructors) are in scope for
                // `[symbol]` references in the qualifier's own docs.
                let scope = scope(
                    q.fns
                        .iter()
                        .map(|f| (f.name.name.clone(), f.name.span))
                        .collect(),
                );
                (
                    qualifier_signature(q),
                    docs::render(&q.docs, &scope),
                    // [doc-qualifies-body] A *predicate* qualifier is defined
                    // by its `qualifies`, so a one-line one is shown as the
                    // condition itself. Anything longer is an implementation
                    // rather than a rule, and the qualifier's doc comment is
                    // the better place for it (user decision 2026-09-11).
                    qualifies_section(q, source),
                )
            }
            DeclAt::Effect(e) => {
                let scope = scope(
                    e.fns
                        .iter()
                        .map(|f| (f.name.name.clone(), f.name.span))
                        .collect(),
                );
                (effect_signature(e), docs::render(&e.docs, &scope), None)
            }
            DeclAt::Params(g) => {
                // The group's own members are in scope for `[symbol]`
                // references in its docs [doc-symbol-ref].
                let scope = scope(
                    g.fns
                        .iter()
                        .map(|f| (f.name.name.clone(), f.name.span))
                        .collect(),
                );
                (params_signature(g), docs::render(&g.docs, &scope), None)
            }
            DeclAt::Handler(h) => {
                let scope = scope(
                    h.fns
                        .iter()
                        .map(|f| (f.name.name.clone(), f.name.span))
                        .chain(h.params.iter().map(|p| (p.name.name.clone(), p.name.span)))
                        .chain(h.state.iter().map(|f| (f.name.name.clone(), f.name.span)))
                        .collect(),
                );
                (handler_signature(h), docs::render(&h.docs, &scope), None)
            }
            DeclAt::Type(t) => {
                let scope = scope(Vec::new());
                (type_signature(t), docs::render(&t.docs, &scope), None)
            }
            // A field: its own declaration line, whose declares it, and
            // its docs [doc-struct-fields].
            DeclAt::Field {
                owner,
                siblings,
                decl,
            } => {
                let scope = scope(siblings);
                let default = decl
                    .default
                    .as_ref()
                    .and_then(|e| {
                        let s = e.span();
                        source
                            .get(s.start as usize..s.end as usize)
                            .map(|text| format!(" = {text}"))
                    })
                    .unwrap_or_default();
                (
                    format!("{}: {}{}", decl.name.name, decl.ty, default),
                    docs::render(&decl.docs, &scope),
                    Some(format!("Field of {owner}.")),
                )
            }
            // An effect, handler, or qualifier member fn: its signature
            // rendered from the declaration (members have no `FnKey`, so
            // there are no inferred deductions to show) [doc-comment].
            DeclAt::Member {
                owner,
                siblings,
                decl,
            } => {
                // The member's own parameters and generics, then the
                // owner's names (state, sibling members)
                // [doc-symbol-ref].
                let mut scope = docs::fn_scope(
                    decl,
                    target.file,
                    source,
                    &analysis.program.modules,
                    link,
                );
                scope.locals.extend(siblings);
                (
                    fn_decl_signature(decl, None),
                    docs::render(&decl.docs, &scope),
                    Some(format!("Member of {owner}.")),
                )
            }
        };
        // [lsp-fn-origin] Where it came from, for a declaration that is not
        // in this file — hovering a *local* declaration to be told it is
        // local would be noise, so that case is left out.
        let origin = (target.file != file_idx)
            .then(|| self.origin_section(analysis, target.file, file_idx))
            .flatten();
        // [lsp-hover-overloads] Same-named declarations visible here — the
        // effect-member case is the one that matters most (`read_to` of
        // `core.fs` has three), and it arrives through this path rather than
        // the fn-ref one, since a member has no `FnKey`.
        let others =
            decl_name.and_then(|name| self.overload_section(analysis, file_idx, &name, target));
        Some(markdown_hover(
            docs::hover_markdown(&signature, &[body, fields, origin, others]),
            span_to_range(content, span),
        ))
    }

    /// [lsp-hover-overloads] The *other* declarations visible under this
    /// name, as a hover section (user request 2026-09-18).
    ///
    /// A name in Salvo can carry several declarations at once — overloads by
    /// argument type [fn-overload-rank], effect members of the same name
    /// [effect-member-overload], and both mixed, since a call site cannot tell
    /// a member from a fn. Showing only the one the cursor resolved to hides
    /// the choice that was made: `read_to` in `core.fs` has three, and a
    /// reader hovering it needs to see which three.
    ///
    /// `here` is the resolved declaration, left out of the list; everything
    /// else is rendered from its own declaration, so a member reads as a
    /// member and a fn shows its deductions.
    fn overload_section(
        &self,
        analysis: &Analysis,
        file_idx: usize,
        name: &str,
        here: DefSite,
    ) -> Option<String> {
        let sites = analysis.overloads.get(file_idx)?.get(name)?;
        let mut lines: Vec<String> = Vec::new();
        for site in sites {
            if site.file == here.file && site.span == here.span {
                continue;
            }
            let Some(items) = analysis.program.modules.get(site.file) else {
                continue;
            };
            // A **top-level fn** is not a `DeclAt` — `decl_at` covers the
            // declarations that hover through `declaration_hover`, and a fn
            // resolves through `fn_refs` instead — so it is matched here by
            // its name span before falling back to the declaration kinds.
            let fn_decl = items.items.iter().find_map(|item| match item {
                Item::Fn(f) if f.name.span == site.span => Some(f),
                _ => None,
            });
            // The note is kept out of the code span: an owner renders with
            // backticks of its own (`effect `Store``), and nesting them makes
            // markdown swallow the line.
            let mut note: Option<String> = None;
            let signature = if let Some(f) = fn_decl {
                fn_decl_signature(f, None)
            } else {
                let Some(found) = decl_at(&items.items, &|s: Span| s == site.span) else {
                    continue;
                };
                match found {
                    DeclAt::Member { owner, decl, .. } => {
                        note = Some(format!("of {owner}"));
                        fn_decl_signature(decl, None)
                    }
                    DeclAt::Field { owner, decl, .. } => {
                        note = Some(format!("field of {owner}"));
                        format!("{}: {}", decl.name.name, decl.ty)
                    }
                    DeclAt::Struct(s) => struct_signature(s),
                    DeclAt::Qualifier(q) => qualifier_signature(q),
                    DeclAt::Effect(e) => format!("effect {}", e.name.name),
                    DeclAt::Params(g) => format!("params {}", g.name.name),
                    DeclAt::Handler(h) => handler_signature(h),
                    DeclAt::Type(t) => type_signature(t),
                }
            };
            let module = analysis
                .program
                .files
                .get(site.file)
                .map(|f| f.module.to_string())
                .unwrap_or_default();
            lines.push(match note {
                Some(note) => format!("- `{signature}` — {note} *({module})*"),
                None => format!("- `{signature}` *({module})*"),
            });
        }
        if lines.is_empty() {
            return None;
        }
        lines.sort();
        Some(format!(
            "Also visible under this name:\n{}",
            lines.join("\n")
        ))
    }

    /// Go-to-definition [lsp-definition]: a fn name resolves through
    /// `Checked::fn_refs` (overload-precise) to its declaration's name
    /// span; every other name (struct, effect, handler, qualifier, type
    /// alias, effect member) through `Checked::def_refs`. Definitions in
    /// the embedded std have no on-disk URI and yield `None`.
    fn goto_definition(&self, params: &GotoDefinitionParams) -> Option<GotoDefinitionResponse> {
        let doc = &params.text_document_position_params;
        let path = file_path(&doc.text_document.uri)?;
        let analysis = self.analyze()?;
        let checked = analysis.checked.as_ref()?;

        let file_idx = analysis
            .program
            .files
            .iter()
            .position(|f| !f.is_std && self.root.join(&f.name) == path)?;
        let content = &analysis.program.files[file_idx].content;
        let offset = position_to_offset(content, doc.position);

        // The smallest name span containing the cursor wins, whichever
        // table it comes from.
        let mut best: Option<(Span, DefSite)> = None;
        let consider = |span: Span, site: DefSite, best: &mut Option<(Span, DefSite)>| {
            if span.start <= offset
                && offset < span.end
                && best.is_none_or(|(b, _)| span.len() < b.len())
            {
                *best = Some((span, site));
            }
        };
        for ((file, span), key) in &checked.fn_refs {
            if *file != file_idx {
                continue;
            }
            if let Some(site) = fn_def_site(&analysis.program, *key) {
                consider(*span, site, &mut best);
            }
        }
        // Declared names and fields alike [lsp-definition].
        for ((file, span), site) in checked.def_refs.iter().chain(&checked.field_refs) {
            if *file != file_idx {
                continue;
            }
            consider(*span, *site, &mut best);
        }

        let (_, site) = best?;
        let target = analysis.program.files.get(site.file)?;
        if target.is_std {
            // The embedded std is not on disk; nothing to navigate to.
            return None;
        }
        let target_path = self.root.join(&target.name);
        let uri = match self.doc_uris.get(&target_path) {
            Some(uri) => uri.clone(),
            None => Url::from_file_path(&target_path).ok()?,
        };
        Some(GotoDefinitionResponse::Scalar(Location {
            uri,
            range: span_to_range(&target.content, site.span),
        }))
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

/// The definition site of a fn declaration [lsp-definition]: the file it
/// lives in and the span of its *name*.
fn fn_def_site(program: &Program, key: FnKey) -> Option<DefSite> {
    let module = program.modules.get(key.file)?;
    let Item::Fn(decl) = module.items.get(key.item)? else {
        return None;
    };
    Some(DefSite {
        file: key.file,
        span: decl.name.span,
    })
}

/// Renders the full source-like signature of a fn declaration, with the
/// *effective* deduction list — inferred/validated (`Checked::deductions`)
/// when available, else as declared — and an explicit return type
/// [fn-ref-table]:
///
/// ```text
/// fn add(a: Int, b: Int) -> [] Int
/// fn greet(person: Person) [Console] -> [person] None
/// fn ok<T>(value: T) -> +Ok [] T
/// ```
/// A declaration found at a name span, including the ones nested inside
/// another declaration's body.
enum DeclAt<'a> {
    Struct(&'a salvo_syntax::ast::StructDecl),
    Qualifier(&'a salvo_syntax::ast::QualifierDecl),
    Effect(&'a salvo_syntax::ast::EffectDecl),
    Handler(&'a salvo_syntax::ast::HandlerDecl),
    Type(&'a salvo_syntax::ast::TypeDecl),
    /// [doc-comment] A `params` group — the obligation/spread bundle.
    Params(&'a salvo_syntax::ast::ParamsDecl),
    /// A struct field, handler state field, or qualifier field override.
    Field {
        owner: String,
        /// The owner's own names, in scope for `[symbol]` references
        /// [doc-symbol-ref].
        siblings: Vec<(String, Span)>,
        decl: &'a salvo_syntax::ast::FieldDecl,
    },
    /// A member fn of an effect, handler, or qualifier.
    Member {
        owner: String,
        siblings: Vec<(String, Span)>,
        decl: &'a FnDecl,
    },
}

/// The names a declaration brings into scope for its members' docs:
/// fields, state, constructor parameters and member fns.
fn own_names(item: &Item) -> Vec<(String, Span)> {
    let idents = |names: Vec<(&str, Span)>| -> Vec<(String, Span)> {
        names.into_iter().map(|(n, s)| (n.to_string(), s)).collect()
    };
    match item {
        Item::Struct(s) => idents(
            s.fields
                .iter()
                .map(|f| (f.name.name.as_str(), f.name.span))
                .collect(),
        ),
        Item::Qualifier(q) => idents(
            q.field_overrides
                .iter()
                .map(|f| (f.name.name.as_str(), f.name.span))
                .chain(q.fns.iter().map(|f| (f.name.name.as_str(), f.name.span)))
                .collect(),
        ),
        Item::Effect(e) => idents(
            e.fns
                .iter()
                .map(|f| (f.name.name.as_str(), f.name.span))
                .collect(),
        ),
        Item::Handler(h) => idents(
            h.state
                .iter()
                .map(|f| (f.name.name.as_str(), f.name.span))
                .chain(h.params.iter().map(|p| (p.name.name.as_str(), p.name.span)))
                .chain(h.fns.iter().map(|f| (f.name.name.as_str(), f.name.span)))
                .collect(),
        ),
        _ => Vec::new(),
    }
}

/// The declaration whose *name* span satisfies `hit`. Searches top-level
/// declarations and the ones nested in their bodies — struct fields,
/// handler state, effect/handler/qualifier member fns — so hover and
/// go-to-definition reach all of them.
fn decl_at<'a>(items: &'a [Item], hit: &dyn Fn(Span) -> bool) -> Option<DeclAt<'a>> {
    for item in items {
        match item {
            Item::Struct(s) => {
                if hit(s.name.span) {
                    return Some(DeclAt::Struct(s));
                }
                if let Some(f) = s.fields.iter().find(|f| hit(f.name.span)) {
                    return Some(DeclAt::Field {
                        owner: format!("struct `{}`", s.name.name),
                        siblings: own_names(item),
                        decl: f,
                    });
                }
            }
            Item::Qualifier(q) => {
                if hit(q.name.span) {
                    return Some(DeclAt::Qualifier(q));
                }
                let owner = format!("qualifier `{}`", q.name.name);
                if let Some(f) = q.field_overrides.iter().find(|f| hit(f.name.span)) {
                    return Some(DeclAt::Field {
                        owner: format!("{owner} (field override)"),
                        siblings: own_names(item),
                        decl: f,
                    });
                }
                if let Some(f) = q.fns.iter().find(|f| hit(f.name.span)) {
                    return Some(DeclAt::Member {
                        owner,
                        siblings: own_names(item),
                        decl: f,
                    });
                }
            }
            Item::Effect(e) => {
                if hit(e.name.span) {
                    return Some(DeclAt::Effect(e));
                }
                if let Some(f) = e.fns.iter().find(|f| hit(f.name.span)) {
                    return Some(DeclAt::Member {
                        owner: format!("effect `{}`", e.name.name),
                        siblings: own_names(item),
                        decl: f,
                    });
                }
            }
            Item::Handler(h) => {
                if hit(h.name.span) {
                    return Some(DeclAt::Handler(h));
                }
                let owner = format!("handler `{}`", h.name.name);
                if let Some(f) = h.state.iter().find(|f| hit(f.name.span)) {
                    return Some(DeclAt::Field {
                        owner: format!("{owner} (state)"),
                        siblings: own_names(item),
                        decl: f,
                    });
                }
                if let Some(f) = h.fns.iter().find(|f| hit(f.name.span)) {
                    return Some(DeclAt::Member {
                        owner,
                        siblings: own_names(item),
                        decl: f,
                    });
                }
            }
            Item::Type(t) => {
                if hit(t.name.span) {
                    return Some(DeclAt::Type(t));
                }
            }
            Item::Params(g) => {
                if hit(g.name.span) {
                    return Some(DeclAt::Params(g));
                }
                for f in &g.fns {
                    if hit(f.name.span) {
                        return Some(DeclAt::Member {
                            owner: g.name.name.clone(),
                            siblings: own_names(item),
                            decl: f,
                        });
                    }
                }
            }
            _ => {}
        }
    }
    None
}

/// The name span of the declaration satisfying `hit`, for locating the
/// declaration the cursor sits on.
fn decl_name_span_at(items: &[Item], hit: &dyn Fn(Span) -> bool) -> Option<Span> {
    Some(match decl_at(items, hit)? {
        DeclAt::Struct(s) => s.name.span,
        DeclAt::Qualifier(q) => q.name.span,
        DeclAt::Effect(e) => e.name.span,
        DeclAt::Handler(h) => h.name.span,
        DeclAt::Type(t) => t.name.span,
        DeclAt::Params(g) => g.name.span,
        DeclAt::Field { decl, .. } => decl.name.span,
        DeclAt::Member { decl, .. } => decl.name.span,
    })
}

/// A hover whose contents are markdown [doc-markdown].
fn markdown_hover(value: String, range: Range) -> Hover {
    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value,
        }),
        range: Some(range),
    }
}



/// [doc-comment] The declaration an `import`'s item name refers to, when the
/// cursor sits on it. `import a.b.Point` names `Point`; the last path
/// segment is the item, and the segments before it are the module — so only
/// a declaration in *that* module counts, which is what keeps two structs of
/// the same name apart.
fn import_target(
    items: &[Item],
    hit: &dyn Fn(Span) -> bool,
    analysis: &Analysis,
) -> Option<(Span, DefSite)> {
    for item in items {
        let Item::Import(decl) = item else { continue };
        let (name, module) = decl.path.split_last()?;
        if !hit(name.span) {
            continue;
        }
        let module_path: Vec<&str> = module.iter().map(|m| m.name.as_str()).collect();
        for (file_idx, (file, ast)) in analysis
            .program
            .files
            .iter()
            .zip(&analysis.program.modules)
            .enumerate()
        {
            if file.module.0 != module_path {
                continue;
            }
            if let Some(span) = decl_name_span_at(&ast.items, &|s: Span| {
                span_text(&file.content, s) == name.name
            }) {
                return Some((
                    name.span,
                    DefSite {
                        file: file_idx,
                        span,
                    },
                ));
            }
        }
    }
    None
}

/// The source text a span covers, for matching a declaration by name.
fn span_text(source: &str, span: Span) -> &str {
    source
        .get(span.start as usize..span.end as usize)
        .unwrap_or("")
}

/// [doc-qualifies-body] The *condition* a predicate qualifier holds under,
/// when its `qualifies` is a single `return <expression>` — then the
/// expression alone is shown, inline. Anything longer is hidden and the
/// qualifier's own doc comment is left to explain it (user decision
/// 2026-09-11): a one-line predicate is the rule itself, while a body with
/// branches or locals is an implementation the reader did not ask for.
///
/// `None` for a qualifier with no `qualifies` (a constructive or provenance
/// one — there is no predicate), a body that is not exactly one `return`, or
/// an expression that does not fit on one line.
fn qualifies_section(
    decl: &salvo_syntax::ast::QualifierDecl,
    source: &str,
) -> Option<String> {
    let f = decl.fns.iter().find(|f| f.name.name == "qualifies")?;
    let body = f.body.as_ref()?;
    // [expr-escape] A one-line `qualifies` body is `return <expr>`, which is an
    // expression statement since 2026-09-21.
    let [salvo_syntax::ast::Stmt::Expr(salvo_syntax::ast::Expr::Return {
        value: Some(expr),
        ..
    })] = &body.stmts[..]
    else {
        return None;
    };
    let span = expr.span();
    let text = source
        .get(span.start as usize..span.end as usize)?
        .trim();
    if text.is_empty() || text.contains('\n') {
        return None;
    }
    Some(format!("Holds when `{text}`."))
}

/// A struct's declaration line, without its body: `struct Person canbe Mut`.
fn struct_signature(decl: &salvo_syntax::ast::StructDecl) -> String {
    // [linear-group] [lsp-hover-linear] The `linear` modifier leads the
    // signature, as it does in source: it is the obligation, so a hover that
    // omitted it described the type as if it were ordinary data.
    let mut sig = String::new();
    if decl.linear {
        sig.push_str("linear ");
    }
    sig.push_str(&format!("struct {}", decl.name.name));
    sig.push_str(&generic_list_with_bounds(&decl.generics, &decl.generic_canbe));
    if !decl.auto_qualifiers.is_empty() {
        let quals: Vec<String> = decl.auto_qualifiers.iter().map(|q| q.to_string()).collect();
        sig.push_str(&format!(" canbe {}", quals.join(", ")));
    }
    sig
}

/// [linear-generics] [lsp-hover-linear] The generic list *with its bounds*:
/// `<T canbe linear>`. A bound is the only thing that says a generic function
/// or a container may carry an obligation, so leaving it out of a hover hid the
/// one fact a reader is looking for (user request 2026-09-18).
///
/// A `canbe` entry may name a parameter that is not in `generics` — the parser
/// accepts `fn hold<T canbe linear>(…)` with the bound as the declaration — so
/// those are appended rather than dropped.
fn generic_list_with_bounds(
    generics: &[salvo_syntax::ast::Ident],
    bounds: &[(salvo_syntax::ast::Ident, salvo_syntax::ast::TypeRef)],
) -> String {
    let mut names: Vec<String> = generics.iter().map(|g| g.name.clone()).collect();
    for (g, _) in bounds {
        if !names.contains(&g.name) {
            names.push(g.name.clone());
        }
    }
    if names.is_empty() {
        return String::new();
    }
    let rendered: Vec<String> = names
        .iter()
        .map(|name| {
            let mut out = name.clone();
            for (g, bound) in bounds {
                if &g.name == name {
                    out.push_str(&format!(" canbe {bound}"));
                }
            }
            out
        })
        .collect();
    format!("<{}>", rendered.join(", "))
}

fn generic_list(generics: &[salvo_syntax::ast::Ident]) -> String {
    if generics.is_empty() {
        return String::new();
    }
    let names: Vec<&str> = generics.iter().map(|g| g.name.as_str()).collect();
    format!("<{}>", names.join(", "))
}

/// `provenance qualifier Authenticated of Request with Old`.
fn qualifier_signature(decl: &salvo_syntax::ast::QualifierDecl) -> String {
    let mut sig = String::new();
    if decl.subject == QualSubject::Provenance {
        sig.push_str("provenance ");
    }
    sig.push_str(&format!(
        "qualifier {}{} of {}",
        decl.name.name,
        generic_list(&decl.generics),
        decl.of
    ));
    if !decl.with.is_empty() {
        let with: Vec<String> = decl.with.iter().map(|w| w.to_string()).collect();
        sig.push_str(&format!(" with {}", with.join(", ")));
    }
    sig
}

/// `effect Random<T>` plus its member signatures, which are the whole
/// point of hovering an effect.
fn effect_signature(decl: &salvo_syntax::ast::EffectDecl) -> String {
    let mut sig = format!("effect {}{}", decl.name.name, generic_list(&decl.generics));
    if !decl.fns.is_empty() {
        sig.push_str(" {");
        for f in &decl.fns {
            let params: Vec<String> = f
                .params
                .iter()
                .map(|p| format!("{}: {}", p.name.name, p.ty))
                .collect();
            let ret = match &f.return_type {
                Some(t) => format!(" -> {t}"),
                None => String::new(),
            };
            sig.push_str(&format!("\n    fn {}({}){}", f.name.name, params.join(", "), ret));
        }
        sig.push_str("\n}");
    }
    sig
}

/// [doc-comment] `params Yield<It, T> { fn next(it: Mut It) -> … }` — the
/// members are the point of a group, so they are always shown.
fn params_signature(decl: &salvo_syntax::ast::ParamsDecl) -> String {
    let mut sig = format!("params {}{}", decl.name.name, generic_list(&decl.generics));
    if !decl.fns.is_empty() {
        sig.push_str(" {");
        for f in &decl.fns {
            let params: Vec<String> = f
                .params
                .iter()
                .map(|p| format!("{}: {}", p.name.name, p.ty))
                .collect();
            let ret = match &f.return_type {
                Some(t) => format!(" -> {t}"),
                None => String::new(),
            };
            sig.push_str(&format!(
                "\n    fn {}({}){}",
                f.name.name,
                params.join(", "),
                ret
            ));
        }
        sig.push_str("\n}");
    }
    sig
}

/// `handler CyclicRandom<T>(values: List<T>) of Random<T>` — several faces
/// comma-separated, as the declaration writes them [effect-handler-multi].
fn handler_signature(decl: &salvo_syntax::ast::HandlerDecl) -> String {
    let params: Vec<String> = decl
        .params
        .iter()
        .map(|p| format!("{}: {}", p.name.name, p.ty))
        .collect();
    let params = if params.is_empty() {
        String::new()
    } else {
        format!("({})", params.join(", "))
    };
    let faces: Vec<String> = decl.of.iter().map(|of| of.to_string()).collect();
    format!(
        "handler {}{}{} of {}",
        decl.name.name,
        generic_list(&decl.generics),
        params,
        faces.join(", ")
    )
}

/// `type Number = Int | Long`, or `intrinsic type List<T> canbe Mut`.
fn type_signature(decl: &salvo_syntax::ast::TypeDecl) -> String {
    let mut sig = String::new();
    // [linear-group] [lsp-hover-linear] `linear intrinsic type Reply<T>`, in
    // the source order of its modifiers.
    if decl.linear {
        sig.push_str("linear ");
    }
    if decl.intrinsic {
        sig.push_str("intrinsic ");
    }
    sig.push_str(&format!(
        "type {}{}",
        decl.name.name,
        generic_list_with_bounds(&decl.generics, &decl.generic_canbe)
    ));
    if !decl.auto_qualifiers.is_empty() {
        let quals: Vec<String> = decl.auto_qualifiers.iter().map(|q| q.to_string()).collect();
        sig.push_str(&format!(" canbe {}", quals.join(", ")));
    }
    if let Some(alias) = &decl.alias {
        sig.push_str(&format!(" = {alias}"));
    }
    sig
}

fn fn_signature(program: &Program, checked: &Checked, key: FnKey) -> Option<String> {
    let module = program.modules.get(key.file)?;
    let Item::Fn(decl) = module.items.get(key.item)? else {
        return None;
    };
    Some(fn_decl_signature(decl, checked.deductions.get(&key).map(|d| d.as_slice())))
}

/// A fn's source-like signature. `inferred` is the whole-program deduction
/// list when there is one — effect and handler members have no `FnKey`, so
/// they render their declared list only [doc-comment].
fn fn_decl_signature(decl: &FnDecl, inferred: Option<&[ParamDeduction]>) -> String {
    let mut sig = String::from("fn ");
    sig.push_str(&decl.name.name);
    // [linear-generics] [lsp-hover-linear] With bounds: `<T canbe linear>` is
    // what tells a reader this function may be handed an obligation.
    sig.push_str(&generic_list_with_bounds(&decl.generics, &decl.generic_canbe));
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

    match &decl.return_type {
        Some(ty) => sig.push_str(&format!(" -> {ty}")),
        None => sig.push_str(" -> None"),
    }
    if let Some(cref) = &decl.constructs {
        sig.push_str(&format!(" as {cref}"));
    }
    // [deduce-syntax] The *effective* contract, in the source spelling: the
    // whole-program pass's list when there is one (written entries and the
    // inferred rest, indistinguishable here — which is the point), else the
    // declared clause alone (effect and handler members have no `FnKey`).
    let clause = match inferred {
        Some(deductions) => render_deductions(deductions, decl),
        None => decl
            .deductions
            .as_ref()
            .map(|list| render_declared(list))
            .unwrap_or_default(),
    };
    if !clause.is_empty() {
        sig.push_str(&format!(" => {clause}"));
    }
    // Fn-typed parameters with a written group render theirs too.
    for p in &decl.params {
        if let salvo_syntax::ast::Type::Fn { deductions: Some(list), .. } = &p.ty {
            if !list.is_empty() {
                sig.push_str(&format!(" =>[{}] {}", p.name.name, render_declared(list)));
            }
        }
    }
    sig
}

/// Renders an effective deduction clause [deduce-syntax]: `!p` for a moved
/// parameter, bare for keep-all, `p: A B` for an exhaustive set (`p: None`
/// when it is empty), `p: -A` for a delta, and `proj(a, b)` for the
/// parameters the result holds borrows of [proj-infer]. Empty when there is
/// nothing to say (every parameter kept whole, nothing lent).
fn render_deductions(
    deductions: &[ParamDeduction],
    decl: &salvo_syntax::ast::FnDecl,
) -> String {
    // [copy-scalar-free] A Copy scalar's fate is nothing to deduce, so the
    // hover leaves it out, as the clause may.
    let scalar = |name: &str| -> bool {
        decl.params.iter().any(|p| {
            p.name.name == name
                && matches!(
                    &p.ty,
                    salvo_syntax::ast::Type::Named { qualifiers, base }
                        if qualifiers.is_empty()
                            && base.args.is_empty()
                            && matches!(
                                base.name.name.as_str(),
                                "Int" | "Long" | "Float" | "Double" | "Bool" | "Char" | "Byte"
                            )
                )
        })
    };
    let mut entries: Vec<String> = deductions
        .iter()
        .filter(|d| !scalar(&d.param))
        .filter_map(|d| {
            if !d.kept {
                return Some(format!("!{}", d.param));
            }
            match &d.effect {
                QualEffect::KeepAll => None,
                QualEffect::Exhaustive(keep) if keep.is_empty() => Some(format!("{}: None", d.param)),
                QualEffect::Exhaustive(keep) => Some(format!("{}: {}", d.param, keep.join(" "))),
                QualEffect::Remove(dropped) => Some(format!(
                    "{}: {}",
                    d.param,
                    dropped
                        .iter()
                        .map(|q| format!("-{q}"))
                        .collect::<Vec<_>>()
                        .join(" ")
                )),
            }
        })
        .collect();
    let lent: Vec<&str> = deductions
        .iter()
        .filter(|d| d.lent)
        .map(|d| d.param.as_str())
        .collect();
    if !lent.is_empty() {
        entries.push(format!("proj({})", lent.join(", ")));
    }
    entries.join(", ")
}

/// Renders a written clause as written [deduce-syntax].
fn render_declared(list: &[salvo_syntax::ast::Deduction]) -> String {
    use salvo_syntax::ast::{DeductionKind, DeductionTarget};
    let names = |items: &[salvo_syntax::ast::TypeRef]| -> Vec<String> {
        items.iter().map(|q| q.to_string()).collect()
    };
    let target = |d: &salvo_syntax::ast::Deduction| -> String {
        match &d.target {
            DeductionTarget::Param { name, path } => {
                let mut s = name.name.clone();
                for f in path {
                    s.push('.');
                    s.push_str(&f.name);
                }
                s
            }
            DeductionTarget::Result { path } => {
                let mut s = String::new();
                for f in path {
                    s.push('.');
                    s.push_str(&f.name);
                }
                s
            }
            DeductionTarget::Opaque => String::new(),
        }
    };
    let entries: Vec<String> = list
        .iter()
        // A bare kept entry says what the default says: nothing to show.
        .filter(|d| !matches!(d.kind, DeductionKind::KeepAll))
        .map(|d| {
            let t = target(d);
            match &d.kind {
                DeductionKind::KeepAll => t,
                DeductionKind::Moved => format!("!{t}"),
                DeductionKind::Deferred => format!("defer {t}"),
                DeductionKind::Exhaustive { quals, reapplied }
                    if quals.is_empty() && reapplied.is_empty() =>
                {
                    format!("{t}: None")
                }
                // [deduce-reapply] The `+` is part of the contract a reader
                // needs: it says this function *establishes* the claim rather
                // than passing one along.
                DeductionKind::Exhaustive { quals, reapplied } => {
                    let mut shown = names(quals);
                    shown.extend(names(reapplied).iter().map(|q| format!("+{q}")));
                    format!("{t}: {}", shown.join(" "))
                }
                DeductionKind::Remove(items) => format!(
                    "{t}: {}",
                    names(items).iter().map(|q| format!("-{q}")).collect::<Vec<_>>().join(" ")
                ),
                DeductionKind::Proj(sources) => {
                    let srcs: Vec<&str> = sources.iter().map(|s| s.name.as_str()).collect();
                    if t.is_empty() {
                        format!("proj({})", srcs.join(", "))
                    } else {
                        format!("{t}: proj({})", srcs.join(", "))
                    }
                }
            }
        })
        .collect();
    entries.join(", ")
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
