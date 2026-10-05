//! [fn-emit-name] The emitted name of every top-level fn, shared by the
//! backends (user decision 2026-10-05).
//!
//! Neither target resolves Salvo's overloads (Rust has none; Kotlin would use
//! its own lattice [kt-fn-mangling]), so every overload a module emits needs a
//! name of its own. The rule is **per module and deterministic**: a fn whose
//! name is unique in its module keeps it; overloads within one module are
//! told apart by a suffix spelled from their parameter types, at the first
//! level that distinguishes the overload from every other one of its name in
//! that module:
//!
//! 1. each explicit parameter's base type (`next__StrYield`,
//!    `to_str__Int`, `map__List_Fn`);
//! 2. the base types with their qualifiers (`swap__MutList_IdxInt_IdxInt`);
//! 3. the full types, type arguments included (`get__ListT_Int`);
//! 4. the full types with their qualifiers;
//! 5. level 4 plus the names of the implicit parameters.
//!
//! Parameters are joined with `_`; a fn with no explicit parameters has an
//! empty suffix, so it keeps the bare name. Nothing outside the module takes
//! part, so a name never changes because another module, std or the user's,
//! gained a declaration. There are no positional numbers: overloads no level
//! tells apart are an error naming them, for a rule to be designed.

use std::collections::{BTreeMap, HashMap};

use salvo_syntax::ast::{FnDecl, Item, Type};

use crate::program::Program;
use crate::source::ModulePath;

/// The emitted names, in Salvo spelling (a backend applies its own casing to
/// the part before the first `__`, and keeps the suffix verbatim).
pub struct FnNames {
    by_decl: HashMap<*const FnDecl, String>,
    /// (module, Salvo name) → the emitted names of that name's fns there.
    by_module: HashMap<(ModulePath, String), Vec<String>>,
    /// Every top-level fn's module.
    modules: HashMap<*const FnDecl, ModulePath>,
    pub errors: Vec<String>,
}

/// Whether `f` is emitted as a named fn that takes part in naming: a fn
/// with a body. A platform fn is called through its `<name>_platform`
/// wrapper, and two platform fns never overload each other.
fn named(f: &FnDecl) -> bool {
    f.body.is_some() && !f.platform
}

impl FnNames {
    pub fn compute(program: &Program) -> FnNames {
        let mut groups: BTreeMap<(ModulePath, String), Vec<&FnDecl>> = BTreeMap::new();
        let mut modules = HashMap::new();
        let mut platform: Vec<(ModulePath, &FnDecl)> = Vec::new();
        for unit in program.units() {
            for item in &unit.ast.items {
                if let Item::Fn(f) = item {
                    modules.insert(f as *const FnDecl, unit.file.module.clone());
                    // A platform fn is called through its `<name>_platform`
                    // wrapper, one per name in a module.
                    if f.platform {
                        platform.push((unit.file.module.clone(), f));
                    }
                    if named(f) {
                        groups
                            .entry((unit.file.module.clone(), f.name.name.clone()))
                            .or_default()
                            .push(f);
                    }
                }
            }
        }
        let mut out = FnNames { by_decl: HashMap::new(), by_module: HashMap::new(), modules, errors: Vec::new() };
        for ((module, name), fns) in groups {
            let names: Vec<String> = {
                match overload_names(&name, &fns) {
                    Ok(names) => names,
                    Err(which) => {
                        out.errors.push(format!(
                            "module `{}` declares overloads of `{name}` that no naming rule tells \
                             apart ({}): their parameter types, qualifiers and implicit \
                             parameters all agree, so neither backend can give them names of \
                             their own [fn-emit-name]",
                            module.0.join("."),
                            which.join("; ")
                        ));
                        fns.iter().map(|_| name.clone()).collect()
                    }
                }
            };
            for (f, n) in fns.iter().zip(&names) {
                out.by_decl.insert(*f as *const FnDecl, n.clone());
            }
            out.by_module.insert((module, name), names);
        }
        for (module, f) in platform {
            let wrapper = format!("{}_platform", f.name.name);
            out.by_decl.insert(f as *const FnDecl, wrapper.clone());
            out.by_module.insert((module, wrapper.clone()), vec![wrapper]);
        }
        out
    }

    /// The emitted name of `decl`, when it is a named fn of the program.
    pub fn get(&self, decl: &FnDecl) -> Option<&str> {
        self.by_decl.get(&(decl as *const FnDecl)).map(|s| s.as_str())
    }

    /// The module declaring top-level fn `decl`.
    pub fn module_of(&self, decl: &FnDecl) -> Option<&ModulePath> {
        self.modules.get(&(decl as *const FnDecl))
    }

    /// [fn-emit-name] The foreign fns `file_idx` must call under an alias:
    /// those whose emitted name another fn it uses also has, from another
    /// module, or that its own module emits. Answered from what the checker
    /// resolved the file's uses to, so a name visible but unused never
    /// renames one that is used.
    pub fn clashing(
        &self,
        program: &Program,
        checked: &crate::check::Checked,
        file_idx: usize,
    ) -> std::collections::HashSet<(ModulePath, String)> {
        let own = &program.files[file_idx].module;
        let mut by_emitted: HashMap<String, std::collections::BTreeSet<ModulePath>> = HashMap::new();
        for key in crate::reach::resolved_fn_keys(checked, file_idx) {
            let Some(Item::Fn(f)) = program.modules.get(key.file).and_then(|m| m.items.get(key.item)) else {
                continue;
            };
            let Some(emitted) = self.get(f) else { continue };
            let module = &program.files[key.file].module;
            if module != own {
                by_emitted.entry(emitted.to_string()).or_default().insert(module.clone());
            }
        }
        let mut out = std::collections::HashSet::new();
        for (emitted, modules) in by_emitted {
            let salvo_name = emitted.split("__").next().unwrap_or(&emitted).to_string();
            let own_has = self.module_emits(own, &salvo_name, &emitted);
            if modules.len() > 1 || own_has {
                for m in modules {
                    out.insert((m, emitted.clone()));
                }
            }
        }
        out
    }

    /// Whether `module` emits a fn called `emitted` for Salvo name `name`.
    pub fn module_emits(&self, module: &ModulePath, name: &str, emitted: &str) -> bool {
        self.by_module
            .get(&(module.clone(), name.to_string()))
            .is_some_and(|ns| ns.iter().any(|n| n == emitted))
    }
}

/// [fn-emit-name] The emitted names of one overload set — `name` for one fn,
/// `name__<suffix>` per overload otherwise; `Err` lists the overloads no
/// level tells apart. Also how an effect's overloaded members are named
/// [effect-member-overload].
pub fn overload_names(name: &str, fns: &[&FnDecl]) -> Result<Vec<String>, Vec<String>> {
    if fns.len() <= 1 {
        return Ok(fns.iter().map(|_| name.to_string()).collect());
    }
    Ok(suffixes(fns)?
        .into_iter()
        .map(|s| if s.is_empty() { name.to_string() } else { format!("{name}__{s}") })
        .collect())
}

/// [fn-emit-name] The suffix of each overload, at the first level that sets
/// it apart from the rest; `Err` lists the overloads left indistinguishable.
fn suffixes(fns: &[&FnDecl]) -> Result<Vec<String>, Vec<String>> {
    let levels: Vec<Vec<String>> = (1..=5).map(|l| fns.iter().map(|f| suffix_at(f, l)).collect()).collect();
    let mut chosen = Vec::new();
    let mut stuck = Vec::new();
    for i in 0..fns.len() {
        let pick = levels
            .iter()
            .find(|level| (0..fns.len()).all(|j| j == i || level[j] != level[i]))
            .map(|level| level[i].clone());
        match pick {
            Some(s) => chosen.push(s),
            None => {
                stuck.push(signature(fns[i]));
                chosen.push(String::new());
            }
        }
    }
    // A suffix picked at one level may equal another's picked at another.
    for i in 0..chosen.len() {
        if stuck.is_empty() && chosen[..i].contains(&chosen[i]) {
            stuck.push(signature(fns[i]));
        }
    }
    if stuck.is_empty() {
        Ok(chosen)
    } else {
        Err(stuck)
    }
}

fn signature(f: &FnDecl) -> String {
    let ps: Vec<String> = f
        .params
        .iter()
        .map(|p| format!("{}{}: {}", if p.implicit { "?" } else { "" }, p.name.name, render(&p.ty, true, true)))
        .collect();
    format!("`{}({})`", f.name.name, ps.join(", "))
}

fn suffix_at(f: &FnDecl, level: u8) -> String {
    let (args, quals) = match level {
        1 => (false, false),
        2 => (false, true),
        3 => (true, false),
        _ => (true, true),
    };
    let mut parts: Vec<String> = f
        .params
        .iter()
        .filter(|p| !p.implicit)
        .map(|p| render(&p.ty, args, quals))
        .collect();
    if level >= 5 {
        parts.extend(f.params.iter().filter(|p| p.implicit).map(|p| p.name.name.clone()));
    }
    parts.join("_")
}

/// One parameter type as a suffix part: the base, with its type arguments
/// when `args`, with its qualifiers when `quals`. Dot-names flatten
/// [name-dot].
fn render(ty: &Type, args: bool, quals: bool) -> String {
    let flat = |s: &str| s.replace('.', "");
    let r = |t: &Type| render(t, args, quals);
    match ty {
        Type::Named { qualifiers, base } => {
            let mut out = String::new();
            if quals {
                for q in qualifiers {
                    out.push_str(&flat(&q.name.name));
                }
            }
            // A generated name's leading underscores (`__Iter_reversed_List`)
            // would run into the `__` before the suffix.
            out.push_str(flat(&base.name.name).trim_start_matches('_'));
            if args {
                for a in &base.args {
                    out.push_str(&r(a));
                }
            }
            out
        }
        Type::QualifiedGroup { qualifiers, base, .. } => {
            let mut out = String::new();
            if quals {
                for q in qualifiers {
                    out.push_str(&flat(&q.name.name));
                }
            }
            out.push_str(&r(base));
            out
        }
        Type::Union { arms, .. } => arms.iter().map(r).collect::<Vec<_>>().join("Or"),
        Type::Literal { value, .. } => format!("{value:?}").chars().filter(|c| c.is_ascii_alphanumeric()).collect(),
        Type::Tuple { elems, .. } => {
            let mut out = "Tuple".to_string();
            if args {
                for e in elems {
                    out.push_str(&r(e));
                }
            }
            out
        }
        Type::Array { elem, .. } => format!("{}Array", r(elem)),
        Type::Nullable { inner, .. } => format!("{}Opt", r(inner)),
        Type::Fn { params, ret, .. } => {
            let mut out = "Fn".to_string();
            if args {
                for p in params {
                    out.push_str(&r(p));
                }
                out.push_str("To");
                out.push_str(&r(ret));
            }
            out
        }
    }
}

/// [platform-iterable] The suffix of an iterable platform type's host loop
/// fns (`each`, `each_mut`, `into_each`): none when its module declares one
/// iterable platform type, else `_<type in snake case>` (`each_sorted_set`),
/// since one host file cannot hold two `each`s on Rust.
pub fn each_suffix(program: &Program, module: &ModulePath, type_name: &str) -> String {
    let iterables = program
        .units()
        .filter(|u| u.file.module == *module)
        .flat_map(|u| u.ast.items.iter())
        .filter(|i| matches!(i, Item::Type(t) if t.platform && t.iterable))
        .count();
    if iterables <= 1 {
        return String::new();
    }
    let mut out = String::from("_");
    for (i, c) in type_name.chars().enumerate() {
        if c.is_ascii_uppercase() {
            if i > 0 {
                out.push('_');
            }
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}
