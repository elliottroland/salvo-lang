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

thread_local! {
    static NOTED: std::cell::RefCell<BTreeSet<String>> = const { std::cell::RefCell::new(BTreeSet::new()) };
}

/// The emitter names something the checker's tables do not: a union struct,
/// a generated `__Actor_H`, a qualifier's `..._qualifies`. Registering it is
/// how the module's import plan learns the name is used.
pub fn note(name: &str) {
    let last = name.rsplit("::").next().unwrap_or(name);
    NOTED.with(|n| {
        n.borrow_mut().insert(last.to_string());
    });
}

/// The names noted since the last call.
pub fn take_noted() -> BTreeSet<String> {
    NOTED.with(|n| std::mem::take(&mut *n.borrow_mut()))
}

/// One file's import plan: its index, its module, the foreign modules it
/// depends on, the aliases the emitter chose for clashing fns, and the names
/// the checker says its source refers to.
pub type Plan = (usize, ModulePath, BTreeSet<ModulePath>, BTreeMap<String, (ModulePath, String)>, BTreeSet<String>);

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

/// Replaces each planned file's marker with its explicit imports: one
/// `use` per mentioned name that exactly one of its foreign modules declares,
/// the aliases the emitter chose for clashing fns, and every trait of those
/// modules as `as _` (a method call names no trait, yet needs it in scope).
pub fn explicit_imports(
    files: &mut [EmittedFile],
    plans: &[Plan],
    module_prefixes: &HashMap<ModulePath, String>,
) {
    let mut by_module: HashMap<ModulePath, Declared> = HashMap::new();
    for (idx, module, _, _, _) in plans {
        let d = declared(&files[*idx].content);
        let entry = by_module.entry(module.clone()).or_default();
        entry.names.extend(d.names);
        entry.traits.extend(d.traits);
    }
    for (idx, own, deps, aliases, refs) in plans {
        let empty = Declared::default();
        let mine = by_module.get(own).unwrap_or(&empty);
        let foreign: Vec<(&ModulePath, &Declared, &String)> = deps
            .iter()
            .filter(|m| *m != own)
            .filter_map(|m| Some((m, by_module.get(m)?, module_prefixes.get(m)?)))
            .collect();
        let mut lines: BTreeSet<String> = BTreeSet::new();
        for word in refs.iter().cloned() {
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
