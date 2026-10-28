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

    // ===== expand =====

    // [test-file]
    /// A `test` block in a production source file.
    test_file_test_in_production = "test-file:test-in-production" (annex: &str) => format!(
        "a `test` block belongs in a test annex, not in a production source file: move \
         it to `{annex}`, which is part of this module and sees its private declarations"
    );

    // ===== comptime =====

    // [comptime-access]
    /// `.[b]` with an arm binder.
    comptime_access_arm_binder = "comptime-access:arm-binder" (b: &str) => format!(
        "`.[{b}]` reads a field, and `{b}` is an arm binder"
    );
    /// `.[b]` where `b` is no binder of an enclosing `[for …]`.
    comptime_access_not_a_binder = "comptime-access:not-a-binder" (b: &str) => format!(
        "`{b}` is not a binder of an enclosing `[for …]`"
    );
    /// `[b]:` where `b` is no field binder of an enclosing `[for …]`.
    comptime_access_not_a_field_binder = "comptime-access:not-a-field-binder" (b: &str) => format!(
        "`[{b}]:` names a field through a binder, and `{b}` is not a field binder of an \
         enclosing `[for …]`"
    );

    // [comptime-arm]
    /// A `None` arm called like a constructor.
    comptime_arm_none_called = "comptime-arm:none-called" () =>
        "a `None` arm is built with `None`, not called";
    /// An arm built by calling a type that has no constructor.
    comptime_arm_no_constructor = "comptime-arm:no-constructor" (ty: &dyn std::fmt::Display) => format!(
        "`{ty}` has no constructor to call: an arm built this way is a named type, \
         qualified or not; a `None` arm is `None`"
    );

    // [comptime-bound]
    /// A `by @module` whose module has the member, but not for this kind.
    comptime_bound_wrong_kind = "comptime-bound:wrong-kind" (module: &dyn std::fmt::Display, member: &str, kind: &str) => format!(
        "`{module}` has no `comptime fn {member}<T is {kind}>`: its `{member}` is not \
         written for a `{kind}`"
    );
    /// A module that declares one `comptime fn` for a kind twice.
    comptime_bound_duplicate = "comptime-bound:duplicate" (module: &dyn std::fmt::Display, member: &str, kind: &str) => format!(
        "`{module}` declares `comptime fn {member}<T is {kind}>` more than once"
    );

    // [comptime-default]
    /// `.default` read on an arm binder.
    comptime_default_arm_default = "comptime-default:arm-default" (binder: &str) => format!(
        "`{binder}` is an arm, and only a field has a default"
    );

    // [comptime-docs]
    /// `.docs` read on something that is neither a binder nor a type.
    comptime_docs_not_binder_or_type = "comptime-docs:not-binder-or-type" (name: &str) => format!(
        "`{name}` is neither a binder nor a type"
    );

    // [comptime-fields]
    /// A name used as a binder that no enclosing `[for …]` or `[when …]` binds.
    comptime_fields_not_a_binder = "comptime-fields:not-a-binder" (name: &str) => format!(
        "`{name}` is not a binder of an enclosing `[for …]` or `[when …]`"
    );
    /// A compile-time type root that names nothing walkable.
    comptime_fields_unknown_type = "comptime-fields:unknown-type" (name: &str) => format!(
        "`{name}` is not the bound type parameter, a binder's `.type`, or a declared \
         struct or union type"
    );
    /// `.arms` of a type that is not a union.
    comptime_fields_arms_of_non_union = "comptime-fields:arms-of-non-union" (ty: &dyn std::fmt::Display, kind: &str) => format!(
        "`{ty}` is a `{kind}`, and only a `Union` has `.arms`"
    );
    /// `.fields` of a type that is not a struct.
    comptime_fields_fields_of_non_struct = "comptime-fields:fields-of-non-struct" (ty: &dyn std::fmt::Display, kind: &str) => format!(
        "`{ty}` is a `{kind}`, and only a `Struct` has `.fields`"
    );
    /// A property a `Field`/`Arm` binder does not have.
    comptime_fields_unknown_property = "comptime-fields:unknown-property" (other: &str) => format!(
        "a `Field`/`Arm` has `.name`, `.kind`, `.type`, `.index`, `.first`, `.last`, \
         `.docs` and (a field) `.default`, not `.{other}`"
    );

    // [comptime-generic]
    /// A stamp at a generic type, not supported yet.
    comptime_generic_generic_target = "comptime-generic:generic-target" (target: &str, template: &str, param: &str, group: &str) => format!(
        "`{target}` is generic, and stamping `{template}` at a generic type is not \
         supported yet: the copy for a field of type `{param}` would need an implicit \
         for it — write the fulfilment by hand, with `?{group}<{param}>` (ROADMAP §2c)"
    );

    // [comptime-generic-fn]
    /// A generic struct's fields or arms walked without type arguments.
    comptime_generic_fn_uninstantiated = "comptime-generic-fn:uninstantiated" (name: &str, what: &str, wanted: &str) => format!(
        "`{name}` is generic, so its {what} are walked at an instance: write \
         `{name}<{wanted}>.{what}` with the type arguments meant (a type parameter of \
         this fn, or a concrete type)"
    );
    /// A generic struct walked with the wrong number of type arguments.
    comptime_generic_fn_type_arg_count = "comptime-generic-fn:type-arg-count" (name: &str, params: usize, given: usize) => format!(
        "`{name}` takes {params} type argument(s), found {given}"
    );

    // [comptime-inline]
    /// A compile-time type test naming an unknown type.
    comptime_inline_unknown_type = "comptime-inline:unknown-type" (name: &str) => format!(
        "unknown type `{name}` in a compile-time type test"
    );
    /// A comptime condition's subject that is not a binder.
    comptime_inline_not_a_binder = "comptime-inline:not-a-binder" (name: &str) => format!(
        "`{name}` is not a binder"
    );
    /// An `.index` comparison with a side that is not a binder.
    comptime_inline_non_binder_index_compare = "comptime-inline:non-binder-index-compare" () =>
        "both sides of an `.index` comparison must be binders";
    /// A `[when …]` over a kind meeting a type of a kind it has no arm for.
    comptime_inline_unhandled_kind = "comptime-inline:unhandled-kind" (ty: &dyn std::fmt::Display, kind: &str) => format!(
        "`{ty}` is a `{kind}`, which this `[when …]` does not handle"
    );
    /// A `[when value]` over something other than a field read or parameter.
    comptime_inline_when_value_subject = "comptime-inline:when-value-subject" () =>
        "a `[when value]` dispatches on a field read (`v.[field]`) or a parameter of the \
         bound type";
    /// A `[when value]` over a value that is not a union.
    comptime_inline_when_value_not_union = "comptime-inline:when-value-not-union" (ty: &dyn std::fmt::Display) => format!(
        "`{ty}` is not a `Union`, so there are no arms to dispatch on"
    );
    /// A `[when value]` over a union with an arm `is` cannot test.
    comptime_inline_unnamed_arm = "comptime-inline:unnamed-arm" (arm: &dyn std::fmt::Display, ty: &dyn std::fmt::Display) => format!(
        "arm `{arm}` of `{ty}` cannot be dispatched on: only a named arm can be tested \
         with `is`"
    );

    // [comptime-is-splice]
    /// An `is [b.type]` whose type is not a named type.
    comptime_is_splice_untestable_type = "comptime-is-splice:untestable-type" (b: &str, ty: &dyn std::fmt::Display) => format!(
        "`{b}.type` is `{ty}` here, which an `is` cannot test: an `is` tests a named \
         type, qualified or not"
    );

    // [comptime-kind]
    /// A kind word used after `is` as if it were a type.
    comptime_kind_kind_as_type = "comptime-kind:kind-as-type" (name: &str) => format!(
        "`{name}` is a kind, not a type: a kind is tested on `.kind` (`field.kind is \
         {name}`), and `is` after a type compares types"
    );
    /// `.kind` read as a value.
    comptime_kind_kind_as_value = "comptime-kind:kind-as-value" (root: &str) => format!(
        "`{root}.kind` is tested in a comptime head (`[if {root}.kind is Struct]`, `[when \
         {root}.kind]`), not read as a value"
    );

    // [comptime-literals]
    /// `.literals` of a type that is not a literal union.
    comptime_literals_not_literal_union = "comptime-literals:not-literal-union" (ty: &dyn std::fmt::Display) => format!(
        "`{ty}` has no literals: `.literals` walks a literal union's members (`\"json\" \
         | \"text\"`)"
    );
    /// `.value` read on a binder that is not a literal member.
    comptime_literals_value_outside_literals = "comptime-literals:value-outside-literals" (root: &str) => format!(
        "`{root}.value` is a literal member's, read in a `[for … in x.literals]`"
    );

    // [comptime-name]
    /// A name compared to a binder's `.name` that its sequence does not have.
    comptime_name_unknown_name = "comptime-name:unknown-name" (owner: &str, is_arm: bool, lit: &str) => format!(
        "`{owner}` has no {} `{lit}`",
        if is_arm { "arm" } else { "field" }
    );
    /// A `.name` comparison with a side that is not a binder.
    comptime_name_non_binder_compare = "comptime-name:non-binder-compare" () =>
        "both sides of a `.name` comparison must be binders";
    /// A `[when b.name]` whose range is known only where the template is stamped.
    comptime_name_when_over_derived_name = "comptime-name:when-over-derived-name" (b: &str, bound: &str, f: &str) => format!(
        "a `[when {b}.name]` cannot be checked here: `{b}.name` ranges over the                              names `{bound}` has, which are known only where `{f}` is stamped, and a                              `[when]` with a subject must cover its range — test with `[if {b}.name is …]`"
    );
    /// A `[when b.name]` whose binder no enclosing `[for …]` binds.
    comptime_name_not_a_binder = "comptime-name:not-a-binder" (name: &str) => format!(
        "`{name}` is not a binder of an enclosing `[for …]`"
    );
    /// A `[when b.name]` arm naming what an earlier arm covers.
    comptime_name_covered_twice = "comptime-name:covered-twice" (lit: &str, binder: &str) => format!(
        "`{lit}` is already covered by an earlier arm of this `[when {binder}.name]`"
    );
    /// A `[when b.name]` that leaves names of the sequence out.
    comptime_name_missing_names = "comptime-name:missing-names" (binder: &str, is_arm: bool, owner: &str, missing: &str) => format!(
        "this `[when {binder}.name]` does not cover every {} of `{owner}`: missing \
         {missing} — add them",
        if is_arm { "arm" } else { "field" }
    );

    // [comptime-pattern]
    /// A `[when T]` whose range is known only where the template is stamped.
    comptime_pattern_when_over_derived_type = "comptime-pattern:when-over-derived-type" (subject: &str, bound: &str, f: &str, root: &str) => format!(
        "a `[when {subject}]` cannot be checked here: `{subject}` ranges over the                              types `{bound}` has, which are known only where `{f}` is stamped, and a                              `[when]` with a subject must cover its range — dispatch on `[when {root}.kind]`,                              or test with `[if {subject} is …]`"
    );
    /// A pattern binder no copy used (a warning).
    comptime_pattern_unused_binder = "comptime-pattern:unused-binder" (name: &str) => format!(
        "pattern binder `{name}` is never used: write `*` in its place"
    );
    /// Pattern binders under `||` or `!`.
    comptime_pattern_binder_under_or_not = "comptime-pattern:binder-under-or-not" () =>
        "a pattern's binders bind nothing under `||` or `!` — what the test proves holds \
         only where it did: write `*` there";
    /// A pattern binder declared twice in one condition.
    comptime_pattern_duplicate_binder = "comptime-pattern:duplicate-binder" (name: &str) => format!(
        "pattern binder `{name}` is declared twice in one condition"
    );
    /// A pattern binder that would hide a type, type parameter or binder.
    comptime_pattern_binder_hides_name = "comptime-pattern:binder-hides-name" (name: &str) => format!(
        "pattern binder `{name}` would hide a type, type parameter or binder of                          that name: pick a fresh one"
    );
    /// A `[when T]` with no arm for the type `T` is at this stamp.
    comptime_pattern_no_arm_matches = "comptime-pattern:no-arm-matches" (subject: &str, ty: &dyn std::fmt::Display) => format!(
        "no arm of this `[when {subject}]` matches `{ty}`, which `{subject}` is here: add \
         an arm for it (`is *` matches every type the arms above it did not)"
    );

    // [comptime-qualifier]
    /// A test naming a qualifier that is not a comptime one.
    comptime_qualifier_not_comptime = "comptime-qualifier:not-comptime" (name: &str) => format!(
        "`{name}` is not a comptime qualifier"
    );
    /// A comptime qualifier tested on the wrong kind of declaration.
    comptime_qualifier_wrong_subject = "comptime-qualifier:wrong-subject" (name: &str, of: &str, subject: &str) => format!(
        "`{name}` describes a `{of}`, not a `{subject}`"
    );
    /// A comptime qualifier pattern binder spelled like a type.
    comptime_qualifier_type_binder = "comptime-qualifier:type-binder" (name: &str) => format!(
        "`{name}` would bind a type, and a comptime qualifier holds constants: name a \
         constant binder in lowercase"
    );
    /// A comptime qualifier test with the wrong number of constants.
    comptime_qualifier_constant_count = "comptime-qualifier:constant-count" (name: &str, slots: usize, found: usize) => format!(
        "`{name}` takes {slots} constant(s), found {found}"
    );
    /// A comptime qualifier's pattern argument that is neither a binder nor a constant.
    comptime_qualifier_pattern_argument = "comptime-qualifier:pattern-argument" (pattern: &dyn std::fmt::Display) => format!(
        "a qualifier's argument in a pattern is a constant binder declared before it \
         (`<c> Short(c)`) or a constant, not `{pattern}`"
    );
    /// A comptime qualifier tested on a binder's `.type`.
    comptime_qualifier_on_type = "comptime-qualifier:on-type" (qual: &str, root: &str) => format!(
        "`{qual}` describes a declaration, not a type: test the binder itself (`{root} is \
         {qual}`)"
    );
    /// A comptime qualifier tested on a type that is not a struct.
    comptime_qualifier_not_a_struct = "comptime-qualifier:not-a-struct" (root: &str) => format!(
        "`{root}` is not a struct, so it carries no comptime qualifiers"
    );
    /// A comptime qualifier tested on an arm binder.
    comptime_qualifier_on_arm = "comptime-qualifier:on-arm" (name: &str) => format!(
        "`{name}` is an arm, and an arm carries no comptime qualifiers yet"
    );

    // [comptime-refuse]
    /// A `refuse` message that is not literal text.
    comptime_refuse_non_literal_message = "comptime-refuse:non-literal-message" () =>
        "a `refuse` message is literal text, with `${T.name}`-style names substituted";
    /// A `refuse` reached at a stamp: its text, after what refused it (if anything).
    comptime_refuse_refused = "comptime-refuse:refused" (at: &str, text: &str) =>
        if at.is_empty() { text.to_string() } else { format!("`{at}` refused: {text}") };

    // [fn-by]
    /// A `fn … by` with no parameter to stamp at.
    fn_by_no_param = "fn-by:no-param" (name: &str) => format!(
        "`fn {name}` says `by`, but has no parameter to say which type it is stamped at"
    );
    /// A `fn … by` whose first parameter names no struct or union.
    fn_by_first_param_not_target = "fn-by:first-param-not-target" (name: &str) => format!(
        "`fn {name}` is stamped at the type of its first parameter, which has to name a \
         struct or a declared union type"
    );
    /// A `fn … by` stamped at a type that is opaque here.
    fn_by_opaque_target = "fn-by:opaque-target" (name: &str, target: &str) => format!(
        "`fn {name}` is stamped at `{target}`, which is opaque outside its module: its \
         fields are not this module's to walk, so stamp it where `{target}` is declared"
    );
    /// A `fn … by` stamped at a type that is not a struct or declared union.
    fn_by_target_not_struct_or_union = "fn-by:target-not-struct-or-union" (name: &str, target: &str) => format!(
        "`fn {name}` is stamped at `{target}`, which is not a struct or a declared union \
         type"
    );
    /// A `by` naming several comptime fns of one name for a kind.
    fn_by_ambiguous_compfn = "fn-by:ambiguous-compfn" (by: &str, count: usize, kind: &str, places: &str) => format!(
        "`by {by}` names {count} comptime fns of that name for a `{kind}` (in {places}); \
         select one with `by {by}@<module>`"
    );
    /// A declaration whose parameter count differs from the stamped one's.
    fn_by_param_count = "fn-by:param-count" (what: &str, declared: usize, by: &str, template: &str, target: &str, stamped: usize) => format!(
        "{what} takes {declared} parameter(s), but `{by}`'s `compfn {template}` stamped at \
         `{target}` takes {stamped}"
    );
    /// A declaration's parameter type that differs from the stamped one's.
    fn_by_param_type = "fn-by:param-type" (what: &str, param: &str, declared: &dyn std::fmt::Display, template: &str, stamped: &dyn std::fmt::Display) => format!(
        "{what}'s parameter `{param}` is `{declared}`, but the stamped `{template}` takes \
         `{stamped}` there"
    );
    /// A declaration's return type that differs from the stamped one's.
    fn_by_return_type = "fn-by:return-type" (what: &str, declared: &str, template: &str, stamped: &str) => format!(
        "{what} returns `{declared}`, but the stamped `{template}` returns `{stamped}`"
    );
    /// A declared parameter name that would capture a name the stamped body uses; `member` is the group and fn of a clause's member.
    fn_by_param_hides_name = "fn-by:param-hides-name" (name: &str, called: bool, by: &str, template: &str, member: Option<(&str, &str)>) => {
        let used = if called {
            format!("the function `{name}` that `{by}`'s `{template}` calls in its body")
        } else {
            format!("the name `{name}` that `{by}`'s `{template}` uses in its body")
        };
        match member {
            None => format!("parameter `{name}` hides {used}: name the parameter differently"),
            Some((group, f)) => format!(
                "`{group}<self>`'s `fn {f}` names its parameter `{name}`, which hides {used}: \
                 name the group member's parameter differently"
            ),
        }
    };

    // [obligation-by]
    /// A `by` clause whose group argument is not `self`.
    obligation_by_group_arg_not_self = "obligation-by:group-arg-not-self" (group: &str, by: &str) => format!(
        "`by` stamps the members for the type it is written on, so the group's argument \
         is `self`: write `: {group}<self> by {by}`"
    );
    /// A `by x` where `x` is both a module and a comptime fn.
    obligation_by_ambiguous_by = "obligation-by:ambiguous-by" (by: &str, module: &dyn std::fmt::Display, fn_module: &dyn std::fmt::Display) => format!(
        "`by {by}` is ambiguous: `{by}` is a module (`{module}`) and a `comptime fn` (in \
         `{fn_module}`). Write `by @{by}` for the module, or `by {by}@{fn_module}` for the \
         function"
    );
    /// A `by` path naming several modules.
    obligation_by_several_modules = "obligation-by:several-modules" (by: &str, count: usize, names: &str) => format!(
        "`by {by}` names {count} modules — {names}; write more of the path"
    );
    /// A `by @x` naming no module the file can see; `unseen` ends it.
    obligation_by_no_module = "obligation-by:no-module" (by: &str, unseen: &str) => format!(
        "`by @{by}` names no module this file can see{unseen}"
    );
    /// A `by` module without the member's comptime fn.
    obligation_by_module_lacks_member = "obligation-by:module-lacks-member" (module: &dyn std::fmt::Display, member: &str, by: &str) => format!(
        "`{module}` has no `comptime fn {member}`, so `by {by}` cannot stamp one"
    );
    /// A `by` naming neither a module nor a comptime fn; `unseen` ends it.
    obligation_by_names_nothing = "obligation-by:names-nothing" (by: &str, member: &str, unseen: &str) => format!(
        "`by {by}` names neither a module nor a `comptime fn` this file can see — nothing \
         to stamp `{member}` from{unseen}"
    );

    // [when-exhaustive]
    /// A `[when X.kind]` that leaves an arm of `Kind` out.
    when_exhaustive_kind_arms_missing = "when-exhaustive:kind-arms-missing" (missing: &str) => format!(
        "this `[when …]` over a kind does not consider every arm of `Kind`: missing \
         {missing} — add the arms (`is Opaque` covers `Basic` and `Generic`)"
    );

    // ===== resolve =====

    // [fn-attached]
    /// A private fn fulfilling an obligation of an exported type.
    fn_attached_private_fulfilment = "fn-attached:private-fulfilment" (name: &str, target: &str) => format!(
        "`{name}` fulfils an obligation of `{target}`, which is `export`ed, so it is \
         declared on an exported type and has to say so: write `export fn {name}(…)`. A \
         function declared on a type is imported with the type, and a private one would \
         be unreachable wherever the type is usable — or write it inside `{target}`'s \
         body, where the visibility is the type's"
    );

    // [fn-overload-return]
    /// Two fns of a module no call could tell apart.
    fn_overload_return_duplicate_fn = "fn-overload-return:duplicate-fn" (name: &str, params: &str, ret: &str, module: &dyn std::fmt::Display) => format!(
        "duplicate fn `{name}({params}) -> {ret}` in module `{module}`: an overload set is \
         distinguished by parameter *types*, or by the return type among overloads whose \
         parameters agree, so this one could never be called. Parameter names take no \
         part in choosing an overload"
    );

    // [iter-fn]
    /// An `iter fn` that reached resolution unexpanded (a driver bug).
    iter_fn_unexpanded = "iter-fn:unexpanded" () =>
        "internal: this `iter fn` reached resolution unexpanded — the driver that parsed \
         it skipped the `iter fn` expansion (`expand_iter_fns`); this is a \
         compiler-integration bug, not a mistake in this program";

    // [mod-collision]
    /// Two declarations of one kind and name in a module.
    mod_collision_duplicate = "mod-collision:duplicate" (kind: &str, name: &str, module: &dyn std::fmt::Display) => format!(
        "duplicate {kind} `{name}` in module `{module}`"
    );
    /// One kind and name declared by two implicitly visible core modules.
    mod_collision_core_modules = "mod-collision:core-modules" (kind: &str, name: &str, other: &dyn std::fmt::Display, module: &dyn std::fmt::Display) => format!(
        "{kind} `{name}` is declared in multiple implicitly visible core modules: \
         `{other}` and `{module}`"
    );
    /// An imported name that conflicts with a declaration (`existing` `None`) or another import.
    mod_collision_import_conflict = "mod-collision:import-conflict" (kind: &str, name: &str, module: &dyn std::fmt::Display, existing: Option<&dyn std::fmt::Display>) => format!(
        "{kind} `{name}` (imported from `{module}`) conflicts with {}; rename it with `as`",
        match existing {
            None => "a declaration in this module".to_string(),
            Some(m) => format!("the import from `{m}`"),
        }
    );

    // [mod-export]
    /// An import of a declaration its module does not export.
    mod_export_not_exported = "mod-export:not-exported" (modules: &str, item: &str) => format!(
        "module `{modules}` declares `{item}` but does not export it, so it cannot be \
         imported: write `export` in front of that declaration"
    );

    // [mod-import]
    /// An import path that is not `module.item`.
    mod_import_path_shape = "mod-import:path-shape" () =>
        "import path must be `module.item`";
    /// An import no module declares.
    mod_import_unresolved = "mod-import:unresolved" (prefix: &str, item: &str) => format!(
        "unresolved import: no module matching `{prefix}` declares `{item}`"
    );
    /// An import several modules declare.
    mod_import_ambiguous = "mod-import:ambiguous" (item: &str, modules: &str) => format!(
        "ambiguous import `{item}`: found in modules {modules}"
    );

    // [mod-import-module]
    /// A whole-module import with an alias.
    mod_import_module_alias = "mod-import-module:alias" (alias: &str) => format!(
        "a whole-module import has no alias: `as {alias}` renames one imported *name*, \
         and Salvo has no module-qualified type references to rename a module for. Import \
         the names you want to alias individually"
    );
    /// One type name carried by two whole-module imports (a warning).
    mod_import_module_two_bulk_imports = "mod-import-module:two-bulk-imports" (kind: &str, name: &str, existing: &dyn std::fmt::Display, module: &dyn std::fmt::Display) => format!(
        "{kind} `{name}` is carried by two whole-module imports (`{existing}` and \
         `{module}`); `{existing}`'s wins — import the other by name (`import \
         {module}.{name} as …`) to pick it"
    );
    /// A whole-module import of a path prefix that has modules under it.
    mod_import_module_path_prefix = "mod-import-module:path-prefix" (prefix: &str, under: &[String]) => format!(
        "no module `{prefix}` — it is a path prefix, not a module, and a whole-module \
         import names one module rather than the tree under it. Import the ones you want: \
         {}",
        under.iter().map(|m| format!("`import {m}`")).collect::<Vec<_>>().join(", ")
    );
    /// A single-segment import naming no module; `known` are the importable ones.
    mod_import_module_unknown_module = "mod-import-module:unknown-module" (prefix: &str, known: &[String]) => format!(
        "unknown module `{prefix}`: `import <module>` imports every name in a module{}",
        if known.is_empty() {
            String::new()
        } else {
            format!(" (importable modules: {})", known.join(", "))
        }
    );

    // [mod-namespace]
    /// A namespace import several modules declare.
    mod_namespace_ambiguous_import = "mod-namespace:ambiguous-import" (item: &str, modules: &str) => format!(
        "ambiguous namespace import `{item}`: found in modules {modules}"
    );
    /// A namespace import that brings nothing (a warning).
    mod_namespace_exports_nothing = "mod-namespace:exports-nothing" (ns: &str, module: &dyn std::fmt::Display) => format!(
        "namespace `{ns}` of `{module}` exports nothing, so this import brings no names"
    );

    // [mod-std-internal]
    /// An import of std's internal module from outside std.
    mod_std_internal_import = "mod-std-internal:import" (root: &str, internal: &str) => format!(
        "`{root}.{internal}` is the standard library's own module: the runtime every \
         program runs on, reachable only from std itself"
    );

    // [mod-visibility]
    /// An import of what is already visible (a warning); `sees_itself` when it may be the file's own module.
    mod_visibility_redundant_import = "mod-visibility:redundant-import" (module: &str, sees_itself: bool) => format!(
        "redundant import: everything in `{module}` is already visible here (`core.*` is \
         implicit{})",
        if sees_itself { ", and a module sees itself" } else { "" }
    );

    // [name-casing]
    /// A module path segment that starts uppercase.
    name_casing_module_segment = "name-casing:module-segment" (seg: &str) => format!(
        "module path segment `{seg}` must start with a lowercase letter: module paths are \
         file paths, so rename the file or directory"
    );

    // [name-dot]
    /// A dot-name namespaced by a generic declaration.
    name_dot_generic_namespace = "name-dot:generic-namespace" (ns: &str, name: &str) => format!(
        "`{ns}` is generic, so it cannot namespace `{name}`: Kotlin emits the member as a \
         nested class, which cannot use the outer type parameters"
    );
    /// A dot-name whose prefix is not a namespace.
    name_dot_not_a_namespace = "name-dot:not-a-namespace" (ns: &str, name: &str) => format!(
        "`{ns}` in the dot-name `{name}` is not a namespace here: declare `namespace {ns} \
         {{ … }}`, or a struct or defined `type {ns}` in this file"
    );
    /// A dot-name whose flattened spelling is also visible.
    name_dot_flattened_collision = "name-dot:flattened-collision" (name: &str, concatenated: &str) => format!(
        "the dot-name `{name}` collides with `{concatenated}`, which is also visible here: \
         the Rust backend flattens dot-names and overload mangling uses the same spelling \
         — rename one of them"
    );

    // [name-dot-import]
    /// An import of a dot-name with more than two segments.
    name_dot_import_too_many_segments = "name-dot-import:too-many-segments" () =>
        "a dot-name has exactly two segments (`module.Ns.Name`)";

    // [obligation-by]
    /// A fn a `by` clause stamps that the file also declares by hand.
    obligation_by_stamped_and_written = "obligation-by:stamped-and-written" (name: &str, params: &str, module: &dyn std::fmt::Display) => format!(
        "`{name}({params})` is declared twice in module `{module}`: a `by` clause stamps \
         it, and this file also declares it by hand. Keep one — remove the member's group \
         from the `by` clause, or delete the hand-written fn"
    );

    // [std-root]
    /// A module under `std` outside the standard library.
    std_root_reserved_module = "std-root:reserved-module" (module: &dyn std::fmt::Display, root: &str) => format!(
        "module `{module}` is under `{root}`, which is reserved for the standard library: \
         only a tree whose manifest says `[project] std = true` declares modules there — \
         move this file out of its `{root}/` directory"
    );
    /// An import of a std module without its `std.` root.
    std_root_bare_std_import = "std-root:bare-std-import" (root: &str, path: &str) => format!(
        "the standard library's modules are under `{root}`: write `import {root}.{path}`"
    );

    // [test-file]
    /// A `test` block that reached resolution unexpanded (a driver bug).
    test_file_unexpanded = "test-file:unexpanded" () =>
        "internal: this `test` block reached resolution unexpanded — the driver that \
         parsed it skipped the program-level expansion (`salvo_core::expand`); this is a \
         compiler-integration bug, not a mistake in this program";

    // [type-literal]
    /// Overloads apart only by literal types of one base.
    type_literal_erased_overload = "type-literal:erased-overload" (name: &str, erased: &str) => format!(
        "`{name}` is overloaded on literal types of one base, and a literal is its base at \
         run time, so both would be `{name}({erased})` on either backend; take the union \
         of the literals in one fn and `when` over it"
    );

    // ===== unique shape =====

    // [op-unique-shape]
    /// A unique-shape fn outside its type's file. `declared`: the home declares the type (else std names it); `std_home`: the home is std's and this file is not.
    op_unique_shape_outside_home = "op-unique-shape:outside-home" (sig: &str, shape: &str, what: &str, ops: &str, plural: bool, declared: bool, home: &str, std_home: bool) => format!(
        "`{sig}` has the shape of `{shape}` on {what}, which {ops} {}, and a function of \
         that shape is declared only in {}, {home}: so nothing elsewhere can change what \
         {ops} {} on {what}. {} where it is wanted (`{shape} = by_name`)",
        if plural { "call" } else { "calls" },
        if declared {
            format!("the file that declares {what}")
        } else {
            format!("the file std names as the home of {what}")
        },
        if plural { "mean" } else { "means" },
        if std_home {
            "Give this one another name and pass it explicitly"
        } else {
            "Move this one into that file, or give it another name and pass it explicitly"
        }
    );
    /// A unique-shape fn over a bare type parameter.
    op_unique_shape_bare_type_param = "op-unique-shape:bare-type-param" (sig: &str, shape: &str, param: &str, ops: &str, plural: bool) => format!(
        "`{sig}` has the shape of `{shape}` on the type parameter `{param}`, which has no \
         declaring file: it would be the `{shape}` of every type at once, which {ops} {}. \
         A function of a unique shape is declared in its type's own file; give this one \
         another name and pass it explicitly where it is wanted (`{shape} = by_name`)",
        if plural { "call" } else { "calls" }
    );

    // ===== refine =====

    // [qual-depend]
    /// A `preserve` naming a qualifier with no value slots.
    qual_depend_preserve_not_dependent = "qual-depend:preserve-not-dependent" (q: &str) => format!(
        "`{q}` is not a dependent qualifier: `preserve` speaks about claims other values \
         hold, which only a qualifier with value slots can be"
    );

    // [qual-of]
    /// A refined qualifier whose `of` type is not the parameter's.
    qual_of_wrong_subject = "qual-of:wrong-subject" (q: &str, of: &dyn std::fmt::Display, base: &str, refn: &str, param: &str, ty: &dyn std::fmt::Display) => format!(
        "`{q}` applies to `{of}`, not to `{base}`: `{refn}` is declared `{param}: {ty}`"
    );

    // [qual-preserve]
    /// A `preserve` parameter no value slot of the qualifier accepts.
    qual_preserve_no_value_slot = "qual-preserve:no-value-slot" (q: &str, ty: &dyn std::fmt::Display) => format!(
        "no value slot of `{q}` accepts a `{ty}`: `preserve` names the parameter the \
         claims depend on"
    );

    // [qual-refn]
    /// A refinement deduction naming one parameter twice.
    qual_refn_duplicate_param = "qual-refn:duplicate-param" (param: &str) => format!(
        "`{param}` appears twice in this refinement's deductions"
    );
    /// A refinement deduction naming no parameter of the refinement.
    qual_refn_unknown_param = "qual-refn:unknown-param" (param: &str) => format!(
        "`{param}` is not a parameter of this refinement"
    );
    /// A refinement deduction that states nothing.
    qual_refn_states_nothing = "qual-refn:states-nothing" (param: &str) => format!(
        "`{param}` states nothing: write `+Qual` if the call establishes a qualifier, \
         `-Qual` if it invalidates one"
    );
    /// A refinement of a parameter the refined fn moves.
    qual_refn_moved_param = "qual-refn:moved-param" (name: &str, param: &str) => format!(
        "`{name}` moves `{param}`, so a caller knows nothing about it afterwards — there \
         is nothing to refine"
    );
    /// A refinement entry that both establishes and invalidates a qualifier.
    qual_refn_entry_contradiction = "qual-refn:entry-contradiction" (q: &str, param: &str) => format!(
        "this refinement both establishes and invalidates `{q}` for `{param}`"
    );
    /// A refinement that both establishes and invalidates a qualifier at one parameter position.
    qual_refn_position_contradiction = "qual-refn:position-contradiction" (q: &str, position: usize) => format!(
        "this refinement both establishes and invalidates `{q}` for parameter {position} — \
         it cannot do both"
    );
    /// A qualifier's refinement speaking about another qualifier's claim.
    qual_refn_foreign_qualifier = "qual-refn:foreign-qualifier" (owner: &str, verb: &str, q: &str) => format!(
        "a refinement declared by `{owner}` can only {verb} `{owner}`, not `{q}`: a \
         qualifier states what happens to *its own* claim. A call that needs both \
         statements names the place it means (`f@place`)"
    );
    /// A refinement naming an unknown qualifier.
    qual_refn_unknown_qualifier = "qual-refn:unknown-qualifier" (q: &str) => format!(
        "unknown qualifier `{q}`"
    );

    // [qual-refn-match]
    /// An unknown type in a refinement's parameter list.
    qual_refn_match_unknown_type = "qual-refn-match:unknown-type" (name: &str, refn: &str) => format!(
        "unknown type `{name}` in this refinement's parameter list: if it is a type \
         parameter of the function being refined, declare it here too (`refn \
         {refn}<{name}>(...)`)"
    );
    /// A refinement of a name no function in scope has.
    qual_refn_match_no_function = "qual-refn-match:no-function" (name: &str) => format!(
        "no function `{name}` is visible here, so there is nothing to refine: a \
         refinement names an existing function and repeats its parameter list to pick \
         one overload"
    );
    /// A refinement whose parameters line up with no overload in scope.
    qual_refn_match_no_overload = "qual-refn-match:no-overload" (name: &str, shapes: &str) => format!(
        "this refinement matches no `{name}` in scope: its parameters must line up \
         positionally with an overload's (each base type a super- or subtype of the \
         other, `*` for the rest). In scope: {shapes}"
    );

    // [qual-subject]
    /// A refinement of one of the compiler's own qualifiers.
    qual_subject_compiler_qualifier = "qual-subject:compiler-qualifier" (q: &str, verb: &str) => format!(
        "`{q}` is one of the compiler's own qualifiers: a refinement may only {verb} a \
         state qualifier, since it changes what is *known* about a value and not what may \
         be done with it"
    );
    /// A refinement of a provenance qualifier.
    qual_subject_provenance = "qual-subject:provenance" (q: &str) => format!(
        "`{q}` is a provenance qualifier, which no call can invalidate — so there is \
         nothing for a refinement to say about it; only state qualifiers can be refined"
    );

    // ===== deduce =====

    // [deduce-default]
    /// A written entry keeps a parameter the body moves.
    deduce_default_promised_back_moved = "deduce-default:promised-back-moved" (param: &str) => format!(
        "deduction promises `{param}` back to the caller, but the body moves it"
    );
    /// A defaulted parameter the body moves.
    deduce_default_kept_by_default_moved = "deduce-default:kept-by-default-moved" (param: &str) => format!(
        "`{param}` is kept by default, but the body moves it (returns it, stores it, or \
         passes it to a consuming call): write `=> !{param}` to declare the move, or \
         `copy(…)` at the move site to keep the caller's value"
    );

    // [deduce-gained]
    /// A promised qualifier the body may remove.
    deduce_gained_qualifier_removed = "deduce-gained:qualifier-removed" (q: &str, param: &str) => format!(
        "deduction promises qualifier `{q}` on `{param}`, but the body may remove it"
    );
    /// A kept qualifier the parameter does not declare and the body does not establish.
    deduce_gained_qualifier_not_established = "deduce-gained:qualifier-not-established" (q: &str, param: &str) => format!(
        "deduction keeps qualifier `{q}`, which is not declared on parameter `{param}` and \
         nothing in the body establishes it — a plain entry reports what the body leaves \
         behind, and `+{q}` (legal only in `{q}`'s own file) is how a function *claims* to \
         establish one"
    );
    /// A bodiless declaration keeping a qualifier its parameter does not declare.
    deduce_gained_bodiless = "deduce-gained:bodiless" (q: &str, param: &str) => format!(
        "deduction keeps qualifier `{q}`, which is not declared on parameter `{param}` (a \
         bodiless declaration has nothing to establish it; `+{q}` is how a function says \
         it *establishes* one)"
    );

    // [deduce-syntax]
    /// A deduction naming no parameter of the function.
    deduce_syntax_unknown_param = "deduce-syntax:unknown-param" (name: &str) => format!(
        "deduction names unknown parameter `{name}`"
    );
    /// Two deductions for one parameter.
    deduce_syntax_duplicate_param = "deduce-syntax:duplicate-param" (name: &str) => format!(
        "duplicate deduction for parameter `{name}`"
    );
    /// A keep-all deduction on a parameter the function mutates.
    deduce_syntax_mutated_keep_all = "deduce-syntax:mutated-keep-all" (name: &str, declared: &str) => format!(
        "`{name}` is mutated by this function, so the deduction cannot keep every \
         qualifier: mutation can invalidate qualifiers the caller has and this signature \
         never mentions. List exactly what survives (`[{name}: {declared}]`)"
    );
    /// A `-Q` deduction on a parameter the function mutates.
    deduce_syntax_mutated_remove = "deduce-syntax:mutated-remove" (name: &str, declared: &str) => format!(
        "`{name}` is mutated by this function, so the deduction cannot drop qualifiers \
         selectively: mutation can invalidate qualifiers the caller has and this \
         signature never mentions. List exactly what survives (`[{name}: {declared}]`)"
    );
    /// A deduction entry naming a type rather than a qualifier.
    deduce_syntax_type_narrowing = "deduce-syntax:type-narrowing" () =>
        "a deduction entry may only name qualifiers (and `Never`): type narrowing in \
         deductions is not supported yet";

    // ===== deadlock =====

    // [actor-deadlock-cycle]
    /// A cycle of actors whose members wait for each other.
    actor_deadlock_cycle_wait_cycle = "actor-deadlock-cycle:wait-cycle" (path: &str, handlers: &str) => format!(
        "these actors wait for each other: {path} ({handlers}). A handler whose member \
         waits serves no message while it waits, so each of them is waiting for an answer \
         the next can only produce after its own arrives — and a thread of its own does \
         not help, since the fulfilment routes back through a stalled mailbox. Break the \
         cycle: park a continuation with `replyto` instead of waiting, or have the answer \
         come from a third actor neither of them waits on"
    );
    /// A cycle of `replyto!` gates.
    actor_deadlock_cycle_gate_cycle = "actor-deadlock-cycle:gate-cycle" (path: &str, handlers: &str) => format!(
        "these actors wait for each other: {path} ({handlers}). A `replyto!` gate serves \
         nothing but its own answer, so each of them is waiting for a reply the next can \
         only produce after its own arrives — a deadlock whenever the requests cross. \
         Break the cycle: make one side's continuation ungated (`replyto`, whose mailbox \
         stays open), or have the answer come from a third actor neither of them waits on"
    );
    /// A cycle of sends, which deadlocks only when the mailboxes fill (a warning).
    actor_deadlock_cycle_send_cycle = "actor-deadlock-cycle:send-cycle" (path: &str, handlers: &str) => format!(
        "these actors send to each other in a cycle: {path} ({handlers}). A send blocks \
         while the target's mailbox is full, so this deadlocks if the mailboxes fill at \
         the same time — a failure that appears under load and not in a test. Give the \
         queues room (a larger `capacity`), or break the cycle by routing one direction \
         through a reply token, which has capacity reserved and never blocks"
    );

    // [actor-mixed-handler]
    /// A wait cycle through a mixed handler's servant.
    actor_mixed_handler_occupancy_cycle = "actor-mixed-handler:occupancy-cycle" (path: &str, handlers: &str) => format!(
        "these actors can wait for each other through a shared handler: {path} \
         ({handlers}). A sync member of a mixed handler parks its caller until the servant \
         answers, and a parked activation serves nothing of its own mailbox — so each \
         step of this cycle waits on the next, and the fulfilment routes back through a \
         stalled queue. Break the cycle: have the servant answer without reaching the \
         peer, respell the consulting call as a send plus a continuation, or bind a \
         handler of the effect that does not wait — a monitor, or a scope-local `use`"
    );

    // ===== platform =====

    // [platform-root]
    /// Platform declarations in a program with no `salvo.toml`.
    platform_root_no_manifest = "platform-root:no-manifest" (what: &str) => format!(
        "{what} is implemented in platform files, which are found through a platform root \
         named in the project's `salvo.toml` — and this program has no `salvo.toml`"
    );
    /// Platform declarations with no platform root for a backend.
    platform_root_no_root = "platform-root:no-root" (what: &str, backend: &str) => format!(
        "{what} is implemented in platform files, but `salvo.toml` names no platform root \
         for {backend}: add `[build] platform = \"…\"` (every backend) or `[{backend}] \
         platform = \"…\"`"
    );

    // [platform-tree]
    /// A module whose platform implementation file is missing.
    platform_tree_missing_host_file = "platform-tree:missing-host-file" (what: &str, module: &dyn std::fmt::Display, rel_path: &std::path::Path) => format!(
        "error: module `{module}` declares {what}, implemented by the host in `{}` under \
         the platform root, but that file does not exist: run `salvo platform generate` \
         to create the skeleton",
        rel_path.display()
    );

    // ===== naming =====

    // [fn-emit-name]
    /// Overloads no naming rule tells apart.
    fn_emit_name_indistinct_overloads = "fn-emit-name:indistinct-overloads" (module: &str, name: &str, which: &str) => format!(
        "module `{module}` declares overloads of `{name}` that no naming rule tells apart \
         ({which}): their parameter types, qualifiers, implicit parameters and return \
         types all agree, so neither backend can give them names of their own"
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
