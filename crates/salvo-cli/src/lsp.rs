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
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

use lsp_server::{Connection, Message, Notification, Request, Response};
use lsp_types::notification::{
    DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, DidSaveTextDocument,
    Notification as _, PublishDiagnostics,
};
use lsp_types::request::{
    CodeActionRequest, Completion, GotoDefinition, HoverRequest, Request as _, SignatureHelpRequest,
};
use lsp_types::{
    CodeAction, CodeActionKind, CodeActionOrCommand, CodeActionParams,
    CodeActionProviderCapability, CompletionItem, CompletionItemKind, CompletionOptions,
    CompletionParams, CompletionResponse, Diagnostic, DiagnosticSeverity,
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    DidSaveTextDocumentParams, GotoDefinitionParams, GotoDefinitionResponse, Hover,
    HoverContents, HoverParams, HoverProviderCapability, InitializeParams,
    Location, MarkupContent, MarkupKind, OneOf, ParameterInformation, ParameterLabel, Position,
    PublishDiagnosticsParams, Range, ServerCapabilities, SignatureHelp, SignatureHelpOptions,
    SignatureHelpParams, SignatureInformation, TextDocumentSyncCapability, TextDocumentSyncKind,
    TextEdit, Url, WorkspaceEdit,
};

use salvo_core::{
    Checked, DefSite, FileDiagnostic, FnKey, ModulePath, ParamDeduction, Program, QualEffect, Ty,
};
use salvo_syntax::ast::{FnDecl, Item, QualSubject};
use salvo_syntax::diag::Severity;
use salvo_syntax::Span;

use crate::analysis::{analyze_for_document_cached, Analysis};
use crate::docs;

/// [lsp-debounce] How long the loop waits for the edit stream to go quiet
/// before analysing. Long enough that a run of keystrokes collapses into one
/// analysis, short enough to feel immediate once typing pauses. A fast typist
/// hits ~100ms between keys, so 120ms coalesces a burst without a visible lag
/// after the last keystroke.
const DEBOUNCE: Duration = Duration::from_millis(120);

/// [lsp-trace] Whether `SALVO_LSP_TRACE` is set in the environment: when it
/// is, the server logs every message it dispatches, every diagnostics
/// publish, and every debounce wake to stderr with a monotonic timestamp.
/// Computed once — reading the environment per message would itself be work
/// on the hot path we are trying to observe. The point is diagnosing a
/// runaway (100% CPU "while idle"): if the log scrolls with no edits, the
/// loop is being fed, and the line that repeats names the driver.
fn trace_enabled() -> bool {
    use std::sync::OnceLock;
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("SALVO_LSP_TRACE").is_some())
}

/// [lsp-trace] Milliseconds since the process started, for ordering trace
/// lines without pulling in a clock dependency.
fn trace_ms() -> u128 {
    use std::sync::OnceLock;
    use std::time::Instant;
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_millis()
}

/// [lsp-slow-dump] The wall-time past which `analyze_for` dumps the buffer it
/// was analysing (see `Server::dump_slow_pass`). Default 2000ms — well above a
/// normal whole-program pass (~0.5s at Salvo-project scale), so a dump means a
/// genuine runaway, not routine work. `SALVO_LSP_SLOW_MS` overrides it (`0`
/// dumps every pass, for testing). Read once; the env is not re-read per pass.
fn slow_dump_threshold() -> Duration {
    use std::sync::OnceLock;
    static MS: OnceLock<u64> = OnceLock::new();
    let ms = *MS.get_or_init(|| {
        std::env::var("SALVO_LSP_SLOW_MS")
            .ok()
            .and_then(|v| v.trim().parse::<u64>().ok())
            .unwrap_or(2000)
    });
    Duration::from_millis(ms)
}

/// [lsp-slow-dump] The single fixed pair of paths the slow-pass dump writes,
/// in the OS temp dir and **overwritten every time** so nothing accumulates:
/// the analysed source (`.sv`) and a metadata header (`.txt`).
/// `SALVO_LSP_SLOW_DIR` overrides the directory.
fn slow_dump_paths() -> (PathBuf, PathBuf) {
    let dir = std::env::var_os("SALVO_LSP_SLOW_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    (dir.join("salvo-lsp-slow.sv"), dir.join("salvo-lsp-slow.txt"))
}


/// A macro so the formatting cost is paid only when tracing is on.
macro_rules! lsp_trace {
    ($($arg:tt)*) => {
        if trace_enabled() {
            eprintln!("salvo lsp [{:>8}ms] {}", trace_ms(), format_args!($($arg)*));
        }
    };
}

/// [lsp-watchdog] Exit the process when our parent (the editor that spawned
/// us) dies, independent of stdin. The LSP transport already exits cleanly on
/// a clean stdin EOF, but a force-killed editor can leave the stdio pipe in a
/// state that never signals EOF — and then a server that is somehow busy
/// (the 100%-CPU report) spins forever, reparented to `init`, with nothing
/// left to tell it to stop. This watchdog is that backstop: a cheap poll of
/// the parent PID, and `exit(0)` the moment it changes.
///
/// Unix only (reparenting to PID 1 is the signal); a no-op elsewhere. Costs
/// one `getppid()` per second on a sleeping thread — negligible, and only
/// while the server runs.
#[cfg(unix)]
fn spawn_parent_death_watchdog() {
    extern "C" {
        fn getppid() -> i32;
    }
    // SAFETY: `getppid` is a trivial, always-safe POSIX call.
    let original = unsafe { getppid() };
    // A server started not from a parent process (`original <= 1`) has no
    // parent to watch — don't risk a spurious exit.
    if original <= 1 {
        return;
    }
    std::thread::Builder::new()
        .name("salvo-lsp-parent-watchdog".to_owned())
        .spawn(move || loop {
            std::thread::sleep(Duration::from_secs(1));
            // SAFETY: as above.
            let now = unsafe { getppid() };
            if now != original {
                // The editor is gone (reparented to init, or a new parent):
                // nothing we do matters now, so stop burning CPU.
                lsp_trace!("parent {original} is gone (now {now}); exiting");
                cleanup_std_copies();
                std::process::exit(0);
            }
        })
        .ok();
}

#[cfg(not(unix))]
fn spawn_parent_death_watchdog() {}

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
    // [lsp-watchdog] Backstop against a server outliving its editor and
    // spinning (see the fn docs). Started before anything blocks.
    spawn_parent_death_watchdog();
    // [lsp-std-source] Sweep std-copy directories left by servers that exited
    // without cleanup (a hard kill). Each server's copies are per-pid, so a
    // directory whose pid is no longer a live process is safe to remove.
    sweep_dead_std_copies();
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
        // [lsp-completion] Functions in scope, and after a `.` the ones the
        // receiver can be the first argument of.
        completion_provider: Some(CompletionOptions {
            // `.` opens dot-field/dot-call completion; `[` opens the
            // doc-reference list inside a `//` comment's `[…]`, so a reference
            // auto-suggests as it is typed, the way code completion does
            // (user request 2026-10-14).
            trigger_characters: Some(vec![".".to_string(), "[".to_string()]),
            ..Default::default()
        }),
        // [lsp-signature-help] The parameter list of the call being written,
        // one signature per overload, re-triggered on `(` and `,`.
        signature_help_provider: Some(SignatureHelpOptions {
            trigger_characters: Some(vec!["(".to_string(), ",".to_string()]),
            retrigger_characters: Some(vec![",".to_string()]),
            work_done_progress_options: Default::default(),
        }),
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
        dirty: false,
        parse_cache: std::cell::RefCell::new(crate::analysis::ParseCache::new()),
        std_cache: std::cell::RefCell::new(None),
    };
    server.main_loop()?;
    drop(server);
    // The writer thread only terminates once its channel sender — owned
    // by the connection — is dropped; joining first would deadlock.
    drop(connection);
    io_threads.join()?;
    // [lsp-std-source] Remove the read-only std copies this server extracted
    // for navigation, so they do not accumulate in the temp dir across runs.
    cleanup_std_copies();
    Ok(())
}

/// [lsp-std-source] Delete the temp directory holding this build's extracted
/// std copies (`embedded_std_copy`). The copies are read-only, so clear each
/// file's read-only flag is unnecessary — `remove_dir_all` removes them
/// regardless on the platforms we target. Best-effort: a failure (another
/// server still using it, a permission quirk) is ignored, since the OS clears
/// its temp dir eventually anyway.
fn cleanup_std_copies() {
    let dir = crate::analysis::std_copy_root();
    // The extracted files are read-only; on some platforms `remove_dir_all`
    // refuses a read-only file, so clear the flag first (best-effort).
    if let Ok(entries) = walk_files(&dir) {
        for f in entries {
            if let Ok(meta) = std::fs::metadata(&f) {
                let mut perms = meta.permissions();
                #[allow(clippy::permissions_set_readonly_false)]
                perms.set_readonly(false);
                let _ = std::fs::set_permissions(&f, perms);
            }
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// Every regular file under `dir`, recursively (for clearing read-only flags
/// before removal). `Err` only when `dir` cannot be read.
fn walk_files(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&d) else { continue };
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                stack.push(p);
            } else {
                out.push(p);
            }
        }
    }
    Ok(out)
}

/// [lsp-std-source] Remove std-copy directories (`salvo-std-<version>-<pid>`)
/// whose process is no longer alive — leftovers from a server that was hard-
/// killed before its own `cleanup_std_copies` ran. Best-effort; a directory
/// whose liveness cannot be determined is left alone. Unix checks liveness
/// with `kill(pid, 0)`; elsewhere it is a no-op (the per-pid dir still gets
/// cleaned on this server's own clean shutdown).
fn sweep_dead_std_copies() {
    let prefix = format!("salvo-std-{}-", env!("CARGO_PKG_VERSION"));
    let temp = std::env::temp_dir();
    let Ok(entries) = std::fs::read_dir(&temp) else { return };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        let Some(pid_str) = name.strip_prefix(&prefix) else { continue };
        let Ok(pid) = pid_str.parse::<i32>() else { continue };
        if pid == std::process::id() as i32 {
            continue;
        }
        if pid_is_dead(pid) {
            let dir = entry.path();
            if let Ok(files) = walk_files(&dir) {
                for f in files {
                    if let Ok(meta) = std::fs::metadata(&f) {
                        let mut perms = meta.permissions();
                        #[allow(clippy::permissions_set_readonly_false)]
                        perms.set_readonly(false);
                        let _ = std::fs::set_permissions(&f, perms);
                    }
                }
            }
            let _ = std::fs::remove_dir_all(&dir);
        }
    }
}

/// Whether `pid` is definitely not a running process. Conservative: only
/// `true` when we can confirm the pid is gone, so a live or unknown process's
/// directory is never swept.
#[cfg(unix)]
fn pid_is_dead(pid: i32) -> bool {
    extern "C" {
        fn kill(pid: i32, sig: i32) -> i32;
    }
    // SAFETY: `kill` with signal 0 performs only the existence/permission
    // check, sending no signal. errno is not read; the return value suffices.
    let r = unsafe { kill(pid, 0) };
    if r == 0 {
        return false; // alive (and we may signal it)
    }
    // r == -1: either ESRCH (no such process -> dead) or EPERM (alive, not
    // ours). Distinguish via errno; EPERM means alive.
    std::io::Error::last_os_error().raw_os_error() == Some(/* ESRCH */ 3)
}

#[cfg(not(unix))]
fn pid_is_dead(_pid: i32) -> bool {
    false
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
    /// [lsp-coalesce] A document changed since diagnostics were last
    /// published: they are published once the queue is drained.
    dirty: bool,
    /// [lsp-parse-cache] Parsed modules kept across analyses, so an unchanged
    /// file is not re-parsed on every keystroke. A `RefCell` because the
    /// request handlers analyse through `&self`; the server loop is
    /// single-threaded, so there is never contention.
    parse_cache: std::cell::RefCell<crate::analysis::ParseCache>,
    /// [std-cache] The embedded std checked on its own, built on the first
    /// analysis and reused by every later one whose program starts with that
    /// std unshadowed. A `RefCell` for the same reason as `parse_cache`.
    std_cache: std::cell::RefCell<Option<salvo_core::StdCache>>,
}

impl Server<'_> {
    /// [lsp-coalesce] Edits only update the overlay; diagnostics are
    /// published once nothing is waiting. Each keystroke used to re-analyse the
    /// whole program before the next message was read, so a completion asked
    /// for after five keystrokes waited behind five analyses.
    ///
    /// [lsp-debounce] Publishing is also debounced: after the queue drains
    /// with edits pending, the loop waits for a short quiet period before
    /// analysing, and any message arriving inside that window is handled and
    /// resets the wait. A burst of keystrokes therefore produces **one**
    /// analysis, of the latest buffer — the whole-program check is the
    /// expensive step [cli-lsp], and running it once per burst instead of once
    /// per keystroke is what keeps diagnostics from trailing several edits
    /// behind. Requests (hover, completion, go-to-definition) are unaffected:
    /// they analyse on demand and always see the current overlay.
    fn main_loop(&mut self) -> Result<(), Box<dyn Error + Sync + Send>> {
        while let Ok(msg) = self.connection.receiver.recv() {
            if self.dispatch(msg)? {
                return Ok(());
            }
            // Drain whatever is already queued, then — if an edit is pending —
            // wait out a quiet period, absorbing any late-arriving messages so
            // only the final buffer state is analysed [lsp-debounce].
            loop {
                while let Ok(msg) = self.connection.receiver.try_recv() {
                    if self.dispatch(msg)? {
                        return Ok(());
                    }
                }
                if !self.dirty {
                    break;
                }
                // A message inside the window keeps the burst open; silence (or
                // disconnect — the outer `recv` exits next) ends it and we
                // publish. `recv_timeout`'s error does not distinguish the two,
                // but publishing on a disconnect is harmless.
                match self.connection.receiver.recv_timeout(DEBOUNCE) {
                    Ok(msg) => {
                        lsp_trace!("debounce wake: message arrived, burst continues");
                        if self.dispatch(msg)? {
                            return Ok(());
                        }
                    }
                    Err(_) => {
                        lsp_trace!("debounce quiet: publishing");
                        break;
                    }
                }
            }
            if self.dirty {
                self.dirty = false;
                self.publish_diagnostics()?;
            }
        }
        Ok(())
    }

    /// Handles one message. Returns `true` when it was a shutdown request and
    /// the loop should exit.
    fn dispatch(&mut self, msg: Message) -> Result<bool, Box<dyn Error + Sync + Send>> {
        // [lsp-trace] Name the message, so a flood of them (the 100%-CPU
        // "while idle" case) shows exactly what the loop is being fed.
        if trace_enabled() {
            let what = match &msg {
                Message::Request(r) => format!("request {}", r.method),
                Message::Notification(n) => format!("notification {}", n.method),
                Message::Response(_) => "response".to_string(),
            };
            lsp_trace!("dispatch {what} (dirty={})", self.dirty);
        }
        match msg {
            Message::Request(req) => {
                if self.connection.handle_shutdown(&req)? {
                    return Ok(true);
                }
                self.handle_request(req)?;
            }
            Message::Notification(note) => self.handle_notification(note)?,
            Message::Response(_) => {}
        }
        Ok(false)
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
            Completion::METHOD => {
                let params: CompletionParams = serde_json::from_value(req.params)?;
                let result = self.completion(&params);
                self.respond(Response::new_ok(req.id, result))?;
            }
            SignatureHelpRequest::METHOD => {
                let params: SignatureHelpParams = serde_json::from_value(req.params)?;
                let result = self.signature_help(&params);
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
                    self.dirty = true;
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
                    self.dirty = true;
                }
            }
            DidCloseTextDocument::METHOD => {
                let params: DidCloseTextDocumentParams = serde_json::from_value(note.params)?;
                if let Some(path) = file_path(&params.text_document.uri) {
                    // Back to the on-disk contents.
                    self.overlay.remove(&path);
                    self.doc_uris.remove(&path);
                    self.dirty = true;
                }
            }
            DidSaveTextDocument::METHOD => {
                let _params: DidSaveTextDocumentParams = serde_json::from_value(note.params)?;
                self.dirty = true;
            }
            _ => {}
        }
        Ok(())
    }

    /// [std-cache] The server's std cache, checked on first use. The returned
    /// borrow is shared, so hold it only across one analysis.
    fn std_cache(&self) -> std::cell::Ref<'_, salvo_core::StdCache> {
        {
            let mut std_cache = self.std_cache.borrow_mut();
            if std_cache.is_none() {
                *std_cache = Some(crate::analysis::build_std_cache(""));
            }
        }
        std::cell::Ref::map(self.std_cache.borrow(), |c| c.as_ref().expect("built above"))
    }

    /// [manifest-discovery] The analysis a document belongs to: its
    /// project's source root, or the workspace root when no manifest is
    /// above it.
    fn analyze_for(&self, path: &Path) -> Option<Analysis> {
        // [lsp-std-source] A copy of the embedded std the user navigated into:
        // analyse std as its own program so hover and go-to-definition work
        // *inside* std (its references resolve to other std files, which
        // `site_location` routes back to their read-only copies). Built on
        // demand — navigating within std is rare, and the check is sub-second.
        if is_embedded_std_copy(path) {
            return Some(crate::analysis::analyze_std());
        }
        let std_cache = self.std_cache();
        // [lsp-slow-dump] Time the pass so a runaway (the 100%-CPU report) can
        // be caught in the act: an analysis slower than the threshold dumps the
        // exact buffer it was given to a single fixed file, overwritten each
        // time, so the transient editor state that no saved file reproduces is
        // captured without filling the disk. Timing is one `Instant`; the dump
        // only runs past the threshold.
        let started = std::time::Instant::now();
        let result = analyze_for_document_cached(
            path,
            &self.root,
            &self.overlay,
            &mut self.parse_cache.borrow_mut(),
            &std_cache,
        );
        let elapsed = started.elapsed();
        if elapsed >= slow_dump_threshold() {
            self.dump_slow_pass(path, elapsed);
        }
        match result {
            Ok(analysis) => Some(analysis),
            Err(err) => {
                eprintln!("salvo lsp: {err}");
                None
            }
        }
    }

    /// [lsp-slow-dump] Write the buffer that `analyze_for` was analysing when
    /// a pass ran long to a **single fixed path, overwritten every time**, so
    /// a slow pass can be reproduced from the exact (often unsaved) source that
    /// caused it. One `.sv` plus a `.txt` header (the path, elapsed ms, which
    /// other buffers were open); never accumulates. Any I/O error is ignored —
    /// this is a diagnostic, never a reason to disturb the server.
    fn dump_slow_pass(&self, path: &Path, elapsed: Duration) {
        let (sv_path, meta_path) = slow_dump_paths();
        // The source actually analysed: the open-editor overlay if there is
        // one (the transient state), else the on-disk file.
        let source = self
            .overlay
            .get(path)
            .cloned()
            .or_else(|| std::fs::read_to_string(path).ok())
            .unwrap_or_default();
        let ms = elapsed.as_millis();
        lsp_trace!("slow pass: {ms}ms analysing {}; dumping to {}", path.display(), sv_path.display());
        eprintln!(
            "salvo lsp: analysis took {ms}ms for {} (threshold {}ms); \
             buffer dumped to {} [lsp-slow-dump]",
            path.display(),
            slow_dump_threshold().as_millis(),
            sv_path.display()
        );
        let mut others: Vec<String> = self
            .overlay
            .keys()
            .filter(|p| p.as_path() != path)
            .map(|p| p.display().to_string())
            .collect();
        others.sort();
        let meta = format!(
            "salvo lsp slow-pass dump\n\
             document: {}\n\
             elapsed:  {ms}ms (threshold {}ms)\n\
             overlay:  {} open buffer(s){}\n\
             source dumped to: {}\n\n\
             This file is overwritten on every slow pass. To reproduce:\n\
             copy the .sv into a scratch project and run `salvo analyze`.\n",
            path.display(),
            slow_dump_threshold().as_millis(),
            self.overlay.len(),
            if others.is_empty() { String::new() } else { format!("\nother open buffers:\n  {}", others.join("\n  ")) },
            sv_path.display(),
        );
        let _ = std::fs::write(&sv_path, source);
        let _ = std::fs::write(&meta_path, meta);
    }

    /// Re-analyzes every project an open document belongs to and publishes
    /// diagnostics per file URI, clearing files that no longer have any
    /// [cli-lsp] [manifest-discovery].
    fn publish_diagnostics(&mut self) -> Result<(), Box<dyn Error + Sync + Send>> {
        lsp_trace!("publish_diagnostics: analysing {} open doc(s)", self.doc_uris.len());
        // Every open document gets a publish — an explicit empty list
        // tells the client the file was re-checked and is clean, and
        // clears anything it showed before.
        let mut by_uri: HashMap<Url, Vec<Diagnostic>> = self
            .doc_uris
            .values()
            .map(|uri| (uri.clone(), Vec::new()))
            .collect();
        // One analysis per project root: documents under one manifest share
        // it, and a workspace holding several projects analyses each as its
        // own program.
        let mut done: HashSet<PathBuf> = HashSet::new();
        let docs: Vec<PathBuf> = if self.doc_uris.is_empty() {
            vec![self.root.clone()]
        } else {
            self.doc_uris.keys().cloned().collect()
        };
        for doc in docs {
            let Some(analysis) = self.analyze_for(&doc) else { continue };
            if !done.insert(analysis.root.clone()) {
                continue;
            }
            if analysis.project.is_none() && doc.is_file() && self.workspace_has_manifests() {
                // No manifest above this document, in a workspace that has
                // projects: the fallback is visible rather than silent, since
                // it is what makes a repository of several trees check as one
                // program. A workspace with no manifest anywhere is one
                // project, as it always was, and gets no note.
                if let Some(uri) = self.doc_uris.get(&doc) {
                    by_uri.entry(uri.clone()).or_default().push(Diagnostic {
                        range: Range::new(Position::new(0, 0), Position::new(0, 0)),
                        severity: Some(DiagnosticSeverity::INFORMATION),
                        message: format!(
                            "no `{}` above this file: analysed under the workspace root `{}` \
                             [manifest-discovery]",
                            salvo_core::MANIFEST_FILE,
                            self.root.display()
                        ),
                        source: Some("salvo".to_string()),
                        ..Diagnostic::default()
                    });
                }
            }
            for diag in &analysis.diagnostics {
                let file = &analysis.program.files[diag.file];
                if file.is_std && !file.is_shadow {
                    // Embedded std has no on-disk URI; it should be clean.
                    // [std-shadow] A shadowing file is on disk — the standard
                    // library being developed — so its diagnostics go to the
                    // editor like any file's.
                    eprintln!("salvo lsp: diagnostic in std: {}", diag.message);
                    continue;
                }
                // Open documents keep the URI the client opened them under
                // (clients compare URIs exactly).
                let (content, span) = (&file.content, diag.span);
                let path = analysis.root.join(&file.name);
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
                    .push(to_lsp_diagnostic(diag, content, span));
            }
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

    /// [manifest-discovery] Whether any `salvo.toml` lies under the
    /// workspace root (a few levels down), which is what makes a file with
    /// none above it worth a note.
    fn workspace_has_manifests(&self) -> bool {
        fn walk(dir: &Path, depth: u32) -> bool {
            if dir.join(salvo_core::MANIFEST_FILE).is_file() {
                return true;
            }
            if depth == 0 {
                return false;
            }
            let Ok(entries) = std::fs::read_dir(dir) else { return false };
            entries.flatten().any(|e| {
                let p = e.path();
                p.is_dir()
                    && !p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with('.'))
                    && p.file_name().and_then(|n| n.to_str()) != Some("target")
                    && walk(&p, depth - 1)
            })
        }
        walk(&self.root, 3)
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
        let analysis = self.analyze_for(&path)?;
        let checked = analysis.checked.as_ref()?;

        let view = doc_view(&analysis, &path)?;
        let offset = view.offset(doc.position);
        self.hover_at(&analysis, checked, &view, offset)
    }

    /// The hover at `offset` of the analysed file `view` is about.
    fn hover_at(
        &self,
        analysis: &Analysis,
        checked: &Checked,
        view: &DocView<'_>,
        offset: u32,
    ) -> Option<Hover> {
        let file_idx = view.file;
        let link = self.doc_linker(analysis);

        // [comptime-fields] Inside a `comptime fn` body the checker has no
        // tables (the body is stamped, never checked as itself), so the
        // expansion's own record of each comptime name's type answers first:
        // `T is Struct`, `field: Field`, `field.type: Type`, `T.fields:
        // List<Field>`.
        let mut best_comp: Option<&salvo_core::CompHover> = None;
        for h in &analysis.comptime_hovers {
            if h.file == file_idx
                && h.span.start <= offset
                && offset < h.span.end
                && best_comp.is_none_or(|b| h.span.len() < b.span.len())
            {
                best_comp = Some(h);
            }
        }
        if let Some(h) = best_comp {
            // [doc-module] A `by` site names a module too: its docs follow the
            // comptime fn's.
            let module = h
                .module
                .as_ref()
                .and_then(|m| self.module_section(&analysis, m, file_idx, &link));
            let mut text = h.text.clone();
            if let Some(section) = module {
                text.push_str("\n\n---\n\n");
                text.push_str(&section);
            }
            return Some(markdown_hover(text, view.range(h.span)));
        }

        // [doc-module] A module *name*: an `@module` selector the checker
        // resolved, or the module part of an `import` path. Its hover is the
        // module's own documentation — the file's leading comment — and where
        // the module lives.
        if let Some((span, module)) = module_at(&analysis, checked, file_idx, offset) {
            let module_file = module_file(&analysis, &module)?;
            let ast = analysis.program.modules.get(module_file)?;
            let source = &analysis.program.files.get(module_file)?.content;
            let scope = docs::DocScope {
                locals: Vec::new(),
                file: module_file,
                source,
                modules: &analysis.program.modules,
                link: &link,
            };
            let docs = docs::render(&ast.docs, &scope);
            let origin = self.module_origin(&analysis, module_file, file_idx);
            let exports = module_exports_section(ast, module_file, file_idx);
            return Some(markdown_hover(
                docs::hover_markdown(&format!("module {module}"), &[docs, exports, origin]),
                view.range(span),
            ));
        }

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
                // [lsp-hover-iter-fn] An `iter fn` reads as itself: no docs
                // or overload set of the generated halves (the step's name,
                // `next`, is shared by every iterator).
                let iter_fn = decl.is_some_and(|d| iter_fn_signature(&analysis.program.modules[key.file], d).is_some());
                let docs = if iter_fn {
                    decl.and_then(|d| iter_fn_minter(&analysis.program.modules[key.file], d)).and_then(|m| {
                        let source = &analysis.program.files.get(key.file)?.content;
                        let scope = docs::fn_scope(m, key.file, source, &analysis.program.modules, &link);
                        docs::render(&m.docs, &scope)
                    })
                } else {
                    docs
                };
                // [lsp-hover-overloads] The rest of the overload set, so the
                // resolved signature reads as *a choice* rather than as the
                // only candidate.
                let others = decl.filter(|_| !iter_fn).and_then(|d| {
                    self.overload_section(
                        &analysis,
                        file_idx,
                        &d.name.name,
                        DefSite {
                            file: key.file,
                            span: d.name.span,
                        },
                        &link,
                    )
                });
                return Some(markdown_hover(
                    docs::hover_markdown(&signature, &[docs, refinements, origin, others]),
                    view.range(span),
                ));
            }
        }

        // Any other declared name: the declaration under the cursor, or
        // the declaration a name refers to [lsp-definition].
        if let Some(hover) = self.declaration_hover(&analysis, &checked, &view, offset, &link) {
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
                    let pos = view.range(r.bind_span).start;
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
                view.range(span),
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
            view.range(span),
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
        // [manifest-deps] A dependency's module says which dependency.
        if let Some(dep) = &file.dependency {
            return Some(format!("From `{module}` — dependency `{dep}`."));
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

    /// [doc-module] Where a *module* lives, as a hover section: the standard
    /// library, a dependency (named), the file being edited, or another file
    /// of the program (its path).
    fn module_origin(&self, analysis: &Analysis, module_file: usize, here: usize) -> Option<String> {
        let file = analysis.program.files.get(module_file)?;
        if file.is_std && !file.is_shadow {
            return Some("The standard library.".to_string());
        }
        if let Some(dep) = &file.dependency {
            return Some(format!("Dependency `{dep}`, in `{}`.", file.name));
        }
        if module_file == here {
            return Some("This file.".to_string());
        }
        Some(format!(
            "In `{}`{}.",
            file.name,
            if file.is_std { " — the standard library" } else { "" }
        ))
    }

    /// [doc-module] A module's documentation as a hover section of its own,
    /// for a site whose hover is about something else (a `by` site): the
    /// module named, then its docs when it has any.
    fn module_section(
        &self,
        analysis: &Analysis,
        module: &ModulePath,
        here: usize,
        link: &dyn Fn(usize, Span) -> Option<String>,
    ) -> Option<String> {
        let module_file = module_file(analysis, module)?;
        let ast = analysis.program.modules.get(module_file)?;
        let source = &analysis.program.files.get(module_file)?.content;
        let scope = docs::DocScope {
            locals: Vec::new(),
            file: module_file,
            source,
            modules: &analysis.program.modules,
            link,
        };
        let mut out = format!("Module `{module}`");
        match self.module_origin(analysis, module_file, here) {
            Some(origin) => out.push_str(&format!(" — {}", lowercase_first(&origin))),
            None => out.push('.'),
        }
        if let Some(docs) = docs::render(&ast.docs, &scope) {
            out.push_str("\n\n");
            out.push_str(&docs);
        }
        Some(out)
    }

    /// A markdown link target for a definition site, or `None` for the
    /// embedded std (not on disk, so nothing to link to).
    fn doc_linker<'a>(
        &'a self,
        analysis: &'a Analysis,
    ) -> impl Fn(usize, Span) -> Option<String> + 'a {
        move |file: usize, span: Span| {
            let (uri, range) = self.site_location(analysis, file, span)?;
            let pos = range.start;
            Some(format!("{uri}#L{},{}", pos.line + 1, pos.character + 1))
        }
    }

    /// Where a span of the analysis is, as a document: the file's own URI —
    /// the one an open document was opened under. A file of the embedded std
    /// is written to a read-only copy on disk first [lsp-std-source], so
    /// navigation and doc links reach it too.
    ///
    /// A file's `name` resolves against the analysis's source root
    /// [manifest-discovery], which is the workspace root only when no
    /// manifest was found.
    fn site_location(&self, analysis: &Analysis, file: usize, span: Span) -> Option<(Url, Range)> {
        let target = analysis.program.files.get(file)?;
        if target.is_std && !target.is_shadow {
            let path = embedded_std_copy(&target.name, &target.content)?;
            let uri = Url::from_file_path(&path).ok()?;
            return Some((uri, span_to_range(&target.content, span)));
        }
        let (content, span) = (&target.content, span);
        let path = analysis.root.join(&target.name);
        let uri = match self.doc_uris.get(&path) {
            Some(uri) => uri.clone(),
            None => Url::from_file_path(&path).ok()?,
        };
        Some((uri, span_to_range(content, span)))
    }

    /// Hover for a declared name: the declaration the cursor sits on, or
    /// the one a reference points at [lsp-definition]. Structs add a
    /// **Fields** section [doc-struct-fields].
    fn declaration_hover(
        &self,
        analysis: &Analysis,
        checked: &Checked,
        view: &DocView<'_>,
        offset: u32,
        link: &dyn Fn(usize, Span) -> Option<String>,
    ) -> Option<Hover> {
        let file_idx = view.file;
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
        // [lsp-hover-ref-name] A reference hover must land on a declaration
        // whose name is the token under the cursor. A built-in auto-qualifier
        // (`Mut`) has no declaration, but a spurious def-ref can still point
        // its span at some `canbe Mut` type (`Set`) — which showed `type
        // Set<T>` when hovering the `Mut` of `Mut Map`. When the hovered text
        // is a bare identifier that disagrees with the resolved declaration's
        // name, the resolution is wrong: fall through rather than show it.
        // Only applied to recorded references (`site`), never the
        // hover-your-own-declaration fallback, and only for a plain-identifier
        // token so an alias or dotted path is unaffected.
        if site.is_some() {
            if let Some(dname) = &decl_name {
                let hovered = view
                    .content
                    .get(span.start as usize..span.end as usize)
                    .unwrap_or("")
                    .trim();
                let plain_ident = !hovered.is_empty()
                    && hovered
                        .chars()
                        .all(|c| c.is_alphanumeric() || c == '_');
                if plain_ident && hovered != dname {
                    // A dot-name declaration (`Ns.Name`) is referenced by its
                    // last segment; that is a match, not a mismatch.
                    let last_seg = dname.rsplit('.').next().unwrap_or(dname);
                    if hovered != last_seg {
                        return None;
                    }
                }
            }
        }
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
            // A field: its own declaration line, who declares it, its docs
            // [doc-struct-fields], and — when its type is a struct — that
            // struct's own field list [lsp-hover-field-struct].
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
                // The owner note, then the referenced struct's fields when
                // there is one — one section, so both reach the hover.
                let mut extra = format!("Field of {owner}.");
                // [state-infer] A nested-field section needs the written type
                // to resolve the referenced struct; an inferred field has none.
                if let Some(fty) = &decl.ty {
                    if let Some(fields) = struct_field_section(analysis, file_idx, fty, link) {
                        extra.push_str("\n\n");
                        extra.push_str(&fields);
                    }
                }
                (
                    format!("{}: {}{}", decl.name.name, field_ty_display(&decl.ty), default),
                    docs::render(&decl.docs, &scope),
                    Some(extra),
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
        let others = decl_name
            .and_then(|name| self.overload_section(analysis, file_idx, &name, target, link));
        Some(markdown_hover(
            docs::hover_markdown(&signature, &[body, fields, origin, others]),
            view.range(span),
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
        link: &dyn Fn(usize, Span) -> Option<String>,
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
            let source = analysis.program.files.get(site.file).map(|f| f.content.as_str()).unwrap_or("");
            // A **top-level fn** is not a `DeclAt` — `decl_at` covers the
            // declarations that hover through `declaration_hover`, and a fn
            // resolves through `fn_refs` instead — so it is matched here by
            // its name span before falling back to the declaration kinds.
            let fn_decl = items.items.iter().find_map(|item| match item {
                Item::Fn(f) if f.name.span == site.span => Some(f),
                _ => None,
            });
            // [lsp-hover-overloads] [iter-fn] A generated iterator-struct
            // `next` is not a user-visible overload: its struct
            // (`__Iter_<fn>_…`) is compiler-written and unspellable, so listing
            // its `next(__p: Mut __Iter_…)` beside the real ones only leaks the
            // backend. Skip any overload whose first parameter's base type is a
            // compiler-generated name (leading `_`).
            if let Some(f) = fn_decl {
                let generated_recv = f
                    .params
                    .first()
                    .and_then(|p| match &p.ty {
                        salvo_syntax::ast::Type::Named { base, .. } => Some(base.name.name.as_str()),
                        _ => None,
                    })
                    .is_some_and(|n| n.starts_with('_'));
                if generated_recv {
                    continue;
                }
            }
            // The note is kept out of the code span: an owner renders with
            // backticks of its own (`effect `Store``), and nesting them makes
            // markdown swallow the line.
            let mut note: Option<String> = None;
            // [lsp-hover-overloads] A sibling's own doc comment, rendered the
            // same way the resolved overload's is — a reader comparing
            // candidates needs more than a bare signature to choose between
            // them.
            let mut docs: Option<String> = None;
            let signature = if let Some(f) = fn_decl {
                let scope = docs::fn_scope(f, site.file, source, &analysis.program.modules, link);
                docs = docs::render(&f.docs, &scope);
                fn_decl_signature(f, None)
            } else {
                let Some(found) = decl_at(&items.items, &|s: Span| s == site.span) else {
                    continue;
                };
                match found {
                    DeclAt::Member { owner, decl, .. } => {
                        note = Some(format!("of {owner}"));
                        let scope = docs::fn_scope(decl, site.file, source, &analysis.program.modules, link);
                        docs = docs::render(&decl.docs, &scope);
                        fn_decl_signature(decl, None)
                    }
                    DeclAt::Field { owner, decl, .. } => {
                        note = Some(format!("field of {owner}"));
                        format!("{}: {}", decl.name.name, field_ty_display(&decl.ty))
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
            let heading = match note {
                Some(note) => format!("- `{signature}` — {note} *({module})*"),
                None => format!("- `{signature}` *({module})*"),
            };
            // A doc comment renders as an indented continuation of the
            // bullet, so it reads as *that* overload's description rather
            // than a new list item.
            lines.push(match docs {
                Some(docs) => {
                    let indented: Vec<String> = docs.lines().map(|l| format!("  {l}")).collect();
                    format!("{heading}\n{}", indented.join("\n"))
                }
                None => heading,
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
        let analysis = self.analyze_for(&path)?;
        let checked = analysis.checked.as_ref()?;

        let view = doc_view(&analysis, &path)?;
        let file_idx = view.file;
        let offset = view.offset(doc.position);

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

        let (_, site) = match best {
            Some(found) => found,
            // [doc-module] A module name jumps to the top of its file.
            None => {
                let (_, module) = module_at(&analysis, checked, file_idx, offset)?;
                let file = module_file(&analysis, &module)?;
                (
                    Span::new(0, 0),
                    DefSite {
                        file,
                        span: Span::new(0, 0),
                    },
                )
            }
        };
        // The embedded std is not on disk; nothing to navigate to.
        // [std-shadow] A shadowing file is, and navigates like any.
        let (uri, range) = self.site_location(&analysis, site.file, site.span)?;
        Some(GotoDefinitionResponse::Scalar(Location { uri, range }))
    }

    /// [lsp-completion] Completion at a position: every function the file can
    /// call; after `receiver.`, only those whose first parameter the receiver
    /// fills [fn-dot] — matched by the receiver's type name — then those
    /// taking any type, ranked after.
    ///
    /// The buffer is analysed with the word being typed and its dot removed,
    /// so a half-written call still parses and the receiver has a type.
    fn completion(&self, params: &CompletionParams) -> Option<CompletionResponse> {
        let doc = &params.text_document_position;
        let path = file_path(&doc.text_document.uri)?;
        let text = match self.overlay.get(&path) {
            Some(t) => t.clone(),
            None => std::fs::read_to_string(&path).ok()?,
        };
        let cursor = position_to_offset(&text, doc.position) as usize;
        // [lsp-completion] On an `import` line: modules, then a module's items.
        let line_start = text[..cursor.min(text.len())].rfind('\n').map(|i| i + 1).unwrap_or(0);
        let line = &text[line_start..cursor.min(text.len())];
        let trimmed = line.trim_start();
        // [lsp-completion] [doc-symbol-ref] Inside a `//` comment, ordinary
        // code completion does not apply. A `[` reference links to code
        // (`[Notice]`, `[read_notice]`), so when the cursor sits inside an
        // unclosed `[…]` on the comment line, offer the symbols a
        // `[symbol]` reference resolves to; anywhere else in a comment,
        // offer nothing (prose is not code).
        if let Some(rel) = comment_start(line) {
            let after = &line[rel..];
            let in_ref = after.rfind('[').is_some_and(|o| !after[o..].contains(']'));
            if !in_ref {
                return Some(CompletionResponse::Array(Vec::new()));
            }
            let analysis = analyze_for_document_cached(
                &path,
                &self.root,
                &self.overlay,
                &mut self.parse_cache.borrow_mut(),
                &self.std_cache(),
            )
            .ok()?;
            let view = doc_view(&analysis, &path)?;
            return Some(CompletionResponse::Array(doc_ref_completions(&analysis, view.file, cursor as u32)));
        }
        let import_path = trimmed
            .strip_prefix("export import ")
            .or_else(|| trimmed.strip_prefix("import "));
        if let Some(written) = import_path {
            if written.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.') {
                let analysis = analyze_for_document_cached(
                    &path,
                    &self.root,
                    &self.overlay,
                    &mut self.parse_cache.borrow_mut(),
                    &self.std_cache(),
                )
                .ok()?;
                return Some(CompletionResponse::Array(import_completions(&analysis, written)));
            }
        }
        let bytes = text.as_bytes();
        let mut word = cursor.min(bytes.len());
        while word > 0 && (bytes[word - 1].is_ascii_alphanumeric() || bytes[word - 1] == b'_') {
            word -= 1;
        }
        let dot = word > 0 && bytes[word - 1] == b'.';
        let patched = if dot {
            format!("{}{}", &text[..word - 1], &text[cursor.min(text.len())..])
        } else {
            format!("{}{}", &text[..word], &text[cursor.min(text.len())..])
        };
        let mut overlay = self.overlay.clone();
        overlay.insert(path.clone(), patched);
        let analysis = analyze_for_document_cached(
            &path,
            &self.root,
            &overlay,
            &mut self.parse_cache.borrow_mut(),
            &self.std_cache(),
        )
        .ok()?;
        let checked = analysis.checked.as_ref()?;
        let view = doc_view(&analysis, &path)?;
        let fns = analysis.completions.get(view.file)?;
        // [effect-member-completion] The effects available at the cursor: the
        // expanded effect set of the innermost fn/handler-member body that
        // contains the cursor (smallest such span wins). Names are the bare
        // effect names, module key stripped, to match an owning effect. Empty
        // when the cursor is in no effectful body — then no member is offered.
        let available_effects: HashSet<String> = {
            let cursor_off = word as u32;
            checked
                .fn_body_effects
                .iter()
                .filter(|(file, span, _)| *file == view.file && span.start <= cursor_off && cursor_off <= span.end)
                .min_by_key(|(_, span, _)| span.len())
                .map(|(_, _, tys)| {
                    tys.iter()
                        .filter_map(|ty| match ty {
                            Ty::Named { name, .. } => Some(name.split('§').next().unwrap_or(name).to_string()),
                            _ => None,
                        })
                        .collect()
                })
                .unwrap_or_default()
        };
        // The receiver: the smallest typed expression ending at the dot.
        let receiver: Option<String> = if dot {
            let end = (word - 1) as u32;
            checked
                .expr_ty
                .iter()
                .filter(|((file, span), ty)| *file == view.file && span.end == end && !matches!(ty, Ty::Unknown))
                .min_by_key(|((_, span), _)| span.len())
                .and_then(|(_, ty)| receiver_name(ty))
        } else {
            None
        };
        let mut items: Vec<CompletionItem> = Vec::new();
        // [lsp-completion] After a dot on a struct: its fields, first.
        if let (true, Some(r)) = (dot, &receiver) {
            for f in struct_fields(&analysis.program, r, view.file) {
                items.push(CompletionItem {
                    label: f.0.clone(),
                    kind: Some(CompletionItemKind::FIELD),
                    detail: Some(f.1),
                    sort_text: Some(format!("0{}", f.0)),
                    ..Default::default()
                });
            }
        }
        // [lsp-completion] Where a type is written (after `:`, `->`, `<` or
        // `,` inside `<…>`), types come first; elsewhere after the fns.
        let before = text[..word].trim_end();
        let type_position = before.ends_with(':') || before.ends_with("->") || before.ends_with('<') || before.ends_with('|');
        if !dot {
            let type_rank = if type_position { "0" } else { "2" };
            for (name, kind) in analysis.type_completions.get(view.file).into_iter().flatten() {
                use crate::analysis::TypeKind;
                let k = match kind {
                    TypeKind::Struct => CompletionItemKind::STRUCT,
                    TypeKind::Effect => CompletionItemKind::INTERFACE,
                    TypeKind::Handler => CompletionItemKind::CLASS,
                    TypeKind::Qualifier => CompletionItemKind::TYPE_PARAMETER,
                    TypeKind::Type => CompletionItemKind::CLASS,
                };
                items.push(CompletionItem {
                    label: name.clone(),
                    kind: Some(k),
                    sort_text: Some(format!("{type_rank}{name}")),
                    ..Default::default()
                });
            }
        }
        // [lsp-completion] Local bindings in scope — parameters and `let`/
        // binding locals of the enclosing fn, declared before the cursor.
        // Offered where a value is written (not after a `.`, not where a
        // type is expected), and ranked first there, since the thing a
        // reader reaches for is usually a name already in hand.
        if !dot && !type_position {
            let cursor_off = word as u32;
            let mut seen_locals: HashSet<String> = HashSet::new();
            for local in checked.local_decls.get(&view.file).into_iter().flatten() {
                let in_scope = local.fn_body.start <= cursor_off
                    && cursor_off <= local.fn_body.end
                    // A parameter or state field is in scope for the whole
                    // body; an ordinary binding only after its declaration.
                    && (local.always_in_scope || local.span.start < cursor_off);
                if !in_scope || !seen_locals.insert(local.name.clone()) {
                    continue;
                }
                items.push(CompletionItem {
                    label: local.name.clone(),
                    kind: Some(CompletionItemKind::VARIABLE),
                    detail: Some(local.ty.to_string()),
                    sort_text: Some(format!("0{}", local.name)),
                    ..Default::default()
                });
            }
        }
        // [lsp-completion] [assert] [unreachable] The two assertion builtins,
        // offered where a statement/value is written (not after a `.`, not in
        // a type position). They match by prefix like any name (`unr` →
        // `unreachable!`, `ass` → `assert!`), and a bare `!` just typed
        // offers exactly these two. The `!` is part of the inserted text, not
        // the match word, so completing replaces the typed prefix with the
        // full `name!`.
        if !dot && !type_position {
            let bang = word > 0 && bytes[word - 1] == b'!';
            for name in ["assert", "unreachable", "todo"] {
                items.push(CompletionItem {
                    label: format!("{name}!"),
                    kind: Some(CompletionItemKind::KEYWORD),
                    detail: Some(match name {
                        "assert" => "assert!(cond, message)".to_string(),
                        "todo" => "todo!(message?) — warns, not implemented".to_string(),
                        _ => "unreachable!(message?)".to_string(),
                    }),
                    // A bare `!` ranks these first; otherwise after names.
                    sort_text: Some(format!("{}{name}", if bang { "0" } else { "3" })),
                    ..Default::default()
                });
            }
        }
        // [lsp-completion] One entry per function *name* when typing a bare
        // prefix (`ne` → one `next`, however many overloads it has): the
        // overloads differ only in signature, which the reader picks among
        // after choosing the name. After a dot, where the receiver already
        // narrows the set, each distinct signature stays its own entry.
        let overloads: HashMap<&str, usize> = {
            let mut m: HashMap<&str, usize> = HashMap::new();
            for f in fns {
                *m.entry(f.name.as_str()).or_default() += 1;
            }
            m
        };
        let mut seen: HashSet<(String, String)> = HashSet::new();
        let mut seen_name: HashSet<String> = HashSet::new();
        for f in fns {
            // [effect-member-completion] An effect member is offered only
            // where its owning effect is available at the cursor.
            if let Some(eff) = &f.effect {
                if !available_effects.contains(eff) {
                    continue;
                }
            }
            let rank = if dot {
                match (&receiver, &f.receiver) {
                    (Some(r), Some(p)) if r == p => "1",
                    (_, _) if f.generic_receiver => "2",
                    _ => continue,
                }
            } else if type_position {
                "1"
            } else {
                "0"
            };
            let detail = match (f.key, &f.detail) {
                (Some(key), _) => fn_signature(&analysis.program, checked, key).unwrap_or_default(),
                (None, Some(d)) => d.clone(),
                (None, None) => String::new(),
            };
            if !dot {
                // Collapse overloads of one name to a single entry; note the
                // count so a reader knows to expand it.
                if !seen_name.insert(f.name.clone()) {
                    continue;
                }
                let n = overloads.get(f.name.as_str()).copied().unwrap_or(1);
                let detail = if n > 1 { format!("{n} overloads") } else { detail };
                items.push(CompletionItem {
                    label: f.name.clone(),
                    kind: Some(CompletionItemKind::FUNCTION),
                    detail: Some(detail),
                    sort_text: Some(format!("{rank}{}", f.name)),
                    ..Default::default()
                });
                continue;
            }
            if !seen.insert((f.name.clone(), detail.clone())) {
                continue;
            }
            items.push(CompletionItem {
                label: f.name.clone(),
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some(detail),
                sort_text: Some(format!("{rank}{}", f.name)),
                ..Default::default()
            });
        }
        Some(CompletionResponse::Array(items))
    }

    /// [lsp-signature-help] The parameter list of the call being written.
    /// Walks left from the cursor to the enclosing unmatched `(`, reads the
    /// callee name before it, and returns one signature per overload of that
    /// name — with the active parameter (the top-level comma count before the
    /// cursor) highlighted.
    fn signature_help(&self, params: &SignatureHelpParams) -> Option<SignatureHelp> {
        let doc = &params.text_document_position_params;
        let path = file_path(&doc.text_document.uri)?;
        let text = match self.overlay.get(&path) {
            Some(t) => t.clone(),
            None => std::fs::read_to_string(&path).ok()?,
        };
        let cursor = position_to_offset(&text, doc.position) as usize;
        let (name_end, active) = enclosing_call(&text, cursor.min(text.len()))?;
        // The callee name immediately before the `(`.
        let bytes = text.as_bytes();
        let mut start = name_end;
        while start > 0 && (bytes[start - 1].is_ascii_alphanumeric() || bytes[start - 1] == b'_') {
            start -= 1;
        }
        if start == name_end {
            return None;
        }
        let name = &text[start..name_end];
        let analysis = analyze_for_document_cached(
            &path,
            &self.root,
            &self.overlay,
            &mut self.parse_cache.borrow_mut(),
            &self.std_cache(),
        )
        .ok()?;
        let checked = analysis.checked.as_ref()?;
        let view = doc_view(&analysis, &path)?;
        let fns = analysis.completions.get(view.file)?;
        let mut signatures: Vec<SignatureInformation> = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();
        for f in fns.iter().filter(|f| f.name == name) {
            let label = match (f.key, &f.detail) {
                (Some(key), _) => match fn_signature(&analysis.program, checked, key) {
                    Some(l) => l,
                    None => continue,
                },
                (None, Some(d)) => d.clone(),
                (None, None) => continue,
            };
            if !seen.insert(label.clone()) {
                continue;
            }
            let parameters = signature_parameters(&label);
            signatures.push(SignatureInformation {
                label,
                documentation: None,
                parameters: Some(parameters),
                active_parameter: Some(active as u32),
            });
        }
        if signatures.is_empty() {
            return None;
        }
        Some(SignatureHelp {
            signatures,
            active_signature: Some(0),
            active_parameter: Some(active as u32),
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
        let uri = params.text_document.uri.clone();
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
                let changes = HashMap::from([(uri.clone(), vec![edit])]);
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
            // [lsp-effect-fix] A missing effect: add it to the enclosing
            // fn's effect list.
            if let Some(effect) = missing_effect(&diag.message) {
                let at = position_to_offset(&content, diag.range.start) as usize;
                if let Some((offset, text)) = effect_list_edit(&content, at, &effect) {
                    let pos = offset_to_position(&content, offset as u32);
                    let edit = TextEdit { range: Range::new(pos, pos), new_text: text };
                    let changes = HashMap::from([(uri.clone(), vec![edit])]);
                    actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                        title: format!("Add `{effect}` to the function's effects"),
                        kind: Some(CodeActionKind::QUICKFIX),
                        diagnostics: Some(vec![diag.clone()]),
                        edit: Some(WorkspaceEdit { changes: Some(changes), ..Default::default() }),
                        ..Default::default()
                    }));
                }
            }
        }
        actions
    }

    fn respond(&self, response: Response) -> Result<(), Box<dyn Error + Sync + Send>> {
        self.connection.sender.send(Message::Response(response))?;
        Ok(())
    }
}

/// [state-infer] A field type for display: the written type, or `_` for an
/// inferred state field whose type is whatever its initializer resolves to.
fn field_ty_display(ty: &Option<salvo_syntax::ast::Type>) -> String {
    match ty {
        Some(t) => t.to_string(),
        None => "_".to_string(),
    }
}

/// [lsp-hover-field-struct] When a field's type is (possibly through `Mut`,
/// `?`, or a module key) a **struct**, the struct's own `docs::field_section`
/// — so hovering a struct-typed field shows that struct's shape, as hovering
/// the struct name does. `None` for a non-struct field type, or a struct the
/// viewer may not see inside [struct-opaque]. The heading names the struct so
/// a reader can tell these fields apart from the field's own owner note.
fn struct_field_section(    analysis: &Analysis,
    viewer: usize,
    ty: &salvo_syntax::ast::Type,
    link: &dyn Fn(usize, Span) -> Option<String>,
) -> Option<String> {
    let name = type_struct_name(ty)?;
    for (idx, (file, module)) in analysis
        .program
        .files
        .iter()
        .zip(&analysis.program.modules)
        .enumerate()
    {
        for item in &module.items {
            if let salvo_syntax::ast::Item::Struct(s) = item {
                if s.name.name == name {
                    // [struct-opaque] An opaque struct's fields are hidden
                    // outside its module (and its test annex), exactly as
                    // completion hides them.
                    if s.opaque && !sees_inside(&analysis.program, &file.module, viewer) {
                        return None;
                    }
                    let source = file.content.as_str();
                    let scope = docs::struct_scope(s, idx, source, &analysis.program.modules, link);
                    let section = docs::field_section(&s.fields, &scope)?;
                    // Re-head the reused **Fields** section with the struct's
                    // name: without it, two field lists (the owner's and this
                    // one's) would read as one.
                    return Some(section.replacen(
                        "**Fields**",
                        &format!("**`{name}` fields**"),
                        1,
                    ));
                }
            }
        }
    }
    None
}

/// [lsp-hover-field-struct] The base **type name** of a field type when it is
/// a plain named type — through `Mut`/qualifiers and `?` (a `Str?` field is a
/// `Str`, an `Address?` field is an `Address`). `None` for arrays, tuples, fn
/// types and unions, whose shape a single struct's field list would not
/// describe. The module key (`§…`) is stripped, as `receiver_name` does.
fn type_struct_name(ty: &salvo_syntax::ast::Type) -> Option<String> {
    use salvo_syntax::ast::Type;
    match ty {
        Type::Named { base, .. } => {
            Some(base.name.name.split('§').next().unwrap_or(&base.name.name).to_string())
        }
        Type::Nullable { inner, .. } => type_struct_name(inner),
        _ => None,
    }
}

/// [lsp-completion] The fields of the struct named `name` (its first
/// declaration), as `(name, type)`.
fn struct_fields(program: &salvo_core::Program, name: &str, viewer: usize) -> Vec<(String, String)> {
    for (file, module) in program.files.iter().zip(&program.modules) {
        for item in &module.items {
            if let salvo_syntax::ast::Item::Struct(s) = item {
                if s.name.name == name {
                    // [struct-opaque] Outside its module (and that module's
                    // test annex) an opaque struct's fields are not offered.
                    if s.opaque && !sees_inside(program, &file.module, viewer) {
                        return Vec::new();
                    }
                    return s.fields.iter().map(|f| (f.name.name.clone(), field_ty_display(&f.ty))).collect();
                }
            }
        }
    }
    Vec::new()
}

/// [struct-opaque] Whether the file `viewer` is the declaring module or its
/// `*.test.sv` annex (module `<m>.test`).
fn sees_inside(program: &salvo_core::Program, declaring: &salvo_core::ModulePath, viewer: usize) -> bool {
    let Some(here) = program.files.get(viewer) else { return false };
    here.module == *declaring
        || (here.is_test
            && here.module.0.len() == declaring.0.len() + 1
            && here.module.0.last().is_some_and(|l| l == "test")
            && here.module.0[..declaring.0.len()] == declaring.0[..])
}

/// [lsp-completion] What may follow `import <written>`: the next segment of
/// every module path extending `written`'s module part, and — when that part
/// names a module — its exported items.
fn import_completions(analysis: &Analysis, written: &str) -> Vec<CompletionItem> {
    use salvo_syntax::ast::Item;
    let (parent, _) = written.rsplit_once('.').unwrap_or(("", written));
    let parent: Vec<&str> = if parent.is_empty() { Vec::new() } else { parent.split('.').collect() };
    let mut out: Vec<CompletionItem> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for (file, ast) in analysis.program.files.iter().zip(&analysis.program.modules) {
        if file.is_test {
            continue;
        }
        let m = &file.module.0;
        if m.len() > parent.len() && m.iter().zip(&parent).all(|(a, b)| a == b) {
            let seg = &m[parent.len()];
            if seen.insert(format!("m:{seg}")) {
                out.push(CompletionItem {
                    label: seg.clone(),
                    kind: Some(CompletionItemKind::MODULE),
                    sort_text: Some(format!("0{seg}")),
                    ..Default::default()
                });
            }
        }
        if !parent.is_empty() && m.len() == parent.len() && m.iter().zip(&parent).all(|(a, b)| a == b) {
            for item in &ast.items {
                let (name, kind) = match item {
                    Item::Fn(f) if f.exported => (f.name.name.clone(), CompletionItemKind::FUNCTION),
                    Item::Struct(s) if s.exported => (s.name.name.clone(), CompletionItemKind::STRUCT),
                    Item::Effect(e) if e.exported => (e.name.name.clone(), CompletionItemKind::INTERFACE),
                    Item::Handler(h) if h.exported => (h.name.name.clone(), CompletionItemKind::CLASS),
                    Item::Type(t) if t.exported => (t.name.name.clone(), CompletionItemKind::CLASS),
                    Item::Qualifier(q) if q.exported => (q.name.name.clone(), CompletionItemKind::TYPE_PARAMETER),
                    _ => continue,
                };
                if !name.starts_with("__") && seen.insert(format!("i:{name}")) {
                    out.push(CompletionItem {
                        label: name.clone(),
                        kind: Some(kind),
                        sort_text: Some(format!("1{name}")),
                        ..Default::default()
                    });
                }
            }
        }
    }
    out
}

/// [lsp-signature-help] From `cursor`, walk left to the `(` of the call the
/// cursor is inside, skipping balanced `()`/`[]` and string literals. Returns
/// the byte offset just after that `(` (where the callee name ends) and the
/// active parameter index — the number of top-level commas between the `(`
/// and the cursor. `None` when the cursor is not inside a call's argument
/// list, or a statement boundary (`{`, `}`, `;`) is reached first.
fn enclosing_call(text: &str, cursor: usize) -> Option<(usize, usize)> {
    let bytes = text.as_bytes();
    let mut depth = 0i32; // nested () inside the arg list
    let mut brackets = 0i32; // nested [] inside the arg list
    let mut commas = 0usize;
    let mut i = cursor;
    // Whether offset `j` sits inside a string literal on its own line. Cheap
    // per-line scan: a call rarely spans many lines, and this only runs on a
    // signature-help request.
    let in_string = |j: usize| -> bool {
        let line_start = text[..j].rfind('\n').map(|p| p + 1).unwrap_or(0);
        let mut s = false;
        let mut k = line_start;
        let lb = text.as_bytes();
        while k < j {
            match lb[k] {
                b'\\' if s => {
                    k += 2;
                    continue;
                }
                b'"' => s = !s,
                _ => {}
            }
            k += 1;
        }
        s
    };
    while i > 0 {
        i -= 1;
        if in_string(i) {
            continue;
        }
        match bytes[i] {
            b')' => depth += 1,
            b']' => brackets += 1,
            b'(' => {
                if depth == 0 {
                    // The opening paren of the enclosing call: commas counted
                    // at this level are the active parameter index.
                    return Some((i, commas));
                }
                depth -= 1;
            }
            b'[' => {
                if brackets == 0 {
                    return None; // inside an index/collection literal, not a call
                }
                brackets -= 1;
            }
            b',' if depth == 0 && brackets == 0 => commas += 1,
            b'{' | b'}' | b';' if depth == 0 && brackets == 0 => return None,
            _ => {}
        }
    }
    None
}

/// [lsp-signature-help] The parameter spans of a signature label
/// (`fn f(a: Int, b: Str) -> …`): each parameter rendered as its own
/// `ParameterInformation` with an offset range into the label, so the client
/// can highlight the active one. The split is on top-level commas between the
/// first `(` and its matching `)`.
fn signature_parameters(label: &str) -> Vec<ParameterInformation> {
    let bytes = label.as_bytes();
    let Some(open) = label.find('(') else { return Vec::new() };
    let mut depth = 0i32;
    let mut params = Vec::new();
    let mut seg_start = open + 1;
    let mut i = open;
    while i < bytes.len() {
        match bytes[i] {
            b'(' | b'[' | b'<' => depth += 1,
            b')' | b']' | b'>' => {
                depth -= 1;
                if depth == 0 {
                    // The closing `)` of the parameter list.
                    push_param(label, seg_start, i, &mut params);
                    return params;
                }
            }
            b',' if depth == 1 => {
                push_param(label, seg_start, i, &mut params);
                seg_start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    params
}

/// Pushes the parameter spanning `[start, end)` of `label` (trimmed of
/// surrounding spaces) as a `ParameterInformation` with its label offsets.
fn push_param(label: &str, start: usize, end: usize, out: &mut Vec<ParameterInformation>) {
    let raw = &label[start..end];
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return;
    }
    let lead = raw.len() - raw.trim_start().len();
    let begin = (start + lead) as u32;
    let finish = begin + trimmed.len() as u32;
    out.push(ParameterInformation {
        label: ParameterLabel::LabelOffsets([begin, finish]),
        documentation: None,
    });
}

/// [lsp-completion] The byte offset within `line` where a `//` comment
/// begins, or `None` when the cursor's line prefix is not in a comment. A
/// `//` inside a string literal does not start a comment, so string state is
/// tracked across the line prefix.
fn comment_start(line: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    let mut in_str = false;
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' if in_str => {
                i += 2;
                continue;
            }
            b'"' => in_str = !in_str,
            b'/' if !in_str && i + 1 < bytes.len() && bytes[i + 1] == b'/' => return Some(i),
            _ => {}
        }
        i += 1;
    }
    None
}

/// [lsp-completion] [doc-symbol-ref] The symbols a `[symbol]` reference in a
/// comment may point to: the program's declared types, qualifiers, effects,
/// handlers, and functions — the same set [`docs::DocScope::resolve`]
/// accepts — **plus** the names in scope where the comment sits (a fn's or
/// handler member's parameters, a handler's or `iter fn`'s state fields, and
/// the body's bindings). A doc-ref resolves against those locals first
/// ([`docs::DocScope`] carries them), so a comment in a handler member can
/// link its state field (`[notice_by_id]`) and completion offers it. One
/// entry per name.
fn doc_ref_completions(analysis: &Analysis, view_file: usize, cursor: u32) -> Vec<CompletionItem> {
    use crate::analysis::TypeKind;
    let mut out: Vec<CompletionItem> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    // In-scope locals at the comment's position, offered first — the same
    // bindings code completion offers (parameters, state fields, body
    // `let`s), so a `[…]` ref auto-suggests "things there, as in normal code".
    if let Some(checked) = analysis.checked.as_ref() {
        for local in checked.local_decls.get(&view_file).into_iter().flatten() {
            let in_scope = local.fn_body.start <= cursor
                && cursor <= local.fn_body.end
                && (local.always_in_scope || local.span.start < cursor);
            if in_scope && seen.insert(local.name.clone()) {
                out.push(CompletionItem {
                    label: local.name.clone(),
                    kind: Some(CompletionItemKind::VARIABLE),
                    detail: Some(local.ty.to_string()),
                    sort_text: Some(format!("0{}", local.name)),
                    ..Default::default()
                });
            }
        }
    }
    for (name, kind) in analysis.type_completions.get(view_file).into_iter().flatten() {
        if seen.insert(name.clone()) {
            let k = match kind {
                TypeKind::Struct => CompletionItemKind::STRUCT,
                TypeKind::Effect => CompletionItemKind::INTERFACE,
                TypeKind::Handler => CompletionItemKind::CLASS,
                TypeKind::Qualifier => CompletionItemKind::TYPE_PARAMETER,
                TypeKind::Type => CompletionItemKind::CLASS,
            };
            out.push(CompletionItem {
                label: name.clone(),
                kind: Some(k),
                sort_text: Some(format!("0{name}")),
                ..Default::default()
            });
        }
    }
    for f in analysis.completions.get(view_file).into_iter().flatten() {
        if !f.name.starts_with("__") && seen.insert(f.name.clone()) {
            out.push(CompletionItem {
                label: f.name.clone(),
                kind: Some(CompletionItemKind::FUNCTION),
                sort_text: Some(format!("1{}", f.name)),
                ..Default::default()
            });
        }
    }
    out
}

/// [lsp-completion] The type name a receiver of type [ty] matches a first
/// parameter by: the base, without qualifiers or a module key.
fn receiver_name(ty: &Ty) -> Option<String> {
    match ty {
        Ty::Named { name, .. } => Some(name.split('§').next().unwrap_or(name).to_string()),
        Ty::Qualified { base, .. } => receiver_name(base),
        Ty::Lit(lit) => Some(lit.base().to_string()),
        _ => None,
    }
}

/// [lsp-std-source] The embedded std's file `name` as a read-only file on
/// disk, for an editor to open: under the system's temporary directory, in a
/// directory per compiler version, rewritten only when its content differs.
/// `None` when it cannot be written.
fn embedded_std_copy(name: &str, content: &str) -> Option<PathBuf> {
    let path = crate::analysis::std_copy_root().join(name);
    if std::fs::read_to_string(&path).ok().as_deref() != Some(content) {
        std::fs::create_dir_all(path.parent()?).ok()?;
        if let Ok(meta) = std::fs::metadata(&path) {
            let mut perms = meta.permissions();
            #[allow(clippy::permissions_set_readonly_false)]
            perms.set_readonly(false);
            let _ = std::fs::set_permissions(&path, perms);
        }
        std::fs::write(&path, content).ok()?;
        let mut perms = std::fs::metadata(&path).ok()?.permissions();
        perms.set_readonly(true);
        let _ = std::fs::set_permissions(&path, perms);
    }
    Some(path)
}

/// Whether [path] is one of [`embedded_std_copy`]'s files, which belong to no
/// project and are not analysed as one.
fn is_embedded_std_copy(path: &Path) -> bool {
    let dir = crate::analysis::std_copy_root();
    let dir = dir.canonicalize().unwrap_or(dir);
    let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    path.starts_with(&dir)
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

/// A sentence with its first letter lowered, for splicing after a dash.
fn lowercase_first(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(c) => c.to_lowercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// [doc-module] The file declaring `module` — the one non-annex file with
/// that path, since a module is a file.
fn module_file(analysis: &Analysis, module: &ModulePath) -> Option<usize> {
    analysis
        .program
        .files
        .iter()
        .position(|f| !f.is_test && f.module == *module)
}

/// [doc-module] The module *named* under the cursor, with the span of the
/// name: an `@module` selector the checker resolved (`Checked::module_refs`
/// [fn-overload-at]), or the module part of an `import` path — every
/// segment of a whole-module import [mod-import-module], the segments before
/// the item of a named one. The module part is one name however many
/// segments it has, so the hover covers all of it: `fs.mem` in
/// `import fs.mem.MemFs`, whichever segment the cursor is on.
fn module_at(
    analysis: &Analysis,
    checked: &Checked,
    file_idx: usize,
    offset: u32,
) -> Option<(Span, ModulePath)> {
    let mut best: Option<(Span, ModulePath)> = None;
    for ((file, span), module) in &checked.module_refs {
        if *file == file_idx
            && span.start <= offset
            && offset < span.end
            && best.as_ref().is_none_or(|(b, _)| span.len() < b.len())
        {
            best = Some((*span, module.clone()));
        }
    }
    if best.is_some() {
        return best;
    }
    let items = &analysis.program.modules.get(file_idx)?.items;
    for item in items {
        let Item::Import(decl) = item else { continue };
        let Some(first) = decl.path.first() else { continue };
        let Some(last) = decl.path.last() else { continue };
        if !(first.span.start <= offset && offset < last.span.end) {
            continue;
        }
        // Whole-module import, or the module part of a named one.
        let full: Vec<&str> = decl.path.iter().map(|p| p.name.as_str()).collect();
        let is_module = |segs: &[&str]| {
            analysis
                .program
                .files
                .iter()
                .any(|f| !f.is_test && f.module.0.iter().map(String::as_str).eq(segs.iter().copied()))
        };
        let module_len = if is_module(&full) {
            full.len()
        } else if full.len() > 1 && is_module(&full[..full.len() - 1]) {
            full.len() - 1
        } else {
            return None;
        };
        let end = decl.path[module_len - 1].span.end;
        if offset >= end {
            // On the item name, which `import_target` answers.
            return None;
        }
        let span = Span::new(first.span.start, end);
        let module = ModulePath(full[..module_len].iter().map(|s| s.to_string()).collect());
        return Some((span, module));
    }
    None
}

/// [doc-module] How many of a module's functions its hover lists before
/// counting the rest.
const MODULE_HOVER_FNS: usize = 20;

/// [doc-module] What a module offers, for its hover: the first
/// [`MODULE_HOVER_FNS`] functions a file `here` can call (its exports, or
/// every function when `here` is the module's own file), one signature per
/// name, then a count of the rest; and its exported types by name.
fn module_exports_section(
    ast: &salvo_syntax::ast::Module,
    module_file: usize,
    here: usize,
) -> Option<String> {
    let all = module_file == here;
    let mut fns: Vec<String> = Vec::new();
    let mut seen: HashSet<&str> = HashSet::new();
    let mut types: Vec<&str> = Vec::new();
    for item in &ast.items {
        match item {
            Item::Fn(f) if (f.exported || all) && !f.name.name.starts_with("__") => {
                if seen.insert(f.name.name.as_str()) {
                    fns.push(fn_decl_signature(f, None));
                }
            }
            Item::Struct(s) if s.exported || all => types.push(&s.name.name),
            Item::Effect(e) if e.exported || all => types.push(&e.name.name),
            Item::Handler(h) if h.exported || all => types.push(&h.name.name),
            Item::Type(t) if t.exported || all => types.push(&t.name.name),
            _ => {}
        }
    }
    if fns.is_empty() && types.is_empty() {
        return None;
    }
    let mut out = String::new();
    if !fns.is_empty() {
        let shown: Vec<&String> = fns.iter().take(MODULE_HOVER_FNS).collect();
        out.push_str("**Functions**\n\n```salvo\n");
        for sig in &shown {
            out.push_str(sig);
            out.push('\n');
        }
        out.push_str("```");
        if fns.len() > shown.len() {
            out.push_str(&format!("\n\nand {} more", fns.len() - shown.len()));
        }
    }
    if !types.is_empty() {
        if !out.is_empty() {
            out.push_str("\n\n");
        }
        let names: Vec<String> = types.iter().map(|t| format!("`{t}`")).collect();
        out.push_str(&format!("**Types**: {}", names.join(", ")));
    }
    Some(out)
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
    // [lsp-hover-iter-fn] An `iter fn` is desugared into a minter and a
    // `next` over a generated struct; either reads as the `iter fn` written.
    if let Some(sig) = iter_fn_signature(module, decl) {
        return Some(sig);
    }
    Some(fn_decl_signature(decl, checked.deductions.get(&key).map(|d| d.as_slice())))
}

/// [lsp-hover-iter-fn] The minter an `iter fn` became, from either half.
fn iter_fn_minter<'a>(module: &'a salvo_syntax::ast::Module, decl: &'a FnDecl) -> Option<&'a FnDecl> {
    use salvo_syntax::ast::Type;
    let prefix = salvo_syntax::desugar::ITER_STRUCT_PREFIX;
    let base_of = |t: &Type| match t {
        Type::Named { base, .. } => Some(base.name.name.clone()),
        _ => None,
    };
    let generated = decl
        .params
        .first()
        .and_then(|p| base_of(&p.ty))
        .filter(|n| n.starts_with(prefix) && decl.name.name == "next")
        .or_else(|| decl.return_type.as_ref().and_then(base_of).filter(|n| n.starts_with(prefix)))?;
    module.items.iter().find_map(|i| match i {
        Item::Fn(f) if f.return_type.as_ref().and_then(base_of).as_deref() == Some(generated.as_str()) => Some(f),
        _ => None,
    })
}

/// [lsp-hover-iter-fn] `iter fn name(params) [effects] -> Emitted T | Finished`
/// for the minter or the `next` an `iter fn` became, from the two of them.
fn iter_fn_signature(module: &salvo_syntax::ast::Module, decl: &FnDecl) -> Option<String> {
    use salvo_syntax::ast::Type;
    let prefix = salvo_syntax::desugar::ITER_STRUCT_PREFIX;
    let base_of = |t: &Type| match t {
        Type::Named { base, .. } => Some(base.name.name.clone()),
        _ => None,
    };
    let generated = decl
        .params
        .first()
        .and_then(|p| base_of(&p.ty))
        .filter(|n| n.starts_with(prefix) && decl.name.name == "next")
        .or_else(|| decl.return_type.as_ref().and_then(base_of).filter(|n| n.starts_with(prefix)))?;
    let fns = module.items.iter().filter_map(|i| match i {
        Item::Fn(f) => Some(f),
        _ => None,
    });
    let mut minter = None;
    let mut next = None;
    for f in fns {
        if f.return_type.as_ref().and_then(base_of).as_deref() == Some(generated.as_str()) {
            minter = Some(f);
        }
        if f.name.name == "next" && f.params.first().and_then(|p| base_of(&p.ty)).as_deref() == Some(generated.as_str()) {
            next = Some(f);
        }
    }
    let (minter, next) = (minter?, next?);
    let mut sig = fn_decl_signature(minter, None);
    // Swap the generated return type for the declared element shape.
    let ret = next.return_type.as_ref()?.to_string();
    if let Some(at) = sig.find(" -> ") {
        let tail_end = sig[at + 4..].find(" => ").map(|e| at + 4 + e).unwrap_or(sig.len());
        sig.replace_range(at + 4..tail_end, &ret);
    }
    if let Some(effects) = &next.effects {
        if minter.effects.as_ref().is_none_or(|e| e.is_empty()) && !effects.is_empty() {
            let list: Vec<String> = effects.iter().map(|e| e.to_string()).collect();
            if let Some(at) = sig.find(" -> ") {
                sig.insert_str(at, &format!(" [{}]", list.join(", ")));
            }
        }
    }
    // The minter's effect list is the generated `[]` when the written fn had
    // none; the `next` carries what it declared.
    if next.effects.as_ref().is_none_or(|e| e.is_empty()) {
        sig = sig.replacen(" [] -> ", " -> ", 1);
    }
    Some(format!("iter {sig}"))
}

/// [fn-type-named] The readable head of a type-named fn: the type's
/// qualifiers and base name, **without** the base's generic arguments
/// (`Mut Map<K, V>` → `Mut Map`). The fn's own `<…>` with `canbe` bounds is
/// appended by the caller, so the generics are shown once, with their bounds.
fn type_fn_name_head(ty: &salvo_syntax::ast::Type) -> String {
    use salvo_syntax::ast::Type;
    match ty {
        Type::Named { qualifiers, base } => {
            let mut head = String::new();
            for q in qualifiers {
                head.push_str(&q.name.name);
                head.push(' ');
            }
            head.push_str(&base.name.name);
            head
        }
        // Not a `Named` type-fn name (a provenance form, say); fall back to its
        // written form.
        other => format!("{other}"),
    }
}

/// A fn's source-like signature. `inferred` is the whole-program deduction
/// list when there is one — effect and handler members have no `FnKey`, so
/// they render their declared list only [doc-comment].
pub(crate) fn fn_decl_signature(decl: &FnDecl, inferred: Option<&[ParamDeduction]>) -> String {
    let mut sig = String::from("fn ");
    // [fn-type-named] A type-named constructor reads as its written type
    // (`Mut Map`), not its canonical key (`Mut_Map`, which is `decl.name.name`).
    // The qualifiers and base come from `type_name`; the generics with their
    // `canbe` bounds come from the fn's own list (so `<K, V canbe linear>` still
    // shows the linear opt-in), replacing the type's own generic arguments.
    match &decl.type_name {
        Some(ty) => sig.push_str(&type_fn_name_head(ty)),
        None => sig.push_str(&decl.name.name),
    }
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

    // [proj-infer] The opaque lends render where they are written — after
    // the return type: `-> T holds proj(p)`.
    let opaque: Vec<&str> = decl
        .deductions
        .iter()
        .flatten()
        .filter(|d| matches!(d.target, salvo_syntax::ast::DeductionTarget::Opaque))
        .filter_map(|d| d.proj_sources())
        .flatten()
        .map(|i| i.name.as_str())
        .collect();
    match &decl.return_type {
        Some(ty) if !opaque.is_empty() => {
            sig.push_str(&format!(" -> {ty} holds proj({})", opaque.join(", ")))
        }
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
    // Fn-typed parameters with a written group render theirs too. (The
    // opaque lends entry renders inline in the parameter's own type — a
    // list holding only that would render an empty group.)
    for p in &decl.params {
        if let salvo_syntax::ast::Type::Fn { deductions: Some(list), .. } = &p.ty {
            let rendered = render_declared(list);
            if !rendered.is_empty() {
                sig.push_str(&format!(" =>[{}] {rendered}", p.name.name));
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
    // [proj-infer] Lends already shown after the return type
    // (`-> T holds proj(p)`) are not repeated here; what remains is lends
    // written per field (`.f: proj(a)`) or inferred from the body.
    let opaque: HashSet<&str> = decl
        .deductions
        .iter()
        .flatten()
        .filter(|d| matches!(d.target, salvo_syntax::ast::DeductionTarget::Opaque))
        .filter_map(|d| d.proj_sources())
        .flatten()
        .map(|i| i.name.as_str())
        .collect();
    let lent: Vec<&str> = deductions
        .iter()
        .filter(|d| d.lent)
        .filter(|d| !opaque.contains(d.param.as_str()))
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
        // [proj-infer] The opaque lends render on the return type
        // (`-> T holds proj(p)`), where they are written.
        .filter(|d| !matches!(d.target, DeductionTarget::Opaque))
        .map(|d| {
            let t = target(d);
            match &d.kind {
                DeductionKind::KeepAll => t,
                // [implicit-with] The fill-together relation, as written: a
                // chain renders as one entry, which is how it was read.
                DeductionKind::With { others } => format!(
                    "{t} with {}",
                    others
                        .iter()
                        .map(|i| i.name.clone())
                        .collect::<Vec<_>>()
                        .join(" with ")
                ),
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
                // [qual-preserve] Part of the contract a reader needs: the
                // call keeps other values' dependent claims about this
                // parameter alive.
                DeductionKind::Preserve(quals) => {
                    format!("{t}: preserve {}", names(quals).join(" "))
                }
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

/// An open document as the analysis sees it: one of `program.files`.
struct DocView<'a> {
    file: usize,
    content: &'a str,
}

impl DocView<'_> {
    /// The document position as an offset into the file.
    fn offset(&self, position: Position) -> u32 {
        position_to_offset(self.content, position)
    }

    /// A span of the file as a range of this document.
    fn range(&self, span: Span) -> Range {
        span_to_range(self.content, span)
    }
}

/// The document at `path`: a file of the analysis. Normally a user `.sv`
/// file; for the std self-analysis [lsp-std-source] the open document *is* a
/// std file (a read-only copy under the temp root), so std files are matched
/// too — the `root.join(name) == path` test is exact, and a project
/// analysis's std files live under the project root, never at an opened path.
/// The document at `path`: a file of the analysis. Normally a user `.sv`
/// file; for the std self-analysis [lsp-std-source] the open document *is* a
/// std file (a read-only copy under the temp root `analysis.root`), so std
/// files are matched too. A **non-std** file is preferred when both would
/// match, so a project analysis (whose std files live under the project root
/// but are never the opened path) is unaffected.
fn doc_view<'a>(analysis: &'a Analysis, path: &Path) -> Option<DocView<'a>> {
    let files = &analysis.program.files;
    let exact = |f: &salvo_core::SourceFile| analysis.root.join(&f.name) == path;
    // Exact match, non-std preferred (a project analysis's std files live
    // under the project root and are never the opened path).
    if let Some(file) = files
        .iter()
        .position(|f| (!f.is_std || f.is_shadow) && exact(f))
        .or_else(|| files.iter().position(exact))
    {
        return Some(DocView { file, content: &files[file].content });
    }
    // [lsp-std-source] Fallback for a std-copy path: the opened URI may be
    // canonicalized (macOS resolves `/var` to `/private/var`) while
    // `analysis.root` is not, so an exact compare misses. Compare canonical
    // forms. Confined to this fallback so normal resolution stays byte-exact.
    let want = path.canonicalize().ok()?;
    let file = files
        .iter()
        .position(|f| analysis.root.join(&f.name).canonicalize().ok() == Some(want.clone()))?;
    Some(DocView { file, content: &files[file].content })
}

/// `diag` as the LSP sees it, at `span` of `content`.
fn to_lsp_diagnostic(diag: &FileDiagnostic, content: &str, span: Span) -> Diagnostic {
    Diagnostic {
        range: span_to_range(content, span),
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

/// [lsp-effect-fix] The effect a "no handler for effect `E` in scope"
/// diagnostic names.
fn missing_effect(message: &str) -> Option<String> {
    let rest = message.strip_prefix("no handler for effect `")?;
    Some(rest[..rest.find('`')?].to_string())
}

/// [lsp-effect-fix] Where `effect` goes in the top-level fn enclosing
/// `offset`, and the text to insert: `, E` before an existing list's `]`, `E`
/// into an empty `[]`, or ` [E]` after the parameter list.
fn effect_list_edit(text: &str, offset: usize, effect: &str) -> Option<(usize, String)> {
    let before = &text[..offset.min(text.len())];
    let mut start = None;
    let mut at = 0;
    for line in before.split_inclusive('\n') {
        let t = line.strip_prefix("export ").unwrap_or(line);
        let t = t.strip_prefix("iter ").unwrap_or(t);
        if t.starts_with("fn ") {
            start = Some(at);
        }
        at += line.len();
    }
    let start = start?;
    let header_end = start + text[start..].find('{')?;
    let header = &text[start..header_end];
    // The parameter list's closing paren, by depth.
    let open = header.find('(')?;
    let mut depth = 0;
    let mut close = None;
    for (i, c) in header[open..].char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    close = Some(open + i);
                    break;
                }
            }
            _ => {}
        }
    }
    let close = close?;
    let after = &header[close + 1..];
    let trimmed = after.trim_start();
    if trimmed.starts_with('[') {
        let lb = close + 1 + (after.len() - trimmed.len());
        let rb = lb + header[lb..].find(']')?;
        let inside = header[lb + 1..rb].trim();
        let text = if inside.is_empty() { effect.to_string() } else { format!(", {effect}") };
        return Some((start + rb, text));
    }
    Some((start + close + 1, format!(" [{effect}]")))
}

#[cfg(test)]
mod effect_fix_tests {
    use super::effect_list_edit;

    fn apply(text: &str, needle: &str) -> String {
        let at = text.find(needle).unwrap();
        let (o, t) = effect_list_edit(text, at, "Console").unwrap();
        format!("{}{}{}", &text[..o], t, &text[o..])
    }

    /// [lsp-effect-fix] No list, an empty one, and one with an effect.
    #[test]
    fn the_effect_goes_into_the_enclosing_fns_list() {
        assert_eq!(apply("fn f(x: Int) -> None {\n    println(\"\")\n}\n", "println"), "fn f(x: Int) [Console] -> None {\n    println(\"\")\n}\n");
        assert_eq!(apply("fn f() [] {\n    println(\"\")\n}\n", "println"), "fn f() [Console] {\n    println(\"\")\n}\n");
        assert_eq!(apply("export fn f(g: (Int) -> Int) [Fs] {\n    println(\"\")\n}\n", "println"), "export fn f(g: (Int) -> Int) [Fs, Console] {\n    println(\"\")\n}\n");
    }
}
