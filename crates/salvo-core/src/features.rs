//! Runtime features [runtime-features]: which backend-neutral runtime
//! services a program's emitted modules need, decided from the declarations
//! and the checker's tables rather than from what an emitter happened to
//! synthesize.
//!
//! The features are an over-approximation by design: a feature too many
//! ships a runtime file nothing calls (wasteful), one too few leaves an
//! emitted call with nothing to bind to (broken).

use std::collections::HashSet;

use salvo_syntax::ast::{Item, Module};

use crate::check::Checked;
use crate::program::Symbols;

/// The neutral runtime services, by what they are.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RuntimeFeatures {
    /// The actor scheduler: an actor effect, a handler of one, or a
    /// spawn, send, reply or wait.
    pub scheduler: bool,
    /// The wire codecs: a message protocol or a struct with a wire form.
    pub wire: bool,
}

impl RuntimeFeatures {
    /// [time-timer] The scheduler's deadline thread and the wire's reply
    /// codecs read the clock, so time travels with either.
    pub fn time(&self) -> bool {
        self.scheduler || self.wire
    }

    pub fn or(&mut self, other: RuntimeFeatures) {
        self.scheduler |= other.scheduler;
        self.wire |= other.wire;
    }
}

/// What one module needs. `emitted` says whether the backend emits an item:
/// a platform-ABI build keeps only the declarations the host needs
/// [platform-abi], and then no fn body is emitted either (`abi`).
pub fn module_features(
    symbols: &Symbols<'_>,
    checked: &Checked,
    file_idx: usize,
    ast: &Module,
    abi: bool,
    emitted: impl Fn(&Item) -> bool,
) -> RuntimeFeatures {
    let mut f = RuntimeFeatures::default();
    for item in ast.items.iter().filter(|i| emitted(i)) {
        match item {
            Item::Effect(e) if e.fns.iter().any(|m| m.is_send) => {
                f.scheduler = true;
                if crate::effect_has_wire_form(symbols, e) {
                    f.wire = true;
                }
            }
            Item::Handler(h) => {
                let mut names: HashSet<&str> = HashSet::new();
                for of in &h.of {
                    crate::reach::type_names(of, &mut names);
                }
                if names
                    .iter()
                    .any(|n| symbols.effects.get(n).is_some_and(|e| e.fns.iter().any(|m| m.is_send)))
                {
                    f.scheduler = true;
                }
            }
            Item::Struct(s) if crate::struct_has_wire_form(symbols, s) => f.wire = true,
            _ => {}
        }
    }
    if abi {
        return f;
    }
    // The scheduler's own handle types and constructors [rs-actor]: an
    // `Addr`, `Pool` or `Reply` named anywhere in the module, or a `pool` /
    // `thread` intrinsic. A `Reply` also carries the wire.
    let refs = crate::reach::used_names(ast);
    let mut tys: HashSet<String> = HashSet::new();
    for ((f, _), t) in checked.expr_ty.iter() {
        if *f == file_idx {
            ty_root_names(t, &mut tys);
        }
    }
    let named = |n: &str| refs.contains(n) || tys.contains(n);
    if named("Addr") || named("Pool") || named("Reply") || refs.contains("pool") || refs.contains("thread") {
        f.scheduler = true;
    }
    if named("Reply") {
        f.wire = true;
    }
    let mine = |k: &&(usize, salvo_syntax::Span)| k.0 == file_idx;
    if checked.spawn_effects.keys().any(|k| mine(&k))
        || checked.use_addrs.keys().any(|k| mine(&k))
        || checked.addr_calls.keys().any(|k| mine(&k))
        || checked.task_mint_effects.keys().any(|k| mine(&k))
        || checked.actor_gates.iter().any(|(_, k)| k.0 == file_idx)
        || checked.facade_sends.keys().any(|k| mine(&k))
        || checked.replyto_members.keys().any(|k| mine(&k))
        || checked.self_sends.keys().any(|k| mine(&k))
        || checked.waitfor_sites.iter().any(|(_, k)| k.0 == file_idx)
        || checked.actor_sends.iter().any(|(_, _, k)| k.0 == file_idx)
        || checked.task_sends.iter().any(|(_, _, k)| k.0 == file_idx)
        || checked.handler_constructs.iter().any(|(_, _, k)| k.0 == file_idx)
    {
        f.scheduler = true;
    }
    f
}

fn ty_root_names(t: &crate::types::Ty, out: &mut HashSet<String>) {
    use crate::types::Ty;
    match t {
        Ty::Named { name, args } => {
            out.insert(crate::typekey::plain(name).to_string());
            args.iter().for_each(|a| ty_root_names(a, out));
        }
        Ty::Qualified { base, .. } => ty_root_names(base, out),
        Ty::Union(v) | Ty::Tuple(v) => v.iter().for_each(|a| ty_root_names(a, out)),
        Ty::Array(b) => ty_root_names(b, out),
        Ty::Fn { params, ret, .. } => {
            params.iter().for_each(|a| ty_root_names(a, out));
            ty_root_names(ret, out);
        }
        _ => {}
    }
}

/// Temporary verification: append a mismatch to the file named by
/// `SALVO_FEATURE_DIFF`.
pub fn report(msg: &str) {
    use std::io::Write;
    if let Ok(path) = std::env::var("SALVO_FEATURE_DIFF") {
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
            let _ = writeln!(f, "{msg}");
        }
    }
}
