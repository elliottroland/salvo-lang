//! [rs-imports] Explicit imports (user decision 2026-10-05): a generated file
//! names every Salvo declaration it uses with a `use` of its own, rather than
//! a glob per module. The emitters write a marker where the imports go; once
//! every module is emitted, the names each module declares are read off its
//! text, the names each file mentions are read off its own, and each name a
//! file mentions that exactly one of its foreign modules declares becomes
//! `use crate::<module>::<name>;`. The backend's runtime files keep their
//! globs: they never declare a Salvo name.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::EmittedFile;
use salvo_core::ModulePath;

/// Where a file's explicit imports go.
pub const IMPORTS_MARK: &str = "//@@salvo-imports@@\n";

/// What one emitted Salvo module declares at top level.
#[derive(Default)]
struct Declared {
    names: BTreeSet<String>,
    traits: BTreeSet<String>,
}

/// The names declared at the top level of `text`: every item at column 0,
/// public or not (an import of a name the file declares would clash), and
/// the names a `use` there brings in.
fn declared(text: &str) -> Declared {
    let mut out = Declared::default();
    for line in text.lines() {
        let mut rest = line;
        for vis in ["pub(crate) ", "pub "] {
            if let Some(r) = rest.strip_prefix(vis) {
                rest = r;
                break;
            }
        }
        if let Some(r) = rest.strip_prefix("use ") {
            let r = r.trim_end_matches(';');
            let name = match r.rsplit_once(" as ") {
                Some((_, alias)) => alias,
                None => r.rsplit("::").next().unwrap_or(r),
            };
            if name != "*" && name != "_" && !name.contains('{') {
                out.names.insert(name.to_string());
            }
            continue;
        }
        for kw in ["fn ", "struct ", "enum ", "trait ", "type ", "const ", "static ", "mod "] {
            if let Some(r) = rest.strip_prefix(kw) {
                let r = r.strip_prefix("mut ").unwrap_or(r);
                let name: String = r.chars().take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '#').collect();
                let name = name.strip_prefix("r#").unwrap_or(&name).to_string();
                if !name.is_empty() && name != "_" {
                    if kw == "trait " {
                        out.traits.insert(name.clone());
                    }
                    out.names.insert(name);
                }
                break;
            }
        }
    }
    out
}

/// The identifiers `text` uses as a path's first segment: not after `.` or
/// `::`, not a macro name, and outside comments, strings and char literals.
pub fn mentioned(text: &str) -> BTreeSet<String> {
    let c: Vec<char> = text.chars().collect();
    let mut out = BTreeSet::new();
    let mut i = 0;
    let n = c.len();
    let ident_start = |ch: char| ch.is_alphabetic() || ch == '_';
    let ident_char = |ch: char| ch.is_alphanumeric() || ch == '_';
    while i < n {
        let ch = c[i];
        // Comments.
        if ch == '/' && i + 1 < n && c[i + 1] == '/' {
            while i < n && c[i] != '\n' {
                i += 1;
            }
            continue;
        }
        if ch == '/' && i + 1 < n && c[i + 1] == '*' {
            let mut depth = 0;
            while i < n {
                if c[i] == '/' && i + 1 < n && c[i + 1] == '*' {
                    depth += 1;
                    i += 2;
                } else if c[i] == '*' && i + 1 < n && c[i + 1] == '/' {
                    depth -= 1;
                    i += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    i += 1;
                }
            }
            continue;
        }
        // Raw strings: r"…", r#"…"#, br#"…"#.
        if (ch == 'r' || (ch == 'b' && i + 1 < n && c[i + 1] == 'r'))
            && (i == 0 || !ident_char(c[i - 1]))
        {
            let mut j = if ch == 'b' { i + 2 } else { i + 1 };
            let mut hashes = 0;
            while j < n && c[j] == '#' {
                hashes += 1;
                j += 1;
            }
            if j < n && c[j] == '"' {
                j += 1;
                loop {
                    if j >= n {
                        break;
                    }
                    if c[j] == '"' && (0..hashes).all(|k| j + 1 + k < n && c[j + 1 + k] == '#') {
                        j += 1 + hashes;
                        break;
                    }
                    j += 1;
                }
                i = j;
                continue;
            }
        }
        if ch == '"' {
            i += 1;
            while i < n && c[i] != '"' {
                if c[i] == '\\' {
                    i += 1;
                }
                i += 1;
            }
            i += 1;
            continue;
        }
        if ch == '\'' {
            // A char literal, or a lifetime (whose name is not a path).
            if i + 1 < n && c[i + 1] == '\\' {
                i += 2;
                while i < n && c[i] != '\'' {
                    i += 1;
                }
                i += 1;
            } else if i + 2 < n && c[i + 2] == '\'' {
                i += 3;
            } else {
                i += 1;
                while i < n && ident_char(c[i]) {
                    i += 1;
                }
            }
            continue;
        }
        if ch.is_ascii_digit() {
            while i < n && ident_char(c[i]) {
                i += 1;
            }
            continue;
        }
        if ident_start(ch) {
            let start = i;
            while i < n && ident_char(c[i]) {
                i += 1;
            }
            let mut word: String = c[start..i].iter().collect();
            // A raw identifier, `r#type`.
            if word == "r" && i + 1 < n && c[i] == '#' && ident_start(c[i + 1]) {
                let s2 = i + 1;
                i = s2;
                while i < n && ident_char(c[i]) {
                    i += 1;
                }
                word = c[s2..i].iter().collect();
            }
            let mut k = start;
            while k > 0 && c[k - 1] == ' ' {
                k -= 1;
            }
            let after_path = k >= 1 && (c[k - 1] == '.' || (k >= 2 && c[k - 1] == ':' && c[k - 2] == ':'));
            let macro_name = i < n && c[i] == '!' && !(i + 1 < n && c[i + 1] == '=');
            if !after_path && !macro_name {
                out.insert(word);
            }
            continue;
        }
        i += 1;
    }
    out
}

/// Replaces each planned file's marker with its explicit imports: one
/// `use` per mentioned name that exactly one of its foreign modules declares,
/// the aliases the emitter chose for clashing fns, and every trait of those
/// modules as `as _` (a method call names no trait, yet needs it in scope).
pub fn explicit_imports(
    files: &mut [EmittedFile],
    plans: &[(usize, ModulePath, BTreeSet<ModulePath>, BTreeMap<String, (ModulePath, String)>)],
    module_prefixes: &HashMap<ModulePath, String>,
) {
    let mut by_module: HashMap<ModulePath, Declared> = HashMap::new();
    for (idx, module, _, _) in plans {
        let d = declared(&files[*idx].content);
        let entry = by_module.entry(module.clone()).or_default();
        entry.names.extend(d.names);
        entry.traits.extend(d.traits);
    }
    for (idx, own, deps, aliases) in plans {
        let text = &files[*idx].content;
        let empty = Declared::default();
        let mine = by_module.get(own).unwrap_or(&empty);
        let foreign: Vec<(&ModulePath, &Declared, &String)> = deps
            .iter()
            .filter(|m| *m != own)
            .filter_map(|m| Some((m, by_module.get(m)?, module_prefixes.get(m)?)))
            .collect();
        let mut lines: BTreeSet<String> = BTreeSet::new();
        for word in mentioned(text) {
            if mine.names.contains(&word) || aliases.contains_key(&word) {
                continue;
            }
            let from: Vec<&String> =
                foreign.iter().filter(|(_, d, _)| d.names.contains(&word)).map(|(_, _, p)| *p).collect();
            // Two modules declaring a mentioned name: the emitter qualifies or
            // aliases the uses that matter, so neither is imported bare.
            if let [prefix] = from.as_slice() {
                lines.insert(format!("use {prefix}{word};"));
            }
        }
        // The fns the emitter named, each from the module its call resolved
        // to: under its own name, or the alias a clash needed.
        for (local, (module, real)) in aliases {
            if let Some(prefix) = module_prefixes.get(module) {
                if local == real {
                    lines.insert(format!("use {prefix}{real};"));
                } else {
                    lines.insert(format!("use {prefix}{real} as {local};"));
                }
            }
        }
        for (_, d, prefix) in &foreign {
            for t in &d.traits {
                if !mine.names.contains(t) {
                    lines.insert(format!("use {prefix}{t} as _;"));
                }
            }
        }
        let block: String = lines.into_iter().map(|l| l + "\n").collect();
        files[*idx].content = files[*idx].content.replacen(IMPORTS_MARK, &block, 1);
    }
}
