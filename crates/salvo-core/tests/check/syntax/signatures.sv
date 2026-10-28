// Signatures the parser refuses: parameters, deduction clauses and the
// return annotation. One program per error, since a parse error stops a
// program.

//--- an_ordinary_parameter_after_an_implicit_is_rejected
// [implicit-param] Implicit parameters trail the ordinary ones: a caller
// could not pass a positional parameter written after one.
//~ error[implicit-param:not-last]
fn f(?cmp: (Int, Int) -> Int, n: Int) -> Int {
    return n
}

// ===== The deduction clause [deduce-syntax] =====

//--- the_old_bracket_list_points_to_the_clause
// [deduce-syntax] The old bracket list is refused with a pointer to `=>`.
//~ error[deduce-syntax:bracketed-deductions]
fn f(a: Int) -> [a] Int {
}

//--- a_fn_type_carries_no_inline_list
// [deduce-syntax] A fn type may not carry an inline list.
//~ error[fn-contract:bracketed-deductions]
fn f(g: (v: Int) -> [v] Int) -> Int {
}

//--- a_group_names_a_parameter
// [deduce-syntax] A group must name a parameter…
//~ error[fn-contract:group-unknown-param]
fn f(a: Int) -> Int =>[b] !a {
}

//--- a_group_names_a_fn_typed_parameter
// …and a fn-typed one.
//~ error[fn-contract:group-not-fn-param]
fn f(a: Int) -> Int =>[a] !a {
}

//--- a_result_path_only_projects
// [deduce-syntax] A result path can only project.
//~ error[deduce-field:result-path-not-projection]
fn f(a: Int) -> Int => .x: Mut {
    return a
}

//--- an_empty_entry_names_none
//~ error[deduce-syntax:missing-qualifiers]
fn f(a: Int) -> Int => a: {
    return a
}

//--- a_field_path_states_mut_or_a_replacement
// [deduce-field] A field path stating anything but a field-level `Mut` or
// `!` is refused.
struct H canbe Mut {
    tags: List<Int> canbe Mut
}

//~ error[deduce-field:param-path-shape]
fn d(h: Mut H) -> None => h.tags: preserve Idx {}

//--- a_canbe_clause_is_a_parse_error
// [ref-anchor] The `canbe` aliasing clause is gone from the language
// (replaced by `ref(c)` parameters): `=> a canbe d` is a plain parse error.
//~ error[syntax-expected:item]
//~ error[platform-fn:missing-body]
//~ error[deduce-syntax:bodiless-clause]
fn f(a: Mut List<Int>, d: Mut List<Int>) -> None => a canbe d {}

// ===== Obligation keywords [obligation-spelling] =====

//--- obligation_keywords_are_reserved
// [obligation-spelling] The keywords stay usable nowhere but in qualifier
// position: a value named `proj` is a parse error, which is what "reserved"
// means.
fn f() {
    //~ error[syntax-expected:identifier]
    let proj = 1
}

// ===== The opaque projection [proj-infer] =====

//--- holds_needs_the_sources
// [proj-infer] `holds` needs its sources, and they live inside `proj(…)`.
//~ error[proj-infer:holds-without-sources]
fn view(a: List<Int>) -> Mut List<Int> holds proj() => a {
    return Mut List<Int>()
}

//--- the_prefix_annotation_is_gone
// [proj-infer] Both older spellings stopped parsing (plain errors, no
// shim): the prefix annotation of 2026-09-24…
//~ error[syntax-expected:item]
//~ error[platform-fn:missing-body]
//~ error[name-resolve:unknown-type]
//~ error[deduce-syntax:bodiless-clause]
fn view(a: List<Int>) -> proj(a) in (Mut View) => a {
}

//--- the_clause_entry_is_gone
// …and the clause entry before it.
//~ error[syntax-expected:identifier]
fn view(a: List<Int>) -> Mut View => a, proj(a) {
}

//--- holds_is_reserved
// [proj-infer] `holds` is reserved, which is what keeps a type's qualifier
// chain from eating it — so it is not an identifier any more.
//~ error[syntax-expected:identifier]
fn holds(a: Int) -> Int => a {
    return a
}
