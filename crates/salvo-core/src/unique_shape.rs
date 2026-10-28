//! [unique-shape] **A function of a unique shape on a type is declared in the
//! type's own file** (user decision O1-D, 2026-10-27; OPERATOR_OVERLOAD.md
//! "D's details"). The operators are names (`a == b` is `eq(a, b)`, `"${x}"`
//! is `to_str(x)`, `for x in c` is `next(iter(c))`, O1-B), resolved exactly
//! as those calls are; this placement rule is what makes the name-based call
//! *the* type's operator: for a concrete type there is never a second
//! candidate, so no import or local declaration can change what `==` means
//! on it.
//!
//! A per-declaration check, an interface fact of the `resolve` phase: each
//! fn declaration (top-level, hoisted from a struct body, stamped, an effect
//! member, a qualifier's fn) whose name, explicit arity and — for `eq`,
//! `cmp` and `next` — return type match a row of [`SHAPES`] is *on* the base
//! type of its first explicit parameter, qualifiers stripped (D-2), and that
//! type's declaring file (D-1) is the only file it may be declared in. A
//! second one anywhere else is an error at its declaration, used or not.
//!
//! * Exempt (D-4): a `fns` group's member signatures (they state a shape,
//!   they implement nothing), comptime templates (what they stamp lands on
//!   the type and is checked as a written fn there), and a handler's members
//!   (each implements its effect's member, which is the declaration checked).
//! * Structural types have a std home (D-5): tuples and fn types
//!   `std.core.basic`, arrays `std.core.array`, anonymous unions (and `T?`)
//!   `std.core.basic`; an `eff[E]` handle is the type `Addr` and lives where
//!   `Addr` is declared. A literal type is on its base. A non-union alias
//!   names the type it expands to ([type-alias]: aliases are structural); a
//!   union alias is a declared union, its own home (O4).
//! * A bare type parameter has no home, so a shape on one is refused: the
//!   catch-all `fn to_str<T>(x: T)`.

use std::collections::{HashMap, HashSet};

use salvo_syntax::ast::{FnDecl, Item, Type, TypeDecl};

use crate::diag::FileDiagnostic;
use crate::resolve::ModuleScope;
use crate::source::{ModulePath, SourceFile};

/// What a shape's return type must be.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ret {
    /// Anything.
    Any,
    /// std's scalar of this name.
    Scalar(&'static str),
    /// `Emitted T | Finished`, for any `T`.
    Next,
}

/// [unique-shape] The class (D-3, D-6): name, explicit parameters (implicit
/// ones not counted), return. Adding a member is a row here.
const SHAPES: &[(&str, usize, Ret)] = &[
    ("eq", 2, Ret::Scalar("Bool")),
    ("cmp", 2, Ret::Scalar("Int")),
    ("plus", 2, Ret::Any),
    ("minus", 2, Ret::Any),
    ("times", 2, Ret::Any),
    ("divide", 2, Ret::Any),
    ("pow", 2, Ret::Any),
    ("to_str", 1, Ret::Any),
    ("iter", 1, Ret::Any),
    ("next", 1, Ret::Next),
];

/// The surface syntax a shape answers, for the message, and whether it reads
/// as a plural (`` `<`, `>`, `<=` and `>=` mean ``).
fn spelled(name: &str) -> (&'static str, bool) {
    match name {
        "eq" => ("`==` and `!=`", true),
        "cmp" => ("`<`, `>`, `<=` and `>=`", true),
        "plus" => ("`+`", false),
        "minus" => ("`-`", false),
        "times" => ("`*`", false),
        "divide" => ("`/`", false),
        "pow" => ("`**`", false),
        "to_str" => ("interpolation (`${…}`)", false),
        _ => ("`for`", false),
    }
}

/// [unique-shape] Which unique shape `f` has, if any: a row's name, its
/// explicit arity, and its return type where the row names one
/// (`written_ret_is` says whether a written type is std's scalar of a name).
/// A type-named fn (`fn Mut List<T>(…)`) is a constructor, never a shape.
fn shape_of(f: &FnDecl, written_ret_is: &dyn Fn(&Type, &str) -> bool) -> Option<&'static str> {
    let (name, arity, ret) = SHAPES.iter().find(|(n, _, _)| *n == f.name.name)?;
    if f.type_name.is_some() || f.params.iter().filter(|p| !p.implicit).count() != *arity {
        return None;
    }
    let fits = match ret {
        Ret::Any => true,
        Ret::Scalar(s) => f.return_type.as_ref().is_some_and(|t| written_ret_is(t, s)),
        Ret::Next => f.return_type.as_ref().is_some_and(is_next_return),
    };
    fits.then_some(*name)
}

/// `Emitted T | Finished`, in either order, `T` anything.
fn is_next_return(t: &Type) -> bool {
    let Type::Union { arms, .. } = t else { return false };
    if arms.len() != 2 {
        return false;
    }
    let emitted = |a: &Type| match a {
        Type::Named { qualifiers, .. } | Type::QualifiedGroup { qualifiers, .. } => {
            qualifiers.iter().any(|q| q.name.name == "Emitted")
        }
        _ => false,
    };
    let finished = |a: &Type| {
        matches!(a, Type::Named { qualifiers, base } if qualifiers.is_empty() && base.name.name == "Finished")
    };
    (emitted(&arms[0]) && finished(&arms[1])) || (emitted(&arms[1]) && finished(&arms[0]))
}

/// Where a shape on a type may be declared.
enum Home {
    /// The file declaring the type, how the message names the type, and
    /// whether the type is declared there (or is a structural type std gives
    /// that file as its home).
    File(usize, String, bool),
    /// A bare type parameter: nowhere.
    Nowhere(String),
    /// Not resolvable here (an unresolved name is reported elsewhere, and a
    /// program without std has no structural homes): not checked.
    Unknown,
}

/// The std files structural types live in [unique-shape] (D-5).
struct StdHomes {
    basic: Option<usize>,
    array: Option<usize>,
}

struct Ctx<'a, 'p> {
    files: &'a [SourceFile],
    scopes: &'a [ModuleScope<'p>],
    decl_files: &'a HashMap<usize, usize>,
    homes: StdHomes,
}

impl Ctx<'_, '_> {
    /// [unique-shape] The home of `ty` written in `file` with the type
    /// parameters `generics` in scope (D-2, D-5).
    fn home(&self, ty: &Type, file: usize, generics: &[&str], depth: u8) -> Home {
        let at = |f: Option<usize>, what: &str| match f {
            Some(f) => Home::File(f, what.to_string(), false),
            None => Home::Unknown,
        };
        match ty {
            Type::Named { base, .. } => {
                let name = base.name.name.as_str();
                if generics.contains(&name) {
                    return Home::Nowhere(name.to_string());
                }
                let scope = &self.scopes[file];
                // The checker expands an alias before it looks a struct up
                // (`lower_base_ref`), so a file's own alias beside an
                // imported struct of its name is the alias.
                if let Some(t) = scope.type_aliases.get(name) {
                    return self.alias_home(t, name, depth);
                }
                if let Some(f) = scope.struct_files.get(name) {
                    return Home::File(*f, format!("`{name}`"), true);
                }
                if let Some(f) = scope.opaque_type_files.get(name) {
                    // [eff-handle] `eff[E]` is the type `Addr`, which nobody
                    // writes: named as the handle it is.
                    if name == crate::check::ADDR_TYPE {
                        return Home::File(*f, "`eff[…]` handles".to_string(), false);
                    }
                    return Home::File(*f, format!("`{name}`"), true);
                }
                Home::Unknown
            }
            Type::QualifiedGroup { base, .. } => self.home(base, file, generics, depth),
            Type::Literal { value, .. } => {
                let base = value.base();
                let scope = &self.scopes[file];
                match scope.opaque_type_files.get(base) {
                    Some(f) => Home::File(*f, format!("`{base}`"), true),
                    None => Home::Unknown,
                }
            }
            Type::Tuple { .. } => at(self.homes.basic, "tuples"),
            Type::Fn { .. } => at(self.homes.basic, "fn types"),
            Type::Array { .. } => at(self.homes.array, "arrays (`T[]`)"),
            Type::Union { .. } | Type::Nullable { .. } => at(self.homes.basic, "anonymous unions"),
            Type::Spread { .. } => Home::Unknown,
        }
    }

    /// A union alias is a declared union, its own home (O4); any other alias
    /// is structural [type-alias] and names what it expands to, resolved in
    /// the alias's own file.
    fn alias_home(&self, t: &TypeDecl, name: &str, depth: u8) -> Home {
        let Some(&alias_file) = self.decl_files.get(&(t as *const TypeDecl as usize)) else {
            return Home::Unknown;
        };
        match &t.alias {
            Some(Type::Union { .. } | Type::Nullable { .. }) | None => {
                Home::File(alias_file, format!("`{name}`"), true)
            }
            Some(target) if depth < 8 => {
                let generics: Vec<&str> = t.generics.iter().map(|g| g.name.as_str()).collect();
                match self.home(target, alias_file, &generics, depth + 1) {
                    // `type Id<T> = T`: what it names depends on the use.
                    Home::Nowhere(_) => Home::Unknown,
                    h => h,
                }
            }
            Some(_) => Home::Unknown,
        }
    }

    /// Whether a written return type is std's scalar `name`.
    fn is_scalar(&self, file: usize, ty: &Type, name: &str) -> bool {
        let Type::Named { qualifiers, base } = ty else { return false };
        if !qualifiers.is_empty() || base.name.name != name {
            return false;
        }
        match self.homes.basic {
            Some(basic) => self.scopes[file].opaque_type_files.get(name) == Some(&basic),
            None => true,
        }
    }

    fn file_text(&self, file: usize) -> String {
        let f = &self.files[file];
        format!("`{}` (module `{}`)", f.name, f.module)
    }

    /// [unique-shape] Check one declaration; `outer` are the type parameters
    /// of the declaration containing it (an effect's, a qualifier's).
    fn check(&self, file: usize, f: &FnDecl, outer: &[&str], out: &mut Vec<FileDiagnostic>, refused: &mut HashSet<usize>) {
        if f.compfn.as_ref().is_some_and(|c| c.bound.is_some()) {
            return;
        }
        let Some(shape) = shape_of(f, &|t, s| self.is_scalar(file, t, s)) else { return };
        let Some(first) = f.params.iter().find(|p| !p.implicit) else { return };
        let mut generics: Vec<&str> = outer.to_vec();
        generics.extend(f.generics.iter().map(|g| g.name.as_str()));
        generics.extend(f.generic_canbe.iter().map(|(g, _)| g.name.as_str()));
        let explicit: Vec<String> =
            f.params.iter().filter(|p| !p.implicit).map(|p| p.ty.to_string()).collect();
        let sig = format!("{}({})", f.name.name, explicit.join(", "));
        let (ops, plural) = spelled(shape);
        let (call, mean) = if plural { ("call", "mean") } else { ("calls", "means") };
        let message = match self.home(&first.ty, file, &generics, 0) {
            Home::Unknown => return,
            Home::File(home, ..) if home == file => return,
            Home::File(home, what, declared) => {
                let place = if declared {
                    format!("the file that declares {what}")
                } else {
                    format!("the file std names as the home of {what}")
                };
                // A std file is not the program's to edit: the remedy is the name.
                let remedy = if self.files[home].is_std && !self.files[file].is_std {
                    "Give this one another name and pass it explicitly"
                } else {
                    "Move this one into that file, or give it another name and pass it explicitly"
                };
                format!(
                    "`{sig}` has the shape of `{shape}` on {what}, which {ops} {call}, and a \
                     function of that shape is declared only in {place}, {}: so nothing \
                     elsewhere can change what {ops} {mean} on {what}. {remedy} where it is \
                     wanted (`{shape} = by_name`) [unique-shape]",
                    self.file_text(home)
                )
            }
            Home::Nowhere(param) => format!(
                "`{sig}` has the shape of `{shape}` on the type parameter `{param}`, which \
                 has no declaring file: it would be the `{shape}` of every type at once, \
                 which {ops} {call}. A function of a unique shape is declared in its \
                 type's own file; give this one another name and pass it explicitly where \
                 it is wanted (`{shape} = by_name`) [unique-shape]"
            ),
        };
        out.push(FileDiagnostic::error(file, f.name.span, message));
        refused.insert(f as *const FnDecl as usize);
    }
}

/// [unique-shape] Every declaration of a unique shape outside its type's
/// declaring file, as errors at the declaration (the `resolve` phase), and
/// the refused declarations by address: a refused one is a candidate nowhere
/// (one mistake, one diagnostic; ROADMAP 0ag, closed in B3.3c).
pub fn check_program<'p>(
    files: &[SourceFile],
    modules: &[&'p salvo_syntax::ast::Module],
    scopes: &[ModuleScope<'p>],
    decl_files: &HashMap<usize, usize>,
    errors: &mut Vec<FileDiagnostic>,
) -> HashSet<usize> {
    let mut refused: HashSet<usize> = HashSet::new();
    let home_of = |path: ModulePath| {
        files.iter().position(|f| f.module == path && !f.is_test)
    };
    let ctx = Ctx {
        files,
        scopes,
        decl_files,
        homes: StdHomes {
            basic: home_of(ModulePath::std(&["core", "basic"])),
            array: home_of(ModulePath::std(&["core", "array"])),
        },
    };
    for (file, ast) in modules.iter().enumerate() {
        if file >= scopes.len() {
            break;
        }
        for item in &ast.items {
            match item {
                Item::Fn(f) => ctx.check(file, f, &[], errors, &mut refused),
                Item::Effect(e) => {
                    let outer: Vec<&str> = e.generics.iter().map(|g| g.name.as_str()).collect();
                    for f in &e.fns {
                        ctx.check(file, f, &outer, errors, &mut refused);
                    }
                }
                Item::Qualifier(q) => {
                    let outer: Vec<&str> = q.generics.iter().map(|g| g.name.as_str()).collect();
                    for f in &q.fns {
                        ctx.check(file, f, &outer, errors, &mut refused);
                    }
                }
                _ => {}
            }
        }
    }
    refused
}
