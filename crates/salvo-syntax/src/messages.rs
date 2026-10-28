//! [diag-id] The text of every diagnostic the parser and the core passes
//! report, one function per diagnostic, named after its id (TESTS.md T2;
//! user decisions 2026-10-28): `op-unique-shape:bare-type-param` is
//! `op_unique_shape_bare_type_param(…)`, so one grep
//! (`op.unique.shape.bare.type.param`) finds the rule, the message and the
//! tests together, and a rewording touches this file only.
//!
//! An id is `<rule label>:<diagnostic name>`: the label a spec defines
//! (LANGUAGE_SPEC.md; backend diagnostics live in their own crate's
//! `messages.rs`, under their own prefix [core-layers]), then a name for
//! this one way the rule can fail. A diagnostic with several shapes takes an
//! argument rather than a second function, so function and id stay one to
//! one. Tests assert ids, never the text.

/// A diagnostic's id and its text, as a `messages!` function makes them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Message {
    pub id: &'static str,
    pub text: String,
}

/// Defines one function per diagnostic, and `ALL`, every `(function name,
/// id)` defined, for the test that holds names and ids together.
///
/// ```text
/// messages! {
///     /// What the diagnostic says.
///     op_unique_shape_bare_type_param = "op-unique-shape:bare-type-param" (name: &str) => format!("…{name}…");
/// }
/// ```
#[macro_export]
macro_rules! messages {
    ($( $(#[$meta:meta])* $name:ident = $id:literal ( $($arg:ident : $ty:ty),* $(,)? ) => $text:expr ; )*) => {
        $(
            $(#[$meta])*
            #[allow(clippy::too_many_arguments)]
            pub fn $name($($arg: $ty),*) -> $crate::messages::Message {
                $crate::messages::Message { id: $id, text: ($text).into() }
            }
        )*
        /// Every message this file defines, as `(function name, id)`.
        pub const ALL: &[(&str, &str)] = &[$((stringify!($name), $id)),*];
    };
}

/// Whether `name` is the function name `id` gives: `-` and `:` become `_`.
pub fn name_matches_id(name: &str, id: &str) -> bool {
    id.replace(['-', ':'], "_") == name
}

/// Whether `id` is well formed: a label (lowercase words joined by `-`), a
/// `:`, and a name of the same shape.
pub fn id_well_formed(id: &str) -> bool {
    let word = |w: &str| !w.is_empty() && w.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit());
    let part = |p: &str| p.split('-').all(word);
    matches!(id.split_once(':'), Some((label, name)) if part(label) && part(name))
}

/// The label of `id`: what comes before the `:`.
pub fn label_of(id: &str) -> &str {
    id.split_once(':').map_or(id, |(label, _)| label)
}

messages! {
    // ===== lexer =====

    // [syntax-expected]
    /// A character no token starts with.
    syntax_expected_unexpected_character = "syntax-expected:unexpected-character" (c: char) => format!(
        "unexpected character `{c}`"
    );

    // [type-basic]
    /// A `'…'` that is not exactly one character or escape.
    type_basic_invalid_char_literal = "type-basic:invalid-char-literal" () =>
        "invalid character literal";

    // [type-lit-numeric]
    /// A numeric literal runs on into identifier characters (`10x`, `1.2fx`).
    type_lit_numeric_invalid_literal = "type-lit-numeric:invalid-literal" (source: &str) => format!(
        "invalid numeric literal `{source}`"
    );
    /// An `f` suffix on a literal without a decimal point.
    type_lit_numeric_float_suffix_without_point = "type-lit-numeric:float-suffix-without-point" (text: &str) => format!(
        "float suffix `f` requires a decimal point (write `{text}.0f`)"
    );
    /// An `L` suffix on a literal with a decimal point.
    type_lit_numeric_long_suffix_on_float = "type-lit-numeric:long-suffix-on-float" (text: &str) => format!(
        "long suffix `L` is not valid on the float literal `{text}`"
    );
    /// A float literal that does not parse as a float.
    type_lit_numeric_invalid_float = "type-lit-numeric:invalid-float" (text: &str) => format!(
        "invalid float literal `{text}`"
    );
    /// An integer literal that does not fit its type.
    type_lit_numeric_invalid_integer = "type-lit-numeric:invalid-integer" (text: &str) => format!(
        "invalid integer literal `{text}`"
    );

    // [type-str]
    /// A string literal that the end of the line or file cuts off.
    type_str_unterminated_string = "type-str:unterminated-string" () =>
        "unterminated string literal";
    /// A `${` whose `}` is missing.
    type_str_unterminated_interpolation = "type-str:unterminated-interpolation" () =>
        "unterminated `${...}` interpolation";
    /// A `\` followed by a character no escape starts with.
    type_str_unknown_escape = "type-str:unknown-escape" (c: char) => format!(
        "unknown escape sequence `\\{c}`"
    );

    // ===== parser =====

    // [actor-effect-kind]
    /// The spelling `async effect` for an actor effect.
    actor_effect_kind_async_effect = "actor-effect-kind:async-effect" () =>
        "an asynchronous protocol is declared `actor effect`, not `async effect`: what \
         runs one is an **actor** — and there is no `async` anywhere in Salvo, since \
         nothing is coloured";

    // [actor-mailbox]
    /// A handler with a second `mailbox` slot.
    actor_mailbox_duplicate = "actor-mailbox:duplicate" () =>
        "a handler declares one `mailbox` slot";

    // [actor-replyto]
    /// A trailing `replyto` that follows no call.
    actor_replyto_not_after_call = "actor-replyto:not-after-call" () =>
        "a trailing `replyto` fills the final `Reply<T>` argument of the call before it \
         (`send(x) replyto k(…)`), so it must follow a call";
    /// A `replyto` without a member call.
    actor_replyto_missing_member = "actor-replyto:missing-member" () =>
        "`replyto` names a member and its captures: `replyto k()`";
    /// A `replyto` with named captures.
    actor_replyto_named_captures = "actor-replyto:named-captures" () =>
        "a `replyto` takes the continuation's captures positionally";

    // [actor-self-send]
    /// `@self` not after a plain member name.
    actor_self_send_misplaced_selector = "actor-self-send:misplaced-selector" () =>
        "`@self` selects a member of the enclosing handler, so it follows a plain member \
         name (`k@self(…)`)";
    /// `self.k(…)` written for a self-send.
    actor_self_send_self_dot_call = "actor-self-send:self-dot-call" () =>
        "`self.k(…)` is not the self-send form: write `k@self(…)`, the selector that \
         names the enclosing handler";

    // [actor-waitfor]
    /// A `waitfor` without a binder and a block.
    actor_waitfor_shape = "actor-waitfor:shape" () =>
        "`waitfor` takes a binder and a block that sends the token somewhere: `waitfor \
         out { p.total(out) }`, or `waitfor out: Reply<Int> { … }` to write the type out";

    // [assert-op]
    /// A postfix `!`.
    assert_op_postfix_bang = "assert-op:postfix-bang" () =>
        "a postfix `!` asserts nothing in Salvo: write `x ?: unreachable!(\"why\")` for \
         the value, or `assert!(x !is None)` to narrow a variable";

    // [cmp-carry]
    /// A function slot declared on a function.
    cmp_carry_fn_slot_on_function = "cmp-carry:fn-slot-on-function" (name: &str) => format!(
        "`?{name}` declares a function slot, which only a qualifier or an `intrinsic \
         type` may have: on a function, write the implicit parameter (`?{name}: …`) in \
         the parameter list, and use `?{name}` in the types it applies to"
    );
    /// A function slot on a declaration that cannot carry one.
    cmp_carry_fn_slot_misplaced = "cmp-carry:fn-slot-misplaced" (name: &str) => format!(
        "`?{name}` declares a function slot, which only a qualifier, a struct or an \
         `intrinsic type` may have"
    );
    /// A type parameter after a function slot.
    cmp_carry_type_param_after_slot = "cmp-carry:type-param-after-slot" () =>
        "a type parameter cannot follow a function slot: slots come last, so the types \
         they are written over are already in scope";
    /// A function slot whose type is not a function type.
    cmp_carry_slot_not_fn_type = "cmp-carry:slot-not-fn-type" (ty: &dyn std::fmt::Display, name: &str) => format!(
        "a function slot must have a function type, but `{ty}` is not one: write \
         `?{name}: (T, T) -> Int`"
    );

    // [col-no-literal]
    /// A `{…}` that is neither a struct literal nor a lambda.
    col_no_literal_brace_literal = "col-no-literal:brace-literal" () =>
        "a `{...}` here is neither a struct literal nor a lambda. Collection literals are \
         retired: build a collection with its constructor — `List(1, 2, 3)`, `Set(\"a\", \
         \"b\")`, `Map(\"k\": v)` (or the `Mut`, `Sorted…` variants)";

    // [col-pairs]
    /// A `k: v` pair argument after positional or named ones.
    col_pairs_pair_after_other = "col-pairs:pair-after-other" () =>
        "a `k: v` pair argument cannot be mixed with positional or named arguments: a \
         call uses pairs for every argument or none";
    /// A positional argument after `k: v` pair arguments.
    col_pairs_positional_after_pair = "col-pairs:positional-after-pair" () =>
        "a positional argument cannot be mixed with `k: v` pair arguments: a call uses \
         pairs for every argument or none";

    // [comptime-bound]
    /// `comptime` followed by something it cannot modify.
    comptime_bound_after_comptime = "comptime-bound:after-comptime" (found: &str) => format!(
        "expected `fn`, `struct`, `type` or `qualifier` after `comptime`, found {found}"
    );
    /// The retired `compfn` spelling.
    comptime_bound_compfn_keyword = "comptime-bound:compfn-keyword" () =>
        "`compfn` is spelled `comptime fn`";
    /// A `comptime fn` type parameter without a kind bound.
    comptime_bound_unbounded_param = "comptime-bound:unbounded-param" (found: &str) => format!(
        "a `comptime fn`'s type parameter is bound to a kind: write `<T is Struct>` or \
         `<T is Union>` (found {found})"
    );
    /// A `comptime fn` without exactly one bounded type parameter.
    comptime_bound_param_count = "comptime-bound:param-count" () =>
        "a `comptime fn` takes exactly one bounded type parameter: the type it is stamped \
         at";
    /// A `comptime fn` without a body.
    comptime_bound_missing_body = "comptime-bound:missing-body" (name: &str) => format!(
        "`comptime fn {name}` has no body: it is the code that is stamped, so it always \
         has one"
    );
    /// A `comptime fn` that says `by` itself.
    comptime_bound_says_by = "comptime-bound:says-by" (name: &str) => format!(
        "`comptime fn {name}` cannot itself say `by`: it is what `by` stamps from"
    );

    // [comptime-default]
    /// `x.default` compared with something other than `None`.
    comptime_default_unset_test = "comptime-default:unset-test" (binder: &str) => format!(
        "a default is tested for being there: `{binder}.default is None` or `!is None`"
    );

    // [comptime-docs]
    /// `x.docs` compared with something other than `None`.
    comptime_docs_unset_test = "comptime-docs:unset-test" (binder: &str) => format!(
        "a docs is tested for being there: `{binder}.docs is None` or `!is None`"
    );

    // [comptime-fields]
    /// A token that is no compile-time type.
    comptime_fields_expected_type = "comptime-fields:expected-type" (found: &str) => format!(
        "expected a compile-time type — the bound parameter (`T`), a type name, or a \
         binder's type (`field.type`) — found {found}"
    );
    /// A `[for …]` over something other than `.fields`, `.arms` or `.literals`.
    comptime_fields_unknown_sequence = "comptime-fields:unknown-sequence" (other: &str) => format!(
        "a `[for …]` walks `.fields` (of a struct), `.arms` (of a union) or `.literals` \
         (of a literal union), not `.{other}`"
    );

    // [comptime-hygiene]
    /// A written name in the compiler's `_` namespace.
    comptime_hygiene_generated_name = "comptime-hygiene:generated-name" (name: &str) => format!(
        "`{name}` is a compiler-generated name and cannot be written"
    );

    // [comptime-inline]
    /// A binder's `.type` in a type position without brackets.
    comptime_inline_unbracketed_binder_type = "comptime-inline:unbracketed-binder-type" (binder: &str) => format!(
        "a binder's type is spliced into a type position in brackets: write \
         `[{binder}.type]`, not `{binder}.type`"
    );
    /// Comptime syntax outside a `comptime fn`.
    comptime_inline_outside_compfn = "comptime-inline:outside-compfn" (what: &str) => format!(
        "{what} is comptime syntax and belongs inside a `comptime fn`: an ordinary fn has \
         no fields to walk or instantiations to refuse"
    );
    /// A `[when value]` over a union without exactly one written arm.
    comptime_inline_union_when_arms = "comptime-inline:union-when-arms" () =>
        "a `[when value]` over a union has exactly one written arm, `[arm] { … }`, \
         stamped once per declared arm";
    /// A `[when]` arm that is not an arm of `Kind`.
    comptime_inline_expected_kind_arm = "comptime-inline:expected-kind-arm" (found: &str) => format!(
        "expected an arm of `Kind` — `Struct`, `Union`, `Tuple`, `FnType`, `Opaque` (or \
         its halves `Basic`, `Generic`) — found {found}"
    );
    /// A binder's `.index` compared to something other than a position.
    comptime_inline_position_compare = "comptime-inline:position-compare" (found: &str) => format!(
        "a position is compared to another binder's position: `x.index == y.index` (found \
         {found})"
    );
    /// A binder's `.index` compared to a binder rather than its `.index`.
    comptime_inline_position_rhs = "comptime-inline:position-rhs" () =>
        "a position compares to a position: write `y.index`";
    /// A comptime condition that tests `canbe`.
    comptime_inline_canbe_test = "comptime-inline:canbe-test" () =>
        "a type's mutability is read as `T.mutable` in a comptime condition \
         (`core.comptime`'s `Struct.mutable`), not with `canbe`";
    /// A type splice in an expression that heads no struct literal or call.
    comptime_inline_splice_not_head = "comptime-inline:splice-not-head" () =>
        "a type splice in an expression is the head of a struct literal or a constructor \
         call: `[arm.type] { … }`, `[arm.type](value)`";

    // [comptime-name]
    /// A binder's name compared to something other than a string literal.
    comptime_name_expected_string = "comptime-name:expected-string" (found: &str) => format!(
        "expected a string literal naming a field or arm, found {found}"
    );
    /// A binder's name compared to an interpolated string.
    comptime_name_interpolated = "comptime-name:interpolated" () =>
        "the name a binder is compared to is a plain literal, with no interpolation";
    /// A binder's `.name` compared other than to a literal.
    comptime_name_name_compare = "comptime-name:name-compare" () =>
        "a binder's name is compared to a literal: `field.name == \"value\"`, or tested \
         against a union of them: `field.name is \"a\" | \"b\"`";

    // [comptime-qualifier]
    /// A comptime qualifier before a declaration that is not a `struct`.
    comptime_qualifier_not_before_struct = "comptime-qualifier:not-before-struct" () =>
        "a comptime qualifier before a declaration describes a struct: only a `struct` \
         may follow it";
    /// A comptime qualifier on a `state` field.
    comptime_qualifier_on_state_field = "comptime-qualifier:on-state-field" () =>
        "a comptime qualifier describes a struct's field; a `state` field is never walked \
         by comptime code";
    /// A comptime qualifier on a qualifier's field override.
    comptime_qualifier_on_field_override = "comptime-qualifier:on-field-override" () =>
        "a qualifier's field override is a claim about the field's value; a comptime \
         qualifier belongs on the struct's own field";
    /// A comptime `is` test on a binder that should be on `.type`.
    comptime_qualifier_type_test = "comptime-qualifier:type-test" (subject: &str) => format!(
        "`{subject} is …` tests a comptime qualifier (`{subject} is Short`); a type is \
         tested on `.type` (`{subject}.type is Int`)"
    );

    // [decl-body]
    /// A `type` with neither an alias nor `intrinsic`.
    decl_body_type_without_definition = "decl-body:type-without-definition" (name: &str) => format!(
        "`type {name}` declares nothing: give it a definition (`type {name} = ...`)"
    );

    // [deduce-field]
    /// A result path entry that states something other than a projection.
    deduce_field_result_path_not_projection = "deduce-field:result-path-not-projection" () =>
        "a result path (`.field`) can only state a projection: `.field: proj(p)`";
    /// A parameter field path entry of a shape it cannot take.
    deduce_field_param_path_shape = "deduce-field:param-path-shape" () =>
        "a parameter's field path can state a projection                      (`v.field: \
         proj(p)`), a field-granular mutation                      (`v.field: Mut`) or a \
         replacement (`!v.field`)";

    // [deduce-syntax]
    /// A `=>` with no deduction entry after it.
    deduce_syntax_missing_entry = "deduce-syntax:missing-entry" () =>
        "expected a deduction entry after `=>`";
    /// Deductions written in brackets after `->`.
    deduce_syntax_bracketed_deductions = "deduce-syntax:bracketed-deductions" () =>
        "deductions are written after the return type, behind `=>` (`-> Int => list: \
         Mut`), not in brackets after `->`";
    /// A `proj(…)` deduction entry with no sources.
    deduce_syntax_projection_without_sources = "deduce-syntax:projection-without-sources" () =>
        "a projection entry needs its sources: `proj(c)`";
    /// A `:` in a deduction entry with no qualifiers after it.
    deduce_syntax_missing_qualifiers = "deduce-syntax:missing-qualifiers" () =>
        "expected qualifiers after `:` — `None` to strip every qualifier, `Never` to \
         consume the value";
    /// A deduction entry that mixes plain or `+` qualifiers with `-` ones.
    deduce_syntax_exhaustive_and_delta = "deduce-syntax:exhaustive-and-delta" () =>
        "a deduction entry is either exhaustive (plain qualifier names, `+Qual` for one \
         this function re-establishes) or a delta (`-Qual`), not both: an exhaustive list \
         already drops everything it does not name";

    // [effect-at]
    /// An effect `@` selector not after a member name.
    effect_at_misplaced = "effect-at:misplaced" () =>
        "`@` selects which effect's member a *name* means, so it follows a member name \
         (`close@Fs(s)`) or a dot-notation call (`s.close@Fs()`)";

    // [effect-eff-handle]
    /// An `eff[]` naming no effect.
    effect_eff_handle_empty = "effect-eff-handle:empty" () =>
        "`eff[…]` names at least one effect";
    /// `Addr<…>` written as a type.
    effect_eff_handle_addr_type = "effect-eff-handle:addr-type" () =>
        "`Addr<…>` is not a type you write: a first-class effect handle is spelled \
         `eff[E]` (several effects: `eff[A, B]`)";

    // [effect-with-clause]
    /// The retired `use` spelling of a dependency clause.
    effect_with_clause_use_spelling = "effect-with-clause:use-spelling" () =>
        "the dependency clause of a `use`/`spawn` is spelled `with` now (`spawn H(args) \
         with D(), addr on POOL`): `use` is the statement's word";

    // [expr-terminal-lambda]
    /// A terminal `(…) -> ...` lambda in a nested or stored position.
    expr_terminal_lambda_nested = "expr-terminal-lambda:nested" () =>
        "a `(…) -> ...` lambda body is a terminal scope — the rest of the enclosing block \
         becomes it — so it is only the last argument of a statement's call, never a \
         nested or stored expression: write a `{ … }` body here";
    /// A second terminal lambda in one statement.
    expr_terminal_lambda_two_per_statement = "expr-terminal-lambda:two-per-statement" () =>
        "only one `(…) -> ...` terminal lambda per statement: the rest of the block \
         cannot be two bodies — write `{ … }` on all but one";

    // [expr-terminal-scope]
    /// A terminal `...` body where a value or nested expression is required.
    expr_terminal_scope_nested_body = "expr-terminal-scope:nested-body" (what: &str) => format!(
        "`...` is a terminal scope — the rest of the enclosing block becomes {what}'s \
         body — so it is only a statement, never a value or a nested expression: write a \
         `{{ … }}` block here"
    );
    /// A terminal `[if …] ...` in a nested position.
    expr_terminal_scope_nested_comptime_if = "expr-terminal-scope:nested-comptime-if" () =>
        "`...` is a terminal scope — the rest of the enclosing block becomes the `[if \
         …]`'s body — so it is only a statement: write a `{ … }` block";
    /// `else ...` on a terminal `[if …]`.
    expr_terminal_scope_terminal_comptime_if_else = "expr-terminal-scope:terminal-comptime-if-else" () =>
        "`else ...` is not allowed on a terminal `[if …]`: only the `[if]` branch is the \
         terminal scope, and its `else` is an ordinary `{ … }` block";
    /// A terminal `if … ...` in a nested position.
    expr_terminal_scope_nested_if = "expr-terminal-scope:nested-if" () =>
        "`...` is a terminal scope — the rest of the enclosing block becomes the `if`'s \
         then-body — so it is only a statement, never a value or a nested expression: \
         write a `{ … }` block";
    /// `elif` after a terminal `if … ...`.
    expr_terminal_scope_terminal_if_elif = "expr-terminal-scope:terminal-if-elif" () =>
        "a terminal `if … ...` cannot have `elif`: the `...` body is the rest of the \
         block, so there is nowhere for an `elif` to go — write the braced `if { … } elif \
         { … }` form";
    /// `else ...` on a terminal `if`.
    expr_terminal_scope_terminal_if_else = "expr-terminal-scope:terminal-if-else" () =>
        "`else ...` is not allowed on a terminal `if`: only the `if` branch is the \
         terminal scope, and its `else` is an ordinary `{ … }` block";

    // [expr-tuple-index]
    /// A tuple index with an `L` suffix.
    expr_tuple_index_long_suffix = "expr-tuple-index:long-suffix" () =>
        "a tuple index has no `L` suffix";
    /// A tuple index that does not fit a `usize`.
    expr_tuple_index_invalid = "expr-tuple-index:invalid" (value: i64) => format!(
        "`{value}` is not a valid tuple index"
    );

    // [fn-attached]
    /// `export` on a function declared inside a struct.
    fn_attached_export_inside_struct = "fn-attached:export-inside-struct" () =>
        "a function declared inside a struct takes the struct's own visibility: drop the \
         `export` (export the struct instead)";
    /// A declaration written with a `@` selector.
    fn_attached_at_selector = "fn-attached:at-selector" (target: &str) => format!(
        "a declaration does not take a `@` selector: write the function inside \
         `{target}`'s body to declare it on the type, or let it fulfil one of \
         `{target}`'s obligations (`: Hashed<self>`) in this file"
    );

    // [fn-by]
    /// The retired `auto fn` spelling.
    fn_by_auto_fn = "fn-by:auto-fn" () =>
        "`auto fn` no longer exists: write `fn cmp(a: T, b: T) -> Int by auto` — the \
         structural implementation is stamped from `core.auto`'s `compfn`";
    /// A function that says `by` and has a body.
    fn_by_body = "fn-by:body" (name: &str) => format!(
        "`fn {name}` says `by`, so its body is stamped: drop the body, or drop the `by` \
         to write it yourself"
    );

    // [fn-contract]
    /// A `=>[name]` group naming no parameter.
    fn_contract_group_unknown_param = "fn-contract:group-unknown-param" (name: &str) => format!(
        "`=>[{name}]` names no parameter of this function"
    );
    /// A `=>[name]` group naming a parameter that is not fn-typed.
    fn_contract_group_not_fn_param = "fn-contract:group-not-fn-param" (name: &str) => format!(
        "`=>[{name}]` scopes deductions to a fn-typed parameter, but `{name}` is not one"
    );
    /// A fn type's deductions written in brackets after its `->`.
    fn_contract_bracketed_deductions = "fn-contract:bracketed-deductions" () =>
        "a fn type's deductions are written on the enclosing declaration as a group, \
         `=>[name] …`, not in brackets after its `->`";
    /// Named parameters in a parenthesized type that is not a function type.
    fn_contract_named_params_outside_fn_type = "fn-contract:named-params-outside-fn-type" () =>
        "named parameters are only meaningful in function types (expected `->` after `)`)";

    // [fn-overload-at]
    /// A module `@` selector not after a function name.
    fn_overload_at_misplaced = "fn-overload-at:misplaced" () =>
        "`@` selects which module's overload a *name* means, so it follows a function \
         name (`add@core.list(x)`) or a dot-notation call (`xs.add@core.list(x)`)";

    // [fn-param-default]
    /// A default value on a variadic parameter.
    fn_param_default_variadic_default = "fn-param-default:variadic-default" () =>
        "a variadic parameter cannot have a default value";
    /// A default value on an implicit parameter.
    fn_param_default_implicit_default = "fn-param-default:implicit-default" () =>
        "an implicit parameter cannot have a default value: it is resolved automatically, \
         or overridden with `name = value`";

    // [fn-rename]
    /// A `rename fn` signature that names a `?Group`.
    fn_rename_implicit_group = "fn-rename:implicit-group" (name: &str) => format!(
        "`rename fn {name}` names an overload by its ordinary parameters: implicit \
         parameters take no part in choosing one, so `?Group` does not belong here"
    );
    /// A `rename fn` signature with an effect or deduction clause.
    fn_rename_effects_or_deductions = "fn-rename:effects-or-deductions" (name: &str) => format!(
        "`rename fn {name}` names an overload by its parameters, so it takes no effect or \
         deduction clause: neither takes part in choosing an overload"
    );
    /// A `rename fn` signature with a return type.
    fn_rename_return_type = "fn-rename:return-type" (name: &str) => format!(
        "`rename fn {name}` names an overload by its parameters, so it takes no return \
         type: overloads are never chosen by what they return"
    );

    // [handler-init]
    /// A handler with a second `init` block.
    handler_init_duplicate = "handler-init:duplicate" () =>
        "a handler declares one `init` block";

    // [handler-state]
    /// A handler's state field without an initializer.
    handler_state_missing_initializer = "handler-state:missing-initializer" (field: &str) => format!(
        "state field `{field}` needs an initializer: a handler's state is built once, at \
         construction"
    );

    // [handler-state-name]
    /// A handler with a second `state` block.
    handler_state_name_duplicate_block = "handler-state-name:duplicate-block" () =>
        "a handler declares one `state` block";
    /// A bare field in a handler's body, outside its `state` block.
    handler_state_name_bare_field = "handler-state-name:bare-field" () =>
        "a handler's state goes in a `state { … }` block now, not as a bare field in the \
         body: wrap it (`state { count: Int = 0 }`, or `state { count = 0 }` to infer the \
         type)";

    // [implicit-group]
    /// A fns group's clause with a body or parameters.
    implicit_group_clause_shape = "implicit-group:clause-shape" () =>
        "a fns group's clause states only which of its members are filled together (`=> \
         eq with hash`): it has no body and no parameters of its own";

    // [implicit-override]
    /// A named argument where only an implicit parameter can be named.
    implicit_override_named_ordinary_param = "implicit-override:named-ordinary-param" () =>
        "only an implicit parameter can be given by name here: write `name = value`";
    /// A positional argument after a named one.
    implicit_override_positional_after_named = "implicit-override:positional-after-named" () =>
        "a positional argument cannot follow a named one";

    // [implicit-param]
    /// A variadic implicit parameter.
    implicit_param_variadic = "implicit-param:variadic" () =>
        "a variadic parameter cannot be implicit";
    /// An ordinary parameter after an implicit one.
    implicit_param_not_last = "implicit-param:not-last" () =>
        "implicit parameters must come last: a parameter after one could not be passed \
         positionally";

    // [implicit-with]
    /// A `with` entry whose subject is a result path.
    implicit_with_result_path = "implicit-with:result-path" () =>
        "a `with` entry names implicit parameters, not a result path";
    /// A `with` entry whose subject is a field path.
    implicit_with_field_path = "implicit-with:field-path" () =>
        "a `with` entry's subject is a parameter, not a field path";

    // [is-not]
    /// A binding on a `!is` test.
    is_not_binding = "is-not:binding" () =>
        "`!is` binds nothing: the value is only known on the branch the test *fails*, so \
         there is nothing to name — drop the binding, or write the positive test with an \
         `else`";
    /// A `^` lift on a `!is` test.
    is_not_lift = "is-not:lift" () =>
        "`!is ^Q` is not a check: `^` widens the value it tested, and a negated test \
         produces no value to widen — write `!(x is ^Q)` if the boolean is all you want";

    // [is-union]
    /// A binding on an `is` over several tests.
    is_union_binding = "is-union:binding" () =>
        "an `is` over several tests binds nothing: bind the narrowed value with `let` \
         after the test";
    /// A `^` lift on an `is` over several tests.
    is_union_lift = "is-union:lift" () =>
        "an `is` over several tests cannot lift `^`: lift in one test";
    /// An `is` over several tests whose subject is not a variable or field.
    is_union_subject_not_place = "is-union:subject-not-place" () =>
        "an `is` over several tests reads its subject once per test, so the subject is a \
         variable or a field: bind the value first (`let v = …`)";

    // [iter-fn]
    /// An `iter fn` whose body does not begin with its `state` block.
    iter_fn_missing_state = "iter-fn:missing-state" () =>
        "an `iter fn` names its iterator struct: begin the body with `state Name { … }`";
    /// An `iter fn`'s `state` block without the iterator struct's name.
    iter_fn_unnamed_state = "iter-fn:unnamed-state" () =>
        "an `iter fn`'s `state` block names the iterator struct: `state Name { … }`";
    /// An `iter fn`'s state field without an initializer.
    iter_fn_state_missing_initializer = "iter-fn:state-missing-initializer" (field: &str) => format!(
        "`state` field `{field}` needs an initializer: it is evaluated once, when the \
         pass is minted"
    );

    // [linear-generics]
    /// `canbe` on a type parameter of a declaration that cannot take one.
    linear_generics_canbe_type_param_unsupported = "linear-generics:canbe-type-param-unsupported" () =>
        "`canbe` on a type parameter is only supported on functions, structs and qualifiers";

    // [linear-opaque]
    /// `linear intrinsic` not followed by `type`.
    linear_opaque_linear_intrinsic_not_type = "linear-opaque:linear-intrinsic-not-type" (found: &str) => format!(
        "`linear` before `intrinsic` declares an opaque linear type, so `type` must \
         follow: `linear intrinsic type Reply<T>` (found {found})"
    );

    // [mod-export]
    /// `export` before an `import`.
    mod_export_export_import = "mod-export:export-import" () =>
        "`export` cannot precede an `import`: a module states what *it* declares, and \
         there is no re-export — import the original name where you need it";
    /// `export` before a `refn`.
    mod_export_export_refn = "mod-export:export-refn" () =>
        "`export` cannot precede a `refn`: a refinement travels with the qualifier whose \
         claim it is about, so it is visible wherever that qualifier is";
    /// `export` before a `rename`.
    mod_export_export_rename = "mod-export:export-rename" () =>
        "`export` cannot precede a `rename`: a rename is a name for this file's own use, \
         not a declaration — export the function it names instead";

    // [mod-namespace]
    /// A namespace name followed by neither `{` nor `...`.
    mod_namespace_expected_body = "mod-namespace:expected-body" (name: &str, found: &str) => format!(
        "expected `{{` or `...` after the namespace name `{name}`, found {found}"
    );
    /// A `namespace` block whose `}` is missing.
    mod_namespace_unterminated = "mod-namespace:unterminated" (name: &str) => format!(
        "unterminated `namespace {name}`: expected `}}`"
    );

    // [name-casing]
    /// A type's name that starts lowercase.
    name_casing_type_name_lowercase = "name-casing:type-name-lowercase" (what: &str) => format!(
        "{what} names must start with an uppercase letter"
    );
    /// A value's name that starts uppercase.
    name_casing_value_name_uppercase = "name-casing:value-name-uppercase" (what: &str) => format!(
        "{what} names must not start with an uppercase letter"
    );

    // [name-dot]
    /// A dot-name with more than two segments.
    name_dot_too_many_segments = "name-dot:too-many-segments" () =>
        "a dot-name has exactly two segments (`Ns.Name`)";

    // [net-noremote]
    /// `noremote` before a declaration that is not a type.
    net_noremote_not_before_type = "net-noremote:not-before-type" () =>
        "`noremote` keeps a *type* off the wire, so it precedes a `struct` or `type` \
         declaration — a function, handler or effect has no wire form to refuse";

    // [obligation-by]
    /// The retired `: auto Group<self>` spelling.
    obligation_by_auto_obligation = "obligation-by:auto-obligation" () =>
        "`: auto Group<self>` no longer exists: write `: Group<self> by auto` — the \
         members are stamped from `core.auto`'s `compfn`s";
    /// `by @x@y`, which says both readings of `by`.
    obligation_by_double_at = "obligation-by:double-at" () =>
        "`by @x@y` says both readings: `by @x` is the module `x`, `by x@y` the comptime \
         fn `x` of module `y`";

    // [platform-fn]
    /// A top-level `fn` without a body.
    platform_fn_missing_body = "platform-fn:missing-body" (name: &str) => format!(
        "`fn {name}` has no body: write one, or declare it `platform fn {name}` for the \
         host to implement"
    );
    /// A `platform fn` with a body.
    platform_fn_has_body = "platform-fn:has-body" (name: &str) => format!(
        "`platform fn {name}` has a body: a platform fn is implemented by the host, so it \
         is a signature only"
    );

    // [platform-iterable]
    /// `iterable platform` not followed by `type`.
    platform_iterable_not_before_type = "platform-iterable:not-before-type" (found: &str) => format!(
        "expected `type` after `iterable platform`, found {found}: native iteration is a \
         claim about a host type, so it belongs on a `platform type`"
    );

    // [platform-threadsafe]
    /// `threadsafe platform` followed by neither `handler` nor `type`.
    platform_threadsafe_not_before_handler_or_type = "platform-threadsafe:not-before-handler-or-type" (found: &str) => format!(
        "expected `handler` or `type` after `threadsafe platform`, found {found}: the \
         thread-safety contract is a claim about a host class, so it belongs on a \
         `platform handler` or a `platform type`"
    );

    // [platform-type]
    /// `linear platform` not followed by `type`.
    platform_type_linear_platform_not_type = "platform-type:linear-platform-not-type" (found: &str) => format!(
        "`linear platform` declares a linear host handle, so `type` must follow: `linear \
         platform type HostIn` (found {found})"
    );
    /// A `platform type` written as an alias.
    platform_type_alias = "platform-type:alias" (name: &str) => format!(
        "`platform type {name}` cannot be an alias: a platform type is an opaque handle \
         to the host class of the same name"
    );

    // [proj-infer]
    /// A `holds` with no sources.
    proj_infer_holds_without_sources = "proj-infer:holds-without-sources" () =>
        "`holds` needs the sources the result borrows: `T holds proj(a)`";

    // [qual-const]
    /// A qualifier constant that interpolates.
    qual_const_interpolated = "qual-const:interpolated" () =>
        "a constant in a qualifier cannot interpolate";

    // [qual-depend]
    /// A value slot on a declaration that is not a qualifier.
    qual_depend_value_slot_outside_qualifier = "qual-depend:value-slot-outside-qualifier" () =>
        "a value slot belongs to a qualifier, not a type: only a claim can depend on \
         another value";

    // [qual-lift]
    /// An `is` check that lifts some of its qualifiers but not all.
    qual_lift_partial = "qual-lift:partial" () =>
        "an `is` check either lifts every qualifier or none: mark them all with `^` (`is \
         ^Mut ^NonEmpty`), or split the check";
    /// A `when` branch head that lifts some of its qualifiers but not all.
    qual_lift_partial_branch = "qual-lift:partial-branch" () =>
        "a branch head either lifts every qualifier or none: mark them all with `^`, or \
         split the branch";

    // [qual-refn]
    /// A `refn` that declares effects.
    qual_refn_effects = "qual-refn:effects" (name: &str) => format!(
        "a refinement cannot declare effects: `refn {name}` only states what is known \
         about the arguments afterwards, so write `=> param: +Qual` here"
    );
    /// A `refn` that declares a return type.
    qual_refn_return_type = "qual-refn:return-type" (name: &str, param: &str) => format!(
        "a refinement cannot declare a return type: `refn {name}` refines an existing \
         function's deductions, and the function decides what it returns; write `=> \
         {param}: +Qual` (or `-> {param} is Qual` for a Bool predicate)"
    );
    /// A `refn` without a deduction clause.
    qual_refn_missing_deductions = "qual-refn:missing-deductions" (name: &str, param: &str) => format!(
        "expected a deduction clause in `refn {name}`, e.g. `=> {param}: +Qual`: a \
         refinement exists to state one"
    );
    /// A `refn` entry that names no qualifier.
    qual_refn_empty_entry = "qual-refn:empty-entry" (param: &str) => format!(
        "`{param}` states nothing: a refinement entry adds or removes qualifiers, e.g. \
         `[{param}: +Qual]`"
    );
    /// A `refn` qualifier written without `+` or `-`.
    qual_refn_unsigned_qualifier = "qual-refn:unsigned-qualifier" (name: &str) => format!(
        "a refinement's qualifiers need a sign: write `+{name}` if the call establishes \
         it or `-{name}` if it invalidates it (a plain name would mean \"only this \
         survives\", which is the function's own deduction to make)"
    );

    // [qual-refn-scope]
    /// A `refn` outside the qualifier it refines.
    qual_refn_scope_top_level_refn = "qual-refn-scope:top-level-refn" (name: &str) => format!(
        "a refinement belongs to the qualifier whose claim it is about: move `refn \
         {name}` into that qualifier's body"
    );

    // [qual-value-arg]
    /// A slot declared in the `<…>` list instead of a `(…)` block.
    qual_value_arg_slot_in_generics = "qual-value-arg:slot-in-generics" () =>
        "a slot is declared in a `(…)` block after the type generics — `Sorted<T>(?cmp: \
         (T, T) -> Int)` — not in the `<…>` list";
    /// A `?` value argument in the `<…>` list instead of a `(…)` block.
    qual_value_arg_value_arg_in_generics = "qual-value-arg:value-arg-in-generics" () =>
        "`?` names a value argument, which lives in a `(…)` block after the type \
         generics: `Sorted(?cmp)`, `Heap<Person>(?cmp)`";

    // [qual-whynot]
    /// A `whynot?` whose argument is not an `is` test.
    qual_whynot_not_is_test = "qual-whynot:not-is-test" () =>
        "`whynot?` explains an `is` test — `whynot?(port is InRange(1, 65535))`";

    // [struct-field-canbe-mut]
    /// `canbe` on a field type followed by something other than `Mut`.
    struct_field_canbe_mut_not_mut = "struct-field-canbe-mut:not-mut" (name: &str) => format!(
        "only `Mut` may follow `canbe` on a field type, found `{name}`"
    );

    // [struct-opaque]
    /// `opaque` before a declaration that is not a `struct`.
    struct_opaque_not_before_struct = "struct-opaque:not-before-struct" () =>
        "`opaque` hides a struct's fields outside its module, so it precedes a `struct` \
         declaration";

    // [syntax-expected]
    /// A token other than the one the grammar requires here.
    syntax_expected_token = "syntax-expected:token" (expected: &str, found: &str) => format!(
        "expected {expected}, found {found}"
    );
    /// An identifier was required.
    syntax_expected_identifier = "syntax-expected:identifier" (found: &str) => format!(
        "expected identifier, found {found}"
    );
    /// `intrinsic` followed by something it cannot modify.
    syntax_expected_after_intrinsic = "syntax-expected:after-intrinsic" (found: &str) => format!(
        "expected `type`, `fn`, `qualifier`, or `handler` after `intrinsic`, found {found}"
    );
    /// `provenance` not followed by `qualifier`.
    syntax_expected_after_provenance = "syntax-expected:after-provenance" (found: &str) => format!(
        "expected `qualifier` after `provenance`, found {found}"
    );
    /// A token that starts no top-level declaration.
    syntax_expected_item = "syntax-expected:item" (found: &str) => format!(
        "expected item, found {found}"
    );
    /// A contextual keyword was required.
    syntax_expected_word = "syntax-expected:word" (word: &str, found: &str) => format!(
        "expected `{word}`, found {found}"
    );
    /// `platform` followed by something it cannot modify.
    syntax_expected_after_platform = "syntax-expected:after-platform" (found: &str) => format!(
        "expected `handler`, `fn` or `type` after `platform`, found {found}: a platform \
         declaration is a host implementation of a Salvo effect (`platform handler`), of \
         one function (`platform fn`), or a host object's handle (`platform type`)"
    );
    /// A function type's effect list not followed by `->`.
    syntax_expected_fn_type_arrow = "syntax-expected:fn-type-arrow" () =>
        "expected `->` after effect list in function type";
    /// An `is` not followed by a type.
    syntax_expected_type_after_is = "syntax-expected:type-after-is" (found: &str) => format!(
        "expected type after `is`, found {found}"
    );
    /// A token that starts no expression.
    syntax_expected_expression = "syntax-expected:expression" (found: &str) => format!(
        "expected expression, found {found}"
    );
    /// Tokens left over after an interpolation's expression.
    syntax_expected_in_interpolation = "syntax-expected:in-interpolation" (found: &str) => format!(
        "unexpected {found} in string interpolation"
    );

    // [test-actor]
    /// An actor test's argument that is not an integer literal.
    test_actor_non_integer_argument = "test-actor:non-integer-argument" (arg: &str) => format!(
        "an actor test's `{arg}` must be an integer literal"
    );
    /// An actor test's argument other than `seed`.
    test_actor_unknown_argument = "test-actor:unknown-argument" (other: &str) => format!(
        "an actor test takes `seed`, not `{other}`"
    );

    // [test-decl]
    /// `export` before a `test`.
    test_decl_exported_test = "test-decl:exported-test" () =>
        "`export` cannot precede a `test`: a test is run, never referenced, so there is \
         nothing to make visible";
    /// A test whose name is not a string literal.
    test_decl_name_not_string = "test-decl:name-not-string" (found: &str) => format!(
        "a test's name must be a string literal, found {found}"
    );
    /// A test whose name interpolates.
    test_decl_interpolated_name = "test-decl:interpolated-name" () =>
        "a test's name must be a plain string literal: `salvo test` lists and filters \
         tests without running them, so an interpolated name could not be known";

    // [throw-try]
    /// A `try` not followed by a block.
    throw_try_missing_block = "throw-try:missing-block" () =>
        "`try` takes a block: `try { ... }` (or `try ...`)";

    // [type-alias]
    /// A `linear` type alias.
    type_alias_linear_alias = "type-alias:linear-alias" () =>
        "an alias cannot be linear: the obligation belongs to the type itself, so write \
         `linear intrinsic type` (or `linear struct`) where it is declared";

    // [type-literal]
    /// A literal type that interpolates.
    type_literal_interpolated = "type-literal:interpolated" () =>
        "a literal type cannot interpolate";
    /// A float literal used as a type.
    type_literal_float = "type-literal:float" () =>
        "a float cannot be a literal type: equality on floats does not name a value";

    // [waitfor-effect]
    /// `waitfor` in an effect list.
    waitfor_effect_declared = "waitfor-effect:declared" () =>
        "`waitfor` is not a declarable effect: occupancy is inferred (a wait needs no \
         capability since 2026-09-19) — delete it from this list";

    // [when-condition]
    /// A `when` branch after its `else`.
    when_condition_else_not_last = "when-condition:else-not-last" () =>
        "`else` is the last branch of a `when`: it runs when no condition above it held";
    /// A subject-less `when` without an `else`.
    when_condition_missing_else = "when-condition:missing-else" () =>
        "a subject-less `when` must end with an `else`: `when` is always exhaustive, and \
         with no subject the `else` is what makes it so (use `if`/`elif` when there is \
         nothing to fall back to)";
    /// A `when` with only an `else`.
    when_condition_only_else = "when-condition:only-else" () =>
        "a `when` with only an `else` has nothing to decide: write the block on its own";

    // [when-exhaustive]
    /// An `else` on a `[when]` with a subject.
    when_exhaustive_comptime_when_else = "when-exhaustive:comptime-when-else" (remedy: &str) => format!(
        "a `[when]` with a subject takes no `else`, as a `when` with one does not: {remedy}"
    );

    // [when-union-subject]
    /// An `else` on a `when` over a subject.
    when_union_subject_else = "when-union-subject:else" () =>
        "`when` over a subject is exhaustive over the union's arms, so it takes no \
         `else`: drop the subject to write a condition chain (`when { cond { … } else { … \
         } }`)";

    // ===== desugar =====

    // [iter-fn]
    /// An `iter fn` without a body.
    iter_fn_missing_body = "iter-fn:missing-body" () =>
        "an `iter fn` needs a body: it *is* the `next` the compiler would otherwise \
         generate";
    /// An `iter fn` with implicit parameters.
    iter_fn_implicit_params = "iter-fn:implicit-params" () =>
        "an `iter fn` cannot take implicit parameters yet: write the iterator struct and \
         its `next` by hand, or take the iterator as a concrete type";
    /// An `iter fn` with a variadic parameter.
    iter_fn_variadic_param = "iter-fn:variadic-param" (param: &str) => format!(
        "an `iter fn` cannot take a variadic parameter (`...{param}`): the iterator \
         struct holds each parameter as a field"
    );
    /// An `iter fn` parameter whose type is not a named type.
    iter_fn_unnamed_param_type = "iter-fn:unnamed-param-type" (param: &str) => format!(
        "an `iter fn`'s parameter `{param}` must be a named type — the iterator struct \
         holds it as a field; wrap an array, tuple or union in a struct of your own"
    );
    /// An `iter fn` parameter with a qualifier.
    iter_fn_qualified_param = "iter-fn:qualified-param" (qualifier: &str, param: &str) => format!(
        "an `iter fn`'s parameter is read, not advanced: drop the `{qualifier}` from \
         `{param}` — the generated iterator struct holds the position, and each call \
         mints a fresh one"
    );
    /// An `iter fn` whose result is not `Emitted T | Finished`.
    iter_fn_return_type = "iter-fn:return-type" () =>
        "an `iter fn` returns `Emitted T | Finished`: it reports either an element or the \
         end of the sequence";
    /// An `iter fn` state field named like a parameter.
    iter_fn_state_shadows_param = "iter-fn:state-shadows-param" (field: &str) => format!(
        "`{field}` is a parameter's name: a `state` field may not shadow it"
    );
    /// An `iter fn` state field declared twice.
    iter_fn_duplicate_state_field = "iter-fn:duplicate-state-field" (field: &str) => format!(
        "`state` field `{field}` is declared twice"
    );
    /// A binding in an `iter fn`'s body that reuses a parameter or state field name.
    iter_fn_shadowing_binding = "iter-fn:shadowing-binding" (name: &str, iter_fn: &str) => format!(
        "`{name}` is already a name of `iter fn {iter_fn}` (a parameter or a `state` \
         field), and a binding may not reuse it: rename the binding"
    );

    // [mod-namespace]
    /// A file- or module-scoped item (an import, refn, rename, test or effects alias) in a `namespace`.
    mod_namespace_not_a_declaration = "mod-namespace:not-a-declaration" (what: &str) => format!(
        "a `{what}` cannot appear inside a `namespace`: it is file- or module-scoped, not a \
         named declaration"
    );

    // [test-unique]
    /// Two tests with one name in a file.
    test_unique_duplicate = "test-unique:duplicate" (name: &str) => format!(
        "a test called `{name}` is already declared in this file: test names identify a \
         test to the runner, so they have to differ"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    /// [diag-id] Every message's function is named after its id, and every id
    /// is well formed.
    #[test]
    fn every_message_is_named_after_its_id() {
        for (name, id) in ALL {
            assert!(id_well_formed(id), "`{id}` is not `label:name`");
            assert!(name_matches_id(name, id), "`{name}` is not named after `{id}`");
        }
        assert!(name_matches_id("op_unique_shape_bare_type_param", "op-unique-shape:bare-type-param"));
        assert!(!id_well_formed("op-unique-shape"));
        assert!(!id_well_formed("Op-x:y"));
    }
}
