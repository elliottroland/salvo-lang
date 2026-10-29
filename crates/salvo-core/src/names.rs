//! [name-table] The resolved form of every written type-position name
//! (PIPELINE.md C1.2): what each name a program writes where a type, a
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
//! checker, the IR builder and the emitters clone types freely) by its node
//! [node-id], its span and its written name — what a copy keeps — when only
//! one written reference has them, or only references meaning the same.

use std::collections::HashMap;

use salvo_syntax::ast::{EffectAliasDecl, EffectDecl, FnsDecl, HandlerDecl, Ident, Module, QualifierDecl, StructDecl, TypeDecl, TypeRef};
use salvo_syntax::Span;

use crate::resolve::{DefSite, ModuleScope};

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
    fn same(&self, other: &Meaning<'p>) -> bool {
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
    }
}

/// [comptime-hygiene] The scope a reference is resolved in: its file's, or
/// the stamp scope of the comptime-stamped body it is in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum At {
    File(u32),
    Stamp(u32),
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
    /// A copy's way in: by node, span and name — `None` where two written
    /// references there mean different things.
    by_copy: HashMap<(u32, Span, u32), Option<u32>>,
}

impl<'p> Names<'p> {
    /// Resolves every written reference of `modules`, each in `scopes`' scope
    /// of its file — or the stamp scope `stamped` gives it, by address.
    pub fn build<M: std::borrow::Borrow<Module>>(
        modules: &[M],
        scopes: &[ModuleScope<'p>],
        stamp_scopes: &[ModuleScope<'p>],
        stamped: &HashMap<usize, usize>,
    ) -> Names<'p> {
        use salvo_syntax::visit::Visitor;
        struct Walk<'w, 'p> {
            file: u32,
            names: &'w mut Names<'p>,
            interned: &'w mut HashMap<(At, u32), u32>,
            scopes: &'w [ModuleScope<'p>],
            stamp_scopes: &'w [ModuleScope<'p>],
            stamped: &'w HashMap<usize, usize>,
            generics: Vec<Vec<String>>,
        }
        impl<'p> Visitor for Walk<'_, 'p> {
            fn visit_type_ref(&mut self, t: &TypeRef) {
                let addr = t as *const TypeRef as usize;
                let at = match self.stamped.get(&addr) {
                    Some(&i) => At::Stamp(i as u32),
                    None => At::File(self.file),
                };
                let name = t.name.name.as_str();
                let spelling = self.names.spell(name);
                let meaning = match self.interned.get(&(at, spelling)) {
                    Some(m) => *m,
                    None => {
                        let scope = match at {
                            At::File(f) => &self.scopes[f as usize],
                            At::Stamp(i) => &self.stamp_scopes[i as usize],
                        };
                        self.names.meanings.push(Meaning::of(scope, name));
                        let m = self.names.meanings.len() as u32 - 1;
                        self.interned.insert((at, spelling), m);
                        m
                    }
                };
                let generic = self.generics.iter().any(|gs| gs.iter().any(|g| g == name));
                let idx = self.names.refs.len() as u32;
                self.names.refs.push(Written { addr, file: self.file, at, span: t.name.span, generic, name: spelling, meaning });
                self.names.by_addr.insert(addr, idx);
                if t.name.span.is_numbered() {
                    let key = (t.name.span.node, t.name.span, spelling);
                    match self.names.by_copy.get(&key).copied() {
                        None => {
                            self.names.by_copy.insert(key, Some(idx));
                        }
                        Some(Some(prev)) => {
                            let prev = self.names.refs[prev as usize];
                            let agree = prev.generic == generic
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
            }
            fn leave_generics(&mut self) {
                self.generics.pop();
            }
        }
        let mut names = Names::default();
        let mut interned = HashMap::new();
        for (file, module) in modules.iter().enumerate() {
            let mut walk = Walk {
                file: file as u32,
                names: &mut names,
                interned: &mut interned,
                scopes,
                stamp_scopes,
                stamped,
                generics: Vec::new(),
            };
            crate::check_cache::walk_module(&mut walk, module.borrow());
        }
        names
    }

    fn spell(&mut self, name: &str) -> u32 {
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
                (*self.by_copy.get(&(r.name.span.node, r.name.span, spelling))?)?
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
