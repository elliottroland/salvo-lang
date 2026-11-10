//! The renames beyond a declaration's own name [lsp-rename-more]: a test's
//! name (the string after `test`), and a module (its import paths, its
//! `@module` selectors and its files).

use std::collections::HashMap;

use lsp_types::{
    DocumentChangeOperation, DocumentChanges, OptionalVersionedTextDocumentIdentifier, Range,
    RenameFile, ResourceOp, TextDocumentEdit, TextEdit, Url, WorkspaceEdit,
};
use salvo_syntax::ast::Item;
use salvo_syntax::Span;

use super::{module_at, span_text, DocView, Server};
use crate::analysis::Analysis;
use salvo_core::Checked;

/// What a rename of something other than a declaration would touch.
pub(super) enum Other {
    /// A test's name: the range inside its quotes.
    TestName(Range),
    /// A module's last path segment, where the cursor is.
    Module { segment: Range },
}

fn valid_module_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|c| c.is_ascii_lowercase() || c == '_')
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// The test whose name is under `offset`, read from the buffer's text — the
/// analysed program has turned its tests into fns.
fn test_name_at(content: &str, offset: u32) -> Option<Span> {
    let (mut module, _) = salvo_syntax::parse_module(content);
    // [mod-namespace] As the analysed program has them.
    salvo_syntax::desugar::flatten_namespaces(&mut module);
    module.items.iter().find_map(|item| match item {
        Item::Test(t) if t.name_span.start <= offset && offset <= t.name_span.end => {
            let text = span_text(content, t.name_span);
            let inner = text.strip_prefix('"')?.strip_suffix('"')?;
            let start = t.name_span.start + 1;
            Some(Span::new(start, start + inner.len() as u32))
        }
        _ => None,
    })
}

/// Every place a module's last path segment is written, as `(file, span)`:
/// in import paths, and at the end of `@module` selectors.
fn module_segments(analysis: &Analysis, checked: &Checked, module: &[String]) -> Vec<(usize, Span)> {
    let mut out = Vec::new();
    let Some(last) = module.last() else { return out };
    for (idx, m) in analysis.program.modules.iter().enumerate() {
        for item in &m.items {
            let Item::Import(decl) = item else { continue };
            if decl.path.len() >= module.len()
                && decl.path.iter().zip(module).all(|(seg, want)| seg.name == *want)
            {
                out.push((idx, decl.path[module.len() - 1].span));
            }
        }
    }
    for (k, m) in checked.refs.iter().filter_map(|(k, r)| Some((k, r.module()?))) {
        if m.0 != module || !k.span.is_written() {
            continue;
        }
        let file = k.file();
        let Some(f) = analysis.program.files.get(file) else { continue };
        if span_text(&f.content, k.span).ends_with(last.as_str()) {
            out.push((file, Span::new(k.span.end - last.len() as u32, k.span.end)));
        }
    }
    out.sort_unstable_by_key(|(f, s)| (*f, s.start));
    out.dedup();
    out
}

impl Server<'_> {
    /// What `offset` names, when it is a test or a module rather than a
    /// declaration.
    pub(super) fn other_rename_target(
        &self,
        analysis: &Analysis,
        checked: &Checked,
        view: &DocView<'_>,
        offset: u32,
    ) -> Option<Other> {
        if let Some(span) = test_name_at(view.content, offset) {
            return Some(Other::TestName(super::span_to_range(view.content, span)));
        }
        let (_, module) = module_at(analysis, checked, view.file, offset)?;
        let segments = module_segments(analysis, checked, &module.0);
        let (file, span) = segments
            .into_iter()
            .find(|(file, span)| *file == view.file && span.start <= offset && offset <= span.end)?;
        let _ = file;
        Some(Other::Module { segment: super::span_to_range(view.content, span) })
    }

    /// The edit for a test or module rename found by `other_rename_target`.
    pub(super) fn rename_other(
        &self,
        analysis: &Analysis,
        checked: &Checked,
        view: &DocView<'_>,
        offset: u32,
        target: Other,
        new_name: &str,
    ) -> Result<Option<WorkspaceEdit>, String> {
        match target {
            Other::TestName(range) => {
                if new_name.is_empty() || new_name.contains(['"', '\n', '\\']) || new_name.contains("${") {
                    return Err("a test's name is a plain string: no quotes, newlines, backslashes or interpolation".to_string());
                }
                let uri = self.uri_of(analysis, view.file)?;
                Ok(Some(WorkspaceEdit {
                    changes: Some(HashMap::from([(uri, vec![TextEdit { range, new_text: new_name.to_string() }])])),
                    ..Default::default()
                }))
            }
            Other::Module { .. } => {
                let (_, module) = module_at(analysis, checked, view.file, offset)
                    .ok_or_else(|| "nothing renameable at this position".to_string())?;
                if !valid_module_name(new_name) {
                    return Err("a module's name is lowercase letters, digits and `_`".to_string());
                }
                if !self.file_renames {
                    return Err("renaming a module renames its files; this editor does not support that".to_string());
                }
                // The module's files: `shapes.sv`, its annex and its backend companions.
                let stem = module.0.join("/");
                let files: Vec<usize> = analysis
                    .program
                    .files
                    .iter()
                    .enumerate()
                    .filter(|(_, f)| {
                        f.name == format!("{stem}.sv")
                            || (f.name.starts_with(&format!("{stem}.")) && f.name.ends_with(".sv") && !f.name[stem.len() + 1..].contains('/'))
                    })
                    .map(|(i, _)| i)
                    .collect();
                if files.is_empty() {
                    return Err("this module has no file of its own to rename".to_string());
                }
                if analysis.program.files.iter().any(|f| f.name.starts_with(&format!("{stem}/"))) {
                    return Err("this module has submodules in a directory of its name: rename that directory by hand".to_string());
                }
                if files.iter().any(|&i| {
                    let f = &analysis.program.files[i];
                    f.is_std && !f.is_shadow
                }) {
                    return Err("this module is part of the standard library and cannot be renamed".to_string());
                }
                let mut by_uri: HashMap<Url, Vec<TextEdit>> = HashMap::new();
                for (file, span) in module_segments(analysis, checked, &module.0) {
                    let (uri, range) = self
                        .site_location(analysis, file, span)
                        .ok_or_else(|| "one occurrence has no editable location".to_string())?;
                    by_uri.entry(uri).or_default().push(TextEdit { range, new_text: new_name.to_string() });
                }
                let mut ops: Vec<DocumentChangeOperation> = by_uri
                    .into_iter()
                    .map(|(uri, edits)| {
                        DocumentChangeOperation::Edit(TextDocumentEdit {
                            text_document: OptionalVersionedTextDocumentIdentifier { uri, version: None },
                            edits: edits.into_iter().map(lsp_types::OneOf::Left).collect(),
                        })
                    })
                    .collect();
                let last = module.0.last().cloned().unwrap_or_default();
                for i in files {
                    let (old_uri, _) = self
                        .site_location(analysis, i, Span::new(0, 0))
                        .ok_or_else(|| "a file of the module has no location".to_string())?;
                    let name = &analysis.program.files[i].name;
                    let file_name = name.rsplit('/').next().unwrap_or(name);
                    let renamed = format!("{new_name}{}", &file_name[last.len()..]);
                    let mut new_uri = old_uri.clone();
                    new_uri
                        .path_segments_mut()
                        .map_err(|_| "cannot rename this file".to_string())?
                        .pop()
                        .push(&renamed);
                    ops.push(DocumentChangeOperation::Op(ResourceOp::Rename(RenameFile {
                        old_uri,
                        new_uri,
                        options: None,
                        annotation_id: None,
                    })));
                }
                Ok(Some(WorkspaceEdit {
                    document_changes: Some(DocumentChanges::Operations(ops)),
                    ..Default::default()
                }))
            }
        }
    }

    /// The URI the client knows file `idx` by.
    fn uri_of(&self, analysis: &Analysis, idx: usize) -> Result<Url, String> {
        self.site_location(analysis, idx, Span::new(0, 0))
            .map(|(uri, _)| uri)
            .ok_or_else(|| "this document has no location".to_string())
    }
}
