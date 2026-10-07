//! [kt-ir] [platform-check] [platform-factory] The host boundary over the
//! IR: the checks a value crossing from the host gets (ABI D7), and the
//! factory objects a host builds unions with (ABI D5). Both plans are the
//! checker's, carried on the IR declarations; this file only spells them.

use salvo_core::abi::{BoundaryCheck, Factories};

use super::ModuleEmitter;
use crate::emit::{escape_string, kotlin_package, kt_ident, kt_literal, tuple_field};

impl<'a, 'p> ModuleEmitter<'a, 'p> {
    /// The statements checking `v` against `plan`; `what` names the value
    /// in the failure message.
    pub(super) fn boundary_check(&mut self, plan: &BoundaryCheck, v: &str, what: &str, indent: usize) -> String {
        use BoundaryCheck as C;
        let pad = "    ".repeat(indent);
        // `\u{1}` marks where the value goes: everything else is escaped.
        let fail = |msg: String| {
            format!(
                "throw IllegalStateException(\"salvo: {} [platform-check]\")",
                escape_string(&format!("{what} {msg}")).replace('\u{1}', &format!("${{{v}}}"))
            )
        };
        let d = indent;
        match plan {
            C::OneOf(lits) => {
                let cond: Vec<String> = lits.iter().map(|l| format!("{v} != {}", kt_literal(l))).collect();
                let listed: Vec<String> = lits.iter().map(|l| kt_literal(l).replace("\\$", "$")).collect();
                format!("{pad}if ({}) {}\n", cond.join(" && "), fail(format!("was \u{1}, which is not one of {}", listed.join(", "))))
            }
            C::Qualifies { quals, inner, .. } => {
                let mut out = String::new();
                for (q, m) in quals {
                    out.push_str(&format!("{pad}if (!{}.{q}_qualifies({v})) {}\n", kotlin_package(m), fail(format!("was \u{1}, which is not `{q}`"))));
                }
                if let Some(inner) = inner {
                    out.push_str(&self.boundary_check(inner, v, what, indent));
                }
                out
            }
            C::Elems(inner) => {
                let e = format!("__e{d}");
                let body = self.boundary_check(inner, &e, what, indent + 1);
                format!("{pad}for ({e} in {v}) {{\n{body}{pad}}}\n")
            }
            C::Entries(k, val) => {
                let (kn, vn) = (format!("__k{d}"), format!("__v{d}"));
                let mut body = String::new();
                if let Some(k) = k {
                    body.push_str(&self.boundary_check(k, &kn, what, indent + 1));
                }
                if let Some(val) = val {
                    body.push_str(&self.boundary_check(val, &vn, what, indent + 1));
                }
                format!("{pad}for (({kn}, {vn}) in {v}) {{\n{body}{pad}}}\n")
            }
            C::Nullable(inner) => {
                let body = self.boundary_check(inner, v, what, indent + 1);
                format!("{pad}if ({v} != null) {{\n{body}{pad}}}\n")
            }
            C::Union { arity, arms } => {
                self.s.union_sizes.insert(*arity);
                let stars = vec!["*"; *arity].join(", ");
                let mut out = format!("{pad}when ({v}) {{\n");
                for (i, ty, inner) in arms {
                    let a = format!("__a{d}_{i}");
                    let kt = self.ty(ty);
                    let body = self.boundary_check(inner, &a, what, indent + 2);
                    out.push_str(&format!(
                        "{pad}    is salvo.Union{arity}.U{}<{stars}> -> {{\n{pad}        @Suppress(\"UNCHECKED_CAST\") val {a} = {v}.value as {kt}\n{body}{pad}    }}\n",
                        i + 1
                    ));
                }
                out.push_str(&format!("{pad}    else -> {{}}\n{pad}}}\n"));
                out
            }
            C::Struct { fields, .. } => {
                let mut out = String::new();
                for (field, inner) in fields {
                    let f = format!("__f{d}_{}", kt_ident(field).trim_matches('`'));
                    out.push_str(&format!("{pad}val {f} = {v}.{}\n", kt_ident(field)));
                    out.push_str(&self.boundary_check(inner, &f, what, indent));
                }
                out
            }
            C::Tuple { elems, .. } => {
                let mut out = String::new();
                for (i, inner) in elems {
                    let t = format!("__t{d}_{i}");
                    out.push_str(&format!("{pad}val {t} = {v}.{}\n", tuple_field(*i)));
                    out.push_str(&self.boundary_check(inner, &t, what, indent));
                }
                out
            }
        }
    }

    /// `object Name { fun arm(value: A): U = … }`: one factory per arm of
    /// each set, with the arm's check when it has one.
    pub(super) fn factory_object(&mut self, object: &str, sets: &[Factories]) {
        if self.s.symbols.structs.contains_key(object) || self.s.symbols.effects.contains_key(object) || self.s.symbols.type_aliases.contains_key(object) {
            self.error(format!("the factories of `{object}` would be named like the type `{object}`: rename one [platform-factory]"));
            return;
        }
        let mut out = format!("\n// Factories for the host: one per arm of the union [platform-factory].\nobject {object} {{\n");
        let mut seen: Vec<String> = Vec::new();
        for set in sets {
            self.s.union_sizes.insert(set.arity);
            let ret = self.ty(&set.union);
            for name in &set.dropped {
                out.push_str(&format!("    // no `{}`: two arms would share the name; build them as `Union{}.Uk(…)`\n", kt_ident(name), set.arity));
            }
            for f in &set.arms {
                if seen.contains(&f.name) {
                    continue;
                }
                seen.push(f.name.clone());
                let param = self.ty(&f.param);
                let build = format!("salvo.Union{}.U{}(value)", set.arity, f.arm + 1);
                match &f.check {
                    Some(check) => {
                        let what = format!("`{object}.{}`'s argument", kt_ident(&f.name));
                        let body = self.boundary_check(check, "value", &what, 2);
                        out.push_str(&format!("    fun {}(value: {param}): {ret} {{\n{body}        return {build}\n    }}\n", kt_ident(&f.name)));
                    }
                    None => out.push_str(&format!("    fun {}(value: {param}): {ret} = {build}\n", kt_ident(&f.name))),
                }
            }
        }
        out.push_str("}\n");
        self.out.push_str(&out);
    }
}
