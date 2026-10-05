//! [kt-imports] Explicit imports (user decision 2026-10-05): a generated file
//! imports every Salvo declaration it uses by name, rather than a wildcard
//! per module. The emitter writes a marker after the package's runtime
//! imports; once every module is emitted, the names each module declares are
//! read off its text, the names each file mentions off its own, and each name
//! a file mentions that exactly one of its foreign modules declares becomes
//! `import salvo.<module>.<name>`. The `salvo` runtime package keeps its
//! wildcard: it never declares a Salvo name.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::EmittedFile;
use salvo_core::ModulePath;

/// Where a file's explicit imports go.
pub const IMPORTS_MARK: &str = "//@@salvo-imports@@\n";

#[derive(Default)]
struct Declared {
    names: BTreeSet<String>,
    /// The subset of `names` that are fns: Kotlin overloads a fn name
    /// imported from several packages, where a class name would conflict.
    fns: BTreeSet<String>,
    /// Extension fns: reached after a `.`, so imported wherever their name
    /// appears at all.
    extensions: BTreeSet<String>,
}

const MODIFIERS: &[&str] = &[
    "public ", "internal ", "data ", "sealed ", "abstract ", "open ", "inline ", "enum ", "value ",
    "operator ", "infix ", "const ", "suspend ", "tailrec ", "annotation ", "fun interface ",
];

fn ident_of(s: &str) -> String {
    if let Some(rest) = s.strip_prefix('`') {
        return rest.split('`').next().unwrap_or("").to_string();
    }
    s.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect()
}

/// Skips a leading `<…>`, balanced.
fn skip_type_params(s: &str) -> &str {
    let s = s.trim_start();
    if !s.starts_with('<') {
        return s;
    }
    let mut depth = 0;
    for (i, ch) in s.char_indices() {
        match ch {
            '<' => depth += 1,
            '>' => {
                depth -= 1;
                if depth == 0 {
                    return s[i + 1..].trim_start();
                }
            }
            _ => {}
        }
    }
    s
}

/// The names declared at the top level of `text` (column 0, not `private`).
fn declared(text: &str) -> Declared {
    let mut out = Declared::default();
    for line in text.lines() {
        if line.starts_with("private ") {
            continue;
        }
        let mut rest = line;
        loop {
            let before = rest;
            for m in MODIFIERS {
                if let Some(r) = rest.strip_prefix(m) {
                    rest = r;
                }
            }
            if rest == before {
                break;
            }
        }
        if let Some(r) = rest.strip_prefix("fun").filter(|r| r.starts_with(' ') || r.starts_with('<')) {
            let r = skip_type_params(r);
            let head = r.split('(').next().unwrap_or("");
            // An extension: `fun <T> List<T>.name(`.
            let mut depth = 0;
            let mut dot = None;
            for (i, ch) in head.char_indices() {
                match ch {
                    '<' => depth += 1,
                    '>' => depth -= 1,
                    '.' if depth == 0 => dot = Some(i),
                    _ => {}
                }
            }
            match dot {
                Some(i) => {
                    let name = ident_of(&head[i + 1..]);
                    if !name.is_empty() {
                        out.extensions.insert(name);
                    }
                }
                None => {
                    let name = ident_of(head.trim());
                    if !name.is_empty() {
                        out.fns.insert(name.clone());
                        out.names.insert(name);
                    }
                }
            }
            continue;
        }
        for kw in ["class ", "interface ", "object ", "typealias ", "val ", "var "] {
            if let Some(r) = rest.strip_prefix(kw) {
                let name = ident_of(r.trim_start());
                if !name.is_empty() {
                    out.names.insert(name);
                }
                break;
            }
        }
    }
    out
}

/// The identifiers `text` uses, split into those that stand as a name of
/// their own and those after a `.` (only an extension fn is reached that
/// way). Comments, string text and char literals are skipped; the code in a
/// string template is scanned.
fn mentioned(text: &str) -> (BTreeSet<String>, BTreeSet<String>) {
    let c: Vec<char> = text.chars().collect();
    let mut bare = BTreeSet::new();
    let mut dotted = BTreeSet::new();
    scan(&c, 0, c.len(), &mut bare, &mut dotted);
    (bare, dotted)
}

fn ident_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

/// Scans code in `c[i..end]`, stopping early at an unbalanced `}` (the end of
/// a template expression); answers where it stopped.
fn scan(c: &[char], mut i: usize, end: usize, bare: &mut BTreeSet<String>, dotted: &mut BTreeSet<String>) -> usize {
    let mut braces = 0usize;
    while i < end {
        let ch = c[i];
        if ch == '/' && i + 1 < end && c[i + 1] == '/' {
            while i < end && c[i] != '\n' {
                i += 1;
            }
            continue;
        }
        if ch == '/' && i + 1 < end && c[i + 1] == '*' {
            let mut depth = 0;
            while i < end {
                if c[i] == '/' && i + 1 < end && c[i + 1] == '*' {
                    depth += 1;
                    i += 2;
                } else if c[i] == '*' && i + 1 < end && c[i + 1] == '/' {
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
        if ch == '"' {
            let raw = i + 2 < end && c[i + 1] == '"' && c[i + 2] == '"';
            i += if raw { 3 } else { 1 };
            while i < end {
                if raw && c[i] == '"' && i + 2 < end && c[i + 1] == '"' && c[i + 2] == '"' {
                    i += 3;
                    while i < end && c[i] == '"' {
                        i += 1;
                    }
                    break;
                }
                if !raw && c[i] == '"' {
                    i += 1;
                    break;
                }
                if !raw && c[i] == '\\' {
                    i += 2;
                    continue;
                }
                if c[i] == '$' && i + 1 < end && c[i + 1] == '{' {
                    i = scan(c, i + 2, end, bare, dotted) + 1;
                    continue;
                }
                if c[i] == '$' && i + 1 < end && (c[i + 1].is_alphabetic() || c[i + 1] == '_') {
                    let s = i + 1;
                    i = s;
                    while i < end && ident_char(c[i]) {
                        i += 1;
                    }
                    bare.insert(c[s..i].iter().collect());
                    continue;
                }
                i += 1;
            }
            continue;
        }
        if ch == '\'' {
            i += 1;
            while i < end && c[i] != '\'' {
                if c[i] == '\\' {
                    i += 1;
                }
                i += 1;
            }
            i += 1;
            continue;
        }
        if ch == '{' {
            braces += 1;
        }
        if ch == '}' {
            if braces == 0 {
                return i;
            }
            braces -= 1;
        }
        if ch.is_ascii_digit() {
            while i < end && ident_char(c[i]) {
                i += 1;
            }
            continue;
        }
        if ch == '`' {
            let s = i + 1;
            i = s;
            while i < end && c[i] != '`' {
                i += 1;
            }
            let word: String = c[s..i.min(end)].iter().collect();
            i += 1;
            classify(c, s - 1, word, bare, dotted);
            continue;
        }
        if ch.is_alphabetic() || ch == '_' {
            let s = i;
            while i < end && ident_char(c[i]) {
                i += 1;
            }
            classify(c, s, c[s..i].iter().collect(), bare, dotted);
            continue;
        }
        i += 1;
    }
    end
}

fn classify(c: &[char], start: usize, word: String, bare: &mut BTreeSet<String>, dotted: &mut BTreeSet<String>) {
    let mut k = start;
    while k > 0 && (c[k - 1] == ' ' || c[k - 1] == '\n') {
        k -= 1;
    }
    // `::name` is a callable reference, which names a top-level fn bare.
    if k >= 1 && c[k - 1] == '.' {
        dotted.insert(word);
    } else {
        bare.insert(word);
    }
}

/// Replaces each planned file's marker with its explicit imports: one per
/// mentioned name exactly one of its foreign modules declares, every
/// mentioned extension fn of those modules, and the aliases the emitter
/// chose for clashing fns.
pub fn explicit_imports(
    files: &mut [EmittedFile],
    plans: &[(usize, ModulePath, BTreeSet<ModulePath>, BTreeMap<String, (ModulePath, String)>)],
    package: fn(&ModulePath) -> String,
) {
    let mut by_module: HashMap<ModulePath, Declared> = HashMap::new();
    for (idx, module, _, _) in plans {
        let d = declared(&files[*idx].content);
        let entry = by_module.entry(module.clone()).or_default();
        entry.names.extend(d.names);
        entry.fns.extend(d.fns);
        entry.extensions.extend(d.extensions);
    }
    for (idx, own, deps, aliases) in plans {
        let (bare, dotted) = mentioned(&files[*idx].content);
        let empty = Declared::default();
        let mine = by_module.get(own).unwrap_or(&empty);
        let foreign: Vec<(&Declared, String)> = deps
            .iter()
            .filter(|m| *m != own)
            .filter_map(|m| Some((by_module.get(m)?, package(m))))
            .collect();
        let mut lines: BTreeSet<String> = BTreeSet::new();
        for word in &bare {
            if mine.names.contains(word) || aliases.contains_key(word) {
                continue;
            }
            let from: Vec<(&Declared, &String)> =
                foreign.iter().filter(|(d, _)| d.names.contains(word)).map(|(d, p)| (*d, p)).collect();
            // One declaring module; or several that all declare it as a fn,
            // which is how a predicate qualifier's `Q_qualifies` overloads by
            // subject across modules (the emitter's own fns never meet here:
            // a clash is aliased [fn-emit-name]). Several where one is a
            // class would conflict, so none is imported.
            let all_fns = from.iter().all(|(d, _)| d.fns.contains(word));
            if from.len() == 1 || all_fns {
                for (_, pkg) in from {
                    lines.insert(format!("import {pkg}.{}", quote(word)));
                }
            }
        }
        for word in bare.iter().chain(&dotted) {
            for (d, pkg) in &foreign {
                if d.extensions.contains(word) && !mine.names.contains(word) {
                    lines.insert(format!("import {pkg}.{}", quote(word)));
                }
            }
        }
        // The fns the emitter named, each from the module its call resolved
        // to: under its own name, or the alias a clash needed.
        for (local, (module, real)) in aliases {
            if local == real {
                lines.insert(format!("import {}.{real}", package(module)));
            } else {
                lines.insert(format!("import {}.{real} as {local}", package(module)));
            }
        }
        let block: String = lines.into_iter().map(|l| l + "\n").collect();
        files[*idx].content = files[*idx].content.replacen(IMPORTS_MARK, &block, 1);
    }
}

/// A Kotlin keyword used as a name is written in backticks.
fn quote(word: &str) -> String {
    if crate::emit::is_kotlin_keyword(word) {
        format!("`{word}`")
    } else {
        word.to_string()
    }
}
