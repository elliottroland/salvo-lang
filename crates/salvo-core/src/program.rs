//! The parsed program and its global symbol tables.
//!
//! For now Salvo uses a flat global namespace: imports are parsed but
//! cross-module name resolution simply merges every module's declarations.
//! Proper per-module scoping (honoring `import` and aliases) comes with the
//! full resolver.

use std::collections::HashMap;

use salvo_syntax::ast::{
    DefineFn, DefineHandler, DefineType, EffectDecl, FnDecl, HandlerDecl, Item, Module,
    QualifierDecl, StructDecl, TypeDecl,
};

use crate::source::{SourceFile, SourceKind};

/// A parsed compilation: one AST per source file, in `SourceSet` order.
pub struct Program {
    pub files: Vec<SourceFile>,
    pub modules: Vec<Module>,
}

/// A single parsed unit (file + AST).
pub struct Unit<'p> {
    pub file: &'p SourceFile,
    pub ast: &'p Module,
}

impl Program {
    pub fn units(&self) -> impl Iterator<Item = Unit<'_>> {
        self.files
            .iter()
            .zip(&self.modules)
            .map(|(file, ast)| Unit { file, ast })
    }
}

/// Global symbol tables, keyed by simple name.
#[derive(Default)]
pub struct Symbols<'p> {
    /// Top-level functions (including `external fn` signatures), by name.
    /// Multiple entries are overloads.
    pub fns: HashMap<&'p str, Vec<&'p FnDecl>>,
    pub structs: HashMap<&'p str, &'p StructDecl>,
    pub effects: HashMap<&'p str, &'p EffectDecl>,
    pub handlers: HashMap<&'p str, &'p HandlerDecl>,
    pub qualifiers: HashMap<&'p str, &'p QualifierDecl>,
    pub type_aliases: HashMap<&'p str, &'p TypeDecl>,
    /// `internal type` declarations (mapped natively by each backend).
    pub internal_types: HashMap<&'p str, &'p TypeDecl>,
    /// `external type` declarations (mapped via `define type`).
    pub external_types: HashMap<&'p str, &'p TypeDecl>,
    /// Backend `define fn` templates, by function name.
    pub define_fns: HashMap<&'p str, Vec<&'p DefineFn>>,
    /// Backend `define type` templates, by type name.
    pub define_types: HashMap<&'p str, &'p DefineType>,
    /// Backend `define handler` templates, by handler name.
    pub define_handlers: HashMap<&'p str, &'p DefineHandler>,
    /// Effect-member function name -> owning effect name.
    pub effect_of_fn: HashMap<&'p str, &'p str>,
}

impl<'p> Symbols<'p> {
    /// Collects symbols from every module of the program.
    pub fn collect(program: &'p Program) -> Self {
        let mut symbols = Symbols::default();
        for unit in program.units() {
            let is_define_file = unit.file.kind == SourceKind::BackendDefine;
            for item in &unit.ast.items {
                match item {
                    Item::Fn(f) => symbols.fns.entry(&f.name.name).or_default().push(f),
                    Item::Struct(s) => {
                        symbols.structs.insert(&s.name.name, s);
                    }
                    Item::Effect(e) => {
                        symbols.effects.insert(&e.name.name, e);
                        for f in &e.fns {
                            symbols.effect_of_fn.insert(&f.name.name, &e.name.name);
                        }
                    }
                    Item::Handler(h) => {
                        symbols.handlers.insert(&h.name.name, h);
                    }
                    Item::Qualifier(q) => {
                        symbols.qualifiers.insert(&q.name.name, q);
                    }
                    Item::Type(t) => {
                        use salvo_syntax::ast::BackingMod;
                        match (t.backing, &t.alias) {
                            (Some(BackingMod::Internal), _) => {
                                symbols.internal_types.insert(&t.name.name, t);
                            }
                            (Some(BackingMod::External), _) => {
                                symbols.external_types.insert(&t.name.name, t);
                            }
                            (None, Some(_)) => {
                                symbols.type_aliases.insert(&t.name.name, t);
                            }
                            (None, None) => {
                                // A bodiless `type` outside std: treat like
                                // an external type.
                                symbols.external_types.insert(&t.name.name, t);
                            }
                        }
                    }
                    Item::DefineFn(d) if is_define_file => {
                        symbols
                            .define_fns
                            .entry(&d.sig.name.name)
                            .or_default()
                            .push(d);
                    }
                    Item::DefineType(d) if is_define_file => {
                        symbols.define_types.insert(&d.name.name, d);
                    }
                    Item::DefineHandler(d) if is_define_file => {
                        symbols.define_handlers.insert(&d.name.name, d);
                    }
                    Item::DefineFn(_) | Item::DefineType(_) | Item::DefineHandler(_) => {}
                    Item::Import(_) => {}
                }
            }
        }
        symbols
    }

    /// Finds the best `fn` overload for a call with `arg_count` arguments.
    /// Falls back to the first overload when none matches exactly (variadic
    /// functions accept any remaining arity).
    pub fn resolve_fn(&self, name: &str, arg_count: usize) -> Option<&'p FnDecl> {
        let overloads = self.fns.get(name)?;
        overloads
            .iter()
            .find(|f| {
                let required = f.params.iter().filter(|p| !p.variadic).count();
                let variadic = f.params.iter().any(|p| p.variadic);
                if variadic {
                    arg_count >= required
                } else {
                    arg_count == required
                }
            })
            .or_else(|| overloads.first())
            .copied()
    }

    /// Finds the best `define fn` template for a call with `arg_count`
    /// arguments.
    pub fn resolve_define_fn(&self, name: &str, arg_count: usize) -> Option<&'p DefineFn> {
        let overloads = self.define_fns.get(name)?;
        overloads
            .iter()
            .find(|d| {
                let required = d.sig.params.iter().filter(|p| !p.variadic).count();
                let variadic = d.sig.params.iter().any(|p| p.variadic);
                if variadic {
                    arg_count >= required
                } else {
                    arg_count == required
                }
            })
            .or_else(|| overloads.first())
            .copied()
    }
}
