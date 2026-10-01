//! [name-camel] The camel-case spelling of a Salvo value name, and the rule
//! that keeps it unambiguous.
//!
//! Generated Kotlin follows Kotlin's conventions (user decision 2026-10-01,
//! ABI.md D6): `read_to_str` is `readToStr`, `data_type` is `dataType`. The
//! mapping is defined here, beside the checker, because two Salvo names that
//! map to one camel-case name are an error in **all** Salvo code — a
//! project's validity must not depend on the backend it builds for — and the
//! checker and the Kotlin emitter must agree on what a clash is.

/// The camel-case spelling of `name`: each `_` followed by a lowercase ASCII
/// letter is dropped and the letter uppercased, so `read_to_str` is
/// `readToStr`.
///
/// Everything else is kept as written, which keeps the mapping injective
/// over the names people write and leaves the compiler's own names alone:
/// - a name not starting with a lowercase ASCII letter (a type, a
///   qualifier, an internal `__p`) is unchanged;
/// - the part from the first `__` on is kept verbatim (an overload suffix,
///   `close__2`, becomes `close__2`; `read_to__2` becomes `readTo__2`);
/// - an `_` before a digit, an uppercase letter or the end is kept (`a_1`,
///   `x_`, `foo_Bar`).
pub fn camel(name: &str) -> String {
    if !name.starts_with(|c: char| c.is_ascii_lowercase()) {
        return name.to_string();
    }
    let (head, tail) = match name.find("__") {
        Some(i) => name.split_at(i),
        None => (name, ""),
    };
    let mut out = String::with_capacity(name.len());
    let mut chars = head.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '_' {
            if let Some(&next) = chars.peek() {
                if next.is_ascii_lowercase() {
                    out.push(next.to_ascii_uppercase());
                    chars.next();
                    continue;
                }
            }
        }
        out.push(c);
    }
    out.push_str(tail);
    out
}

/// [name-camel] The first pair of distinct names in `names` that share a
/// camel-case spelling, in the order given (the second is the one to report).
pub fn camel_clash<'a, I>(names: I) -> Option<(&'a str, &'a str)>
where
    I: IntoIterator<Item = &'a str>,
{
    let mut seen: std::collections::HashMap<String, &'a str> = std::collections::HashMap::new();
    for name in names {
        let key = camel(name);
        match seen.get(&key) {
            Some(first) if *first != name => return Some((first, name)),
            Some(_) => {}
            None => {
                seen.insert(key, name);
            }
        }
    }
    None
}

/// [name-camel] Every pair of distinct names in one Kotlin scope that share a
/// camel-case spelling, as `(span of the second, message)`. The scopes are the
/// ones the Kotlin emitter maps into one namespace:
/// - a module's top-level fns (one Kotlin package);
/// - a struct's fields; an effect's members;
/// - a handler's constructor parameters and state together, and its members;
/// - every fn, member and lambda's parameters with the names the body binds
///   (`let`, `for`, `is` bindings, lambda parameters), as one set — Kotlin
///   would refuse or shadow a second spelling the Salvo program means apart.
pub fn module_clashes(module: &salvo_syntax::ast::Module) -> Vec<(salvo_syntax::Span, String)> {
    use salvo_syntax::ast::{FnDecl, Ident, Item};
    let mut out = Vec::new();
    let mut report = |what: &str, ids: &[&Ident]| {
        let names: Vec<&str> = ids.iter().map(|i| i.name.as_str()).collect();
        if let Some((first, second)) = camel_clash(names.iter().copied()) {
            let span = ids.iter().filter(|i| i.name == second).map(|i| i.span).next().unwrap_or_default();
            out.push((
                span,
                format!(
                    "`{second}` and `{first}` are both `{}` in camel case, and {what} cannot hold \
                     both: the Kotlin backend writes camel case — rename one [name-camel]",
                    camel(second)
                ),
            ));
        }
    };
    let fn_scope = |f: &FnDecl, extra: &[&Ident], report: &mut dyn FnMut(&str, &[&Ident])| {
        let mut ids: Vec<Ident> = f.params.iter().map(|p| p.name.clone()).collect();
        ids.extend(extra.iter().map(|i| (*i).clone()));
        let mut clone = f.clone();
        let mut binds = Bindings(Vec::new());
        salvo_syntax::visit_mut::walk_fn(&mut binds, &mut clone);
        ids.extend(binds.0);
        let refs: Vec<&Ident> = ids.iter().collect();
        report(&format!("the scope of `{}`", f.name.name), &refs);
    };
    let top: Vec<&Ident> = module
        .items
        .iter()
        .filter_map(|i| match i {
            Item::Fn(f) => Some(&f.name),
            _ => None,
        })
        .collect();
    report("one module", &top);
    for item in &module.items {
        match item {
            Item::Fn(f) => fn_scope(f, &[], &mut report),
            Item::Struct(s) => {
                let fields: Vec<&Ident> = s.fields.iter().map(|f| &f.name).collect();
                report(&format!("struct `{}`", s.name.name), &fields);
                for f in &s.fns {
                    fn_scope(f, &[], &mut report);
                }
            }
            Item::Effect(e) => {
                let members: Vec<&Ident> = e.fns.iter().map(|f| &f.name).collect();
                report(&format!("effect `{}`", e.name.name), &members);
            }
            Item::Handler(h) => {
                let state: Vec<&Ident> =
                    h.params.iter().map(|p| &p.name).chain(h.state.iter().map(|f| &f.name)).collect();
                report(&format!("handler `{}`", h.name.name), &state);
                let members: Vec<&Ident> = h.fns.iter().map(|f| &f.name).collect();
                report(&format!("handler `{}`", h.name.name), &members);
                for f in h.fns.iter().chain(h.init.iter()) {
                    fn_scope(f, &state, &mut report);
                }
            }
            Item::Qualifier(q) => {
                for f in &q.fns {
                    fn_scope(f, &[], &mut report);
                }
            }
            _ => {}
        }
    }
    out
}

/// The names a fn body binds: `let` patterns, `for` patterns, `is` and
/// `when` bindings, lambda parameters.
struct Bindings(Vec<salvo_syntax::ast::Ident>);

impl Bindings {
    fn pattern(&mut self, p: &salvo_syntax::ast::Pattern) {
        use salvo_syntax::ast::Pattern;
        match p {
            Pattern::Ident(id) => self.0.push(id.clone()),
            Pattern::Tuple { elems, .. } => elems.iter().for_each(|e| self.pattern(e)),
            Pattern::Struct { fields, .. } => self.0.extend(fields.iter().map(|f| f.binding.clone())),
        }
    }
}

impl salvo_syntax::visit_mut::MutVisitor for Bindings {
    fn visit_stmt(&mut self, stmt: &mut salvo_syntax::ast::Stmt) {
        if let salvo_syntax::ast::Stmt::Let { pattern, .. } = stmt {
            self.pattern(pattern);
        }
    }
    fn visit_expr(&mut self, expr: &mut salvo_syntax::ast::Expr) {
        use salvo_syntax::ast::Expr;
        match expr {
            Expr::For { pattern, .. } => self.pattern(pattern),
            Expr::Is { binding: Some(b), .. } | Expr::Widen { binding: Some(b), .. } => self.0.push(b.clone()),
            Expr::Lambda { params, .. } => self.0.extend(params.iter().map(|p| p.name.clone())),
            Expr::When { branches, .. } => {
                self.0.extend(branches.iter().filter_map(|b| b.binding.clone()));
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // [name-camel] The mapping: separators before a lowercase letter go, the
    // compiler's own names and anything that would lose information stay.
    #[test]
    fn camel_case_keeps_what_it_cannot_map_safely() {
        for (from, to) in [
            ("read_to_str", "readToStr"),
            ("data_type", "dataType"),
            ("x", "x"),
            ("fooBar", "fooBar"),
            ("a_1", "a_1"),
            ("x_", "x_"),
            ("foo_Bar", "foo_Bar"),
            ("close__2", "close__2"),
            ("read_to__2", "readTo__2"),
            ("__p", "__p"),
            ("_unused", "_unused"),
            ("Point", "Point"),
            ("Foo_bar", "Foo_bar"),
        ] {
            assert_eq!(camel(from), to, "{from}");
        }
        assert_eq!(camel_clash(["foo_bar", "x", "fooBar"]), Some(("foo_bar", "fooBar")));
        assert_eq!(camel_clash(["foo_bar", "foo_bar", "x"]), None);
    }

    // [name-camel] Clashes are found per Kotlin scope — a module's fns, a
    // struct's fields, a fn with its bindings — and nowhere else: one name in
    // two scopes, or the same name twice, is fine.
    #[test]
    fn clashes_are_found_per_scope() {
        let src = "struct P { data_type: Int, dataType: Int }\n\
                   fn read_to(x: Int) [] -> Int {\n    let read_x = 1\n    let readX = 2\n    return x\n}\n\
                   fn readTo(y: Str) [] -> Int {\n    return 1\n}\n\
                   fn fine(a_b: Int) [] -> Int {\n    let f = { a_c: Int -> a_c }\n    let a_b2 = a_b\n    return a_b\n}\n\
                   fn other(read_x: Int) [] -> Int {\n    return read_x\n}\n";
        let (module, _) = salvo_syntax::parse_module(src);
        let msgs: Vec<String> = module_clashes(&module).into_iter().map(|(_, m)| m).collect();
        assert_eq!(msgs.len(), 3, "{msgs:#?}");
        assert!(msgs.iter().any(|m| m.contains("`readTo` and `read_to`") && m.contains("one module")));
        assert!(msgs.iter().any(|m| m.contains("`dataType` and `data_type`") && m.contains("struct `P`")));
        assert!(msgs.iter().any(|m| m.contains("`readX` and `read_x`") && m.contains("`read_to`")));
    }
}
