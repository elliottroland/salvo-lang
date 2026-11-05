//! [name-table] The resolved form of every written type-position name
//! (the pipeline record C1.2): what each name a program writes where a type, a
//! qualifier, an effect or a group goes means, bound once by resolution in
//! the file that wrote it.
//!
//! A name is bound in every namespace at once ([`Meaning`]): the struct, the
//! effect, the `intrinsic`/`platform` type, the alias, the handler, the
//! qualifiers, the group and the effect alias it names there, and whether it
//! is a type parameter of the declaration it is written in. Which namespace
//! counts is the position's question, and the checker asks it of the
//! position (a qualifier list, a base type, an effect list) — the questions
//! it asked of the file's scope before, now answered for the reference
//! rather than for whatever file is being checked when it is lowered (a
//! callee's signature is lowered at every call).
//!
//! A reference is found by its address, as resolution walked it; a copy (the
//! checker, the IR builder and the emitters clone types freely) by its file's
//! tag and node [node-file] [node-id], its span and its written name — what a
//! copy keeps — when only one written reference has them, or only references
//! meaning the same (a tree no program tagged has no tag to tell its files
//! apart).

use std::collections::HashMap;

use salvo_syntax::ast::{EffectAliasDecl, EffectDecl, FnsDecl, HandlerDecl, Ident, Module, QualifierDecl, StructDecl, TypeDecl, TypeRef};
use salvo_syntax::Span;

use crate::resolve::{DefSite, FnEntry, ModuleScope};

pub mod dump;
mod values;
pub use values::{ValueRole, WrittenValue};

/// [name-table] What one written name means where it is written, in each
/// namespace: the scope's answer for that file (or for the stamp a
/// comptime-stamped body is checked in), fixed at resolution.
#[derive(Clone, Default)]
pub struct Meaning<'p> {
    pub structure: Option<&'p StructDecl>,
    pub effect: Option<&'p EffectDecl>,
    /// An `intrinsic type` or a `platform type`.
    pub opaque: Option<&'p TypeDecl>,
    /// A `type` alias or union.
    pub alias: Option<&'p TypeDecl>,
    pub handler: Option<&'p HandlerDecl>,
    /// [qual-overload] The qualifiers of the name, over every subject.
    pub qualifiers: Vec<&'p QualifierDecl>,
    pub group: Option<&'p FnsDecl>,
    /// [effect-alias] With the file it is declared in.
    pub effect_alias: Option<(&'p EffectAliasDecl, usize)>,
    /// [lsp-definition] Where the declaration's name is written.
    pub def: Option<DefSite>,
    /// [fn-overload-scope] The fns of the name, every rung.
    pub fns: Vec<FnEntry<'p>>,
    /// [effect-member-overload] The effect members of the name this scope
    /// sees (an effect available in a body adds its own: the checker's).
    pub members: Vec<(&'p EffectDecl, &'p salvo_syntax::ast::FnDecl)>,
}

/// [qual-identity] [qual-overload] Of the qualifier declarations a written
/// name means, the key it is: theirs when they share one; else the one whose
/// subject is the written base type (`base`, a plain name), else a generic
/// one, else the first.
pub fn pick_qual_key<'p>(candidates: &[&'p QualifierDecl], symbols: &crate::program::Symbols<'p>, base: Option<&str>) -> Option<&'p str> {
    let first = candidates.first()?;
    let key = |d: &QualifierDecl| symbols.qual_key(d);
    let k0 = key(first);
    if candidates.iter().all(|d| key(d) == k0) {
        return Some(k0);
    }
    let about = |d: &&&'p QualifierDecl| crate::refine::of_base(&d.of, &d.generics);
    if let Some(base) = base.map(crate::typekey::plain) {
        if let Some(d) = candidates.iter().find(|d| about(d).as_deref() == Some(base)) {
            return Some(key(d));
        }
    }
    candidates.iter().find(|d| about(d).is_none()).map(|d| key(d)).or(Some(k0))
}

impl<'p> Meaning<'p> {
    /// What `name` means in `scope`.
    pub fn of(scope: &ModuleScope<'p>, name: &str) -> Meaning<'p> {
        Meaning {
            structure: scope.structs.get(name).copied(),
            effect: scope.effects.get(name).copied(),
            opaque: scope.opaque_types.get(name).copied(),
            alias: scope.type_aliases.get(name).copied(),
            handler: scope.handlers.get(name).copied(),
            qualifiers: scope.qualifiers.get(name).cloned().unwrap_or_default(),
            group: scope.fn_groups.get(name).copied(),
            effect_alias: scope.effect_aliases.get(name).copied(),
            def: scope.def_sites.get(name).copied(),
            fns: scope.fns.get(name).cloned().unwrap_or_default(),
            members: scope.effect_members.get(name).cloned().unwrap_or_default(),
        }
    }

    /// [name-resolve] Whether the name is a type as a base type position
    /// reads it: a struct, an `intrinsic` or `platform` type, an alias, or
    /// an effect (effect lists are written as types). Generics and `None`
    /// are the position's to add.
    pub fn is_type(&self) -> bool {
        self.structure.is_some() || self.opaque.is_some() || self.alias.is_some() || self.effect.is_some()
    }

    /// [name-resolve] Whether the name is a declared qualifier.
    pub fn is_qualifier(&self) -> bool {
        !self.qualifiers.is_empty()
    }

    /// [type-identity] The key of the declaration the name means as a type
    /// (or, failing one, as a handler): what `resolve::key_in` answers.
    /// [qual-identity] [qual-overload] The key of the qualifier the name
    /// means, where several are visible picked by the written subject's base
    /// name (`base`): one module's declarations share a key, and of two
    /// modules' a declaration about that very type beats a generic one (`for
    /// T`), as the checker's subject pick does. `None` when no qualifier has
    /// the name (a compiler qualifier, `Mut`, or nothing).
    pub fn qual_key(&self, symbols: &crate::program::Symbols<'p>, base: Option<&str>) -> Option<&'p str> {
        pick_qual_key(&self.qualifiers, symbols, base)
    }

    pub fn key(&self, symbols: &crate::program::Symbols<'p>) -> Option<&'p str> {
        let addr = self
            .structure
            .map(|d| d as *const StructDecl as usize)
            .or_else(|| self.effect.map(|d| d as *const EffectDecl as usize))
            .or_else(|| self.opaque.map(|d| d as *const TypeDecl as usize))
            .or_else(|| self.alias.map(|d| d as *const TypeDecl as usize))
            .or_else(|| self.handler.map(|d| d as *const HandlerDecl as usize))?;
        symbols.decl_keys.get(&addr).copied()
    }

    /// The generics a struct, an `intrinsic`/`platform` type or an alias of
    /// the name declares (`None` for anything else).
    pub fn declared_generics(&self) -> Option<&'p [Ident]> {
        self.structure
            .map(|d| d.generics.as_slice())
            .or_else(|| self.opaque.map(|d| d.generics.as_slice()))
            .or_else(|| self.alias.map(|d| d.generics.as_slice()))
    }

    /// Whether two meanings name the same declarations (a qualifier set in
    /// any order).
    pub fn same(&self, other: &Meaning<'p>) -> bool {
        fn p<T>(a: Option<&T>) -> usize {
            a.map_or(0, |a| a as *const T as usize)
        }
        p(self.structure) == p(other.structure)
            && p(self.effect) == p(other.effect)
            && p(self.opaque) == p(other.opaque)
            && p(self.alias) == p(other.alias)
            && p(self.handler) == p(other.handler)
            && p(self.group) == p(other.group)
            && p(self.effect_alias.map(|(a, _)| a)) == p(other.effect_alias.map(|(a, _)| a))
            && self.qualifiers.len() == other.qualifiers.len()
            && self.qualifiers.iter().all(|a| other.qualifiers.iter().any(|b| std::ptr::eq(*a, *b)))
            && self.fns.len() == other.fns.len()
            && self.fns.iter().all(|a| other.fns.iter().any(|b| a.key == b.key))
            && self.members.len() == other.members.len()
            && self.members.iter().all(|a| other.members.iter().any(|b| std::ptr::eq(a.1, b.1)))
    }
}

/// [comptime-hygiene] The scope a reference is resolved in: a file's — the
/// one it is written in, or for a comptime-stamped body's copy of its
/// template's text, the template's file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum At {
    File(u32),
}

/// [name-table] One written reference.
#[derive(Clone, Copy, Debug)]
pub struct Written {
    /// The reference's address in the tree resolution walked.
    pub addr: usize,
    /// The file it is written in.
    pub file: u32,
    /// The scope it is resolved in.
    pub at: At,
    /// The name's span, its node [node-id] included.
    pub span: Span,
    /// Whether the name is a type parameter of a declaration it is written
    /// in (a fn's, its container's, a refinement's).
    pub generic: bool,
    /// [effect-var] Whether that type parameter is an effect variable (`[E]`).
    pub effect_var: bool,
    name: u32,
    meaning: u32,
}

/// [name-table] Every written type-position name of a program, resolved.
#[derive(Default)]
pub struct Names<'p> {
    meanings: Vec<Meaning<'p>>,
    /// The written names, interned.
    spellings: Vec<String>,
    spelling_ids: HashMap<String, u32>,
    refs: Vec<Written>,
    by_addr: HashMap<usize, u32>,
    /// A copy's way in: by file tag and node [node-file], span and name —
    /// `None` where two written references there mean different things.
    by_copy: HashMap<(u32, u32, Span, u32), Option<u32>>,
    /// [name-table] The value-position names, the same three ways.
    values: Vec<WrittenValue>,
    value_by_addr: HashMap<usize, u32>,
    value_by_copy: HashMap<(u32, u32, Span, u32), Option<u32>>,
    empty: Option<u32>,
    /// [effect-available] The members a body's written effects add to a
    /// value name (`WrittenValue::available`).
    member_sets: Vec<Vec<(&'p EffectDecl, &'p salvo_syntax::ast::FnDecl)>>,
}

impl<'p> Names<'p> {
    /// Resolves every written reference of `modules`, each in `scopes`' scope
    /// of the file that wrote it. `templates` gives each stamped fn (by
    /// address) its template's file, and `template_refs` each type reference
    /// of a stamped fn's copy of its template's text: those resolve where the
    /// template is written when that file sees the name, and at the stamping
    /// site otherwise (a name the stamp put there, such as a field's type)
    /// [comptime-hygiene].
    pub fn build<M: std::borrow::Borrow<Module>>(
        modules: &[M],
        scopes: &[ModuleScope<'p>],
        templates: &HashMap<usize, usize>,
        template_refs: &HashMap<usize, usize>,
    ) -> Names<'p> {
        use salvo_syntax::visit::Visitor;
        struct Walk<'w, 'p> {
            file: u32,
            names: &'w mut Names<'p>,
            interned: &'w mut HashMap<(At, u32), u32>,
            scopes: &'w [ModuleScope<'p>],
            template_refs: &'w HashMap<usize, usize>,
            generics: Vec<Vec<String>>,
            effect_vars: Vec<Vec<String>>,
        }
        impl<'p> Visitor for Walk<'_, 'p> {
            fn visit_type_ref(&mut self, t: &TypeRef) {
                let addr = t as *const TypeRef as usize;
                let at = match self.template_refs.get(&addr) {
                    Some(&f) if self.scopes[f].declares_name(&t.name.name) => At::File(f as u32),
                    _ => At::File(self.file),
                };
                let name = t.name.name.as_str();
                let spelling = self.names.spell(name);
                let meaning = match self.interned.get(&(at, spelling)) {
                    Some(m) => *m,
                    None => {
                        let At::File(f) = at;
                        let scope = &self.scopes[f as usize];
                        self.names.meanings.push(Meaning::of(scope, name));
                        let m = self.names.meanings.len() as u32 - 1;
                        self.interned.insert((at, spelling), m);
                        m
                    }
                };
                let generic = self.generics.iter().any(|gs| gs.iter().any(|g| g == name));
                // [effect-var] The innermost declaration naming it decides.
                let effect_var = generic
                    && self.generics.iter().zip(&self.effect_vars).rev().find(|(gs, _)| gs.iter().any(|g| g == name)).is_some_and(|(_, es)| es.iter().any(|e| e == name));
                let idx = self.names.refs.len() as u32;
                self.names.refs.push(Written { addr, file: self.file, at, span: t.name.span, generic, effect_var, name: spelling, meaning });
                self.names.by_addr.insert(addr, idx);
                if t.name.span.is_numbered() {
                    let key = (t.name.span.file, t.name.span.node, t.name.span, spelling);
                    match self.names.by_copy.get(&key).copied() {
                        None => {
                            self.names.by_copy.insert(key, Some(idx));
                        }
                        Some(Some(prev)) => {
                            let prev = self.names.refs[prev as usize];
                            let agree = prev.generic == generic && prev.effect_var == effect_var
                                && self.names.meanings[prev.meaning as usize].same(&self.names.meanings[meaning as usize]);
                            if !agree {
                                self.names.by_copy.insert(key, None);
                            }
                        }
                        Some(None) => {}
                    }
                }
            }
            fn enter_generics(&mut self, generics: &[Ident], canbe: &[(Ident, TypeRef)]) {
                let gs = generics.iter().chain(canbe.iter().map(|(g, _)| g)).map(|g| g.name.clone()).collect();
                self.generics.push(gs);
                self.effect_vars.push(Vec::new());
            }
            fn enter_effect_generics(&mut self, effect_generics: &[Ident]) {
                if let Some(top) = self.effect_vars.last_mut() {
                    top.extend(effect_generics.iter().map(|g| g.name.clone()));
                }
            }
            fn leave_generics(&mut self) {
                self.generics.pop();
                self.effect_vars.pop();
            }
        }
        let mut names = Names::default();
        let mut interned = HashMap::new();
        let (mut types_t, mut values_t) = (std::time::Duration::ZERO, std::time::Duration::ZERO);
        for (file, module) in modules.iter().enumerate() {
            let mut walk = Walk {
                file: file as u32,
                names: &mut names,
                interned: &mut interned,
                scopes,
                template_refs,
                generics: Vec::new(),
                effect_vars: Vec::new(),
            };
            let t0 = std::time::Instant::now();
            crate::check_cache::walk_module(&mut walk, module.borrow());
            types_t += t0.elapsed();
        }
        // Every file's types first: a body's effects reach other files'
        // declarations (an effect's prerequisites, an alias's members).
        for (file, module) in modules.iter().enumerate() {
            let t0 = std::time::Instant::now();
            values::Binder::new(&mut names, &mut interned, scopes, templates, file as u32).module(module.borrow());
            values_t += t0.elapsed();
        }
        crate::timing::report("      names: types", types_t);
        crate::timing::report("      names: values", values_t);
        names
    }

    /// [name-table] The value-position name `id` is, by its address; for a
    /// copy, by its node, span and name, when that settles it.
    pub fn value(&self, id: &Ident) -> Option<&WrittenValue> {
        let idx = match self.value_by_addr.get(&(id as *const Ident as usize)) {
            Some(i) => *i,
            None if id.span.is_numbered() => {
                let spelling = *self.spelling_ids.get(id.name.as_str())?;
                (*self.value_by_copy.get(&(id.span.file, id.span.node, id.span, spelling))?)?
            }
            None => return None,
        };
        self.values.get(idx as usize)
    }

    /// What a value-position name means as a declaration where it is
    /// written (its binding, if any, comes first: [`WrittenValue::local`]).
    pub fn value_meaning(&self, w: &WrittenValue) -> &Meaning<'p> {
        &self.meanings[w.meaning as usize]
    }

    /// [effect-available] The members of the value name's spelling that the
    /// effects its body has from what is written add to its scope's own
    /// (`Meaning::members`).
    pub fn available_members(&self, w: &WrittenValue) -> &[(&'p EffectDecl, &'p salvo_syntax::ast::FnDecl)] {
        self.member_sets.get(w.available as usize).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// The name a value-position name wrote.
    pub fn value_name(&self, w: &WrittenValue) -> &str {
        &self.spellings[w.name as usize]
    }

    /// Every value-position name, in walk order, file by file.
    pub fn values(&self) -> &[WrittenValue] {
        &self.values
    }

    /// The index of a meaning with nothing in it, made on first need.
    fn empty_meaning(&mut self) -> u32 {
        if let Some(i) = self.empty {
            return i;
        }
        self.meanings.push(Meaning::default());
        let i = self.meanings.len() as u32 - 1;
        self.empty = Some(i);
        i
    }

    pub(crate) fn spell(&mut self, name: &str) -> u32 {
        if let Some(id) = self.spelling_ids.get(name) {
            return *id;
        }
        let id = self.spellings.len() as u32;
        self.spellings.push(name.to_string());
        self.spelling_ids.insert(name.to_string(), id);
        id
    }

    /// The written reference `r` is, by its address; for a copy, by its
    /// node, span and name, when that settles it.
    pub fn written(&self, r: &TypeRef) -> Option<&Written> {
        let idx = match self.by_addr.get(&(r as *const TypeRef as usize)) {
            Some(i) => *i,
            None if r.name.span.is_numbered() => {
                let spelling = *self.spelling_ids.get(r.name.name.as_str())?;
                (*self.by_copy.get(&(r.name.span.file, r.name.span.node, r.name.span, spelling))?)?
            }
            None => return None,
        };
        self.refs.get(idx as usize)
    }

    /// The written reference at `r`'s address only: what a question about
    /// the tree itself (which file wrote this node) asks.
    pub fn original(&self, r: &TypeRef) -> Option<&Written> {
        self.refs.get(*self.by_addr.get(&(r as *const TypeRef as usize))? as usize)
    }

    /// What a written reference means.
    pub fn meaning(&self, w: &Written) -> &Meaning<'p> {
        &self.meanings[w.meaning as usize]
    }

    /// The name a written reference wrote.
    pub fn name(&self, w: &Written) -> &str {
        &self.spellings[w.name as usize]
    }

    /// Every written reference, in walk order, file by file.
    pub fn refs(&self) -> &[Written] {
        &self.refs
    }
}

/// [comptime-hygiene] The expansions of a stamp that are its template's own
/// text: the template copied whole, and every unrolled copy of a part of it
/// — not what the stamp made up (a clause member's signature, an unrolled
/// `when`'s tests) or substituted (a field's type keeps its declaration's).
pub fn template_text(stamp: &salvo_syntax::ast::Stamp) -> std::collections::HashSet<u32> {
    use salvo_syntax::ast::ExpansionKind;
    let root_is_template = |mut id: u32| loop {
        let Some(e) = stamp.expansion(id) else { return false };
        match e.kind {
            ExpansionKind::Template => return true,
            ExpansionKind::Copy { .. } if e.parent != 0 => id = e.parent,
            _ => return false,
        }
    };
    stamp
        .expansions
        .iter()
        .filter(|e| matches!(e.kind, ExpansionKind::Template | ExpansionKind::Copy { .. }) && root_is_template(e.id))
        .map(|e| e.id)
        .collect()
}
