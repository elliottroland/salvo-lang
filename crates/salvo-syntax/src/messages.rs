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

/// [diag-id] What a message says when it leaves the compiler as a bare line
/// of text (a codegen refusal, an IR build error): its text, led by `[id]`
/// when ids are shown (`salvo -v`).
pub fn rendered(m: &Message) -> String {
    if crate::diag::ids_shown() {
        format!("[{}] {}", m.id, m.text)
    } else {
        m.text.clone()
    }
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
    // ===== check =====

    // [actor-effect-kind]
    /// A plain effect bound to an actor.
    actor_effect_kind_plain_effect_actor = "actor-effect-kind:plain-effect-actor" (form: &dyn std::fmt::Display, name: &dyn std::fmt::Display) => format!(
        "`{form}` cannot bind `{name}` to an actor: it is a plain effect, and a \
         synchronous protocol has no mailbox — declare `actor effect {name}` (its \
         members then give up kept and `Mut` parameters, `proj` returns and non-sendable \
         payloads)"
    );

    // [actor-free-send-fn]
    /// A generic free `send fn`.
    actor_free_send_fn_generic = "actor-free-send-fn:generic" (fn_name: &dyn std::fmt::Display, generic: &dyn std::fmt::Display) => format!(
        "`send fn {fn_name}` cannot be generic: a mint carries captures and no type \
         arguments, so `{generic}` would have nothing to be chosen by"
    );
    /// A free `send fn` that keeps a parameter.
    actor_free_send_fn_kept_param = "actor-free-send-fn:kept-param" (fn_name: &dyn std::fmt::Display, param: &dyn std::fmt::Display) => format!(
        "`send fn {fn_name}` cannot keep `{param}`: a scheduled body outlives the frame \
         that minted it, so what it is given is always consumed — write `=> !{param}`, \
         or make `{fn_name}` an ordinary `fn`"
    );
    /// A free `send fn` with a `Mut` parameter.
    actor_free_send_fn_mut_param = "actor-free-send-fn:mut-param" (fn_name: &dyn std::fmt::Display) => format!(
        "`send fn {fn_name}` cannot take a `Mut` parameter: mutating a value from a \
         scheduled body would share what the minting frame still owns"
    );
    /// A free `send fn` taking a type that cannot cross.
    actor_free_send_fn_unsendable_param = "actor-free-send-fn:unsendable-param" (fn_name: &dyn std::fmt::Display, ty: &dyn std::fmt::Display, why: &dyn std::fmt::Display) => format!(
        "`send fn {fn_name}` cannot take `{ty}`: {why}, and everything a scheduled body \
         is given crosses a thread boundary"
    );
    /// A free `send fn` returning a `proj` view.
    actor_free_send_fn_borrowed_return = "actor-free-send-fn:borrowed-return" (fn_name: &dyn std::fmt::Display) => format!(
        "`send fn {fn_name}` cannot return a `proj` view: it would borrow a value the \
         minting frame owns"
    );
    /// A free `send fn` declaring an effect.
    actor_free_send_fn_effect = "actor-free-send-fn:effect" (fn_name: &dyn std::fmt::Display, eff: &dyn std::fmt::Display, why: &dyn std::fmt::Display) => format!(
        "`send fn {fn_name}` cannot declare `{eff}`: {why}. Take an `eff[…]` as a capture \
         and send to it — reaching an actor needs no effect declaration — or do that \
         work in the function that mints"
    );
    /// A `send fn` passed as a value.
    actor_free_send_fn_not_a_value = "actor-free-send-fn:not-a-value" (name: &dyn std::fmt::Display) => format!(
        "`{name}` is a `send fn`, so it is not a value: it runs by being scheduled, and a \
         function value runs by being called. Mint a continuation for it instead — \
         `replyto {name}(…)`"
    );
    /// A free `send fn` called directly.
    actor_free_send_fn_called = "actor-free-send-fn:called" (name: &dyn std::fmt::Display) => format!(
        "`{name}` is a `send fn`, so it runs by being scheduled rather than called: mint \
         a continuation for it with `replyto {name}(…)` and let whoever answers the \
         token run it"
    );

    // [actor-group]
    /// `in` of something that is not a group handle.
    actor_group_not_a_group = "actor-group:not-a-group" (gty: &dyn std::fmt::Display) => format!(
        "`in` takes a group handle — an `eff[ActorGroup<E>]` — and this is `{gty}`"
    );
    /// A spawn joining a group of none of its faces.
    actor_group_no_face = "actor-group:no-face" (of: &dyn std::fmt::Display, serves: &dyn std::fmt::Display) => format!(
        "this spawn answers no `eff[{of}]` to join the group with: it serves {serves} — \
         `in` needs a group of one of the handler's faces"
    );

    // [actor-mailbox]
    /// A handler with a mailbox that does not say how deep it is.
    actor_mailbox_no_capacity = "actor-mailbox:no-capacity" (handler: &dyn std::fmt::Display, why: &dyn std::fmt::Display) => format!(
        "`{handler}` has a mailbox — {why} — and has to say how deep it is: add `mailbox \
         {{ capacity: 16 }}` (or take it as a constructor parameter — `mailbox \
         {{ capacity: capacity }}`). There is no default, because a queue bound chosen \
         by the compiler is a performance cliff nobody wrote"
    );
    /// A mailbox slot on a handler with nothing to enqueue.
    actor_mailbox_nothing_to_enqueue = "actor-mailbox:nothing-to-enqueue" (handler: &dyn std::fmt::Display) => format!(
        "only a handler with something to enqueue has a mailbox, and `{handler}` has no \
         actor face and no `send fn` member: its members run on the caller's thread, so \
         there is no queue to bound"
    );
    /// A mailbox slot of the wrong type.
    actor_mailbox_slot_type = "actor-mailbox:slot-type" (expected: &dyn std::fmt::Display, got: &dyn std::fmt::Display) => format!(
        "a `mailbox` slot builds a `{expected}`, found `{got}`"
    );

    // [actor-mixed-handler]
    /// An `init` block in a mixed handler.
    actor_mixed_handler_init_block = "actor-mixed-handler:init-block" (handler: &dyn std::fmt::Display) => format!(
        "handler `{handler}` is mixed, and a mixed handler has no `init` block yet: start \
         it with a send member instead"
    );
    /// A handler of both actor and plain effects.
    actor_mixed_handler_two_kinds = "actor-mixed-handler:two-kinds" (handler: &dyn std::fmt::Display, async_kind: &dyn std::fmt::Display, sync_kind: &dyn std::fmt::Display) => format!(
        "handler `{handler}` implements effects of two kinds — {async_kind} is an `actor \
         effect` and {sync_kind} is not: a handler is bound one way or the other \
         (`spawn` or `use`), so its faces must agree"
    );
    /// A mixed handler bound with `use`.
    actor_mixed_handler_use_bound = "actor-mixed-handler:use-bound" (handler: &dyn std::fmt::Display) => format!(
        "handler `{handler}` is mixed — its state is confined to `send fn` members, and \
         its sync members send to that servant — so it can only be `spawn`ed: bound with \
         `use` there would be no mailbox for the sends to arrive on. Write `let handle = \
         spawn {handler}(...)` and `use handle`"
    );
    /// A servant `send fn` without a written deduction clause.
    actor_mixed_handler_send_clause = "actor-mixed-handler:send-clause" (fn_name: &dyn std::fmt::Display, entries: &dyn std::fmt::Display) => format!(
        "`send fn {fn_name}` implements no effect member, so its deduction clause must be \
         written out: a scheduled body outlives the frame that enqueued it, so every \
         parameter is consumed — `=> {entries}`"
    );
    /// A servant `send fn` that keeps a parameter.
    actor_mixed_handler_kept_param = "actor-mixed-handler:kept-param" (fn_name: &dyn std::fmt::Display, param: &dyn std::fmt::Display) => format!(
        "`send fn {fn_name}` cannot keep `{param}`: a scheduled body outlives the frame \
         that enqueued it, so every parameter is consumed (`=> !{param}`)"
    );
    /// A generic servant `send fn`.
    actor_mixed_handler_generic_send = "actor-mixed-handler:generic-send" (fn_name: &dyn std::fmt::Display, generic: &dyn std::fmt::Display) => format!(
        "`send fn {fn_name}` cannot be generic: a message carries payloads and no type \
         arguments, so `{generic}` would have nothing to be chosen by"
    );
    /// A servant `send fn` with a `Mut` parameter.
    actor_mixed_handler_mut_param = "actor-mixed-handler:mut-param" (fn_name: &dyn std::fmt::Display, param: &dyn std::fmt::Display) => format!(
        "`send fn {fn_name}` cannot take `Mut` parameter `{param}`: mutating a value \
         across a seam would share what the servant owns alone"
    );
    /// A servant `send fn` carrying a type that cannot cross.
    actor_mixed_handler_unsendable_param = "actor-mixed-handler:unsendable-param" (fn_name: &dyn std::fmt::Display, ty: &dyn std::fmt::Display, why: &dyn std::fmt::Display) => format!(
        "`send fn {fn_name}` cannot carry `{ty}`: {why}, and everything crossing to the \
         servant must be sendable"
    );
    /// An overloaded servant `send fn`.
    actor_mixed_handler_overloaded_send = "actor-mixed-handler:overloaded-send" (fn_name: &dyn std::fmt::Display) => format!(
        "`send fn {fn_name}` is overloaded, which a servant protocol does not support \
         yet: the message dispatch is by member name — rename one"
    );
    /// A call that is both a servant send and an effect member.
    actor_mixed_handler_send_or_member = "actor-mixed-handler:send-or-member" (name: &dyn std::fmt::Display, effects: &dyn std::fmt::Display) => format!(
        "`{name}` is both a `send fn` of this handler and a member of {effects} available \
         here: say which — `{name}@self(…)` for a send to this handler's servant, or \
         name the effect (`{name}@E(…)`)"
    );
    /// A servant send with the wrong number of arguments.
    actor_mixed_handler_arg_count = "actor-mixed-handler:arg-count" (name: &dyn std::fmt::Display, expected: &dyn std::fmt::Display, found: &dyn std::fmt::Display) => format!(
        "`{name}` takes {expected} argument(s), found {found}"
    );
    /// A servant send argument of the wrong type.
    actor_mixed_handler_arg_mismatch = "actor-mixed-handler:arg-mismatch" (want: &dyn std::fmt::Display, got: &dyn std::fmt::Display) => format!(
        "expected `{want}`, found `{got}`"
    );
    /// A received reply whose answer outlives the activation undeclared.
    actor_mixed_handler_undeclared_defer = "actor-mixed-handler:undeclared-defer" (name: &dyn std::fmt::Display, how: &dyn std::fmt::Display) => format!(
        "`{name}` is a reply this member received, and {how} lets its answer outlive the \
         activation: declare it — `=> defer {name}` — which is the handler's opt-in to \
         deferred answers, and what the deadlock graph prices"
    );
    /// A mixed handler of several plain effects.
    actor_mixed_handler_several_faces = "actor-mixed-handler:several-faces" (handler: &dyn std::fmt::Display, faces: &dyn std::fmt::Display) => format!(
        "handler `{handler}` implements several plain effects ({faces}), and a façade \
         behind several faces is not supported yet: split the handler, or give it a \
         single face"
    );
    /// A mixed handler with a `Mut` constructor parameter.
    actor_mixed_handler_mut_ctor_param = "actor-mixed-handler:mut-ctor-param" (handler: &dyn std::fmt::Display, param: &dyn std::fmt::Display) => format!(
        "`{handler}` cannot be shared: its constructor parameter `{param}` is `Mut`, and \
         the façade value carries a copy of every parameter — a mutable one would go two \
         ways at once. Move the mutable state into a field (the servant owns it), or \
         pass immutable data"
    );
    /// A mixed handler whose constructor parameter cannot cross.
    actor_mixed_handler_unsendable_ctor_param = "actor-mixed-handler:unsendable-ctor-param" (handler: &dyn std::fmt::Display, param: &dyn std::fmt::Display, ty: &dyn std::fmt::Display, why: &dyn std::fmt::Display) => format!(
        "`{handler}` cannot be shared: its constructor parameter `{param}` is `{ty}` — \
         {why}, and the façade value carries a copy of it to every thread that binds the \
         handle"
    );
    /// A servant's state read from a sync member.
    actor_mixed_handler_servant_state_sync = "actor-mixed-handler:servant-state-sync" (name: &dyn std::fmt::Display) => format!(
        "`{name}` is the servant's state, which a mixed handler confines to its `send fn` \
         members: a sync member runs on the caller's thread, where reading it would race \
         an activation — send to a member that answers instead"
    );
    /// A mixed handler's sync member calling a dependency.
    actor_mixed_handler_sync_member_dependency = "actor-mixed-handler:sync-member-dependency" (member: &dyn std::fmt::Display, effect: &dyn std::fmt::Display) => format!(
        "`{member}` is a member of `{effect}`, a dependency of this handler, and a mixed \
         handler's dependencies are its servant's: a sync member runs on the caller's \
         thread and cannot reach them. Call it from a `send fn` member and send to that"
    );

    // [actor-monitor-handler]
    /// A `with` clause on a spawn that shares a monitor.
    actor_monitor_handler_with_clause = "actor-monitor-handler:with-clause" (handler: &dyn std::fmt::Display) => format!(
        "`{handler}` is shared as a monitor, and a monitor's dependencies come from the \
         enclosing scope at the spawn — not from a `with` clause: bind the dependency \
         with `use` before this spawn and drop the clause"
    );
    /// A monitor's dependency with no handler in scope.
    actor_monitor_handler_missing_dep = "actor-monitor-handler:missing-dep" (handler: &dyn std::fmt::Display, want: &dyn std::fmt::Display) => format!(
        "handler `{handler}` depends on effect `{want}`, which has no handler in scope \
         here: register one before this spawn"
    );
    /// A monitor of several plain effects.
    actor_monitor_handler_several_faces = "actor-monitor-handler:several-faces" (handler: &dyn std::fmt::Display, faces: &dyn std::fmt::Display) => format!(
        "handler `{handler}` implements several plain effects ({faces}), and a shared \
         instance behind several faces is not supported yet: split the handler, or give \
         it a single face"
    );
    /// A monitor spawn with an `on` clause.
    actor_monitor_handler_on_pool = "actor-monitor-handler:on-pool" (handler: &dyn std::fmt::Display) => format!(
        "`{handler}` implements a plain effect, so this spawn shares it as a monitor — \
         its members run on the callers' threads, and there is nothing to place `on` a \
         pool. Remove the `on` clause"
    );
    /// A monitor spawn of a handler with a send member (a compiler bug).
    actor_monitor_handler_send_member = "actor-monitor-handler:send-member" (handler: &dyn std::fmt::Display, member: &dyn std::fmt::Display) => format!(
        "internal: monitor spawn of `{handler}` with send member `{member}` — the mixed \
         path should have taken it"
    );
    /// A monitor whose constructor parameter cannot cross.
    actor_monitor_handler_unsendable_param = "actor-monitor-handler:unsendable-param" (handler: &dyn std::fmt::Display, param: &dyn std::fmt::Display, ty: &dyn std::fmt::Display, why: &dyn std::fmt::Display) => format!(
        "`{handler}` cannot be shared: its constructor parameter `{param}` is `{ty}` — \
         {why}, and a shared handler's state must be sendable"
    );
    /// A monitor whose state field cannot cross.
    actor_monitor_handler_unsendable_field = "actor-monitor-handler:unsendable-field" (handler: &dyn std::fmt::Display, field: &dyn std::fmt::Display, ty: &dyn std::fmt::Display, why: &dyn std::fmt::Display) => format!(
        "`{handler}` cannot be shared: its state field `{field}` is `{ty}` — {why}, and a \
         shared handler's state must be sendable"
    );

    // [actor-replyto]
    /// A handler that mints continuations, bound with `use`.
    actor_replyto_use_bound = "actor-replyto:use-bound" (handler: &dyn std::fmt::Display) => format!(
        "handler `{handler}` mints a continuation with `replyto`, or names its own \
         address with `self@…`, so it can only be `spawn`ed: bound with `use` its \
         members run inline, and there is no mailbox for the answer to arrive on and no \
         address to name — write `spawn {handler}(...) on pool(1)` instead"
    );
    /// A continuation `on` something that is not a pool.
    actor_replyto_pool_type = "actor-replyto:pool-type" (pool_type: &dyn std::fmt::Display, pool_ty: &dyn std::fmt::Display) => format!(
        "a continuation runs on a `{pool_type}`, found `{pool_ty}`: `on pool(2)` builds \
         one, `on thread()` a dedicated one"
    );
    /// A `replyto` with the wrong number of captures.
    actor_replyto_capture_count = "actor-replyto:capture-count" (member: &dyn std::fmt::Display, params: &dyn std::fmt::Display, wanted: &dyn std::fmt::Display, found: &dyn std::fmt::Display) => format!(
        "`{member}` takes {params} parameter(s): the last is the answer the token carries \
         and the {wanted} before it are the captures, so `replyto` here wants {wanted} \
         capture(s), found {found}"
    );
    /// A `replyto` capture of the wrong type.
    actor_replyto_capture_mismatch = "actor-replyto:capture-mismatch" (want: &dyn std::fmt::Display, got: &dyn std::fmt::Display) => format!(
        "expected `{want}`, found `{got}`"
    );
    /// An `on` clause on a mint whose target is a member of this handler.
    actor_replyto_own_member_on = "actor-replyto:own-member-on" (form: &dyn std::fmt::Display) => format!(
        "`{form}` targeting a member of this handler takes no `on` clause: the answer \
         arrives on the actor's own mailbox, which is where the continuation belongs. A \
         placement is written where the *actor* is spawned, or on a mint whose target is \
         a free `send fn`"
    );
    /// A mint into the own handler outside a handler member.
    actor_replyto_outside_member = "actor-replyto:outside-member" (form: &dyn std::fmt::Display, detail: &dyn std::fmt::Display) => format!(
        "`{form}` mints a continuation into the handler it is written in, so it is legal \
         only inside a handler member{detail}"
    );
    /// A mint toward another actor's member.
    actor_replyto_other_actor_member = "actor-replyto:other-actor-member" (form: &dyn std::fmt::Display, member: &dyn std::fmt::Display, handlers: &dyn std::fmt::Display) => format!(
        "`{form}` targets a member of the handler it is written in, and `{member}` is a \
         member of {handlers} instead: minting toward another actor's member is not \
         supported yet. Have the handler that owns `{member}` mint the token and pass it \
         here — a `Reply<T>` is an ordinary linear value"
    );
    /// A mint naming no member of this handler.
    actor_replyto_unknown_member = "actor-replyto:unknown-member" (handler: &dyn std::fmt::Display, member: &dyn std::fmt::Display, form: &dyn std::fmt::Display) => format!(
        "handler `{handler}` has no member `{member}` for `{form}` to deliver to"
    );
    /// A mint targeting a member that is not a `send fn`.
    actor_replyto_not_send = "actor-replyto:not-send" (member: &dyn std::fmt::Display, form: &dyn std::fmt::Display) => format!(
        "`{member}` is not a `send fn`, so `{form}` cannot deliver to it: an answer \
         arrives as a message"
    );
    /// A mint into the own handler with the wrong number of captures.
    actor_replyto_member_capture_count = "actor-replyto:member-capture-count" (member: &dyn std::fmt::Display, params: &dyn std::fmt::Display, wanted: &dyn std::fmt::Display, form: &dyn std::fmt::Display, found: &dyn std::fmt::Display) => format!(
        "`{member}` takes {params} parameter(s): the last is the answer the token carries \
         and the {wanted} before it are the captures, so `{form}` here wants {wanted} \
         capture(s), found {found}"
    );

    // [actor-route-stub]
    /// A member marking several parameters `Key`.
    actor_route_stub_several_keys = "actor-route-stub:several-keys" (effect: &dyn std::fmt::Display, member: &dyn std::fmt::Display, count: &dyn std::fmt::Display) => format!(
        "`{effect}.{member}` marks {count} parameters `Key`; a member routes on at most \
         one"
    );
    /// `use any` of something that is not a group handle.
    actor_route_stub_not_a_group = "actor-route-stub:not-a-group" (ty: &dyn std::fmt::Display) => format!(
        "`use any` binds a group handle — an `eff[ActorGroup<E>]` for a non-generic actor \
         effect `E` — and this is `{ty}`"
    );
    /// `use any` with no route selector in scope.
    actor_route_stub_no_selector = "actor-route-stub:no-selector" (shown: &dyn std::fmt::Display) => format!(
        "`use any` routes each send through a `RouteSelector<{shown}>` policy, and none \
         is in scope: bind one first — `use Sharded<{shown}>()`, `use \
         LeastLoaded<{shown}>(true)`, `use Elected<{shown}>()`, or a handler of your own"
    );

    // [actor-self-send]
    /// `k@self(…)` outside a handler member.
    actor_self_send_outside_member = "actor-self-send:outside-member" (member: &dyn std::fmt::Display, detail: &dyn std::fmt::Display) => format!(
        "`{member}@self(…)` names a member of the enclosing handler, so it is legal only \
         inside a handler member{detail}"
    );
    /// A self-send naming no member.
    actor_self_send_no_member = "actor-self-send:no-member" (handler: &dyn std::fmt::Display, member: &dyn std::fmt::Display) => format!(
        "handler `{handler}` has no member `{member}` to send to"
    );
    /// A self-send to a member that is not a `send fn`.
    actor_self_send_not_send = "actor-self-send:not-send" (member: &dyn std::fmt::Display) => format!(
        "`{member}` is not a `send fn`, so `{member}@self(…)` cannot reach it: a \
         self-send is a message, and a member that answers would have to wait for itself"
    );
    /// A self-send with the wrong number of arguments.
    actor_self_send_arg_count = "actor-self-send:arg-count" (member: &dyn std::fmt::Display, expected: &dyn std::fmt::Display, found: &dyn std::fmt::Display) => format!(
        "`{member}` takes {expected} argument(s), found {found}"
    );
    /// A self-send argument of the wrong type.
    actor_self_send_arg_mismatch = "actor-self-send:arg-mismatch" (want: &dyn std::fmt::Display, got: &dyn std::fmt::Display) => format!(
        "expected `{want}`, found `{got}`"
    );
    /// `k@self` as a value.
    actor_self_send_member_value = "actor-self-send:member-value" (member: &dyn std::fmt::Display) => format!(
        "`{member}@self` names a member of the enclosing handler, which is not a value: \
         call it (`{member}@self(…)`) to send it a message"
    );
    /// `self@…` outside a handler member.
    actor_self_send_address_outside_member = "actor-self-send:address-outside-member" () =>
        "`self@…` names the enclosing handler's own address, so it is legal only inside a \
         handler member or its `init` block";
    /// `self@Face` naming no actor face of the handler.
    actor_self_send_no_face = "actor-self-send:no-face" (ty: &dyn std::fmt::Display, handler: &dyn std::fmt::Display, faces: &dyn std::fmt::Display) => format!(
        "`self@{ty}`: handler `{handler}` has no actor face `{ty}` — it implements \
         `{faces}` — and an address is a face of an actor effect"
    );

    // [actor-send-fn]
    /// A `send fn` in a plain effect.
    actor_send_fn_plain_effect = "actor-send-fn:plain-effect" (member: &dyn std::fmt::Display, effect: &dyn std::fmt::Display) => format!(
        "`send fn {member}` needs an `actor effect`: a message is enqueued on an actor, \
         which a synchronous effect never has — declare `actor effect {effect}` if this \
         protocol is meant to be spawned"
    );
    /// A `send fn` with a return type.
    actor_send_fn_return_type = "actor-send-fn:return-type" (fn_name: &dyn std::fmt::Display) => format!(
        "`send fn {fn_name}` cannot declare a return type: sending a member enqueues it \
         and answers nothing, so a reply travels as a parameter — `send fn {fn_name}(…, \
         out: Reply<T>)`, which the sender mints with `replyto`"
    );

    // [actor-sendable]
    /// An actor effect member that keeps a parameter.
    actor_sendable_kept_payload = "actor-sendable:kept-payload" (member: &dyn std::fmt::Display, effect: &dyn std::fmt::Display, param: &dyn std::fmt::Display) => format!(
        "member `{member}` of `actor effect {effect}` cannot keep `{param}`: a message \
         payload crosses to another actor, so it is always consumed — write `=> \
         !{param}`, or make `{effect}` a plain effect"
    );
    /// An actor effect member with a `Mut` parameter.
    actor_sendable_mut_payload = "actor-sendable:mut-payload" (member: &dyn std::fmt::Display, effect: &dyn std::fmt::Display) => format!(
        "member `{member}` of `actor effect {effect}` cannot take a `Mut` parameter: \
         mutating a value across a actor boundary would share it, and an actor owns its \
         state alone"
    );
    /// An actor effect member carrying a type that cannot cross.
    actor_sendable_unsendable_payload = "actor-sendable:unsendable-payload" (member: &dyn std::fmt::Display, effect: &dyn std::fmt::Display, ty: &dyn std::fmt::Display, why: &dyn std::fmt::Display) => format!(
        "member `{member}` of `actor effect {effect}` cannot carry `{ty}`: {why}, and \
         everything crossing to another actor must be sendable"
    );
    /// An actor effect member returning a `ref` or `proj`.
    actor_sendable_borrowed_return = "actor-sendable:borrowed-return" (member: &dyn std::fmt::Display, effect: &dyn std::fmt::Display, what: &dyn std::fmt::Display) => format!(
        "member `{member}` of `actor effect {effect}` cannot return {what}: it would \
         borrow state the actor owns and keeps mutating"
    );
    /// A task capture of a type that cannot cross.
    actor_sendable_capture = "actor-sendable:capture" (member: &dyn std::fmt::Display, got: &dyn std::fmt::Display, why: &dyn std::fmt::Display) => format!(
        "a capture of `{member}` cannot be `{got}`: {why}, and a task's captures cross a \
         thread boundary"
    );

    // [actor-spawn-effect]
    /// `spawn` in a fn type.
    actor_spawn_effect_fn_type = "actor-spawn-effect:fn-type" () =>
        "a fn type cannot declare `spawn`: creating an actor is local to a body, so a \
         lambda may spawn exactly when the function containing it may";
    /// `spawn` inside a lambda.
    actor_spawn_effect_in_lambda = "actor-spawn-effect:in-lambda" () =>
        "`spawn` cannot be written inside a lambda: a function value's body runs wherever \
         it is called, and a fn type cannot declare `spawn` — spawn in the function that \
         holds the capability and pass the `eff[…]` handle";
    /// `spawn` without the `spawn` capability.
    actor_spawn_effect_missing = "actor-spawn-effect:missing" () =>
        "`spawn` requires the `spawn` capability in the function's effect list (`[use, \
         spawn]`)";
    /// A call of a fn needing `spawn` from one without it.
    actor_spawn_effect_callee_needs_spawn = "actor-spawn-effect:callee-needs-spawn" (name: &dyn std::fmt::Display) => format!(
        "`{name}` requires the `spawn` capability, so this function must declare it too \
         (`[spawn]` in its effect list)"
    );

    // [actor-spawn-expr]
    /// A spawn `on` something that is not a pool.
    actor_spawn_expr_pool_type = "actor-spawn-expr:pool-type" (pool_type: &dyn std::fmt::Display, pool_ty: &dyn std::fmt::Display) => format!(
        "a spawn runs on a `{pool_type}`, found `{pool_ty}`: `on pool(2)` builds one"
    );
    /// A spawn without `on`.
    actor_spawn_expr_no_pool = "actor-spawn-expr:no-pool" (handler: &dyn std::fmt::Display) => format!(
        "`spawn` must name the pool it runs on: write `on pool(n)` (or `on POOL` for a \
         pool already built). To register `{handler}` in this scope instead, write `use \
         {handler}(...)`"
    );
    /// A handler of several effects constructed in a spawn's `with` clause.
    actor_spawn_expr_with_several_faces = "actor-spawn-expr:with-several-faces" (handler: &dyn std::fmt::Display, faces: &dyn std::fmt::Display) => format!(
        "handler `{handler}` implements several effects ({faces}), so it cannot be \
         constructed in a spawn's `with` clause: the child would own it, and one \
         instance cannot be two of its dependencies — spawn it separately and pass an \
         `eff[…]` per face"
    );
    /// A handler with dependencies constructed in a spawn's `with` clause.
    actor_spawn_expr_with_dependencies = "actor-spawn-expr:with-dependencies" (handler: &dyn std::fmt::Display, dep: &dyn std::fmt::Display, effect: &dyn std::fmt::Display) => format!(
        "handler `{handler}` depends on effect `{dep}`, so it cannot be constructed in a \
         spawn's `with` clause: give the child a `eff[…]` of an actor serving `{effect}` \
         instead, or construct it inside the child with `use`"
    );
    /// A spawn's `with` item that is neither a handler nor a handle.
    actor_spawn_expr_with_not_a_handler = "actor-spawn-expr:with-not-a-handler" (ty: &dyn std::fmt::Display) => format!(
        "a spawn's `with` clause supplies handlers: write a handler construction \
         (`SomeHandler(...)`) or a `eff[…]` of the effect an actor already serves, not a \
         `{ty}`"
    );

    // [actor-spawn-inherit]
    /// A dependency several instances in scope could supply.
    actor_spawn_inherit_ambiguous_dep = "actor-spawn-inherit:ambiguous-dep" (handler: &dyn std::fmt::Display, want: &dyn std::fmt::Display, instances: &dyn std::fmt::Display) => format!(
        "handler `{handler}` depends on effect `{want}`, and this scope has several \
         instances it could mean ({instances}): name the one you want with `with`"
    );
    /// A dependency neither the spawn nor the scope supplies.
    actor_spawn_inherit_missing_dep = "actor-spawn-inherit:missing-dep" (handler: &dyn std::fmt::Display, want: &dyn std::fmt::Display) => format!(
        "handler `{handler}` depends on effect `{want}`, which this spawn does not supply \
         and this scope has no handler for: bind one before the spawn (it is then \
         inherited), or supply it in the clause (`spawn {handler}(...) with \
         SomeHandler()`)"
    );
    /// A spawn's `with` item the handler does not depend on.
    actor_spawn_inherit_not_a_dep = "actor-spawn-inherit:not-a-dep" (handler: &dyn std::fmt::Display, eff: &dyn std::fmt::Display) => format!(
        "handler `{handler}` does not depend on effect `{eff}`, so supplying it here has \
         no effect"
    );

    // [actor-use-addr]
    /// `use … on POOL` of a handler with several faces.
    actor_use_addr_several_faces = "actor-use-addr:several-faces" () =>
        "`use … on POOL` binds one face, and this handler has several: spawn it (`let (a, \
         b) = spawn H(...) on POOL`) and `use` each face yourself";
    /// `use` of a tuple that is not all handles.
    actor_use_addr_tuple = "actor-use-addr:tuple" (ty: &dyn std::fmt::Display) => format!(
        "`use` binds a handler or an effect handle, and `{ty}` is a tuple that is not all \
         handles"
    );
    /// `use` of something that is neither a handler nor a handle.
    actor_use_addr_not_a_handle = "actor-use-addr:not-a-handle" (ty: &dyn std::fmt::Display) => format!(
        "`use` registers a handler or binds an `eff[…]`, and `{ty}` is neither: write a \
         handler construction (`use SomeHandler(...)`), or bind the addr a `spawn` \
         answered"
    );
    /// A send through an effect not in scope.
    actor_use_addr_not_in_scope = "actor-use-addr:not-in-scope" (effect_name: &dyn std::fmt::Display) => format!(
        "effect `{effect_name}` is not in scope here"
    );
    /// A send through a handle that argument counts cannot choose.
    actor_use_addr_overload_by_count = "actor-use-addr:overload-by-count" (effect_name: &dyn std::fmt::Display, member: &dyn std::fmt::Display, count: &dyn std::fmt::Display, found: &dyn std::fmt::Display) => format!(
        "effect `{effect_name}` overloads `{member}`, and {count} of its overloads take \
         {found} argument(s): a send through a `eff[…]` picks by argument count, so this \
         call cannot say which one it means"
    );
    /// A send through a handle naming no member.
    actor_use_addr_no_member = "actor-use-addr:no-member" (instance: &dyn std::fmt::Display, effect_name: &dyn std::fmt::Display, member: &dyn std::fmt::Display) => format!(
        "actor `{instance}` serves effect `{effect_name}`, which has no member named \
         `{member}`"
    );
    /// A call through a handle to a member that is not a `send fn`.
    actor_use_addr_not_send = "actor-use-addr:not-send" (member: &dyn std::fmt::Display) => format!(
        "`{member}` is not a `send fn`, so it cannot be called through a `eff[…]`: an \
         actor serves messages, and a member that answers would have to park its caller"
    );
    /// A send through a handle with the wrong number of arguments.
    actor_use_addr_arg_count = "actor-use-addr:arg-count" (member: &dyn std::fmt::Display, expected: &dyn std::fmt::Display, found: &dyn std::fmt::Display) => format!(
        "`{member}` takes {expected} argument(s), found {found}"
    );
    /// A send-through-a-handle argument of the wrong type.
    actor_use_addr_arg_mismatch = "actor-use-addr:arg-mismatch" (want: &dyn std::fmt::Display, got: &dyn std::fmt::Display) => format!(
        "expected `{want}`, found `{got}`"
    );

    // [actor-waitfor]
    /// A `waitfor` inside a lambda.
    actor_waitfor_in_lambda = "actor-waitfor:in-lambda" () =>
        "`waitfor` cannot be written inside a lambda: a function value runs wherever it \
         is called, so the wait would occupy call sites nothing can enumerate — wait in \
         the function that calls the value, and pass what it needs";
    /// A `waitfor` binding annotated with something other than a `Reply<T>`.
    actor_waitfor_binding_type = "actor-waitfor:binding-type" (reply_type: &dyn std::fmt::Display, other: &dyn std::fmt::Display) => format!(
        "`waitfor` mints a reply token, so its binding is a `{reply_type}<T>`, not a \
         `{other}`"
    );

    // [assert-fn]
    /// An assertion message that is not a `Str`.
    assert_fn_message_not_str = "assert-fn:message-not-str" (t: &dyn std::fmt::Display) => format!(
        "an assertion's message must be a `Str`, found `{t}`"
    );
    /// A `todo!()` (a warning).
    assert_fn_todo = "assert-fn:todo" () =>
        "`todo!()` marks code that is not implemented yet";

    // [call-resolve]
    /// A call or fn value naming no function in scope.
    call_resolve_unknown_fn = "call-resolve:unknown-fn" (name: &dyn std::fmt::Display) => format!(
        "no function named `{name}` is in scope"
    );
    /// A call of a local whose type is a type parameter.
    call_resolve_type_param_call = "call-resolve:type-param-call" (name: &dyn std::fmt::Display, vty: &dyn std::fmt::Display) => format!(
        "`{name}` is a type parameter (`{vty}`), so nothing is known about its values — \
         including whether they are functions. Declare it with a fn type to call it"
    );
    /// A call of a local that is not a function.
    call_resolve_not_callable_local = "call-resolve:not-callable-local" (name: &dyn std::fmt::Display, vty: &dyn std::fmt::Display) => format!(
        "`{name}` is not callable: its type is `{vty}`"
    );
    /// A call of an expression that is not a function.
    call_resolve_not_callable = "call-resolve:not-callable" (cty: &dyn std::fmt::Display) => format!(
        "this expression is not callable: its type is `{cty}`"
    );
    /// A fn value called with an argument of the wrong type.
    call_resolve_fn_value_arg_mismatch = "call-resolve:fn-value-arg-mismatch" (callee: &dyn std::fmt::Display, p: &dyn std::fmt::Display, aty: &dyn std::fmt::Display) => format!(
        "{callee} takes `{p}` here, found `{aty}`"
    );
    /// A fn value called with an argument that lacks what the parameter claims.
    call_resolve_fn_value_claim_mismatch = "call-resolve:fn-value-claim-mismatch" (callee: &dyn std::fmt::Display, p: &dyn std::fmt::Display, aty: &dyn std::fmt::Display) => format!(
        "{callee} takes `{p}` here, found `{aty}`: the argument does not carry what the \
         parameter claims"
    );
    /// A fn value called with the wrong number of arguments.
    call_resolve_fn_value_arg_count = "call-resolve:fn-value-arg-count" (callee: &dyn std::fmt::Display, expected: &dyn std::fmt::Display, found: &dyn std::fmt::Display) => format!(
        "{callee} takes {expected} argument(s), found {found}"
    );
    /// A call no overload fits, with the reason.
    call_resolve_no_overload_why = "call-resolve:no-overload-why" (name: &dyn std::fmt::Display, args: &dyn std::fmt::Display, hint: &dyn std::fmt::Display) => format!(
        "no matching overload for `{name}({args})`: {hint}"
    );
    /// A call no overload fits.
    call_resolve_no_overload = "call-resolve:no-overload" (name: &dyn std::fmt::Display, args: &dyn std::fmt::Display, hint: &dyn std::fmt::Display) => format!(
        "no matching overload for `{name}({args})`{hint}"
    );

    // [call-type-args]
    /// A call whose type arguments nothing determines.
    call_type_args_cannot_infer = "call-type-args:cannot-infer" (plural: &dyn std::fmt::Display, list: &dyn std::fmt::Display, name: &dyn std::fmt::Display, pronoun: &dyn std::fmt::Display) => format!(
        "cannot infer type argument{plural} {list} of `{name}`: no argument determines \
         {pronoun} and neither does the context — write the type argument{plural} \
         (`{name}<...>(...)`) or annotate where the result goes"
    );

    // [cmp-binder]
    /// A `?name` argument naming no function slot.
    cmp_binder_unknown_slot = "cmp-binder:unknown-slot" (slot: &dyn std::fmt::Display, known: &dyn std::fmt::Display) => format!(
        "`?{slot}` names no function slot here{known}"
    );
    /// More function-slot arguments than slots.
    cmp_binder_too_many = "cmp-binder:too-many" (count: &dyn std::fmt::Display, names: &dyn std::fmt::Display) => format!(
        "too many arguments: this declaration has {count} function slot(s){names}"
    );
    /// A function slot filled twice.
    cmp_binder_filled_twice = "cmp-binder:filled-twice" (slot: &dyn std::fmt::Display) => format!(
        "the function slot `?{slot}` is filled twice"
    );

    // [cmp-carry]
    /// A carried identity that does not fit the callee's implicit.
    cmp_carry_carried_mismatch = "cmp-carry:carried-mismatch" (callee: &dyn std::fmt::Display, implicit: &dyn std::fmt::Display, id: &dyn std::fmt::Display, want: &dyn std::fmt::Display) => format!(
        "`{callee}` needs the `{implicit}` this value was \
         built                                              with — `{id}` — and it does \
         not fit `?{implicit}: {want}`                                              here"
    );
    /// A value with no identity where a carried one is claimed.
    cmp_carry_no_identity = "cmp-carry:no-identity" () =>
        "this value has no identity a type can carry: the position's claim names the \
         function it was built with, so pass a named top-level `fn` (a lambda or a local \
         cannot be named in a type)";
    /// A carried identity that does not fit its slot.
    cmp_carry_slot_mismatch = "cmp-carry:slot-mismatch" (id: &dyn std::fmt::Display, slot: &dyn std::fmt::Display, name: &dyn std::fmt::Display, want: &dyn std::fmt::Display) => format!(
        "`{id}` does not fit the `{slot}` slot of `{name}`, which needs `{want}`"
    );
    /// A `?Group` slot spread naming no group.
    cmp_carry_unknown_group = "cmp-carry:unknown-group" (name: &dyn std::fmt::Display) => format!(
        "no `fns` group named `{name}` is in scope: `?{name}` spreads a group's members \
         as function slots"
    );
    /// A type where a function identity goes.
    cmp_carry_type_not_identity = "cmp-carry:type-not-identity" () =>
        "a function slot takes a function identity — a name like `cmp`, `cmp@Person` or \
         the signature's `?cmp` — not a type";
    /// A function identity written with qualifiers or type arguments.
    cmp_carry_identity_not_plain = "cmp-carry:identity-not-plain" () =>
        "a function identity is a plain name (`cmp`, `cmp@Person`): it takes no \
         qualifiers and no type arguments of its own";
    /// A local or lambda where a function identity goes.
    cmp_carry_local_identity = "cmp-carry:local-identity" (name: &dyn std::fmt::Display) => format!(
        "`{name}` is a local here, and a lambda or local has no identity a type can \
         carry: declare it as a `fn` (a type carries only what it can print — a named, \
         top-level, capture-free function)"
    );
    /// A type parameter where a function identity goes.
    cmp_carry_type_param_identity = "cmp-carry:type-param-identity" (name: &dyn std::fmt::Display) => format!(
        "`{name}` is a type parameter, and this position carries a function identity: \
         name a function, or the signature's binder (`?{name}`)"
    );
    /// A function identity naming no function in scope.
    cmp_carry_unknown_fn = "cmp-carry:unknown-fn" (name: &dyn std::fmt::Display) => format!(
        "no function `{name}` in scope to carry here"
    );
    /// A function identity several functions fit.
    cmp_carry_ambiguous = "cmp-carry:ambiguous" (shown: &dyn std::fmt::Display, want: &dyn std::fmt::Display, text: &dyn std::fmt::Display) => format!(
        "the identity `{shown}` is ambiguous at `{want}`: {text}"
    );

    // [col-index-resolve]
    /// `[]` on something that is not an array.
    col_index_resolve_not_indexable = "col-index-resolve:not-indexable" (other: &dyn std::fmt::Display) => format!(
        "`{other}` cannot be indexed with `[]`: only arrays can, and other collections \
         expose element access as functions (e.g. `get(collection, index)`)"
    );

    // [col-key-eligible]
    /// A set or map keyed by something not hashable.
    col_key_eligible_key = "col-key-eligible:key" (ty: &dyn std::fmt::Display, reason: &dyn std::fmt::Display) => format!(
        "`{ty}` cannot be a key: {reason}. A `Set` element and a `Map` key have to be \
         hashable"
    );

    // [col-sorted]
    /// A sorted collection keyed by something not orderable.
    col_sorted_key = "col-sorted:key" (ty: &dyn std::fmt::Display, reason: &dyn std::fmt::Display) => format!(
        "`{ty}` cannot be a sorted collection's key: {reason}. A `SortedSet` element and \
         a `SortedMap` key have to be orderable"
    );

    // [col-sorted-list]
    /// A `Sorted List` of elements that are not orderable.
    col_sorted_list_element = "col-sorted-list:element" (elem: &dyn std::fmt::Display, reason: &dyn std::fmt::Display) => format!(
        "`{elem}` cannot be the element of a `Sorted List`: {reason}. Sorting compares \
         the elements, so they have to be orderable"
    );

    // [comptime-fields]
    /// A compile-time type in an ordinary type position.
    comptime_fields_data_position = "comptime-fields:data-position" (name: &dyn std::fmt::Display) => format!(
        "`{name}` is a compile-time type (`core.comptime`): it describes a type to a \
         `comptime fn` and is never a value, so it cannot appear in an ordinary type \
         position"
    );
    /// A literal of a compile-time type.
    comptime_fields_literal = "comptime-fields:literal" (name: &dyn std::fmt::Display) => format!(
        "`{name}` is a compile-time type (`core.comptime`) and has no values: the \
         compiler fills it for a `comptime fn`, and nothing constructs one"
    );

    // [comptime-inline]
    /// A comptime statement that reached the checker (a compiler bug).
    comptime_inline_not_expanded = "comptime-inline:not-expanded" () =>
        "internal: a comptime statement reached the checker — the comptime expansion did \
         not run";

    // [comptime-qualifier]
    /// A comptime qualifier written in a type.
    comptime_qualifier_in_type = "comptime-qualifier:in-type" (qualifier: &dyn std::fmt::Display) => format!(
        "`{qualifier}` is a comptime qualifier: it is written before a field's name \
         (`{qualifier}(…) port: Int`) or a declaration, and is never part of a type"
    );
    /// A comptime qualifier applied twice.
    comptime_qualifier_applied_twice = "comptime-qualifier:applied-twice" (name: &dyn std::fmt::Display) => format!(
        "qualifier `{name}` is applied more than once"
    );
    /// A qualifier that is not comptime written where a comptime one goes.
    comptime_qualifier_not_comptime_annotation = "comptime-qualifier:not-comptime-annotation" (name: &dyn std::fmt::Display, place: &dyn std::fmt::Display, subject: &dyn std::fmt::Display) => format!(
        "`{name}` is not a comptime qualifier: what is written {place} is a `comptime \
         qualifier … of {subject}`, read by comptime code; a claim about the value goes \
         in the type"
    );
    /// A comptime qualifier written with type arguments.
    comptime_qualifier_type_args = "comptime-qualifier:type-args" (name: &dyn std::fmt::Display) => format!(
        "`{name}` takes no type arguments"
    );
    /// A comptime qualifier with the wrong number of constants.
    comptime_qualifier_const_count = "comptime-qualifier:const-count" (name: &dyn std::fmt::Display, expected: &dyn std::fmt::Display, found: &dyn std::fmt::Display) => format!(
        "`{name}` takes {expected} constant(s), found {found}"
    );
    /// A comptime qualifier's slot given something other than a constant.
    comptime_qualifier_slot_not_constant = "comptime-qualifier:slot-not-constant" () =>
        "a comptime qualifier's slot takes a constant (`'p'`, `\"rm\"`, `3`): nothing \
         else exists where it is written";
    /// A comptime qualifier of something other than `Field` or `Struct`.
    comptime_qualifier_subject = "comptime-qualifier:subject" (subject: &dyn std::fmt::Display) => format!(
        "a comptime qualifier describes a `core.comptime` `Field` or `Struct`, not \
         `{subject}`"
    );
    /// A comptime qualifier with a body.
    comptime_qualifier_body = "comptime-qualifier:body" (qualifier: &dyn std::fmt::Display) => format!(
        "comptime qualifier `{qualifier}` cannot have a body: it is written on a \
         declaration and read by comptime code, so there is nothing to test or refine"
    );
    /// A comptime qualifier with type parameters or fn slots.
    comptime_qualifier_generics = "comptime-qualifier:generics" (qualifier: &dyn std::fmt::Display, subject: &dyn std::fmt::Display) => format!(
        "comptime qualifier `{qualifier}` takes no type parameters or fn slots: \
         `{subject}` already ranges over every declaration it can describe"
    );
    /// A comptime qualifier slot of a type other than `Int`, `Char` or `Str`.
    comptime_qualifier_slot_type = "comptime-qualifier:slot-type" (ty: &dyn std::fmt::Display) => format!(
        "a comptime qualifier's slot holds a constant: `Int`, `Char` or `Str`, not `{ty}`"
    );

    // [copy-scalar-free]
    /// A `!v` on a Copy scalar parameter (a warning).
    copy_scalar_free_moved_scalar = "copy-scalar-free:moved-scalar" (ty: &dyn std::fmt::Display, param: &dyn std::fmt::Display) => format!(
        "moving an `{ty}` is a copy, so `!{param}` changes nothing for the caller: drop it"
    );

    // [decl-body]
    /// A bodiless declaration without a return type.
    decl_body_return_type = "decl-body:return-type" (kind: &dyn std::fmt::Display, name: &dyn std::fmt::Display) => format!(
        "{kind} `{name}` must declare its return type (write `-> None` when it returns no \
         value): with no body to infer from, the signature is the whole contract"
    );
    /// A bodiless declaration without an effect list.
    decl_body_effect_list = "decl-body:effect-list" (kind: &dyn std::fmt::Display, name: &dyn std::fmt::Display) => format!(
        "{kind} `{name}` must declare its effect list (write `[]` when it is pure): with \
         no body to infer from, the signature is the whole contract"
    );

    // [deduce-conditional]
    /// A conditional response naming no parameter.
    deduce_conditional_not_a_param = "deduce-conditional:not-a-param" (subject: &dyn std::fmt::Display, fn_name: &dyn std::fmt::Display) => format!(
        "a conditional response names `{subject}`, which is not a parameter of `{fn_name}`"
    );
    /// A conditional response about a moved parameter.
    deduce_conditional_moved = "deduce-conditional:moved" (subject: &dyn std::fmt::Display, fn_name: &dyn std::fmt::Display) => format!(
        "a conditional response speaks about `{subject}`, but `{fn_name}` moves it — a \
         caller knows nothing about a moved value afterwards. Keep the parameter (drop \
         its `!{subject}`)"
    );
    /// A conditional response naming no qualifier.
    deduce_conditional_unknown_qualifier = "deduce-conditional:unknown-qualifier" (qualifier: &dyn std::fmt::Display) => format!(
        "unknown qualifier `{qualifier}`"
    );
    /// A conditional response's qualifier that does not apply to its subject.
    deduce_conditional_not_applicable = "deduce-conditional:not-applicable" (qualifier: &dyn std::fmt::Display, subject: &dyn std::fmt::Display) => format!(
        "`{qualifier}` does not apply to `{subject}`"
    );
    /// A `return true` claiming `!is` where the claim holds.
    deduce_conditional_claims_not = "deduce-conditional:claims-not" (subject: &dyn std::fmt::Display, qualifier: &dyn std::fmt::Display) => format!(
        "this `return true` claims `{subject} !is {qualifier}`, but `{subject}` carries \
         `{qualifier}` on this path"
    );
    /// A `return true` claiming `is` where nothing proved it.
    deduce_conditional_unproven = "deduce-conditional:unproven" (subject: &dyn std::fmt::Display, qualifier: &dyn std::fmt::Display) => format!(
        "this `return true` claims `{subject} is {qualifier}`, but nothing on this path \
         proved it — narrow `{subject}` with `is` first"
    );

    // [deduce-field]
    /// A mutation outside the fields a clause promises.
    deduce_field_unpromised_mutation = "deduce-field:unpromised-mutation" (promised: &dyn std::fmt::Display, param: &dyn std::fmt::Display, did: &dyn std::fmt::Display) => format!(
        "this function's clause promises it mutates only {promised} of `{param}`, but the \
         body mutates {did}: widen the entry, or write `{param}: Mut` for the whole value"
    );

    // [deduce-reapply]
    /// A trusted `+Q` outside the file that declares `Q`.
    deduce_reapply_other_file = "deduce-reapply:other-file" (name: &dyn std::fmt::Display) => format!(
        "`+{name}` is trusted, so it is allowed only in the file that declares `{name}` — \
         exactly like a constructor"
    );
    /// A `+Q` in a `=>[…]` group.
    deduce_reapply_group_claim = "deduce-reapply:group-claim" (qualifier: &dyn std::fmt::Display, fn_name: &dyn std::fmt::Display) => format!(
        "`+{qualifier}` cannot appear in `{fn_name}`'s contract: a `=>[{fn_name}]` group \
         says what *that* function does, and nothing here could establish the claim on \
         its behalf. State what a call of someone else's function leaves behind with a \
         `refn` in the qualifier"
    );
    /// A `+Q` on no parameter.
    deduce_reapply_no_param = "deduce-reapply:no-param" (qualifier: &dyn std::fmt::Display) => format!(
        "`+{qualifier}` states what this function establishes about a *parameter*, so it \
         needs one: write it on the parameter's entry (`=> p: +{qualifier} Mut`)"
    );
    /// A `+Q` of a compiler qualifier.
    deduce_reapply_compiler_qualifier = "deduce-reapply:compiler-qualifier" (name: &dyn std::fmt::Display) => format!(
        "`{name}` cannot be established: it is the compiler's own qualifier — a \
         representation choice rather than a claim about the value — and nothing could \
         establish it. Only a qualifier declared in this file may be written `+{name}`"
    );
    /// A `+Q` of a qualifier another file declares.
    deduce_reapply_foreign_qualifier = "deduce-reapply:foreign-qualifier" (name: &dyn std::fmt::Display) => format!(
        "`+{name}` establishes `{name}`, which this file does not declare: a function may \
         only claim to establish a qualifier its own file owns, the same rule a \
         constructor fn and a refinement follow. Declare a constructor there, or consume \
         the value and return it as `{name}`"
    );
    /// A `+Q<…>` whose arguments differ from the parameter's claim.
    deduce_reapply_args_differ = "deduce-reapply:args-differ" (name: &dyn std::fmt::Display, args: &dyn std::fmt::Display, param: &dyn std::fmt::Display, declared: &dyn std::fmt::Display) => format!(
        "`+{name}<{args}>` does not match the claim on `{param}`, which is \
         `{name}<{declared}>`: a re-established claim is the parameter's own, so its \
         arguments cannot differ — a *different* one is a new value, and belongs in the \
         return type (`-> … as {name}<{args}>`)"
    );
    /// A `+Q` of a qualifier that does not apply to the parameter.
    deduce_reapply_does_not_apply = "deduce-reapply:does-not-apply" (name: &dyn std::fmt::Display, param_ty: &dyn std::fmt::Display, param: &dyn std::fmt::Display) => format!(
        "qualifier `{name}` does not apply to `{param_ty}`, so this function cannot \
         establish it on `{param}`"
    );
    /// A `+Q` of a qualifier with fn slots, written without them.
    deduce_reapply_no_identity = "deduce-reapply:no-identity" (name: &dyn std::fmt::Display, what: &dyn std::fmt::Display, slots: &dyn std::fmt::Display) => format!(
        "`+{name}` establishes a claim with no identity: `{name}` holds {what} that every \
         operation reading the claim binds, so the claim has to name what this function \
         established it with — write `+{name}<…, {slots}>`"
    );

    // [deduce-std-explicit]
    /// An exported std fn whose deduction clause leaves parameters out.
    deduce_std_explicit_uncovered_params = "deduce-std-explicit:uncovered-params" (fn_name: &dyn std::fmt::Display, params: &dyn std::fmt::Display, verb: &dyn std::fmt::Display, fix: &dyn std::fmt::Display) => format!(
        "exported `fn {fn_name}` in a `std = true` tree must declare a deduction clause \
         naming every parameter, so its ownership contract does not depend on the \
         program that uses it: {params} {verb} not covered{fix}"
    );

    // [deduce-syntax]
    /// A bodiless declaration whose clause leaves a parameter out.
    deduce_syntax_bodiless_clause = "deduce-syntax:bodiless-clause" (what: &dyn std::fmt::Display, param: &dyn std::fmt::Display) => format!(
        "{what} has no body to infer from, so its deduction clause must say what happens \
         to `{param}`: `=> {param}` to keep it, `=> !{param}` to consume it"
    );

    // [default-fits]
    /// A default that does not fit its parameter's type.
    default_fits_mismatch = "default-fits:mismatch" (expected: &dyn std::fmt::Display, got: &dyn std::fmt::Display, why: &dyn std::fmt::Display) => format!(
        "this default does not fit its type: expected `{expected}`, found `{got}`{why}"
    );

    // [doc-compilerrules]
    /// An `@compilerrules` without its bracketed rule list (a warning).
    doc_compilerrules_no_rules = "doc-compilerrules:no-rules" () =>
        "`@compilerrules` lists the rules it cites in brackets: `@compilerrules [rule-a, \
         rule-b]`, then how they apply here, if that needs saying";

    // [doc-tags]
    /// A doc tag this version does not know (a warning).
    doc_tags_unknown_tag = "doc-tags:unknown-tag" (tag: &dyn std::fmt::Display, known: &dyn std::fmt::Display) => format!(
        "`@{tag}` is not a doc tag: the tags are {known}"
    );
    /// An `@param` naming no parameter (a warning).
    doc_tags_param_unnamed = "doc-tags:param-unnamed" () =>
        "`@param` names the parameter it describes: `@param name  what it is`";
    /// An `@param` naming a parameter the fn does not have (a warning).
    doc_tags_param_unknown = "doc-tags:param-unknown" (param: &dyn std::fmt::Display, fn_name: &dyn std::fmt::Display) => format!(
        "`@param {param}`: `{fn_name}` has no parameter `{param}`"
    );
    /// An `@param` on something that is not a fn (a warning).
    doc_tags_param_not_fn = "doc-tags:param-not-fn" (name: &dyn std::fmt::Display) => format!(
        "`@param` describes a fn's parameter, and `{name}` is not a fn"
    );

    // [effect-alias]
    /// `use` or `spawn` in an effect alias.
    effect_alias_capability = "effect-alias:capability" () =>
        "an effect alias names effects: `use` and `spawn` are per-function capabilities, \
         not effects a group can stand for";
    /// `any` in an effect alias.
    effect_alias_any = "effect-alias:any" () =>
        "`any` has no meaning in an effect alias: an alias is a set of effects, and `any` \
         is about one instance of one effect — write it where the effect is required";
    /// `any` on an effect alias.
    effect_alias_any_alias = "effect-alias:any-alias" (name: &dyn std::fmt::Display) => format!(
        "`any {name}` is not allowed: `{name}` is an effect alias (several effects), and \
         `any` is about one instance of one effect — write `any` on the individual effect"
    );
    /// An effect alias that is cyclic.
    effect_alias_cycle = "effect-alias:cycle" (alias: &dyn std::fmt::Display, chain: &dyn std::fmt::Display) => format!(
        "the effect alias `{alias}` is cyclic: {chain}"
    );

    // [effect-any]
    /// `any E` in a fn type.
    effect_any_fn_type = "effect-any:fn-type" (effect: &dyn std::fmt::Display) => format!(
        "a fn type's effects carry no strength — `any {effect}` is refused here: declare \
         `[any {effect}]` on the function that calls the value, which inherits the \
         requirement"
    );
    /// A callee of one instance called where many are bound.
    effect_any_single_instance_callee = "effect-any:single-instance-callee" (name: &dyn std::fmt::Display, want: &dyn std::fmt::Display) => format!(
        "`{name}` declares `[{want}]` — one instance, sends in order — but `{want}` is \
         bound to many instances here (`any {want}`): declare `[any {want}]` on `{name}` \
         if any member may take each send, or bind one instance"
    );

    // [effect-at]
    /// A `member@Effect` value whose own implicits cannot be filled.
    effect_at_value_incomplete = "effect-at:value-incomplete" (member: &dyn std::fmt::Display, effect: &dyn std::fmt::Display, want: &dyn std::fmt::Display, text: &dyn std::fmt::Display) => format!(
        "`{member}@{effect}` fits `{want}` here, but cannot be completed: {text}"
    );
    /// A `member@Effect` value several candidates fit.
    effect_at_value_ambiguous = "effect-at:value-ambiguous" (member: &dyn std::fmt::Display, effect: &dyn std::fmt::Display, want: &dyn std::fmt::Display, text: &dyn std::fmt::Display) => format!(
        "`{member}@{effect}` is ambiguous at `{want}`: {text}"
    );
    /// A `member@Effect` value that does not fit, with the reason.
    effect_at_value_near_miss = "effect-at:value-near-miss" (member: &dyn std::fmt::Display, effect: &dyn std::fmt::Display, want: &dyn std::fmt::Display, reason: &dyn std::fmt::Display) => format!(
        "`{member}@{effect}` does not fit `{want}`: {reason}"
    );
    /// A `member@Effect` value that does not fit.
    effect_at_value_no_fit = "effect-at:value-no-fit" (member: &dyn std::fmt::Display, effect: &dyn std::fmt::Display, want: &dyn std::fmt::Display) => format!(
        "`{member}@{effect}` does not fit `{want}`"
    );
    /// A `member@Effect` value where no function type is expected.
    effect_at_value_not_expected = "effect-at:value-not-expected" (member: &dyn std::fmt::Display, effect: &dyn std::fmt::Display) => format!(
        "`{member}@{effect}` selects an effect's member, which is a value only where a \
         function type is expected and a handler for it is in scope: call it \
         (`{member}@{effect}(…)`), or pass it where a function is wanted"
    );
    /// An `@Effect` naming no effect in scope.
    effect_at_unknown_effect = "effect-at:unknown-effect" (effect: &dyn std::fmt::Display) => format!(
        "no effect named `{effect}` is in scope"
    );
    /// An `@Effect` call naming no member of the effect.
    effect_at_no_member = "effect-at:no-member" (effect: &dyn std::fmt::Display, member: &dyn std::fmt::Display) => format!(
        "effect `{effect}` has no member named `{member}`"
    );
    /// A member call several effects in scope could go through.
    effect_at_ambiguous_member = "effect-at:ambiguous-member" (name: &dyn std::fmt::Display, owners: &dyn std::fmt::Display, first: &dyn std::fmt::Display) => format!(
        "`{name}` is a member of {owners} — more than one is in scope, so the call must \
         pick its effect: `{name}@{first}(…)`"
    );

    // [effect-available]
    /// A lambda passed where a member and a function of one name both take it.
    effect_available_lambda_member_or_fn = "effect-available:lambda-member-or-fn" (name: &dyn std::fmt::Display, effect: &dyn std::fmt::Display, why: &dyn std::fmt::Display) => format!(
        "cannot type this lambda: `{name}` is both a member of `{effect}` and a function, \
         and {why}; write the lambda's parameter types"
    );
    /// A call no member or function of its name accepts.
    effect_available_no_candidate = "effect-available:no-candidate" (name: &dyn std::fmt::Display, args: &dyn std::fmt::Display, candidates: &dyn std::fmt::Display) => format!(
        "no `{name}` accepts ({args}): the candidates are {candidates}"
    );
    /// A call a member and a function both accept, neither more specific.
    effect_available_member_or_fn = "effect-available:member-or-fn" (name: &dyn std::fmt::Display, effect: &dyn std::fmt::Display, member_params: &dyn std::fmt::Display, fn_params: &dyn std::fmt::Display, args: &dyn std::fmt::Display, module: &dyn std::fmt::Display) => format!(
        "ambiguous call to `{name}`: the member `{effect}.{name}({member_params})` and \
         the function `{name}({fn_params})` both accept ({args}), and neither is more \
         specific — write `{name}@{effect}(…)` for the member or `{name}@{module}(…)` \
         for the function"
    );
    /// A function value whose effect has no handler in scope.
    effect_available_fn_value = "effect-available:fn-value" (want: &dyn std::fmt::Display) => format!(
        "no handler for effect `{want}` in scope, required by this function value \
         (declare it in the function's effect list or `use` a handler)"
    );
    /// A member call none of whose effects has a handler in scope.
    effect_available_member_no_handler = "effect-available:member-no-handler" (name: &dyn std::fmt::Display, owners: &dyn std::fmt::Display) => format!(
        "`{name}` is a member of {owners}, none of which has a handler in scope: declare \
         one in the function's effect list or `use` a handler"
    );
    /// A callee's effect with no handler in scope.
    effect_available_callee_no_handler = "effect-available:callee-no-handler" (want: &dyn std::fmt::Display, name: &dyn std::fmt::Display) => format!(
        "no handler for effect `{want}` in scope, required by `{name}` (declare it in the \
         function's effect list or `use` a handler)"
    );
    /// An effect with no handler in scope.
    effect_available_no_handler = "effect-available:no-handler" (want: &dyn std::fmt::Display) => format!(
        "no handler for effect `{want}` in scope (declare it in the function's effect \
         list or `use` a handler)"
    );
    /// A handler member calling a member of its own handler's effect.
    effect_available_own_effect = "effect-available:own-effect" (member: &dyn std::fmt::Display, of: &dyn std::fmt::Display, remedy: &dyn std::fmt::Display) => format!(
        "a handler member cannot call `{member}`, a member of `{of}` — the effect its own \
         handler implements: a handler cannot dispatch to itself, and declaring `{of}` \
         as a dependency would bind to the handler registered *before* this one. {remedy}"
    );
    /// A member call no handler in scope matches.
    effect_available_no_handler_matches = "effect-available:no-handler-matches" (effect: &dyn std::fmt::Display) => format!(
        "no handler for effect `{effect}` in scope matches this call (declare it in the \
         function's effect list or `use` a handler)"
    );

    // [effect-disambiguation]
    /// A member call several handlers of one effect could take.
    effect_disambiguation_ambiguous = "effect-disambiguation:ambiguous" (effect: &dyn std::fmt::Display, member: &dyn std::fmt::Display) => format!(
        "ambiguous effect call: multiple `{effect}` handlers in scope; specify the type, \
         e.g. `{member}<T>()`"
    );

    // [effect-eff-handle]
    /// A projection `e[A]` of something that is not an effect handle.
    effect_eff_handle_not_a_handle = "effect-eff-handle:not-a-handle" (base_ty: &dyn std::fmt::Display) => format!(
        "`{base_ty}` is not an effect handle, so it has no effects to project — \
         projection `e[A]` narrows an `eff[…]` handle"
    );
    /// A projection of an effect the handle does not carry.
    effect_eff_handle_effect_not_carried = "effect-eff-handle:effect-not-carried" (base_ty: &dyn std::fmt::Display, w: &dyn std::fmt::Display) => format!(
        "`{base_ty}` does not carry effect `{w}`, so it cannot be projected out — \
         projection narrows to a subset"
    );
    /// An `eff[…]` merge item that is not a handle.
    effect_eff_handle_merge_not_handle = "effect-eff-handle:merge-not-handle" (ty: &dyn std::fmt::Display) => format!(
        "`eff[…]` merges effect handles, and `{ty}` is not one: each item is a bound \
         handle"
    );

    // [effect-generic-decl]
    /// An effect-argument effect bound while a sibling instance is.
    effect_generic_decl_sibling_bound = "effect-generic-decl:sibling-bound" (effect: &dyn std::fmt::Display, sibling: &dyn std::fmt::Display, name: &dyn std::fmt::Display) => format!(
        "`{effect}` cannot be bound while `{sibling}` is: an effect whose arguments are \
         effects is one type in the generated code, so one scope holds one instance of \
         `{name}` — bind this one in a nested block (`{{ use …; … }}`), or after the \
         other has gone out of scope"
    );

    // [effect-handler-deps]
    /// A handler whose dependency has no handler in scope.
    effect_handler_deps_missing = "effect-handler-deps:missing" (handler: &dyn std::fmt::Display, want: &dyn std::fmt::Display) => format!(
        "handler `{handler}` depends on effect `{want}`, which has no handler in scope \
         here: register one before it, or supply it with `with {want}Handler(...)`"
    );
    /// A handler depending on `use`.
    effect_handler_deps_use = "effect-handler-deps:use" (handler: &dyn std::fmt::Display) => format!(
        "handler `{handler}` cannot depend on `use`: a handler registers no handlers of \
         its own"
    );
    /// A handler depending on `Throw`.
    effect_handler_deps_throw = "effect-handler-deps:throw" (handler: &dyn std::fmt::Display, throw_effect: &dyn std::fmt::Display) => format!(
        "handler `{handler}` cannot depend on `{throw_effect}`: a throw is delimited by a \
         `try {{ ... }}` block, not supplied by a handler"
    );
    /// A handler declaring one dependency twice.
    effect_handler_deps_duplicate = "effect-handler-deps:duplicate" (handler: &dyn std::fmt::Display, effect: &dyn std::fmt::Display) => format!(
        "handler `{handler}` declares `{effect}` twice"
    );

    // [effect-handler-generics]
    /// A handler written with the wrong number of type arguments.
    effect_handler_generics_type_arg_count = "effect-handler-generics:type-arg-count" (handler: &dyn std::fmt::Display, expected: &dyn std::fmt::Display, found: &dyn std::fmt::Display) => format!(
        "handler `{handler}` takes {expected} type argument(s), found {found}"
    );
    /// A handler type argument an argument contradicts.
    effect_handler_generics_conflict = "effect-handler-generics:conflict" (handler: &dyn std::fmt::Display, name: &dyn std::fmt::Display, written: &dyn std::fmt::Display, ty: &dyn std::fmt::Display) => format!(
        "handler `{handler}` was used at `{name} = {written}`, but an argument makes it \
         `{ty}`"
    );
    /// A handler argument that does not fit its parameter.
    effect_handler_generics_arg_mismatch = "effect-handler-generics:arg-mismatch" (handler: &dyn std::fmt::Display, pname: &dyn std::fmt::Display, sp: &dyn std::fmt::Display, logical: &dyn std::fmt::Display) => format!(
        "handler `{handler}` declares `{pname}: {sp}`, and this argument is `{logical}`"
    );

    // [effect-handler-multi]
    /// A handler naming one effect twice.
    effect_handler_multi_face_twice = "effect-handler-multi:face-twice" (handler: &dyn std::fmt::Display, ty: &dyn std::fmt::Display) => format!(
        "handler `{handler}` already implements `{ty}`: a handler wears each face once"
    );
    /// An intrinsic or platform handler of several effects.
    effect_handler_multi_bodyless_faces = "effect-handler-multi:bodyless-faces" (kind: &dyn std::fmt::Display, handler: &dyn std::fmt::Display) => format!(
        "`{kind} handler {handler}` implements several effects: its members are \
         implemented outside Salvo, where one class implements one generated interface — \
         declare one handler per effect"
    );
    /// A handler missing a member of an effect it implements.
    effect_handler_multi_missing_member = "effect-handler-multi:missing-member" (handler: &dyn std::fmt::Display, effect: &dyn std::fmt::Display, member: &dyn std::fmt::Display, params: &dyn std::fmt::Display) => format!(
        "handler `{handler}` does not implement `{effect}.{member}({params})`: a handler \
         covers every member of every effect it implements"
    );
    /// One member implementing two faces' members whose signatures differ.
    effect_handler_multi_member_conflict = "effect-handler-multi:member-conflict" (member: &dyn std::fmt::Display, first: &dyn std::fmt::Display, first_member: &dyn std::fmt::Display, other: &dyn std::fmt::Display, other_member: &dyn std::fmt::Display) => format!(
        "`{member}` implements `{first}.{first_member}` and `{other}.{other_member}`, \
         whose signatures differ in what overloading cannot see — the parameters are the \
         same, so one member cannot answer for both: give them different parameters, or \
         split the faces across two handlers"
    );

    // [effect-intercept]
    /// An intercepting handler with nothing in scope to wrap.
    effect_intercept_nothing_to_wrap = "effect-intercept:nothing-to-wrap" (handler: &dyn std::fmt::Display, want: &dyn std::fmt::Display) => format!(
        "handler `{handler}` intercepts `{want}` — it depends on the effect it implements \
         — but no handler for `{want}` is registered before this `use`, and the `with` \
         clause does not supply one: an intercepting handler wraps the instance already \
         in scope, or the one you name with `with`"
    );

    // [effect-member-call]
    /// A member call with the wrong number of arguments.
    effect_member_call_arg_count = "effect-member-call:arg-count" (member: &dyn std::fmt::Display, fixed: &dyn std::fmt::Display, found: &dyn std::fmt::Display) => format!(
        "`{member}` takes {fixed} argument(s), found {found}"
    );
    /// A member call argument of the wrong type.
    effect_member_call_arg_mismatch = "effect-member-call:arg-mismatch" (member: &dyn std::fmt::Display, want: &dyn std::fmt::Display, got: &dyn std::fmt::Display) => format!(
        "`{member}` expects `{want}` here, found `{got}`"
    );

    // [effect-member-no-effects]
    /// Effect dependencies on a member.
    effect_member_no_effects_declared = "effect-member-no-effects:declared" (what: &dyn std::fmt::Display) => format!(
        "{what} cannot declare effect dependencies yet"
    );

    // [effect-member-overload]
    /// A member call no overload of the member takes.
    effect_member_overload_no_fit = "effect-member-overload:no-fit" (effect: &dyn std::fmt::Display, member: &dyn std::fmt::Display, args: &dyn std::fmt::Display) => format!(
        "no overload of `{effect}.{member}` takes ({args})"
    );
    /// A member call several overloads take equally.
    effect_member_overload_ambiguous = "effect-member-overload:ambiguous" (effect: &dyn std::fmt::Display, member: &dyn std::fmt::Display, args: &dyn std::fmt::Display) => format!(
        "ambiguous call to `{effect}.{member}`: several overloads take ({args}), and none \
         is more specific"
    );

    // [effect-member-unique]
    /// Two members of an effect with one name and parameter types.
    effect_member_unique_duplicate = "effect-member-unique:duplicate" (effect: &dyn std::fmt::Display, member: &dyn std::fmt::Display) => format!(
        "effect `{effect}` already declares a member named `{member}` with these \
         parameter types: members overload like functions, so two of them must differ in \
         what they take"
    );

    // [effect-no-dup]
    /// One effect twice in an effect list.
    effect_no_dup_duplicate = "effect-no-dup:duplicate" (effect: &dyn std::fmt::Display) => format!(
        "duplicate effect `{effect}` in the effect list (two effects of the same type \
         must differ in their generic arguments)"
    );

    // [effect-not-data]
    /// An effect in a data position.
    effect_not_data_data_position = "effect-not-data:data-position" (name: &dyn std::fmt::Display) => format!(
        "`{name}` is an effect, not a data type: effects appear in a function's effect \
         list (`[{name}]`) or a handler's `of` clause, and their handlers are reached \
         with `use`"
    );

    // [effect-prereq]
    /// Prerequisites on an actor effect.
    effect_prereq_actor_effect = "effect-prereq:actor-effect" (name: &dyn std::fmt::Display) => format!(
        "`actor effect {name}` cannot have prerequisites: a mailbox protocol that needs \
         another effect in scope is not supported yet"
    );
    /// `use` or `spawn` as a prerequisite.
    effect_prereq_capability = "effect-prereq:capability" () =>
        "a prerequisite names an effect: `use` and `spawn` are capabilities of a \
         function, not something an effect can require";
    /// `any` in a prerequisite.
    effect_prereq_any = "effect-prereq:any" () =>
        "`any` has no meaning in a prerequisite: the prerequisite is the one instance \
         bound around every use of the effect";
    /// A prerequisite naming no effect.
    effect_prereq_unknown = "effect-prereq:unknown" (pname: &dyn std::fmt::Display, name: &dyn std::fmt::Display) => format!(
        "unknown effect `{pname}` in the prerequisites of `{name}`"
    );
    /// An actor effect as a prerequisite.
    effect_prereq_actor_prereq = "effect-prereq:actor-prereq" (prereq: &dyn std::fmt::Display) => format!(
        "`{prereq}` is an actor effect, which cannot be a prerequisite yet"
    );
    /// An exported effect whose prerequisite is not exported.
    effect_prereq_not_exported = "effect-prereq:not-exported" (name: &dyn std::fmt::Display, prereq: &dyn std::fmt::Display) => format!(
        "`{name}` is exported but its prerequisite `{prereq}` is not: every module that \
         names `{name}` needs `{prereq}` too — export it"
    );
    /// Prerequisites that form a cycle.
    effect_prereq_cycle = "effect-prereq:cycle" (chain: &dyn std::fmt::Display) => format!(
        "effect prerequisites form a cycle: {chain}"
    );
    /// A handler whose face's prerequisite is not in scope.
    effect_prereq_face_needs = "effect-prereq:face-needs" (handler: &dyn std::fmt::Display, face: &dyn std::fmt::Display, want: &dyn std::fmt::Display) => format!(
        "handler `{handler}` implements `{face}`, which needs `{want}` in scope: `use` a \
         handler of `{want}` before it"
    );

    // [effect-state-store]
    /// A value moved out of handler storage the handler still owns.
    effect_state_store_moved_out = "effect-state-store:moved-out" (action: &dyn std::fmt::Display, name: &dyn std::fmt::Display, kind: &dyn std::fmt::Display, handler: &dyn std::fmt::Display) => format!(
        "cannot {action} `{name}`: it is a {kind} of handler `{handler}`, which still \
         owns it after this member returns — so the value cannot move out. Use \
         `copy({name})` to hand over an independent value"
    );

    // [effect-use-requires-use]
    /// A `use` in a fn without the `use` effect.
    effect_use_requires_use_missing = "effect-use-requires-use:missing" () =>
        "`use` requires the `use` effect in the function's effect list";

    // [effect-with-clause]
    /// A `with` clause on a `use` of an existing handle.
    effect_with_clause_on_handle = "effect-with-clause:on-handle" () =>
        "`with` supplies a handler's dependencies at construction, and this `use` binds \
         an existing handle — its dependencies were settled where it was built: drop the \
         clause";
    /// A `with` item that is neither a handler nor a handle.
    effect_with_clause_not_a_handler = "effect-with-clause:not-a-handler" (ty: &dyn std::fmt::Display) => format!(
        "a `with` clause supplies handlers: write a handler construction \
         (`SomeHandler(...)`) or an `eff[…]` of the effect a shared handler already \
         serves, not a `{ty}`"
    );
    /// A handler of several effects constructed in a `with` clause.
    effect_with_clause_several_faces = "effect-with-clause:several-faces" (handler: &dyn std::fmt::Display, faces: &dyn std::fmt::Display) => format!(
        "handler `{handler}` implements several effects ({faces}), so it cannot be \
         constructed in a `with` clause: the clause builds one private instance per item \
         — bind it with its own `use` (or spawn it) and supply an `eff[…]` per face"
    );
    /// A handler with dependencies constructed in a `with` clause.
    effect_with_clause_has_dependencies = "effect-with-clause:has-dependencies" (handler: &dyn std::fmt::Display, dep: &dyn std::fmt::Display) => format!(
        "handler `{handler}` depends on effect `{dep}`, so it cannot be constructed in a \
         `with` clause: a clause item is a private instance with no scope to resolve its \
         own dependencies — register it with `use` before this one instead"
    );
    /// A `with` item the handler does not depend on.
    effect_with_clause_not_a_dep = "effect-with-clause:not-a-dep" (handler: &dyn std::fmt::Display, eff: &dyn std::fmt::Display) => format!(
        "handler `{handler}` does not depend on effect `{eff}`, so supplying it with \
         `with` has no effect"
    );

    // [expr-cond-bool]
    /// An `assert!` condition that is not a `Bool`.
    expr_cond_bool_assert_condition = "expr-cond-bool:assert-condition" (t: &dyn std::fmt::Display) => format!(
        "an `assert!` condition must be a `Bool`, found `{t}`"
    );
    /// A condition that is not a `Bool`.
    expr_cond_bool_not_bool = "expr-cond-bool:not-bool" (ty: &dyn std::fmt::Display) => format!(
        "a condition must be a `Bool` (found `{ty}`): Salvo has no truthiness — compare \
         explicitly (`n != 0`, `list.size() > 0`) or test the type with `is`"
    );

    // [expr-dead-code]
    /// Code after a statement that never returns (a warning).
    expr_dead_code_unreachable = "expr-dead-code:unreachable" () =>
        "this code is unreachable: an earlier statement never returns";

    // [expr-escape]
    /// `break` outside a loop.
    expr_escape_break_outside_loop = "expr-escape:break-outside-loop" () =>
        "`break` outside of a loop";
    /// `continue` outside a loop.
    expr_escape_continue_outside_loop = "expr-escape:continue-outside-loop" () =>
        "`continue` outside of a loop";

    // [expr-pipe]
    /// A `|>` whose right side does not use `_`.
    expr_pipe_unused_placeholder = "expr-pipe:unused-placeholder" () =>
        "the right side of `|>` must use the piped value `_` (`x |> f(_)`); it reads \
         nothing here, so write a plain statement instead";

    // [expr-placeholder]
    /// `_` read outside a pick or a pipe.
    expr_placeholder_outside_pick = "expr-placeholder:outside-pick" () =>
        "`_` is the arms a pick left behind, so it reads only in the right-hand side of a \
         pick (`r ?: f(_)`) or a pipe (`x |> f(_)`)";

    // [expr-safe-pipe]
    /// `?>` on a value with one arm.
    expr_safe_pipe_single_arm = "expr-safe-pipe:single-arm" (lhs_ty: &dyn std::fmt::Display) => format!(
        "`?>` pipes the first arm of a union, but this value is `{lhs_ty}` — it has only \
         one arm, so write a plain `|>`"
    );
    /// A `?>` whose right side does not use `_`.
    expr_safe_pipe_unused_placeholder = "expr-safe-pipe:unused-placeholder" () =>
        "the right side of `?>` must use the piped value `_` (`x ?> f(_)`); it reads \
         nothing here, so write a plain statement instead";

    // [expr-tuple-index]
    /// An assignment to a tuple element.
    expr_tuple_index_assign = "expr-tuple-index:assign" (index: &dyn std::fmt::Display) => format!(
        "cannot assign to tuple element `{index}`: `Mut` cannot apply to a tuple, so \
         tuple elements are read-only — rebuild the tuple instead"
    );
    /// A tuple element index past the end.
    expr_tuple_index_out_of_range = "expr-tuple-index:out-of-range" (base_ty: &dyn std::fmt::Display, len: &dyn std::fmt::Display, index: &dyn std::fmt::Display) => format!(
        "tuple `{base_ty}` has {len} element(s), so it has no element `{index}`"
    );
    /// A tuple element read on something that is not a tuple.
    expr_tuple_index_not_a_tuple = "expr-tuple-index:not-a-tuple" (index: &dyn std::fmt::Display, base_ty: &dyn std::fmt::Display) => format!(
        "`.{index}` reads a tuple element, but `{base_ty}` is not a tuple"
    );

    // [fate-derived-readonly]
    /// A view moved or mutated where it can only be read.
    fate_derived_readonly_view_moved = "fate-derived-readonly:view-moved" (action: &dyn std::fmt::Display, root: &dyn std::fmt::Display) => format!(
        "cannot {action} this value: it is a view of `{root}` (it borrows from it), so it \
         can only be read here; use `copy` to make an independent value, or declare the \
         borrow on this function's return (`proj({root})`)"
    );

    // [fate-elem-distinct]
    /// Two mutable handles into one container in one call.
    fate_elem_distinct_two_handles = "fate-elem-distinct:two-handles" (root: &dyn std::fmt::Display) => format!(
        "a mutable element handle of `{root}` cannot be passed here: an earlier argument \
         of this call is already a mutable handle into `{root}`, and the two may be the \
         same element; bind both and prove them apart first (`b is NotSame(a)`), or \
         mutate through one handle at a time"
    );

    // [fate-lambda]
    /// A lambda that captures and mutates a kept parameter.
    fate_lambda_captures_kept = "fate-lambda:captures-kept" (name: &dyn std::fmt::Display) => format!(
        "this lambda captures and mutates `{name}`, which is a kept parameter (the caller \
         keeps it): capture `copy({name})` instead, or declare the move with `=> !{name}`"
    );

    // [fate-move-mode]
    /// Mutable data moved out of a kept parameter.
    fate_move_mode_kept_mutable = "fate-move-mode:kept-mutable" (root: &dyn std::fmt::Display) => format!(
        "cannot move mutable data out of `{root}`: it is a kept parameter (the caller \
         keeps it), so its projections can only be read; use `copy` to pass an \
         independent value"
    );

    // [fn-attached]
    /// A function passed by name with an `@Type` that declares none.
    fn_attached_no_fn_on_type = "fn-attached:no-fn-on-type" (name: &dyn std::fmt::Display, wanted: &dyn std::fmt::Display) => format!(
        "no `fn {name}@{wanted}` is declared: `@{wanted}` names the canonical `{name}` \
         for that type, which is declared in the type's own file"
    );
    /// An `@Type` call whose type declares no fitting fn.
    fn_attached_no_fit_on_type = "fn-attached:no-fit-on-type" (name: &dyn std::fmt::Display, wanted: &dyn std::fmt::Display, args: &dyn std::fmt::Display) => format!(
        "no `fn {name}@{wanted}` fits `{name}({args})`: `@{wanted}` names the canonical \
         `{name}` for that type, declared in the type's own file"
    );
    /// A function identity `name@Type` whose type declares no such fn.
    fn_attached_no_identity_on_type = "fn-attached:no-identity-on-type" (name: &dyn std::fmt::Display, at_name: &dyn std::fmt::Display) => format!(
        "no `{name}@{at_name}` in scope: declare `fn {name}@{at_name}(…)` in the file \
         that declares `{at_name}`"
    );

    // [fn-contract]
    /// A lambda consuming a parameter its contract keeps.
    fn_contract_consume_kept = "fn-contract:consume-kept" (name: &dyn std::fmt::Display) => format!(
        "cannot consume `{name}`: this lambda's contract keeps it (the caller retains the \
         value); use `copy({name})`"
    );
    /// A fn type's contract naming no parameter of it.
    fn_contract_not_a_param = "fn-contract:not-a-param" (name: &dyn std::fmt::Display) => format!(
        "`{name}` names no parameter of this function type (name the parameter in the \
         type: `({name}: ...) -> ...`)"
    );

    // [fn-dot]
    /// A dot-notation call naming no function in scope.
    fn_dot_unknown_fn = "fn-dot:unknown-fn" (name: &dyn std::fmt::Display) => format!(
        "no function named `{name}` is in scope: dot-notation calls a function with the \
         receiver as its first argument, so a target-language method is reached by \
         declaring it in an effect a `platform handler` implements"
    );

    // [fn-effects]
    /// `use` in a fn type.
    fn_effects_use = "fn-effects:use" () =>
        "a fn type cannot declare `use`: registering a handler is local to a body, so a \
         lambda may `use` exactly when the function containing it may";
    /// An effect written as a qualifier of a fn type.
    fn_effects_in_qualifier_position = "fn-effects:in-qualifier-position" (effect: &dyn std::fmt::Display) => format!(
        "a function type declares its effects in its own list (`(…) [{effect}] -> …`), \
         not in qualifier position"
    );
    /// An effect written as a qualifier of a type.
    fn_effects_effect_as_qualifier = "fn-effects:effect-as-qualifier" (effect: &dyn std::fmt::Display) => format!(
        "`{effect}` is an effect, not a qualifier: an iterator performs its effects in \
         its `next`, so declare them there (`yield fn next(…) [{effect}] -> …`) rather \
         than on the type"
    );

    // [fn-must-return]
    /// A path through a fn that returns no value.
    fn_must_return_missing_return = "fn-must-return:missing-return" (fn_name: &dyn std::fmt::Display, ret: &dyn std::fmt::Display) => format!(
        "missing return: not all paths in `{fn_name}` return a value (declared return \
         type is `{ret}`)"
    );

    // [fn-overload-ambiguous]
    /// An implicit several candidates fit equally.
    fn_overload_ambiguous_implicit = "fn-overload-ambiguous:implicit" (implicit: &dyn std::fmt::Display, callee: &dyn std::fmt::Display, want: &dyn std::fmt::Display, text: &dyn std::fmt::Display) => format!(
        "`{implicit}` is ambiguous for `{callee}` at `?{implicit}: {want}`: {text}"
    );
    /// A call several overloads fit with none more specific.
    fn_overload_ambiguous_call = "fn-overload-ambiguous:call" (name: &dyn std::fmt::Display, args: &dyn std::fmt::Display, candidates: &dyn std::fmt::Display, remedy: &dyn std::fmt::Display) => format!(
        "ambiguous call to `{name}({args})`: {candidates} all fit and none is more \
         specific — a broader union, a smaller qualifier set and a type variable are \
         each less specific, but these differ in ways the rule does not rank. {remedy}"
    );

    // [fn-overload-at]
    /// An `@` selector that is a suffix of several modules.
    fn_overload_at_ambiguous_suffix = "fn-overload-at:ambiguous-suffix" (wanted: &dyn std::fmt::Display, modules: &dyn std::fmt::Display) => format!(
        "`@{wanted}` is ambiguous: it is a suffix of {modules} — write more of the path"
    );
    /// A function passed by name with an `@module` that declares none.
    fn_overload_at_no_fn_in_module = "fn-overload-at:no-fn-in-module" (name: &dyn std::fmt::Display, wanted: &dyn std::fmt::Display) => format!(
        "no function named `{name}` is declared in module `{wanted}`"
    );
    /// An `@module` call whose module declares no fitting overload.
    fn_overload_at_no_overload_in_module = "fn-overload-at:no-overload-in-module" (name: &dyn std::fmt::Display, args: &dyn std::fmt::Display, wanted: &dyn std::fmt::Display, modules: &dyn std::fmt::Display) => format!(
        "no overload of `{name}({args})` is declared in module `{wanted}`; the modules \
         that declare a fitting one are: {modules}"
    );
    /// An `@` selector the call does not need (a warning).
    fn_overload_at_not_needed = "fn-overload-at:not-needed" (written: &dyn std::fmt::Display, name: &dyn std::fmt::Display, args: &dyn std::fmt::Display) => format!(
        "this `@{written}` is not needed: `{name}({args})` resolves to the same function \
         without it, and no refinement of it disagrees here"
    );
    /// A function identity `name@module` whose module declares no such fn.
    fn_overload_at_no_identity_in_module = "fn-overload-at:no-identity-in-module" (name: &dyn std::fmt::Display, at_name: &dyn std::fmt::Display) => format!(
        "no `{name}@{at_name}` in scope: module `{at_name}` declares no `{name}`"
    );
    /// A receiver on a scope-selected fn value.
    fn_overload_at_value_receiver = "fn-overload-at:value-receiver" (name: &dyn std::fmt::Display, module: &dyn std::fmt::Display) => format!(
        "`{name}` here is a *value*, so it takes no receiver: write `{name}@{module}` to \
         name the function, or call it"
    );

    // [fn-overload-rank]
    /// A lambda argument the overloads cannot type.
    fn_overload_rank_lambda_untyped = "fn-overload-rank:lambda-untyped" (name: &dyn std::fmt::Display, why: &dyn std::fmt::Display) => format!(
        "cannot type this lambda: the overloads of `{name}` {why}; write the lambda's \
         parameter types"
    );

    // [fn-overload-return]
    /// Return-type overloads whose return types overlap.
    fn_overload_return_overlap = "fn-overload-return:overlap" (fn_name: &dyn std::fmt::Display, a: &dyn std::fmt::Display, b: &dyn std::fmt::Display, expected: &dyn std::fmt::Display) => format!(
        "`{fn_name}` is overloaded on its return type alone, and `{a}` and `{b}` overlap: \
         where `{expected}` is expected both would fit, so nothing could choose. Give \
         the overloads return types neither of which is a subtype of the other, or \
         different parameters"
    );
    /// Return-type overloads the call site does not choose between.
    fn_overload_return_ambiguous = "fn-overload-return:ambiguous" (name: &dyn std::fmt::Display, args: &dyn std::fmt::Display, rets: &dyn std::fmt::Display, why: &dyn std::fmt::Display, first: &dyn std::fmt::Display) => format!(
        "`{name}({args})` has overloads that differ only in their return type ({rets}), \
         and {why}: annotate where the result goes (`let x: {first} = {name}(…)`), or \
         pass it where one type is expected"
    );

    // [fn-param-const]
    /// An assignment to a parameter.
    fn_param_const_assigned = "fn-param-const:assigned" (how: &dyn std::fmt::Display, name: &dyn std::fmt::Display) => format!(
        "cannot {how} parameter `{name}`: a parameter is a constant binding. Bind a local \
         instead (`let next = {name} + 1`), or — to change what the caller holds — \
         assign a field of it (`{name}.field = …`)"
    );

    // [fn-param-default]
    /// A handler argument given by name that has no default.
    fn_param_default_named_without_default = "fn-param-default:named-without-default" (name: &dyn std::fmt::Display) => format!(
        "`{name}` has no default, so it cannot be given by name here: only a defaulted \
         parameter may be named"
    );
    /// A named argument to a handler with a variadic parameter.
    fn_param_default_variadic_named = "fn-param-default:variadic-named" (handler: &dyn std::fmt::Display) => format!(
        "handler `{handler}` has a variadic constructor parameter, so its arguments are \
         given positionally"
    );
    /// A handler argument given both by position and by name.
    fn_param_default_named_and_positional = "fn-param-default:named-and-positional" (name: &dyn std::fmt::Display) => format!(
        "`{name}` is already given positionally"
    );
    /// A handler argument given by name twice.
    fn_param_default_named_twice = "fn-param-default:named-twice" (name: &dyn std::fmt::Display) => format!(
        "`{name}` is given by name twice"
    );
    /// A handler constructed with the wrong number of arguments.
    fn_param_default_handler_arg_count = "fn-param-default:handler-arg-count" (handler: &dyn std::fmt::Display, expected: &dyn std::fmt::Display, found: &dyn std::fmt::Display, dep_note: &dyn std::fmt::Display) => format!(
        "handler `{handler}` expects {expected} constructor argument(s), found \
         {found}{dep_note}"
    );
    /// A handler constructed with too many arguments.
    fn_param_default_handler_too_many = "fn-param-default:handler-too-many" (handler: &dyn std::fmt::Display, max: &dyn std::fmt::Display, found: &dyn std::fmt::Display, dep_note: &dyn std::fmt::Display) => format!(
        "handler `{handler}` takes at most {max} constructor argument(s), found \
         {found}{dep_note}"
    );
    /// A handler constructed without a parameter that has no default.
    fn_param_default_handler_missing = "fn-param-default:handler-missing" (handler: &dyn std::fmt::Display, missing: &dyn std::fmt::Display, verb: &dyn std::fmt::Display, dep_note: &dyn std::fmt::Display) => format!(
        "handler `{handler}` is constructed without {missing}, which ha{verb} no \
         default{dep_note}"
    );

    // [fn-rename]
    /// A rename whose new name is a function in scope.
    fn_rename_name_is_fn = "fn-rename:name-is-fn" (new_name: &dyn std::fmt::Display, target: &dyn std::fmt::Display) => format!(
        "`{new_name}` is already a function in scope, so it cannot name a renamed \
         overload too: pick a name of its own (a rename is not an alias — the overload \
         stops answering to `{target}`)"
    );
    /// A rename whose new name is an effect member in scope.
    fn_rename_name_is_member = "fn-rename:name-is-member" (new_name: &dyn std::fmt::Display) => format!(
        "`{new_name}` is already an effect member in scope"
    );
    /// Two renames with one new name.
    fn_rename_name_taken = "fn-rename:name-taken" (new_name: &dyn std::fmt::Display, start: &dyn std::fmt::Display, end: &dyn std::fmt::Display) => format!(
        "`{new_name}` already names a renamed overload in this scope (declared at \
         {start}..{end})"
    );
    /// A rename of a function that is not visible.
    fn_rename_no_target = "fn-rename:no-target" (target: &dyn std::fmt::Display) => format!(
        "no function `{target}` is visible here, so there is nothing to rename"
    );
    /// A rename whose parameters match no overload.
    fn_rename_no_overload = "fn-rename:no-overload" (target: &dyn std::fmt::Display, shapes: &dyn std::fmt::Display) => format!(
        "this names no `{target}` in scope: a rename repeats one overload's parameters \
         exactly — same names, same types (type parameters match by position). In scope: \
         {shapes}"
    );
    /// An `@module` on a renamed overload.
    fn_rename_at_on_renamed = "fn-rename:at-on-renamed" (name: &dyn std::fmt::Display) => format!(
        "`{name}` is a renamed overload, so it already names one declaration: drop the \
         `@module`"
    );

    // [fn-return-none]
    /// A bare `return` in a fn that returns a value.
    fn_return_none_bare_return = "fn-return-none:bare-return" (expected: &dyn std::fmt::Display) => format!(
        "bare `return` in a function returning `{expected}`"
    );

    // [fn-syntax]
    /// A returned value that does not fit the return type.
    fn_syntax_return_mismatch = "fn-syntax:return-mismatch" (expected: &dyn std::fmt::Display, vty: &dyn std::fmt::Display) => format!(
        "expected return type `{expected}`, found `{vty}`"
    );

    // [fn-type-named]
    /// A type-named function without a return type.
    fn_type_named_no_return = "fn-type-named:no-return" () =>
        "a type-named function must declare a return type";
    /// A type-named function whose return type lacks its qualifier.
    fn_type_named_missing_qualifier = "fn-type-named:missing-qualifier" (fn_name: &dyn std::fmt::Display, named: &dyn std::fmt::Display, q: &dyn std::fmt::Display) => format!(
        "`{fn_name}` is named after `{named}`, so its return type must carry the `{q}` \
         qualifier"
    );
    /// A type-named function whose return type lacks its provenance qualifier.
    fn_type_named_missing_provenance = "fn-type-named:missing-provenance" (fn_name: &dyn std::fmt::Display, bn: &dyn std::fmt::Display) => format!(
        "`{fn_name}` is named after the provenance qualifier `{bn}`, so its return type \
         must carry it (`-> +{bn} T`)"
    );
    /// A type-named function whose return type has another base.
    fn_type_named_wrong_base = "fn-type-named:wrong-base" (fn_name: &dyn std::fmt::Display, named: &dyn std::fmt::Display, bn: &dyn std::fmt::Display, ret: &dyn std::fmt::Display) => format!(
        "`{fn_name}` is named after `{named}`, so its return type's base must be `{bn}`, \
         not `{ret}`"
    );
    /// A type-named function whose name has no base type.
    fn_type_named_no_base = "fn-type-named:no-base" () =>
        "a type-named function's name must have a base type";

    // [fn-value-select]
    /// An overloaded function passed by name with nothing to select by.
    fn_value_select_overloaded = "fn-value-select:overloaded" (name: &dyn std::fmt::Display, candidates: &dyn std::fmt::Display) => format!(
        "`{name}` is overloaded, so passing it by name is ambiguous: {candidates} all fit \
         here. Annotate the position with the fn type you mean, or give one overload its \
         own name with `rename fn <new> = {name}(...)`"
    );
    /// A name passed as a function that is an effect member here.
    fn_value_select_member = "fn-value-select:member" (name: &dyn std::fmt::Display) => format!(
        "`{name}` is not a function here"
    );
    /// A function passed by name that several overloads fit equally.
    fn_value_select_ambiguous = "fn-value-select:ambiguous" (name: &dyn std::fmt::Display, want: &dyn std::fmt::Display, text: &dyn std::fmt::Display) => format!(
        "`{name}` passed by name is ambiguous at `{want}`: {text}"
    );
    /// A function passed by name that no overload fits.
    fn_value_select_no_fit = "fn-value-select:no-fit" (name: &dyn std::fmt::Display, want: &dyn std::fmt::Display, candidates: &dyn std::fmt::Display) => format!(
        "no overload of `{name}` fits `{want}` here: {candidates}"
    );

    // [group-not-a-value]
    /// A `fns` group in a type position.
    group_not_a_value_data_position = "group-not-a-value:data-position" (name: &dyn std::fmt::Display) => format!(
        "`{name}` is a `fns` group, not a type: no value may have a group as its type — a \
         named type satisfies it with `: {name}<...>` on its declaration, or a signature \
         spreads its members with `?{name}<...>`"
    );

    // [group-obligation]
    /// One obligation written twice.
    group_obligation_duplicate = "group-obligation:duplicate" (obligation: &dyn std::fmt::Display) => format!(
        "obligation `{obligation}` is declared more than once"
    );
    /// An obligation naming no `fns` group.
    group_obligation_unknown_group = "group-obligation:unknown-group" (name: &dyn std::fmt::Display, hint: &dyn std::fmt::Display) => format!(
        "unknown `fns` group `{name}`{hint}"
    );
    /// An obligation with the wrong number of type arguments.
    group_obligation_type_arg_count = "group-obligation:type-arg-count" (group: &dyn std::fmt::Display, expected: &dyn std::fmt::Display, found: &dyn std::fmt::Display) => format!(
        "`{group}` takes {expected} type argument(s), found {found}"
    );
    /// An obligation with no visible fn for a member.
    group_obligation_no_member = "group-obligation:no-member" (name: &dyn std::fmt::Display, obligation: &dyn std::fmt::Display, member: &dyn std::fmt::Display, params: &dyn std::fmt::Display, ret: &dyn std::fmt::Display) => format!(
        "`{name}` declares `: {obligation}` but no visible `{member}` matches `fn \
         {member}({params}) -> {ret}`"
    );

    // [handler-init]
    /// A member named `init` beside an `init` block.
    handler_init_member_named_init = "handler-init:member-named-init" (handler: &dyn std::fmt::Display) => format!(
        "handler `{handler}` has an `init` block, so it cannot also declare a member \
         named `init`: the block is that member"
    );

    // [handler-not-value]
    /// `use`/`spawn` of something that is not a handler name or construction.
    handler_not_value_construction = "handler-not-value:construction" (form: &dyn std::fmt::Display) => format!(
        "`{form}` expects a handler name or constructor call"
    );
    /// A handler called like a function.
    handler_not_value_called = "handler-not-value:called" (name: &dyn std::fmt::Display) => format!(
        "`{name}` is a handler, not a value: register it with `use {name}(...)` and call \
         the effect's members"
    );

    // [handler-state-name]
    /// A handler whose default state name is a type's.
    handler_state_name_default_clash = "handler-state-name:default-clash" (handler: &dyn std::fmt::Display) => format!(
        "handler `{handler}`'s state is named `{handler}` by default, which is also a \
         type's name here: name it explicitly, `state {handler}State {{ … }}`"
    );
    /// A handler state named like a declaration in scope.
    handler_state_name_declared_clash = "handler-state-name:declared-clash" (handler: &dyn std::fmt::Display, state: &dyn std::fmt::Display) => format!(
        "handler `{handler}`'s state `{state}` has the name of a declaration already in \
         scope: rename the state"
    );

    // [implicit-fit]
    /// A call whose only fitting overload needs an implicit nothing fills.
    implicit_fit_no_fill = "implicit-fit:no-fill" (name: &dyn std::fmt::Display, args: &dyn std::fmt::Display, module: &dyn std::fmt::Display, missing: &dyn std::fmt::Display) => format!(
        "no matching overload for `{name}({args})`: the one from `{module}` that fits by \
         type needs `{missing}`, which nothing in scope fills"
    );

    // [implicit-fn-only]
    /// An implicit parameter on something other than a fn.
    implicit_fn_only_implicit_param = "implicit-fn-only:implicit-param" (what: &dyn std::fmt::Display) => format!(
        "{what} cannot take implicit parameters: only a fn declaration has a call site \
         that resolves them"
    );
    /// A `fns` group spread on something other than a fn.
    implicit_fn_only_group_spread = "implicit-fn-only:group-spread" (what: &dyn std::fmt::Display) => format!(
        "{what} cannot spread a `fns` group: only a fn declaration has a call site that \
         resolves one"
    );

    // [implicit-group]
    /// A `fns` group member with a body.
    implicit_group_member_body = "implicit-group:member-body" (group: &dyn std::fmt::Display, member: &dyn std::fmt::Display) => format!(
        "`{group}.{member}` is a signature, not an implementation: a `fns` group declares \
         what a caller must supply, and the default comes from a matching top- level fn"
    );
    /// A `?Group` spread naming no group.
    implicit_group_unknown_group = "implicit-group:unknown-group" (group: &dyn std::fmt::Display) => format!(
        "no `fns` group named `{group}` is in scope: `?{group}` spreads a group's members \
         as implicit parameters"
    );
    /// A group spread with the wrong number of type arguments.
    implicit_group_type_arg_count = "implicit-group:type-arg-count" (group: &dyn std::fmt::Display, expected: &dyn std::fmt::Display, found: &dyn std::fmt::Display) => format!(
        "`{group}` takes {expected} type argument(s), found {found}"
    );
    /// A function slot declared twice at different types.
    implicit_group_slot_twice = "implicit-group:slot-twice" (owner: &dyn std::fmt::Display, slot: &dyn std::fmt::Display, first: &dyn std::fmt::Display, second: &dyn std::fmt::Display) => format!(
        "`{owner}` declares the function slot `?{slot}` twice, at two different types \
         (`{first}` and `{second}`): with no binder nothing tells two slots of one name \
         apart"
    );

    // [implicit-override]
    /// A named argument naming no implicit parameter of a callee.
    implicit_override_no_implicit = "implicit-override:no-implicit" (callee: &dyn std::fmt::Display, name: &dyn std::fmt::Display) => format!(
        "`{callee}` has no implicit parameter named `{name}`"
    );
    /// An override that does not fit its implicit.
    implicit_override_does_not_fit = "implicit-override:does-not-fit" (implicit: &dyn std::fmt::Display, detail: &dyn std::fmt::Display) => format!(
        "`{implicit}` does not fit here: {detail}"
    );

    // [implicit-param]
    /// A handler's implicit parameter that is not of a function type.
    implicit_param_handler_not_fn_type = "implicit-param:handler-not-fn-type" () =>
        "an implicit parameter must have a function type: what fills it is resolved as a \
         function of that name";
    /// An implicit parameter that is not of a function type.
    implicit_param_not_fn_type = "implicit-param:not-fn-type" (param: &dyn std::fmt::Display, ty: &dyn std::fmt::Display) => format!(
        "an implicit parameter must have a function type, but `{param}` is `{ty}`: what \
         fills it is resolved as a function of that name"
    );

    // [implicit-recursive]
    /// An implicit of a nested fill that cannot be filled.
    implicit_recursive_nested_unfilled = "implicit-recursive:nested-unfilled" (implicit: &dyn std::fmt::Display, want: &dyn std::fmt::Display) => format!(
        "`?{implicit}: {want}` cannot be filled"
    );
    /// An implicit whose candidate's own implicits cannot be filled.
    implicit_recursive_incomplete = "implicit-recursive:incomplete" (implicit: &dyn std::fmt::Display, want: &dyn std::fmt::Display, callee: &dyn std::fmt::Display, text: &dyn std::fmt::Display) => format!(
        "`?{implicit}: {want}` for `{callee}` cannot be completed: {text}"
    );
    /// A function value whose own implicits cannot be filled.
    implicit_recursive_value_incomplete = "implicit-recursive:value-incomplete" (name: &dyn std::fmt::Display, w: &dyn std::fmt::Display, reason: &dyn std::fmt::Display) => format!(
        "`{name}` fits `{w}` here, but cannot be completed: {reason}"
    );

    // [implicit-resolve]
    /// No function of an implicit's name.
    implicit_resolve_none = "implicit-resolve:none" (implicit: &dyn std::fmt::Display, callee: &dyn std::fmt::Display, want: &dyn std::fmt::Display, remedy: &dyn std::fmt::Display) => format!(
        "no `{implicit}` for this call: `{callee}` needs `?{implicit}: {want}` — {remedy}"
    );
    /// Functions of an implicit's name, none of which fits.
    implicit_resolve_no_fit = "implicit-resolve:no-fit" (implicit: &dyn std::fmt::Display, want: &dyn std::fmt::Display, callee: &dyn std::fmt::Display, reason: &dyn std::fmt::Display) => format!(
        "no `{implicit}` fits `?{implicit}: {want}` for `{callee}`: {reason}"
    );

    // [implicit-same-name]
    /// An override by name of an implicit two positions share.
    implicit_same_name_override = "implicit-same-name:override" (callee: &dyn std::fmt::Display, name: &dyn std::fmt::Display) => format!(
        "`{callee}` has more than one implicit named `{name}`, at different types, so \
         `{name} = …` does not say which it fills; the typed override (`{name}: (…) -> … \
         = f`) is not built yet"
    );
    /// A call that could go through several implicits of one name.
    implicit_same_name_ambiguous = "implicit-same-name:ambiguous" (name: &dyn std::fmt::Display, candidates: &dyn std::fmt::Display) => format!(
        "`{name}` here could go through more than one implicit of that name \
         ({candidates}): the arguments fit each and none is more specific"
    );
    /// An operator several implicits of its name could go through.
    implicit_same_name_operator = "implicit-same-name:operator" (sym: &dyn std::fmt::Display, lt: &dyn std::fmt::Display, member: &dyn std::fmt::Display, rt: &dyn std::fmt::Display, tied: &dyn std::fmt::Display, candidates: &dyn std::fmt::Display) => format!(
        "`{sym}` on `{lt}` is `{member}({lt}, {rt})`, and this function has {tied} of \
         them in scope ({candidates}), and none is more specific: the operator cannot \
         choose, so call the one you mean"
    );

    // [implicit-with]
    /// A `with` entry naming no implicit parameter.
    implicit_with_not_implicit = "implicit-with:not-implicit" (missing: &dyn std::fmt::Display) => format!(
        "`{missing}` is not an implicit parameter of this signature, so it cannot be \
         filled together with anything: a `with` entry relates the `?name` positions a \
         call fills"
    );
    /// Implicits filled together that come from different places.
    implicit_with_different_places = "implicit-with:different-places" (names: &dyn std::fmt::Display, callee: &dyn std::fmt::Display, places: &dyn std::fmt::Display, remedy: &dyn std::fmt::Display) => format!(
        "{names} of `{callee}` are filled together, but they come from different places \
         here: {places}. {remedy}"
    );

    // [interp-no-none]
    /// `None` interpolated into a string.
    interp_no_none_none_value = "interp-no-none:none-value" () =>
        "cannot interpolate `None` into a string: it has no text form";
    /// A value that may be `None` interpolated into a string.
    interp_no_none_optional_value = "interp-no-none:optional-value" (ty: &dyn std::fmt::Display) => format!(
        "cannot interpolate a value that may be `None` (its type is `{ty}`): narrow it \
         first (`is` / `when`) or assert it with `!`"
    );

    // [interp-to-str]
    /// A value whose text form several functions give.
    interp_to_str_ambiguous = "interp-to-str:ambiguous" (ty: &dyn std::fmt::Display, text: &dyn std::fmt::Display) => format!(
        "the text form of `{ty}` is ambiguous: {text}"
    );
    /// A value with no text form, interpolated.
    interp_to_str_no_text = "interp-to-str:no-text" (ty: &dyn std::fmt::Display, remedy: &dyn std::fmt::Display) => format!(
        "`{ty}` has no text form, so it cannot be interpolated: declare `fn to_str({ty}) \
         -> Str` (or one that accepts it) and it will be used here{remedy}"
    );

    // [interp-union]
    /// A union with an arm that has no text form.
    interp_union_arm_no_text = "interp-union:arm-no-text" (ty: &dyn std::fmt::Display, arm: &dyn std::fmt::Display) => format!(
        "`{ty}` cannot be interpolated: its arm `{arm}` has no text form. Declare `fn \
         to_str({arm}) -> Str` (or one that accepts it); a union is rendered by the \
         `to_str` of the arm it holds"
    );

    // [is-bind-once]
    /// A binding `is` on a subject that cannot be evaluated once.
    is_bind_once_not_hoistable = "is-bind-once:not-hoistable" (what: &dyn std::fmt::Display) => format!(
        "this `is` binds its subject, and the subject is not a variable or a field chain \
         — so it has to be evaluated exactly once, which is only possible when the `is` \
         is the whole condition of a `while` or of an `if`'s first branch (here it is \
         {what}). Bind the subject first: `let x = …` and then test `x`"
    );

    // [is-binding]
    /// An `is` binding whose check matches several types.
    is_binding_several_types = "is-binding:several-types" () =>
        "an `is` binding requires a check matching a single type";

    // [is-qualifies]
    /// An `is Q` whose qualifier does not apply to the subject.
    is_qualifies_does_not_apply = "is-qualifies:does-not-apply" (q: &dyn std::fmt::Display, subj_ty: &dyn std::fmt::Display) => format!(
        "qualifier `{q}` does not apply to `{subj_ty}`"
    );
    /// An `is Q T` whose subject can never be a `T`.
    is_qualifies_never_succeeds = "is-qualifies:never-succeeds" () =>
        "this check can never succeed";
    /// An `is` test of a qualifier with no runtime test.
    is_qualifies_unknown_qualifier = "is-qualifies:unknown-qualifier" () =>
        "this qualifier check cannot be performed at runtime (unknown qualifier)";

    // [is-qualifies-effects]
    /// A predicate qualifier whose effect has no handler in scope.
    is_qualifies_effects_no_handler = "is-qualifies-effects:no-handler" (q: &dyn std::fmt::Display, ename: &dyn std::fmt::Display) => format!(
        "predicate qualifier `{q}` requires effect `{ename}`, but no handler for it is in \
         scope (declare it in the function's effect list or `use` a handler)"
    );

    // [is-union]
    /// An `is` that matches no arm of its union subject.
    is_union_no_arm_matches = "is-union:no-arm-matches" () =>
        "this check can never succeed";

    // [iter-drive-in-place]
    /// A `for` that would consume a linear iterator.
    iter_drive_in_place_linear_consumed = "iter-drive-in-place:linear-consumed" (bare: &dyn std::fmt::Display, hint: &dyn std::fmt::Display) => format!(
        "a `for` cannot consume a linear iterator (`{bare}`): the loop never discharges \
         what it drives — bind the iterator with `let`, loop over it, then discharge it \
         with {hint}"
    );
    /// A `for` that would consume an iterator that may be linear.
    iter_drive_in_place_maybe_linear_consumed = "iter-drive-in-place:maybe-linear-consumed" (bare: &dyn std::fmt::Display) => format!(
        "a `for` cannot consume an iterator that may be linear (`{bare}` is `canbe \
         linear`): the loop never discharges what it drives — keep the iterator (drive \
         it in place and let the caller discharge it), or take a consuming callback \
         (`end: ({bare}) -> None` with `=>[end] !it`) and hand the iterator to it after \
         the loop"
    );

    // [iter-fn]
    /// A mutation of what an open `for` iterates.
    iter_fn_origin_mutated = "iter-fn:origin-mutated" (name: &dyn std::fmt::Display) => format!(
        "`{name}` is being iterated, so it cannot be mutated here: the pass reads its \
         origin as it goes, and what it would see has no answer both backends agree on. \
         Mutate it before or after the loop, or iterate `copy({name})` to work from a \
         snapshot"
    );

    // [iter-group]
    /// A type that is both a source and an iterator.
    iter_group_source_and_iterator = "iter-group:source-and-iterator" (name: &dyn std::fmt::Display, obligation: &dyn std::fmt::Display) => format!(
        "`{name}` declares both `: {obligation}` and `: Next`: a type is either a source \
         (`{obligation}` mints an iterator from it) or an iterator struct (`next` \
         advances it), not both — `for x in {name}` would have two answers"
    );

    // [iter-protocol]
    /// A `for` whose `iter(…)` result no `next` takes.
    iter_protocol_no_next = "iter-protocol:no-next" (subject: &dyn std::fmt::Display, call: &dyn std::fmt::Display, it: &dyn std::fmt::Display, state: &dyn std::fmt::Display) => format!(
        "`{subject}` is not iterable: `for` over it is `{call}`, and `iter({subject})` is \
         a `{it}`, which no `next` in scope takes — declare `next(it: Mut {state}) -> \
         Emitted T | Finished` beside it"
    );
    /// A `next` that does not return `Emitted T | Finished`.
    iter_protocol_next_return = "iter-protocol:next-return" (subject: &dyn std::fmt::Display, call: &dyn std::fmt::Display, ret: &dyn std::fmt::Display) => format!(
        "`for` over `{subject}` is `{call}`, which must return `Emitted T | Finished`, \
         and the `next` it resolves to returns `{ret}`"
    );
    /// A `next` that does not take its state as `Mut`.
    iter_protocol_next_not_mut = "iter-protocol:next-not-mut" (state: &dyn std::fmt::Display) => format!(
        "`next` has to take its state as `Mut {state}` — advancing an iterator mutates \
         its position"
    );

    // [iter-resolve]
    /// A `for` subject no `next` or `iter` takes.
    iter_resolve_not_iterable = "iter-resolve:not-iterable" (subject: &dyn std::fmt::Display, remedy: &dyn std::fmt::Display) => format!(
        "`{subject}` is not iterable: `for x in e` is `next(e)` when a `next` takes `e`, \
         and `next(iter(e))` otherwise, and no `next` or `iter` in scope takes \
         `{subject}`{remedy}"
    );

    // [iter-step-call]
    /// A step's result iterated instead of its call.
    iter_step_call_step_value = "iter-step-call:step-value" (iter_ty: &dyn std::fmt::Display) => format!(
        "`{iter_ty}` is a step's result, not something to iterate: `for` re-invokes a \
         step *call* each turn (`for x in next(p)`), so write the call in the loop \
         header rather than its value"
    );
    /// A step call in a `for` header with an argument that is not a place.
    iter_step_call_argument_not_place = "iter-step-call:argument-not-place" () =>
        "a step call in a `for` header is re-invoked every turn, so each argument has to \
         be a place (a variable or a field path) or a literal: bind this expression with \
         `let` first";
    /// A step call in a `for` header that consumes an argument.
    iter_step_call_consumes_argument = "iter-step-call:consumes-argument" (callee_name: &dyn std::fmt::Display, param: &dyn std::fmt::Display) => format!(
        "`{callee_name}` consumes its parameter `{param}`, so it cannot be re-invoked on \
         the same argument each turn: a step called from a `for` header keeps every \
         argument (`=> {param}: Mut`)"
    );

    // [iter-yield-proj]
    /// A `proj` obligation whose member returns an owned element.
    iter_yield_proj_owned_element = "iter-yield-proj:owned-element" (name: &dyn std::fmt::Display, obligation: &dyn std::fmt::Display, member: &dyn std::fmt::Display) => format!(
        "`{name}` declares `: {obligation}<self, proj …>`, but its `{member}` returns an \
         owned element: write `Emitted (proj(p) T) | Finished`, or drop the `proj` from \
         the obligation"
    );
    /// A member returning a borrow its obligation does not declare.
    iter_yield_proj_undeclared_borrow = "iter-yield-proj:undeclared-borrow" (name: &dyn std::fmt::Display, member: &dyn std::fmt::Display, obligation: &dyn std::fmt::Display) => format!(
        "`{name}`'s `{member}` returns a borrowed element (`proj`), so its obligation \
         must say so: `: {obligation}<self, proj T>`"
    );

    // [linear-composite]
    /// A linear type in a component that cannot hold it.
    linear_composite_position = "linear-composite:position" (linear: &dyn std::fmt::Display, reason: &dyn std::fmt::Display) => format!(
        "`{linear}` is linear, so it cannot go here: {reason}"
    );
    /// A linear field in a struct not declared `linear`.
    linear_composite_field = "linear-composite:field" (linear: &dyn std::fmt::Display, owner: &dyn std::fmt::Display, field: &dyn std::fmt::Display) => format!(
        "`{linear}` is linear, so field `{owner}.{field}` makes the container a resource \
         too: declare `linear struct {owner}` (and a same-file fn consuming it) to say so"
    );
    /// A linear value consumed out of a handler's storage.
    linear_composite_handler_storage = "linear-composite:handler-storage" (action: &dyn std::fmt::Display, name: &dyn std::fmt::Display, kind: &dyn std::fmt::Display, handler: &dyn std::fmt::Display) => format!(
        "cannot {action} `{name}`: it is a {kind} of handler `{handler}`, which owns it \
         for its whole lifetime — and a linear value cannot be copied out of it, so \
         there is nothing to hand over here. Discharge it where the handler itself is \
         settled"
    );

    // [linear-container]
    /// A linear type where nothing would carry its obligation.
    linear_container_position = "linear-container:position" (linear: &dyn std::fmt::Display, position: &dyn std::fmt::Display) => format!(
        "`{linear}` is linear, so it cannot be {position}: nothing here would carry its \
         obligation onward. A linear value lives in a local, a parameter, a return \
         value, a union arm, a `linear struct`'s field, or a container that opts in — a \
         `Mut List<{linear}>` or a `Mut Map<K, {linear}>`, whose terminal is `drain`"
    );

    // [linear-discard]
    /// `discard` of a linear value outside its type's own dischargers.
    linear_discard_linear = "linear-discard:linear" (arg_ty: &dyn std::fmt::Display, set: &dyn std::fmt::Display) => format!(
        "`discard` cannot drop a linear value (`{arg_ty}`) here: that is the leak the \
         obligation exists to prevent — discharge it with {set}, or `discard` it inside \
         one of those (a fn, or an effect member, declared in the type's own file and \
         consuming it — for a member, inside any handler's implementation of it)"
    );

    // [linear-generics]
    /// A function value instantiating an unopted generic with a linear type.
    linear_generics_fn_value = "linear-generics:fn-value" (generic: &dyn std::fmt::Display, name: &dyn std::fmt::Display, bound: &dyn std::fmt::Display) => format!(
        "cannot instantiate generic parameter `{generic}` of `{name}` with linear type \
         `{bound}`: `{name}` does not declare `<{generic} canbe linear>`, so it does not \
         honor the use obligation"
    );
    /// A type-parameter `with` clause naming something other than `linear`.
    linear_generics_with_clause = "linear-generics:with-clause" (qualifier: &dyn std::fmt::Display) => format!(
        "only `linear` is supported in a type-parameter `with` clause (found \
         `{qualifier}`)"
    );
    /// `copy` of a linear value.
    linear_generics_copy_linear = "linear-generics:copy-linear" (ty: &dyn std::fmt::Display) => format!(
        "cannot `copy` a value of linear type `{ty}`: that would duplicate its \
         obligation; every linear value has exactly one owner"
    );
    /// A generic instantiated with a linear type it does not opt into.
    linear_generics_not_opted = "linear-generics:not-opted" (var_name: &dyn std::fmt::Display, name: &dyn std::fmt::Display, ty: &dyn std::fmt::Display) => format!(
        "cannot instantiate generic parameter `{var_name}` of `{name}` with linear type \
         `{ty}`: `{name}` does not declare `<{var_name} canbe linear>`, so it does not \
         honor the use obligation"
    );
    /// A linear value in a variadic position.
    linear_generics_variadic = "linear-generics:variadic" () =>
        "a linear value cannot be passed in a variadic position (variadic arguments are \
         not tracked); add it to the collection individually instead";
    /// An effect member instantiated with a linear type it does not opt into.
    linear_generics_effect_member_not_opted = "linear-generics:effect-member-not-opted" (var_name: &dyn std::fmt::Display, member: &dyn std::fmt::Display, bound: &dyn std::fmt::Display) => format!(
        "cannot instantiate generic parameter `{var_name}` of effect member `{member}` \
         with linear type `{bound}`: the member does not declare `<{var_name} canbe \
         linear>`, so its handlers do not honor the use obligation"
    );

    // [linear-group]
    /// Linearity written as an obligation.
    linear_group_linear_obligation = "linear-group:linear-obligation" () =>
        "linearity is declared with the `linear struct` modifier, not as an obligation: \
         write `linear struct …` and supply a same-file fn that consumes the value";
    /// `canbe linear` on a type.
    linear_group_canbe_linear = "linear-group:canbe-linear" () =>
        "linearity is declared as an obligation, not granted with `canbe`: write `linear \
         struct` and supply a same-file discharger";
    /// `Linear` written in a type.
    linear_group_linear_in_type = "linear-group:linear-in-type" () =>
        "`Linear` cannot be written in a type: linearity is declared on the type itself \
         (`canbe linear`) and applies to every value of it";

    // [linear-lambda]
    /// A lambda that consumes a linear value.
    linear_lambda_consume = "linear-lambda:consume" (name: &dyn std::fmt::Display) => format!(
        "a lambda cannot consume `{name}`: it holds a linear value, and the closure would \
         inherit an obligation to be called exactly once (not supported yet); pass the \
         value explicitly instead"
    );
    /// A lambda that captures and mutates a linear value.
    linear_lambda_captures_linear = "linear-lambda:captures-linear" (name: &dyn std::fmt::Display) => format!(
        "this lambda captures and mutates `{name}`, which holds a linear value: the \
         closure would swallow its obligation; restructure so the value is passed \
         explicitly"
    );

    // [linear-obligation]
    /// A linear struct nothing in its file consumes.
    linear_obligation_no_discharger = "linear-obligation:no-discharger" (what: &dyn std::fmt::Display, name: &dyn std::fmt::Display) => format!(
        "{what} has no discharger: nothing in this file consumes a `{name}` — no fn, and \
         no member of an effect declared here — so the obligation has no legal death; \
         declare one (e.g. `fn close(x: {name}) -> None => !x {{ discard(x) }}`)"
    );
    /// A linear value dropped in statement position.
    linear_obligation_dropped_statement = "linear-obligation:dropped-statement" () =>
        "this expression produces a linear value that is dropped immediately; bind it, \
         move it onward, or pass it to `discard`";

    // [linear-opaque]
    /// A linear intrinsic type nothing in its file consumes.
    linear_opaque_no_discharger = "linear-opaque:no-discharger" (ty_name: &dyn std::fmt::Display) => format!(
        "linear intrinsic type `{ty_name}` has no discharger: nothing in this file \
         consumes a `{ty_name}` — no fn, and no member of an effect declared here — so \
         the obligation has no legal death; declare one (e.g. `intrinsic fn send<T>(r: \
         {ty_name}<T>, value: T) [] -> None => !r`)"
    );

    // [linear-state]
    /// A linear value held directly in a handler's state.
    linear_state_field = "linear-state:field" (linear: &dyn std::fmt::Display, owner: &dyn std::fmt::Display, field: &dyn std::fmt::Display) => format!(
        "`{linear}` is linear, so field `{owner}.{field}` would hold an obligation this \
         handler can never discharge: taking it back out would leave the state with a \
         hole, and nothing could be put back. Hold it in a container instead — a `Mut \
         List<{linear}>` or a `Mut Map<K, {linear}>`, whose `remove_first`/`remove` take \
         one element at a time and whose `drain` is the terminal"
    );

    // [mod-namespace]
    /// A leading-dot reference with no expected type to resolve against.
    mod_namespace_no_expected_type = "mod-namespace:no-expected-type" (written: &dyn std::fmt::Display) => format!(
        "`{written}` needs a known type to resolve against: there is no expected type \
         here, so write the namespace (`Ns{written}`)"
    );
    /// A leading-dot reference matching no arm of the expected union.
    mod_namespace_no_arm = "mod-namespace:no-arm" (written: &dyn std::fmt::Display, expected: &dyn std::fmt::Display, arms: &dyn std::fmt::Display) => format!(
        "`{written}` matches no arm of `{expected}` — its arms are {arms}"
    );
    /// A leading-dot reference several arms of the expected union match.
    mod_namespace_ambiguous_arms = "mod-namespace:ambiguous-arms" (written: &dyn std::fmt::Display, expected: &dyn std::fmt::Display, candidates: &dyn std::fmt::Display) => format!(
        "`{written}` is ambiguous across the arms of `{expected}`: {candidates} — write \
         more of the path"
    );
    /// A leading-dot reference naming no member of the expected type's namespace.
    mod_namespace_no_member = "mod-namespace:no-member" (written: &dyn std::fmt::Display, expected: &dyn std::fmt::Display) => format!(
        "`{written}` names no member in the namespace of `{expected}`"
    );
    /// A leading-dot reference several namespace members match.
    mod_namespace_ambiguous = "mod-namespace:ambiguous" (written: &dyn std::fmt::Display, candidates: &dyn std::fmt::Display) => format!(
        "`{written}` is ambiguous: {candidates} — write more of the path"
    );
    /// A leading-dot reference whose expected type exposes no namespace.
    mod_namespace_not_namespaced = "mod-namespace:not-namespaced" (written: &dyn std::fmt::Display, expected: &dyn std::fmt::Display) => format!(
        "`{written}` resolves against a namespaced type, but `{expected}` exposes none \
         here — write the namespace explicitly"
    );
    /// A leading-dot arm naming no member of its alias's namespace.
    mod_namespace_no_alias_member = "mod-namespace:no-alias-member" (written: &dyn std::fmt::Display, ns: &dyn std::fmt::Display, suffix: &dyn std::fmt::Display) => format!(
        "`{written}` names no member of `{ns}` — its declared members are what \
         `{ns}.{suffix}` would have to be"
    );

    // [mod-use]
    /// A module-level `use` outside the runtime module.
    mod_use_module_level = "mod-use:module-level" () =>
        "a `use` belongs in a function body: a module-level binding is the runtime \
         module's alone, since anywhere else it would be state every function reaches \
         without declaring it";

    // [name-bare]
    /// A state field named like the handler's constructor parameter.
    name_bare_state_param_clash = "name-bare:state-param-clash" (field: &dyn std::fmt::Display, handler: &dyn std::fmt::Display, param: &dyn std::fmt::Display) => format!(
        "state field `{field}` of handler `{handler}` has the name of its constructor \
         parameter `{param}`: a member reads both by bare name, so one must be renamed"
    );
    /// A member's parameter named like the handler's state or parameter.
    name_bare_param_clash = "name-bare:param-clash" (param: &dyn std::fmt::Display, fn_name: &dyn std::fmt::Display, handler: &dyn std::fmt::Display, what: &dyn std::fmt::Display) => format!(
        "parameter `{param}` of `{fn_name}` has the name of handler `{handler}`'s {what} \
         `{param}`, which the body reads by bare name: rename the parameter"
    );

    // [name-camel]
    /// Two names of one scope spelled alike in camel case.
    name_camel_clash = "name-camel:clash" (second: &str, first: &str, camel: &str, what: &str) => format!(
        "`{second}` and `{first}` are both `{camel}` in camel case, and {what} cannot \
         hold both: the Kotlin backend writes camel case — rename one"
    );

    // [name-ident-resolve]
    /// A name that is a declaration but not a value.
    name_ident_resolve_not_a_value = "name-ident-resolve:not-a-value" (name: &dyn std::fmt::Display, what: &dyn std::fmt::Display) => format!(
        "`{name}` is {what}"
    );
    /// A name that resolves to no variable or function.
    name_ident_resolve_unknown = "name-ident-resolve:unknown" (name: &dyn std::fmt::Display) => format!(
        "no variable or function named `{name}` is in scope"
    );

    // [name-resolve]
    /// An effect list entry naming no effect.
    name_resolve_unknown_effect = "name-resolve:unknown-effect" (name: &dyn std::fmt::Display) => format!(
        "unknown effect `{name}`"
    );
    /// An effect written with the wrong number of type arguments.
    name_resolve_effect_type_arg_count = "name-resolve:effect-type-arg-count" (name: &dyn std::fmt::Display, expected: &dyn std::fmt::Display, found: &dyn std::fmt::Display) => format!(
        "effect `{name}` expects {expected} type argument(s), found {found}"
    );
    /// A `use`/`spawn` naming no handler.
    name_resolve_unknown_handler = "name-resolve:unknown-handler" (handler: &dyn std::fmt::Display, form: &dyn std::fmt::Display) => format!(
        "unknown handler `{handler}` in `{form}`"
    );
    /// A type or qualifier name that resolves to nothing visible.
    name_resolve_unknown_type = "name-resolve:unknown-type" (what: &dyn std::fmt::Display, name: &dyn std::fmt::Display, hint: &dyn std::fmt::Display) => format!(
        "unknown {what} `{name}`{hint}"
    );
    /// An assignment to a variable that is not declared.
    name_resolve_assign_undeclared = "name-resolve:assign-undeclared" (name: &dyn std::fmt::Display) => format!(
        "assignment to undeclared variable `{name}`"
    );
    /// An `is` naming no type or qualifier.
    name_resolve_unknown_type_or_qualifier = "name-resolve:unknown-type-or-qualifier" (name: &dyn std::fmt::Display) => format!(
        "unknown type or qualifier `{name}`"
    );

    // [net-noremote]
    /// A wire-form argument of a type with none.
    net_noremote_no_wire_form = "net-noremote:no-wire-form" (fn_name: &dyn std::fmt::Display, subject: &dyn std::fmt::Display, why: &dyn std::fmt::Display) => format!(
        "`{fn_name}` needs a type with a wire form, and `{subject}` has none: {why}"
    );
    /// A group spanning nodes whose protocol has no wire form.
    net_noremote_group_across_nodes = "net-noremote:group-across-nodes" (effect: &dyn std::fmt::Display, member: &dyn std::fmt::Display, pty: &dyn std::fmt::Display, why: &dyn std::fmt::Display, via: &dyn std::fmt::Display) => format!(
        "a group of `{effect}` cannot span nodes: `{effect}.{member}` takes `{pty}`, and \
         {why} — so `{via}<{effect}>` has nothing to name on the wire. Keep the actor \
         local, or give the payload a wire form"
    );

    // [obligation-by]
    /// `by` on an obligation of an `intrinsic type`.
    obligation_by_intrinsic_type = "obligation-by:intrinsic-type" (by: &dyn std::fmt::Display, obligation: &dyn std::fmt::Display) => format!(
        "`by {by}` has no body to stamp on an `intrinsic type`: its implementation is a \
         hand-written `intrinsic fn`, so declare `{obligation}` without `by` and write \
         the `intrinsic fn`(s) it names"
    );
    /// `by` on an obligation whose argument is not `self`.
    obligation_by_not_self = "obligation-by:not-self" (obligation: &dyn std::fmt::Display, by: &dyn std::fmt::Display) => format!(
        "`by` stamps the implementation for the type it is written on, so its argument is \
         `self`: write `: {obligation}<self> by {by}`"
    );
    /// The removed `canbe hashed` / `canbe ordered`.
    obligation_by_canbe_hashed = "obligation-by:canbe-hashed" (qualifier: &dyn std::fmt::Display, group: &dyn std::fmt::Display, member: &dyn std::fmt::Display) => format!(
        "`canbe {qualifier}` no longer exists: being hashable or orderable is *having the \
         function*, so declare it — `: {group}<self> by auto` on the type for the \
         structural one, or a `{member}` of your own"
    );

    // [op-arith]
    /// An arithmetic operator on operands that are not numbers.
    op_arith_non_numeric = "op-arith:non-numeric" (sym: &dyn std::fmt::Display, l: &dyn std::fmt::Display, r: &dyn std::fmt::Display) => format!(
        "`{sym}` needs numeric operands (`Int`, `Long`, `Float`, `Double`); found `{l}` \
         and `{r}`"
    );
    /// Unary `-` on an operand that is not a number.
    op_arith_unary_minus = "op-arith:unary-minus" (t: &dyn std::fmt::Display) => format!(
        "unary `-` needs a numeric operand (`Int`, `Long`, `Float`, `Double`); found `{t}`"
    );

    // [op-assign]
    /// An assignment to something that is not a place.
    op_assign_not_a_place = "op-assign:not-a-place" () =>
        "this is not a place, so it cannot be assigned to: an assignment writes a \
         variable, a field path (`p.name`) or a subscript (`xs[i]`). A call answers a \
         value, and writing to it would write to nothing";

    // [op-bool]
    /// A logical operator on an operand that is not a `Bool`.
    op_bool_not_bool = "op-bool:not-bool" (sym: &dyn std::fmt::Display, ty: &dyn std::fmt::Display) => format!(
        "`{sym}` needs `Bool` operands (found `{ty}`): Salvo has no truthiness — compare \
         explicitly (`n != 0`) or test the type with `is`"
    );

    // [op-elvis]
    /// `?:` on a value with one arm.
    op_elvis_single_arm = "op-elvis:single-arm" (subj_ty: &dyn std::fmt::Display) => format!(
        "`?:` on `{subj_ty}` keeps its only arm and leaves nothing for the right-hand \
         side: `?:` picks the first arm of a union with more than one (the same dead \
         scaffolding a throw-free `try` is refused for)"
    );

    // [op-no-none]
    /// `None` as an operand.
    op_no_none_none_operand = "op-no-none:none-operand" (symbol: &dyn std::fmt::Display) => format!(
        "`None` is not a valid operand of `{symbol}`"
    );
    /// An operand that may be `None`.
    op_no_none_optional_operand = "op-no-none:optional-operand" (symbol: &dyn std::fmt::Display, ty: &dyn std::fmt::Display) => format!(
        "operand of `{symbol}` may be `None` (its type is `{ty}`): narrow it first (`is` \
         / `when`) or assert it with `!`"
    );

    // [op-order]
    /// A comparison's function that returns the wrong type.
    op_order_return_type = "op-order:return-type" (sym: &dyn std::fmt::Display, lt: &dyn std::fmt::Display, member: &dyn std::fmt::Display, rt: &dyn std::fmt::Display, want: &dyn std::fmt::Display, got: &dyn std::fmt::Display) => format!(
        "`{sym}` on `{lt}` is `{member}({lt}, {rt})`, which must return `{want}`, and the \
         `{member}` it resolves to returns `{got}`"
    );

    // [op-promote]
    /// An operator on an integer and a floating-point operand.
    op_promote_mixed_classes = "op-promote:mixed-classes" (sym: &dyn std::fmt::Display, lw: &dyn std::fmt::Display, rw: &dyn std::fmt::Display) => format!(
        "`{sym}` cannot mix `{lw}` and `{rw}`: integer and floating-point operands need \
         an explicit conversion (`to_double`, `to_long`, …)"
    );

    // [op-safe-nav]
    /// `?.` on a receiver with one arm.
    op_safe_nav_single_arm = "op-safe-nav:single-arm" (base_ty: &dyn std::fmt::Display) => format!(
        "`?.` guards the first arm of a union, but this receiver is `{base_ty}` — it has \
         only one arm, so write a plain `.`"
    );
    /// A `?.` call naming no function in scope.
    op_safe_nav_unknown_fn = "op-safe-nav:unknown-fn" (name: &dyn std::fmt::Display) => format!(
        "no function named `{name}` is in scope: `?.` calls a function with the receiver \
         as its first argument"
    );
    /// A `?.` method call whose receiver is not a place.
    op_safe_nav_receiver_not_place = "op-safe-nav:receiver-not-place" () =>
        "`?.` on a method call needs a variable or field as its receiver; bind the chain \
         so far to a local first";

    // [platform-check]
    /// A `platform fn` returning a `proj` borrow.
    platform_check_borrowed_return = "platform-check:borrowed-return" (fn_name: &dyn std::fmt::Display) => format!(
        "`platform fn {fn_name}` returns a borrow (`proj`), which host code cannot hand \
         back: return an owned value"
    );
    /// A host value checked element by element (a warning).
    platform_check_element_cost = "platform-check:element-cost" (what: &dyn std::fmt::Display, ty: &dyn std::fmt::Display) => format!(
        "{what} is checked element by element when the host returns it, a cost that grows \
         with its size ({ty})"
    );
    /// A host value's claim of a qualifier the boundary cannot check.
    platform_check_unchecked_claim = "platform-check:unchecked-claim" (what: &dyn std::fmt::Display, qualifier: &dyn std::fmt::Display) => format!(
        "{what} claims `{qualifier}`, whose `qualifies` needs effects or slot values the \
         boundary does not have, so the claim cannot be checked when the host returns it"
    );
    /// A host value's claim of another module's constructive qualifier.
    platform_check_foreign_constructive = "platform-check:foreign-constructive" (what: &dyn std::fmt::Display, qualifier: &dyn std::fmt::Display) => format!(
        "{what} claims `{qualifier}`, a constructive qualifier declared in another \
         module: nothing proves it of a value the host returns — declare the platform \
         declaration in that module, or drop the claim"
    );
    /// A host value keyed by a Salvo-defined identity.
    platform_check_custom_key = "platform-check:custom-key" (what: &dyn std::fmt::Display, ty: &dyn std::fmt::Display) => format!(
        "{what} is `{ty}`, a collection keyed by a Salvo-defined identity, which host \
         code cannot build: key it by the canonical identity, or return a `List`"
    );
    /// A host value of a recursive type with checked fields.
    platform_check_recursive_type = "platform-check:recursive-type" (what: &dyn std::fmt::Display, name: &dyn std::fmt::Display) => format!(
        "{what} contains `{name}`, a recursive type with fields the boundary must check, \
         which is not supported yet"
    );

    // [platform-fn]
    /// A `platform fn` with implicits or effects.
    platform_fn_not_allowed = "platform-fn:not-allowed" (fn_name: &dyn std::fmt::Display, why: &dyn std::fmt::Display) => format!(
        "`platform fn {fn_name}` {why}"
    );
    /// Two platform fns of one name.
    platform_fn_overloaded = "platform-fn:overloaded" (fn_name: &dyn std::fmt::Display) => format!(
        "`platform fn {fn_name}` is declared twice: two platform fns may not overload \
         each other — give one another name"
    );

    // [platform-fn-value]
    /// A lambda or local where a platform fn keeps its callback.
    platform_fn_value_kept_callback = "platform-fn-value:kept-callback" (fn_name: &dyn std::fmt::Display, param: &dyn std::fmt::Display) => format!(
        "`{fn_name}` keeps its `{param}` callback past the call, so it takes a named \
         top-level fn, not a lambda or a local: write the callback as a fn and pass it \
         by name"
    );

    // [platform-handler]
    /// A platform handler member with a body.
    platform_handler_body = "platform-handler:body" (handler: &dyn std::fmt::Display) => format!(
        "`platform handler {handler}` has no body in Salvo: the host implements its \
         members, in the module's implementation file under the platform root — or drop \
         `platform` to handle the effect here"
    );
    /// A platform handler with Salvo state.
    platform_handler_state = "platform-handler:state" (handler: &dyn std::fmt::Display) => format!(
        "`platform handler {handler}` declares Salvo state, but its implementation class \
         owns its fields: keep them there, and pass what they need as the handler's \
         parameters"
    );
    /// A platform handler with effect dependencies.
    platform_handler_dependencies = "platform-handler:dependencies" (handler: &dyn std::fmt::Display) => format!(
        "`platform handler {handler}` may not declare effect dependencies: its members \
         are host code, which performs no Salvo effect — write a Salvo handler that \
         depends on this one's effect if you need one in between"
    );
    /// A generic platform handler.
    platform_handler_generic = "platform-handler:generic" (handler: &dyn std::fmt::Display) => format!(
        "`platform handler {handler}` may not be generic: the host implements one \
         concrete class, and an instance per type argument is not expressible on every \
         backend"
    );

    // [platform-intrinsic-std-only]
    /// An `intrinsic` declaration outside std.
    platform_intrinsic_std_only_outside_std = "platform-intrinsic-std-only:outside-std" (kind: &dyn std::fmt::Display, name: &dyn std::fmt::Display) => format!(
        "`intrinsic {kind} {name}` is the compiler's to declare, not yours: every backend \
         lowers intrinsics from a table keyed by name, so one declared here has no \
         implementation anywhere. To reach the target language, declare an effect for \
         what you need and implement it with a `platform handler`"
    );

    // [platform-iterable]
    /// An iterable platform type without `: Iter`.
    platform_iterable_no_iter = "platform-iterable:no-iter" (name: &dyn std::fmt::Display) => format!(
        "`iterable platform type {name}` must also declare `: Iter<self, It, T>`: the \
         host's loop and Salvo's pass are two ways through one sequence, and the clause \
         is what makes them agree on the element type"
    );

    // [platform-slots]
    /// A linear platform type with no slots.
    platform_slots_no_slots = "platform-slots:no-slots" (name: &dyn std::fmt::Display) => format!(
        "`linear platform type {name}` declares no slots"
    );

    // [platform-type]
    /// A platform type with an obligation clause other than `Iter`.
    platform_type_obligation = "platform-type:obligation" (name: &dyn std::fmt::Display) => format!(
        "`platform type {name}` cannot take this obligation clause: it has no fields to \
         stamp over, and its comparisons are `platform fn`s; `: Iter<self, It, T>`, \
         without `by`, is the one it may declare"
    );
    /// A platform type both `threadsafe` and `linear`.
    platform_type_threadsafe_linear = "platform-type:threadsafe-linear" (name: &dyn std::fmt::Display) => format!(
        "`platform type {name}` cannot be both `threadsafe` and `linear`: a linear value \
         has one owner, so there is no sharing to make safe"
    );

    // [platform-value-type]
    /// A platform type both `threadsafe` and `canbe Mut`.
    platform_value_type_threadsafe_mut = "platform-value-type:threadsafe-mut" (name: &dyn std::fmt::Display) => format!(
        "`platform type {name}` cannot be both `threadsafe` and `canbe Mut`: a value type \
         is copied, never shared"
    );

    // [proj-anywhere]
    /// A view of a temporary, bound or returned.
    proj_anywhere_temporary = "proj-anywhere:temporary" (what: &dyn std::fmt::Display, name: &dyn std::fmt::Display) => format!(
        "cannot {what} a view of a temporary: `{name}` borrows an argument that dies at \
         the end of this statement; bind that argument with `let` first, so the view has \
         something to borrow from"
    );

    // [proj-field]
    /// A `proj` field that names a source.
    proj_field_source_named = "proj-field:source-named" (owner: &dyn std::fmt::Display, field: &dyn std::fmt::Display) => format!(
        "field `{owner}.{field}`: a `proj` field names no source — the value stored in it \
         at each literal decides what it projects; write `proj` alone"
    );

    // [proj-infer]
    /// A parameter both consumed and projected by the result.
    proj_infer_consumed_and_projected = "proj-infer:consumed-and-projected" (param: &dyn std::fmt::Display) => format!(
        "`{param}` cannot be both consumed (`!{param}`) and projected by the result: a \
         borrow needs the caller to keep the value"
    );
    /// A `proj` element naming no source.
    proj_infer_element_no_source = "proj-infer:element-no-source" () =>
        "a `proj` element names no source: the container holds the borrow, and which \
         parameter it is of is inferred from the body — or written after the type, `T \
         holds proj(p)`";
    /// A `proj` return type naming no source.
    proj_infer_return_no_source = "proj-infer:return-no-source" () =>
        "`proj` in a return type must name its source: `proj(param)`";
    /// A `proj(…)` or `ref(…)` naming no parameter.
    proj_infer_not_a_param = "proj-infer:not-a-param" (kind: &dyn std::fmt::Display, source: &dyn std::fmt::Display) => format!(
        "`{kind}({source})` names no parameter of this function"
    );
    /// A `proj(p)` return whose source is moved.
    proj_infer_moved_source = "proj-infer:moved-source" (source: &dyn std::fmt::Display) => format!(
        "`proj({source})` requires `{source}` to be kept: a moved parameter is owned by \
         this function, so its data is returned by ordinary moves"
    );
    /// Written `proj` entries that disagree with the body.
    proj_infer_lends_disagree = "proj-infer:lends-disagree" (declared: &dyn std::fmt::Display, inferred: &dyn std::fmt::Display) => format!(
        "the deduction list says the result projects {declared}, but the body returns a \
         value that projects {inferred}: every lend the body performs must be written \
         (or omit the `proj` entries and let them be inferred)"
    );
    /// A returned value holding a borrow of a local.
    proj_infer_local_borrow = "proj-infer:local-borrow" (root: &dyn std::fmt::Display) => format!(
        "cannot return this value: it holds a borrow of `{root}`, a local that dies with \
         this call; return a `copy`, or borrow from a parameter instead"
    );
    /// A returned value holding a borrow the signature does not lend.
    proj_infer_unlent_borrow = "proj-infer:unlent-borrow" (root: &dyn std::fmt::Display) => format!(
        "cannot return this value: it holds a borrow of `{root}`, which this function's \
         signature does not lend; declare it with `[{root}: proj]`, or keep `{root}` so \
         the borrow can be inferred"
    );
    /// `proj` as a type argument of a struct.
    proj_infer_struct_type_arg = "proj-infer:struct-type-arg" (name: &dyn std::fmt::Display) => format!(
        "`proj` cannot be a type argument of `{name}`: a struct's fields own their \
         values, so it cannot hold a borrow through a generic parameter (only the \
         intrinsic containers — `List`, arrays — may be views)"
    );

    // [proj-infer-fn-type]
    /// A `holds proj(…)` the fn type already infers (a warning).
    proj_infer_fn_type_redundant_holds = "proj-infer-fn-type:redundant-holds" (names: &dyn std::fmt::Display) => format!(
        "`holds proj({names})` here says what the signature already infers: a fn type \
         whose return is a bare type parameter lends its named kept positions, because \
         the result's real type is the caller's to choose and only a lend makes the \
         borrow it may hold nameable. Drop the clause — it is still needed where the \
         return is a concrete type"
    );

    // [proj-local]
    /// An owned value assigned to a variable that holds a borrow.
    proj_local_owned_into_borrow = "proj-local:owned-into-borrow" (name: &dyn std::fmt::Display, target_ty: &dyn std::fmt::Display, value_ty: &dyn std::fmt::Display) => format!(
        "`{name}` holds a borrow (`{target_ty}`), and this is an owned `{value_ty}`: a \
         variable is a borrow or an owner for its whole life. Make it the owner where it \
         is declared, `let {name} = copy(…)`"
    );

    // [proj-readonly]
    /// A parameter written `proj Mut`.
    proj_readonly_proj_mut_param = "proj-readonly:proj-mut-param" (param: &dyn std::fmt::Display) => format!(
        "parameter `{param}` cannot be written `proj Mut`: a `proj` value can only be \
         read, whatever its `Mut` says, so no projection could ever be passed here and \
         the body could never mutate it. Drop the `Mut` (a `proj` position accepts `proj \
         Mut` arguments), or drop the `proj` to mutate an owned value in place"
    );

    // [proj-readonly-return]
    /// A returned value not derived from the projected source.
    proj_readonly_return_not_derived = "proj-readonly-return:not-derived" (shown: &dyn std::fmt::Display) => format!(
        "this function returns `proj({shown})`, so every returned value must be derived \
         from `{shown}` (a projection, element, or alias) or be `None`"
    );

    // [proj-type]
    /// A projection passed where the callee consumes.
    proj_type_projection_consumed = "proj-type:projection-consumed" (what: &dyn std::fmt::Display, arg_shown: &dyn std::fmt::Display, callee: &dyn std::fmt::Display, pname: &dyn std::fmt::Display) => format!(
        "{what} is a projection (`{arg_shown}`), and `{callee}` consumes `{pname}`: a \
         borrowed value cannot be given away. Pass `copy(...)`, or keep the parameter \
         (`=> {pname}`)"
    );
    /// A borrowed value passed where the callee expects an owned one.
    proj_type_nested_borrow = "proj-type:nested-borrow" (what: &dyn std::fmt::Display, arg_shown: &dyn std::fmt::Display, callee: &dyn std::fmt::Display, pname: &dyn std::fmt::Display) => format!(
        "{what} holds a borrowed value (`{arg_shown}`) where `{callee}` expects an owned \
         one for `{pname}`: the two are the same on the JVM and different in Rust. Write \
         the parameter's type with the `proj` (as the argument has it), or pass \
         `copy(...)`"
    );

    // [qual-canbe-optin]
    /// A `canbe` of something that cannot be opted into.
    qual_canbe_optin_unknown = "qual-canbe-optin:unknown" (qualifier: &dyn std::fmt::Display) => format!(
        "only `Mut`, `once`, `hashed` and `ordered` can be opted into with `canbe` (found \
         `{qualifier}`)"
    );

    // [qual-const]
    /// A constant that does not fit its qualifier slot.
    qual_const_slot_mismatch = "qual-const:slot-mismatch" (c: &dyn std::fmt::Display, slot: &dyn std::fmt::Display, qualifier: &dyn std::fmt::Display, want: &dyn std::fmt::Display, found: &dyn std::fmt::Display) => format!(
        "`{c}` fills the `{slot}` slot of `{qualifier}`, which needs `{want}`, not \
         `{found}`"
    );

    // [qual-constructive]
    /// An `is` test of a constructive qualifier on a non-union value.
    qual_constructive_is_test = "qual-constructive:is-test" (q: &dyn std::fmt::Display) => format!(
        "`{q}` is a constructive qualifier; values only gain it from constructor \
         functions, so it cannot be tested with `is` on a non-union value"
    );

    // [qual-ctor-fn]
    /// A constructor return type naming no qualifier.
    qual_ctor_fn_unknown_qualifier = "qual-ctor-fn:unknown-qualifier" (name: &dyn std::fmt::Display) => format!(
        "unknown qualifier `{name}` in constructor return type"
    );
    /// A qualifier constructor without a return type.
    qual_ctor_fn_no_return = "qual-ctor-fn:no-return" () =>
        "a qualifier constructor must declare a return type";

    // [qual-ctor-same-file]
    /// A qualifier constructor outside the qualifier's file.
    qual_ctor_same_file_other_file = "qual-ctor-same-file:other-file" (name: &dyn std::fmt::Display) => format!(
        "constructor functions for `{name}` must be declared in the same file as the \
         qualifier"
    );

    // [qual-ctor-simple]
    /// A qualifier constructor returning a union.
    qual_ctor_simple_union = "qual-ctor-simple:union" () =>
        "a qualifier constructor must return a simple type, not a union";
    /// A qualifier constructor returning a tuple.
    qual_ctor_simple_tuple = "qual-ctor-simple:tuple" () =>
        "a qualifier constructor must return a simple type, not a tuple";
    /// A qualifier constructor whose return type the qualifier does not apply to.
    qual_ctor_simple_does_not_apply = "qual-ctor-simple:does-not-apply" (name: &dyn std::fmt::Display, base: &dyn std::fmt::Display) => format!(
        "qualifier `{name}` does not apply to `{base}`"
    );

    // [qual-depend]
    /// A qualifier with both fn slots and value slots.
    qual_depend_mixed_slots = "qual-depend:mixed-slots" () =>
        "a qualifier declares fn slots or value slots, not both";
    /// A dependent `qualifies` without one parameter per value slot.
    qual_depend_param_count = "qual-depend:param-count" (count: &dyn std::fmt::Display) => format!(
        "`qualifies` must take the candidate value and then one parameter per value slot \
         ({count} more)"
    );
    /// A dependent `qualifies` parameter that does not accept its slot's type.
    qual_depend_slot_param_type = "qual-depend:slot-param-type" (slot: &dyn std::fmt::Display, want: &dyn std::fmt::Display) => format!(
        "this parameter fills the `{slot}` slot, so it must accept `{want}`"
    );
    /// Value arguments to a qualifier that takes none.
    qual_depend_no_value_args = "qual-depend:no-value-args" (q: &dyn std::fmt::Display) => format!(
        "`{q}` takes no value arguments"
    );
    /// A dependent qualifier tested without its value arguments.
    qual_depend_unfilled = "qual-depend:unfilled" (q: &dyn std::fmt::Display, count: &dyn std::fmt::Display, filled: &dyn std::fmt::Display) => format!(
        "`{q}` depends on {count} value(s) ({filled}): write the filled form, \
         `{q}({filled})`"
    );
    /// A dependent qualifier's argument that is not a value in scope.
    qual_depend_not_a_value = "qual-depend:not-a-value" (path: &dyn std::fmt::Display) => format!(
        "`{path}` is not a value in scope"
    );
    /// A dependent qualifier's argument that does not fit its slot.
    qual_depend_slot_mismatch = "qual-depend:slot-mismatch" (path: &dyn std::fmt::Display, slot: &dyn std::fmt::Display, qualifier: &dyn std::fmt::Display, want: &dyn std::fmt::Display, place_ty: &dyn std::fmt::Display) => format!(
        "`{path}` fills the `{slot}` slot of `{qualifier}`, which needs `{want}`, not \
         `{place_ty}`"
    );
    /// A dependent qualifier's argument that is not a place.
    qual_depend_arg_not_place = "qual-depend:arg-not-place" () =>
        "a dependent qualifier's argument is a place: a variable or a field chain out of \
         one";

    // [qual-field-override]
    /// A field override that is not a subtype of the field.
    qual_field_override_not_subtype = "qual-field-override:not-subtype" (field: &dyn std::fmt::Display, override_ty: &dyn std::fmt::Display, orig: &dyn std::fmt::Display) => format!(
        "field override `{field}: {override_ty}` is not a subtype of the declared `{orig}`"
    );
    /// A field override naming no field of the struct.
    qual_field_override_no_field = "qual-field-override:no-field" (name: &dyn std::fmt::Display, field: &dyn std::fmt::Display) => format!(
        "struct `{name}` has no field `{field}`"
    );

    // [qual-lift]
    /// A `^` whose right side is not qualifier names.
    qual_lift_not_qualifiers = "qual-lift:not-qualifiers" () =>
        "`^` removes qualifiers, so its right side is qualifier names only (use `is` to \
         check a type)";
    /// A `^` naming no qualifier.
    qual_lift_nothing_named = "qual-lift:nothing-named" () =>
        "`^` needs a qualifier to remove";
    /// A `^` of a qualifier that cannot be removed.
    qual_lift_cannot_remove = "qual-lift:cannot-remove" (q: &dyn std::fmt::Display, reason: &dyn std::fmt::Display) => format!(
        "`{q}` cannot be removed with `^`: {reason}"
    );
    /// An `is ^Q` matching several arms of the subject.
    qual_lift_several_arms = "qual-lift:several-arms" (quals: &dyn std::fmt::Display, subj_ty: &dyn std::fmt::Display) => format!(
        "`is ^{quals}` matches more than one arm of `{subj_ty}`: lifting re-reads the \
         subject, which cannot stand for several arms at once — bind the lifted value \
         instead (`is ^{quals} name`)"
    );
    /// An `is ^Q` whose subject has no arm carrying `Q`.
    qual_lift_no_arm_carries = "qual-lift:no-arm-carries" (subj_ty: &dyn std::fmt::Display, quals: &dyn std::fmt::Display) => format!(
        "no arm of `{subj_ty}` carries `{quals}`, so there is nothing for `^` to remove"
    );
    /// A `^Q` of a value that does not carry `Q`.
    qual_lift_not_carried = "qual-lift:not-carried" (other: &dyn std::fmt::Display, quals: &dyn std::fmt::Display) => format!(
        "`{other}` does not carry `{quals}`, so there is nothing for `^` to remove"
    );
    /// A `^` branch whose head is not qualifier names.
    qual_lift_branch_not_qualifiers = "qual-lift:branch-not-qualifiers" () =>
        "a `^` branch removes qualifiers, so its head is qualifier names only (use `is` \
         to match a type)";
    /// A `^` branch whose arm does not carry the qualifier.
    qual_lift_arm_not_carried = "qual-lift:arm-not-carried" (quals: &dyn std::fmt::Display) => format!(
        "this arm does not carry `{quals}`, so there is nothing for `^` to remove"
    );

    // [qual-no-dup]
    /// A qualifier applied twice in one type.
    qual_no_dup_applied_twice = "qual-no-dup:applied-twice" (qualifier: &dyn std::fmt::Display) => format!(
        "qualifier `{qualifier}` is applied more than once"
    );

    // [qual-of]
    /// A qualifier whose `of` type does not fit.
    qual_of_does_not_apply = "qual-of:does-not-apply" (qualifier: &dyn std::fmt::Display, base: &dyn std::fmt::Display) => format!(
        "qualifier `{qualifier}` does not apply to `{base}`"
    );
    /// A qualifier of a union type.
    qual_of_union_subject = "qual-of:union-subject" () =>
        "a qualifier cannot apply to a union type; qualify the arms instead";
    /// A qualifier of a tuple type.
    qual_of_tuple_subject = "qual-of:tuple-subject" () =>
        "a qualifier cannot apply to a tuple type; qualify the parts instead";

    // [qual-predicate]
    /// A `qualifies` that does not take exactly one parameter.
    qual_predicate_param_count = "qual-predicate:param-count" () =>
        "`qualifies` must take exactly one parameter (the candidate value)";
    /// A `qualifies` whose parameter does not accept the `of` type.
    qual_predicate_param_type = "qual-predicate:param-type" (of_ty: &dyn std::fmt::Display) => format!(
        "`qualifies` must accept a `{of_ty}` parameter"
    );
    /// A `qualifies` that does not return `Bool`.
    qual_predicate_return_type = "qual-predicate:return-type" () =>
        "`qualifies` must return `Bool`";
    /// A `qualifies` with a deduction clause.
    qual_predicate_deductions = "qual-predicate:deductions" () =>
        "`qualifies` does not support deductions (it never moves the value)";

    // [qual-preserve]
    /// A `preserve` entry naming no parameter.
    qual_preserve_not_a_param = "qual-preserve:not-a-param" (name: &dyn std::fmt::Display) => format!(
        "`{name}` names no parameter"
    );
    /// A call that may invalidate a claim the fn promises to preserve.
    qual_preserve_call_invalidates = "qual-preserve:call-invalidates" (q: &dyn std::fmt::Display, name: &dyn std::fmt::Display) => format!(
        "this call may invalidate `{q}` claims about `{name}`, which the enclosing \
         function promises to preserve: only calls that themselves preserve `{q}` may \
         take `{name}` as `Mut`"
    );

    // [qual-refn]
    /// `preserve` on a bodiless declaration.
    qual_refn_bodiless_preserve = "qual-refn:bodiless-preserve" () =>
        "a bodiless declaration cannot promise `preserve`: there is nothing to check it \
         against — the qualifier's owner writes a `refn` instead";

    // [qual-refn-scope]
    /// A qualifier body with members that need a `qualifies`.
    qual_refn_scope_no_qualifies = "qual-refn-scope:no-qualifies" (qualifier: &dyn std::fmt::Display) => format!(
        "qualifier `{qualifier}` has a body but no `qualifies`: a body may hold \
         a                      `qualifies` (which makes the claim testable), \
         refinements, or                      both — this one holds members that need a \
         `qualifies` beside                      them"
    );

    // [qual-subject]
    /// A provenance qualifier with a body.
    qual_subject_provenance_body = "qual-subject:provenance-body" (qualifier: &dyn std::fmt::Display) => format!(
        "provenance qualifier `{qualifier}` cannot have a body: provenance is not \
         testable at runtime (no `qualifies`) and describes the handle, not its contents \
         (no field overrides) — values gain it from constructor functions"
    );

    // [qual-value-arg]
    /// A partial list of type arguments.
    qual_value_arg_partial_type_args = "qual-value-arg:partial-type-args" (type_arity: &dyn std::fmt::Display) => format!(
        "type arguments are all-or-none: this declaration has {type_arity} type \
         parameter(s), so write all of them or none (value arguments go in the `(…)` \
         block)"
    );
    /// A type written with the wrong number of type arguments.
    qual_value_arg_type_arg_count = "qual-value-arg:type-arg-count" (name: &dyn std::fmt::Display, expected: &dyn std::fmt::Display, found: &dyn std::fmt::Display) => format!(
        "`{name}` takes {expected} type argument(s), found {found}: write all of them \
         (`{name}<...>`) — unlike a struct literal's fields, a type position like this \
         has no values to infer a missing one from"
    );

    // [qual-whynot]
    /// A `whynot?` that binds a name.
    qual_whynot_binding = "qual-whynot:binding" () =>
        "`whynot?` answers a message, and binds nothing: drop the name";
    /// A `whynot?` of a qualifier that tests a union's arm.
    qual_whynot_union_test = "qual-whynot:union-test" () =>
        "`whynot?` explains a predicate qualifier's test, and this one tests which arm a \
         union holds: write `x is T` as a condition";
    /// An `@whynot` anywhere but on a qualifier's `fn qualifies` (a warning).
    qual_whynot_misplaced_tag = "qual-whynot:misplaced-tag" () =>
        "`@whynot` says why a value fails a predicate qualifier: it belongs on the \
         qualifier's `fn qualifies`";
    /// A `whynot` fn in a qualifier.
    qual_whynot_on_fn = "qual-whynot:on-fn" (qualifier: &dyn std::fmt::Display) => format!(
        "a qualifier says why a value fails it in a doc tag, not a fn: write `// @whynot \
         …` above `{qualifier}.qualifies`, with `[name]` for its parameters"
    );
    /// An `@whynot` slot whose parameter type cannot be shown.
    qual_whynot_slot_type = "qual-whynot:slot-type" (name: &dyn std::fmt::Display, ty: &dyn std::fmt::Display) => format!(
        "`@whynot` shows `[{name}]`, a `{ty}`: a slot shown in the message is an `Int`, \
         `Long`, `Double`, `Bool`, `Char` or `Str`"
    );
    /// An `@whynot` slot naming no parameter of `qualifies`.
    qual_whynot_unknown_slot = "qual-whynot:unknown-slot" (name: &dyn std::fmt::Display, qualifier: &dyn std::fmt::Display, names: &dyn std::fmt::Display) => format!(
        "`@whynot` shows `[{name}]`, which is not a parameter of `{qualifier}.qualifies`: \
         write one of {names} (`[[` is a literal `[`)"
    );
    /// A direct call of a `qualifies` that takes implicits.
    qual_whynot_call_implicits = "qual-whynot:call-implicits" (qualifier: &dyn std::fmt::Display, member: &dyn std::fmt::Display) => format!(
        "`{qualifier}.{member}` takes implicit parameters, which a direct call does not \
         fill: test with `is`"
    );
    /// A direct call of a `qualifies` with the wrong number of arguments.
    qual_whynot_call_arg_count = "qual-whynot:call-arg-count" (qualifier: &dyn std::fmt::Display, member: &dyn std::fmt::Display, expected: &dyn std::fmt::Display, found: &dyn std::fmt::Display) => format!(
        "`{qualifier}.{member}` takes {expected} argument(s) — the subject, then one per \
         slot — found {found}"
    );
    /// A direct call of a `qualifies` with an argument of the wrong type.
    qual_whynot_call_arg_mismatch = "qual-whynot:call-arg-mismatch" (qualifier: &dyn std::fmt::Display, member: &dyn std::fmt::Display, want: &dyn std::fmt::Display, got: &dyn std::fmt::Display) => format!(
        "`{qualifier}.{member}` expects `{want}`, found `{got}`"
    );

    // [ref-anchor]
    /// A `ref(…)` naming no other parameter.
    ref_anchor_not_a_param = "ref-anchor:not-a-param" (source: &dyn std::fmt::Display) => format!(
        "`ref({source})` names no other parameter of this function: a handle parameter \
         names the container parameter it is a position in"
    );
    /// An argument for a `ref(…)` parameter that is not a handle into its container.
    ref_anchor_not_a_handle = "ref-anchor:not-a-handle" () =>
        "this argument must be a handle into the container passed for its `ref(…)` \
         parameter — the callee reads and writes it at a position of that container; \
         mint it from that container with `at(…)`";

    // [ref-handle]
    /// A platform handler of a member returning a `ref` handle.
    ref_handle_platform_handler = "ref-handle:platform-handler" (handler: &dyn std::fmt::Display, member: &dyn std::fmt::Display) => format!(
        "`platform handler {handler}` cannot implement `{member}`, whose result is a \
         `ref` handle: a handle is a path into the caller's container, which host code \
         does not answer — return the element's value, or handle the effect in Salvo"
    );
    /// A plain projection where a function value mutates.
    ref_handle_plain_projection = "ref-handle:plain-projection" (t: &dyn std::fmt::Display) => format!(
        "this argument is a plain projection (`{t}`), and the function value mutates it: \
         a mutable element handle is written `ref(c)`, not a bare `proj` — `proj Mut` is \
         read-only. Mint a handle with `at(...)` instead of projecting, or pass \
         `copy(...)` for a value of your own"
    );
    /// A `ref` result from a function that answers a borrow.
    ref_handle_from_reader = "ref-handle:from-reader" (name: &dyn std::fmt::Display) => format!(
        "a `ref` result cannot come from a reader: `{name}` answers a borrow, not a \
         position, and a handle is never recovered from a borrow. Declare `{name}` to \
         return `ref(…)` (a position preserved), or return a place — a field path, a \
         search loop's element, or a mint's result"
    );
    /// A read-only projection passed where the callee mutates.
    ref_handle_read_only_projection = "ref-handle:read-only-projection" (what: &dyn std::fmt::Display, arg_shown: &dyn std::fmt::Display, callee: &dyn std::fmt::Display, pname: &dyn std::fmt::Display) => format!(
        "{what} is a read-only projection (`{arg_shown}`), and `{callee}` mutates \
         `{pname}` — a mutable handle needs a `Mut` element type (`List<Mut T>`). Use \
         `copy(...)` for a value of your own"
    );
    /// A plain projection passed where the callee mutates.
    ref_handle_plain_projection_mutated = "ref-handle:plain-projection-mutated" (what: &dyn std::fmt::Display, arg_shown: &dyn std::fmt::Display, callee: &dyn std::fmt::Display, pname: &dyn std::fmt::Display) => format!(
        "{what} is a plain projection (`{arg_shown}`), and `{callee}` mutates `{pname}`: \
         a mutable element handle is written `ref(c)`, not a bare `proj` — `proj Mut` is \
         read-only. Mint a handle with `at(...)` instead of projecting, or pass \
         `copy(...)` for a value of your own"
    );

    // [runtime-layers]
    /// The runtime's core importing one of its services.
    runtime_layers_core_imports_service = "runtime-layers:core-imports-service" (service: &dyn std::fmt::Display) => format!(
        "the runtime's core may not import its service `{service}`: services are built on \
         the core, which never calls them"
    );

    // [seq-lazy]
    /// A stored iterator callback capturing mutable data.
    seq_lazy_mutable_capture = "seq-lazy:mutable-capture" (what: &dyn std::fmt::Display, captured: &dyn std::fmt::Display, param_name: &dyn std::fmt::Display, fn_name: &dyn std::fmt::Display) => format!(
        "this lambda {what} `{captured}`, and it is the `{param_name}` callback of the \
         iterator function `{fn_name}`: a producer's callback runs once per element in \
         *every* pass, so what the capture accumulates would depend on how often the \
         producer was consumed. Capture an immutable snapshot instead (bind \
         `copy({captured})` to a local at a non-`Mut` type *before* the lambda), or pass \
         the state in and yield it back"
    );

    // [struct-decl]
    /// A struct literal field value that does not fit the field.
    struct_decl_field_mismatch = "struct-decl:field-mismatch" (field: &dyn std::fmt::Display, fty: &dyn std::fmt::Display, vty: &dyn std::fmt::Display) => format!(
        "field `{field}` expects `{fty}`, found `{vty}`"
    );

    // [struct-defaults]
    /// A struct literal missing a field that has no default.
    struct_defaults_missing_field = "struct-defaults:missing-field" (field: &dyn std::fmt::Display, name: &dyn std::fmt::Display) => format!(
        "missing field `{field}` in `{name}` literal"
    );

    // [struct-field-canbe-mut]
    /// `canbe Mut` on a handler's state field.
    struct_field_canbe_mut_handler_state = "struct-field-canbe-mut:handler-state" () =>
        "`canbe Mut` is for a struct's fields; a handler's state is mutable already, so \
         write `Mut` or nothing";
    /// `canbe Mut` on a field of a struct that is not `canbe Mut`.
    struct_field_canbe_mut_struct_not_canbe_mut = "struct-field-canbe-mut:struct-not-canbe-mut" (field: &dyn std::fmt::Display, name: &dyn std::fmt::Display) => format!(
        "field `{field}` says `canbe Mut`, but struct `{name}` does not: a `{name}` value \
         is never `Mut`, so the field never would be; declare `struct {name} canbe Mut`"
    );
    /// `canbe Mut` on a field that is `Mut` already.
    struct_field_canbe_mut_already_mut = "struct-field-canbe-mut:already-mut" (field: &dyn std::fmt::Display) => format!(
        "field `{field}` is already `Mut`; `canbe Mut` gives a field `Mut` only when the \
         struct value has it, so write one or the other"
    );
    /// `canbe Mut` on a field of a type that does not opt into `Mut`.
    struct_field_canbe_mut_not_applicable = "struct-field-canbe-mut:not-applicable" (ty: &dyn std::fmt::Display) => format!(
        "`canbe Mut` does not apply to `{ty}` (its declaration does not say `canbe Mut`)"
    );
    /// A `Mut` literal taking a `canbe Mut` field from a plain spread.
    struct_field_canbe_mut_plain_spread = "struct-field-canbe-mut:plain-spread" (name: &dyn std::fmt::Display, field: &dyn std::fmt::Display, sty: &dyn std::fmt::Display) => format!(
        "a `Mut {name}` literal cannot take field `{field}` (`canbe Mut`) from a plain \
         `{sty}`: it would make the field mutable where the plain value is not; spread a \
         `Mut` value, or give `{field}` here"
    );
    /// A `Mut` literal leaving a `canbe Mut` field to its default.
    struct_field_canbe_mut_default_not_mut = "struct-field-canbe-mut:default-not-mut" (name: &dyn std::fmt::Display, field: &dyn std::fmt::Display) => format!(
        "a `Mut {name}` literal must give field `{field}`: its default is a plain value \
         and the field is `canbe Mut`"
    );

    // [struct-field-resolve]
    /// A field read naming no field of the struct.
    struct_field_resolve_no_field = "struct-field-resolve:no-field" (name: &dyn std::fmt::Display, field: &dyn std::fmt::Display) => format!(
        "struct `{name}` has no field `{field}`"
    );
    /// A field read on a value of a type parameter.
    struct_field_resolve_type_param = "struct-field-resolve:type-param" (name: &dyn std::fmt::Display, field: &dyn std::fmt::Display) => format!(
        "`{name}` is a type parameter, so nothing is known about its values: it has no \
         field `{field}`. Take the concrete type as a parameter instead"
    );
    /// A field read on a value that is not a struct.
    struct_field_resolve_not_a_struct = "struct-field-resolve:not-a-struct" (other: &dyn std::fmt::Display, field: &dyn std::fmt::Display) => format!(
        "`{other}` has no field `{field}`: only structs have fields, and a \
         target-language member needs an accessor fn"
    );
    /// A struct literal naming no field of the struct.
    struct_field_resolve_literal_no_field = "struct-field-resolve:literal-no-field" (name: &dyn std::fmt::Display, field: &dyn std::fmt::Display) => format!(
        "struct `{name}` has no field `{field}`"
    );

    // [struct-mut]
    /// `Mut` on a type that does not opt into it.
    struct_mut_not_canbe_mut = "struct-mut:not-canbe-mut" (base: &dyn std::fmt::Display) => format!(
        "`Mut` does not apply to `{base}` (its declaration does not say `canbe Mut`)"
    );
    /// A field assignment on a value that is not `Mut`.
    struct_mut_immutable_field_assign = "struct-mut:immutable-field-assign" (field: &dyn std::fmt::Display, base_ty: &dyn std::fmt::Display) => format!(
        "cannot assign to field `{field}` of an immutable `{base_ty}` value: only \
         `Mut`-qualified values may have fields assigned"
    );

    // [struct-opaque]
    /// An opaque struct opened outside its module.
    struct_opaque_outside_module = "struct-opaque:outside-module" (shown: &dyn std::fmt::Display, module: &dyn std::fmt::Display, what: &dyn std::fmt::Display, pointer: &dyn std::fmt::Display) => format!(
        "`{shown}` is opaque outside `{module}`, so {what} is not allowed here{pointer}"
    );

    // [task-effects]
    /// A task effect several instances in scope could supply.
    task_effects_ambiguous = "task-effects:ambiguous" (target: &dyn std::fmt::Display, want: &dyn std::fmt::Display, instances: &dyn std::fmt::Display) => format!(
        "`send fn {target}` performs `{want}`, and this scope has several instances the \
         task could inherit ({instances}): the mint carries one, so bind only the \
         instance this task means"
    );
    /// A task effect with no handler in scope.
    task_effects_missing = "task-effects:missing" (target: &dyn std::fmt::Display, want: &dyn std::fmt::Display) => format!(
        "`send fn {target}` performs `{want}`, which a task inherits from the frame that \
         mints it — and this scope has no handler for it. Bind one before the mint, or \
         declare `{want}` here so it is supplied from further out"
    );

    // [task-mint]
    /// A mint of a `send fn` several overloads could be.
    task_mint_ambiguous_send = "task-mint:ambiguous-send" (form: &dyn std::fmt::Display, member: &dyn std::fmt::Display, count: &dyn std::fmt::Display, from: &dyn std::fmt::Display) => format!(
        "`{form}` cannot tell which `send fn {member}` it means: {count} of them are \
         visible from {from}, and a mint carries only the captures — not enough to \
         choose an overload. Rename one, or bring in only the one this scope means"
    );
    /// `replyto!` of a free `send fn`.
    task_mint_gated_free_send = "task-mint:gated-free-send" (member: &dyn std::fmt::Display) => format!(
        "`replyto!` gates the minting actor's mailbox until the answer arrives, and \
         `{member}` is a free `send fn` — a task has no mailbox to gate. Use a bare \
         `replyto`"
    );
    /// A mint targeting a plain `fn`.
    task_mint_plain_fn_target = "task-mint:plain-fn-target" (member: &dyn std::fmt::Display, form: &dyn std::fmt::Display) => format!(
        "`{member}` is a plain `fn`, so `{form}` cannot target it: a free function minted \
         this way runs as a detached task, which needs `send fn {member}(...)` instead \
         of `fn {member}(...)`"
    );

    // [throw]
    /// A handler of `Throw`.
    throw_no_handlers = "throw:no-handlers" (throw_effect: &dyn std::fmt::Display) => format!(
        "`{throw_effect}` has no handlers: a throw is delimited by a `try {{ ... }}` \
         block, not handled"
    );
    /// A throw nothing receives.
    throw_no_receiver = "throw:no-receiver" (throw_effect: &dyn std::fmt::Display, message: &dyn std::fmt::Display) => format!(
        "nothing here can receive a throw: wrap the call in a `try {{ ... }}` block, or \
         declare `[{throw_effect}<{message}>]` in this function's effect list to pass it \
         on"
    );
    /// `throw` with other than one argument.
    throw_argument_count = "throw:argument-count" (member: &dyn std::fmt::Display) => format!(
        "`{member}` takes the throw message as its only argument"
    );

    // [throw-not-main]
    /// `main` declaring `Throw`.
    throw_not_main_declared = "throw-not-main:declared" (throw_effect: &dyn std::fmt::Display) => format!(
        "`main` cannot declare `{throw_effect}`: there is no caller to receive the throw \
         — delimit it inside `main` with a `try {{ ... }}` block instead"
    );

    // [throw-try]
    /// A throw whose message the receiving throw does not carry.
    throw_try_message_mismatch = "throw-try:message-mismatch" (message: &dyn std::fmt::Display, target: &dyn std::fmt::Display) => format!(
        "this throw carries a `{message}` message, but the throw it lands in carries \
         `{target}`"
    );
    /// A `try` block nothing in which can throw.
    throw_try_nothing_throws = "throw-try:nothing-throws" (throw_effect: &dyn std::fmt::Display) => format!(
        "nothing in this `try` block can throw, so it has no outcome to produce: drop the \
         `try`, or call something that declares `[{throw_effect}<M>]`"
    );
    /// A `try` whose core-library qualifier is not in scope.
    throw_try_core_qualifier_missing = "throw-try:core-qualifier-missing" (name: &dyn std::fmt::Display) => format!(
        "`try` needs the `{name}` qualifier from the core library, which is not in scope \
         (is `std` on the source path?)"
    );

    // [type-literal]
    /// A literal arm beside its base type's arm (a warning).
    type_literal_subsumed = "type-literal:subsumed" (value: &dyn std::fmt::Display, base: &dyn std::fmt::Display) => format!(
        "`{value}` is subsumed by the `{base}` arm beside it, which already holds every \
         `{base}`; for the rest of a set of values, write `Other {base}`"
    );
    /// An `is` of a literal its subject can never be.
    type_literal_never_matches = "type-literal:never-matches" (repr: &dyn std::fmt::Display, lit: &dyn std::fmt::Display) => format!(
        "this check can never succeed: a `{repr}` is never `{lit}`"
    );
    /// A literal the expected literal union does not list.
    type_literal_not_listed = "type-literal:not-listed" (lit: &dyn std::fmt::Display, want: &dyn std::fmt::Display, listed: &dyn std::fmt::Display, hint: &dyn std::fmt::Display) => format!(
        "`{lit}` is not one of the values `{want}` lists ({listed}){hint}"
    );
    /// A plain value where a literal union is expected.
    type_literal_needs_narrowing = "type-literal:needs-narrowing" (base: &dyn std::fmt::Display, want: &dyn std::fmt::Display, listed: &dyn std::fmt::Display, hint: &dyn std::fmt::Display) => format!(
        "a `{base}` is not a `{want}`: narrow it to one of {listed} with `is` or `when` \
         first{hint}"
    );

    // [type-literal-qualified]
    /// A union listing one literal twice under different qualifiers.
    type_literal_qualified_listed_twice = "type-literal-qualified:listed-twice" (lit: &dyn std::fmt::Display, a: &dyn std::fmt::Display, b: &dyn std::fmt::Display) => format!(
        "this union lists `{lit}` twice, as `{a}` and as `{b}`: qualifiers erase, so a \
         value of `{lit}` cannot say which arm it is; list it once"
    );

    // [type-no-cycle]
    /// A struct that contains itself.
    type_no_cycle_contains_itself = "type-no-cycle:contains-itself" (start: &dyn std::fmt::Display, field: &dyn std::fmt::Display, via: &dyn std::fmt::Display) => format!(
        "`{start}` contains itself through field `{field}`{via}, so a value of it would \
         be infinitely large. Indirect through a container — `List<{start}>` holds any \
         number of them, including none, and is how a tree or a linked structure is \
         written"
    );

    // [type-union]
    /// `None` where the expected union has no `None` arm.
    type_union_none_not_arm = "type-union:none-not-arm" (expected: &dyn std::fmt::Display) => format!(
        "`None` is not an arm of `{expected}`"
    );
    /// A value several arms of the expected union accept.
    type_union_ambiguous_arm = "type-union:ambiguous-arm" (logical: &dyn std::fmt::Display, expected: &dyn std::fmt::Display) => format!(
        "a value of type `{logical}` matches multiple arms of `{expected}`; qualify the \
         value to disambiguate"
    );
    /// A value no arm of the expected union accepts.
    type_union_no_arm = "type-union:no-arm" (expected: &dyn std::fmt::Display, logical: &dyn std::fmt::Display, deduplicated: bool) => format!(
        "no arm of `{expected}` accepts a value of type `{logical}`{}",
        if deduplicated {
            "; a qualifier applied to a value that already carries it deduplicates, so bind \
             the inner union to an annotated local first (`let inner: A | B = ...`)"
        } else {
            ""
        }
    );

    // [type-union-spread]
    /// A union with two spread arms.
    type_union_spread_two_spreads = "type-union-spread:two-spreads" () =>
        "a union may have at most one `...` spread arm: a second spread would make the \
         split of the arms it captures ambiguous";
    /// A spread arm that is not a bare generic parameter.
    type_union_spread_not_generic = "type-union-spread:not-generic" (inner: &dyn std::fmt::Display) => format!(
        "`...{inner}` is not a valid spread arm: a `...` spread captures one or more \
         arms, so its type must be a bare generic parameter in scope"
    );
    /// A spread outside a union.
    type_union_spread_alone = "type-union-spread:alone" () =>
        "a `...` spread is only a union arm (`...T | Bool`): it has no meaning on its own";

    // [var-let-destructure]
    /// A tuple pattern of the wrong arity.
    var_let_destructure_arity = "var-let-destructure:arity" (count: &dyn std::fmt::Display, what: &dyn std::fmt::Display) => format!(
        "this pattern destructures a tuple of {count}, but {what}"
    );

    // [var-let-infer]
    /// A `let` value that does not fit its annotation.
    var_let_infer_annotation_mismatch = "var-let-infer:annotation-mismatch" (ann: &dyn std::fmt::Display, value_ty: &dyn std::fmt::Display, why: &dyn std::fmt::Display) => format!(
        "expected `{ann}`, found `{value_ty}`{why}"
    );

    // [var-no-shadow]
    /// A local that shadows another.
    var_no_shadow_already_declared = "var-no-shadow:already-declared" (name: &dyn std::fmt::Display) => format!(
        "`{name}` is already declared (shadowing is not allowed)"
    );

    // [var-no-widen]
    /// An assignment of a value that does not fit the variable's type.
    var_no_widen_assign_mismatch = "var-no-widen:assign-mismatch" (value_ty: &dyn std::fmt::Display, target_ty: &dyn std::fmt::Display) => format!(
        "cannot assign `{value_ty}` to `{target_ty}` (a variable's type can never widen)"
    );

    // [var-unused]
    /// A local that is never used (a warning).
    var_unused_never_used = "var-unused:never-used" (name: &dyn std::fmt::Display) => format!(
        "`{name}` is never used; prefix it with `_` (`_{name}`) if that is deliberate"
    );

    // [waitfor-infer]
    /// A `waitfor` whose token nothing passes to a `Reply<T>` parameter.
    waitfor_infer_no_reply_param = "waitfor-infer:no-reply-param" (binding: &dyn std::fmt::Display, reply_type: &dyn std::fmt::Display) => format!(
        "cannot tell what `{binding}` waits for: nothing in the block passes it to a \
         `{reply_type}<T>` parameter. Write the type out (`waitfor {binding}: \
         {reply_type}<Int> {{ … }}`) or send the token somewhere"
    );
    /// A `waitfor` whose token several `Reply<T>` types could take.
    waitfor_infer_ambiguous = "waitfor-infer:ambiguous" (binding: &dyn std::fmt::Display, candidates: &dyn std::fmt::Display, first: &dyn std::fmt::Display) => format!(
        "`{binding}` could wait for any of {candidates}: write the type out (`waitfor \
         {binding}: {first} {{ … }}`)"
    );

    // [when-bind]
    /// A `when` binding whose check matches several types.
    when_bind_several_types = "when-bind:several-types" () =>
        "a `when` binding requires a check matching a single type";

    // [when-exhaustive]
    /// A `when` branch that matches no remaining arm.
    when_exhaustive_dead_branch = "when-exhaustive:dead-branch" () =>
        "this `when` branch matches no remaining union arm";
    /// A `when` that leaves union arms unhandled.
    when_exhaustive_missing_arms = "when-exhaustive:missing-arms" (plural: &dyn std::fmt::Display, missing: &dyn std::fmt::Display) => format!(
        "non-exhaustive `when`: unhandled union arm{plural} {missing}"
    );

    // [when-union-subject]
    /// A `when` subject that is not a plain variable.
    when_union_subject_not_a_variable = "when-union-subject:not-a-variable" () =>
        "`when` requires a plain variable as its subject (or the binding form `when name \
         = subject { … }`)";
    /// A `when` subject that is a claim about a union.
    when_union_subject_qualified_union = "when-union-subject:qualified-union" (subj_ty: &dyn std::fmt::Display, qual: &dyn std::fmt::Display, base: &dyn std::fmt::Display) => format!(
        "`when` requires a union-typed subject, and `{subj_ty}` is the claim `{qual}` \
         *about* a union: bind the inner union to a local and match that (`let inner: \
         {base} = ...`)"
    );
    /// A `when` subject that is not a union.
    when_union_subject_not_a_union = "when-union-subject:not-a-union" (subj_ty: &dyn std::fmt::Display) => format!(
        "`when` requires a union-typed subject (found `{subj_ty}`)"
    );
    // ===== ir =====

    // [actor-self-send]
    /// The IR builder met a self-send selector used as a value.
    actor_self_send_lower_value = "actor-self-send:lower-value" (name: &dyn std::fmt::Display) => format!(
        "`{name}@self` as a value"
    );

    // [call-resolve]
    /// The IR builder met a call through a local it has no binding for.
    call_resolve_lower_unknown_local = "call-resolve:lower-unknown-local" (name: &dyn std::fmt::Display) => format!(
        "call through unknown local `{name}`"
    );

    // [comptime-inline]
    /// The IR builder met a comptime inline-for in a struct literal.
    comptime_inline_lower_unexpanded = "comptime-inline:lower-unexpanded" () =>
        "comptime inline-for survived expansion";

    // [effect-available]
    /// The IR builder found no instance in scope for an effect.
    effect_available_lower_no_instance = "effect-available:lower-no-instance" (want: &dyn std::fmt::Display) => format!(
        "no instance of `{want}` in scope"
    );

    // [effect-dispatch]
    /// The IR builder met a member call to an effect with no declaration.
    effect_dispatch_lower_no_declaration = "effect-dispatch:lower-no-declaration" (effect: &dyn std::fmt::Display) => format!(
        "effect `{effect}` has no declaration"
    );

    // [effect-member-overload]
    /// The IR builder met an effect member as a value, of an effect with no declaration.
    effect_member_overload_lower_no_declaration = "effect-member-overload:lower-no-declaration" (effect: &dyn std::fmt::Display) => format!(
        "effect `{effect}` has no declaration"
    );

    // [effect-use]
    /// The IR builder met a `use` whose value is not a handler construction.
    effect_use_lower_not_construction = "effect-use:lower-not-construction" () =>
        "`use` of something that is not a handler construction";

    // [fate-derived-readonly]
    /// A move or mutation of a derived value whose binding stays borrow-mode.
    fate_derived_readonly_shares_fate = "fate-derived-readonly:shares-fate" (action: &dyn std::fmt::Display, name: &dyn std::fmt::Display, root: &dyn std::fmt::Display) => format!(
        "cannot {action} `{name}`: it was bound from `{root}` and shares \
         its fate, so it can only be read; use `copy` to make an \
         independent value (e.g. `copy({name})`, or bind it with \
         `copy(...)`)"
    );

    // [fate-move-mode]
    /// A use of a root a move-mode binding took; `at` names the binding (" at line 3"), or is empty.
    fate_move_mode_bound_away = "fate-move-mode:bound-away" (x: &dyn std::fmt::Display, binding: &dyn std::fmt::Display, at: &dyn std::fmt::Display) => format!(
        "`{x}` cannot be used here: `{binding}` was bound from it{at} and later moves the value, so the binding \
         took ownership; bind with `copy(...)` (e.g. `let {binding} = copy(...)`) to keep `{x}` usable, or \
         reassign it before use"
    );

    // [fate-partial-move]
    /// A use of a place overlapping one moved out; `at` names the move (" at line 3"), or is empty.
    fate_partial_move_overlapping_use = "fate-partial-move:overlapping-use" (place: &dyn std::fmt::Display, moved: &dyn std::fmt::Display, root: &dyn std::fmt::Display, at: &dyn std::fmt::Display) => format!(
        "`{place}` cannot be used here: `{moved}` was moved out of `{root}`{at}, and this reads the same data; \
         `copy` at the site that moved it to keep this readable"
    );
    /// A use as a whole of a value something was moved out of; `at` names the move (" at line 3"), or is empty.
    fate_partial_move_whole_use = "fate-partial-move:whole-use" (root: &dyn std::fmt::Display, moved: &dyn std::fmt::Display, at: &dyn std::fmt::Display) => format!(
        "`{root}` cannot be used as a whole here: `{moved}` was moved out of it{at}, so part of the value is gone; \
         move the remaining parts individually, or `copy` at the site that moved `{moved}`"
    );

    // [fate-poison]
    /// A use of a derived value whose root an event ended; `at` names the event (" at line 3"), or is empty.
    fate_poison_shared_root = "fate-poison:shared-root" (x: &dyn std::fmt::Display, root: &dyn std::fmt::Display, event: &dyn std::fmt::Display, at: &dyn std::fmt::Display) => format!(
        "`{x}` cannot be used here: it was bound from `{root}` and shares its fate, and `{root}` was {event}{at} \
         after the binding; bind it with `copy(...)` to keep an independent value, or reassign it before use"
    );

    // [flow-use-after-move]
    /// A use of a wholly moved value; `line` is the move's, when known.
    flow_use_after_move_consumed = "flow-use-after-move:consumed" (local: &dyn std::fmt::Display, line: Option<&dyn std::fmt::Display>) => match line {
        None => format!("`{local}` cannot be used here: it was consumed (moved) earlier; reassign it before use"),
        Some(line) => format!(
            "`{local}` cannot be used here: it was consumed (moved) at line {line}; reassign it before use, or `copy` it where it is moved"
        ),
    };
    /// An assignment into a place strictly inside one moved out; `at` names the move (" at line 3"), or is empty.
    flow_use_after_move_write_into_moved = "flow-use-after-move:write-into-moved" (place: &dyn std::fmt::Display, moved: &dyn std::fmt::Display, root: &dyn std::fmt::Display, at: &dyn std::fmt::Display) => format!(
        "`{place}` cannot be assigned here: `{moved}` was moved out of `{root}`{at}, and this writes into it; \
         assign `{moved}` as a whole to put it back, or `copy` at the site that moved it"
    );

    // [fn-effects]
    /// The IR builder cannot adapt a fn value to a fn type lacking an effect the fn needs.
    fn_effects_lower_adapter_effect = "fn-effects:lower-adapter-effect" (effect: &dyn std::fmt::Display) => format!(
        "a fn needs effect `{effect}`, which the fn type it is passed as does not declare"
    );

    // [fn-value]
    /// The IR builder met a scoped reference the checker resolved to nothing.
    fn_value_lower_scoped_reference = "fn-value:lower-scoped-reference" () =>
        "unresolved scoped reference";

    // [implicit-forward]
    /// The IR builder met a forwarded implicit that is not in scope.
    implicit_forward_lower_not_in_scope = "implicit-forward:lower-not-in-scope" (name: &dyn std::fmt::Display) => format!(
        "forwarded implicit `{name}` is not in scope"
    );

    // [interp-to-str]
    /// The IR builder found no text form for an interpolated value.
    interp_to_str_lower_no_to_str = "interp-to-str:lower-no-to-str" (ty: &dyn std::fmt::Display) => format!(
        "no `to_str` for `{ty}` in an interpolation"
    );

    // [interp-union]
    /// The IR builder found no text form for an arm of an interpolated union.
    interp_union_lower_arm_no_to_str = "interp-union:lower-arm-no-to-str" (ty: &dyn std::fmt::Display) => format!(
        "no `to_str` for `{ty}` in an interpolation"
    );
    /// The IR builder found an interpolated union's arms unlike the text forms the checker recorded.
    interp_union_lower_arm_mismatch = "interp-union:lower-arm-mismatch" (ty: &dyn std::fmt::Display) => format!(
        "the arms of `{ty}` do not match the text forms the checker found"
    );

    // [ir-build]
    /// The IR builder met an identifier the checker resolved to nothing.
    ir_build_unresolved_identifier = "ir-build:unresolved-identifier" (name: &dyn std::fmt::Display) => format!(
        "unresolved identifier `{name}`"
    );

    // [ir-linear-check]
    /// A linear value still owed across a call that may throw; `arms` names its live linear arms (" (the `T` arm)"), or is empty.
    ir_linear_check_leak_across_throw = "ir-linear-check:leak-across-throw" (subject: &dyn std::fmt::Display, arms: &dyn std::fmt::Display) => format!(
        "{subject} still owns a linear value{arms} across a call that may throw: the code after it does not run on \
         the throw path, so release it before the call, or move it onward so the obligation travels with it"
    );
    /// A linear value still owed when a path leaves; `arms` names its live linear arms (" (the `T` arm)"), or is empty.
    ir_linear_check_leak_at_exit = "ir-linear-check:leak-at-exit" (subject: &dyn std::fmt::Display, arms: &dyn std::fmt::Display) => format!(
        "{subject} still owns a linear value{arms} when the path leaves; move it onward (pass, return, or store it) \
         or discharge it"
    );

    // [iter-for]
    /// The IR builder met a `for` with no driving call recorded.
    iter_for_lower_no_call = "iter-for:lower-no-call" () =>
        "a `for` with no call recorded for its iterator";

    // [linear-obligation]
    /// An assignment over a local (or state field) that still owes a linear value.
    linear_obligation_assign_drops = "linear-obligation:assign-drops" (x: &dyn std::fmt::Display) => format!(
        "assigning to `{x}` drops the linear value it still owns; move it onward or `discard({x})` first"
    );
    /// A linear value consumed on some of the paths that join, and not others.
    linear_obligation_some_paths = "linear-obligation:some-paths" (name: &dyn std::fmt::Display) => format!(
        "`{name}` owns a linear value that is consumed on some paths but not others; discharge or move it on every path"
    );

    // [linear-state]
    /// An exit from a handler member that leaves a state field moved out of; `what` is the exit.
    linear_state_hole = "linear-state:hole" (what: &dyn std::fmt::Display, name: &dyn std::fmt::Display) => format!(
        "cannot {what}: the state field `{name}` was moved out of and nothing was put back, so this \
         activation would leave the handler with a hole — assign it a value first (`{name} = …`), since \
         the obligations an actor holds must survive between activations"
    );

    // [op-arith]
    /// The IR builder found no function for an arithmetic operator.
    op_arith_lower_no_member = "op-arith:lower-no-member" (op: &dyn std::fmt::Debug, member: &dyn std::fmt::Display) => format!(
        "arithmetic `{op:?}` resolves to no `{member}`"
    );

    // [op-assign]
    /// The IR builder met an assignment whose target is not a place.
    op_assign_lower_non_place = "op-assign:lower-non-place" () =>
        "assignment to a non-place";

    // [op-equality]
    /// The IR builder found no function for a comparison operator.
    op_equality_lower_no_member = "op-equality:lower-no-member" (op: &dyn std::fmt::Debug, member: &dyn std::fmt::Display) => format!(
        "comparison `{op:?}` resolves to no `{member}`"
    );

    // [proj-readonly]
    /// A move or mutation of a wholesale projection.
    proj_readonly_projection_use = "proj-readonly:projection-use" (action: &str, name: &dyn std::fmt::Display, root: &dyn std::fmt::Display) => {
        let why = if action == "mutate" {
            " — a read-only projection never satisfies a `Mut` position \
             (a mutable handle is minted with `at(...)` from a container of `Mut` \
             elements, `List<Mut T>`)"
        } else {
            ""
        };
        format!(
            "cannot {action} `{name}`: it is a projection (`proj`) of `{root}`, \
             which can only be read{why}; use `copy({name})` for a value of your own"
        )
    };

    // [qual-whynot]
    /// The IR builder found no such fn on a predicate qualifier.
    qual_whynot_lower_no_member = "qual-whynot:lower-no-member" (qualifier: &dyn std::fmt::Display, member: &dyn std::fmt::Display) => format!(
        "qualifier `{qualifier}` has no `{member}`"
    );

    // ===== manifest =====

    // [manifest]
    /// A `salvo.toml` that does not parse; `message` is the TOML reader's.
    manifest_invalid_toml = "manifest:invalid-toml" (path: &dyn std::fmt::Display, message: &dyn std::fmt::Display) => format!(
        "`{path}`: {message}"
    );
    /// A `backend` key naming no backend.
    manifest_unknown_backend = "manifest:unknown-backend" (path: &dyn std::fmt::Display, section: &str, backend: &str) => format!(
        "`{path}`: `[{section}] backend` is `{backend}`; it names a backend (`rust`, \
         `kotlin`) or `*` for every backend"
    );

    // [manifest-deps]
    /// `[dependencies]` in a manifest with no `[build] modules`.
    manifest_deps_no_modules = "manifest-deps:no-modules" (path: &dyn std::fmt::Display, name: &str, manifest_file: &str) => format!(
        "`{path}`: `[dependencies]` names `{name}`, but nothing says where dependencies \
         live — add `[build] modules = \"salvo_modules\"` (the directory holding \
         `{name}/{manifest_file}`)"
    );
    /// `[dependencies]` without `[build] modules`, in a manifest built by hand.
    manifest_deps_without_modules = "manifest-deps:without-modules" (path: &dyn std::fmt::Display) => format!(
        "`{path}`: `[dependencies]` without `[build] modules`"
    );
    /// A declared dependency whose directory has no manifest.
    manifest_deps_missing_manifest = "manifest-deps:missing-manifest" (path: &dyn std::fmt::Display, name: &str, manifest_path: &dyn std::fmt::Display) => format!(
        "`{path}`: dependency `{name}` is declared, but `{manifest_path}` does not exist"
    );
    /// A dependency whose manifest states another version than the one declared.
    manifest_deps_version_mismatch = "manifest-deps:version-mismatch" (path: &dyn std::fmt::Display, name: &str, version: &str, manifest_path: &dyn std::fmt::Display, found: &str) => format!(
        "`{path}`: dependency `{name}` is declared at version `{version}`, but `{manifest_path}` \
         says `{found}`"
    );

    // [platform-host-deps]
    /// One crate declared at two versions across the build.
    platform_host_deps_crate_twice = "platform-host-deps:crate-twice" (name: &str, spec: &dyn std::fmt::Display, declared_by: &str, existing: &dyn std::fmt::Display) => format!(
        "crate `{name}` is declared twice at different versions: `{spec}` by {declared_by} \
         and `{existing}` earlier in the build — one version per crate"
    );
    /// A Kotlin artifact that is not a Maven coordinate.
    platform_host_deps_bad_coordinate = "platform-host-deps:bad-coordinate" (path: &dyn std::fmt::Display, coord: &str) => format!(
        "`{path}`: artifact `{coord}` is not `group:artifact:version`"
    );
    /// One Kotlin artifact declared at two versions across the build.
    platform_host_deps_artifact_twice = "platform-host-deps:artifact-twice" (ga: &str, coord: &str, declared_by: &str, other: &str) => format!(
        "artifact `{ga}` is declared twice at different versions: `{coord}` by \
         {declared_by} and `{other}` earlier in the build — one version per artifact"
    );

    // [test-output]
    /// A `[test]` output key outside its closed set; `allowed` lists the set.
    test_output_unknown_value = "test-output:unknown-value" (path: &dyn std::fmt::Display, key: &str, value: &str, allowed: &dyn std::fmt::Display) => format!(
        "`{path}`: `[test] {key}` is `{value}`; it is one of {allowed}"
    );

    // ===== sources =====

    // [manifest-deps]
    /// A dependency whose source directory is missing.
    manifest_deps_missing_source_dir = "manifest-deps:missing-source-dir" (name: &str, src: &dyn std::fmt::Display) => format!(
        "dependency `{name}`: source directory `{src}` does not exist"
    );
    /// A dependency, not `std = true`, declaring a module of the standard library.
    manifest_deps_std_module = "manifest-deps:std-module" (file: &str, dep: &str, module: &dyn std::fmt::Display) => format!(
        "`{file}`: dependency `{dep}` declares module `{module}`, which is the standard \
         library's — a dependency may replace std modules only if its own \
         manifest says `[project] std = true`"
    );

    // [platform-root]
    /// A host file under the other backend's platform root; `lang` and `other` are `Kotlin` / `Rust`.
    platform_root_other_backend_root = "platform-root:other-backend-root" (path: &dyn std::fmt::Display, lang: &str, other: &str) => format!(
        "`{path}` is a {lang} file under the {other} platform root, which is not where {lang} \
         platform files are read from"
    );
    /// A host file in a `platform/` directory, with no platform root for its backend.
    platform_root_file_without_root = "platform-root:file-without-root" (path: &dyn std::fmt::Display, backend: &str) => format!(
        "`{path}` is a platform file, but no platform root is set for {backend}: name it in \
         `salvo.toml` — `[build] platform = \"…\"` for every backend, or `[{backend}] \
         platform` for this one"
    );
    /// A host file in a `platform/` directory outside its backend's platform root.
    platform_root_outside_root = "platform-root:outside-root" (path: &dyn std::fmt::Display, backend: &str, root: &dyn std::fmt::Display) => format!(
        "`{path}` is outside the {backend} platform root `{root}`, so it is not read"
    );

    // [test-file]
    /// A test annex with nothing in front of `.test.sv`.
    test_file_nameless_annex = "test-file:nameless-annex" (path: &dyn std::fmt::Display) => format!(
        "`{path}`: a test annex is named after the module it tests, so there \
         has to be a name in front of `.test.sv`"
    );
    /// A test annex with no module beside it to test.
    test_file_no_tested_module = "test-file:no-tested-module" (annex: &str, tested: &dyn std::fmt::Display, stem: &str) => format!(
        "`{annex}`: no module `{tested}` to test — a test annex is named after \
         the module it belongs to, so this one is looking for a \
         `{stem}.sv` beside it"
    );
    /// A test annex whose module path another file also has.
    test_file_module_collision = "test-file:module-collision" (annex: &str, other: &str, module: &dyn std::fmt::Display) => format!(
        "`{annex}` and `{other}` are both module `{module}`: a test annex takes the \
         module path of the file it is named after plus `test`, so the \
         two spellings collide — rename one"
    );

    // ===== lock =====

    // [actor-protocol-lock]
    /// A lock file that does not parse; `message` is the TOML reader's.
    actor_protocol_lock_invalid_toml = "actor-protocol-lock:invalid-toml" (path: &dyn std::fmt::Display, message: &dyn std::fmt::Display) => format!(
        "`{path}`: {message}"
    );
    /// A locked protocol hash that changed without a version bump; `changed` lists them.
    actor_protocol_lock_changed_without_bump = "actor-protocol-lock:changed-without-bump" (changed: &str, version: &str, path: &dyn std::fmt::Display) => format!(
        "protocol changed without a version bump: {changed} — at version {version}, `{path}` locks the \
         protocols a deployed peer was built against, so a rolling fleet would meet \
         two shapes under one name. Bump `[project] version` in salvo.toml, and the \
         lock is rewritten"
    );

    // ===== backend =====

    // [backend-companion]
    /// Two emitted files at one path.
    backend_companion_duplicate_path = "backend-companion:duplicate-path" (path: &dyn std::fmt::Display, n: usize) => format!(
        "two emitted files claim `{path}` ({n} of them): a module's path and a \
         runtime file collide, so one would silently overwrite the other — \
         rename the runtime file"
    );

    // [backend-host-tool]
    /// A host build tool that is not there; `place` says where it was looked for.
    backend_host_tool_not_found = "backend-host-tool:not-found" (place: &dyn std::fmt::Display, key: &str) => format!(
        "{place}: it builds the declared host libraries — install it, or name the \
         one to use with `{key} = \"…\"` in `salvo.toml`"
    );

    // ===== diag =====

    // [diag-std-hidden]
    /// The one line shown in place of errors hidden in the embedded std.
    diag_std_hidden_std_errors = "diag-std-hidden:std-errors" (errors: usize) => format!(
        "the standard library does not check with this program ({errors} error{} in std; \
         this is a compiler bug)",
        if errors == 1 { "" } else { "s" }
    );

    // ===== cli =====

    // [cli-build]
    /// `--backend '*'` with a backend that names no `out` of its own.
    cli_build_star_needs_out = "cli-build:star-needs-out" (name: &str) => format!(
        "`--backend '*'` builds every backend, so each needs its own \
         artifact directory: give `[{name}] out` in the manifest (or pass \
         `--out` and build one backend at a time)"
    );
    /// A build whose tests failed.
    cli_build_tests_failed = "cli-build:tests-failed" () =>
        "tests failed; no artifact was built";

    // [cli-init]
    /// `salvo init` where a manifest already exists.
    cli_init_exists = "cli-init:exists" (path: &dyn std::fmt::Display) => format!(
        "`{path}` already exists: `salvo init` will not overwrite an existing \
         project"
    );

    // [manifest]
    /// One `--target` for a command building both backends.
    manifest_target_for_both_backends = "manifest:target-for-both-backends" (target: &dyn std::fmt::Display) => format!(
        "`--target {target}` cannot serve both backends: drop it and give each a \
         `target` in the manifest (`[rust]`, `[kotlin]`), or pass `--backend`"
    );
    /// No output directory for a backend.
    manifest_no_target = "manifest:no-target" (backend: &str) => format!(
        "no output directory for `{backend}`: pass `--target`, or state one in the \
         manifest (`[build] target`, or `[{backend}] target`)"
    );

    // [manifest-discovery]
    /// No manifest above the starting directory, and no `--src` or `--main`.
    manifest_discovery_no_manifest = "manifest-discovery:no-manifest" (manifest_file: &str, start: &dyn std::fmt::Display) => format!(
        "no `{manifest_file}` found above `{start}`: pass `--src` (or `--main`), or write a manifest"
    );
    /// The language server's note on a document no manifest covers.
    manifest_discovery_workspace_fallback = "manifest-discovery:workspace-fallback" (manifest_file: &str, root: &dyn std::fmt::Display) => format!(
        "no `{manifest_file}` above this file: analysed under the workspace root `{root}`"
    );

    // [manifest-src]
    /// A `--src` inside, or containing, the project's source root.
    manifest_src_overlaps_root = "manifest-src:overlaps-root" (src: &dyn std::fmt::Display, relation: &str, root: &str, manifest: &str) => format!(
        "`{src}` {relation} the project's source root `{root}` (from `{manifest}`): pass \
         `--src {root}`, or no `--src`; scope a command to one module with \
         `--module` where it has one"
    );

    // [platform-stamp]
    /// A dependency with platform code and no generated host project.
    platform_stamp_no_host_project = "platform-stamp:no-host-project" (dep: &str, root: &dyn std::fmt::Display, regenerate: &str) => format!(
        "dependency `{dep}` has platform code but no generated host project under `{root}`: \
         {regenerate}"
    );
    /// A dependency's host project generated for another ABI or other signatures; `why` says which.
    platform_stamp_stale = "platform-stamp:stale" (dep: &str, why: &str, path: &dyn std::fmt::Display, regenerate: &str) => format!(
        "dependency `{dep}`'s host project {why} (`{path}`): {regenerate}"
    );

    // [test-file]
    /// A test run with no tests; `matching` is "match `f` " under a filter, or empty.
    test_file_no_tests = "test-file:no-tests" (matching: &str, src: &dyn std::fmt::Display) => format!(
        "no tests {matching}in `{src}`: a test is a `test \"name\" {{ … }}` block in a \
         `<module>.test.sv` file beside the module it tests"
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

    /// [diag-id] A rendered message is its text, led by its id under `-v`.
    #[test]
    fn rendered_shows_the_id_only_when_ids_are_shown() {
        let m = Message { id: "diag-id:example", text: "some text".into() };
        assert_eq!(rendered(&m), "some text");
        crate::diag::show_ids(true);
        let shown = rendered(&m);
        crate::diag::show_ids(false);
        assert_eq!(shown, "[diag-id:example] some text");
        assert_eq!(rendered(&m), "some text");
    }
}
