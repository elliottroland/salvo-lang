//! The code actions the language server offers beyond the import quickfix:
//! fixes keyed on the diagnostics' ids [lsp-quickfix], stubs for what a
//! declaration is missing [lsp-stub], and organize-imports
//! [lsp-organize-imports].
//!
//! Every action is a text edit (or a command the server runs itself); none
//! rewrites the program's meaning. The fixes keyed on a diagnostic are
//! offered only where the diagnostic is, and `organizeImports` checks each
//! removal by analysing the result.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use lsp_types::{
    CodeAction, CodeActionKind, CodeActionOrCommand, CodeActionParams, Command, Diagnostic,
    NumberOrString, Range, TextEdit, Url, WorkspaceEdit,
};
use salvo_syntax::ast::{Expr, Item};
use salvo_syntax::diag::Severity;
use salvo_syntax::visit::{walk_fn, Visitor};
use salvo_syntax::Span;

use super::{
    offset_to_position, position_to_offset, Server, PLATFORM_GENERATE_COMMAND,
};
use crate::analysis::Analysis;

/// The id a diagnostic carries, as the checker named it.
fn code_of(diag: &Diagnostic) -> Option<&str> {
    match diag.code.as_ref()? {
        NumberOrString::String(s) => Some(s.as_str()),
        NumberOrString::Number(_) => None,
    }
}

/// The names in backticks in a message, in order.
fn ticked(message: &str) -> Vec<&str> {
    message.split('`').skip(1).step_by(2).collect()
}

/// The leading whitespace of the line holding `offset`.
fn indent_of(text: &str, offset: usize) -> &str {
    let start = text[..offset.min(text.len())].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let end = text[start..].find(|c: char| c != ' ' && c != '\t').map(|i| start + i).unwrap_or(text.len());
    &text[start..end]
}

fn quickfix(title: String, uri: &Url, diags: Vec<Diagnostic>, edits: Vec<TextEdit>) -> CodeActionOrCommand {
    CodeActionOrCommand::CodeAction(CodeAction {
        title,
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: Some(diags),
        edit: Some(WorkspaceEdit {
            changes: Some(HashMap::from([(uri.clone(), edits)])),
            ..Default::default()
        }),
        ..Default::default()
    })
}

impl Server<'_> {
    /// Everything but the import quickfix and the missing-effect fix, which
    /// `code_actions` keeps.
    pub(super) fn extra_actions(
        &self,
        params: &CodeActionParams,
        path: &Path,
        content: &str,
        out: &mut Vec<CodeActionOrCommand>,
    ) {
        let uri = &params.text_document.uri;
        let only = params.context.only.as_ref();
        let wants = |kind: &CodeActionKind| only.is_none_or(|o| o.iter().any(|k| kind.as_str().starts_with(k.as_str())));
        let diags = &params.context.diagnostics;

        if wants(&CodeActionKind::QUICKFIX) && !diags.is_empty() {
            self.field_fixes(uri, content, diags, out);
            self.deduction_fixes(uri, content, diags, out);
            let needs_analysis = diags.iter().any(|d| {
                matches!(
                    code_of(d),
                    Some("when-exhaustive:missing-arms")
                        | Some("when-exhaustive:kind-arms-missing")
                        | Some("effect-handler-multi:missing-member")
                        | Some("call-resolve:no-overload")
                        | Some("call-resolve:no-overload-why")
                )
            });
            if needs_analysis {
                if let Some(analysis) = self.analyze_for(path) {
                    self.when_arm_fixes(&analysis, path, uri, content, diags, out);
                    self.comptime_when_fixes(&analysis, path, uri, content, diags, out);
                    self.handler_member_fixes(&analysis, path, uri, content, diags, out);
                    self.obligation_fixes(&analysis, path, uri, content, diags, out);
                }
            }
        }
        // [lsp-organize-imports] Only when asked for by kind: it analyses the
        // program once for each import.
        if only.is_some_and(|o| o.iter().any(|k| CodeActionKind::SOURCE_ORGANIZE_IMPORTS.as_str().starts_with(k.as_str()) || k.as_str() == "source")) {
            if let Some(edit) = self.organize_imports(path, content) {
                out.push(CodeActionOrCommand::CodeAction(CodeAction {
                    title: "Organize imports".to_string(),
                    kind: Some(CodeActionKind::SOURCE_ORGANIZE_IMPORTS),
                    edit: Some(WorkspaceEdit {
                        changes: Some(HashMap::from([(uri.clone(), vec![edit])])),
                        ..Default::default()
                    }),
                    ..Default::default()
                }));
            }
        }
        // [lsp-stub] The host skeleton for a program that declares platform
        // handlers or fns.
        if only.is_none_or(|o| o.iter().any(|k| "source.generate".starts_with(k.as_str()))) {
            if let Some(analysis) = self.analyze_for(path) {
                let declares_platform = super::doc_view(&analysis, path).and_then(|v| v.module().map(|m| {
                    m.items.iter().any(|i| match i {
                        Item::Fn(f) => f.platform,
                        Item::Handler(h) => h.platform,
                        _ => false,
                    })
                })).unwrap_or(false);
                if declares_platform {
                    out.push(CodeActionOrCommand::CodeAction(CodeAction {
                        title: "Generate platform host skeletons (`salvo platform generate`)".to_string(),
                        kind: Some(CodeActionKind::new("source.generate")),
                        command: Some(Command {
                            title: "Generate platform host skeletons".to_string(),
                            command: PLATFORM_GENERATE_COMMAND.to_string(),
                            arguments: Some(vec![serde_json::json!(path.display().to_string())]),
                        }),
                        ..Default::default()
                    }));
                }
            }
        }
    }

    /// [lsp-quickfix] `struct-defaults:missing-field`: the fields a literal
    /// leaves out, written as `todo!()` — a value of every type, trapping if
    /// it is ever reached.
    fn field_fixes(&self, uri: &Url, text: &str, diags: &[Diagnostic], out: &mut Vec<CodeActionOrCommand>) {
        let mut by_literal: Vec<(Range, Vec<String>, Vec<Diagnostic>)> = Vec::new();
        for d in diags.iter().filter(|d| code_of(d) == Some("struct-defaults:missing-field")) {
            let Some(field) = ticked(&d.message).first().map(|f| f.to_string()) else { continue };
            match by_literal.iter_mut().find(|(r, _, _)| *r == d.range) {
                Some((_, fields, ds)) => {
                    fields.push(field);
                    ds.push(d.clone());
                }
                None => by_literal.push((d.range, vec![field], vec![d.clone()])),
            }
        }
        for (range, fields, ds) in by_literal {
            let end = position_to_offset(text, range.end) as usize;
            let start = position_to_offset(text, range.start) as usize;
            if end == 0 || text.as_bytes().get(end - 1) != Some(&b'}') {
                continue;
            }
            let close = end - 1;
            let Some(last) = text[start..close].rfind(|c: char| !c.is_whitespace()).map(|i| start + i) else { continue };
            let multiline = text[start..close].contains('\n');
            let mut insert = String::new();
            let sep = match text.as_bytes()[last] {
                b',' => "",
                b'{' => "",
                _ => ",",
            };
            let indent = indent_of(text, last).to_string();
            for (i, f) in fields.iter().enumerate() {
                let lead = if i == 0 { sep } else { "," };
                if multiline {
                    insert.push_str(&format!("{lead}\n{indent}{f}: todo!()"));
                } else if text.as_bytes()[last] == b'{' && i == 0 {
                    insert.push_str(&format!(" {f}: todo!()"));
                } else {
                    insert.push_str(&format!("{lead} {f}: todo!()"));
                }
            }
            if !multiline && text.as_bytes()[last] == b'{' {
                insert.push(' ');
            }
            let at = offset_to_position(text, (last + 1) as u32);
            let names = fields.iter().map(|f| format!("`{f}`")).collect::<Vec<_>>().join(", ");
            out.push(quickfix(
                format!("Add the missing field{} {names}", if fields.len() == 1 { "" } else { "s" }),
                uri,
                ds,
                vec![TextEdit { range: Range::new(at, at), new_text: insert }],
            ));
        }
    }

    /// [lsp-quickfix] `deduce-default:kept-by-default-moved`: declare the move
    /// the body makes, `=> !xs`, in the fn's deduction clause.
    fn deduction_fixes(&self, uri: &Url, text: &str, diags: &[Diagnostic], out: &mut Vec<CodeActionOrCommand>) {
        for d in diags.iter().filter(|d| code_of(d) == Some("deduce-default:kept-by-default-moved")) {
            let Some(param) = ticked(&d.message).first().map(|p| p.to_string()) else { continue };
            let at = position_to_offset(text, d.range.start) as usize;
            let Some(header) = fn_header(text, at) else { continue };
            let head = &text[header.0..header.1];
            let trimmed_end = header.0 + head.trim_end().len();
            let new_text = if head.contains("=>") { format!(", !{param}") } else { format!(" => !{param}") };
            let pos = offset_to_position(text, trimmed_end as u32);
            out.push(quickfix(
                format!("Declare the move: `=> !{param}`"),
                uri,
                vec![d.clone()],
                vec![TextEdit { range: Range::new(pos, pos), new_text }],
            ));
        }
    }

    /// [lsp-quickfix] `when-exhaustive:missing-arms`: one arm for each union
    /// arm the `when` leaves out, each a `todo!()` to fill in.
    fn when_arm_fixes(
        &self,
        analysis: &Analysis,
        path: &Path,
        uri: &Url,
        text: &str,
        diags: &[Diagnostic],
        out: &mut Vec<CodeActionOrCommand>,
    ) {
        let Some(view) = super::doc_view(analysis, path) else { return };
        let Some(checked) = analysis.checked.as_ref() else { return };
        let Some(module) = view.module() else { return };
        for d in diags.iter().filter(|d| code_of(d) == Some("when-exhaustive:missing-arms")) {
            let start = position_to_offset(text, d.range.start);
            struct Find {
                start: u32,
                found: Option<Span>,
            }
            impl Visitor for Find {
                fn visit_expr(&mut self, e: &Expr) {
                    if let Expr::When { span, .. } = e {
                        if span.is_written() && span.start == self.start && self.found.is_none_or(|f| span.len() > f.len()) {
                            self.found = Some(*span);
                        }
                    }
                }
            }
            let mut find = Find { start, found: None };
            for item in &module.items {
                if let Item::Fn(f) = item {
                    walk_fn(&mut find, f);
                }
            }
            let Some(span) = find.found else { continue };
            let Some(missing) = checked.when_missing.get(&checked.key_at(view.file, span)) else { continue };
            let named: Vec<&String> = missing.iter().filter(|m| m.as_str() != "*").collect();
            if named.is_empty() {
                continue;
            }
            let close = span.end as usize - 1;
            if text.as_bytes().get(close) != Some(&b'}') {
                continue;
            }
            let line_start = text[..close].rfind('\n').map(|i| i + 1).unwrap_or(0);
            let own_line = text[line_start..close].trim().is_empty();
            let base = indent_of(text, start as usize);
            let (at, new_text) = if own_line {
                let mut arms = String::new();
                for m in &named {
                    arms.push_str(&format!("{base}    is {m} {{\n{base}        todo!()\n{base}    }}\n"));
                }
                (offset_to_position(text, line_start as u32), arms)
            } else {
                let arms = named.iter().map(|m| format!("is {m} {{ todo!() }}")).collect::<Vec<_>>().join(" ");
                (offset_to_position(text, close as u32), format!(" {arms} "))
            };
            out.push(quickfix(
                format!("Add the {} missing arm{}", named.len(), if named.len() == 1 { "" } else { "s" }),
                uri,
                vec![d.clone()],
                vec![TextEdit { range: Range::new(at, at), new_text }],
            ));
        }
    }

    /// [lsp-quickfix] `when-exhaustive:kind-arms-missing`: an arm for each kind
    /// a comptime `[when x.type.kind]` leaves out, each a `refuse!` — a compile
    /// error at the instantiation naming the case — until it is written.
    fn comptime_when_fixes(
        &self,
        analysis: &Analysis,
        path: &Path,
        uri: &Url,
        text: &str,
        diags: &[Diagnostic],
        out: &mut Vec<CodeActionOrCommand>,
    ) {
        let Some(view) = super::doc_view(analysis, path) else { return };
        for d in diags.iter().filter(|d| code_of(d) == Some("when-exhaustive:kind-arms-missing")) {
            let at = position_to_offset(text, d.range.start);
            let Some(w) = analysis
                .comptime_when_arms
                .iter()
                .filter(|w| w.file == view.file && w.span.start <= at && at <= w.span.end)
                .min_by_key(|w| w.span.len())
            else {
                continue;
            };
            let named: Vec<&String> = w.missing.iter().filter(|m| m.as_str() != "*").collect();
            let close = w.span.end as usize - 1;
            if named.is_empty() || text.as_bytes().get(close) != Some(&b'}') {
                continue;
            }
            let line_start = text[..close].rfind('\n').map(|i| i + 1).unwrap_or(0);
            let base = indent_of(text, w.span.start as usize);
            let (at_pos, new_text) = if text[line_start..close].trim().is_empty() {
                let arms: String = named
                    .iter()
                    .map(|m| format!("{base}    is {m} {{ refuse!(\"not handled yet: {m}\") }}\n"))
                    .collect();
                (offset_to_position(text, line_start as u32), arms)
            } else {
                let arms = named.iter().map(|m| format!("is {m} {{ refuse!(\"not handled yet: {m}\") }}")).collect::<Vec<_>>().join(" ");
                (offset_to_position(text, close as u32), format!(" {arms} "))
            };
            out.push(quickfix(
                format!("Add the {} missing arm{}", named.len(), if named.len() == 1 { "" } else { "s" }),
                uri,
                vec![d.clone()],
                vec![TextEdit { range: Range::new(at_pos, at_pos), new_text }],
            ));
        }
    }

    /// [lsp-stub] `effect-handler-multi:missing-member`: a stub for each
    /// member of the effect the handler does not implement, with the member's
    /// signature as the effect declares it.
    fn handler_member_fixes(
        &self,
        analysis: &Analysis,
        path: &Path,
        uri: &Url,
        text: &str,
        diags: &[Diagnostic],
        out: &mut Vec<CodeActionOrCommand>,
    ) {
        let Some(view) = super::doc_view(analysis, path) else { return };
        let Some(module) = view.module() else { return };
        // handler name span start -> (stubs, diagnostics)
        let mut by_handler: Vec<(Span, String, Vec<String>, Vec<Diagnostic>)> = Vec::new();
        for d in diags.iter().filter(|d| code_of(d) == Some("effect-handler-multi:missing-member")) {
            let ticks = ticked(&d.message);
            let (Some(handler), Some(member_path)) = (ticks.first(), ticks.get(1)) else { continue };
            let Some((effect, rest)) = member_path.split_once('.') else { continue };
            let member = rest.split('(').next().unwrap_or(rest);
            let params = rest.split_once('(').map(|(_, p)| p.trim_end_matches(')')).unwrap_or("");
            let effect_name = effect.rsplit('.').next().unwrap_or(effect);
            let Some(h) = module.items.iter().find_map(|i| match i {
                Item::Handler(h) if h.name.name == *handler => Some(h),
                _ => None,
            }) else {
                continue;
            };
            // The member as its effect declares it, text and all.
            let mut stub: Option<String> = None;
            for (file, m) in analysis.program.files.iter().zip(&analysis.program.modules) {
                for item in &m.items {
                    let Item::Effect(e) = item else { continue };
                    if e.name.name != effect_name {
                        continue;
                    }
                    let candidates: Vec<_> = e.fns.iter().filter(|f| f.name.name == member).collect();
                    let pick = candidates
                        .iter()
                        .find(|f| f.params.len() == params.split(',').filter(|p| !p.trim().is_empty()).count())
                        .or(candidates.first());
                    if let Some(f) = pick {
                        // The header as the effect writes it, from the name to the
                        // end of its line, without the deduction clause (a
                        // handler's member has none): the parameters, effects
                        // and return type.
                        let from = f.name.span.start as usize;
                        let line_end = file.content[from..].find('\n').map(|i| from + i).unwrap_or(file.content.len());
                        let line = &file.content[from..line_end];
                        let line = line.split(" =>").next().unwrap_or(line).split(" {").next().unwrap_or(line).trim();
                        let modifier = if f.is_send { "send " } else { "" };
                        stub = Some(format!("{modifier}fn {line}"));
                    }
                }
            }
            let Some(stub) = stub else { continue };
            match by_handler.iter_mut().find(|(s, ..)| *s == h.span) {
                Some((_, _, stubs, ds)) => {
                    stubs.push(stub);
                    ds.push(d.clone());
                }
                None => by_handler.push((h.span, handler.to_string(), vec![stub], vec![d.clone()])),
            }
        }
        for (span, name, stubs, ds) in by_handler {
            let close = span.end as usize - 1;
            if text.as_bytes().get(close) != Some(&b'}') {
                continue;
            }
            let base = indent_of(text, span.start as usize);
            let line_start = text[..close].rfind('\n').map(|i| i + 1).unwrap_or(0);
            if !text[line_start..close].trim().is_empty() {
                continue;
            }
            // A blank line between a member and the entry before it, unless
            // it opens the body.
            let previous = text[..line_start].lines().last().map(str::trim).unwrap_or("");
            let mut new_text = String::new();
            if !(previous.is_empty() || previous.ends_with('{')) {
                new_text.push('\n');
            }
            for (i, stub) in stubs.iter().enumerate() {
                if i > 0 {
                    new_text.push('\n');
                }
                new_text.push_str(&format!("{base}    {stub} {{\n{base}        todo!()\n{base}    }}\n"));
            }
            let at = offset_to_position(text, line_start as u32);
            out.push(quickfix(
                format!("Implement the {} missing member{} of `{name}`", stubs.len(), if stubs.len() == 1 { "" } else { "s" }),
                uri,
                ds,
                vec![TextEdit { range: Range::new(at, at), new_text }],
            ));
        }
    }

    /// [lsp-stub] A call with no overload (`to_str(p)`) that a group's
    /// `by auto` obligation would supply: add `: ToStr<self> by auto` to the
    /// struct. The groups std stamps structurally: `to_str` for `ToStr`,
    /// `cmp` for `Ordered`, `hash` for `Hashed`.
    fn obligation_fixes(
        &self,
        analysis: &Analysis,
        path: &Path,
        uri: &Url,
        text: &str,
        diags: &[Diagnostic],
        out: &mut Vec<CodeActionOrCommand>,
    ) {
        let Some(view) = super::doc_view(analysis, path) else { return };
        let Some(module) = view.module() else { return };
        for d in diags
            .iter()
            .filter(|d| matches!(code_of(d), Some("call-resolve:no-overload") | Some("call-resolve:no-overload-why")))
        {
            let Some(call) = ticked(&d.message).first().map(|c| c.to_string()) else { continue };
            let Some((name, args)) = call.split_once('(') else { continue };
            let group = match name {
                "to_str" => "ToStr",
                "cmp" => "Ordered",
                "hash" => "Hashed",
                _ => continue,
            };
            let first_arg = args.trim_end_matches(')').split(',').next().unwrap_or("").trim();
            let ty = first_arg.rsplit(' ').next().unwrap_or(first_arg);
            let Some(s) = module.items.iter().find_map(|i| match i {
                Item::Struct(s) if s.name.name == ty => Some(s),
                _ => None,
            }) else {
                continue;
            };
            if s.obligations.iter().any(|o| o.group.name.name == group) {
                continue;
            }
            let clause = format!("{group}<self> by auto");
            let (offset, new_text) = match s.obligations.last() {
                None => {
                    let mut end = s.name.span.end as usize;
                    if text.as_bytes().get(end) == Some(&b'<') {
                        let mut depth = 0;
                        for (i, c) in text[end..].char_indices() {
                            match c {
                                '<' => depth += 1,
                                '>' => {
                                    depth -= 1;
                                    if depth == 0 {
                                        end += i + 1;
                                        break;
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    (end, format!(" : {clause}"))
                }
                Some(last) => match &last.by {
                    Some(by) => match by.path.last() {
                        Some(id) => (id.span.end as usize, format!(", {clause}")),
                        None => continue,
                    },
                    None => continue,
                },
            };
            let at = offset_to_position(text, offset as u32);
            out.push(quickfix(
                format!("Add `: {clause}` to struct `{ty}`"),
                uri,
                vec![d.clone()],
                vec![TextEdit { range: Range::new(at, at), new_text }],
            ));
        }
    }

    /// [lsp-organize-imports] The file's top-level imports with duplicates
    /// and unused ones removed and the rest sorted. An import is unused when
    /// the program, analysed without it, has no more errors than with it —
    /// so only offered while the program has none, and `export import`s are
    /// kept (the others using them are not in this file).
    fn organize_imports(&self, path: &Path, content: &str) -> Option<TextEdit> {
        let baseline = self.analysis_with(path, &self.overlay)?;
        let errors = |a: &Analysis| a.diagnostics.iter().filter(|d| d.severity == Severity::Error).count();
        let clean = baseline.checked.is_some() && errors(&baseline) == 0;

        // Top-level import lines.
        let lines: Vec<&str> = content.split_inclusive('\n').collect();
        let is_import = |l: &str| l.starts_with("import ") || l.starts_with("export import ");
        let first = lines.iter().position(|l| is_import(l))?;
        let last = lines.iter().rposition(|l| is_import(l))?;
        let region = &lines[first..=last];
        let sortable = region.iter().all(|l| is_import(l) || l.trim().is_empty());

        // Duplicates.
        let mut keep: Vec<&str> = Vec::new();
        for l in region {
            if is_import(l) && keep.iter().any(|k| k.trim() == l.trim()) {
                continue;
            }
            keep.push(l);
        }
        // Unused: removing it leaves the program no worse.
        if clean {
            let mut current: Vec<&str> = keep.clone();
            let mut i = 0;
            while i < current.len() {
                let l = current[i];
                if !l.starts_with("import ") {
                    i += 1;
                    continue;
                }
                let mut trial = current.clone();
                trial.remove(i);
                let text: String = lines[..first].iter().copied().chain(trial.iter().copied()).chain(lines[last + 1..].iter().copied()).collect();
                let mut overlay = self.overlay.clone();
                overlay.insert(path.to_path_buf(), text);
                let ok = self
                    .analysis_with(path, &overlay)
                    .is_some_and(|a| a.checked.is_some() && errors(&a) == 0);
                if ok {
                    current = trial;
                } else {
                    i += 1;
                }
            }
            keep = current;
        }
        let mut rebuilt: Vec<String> = keep.iter().map(|l| l.to_string()).collect();
        if sortable {
            rebuilt.retain(|l| !l.trim().is_empty());
            rebuilt.sort_by(|a, b| a.trim().trim_start_matches("export ").cmp(b.trim().trim_start_matches("export ")));
            for l in rebuilt.iter_mut() {
                if !l.ends_with('\n') {
                    l.push('\n');
                }
            }
        }
        let new_text: String = rebuilt.concat();
        let old_text: String = region.concat();
        if new_text == old_text {
            return None;
        }
        let start: usize = lines[..first].iter().map(|l| l.len()).sum();
        let end = start + old_text.len();
        Some(TextEdit {
            range: Range::new(offset_to_position(content, start as u32), offset_to_position(content, end as u32)),
            new_text,
        })
    }

    /// [lsp-stub] `workspace/executeCommand`: run `salvo platform generate`
    /// for the project of the document the action was offered on, and tell
    /// the user what it said.
    pub(super) fn execute_command(&self, params: &lsp_types::ExecuteCommandParams) -> Option<serde_json::Value> {
        if params.command != PLATFORM_GENERATE_COMMAND {
            return None;
        }
        let doc: PathBuf = params.arguments.first()?.as_str()?.into();
        let dir = doc.parent()?.to_path_buf();
        let exe = std::env::current_exe().ok()?;
        let output = std::process::Command::new(exe)
            .args(["platform", "generate"])
            .current_dir(&dir)
            .stdin(std::process::Stdio::null())
            .output()
            .ok()?;
        let said = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let typ = if output.status.success() { lsp_types::MessageType::INFO } else { lsp_types::MessageType::ERROR };
        let note = lsp_server::Notification::new(
            "window/showMessage".to_string(),
            lsp_types::ShowMessageParams { typ, message: if said.is_empty() { "salvo platform generate finished".to_string() } else { said } },
        );
        let _ = self.connection.sender.send(lsp_server::Message::Notification(note));
        Some(serde_json::Value::Null)
    }
}

/// The text range from the start of the fn enclosing `offset` to the `{`
/// that opens its body.
fn fn_header(text: &str, offset: usize) -> Option<(usize, usize)> {
    let before = &text[..offset.min(text.len())];
    let mut start = None;
    let mut at = 0;
    for line in before.split_inclusive('\n') {
        let t = line.strip_prefix("export ").unwrap_or(line);
        let t = t.strip_prefix("iter ").unwrap_or(t);
        let t = t.strip_prefix("comptime ").unwrap_or(t);
        if t.starts_with("fn ") {
            start = Some(at);
        }
        at += line.len();
    }
    let start = start?;
    let end = start + text[start..].find('{')?;
    Some((start, end))
}
