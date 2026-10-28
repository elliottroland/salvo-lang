// Declaration forms the parser refuses. One program per error, since a
// parse error stops a program.

//--- a_fns_groups_clause_takes_only_with_entries
// [implicit-with] A group has no body and no parameters of its own, so its
// clause states only which members are filled together.
//~ error[implicit-group:clause-shape]
fns Hashed<T> => !hash {
    fn hash(value: T) -> Long
}

//--- provenance_must_precede_a_qualifier
// [qual-subject] `provenance` only modifies a qualifier declaration.
//~ error[syntax-expected:after-provenance]
provenance struct Thing {
    v: Str
}

//--- platform_on_an_effect_names_the_forms
// [platform-handler] [platform-fn] [platform-type] `platform` takes
// `handler`, `fn` or `type`, and nothing else — a `platform effect`
// included, since platform effects were removed (2026-10-01). The
// diagnostic names the forms rather than reporting a bare "expected item".
//~ error[syntax-expected:after-platform]
platform effect Telemetry {
    fn emit(s: Str) [] -> None => s
}

//--- threadsafe_on_a_non_handler_names_the_form
// [platform-threadsafe] The contract is a claim about a host *class*, so it
// belongs on a `platform handler` only: before anything else it is an
// error naming the reason.
//~ error[platform-threadsafe:not-before-handler-or-type]
threadsafe platform effect Telemetry {
    fn emit(s: Str) -> None => s
}

//--- iterable_belongs_on_a_platform_type
// [platform-iterable] `iterable` is a claim about a host type, so only
// `platform type` may follow `iterable platform`.
//~ error[platform-iterable:not-before-type]
iterable platform fn f() [] -> Int

//--- a_bare_bodiless_fn_is_refused
// [platform-fn] [decl-body] A bodiless top-level fn is a `platform fn`:
// without the modifier the missing body is an error naming it.
//~ error[platform-fn:missing-body]
fn chars(s: Str) [] -> Char[] => s

//--- a_platform_fn_has_no_body
// [platform-fn] [decl-body] With the modifier, a body is an error (user
// decision 2026-10-01, ABI D1).
//~ error[platform-fn:has-body]
platform fn chars(s: Str) [] -> Str => !s {
    return s
}

//--- threadsafe_platform_fn
// [platform-threadsafe] `threadsafe` is for a handler or a type, not a fn.
//~ error[platform-threadsafe:not-before-handler-or-type]
threadsafe platform fn chars(s: Str) [] -> Str => s

//--- a_bodiless_non_alias_type_is_an_error
// [decl-body] A `type` with no `= ...` alias (and not `intrinsic`) was only
// ever meaningful as `external type`; with `external` gone it declares
// nothing, so it is a parse error.
//~ error[decl-body:type-without-definition]
type LinkedList<T>

//--- a_canbe_type_without_an_alias_is_an_error
// [decl-body] A `canbe` clause is not a definition: a `type` opting into an
// auto-qualifier with no alias still declares nothing.
//~ error[decl-body:type-without-definition]
type List<T> canbe Mut

//--- external_fn
// `external` is gone (user decision 2026-09-05): it is an ordinary
// identifier now, so an `external` declaration no longer parses — the
// leading identifier is not a valid item.
//~ error[syntax-expected:item]
//~ error[platform-fn:missing-body]
external fn chars(str: Str) [] -> Char[] => str

//--- external_type
//~ error[syntax-expected:item]
//~ error[decl-body:type-without-definition]
external type LinkedList<T>

//--- external_handler
//~ error[syntax-expected:item]
//~ error[effect-handler-multi:missing-member]
//~ error[effect-handler-multi:missing-member]
//~ error[effect-handler-multi:missing-member]
external handler StdConsole of Console

//--- define_fn
// `define` is gone too: the same identifier-at-item-position parse error.
//~ error[syntax-expected:item]
//~ error[fn-must-return:missing-return]
define fn List<T>(...elems: T[]) -> List<T> {
}

//--- define_type
//~ error[syntax-expected:item]
//~ error[syntax-expected:item]
//~ error[decl-body:type-without-definition]
define type LinkedList<T> {
}

//--- define_handler
//~ error[syntax-expected:item]
//~ error[effect-handler-multi:missing-member]
//~ error[effect-handler-multi:missing-member]
//~ error[effect-handler-multi:missing-member]
define handler StdConsole of Console {
}

//--- noremote_on_a_fn_is_an_error
// [net-noremote] Before anything but a type declaration it is an error
// naming what the word is for.
//~ error[net-noremote:not-before-type]
noremote fn go() -> None {
}

//--- a_platform_type_cannot_be_an_alias
// [platform-type] A platform type is the host's; an alias is refused.
//~ error[platform-type:alias]
platform type Client = Int

//--- one_init_block
// [handler-init] A handler has at most one `init` block.
handler H() of R {
    init {}
    //~ error[handler-init:duplicate]
    init {}
}

//--- a_bare_handler_field_is_refused
// [handler-state-infer] A bare field in the handler body (not in a
// `state { }` block) is an error naming the block.
effect E { fn get() -> Int }
handler H() of E {
    //~ error[handler-state-name:bare-field]
    sum: Int = 0
    fn get() -> Int { return sum }
}

//--- a_stray_keyword_in_a_handler_body_does_not_hang
// [effect-handler] A handler body whose line starts with a token that is
// neither a member nor a field (`is`) is one error, not an endless loop:
// field recovery stops at a line start, so the body loop made no progress
// and pushed a diagnostic per turn until memory ran out (found 2026-10-20
// compiling a stale scratch project; the parse never returned).
handler H() of C {
    //~ error[syntax-expected:identifier]
    //~ error[handler-state-name:bare-field]
    is
}

//--- a_comptime_qualifier_before_a_fn
// [comptime-qualifier] Qualifiers written before a declaration annotate a
// `struct`; nothing else may follow them.
//~ error[comptime-qualifier:not-before-struct]
Named("x")
fn f() {}

//--- a_field_canbe_mut_prefix_is_refused
// [struct-field-canbe-mut] A field's `canbe Mut` is written after its type
// (user decision 2026-10-20); the old prefix spelling is a plain parse error.
struct S canbe Mut {
    //~ error[syntax-expected:identifier]
    a: canbe Mut List<Int>
}

//--- only_mut_may_follow_a_field_canbe
// [struct-field-canbe-mut] Only `Mut` may follow `canbe`.
struct S canbe Mut {
    //~ error[struct-field-canbe-mut:not-mut]
    a: List<Int> canbe Ord
}

// ===== Refinements [qual-refn] =====

//--- a_refinement_may_not_declare_effects
// [qual-refn] The things a refinement may not say are diagnostics naming
// the reason, not bare parse errors: it changes what is *known* after a
// call, never what the call does.
//~ error[qual-refn:effects]
refn add<T>(list: Mut List<T>) [Console] => list: +NonEmpty

//--- a_refinement_may_not_declare_a_return_type
//~ error[qual-refn:return-type]
refn add<T>(list: Mut List<T>) -> Int => list: +NonEmpty

//--- a_refinement_qualifier_needs_a_sign
//~ error[qual-refn:unsigned-qualifier]
//~ error[qual-refn-scope:top-level-refn]
//~ error[qual-refn-match:no-overload]
refn add<T>(list: Mut List<T>) => list: NonEmpty

//--- a_refinement_needs_a_deduction_clause
// The error is reported at the token after the parameters, where the clause
// was expected.
refn add<T>(list: Mut List<T>)
//~ error[qual-refn:missing-deductions]
export fn after() -> None {}

// ===== Renames [fn-rename] =====

//--- a_rename_takes_no_effects
// [fn-rename] A rename names an overload by its parameters, so anything that
// takes no part in *choosing* one is refused by name: an effect list, a
// deduction list, a return type, and an implicit-group spread.
//~ error[fn-rename:effects-or-deductions]
rename fn add2 = add(a: Int) [Console]

//--- a_rename_takes_no_deductions
// The `=>` is reported: a deduction clause is not part of a rename.
//~ error[fn-rename:effects-or-deductions]
rename fn add2 = add(a: Int) => a

//--- a_rename_takes_no_return_type
//~ error[fn-rename:return-type]
rename fn add2 = add(a: Int) -> Int

//--- a_rename_takes_no_implicit_group
//~ error[fn-rename:implicit-group]
rename fn total2 = total(xs: List<Int>, ?Field<Int>)

// ===== Tests [test-decl] =====

//--- an_actor_test_refuses_an_unknown_argument
// FILE: main.test.sv
// [test-actor] An actor test takes `seed`, and nothing else.
//~ error[test-actor:unknown-argument]
test actor(speed: 2) "x" {
}

//--- a_test_name_may_not_interpolate
// FILE: main.test.sv
// [test-decl] The name must be knowable without running anything — `--list`
// and the filter depend on it — so interpolation is refused.
//~ error[test-decl:interpolated-name]
test "case ${n}" {
    expect(true, "x")
}

//--- a_test_cannot_be_exported
// FILE: main.test.sv
// [test-decl] A test is run, never referenced, so there is nothing to export.
//~ error[test-decl:exported-test]
export test "nope" {
    expect(true, "x")
}
