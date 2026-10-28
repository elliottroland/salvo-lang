//! [diag-id] The text of every diagnostic the Rust backend reports, one
//! function per diagnostic, named after its id — the backend's own
//! `messages!`, beside salvo-syntax's (TESTS.md T2). A backend owns its
//! texts: an id here is an `rs-…` label, or a core label the code
//! implements; a core crate never names an `rs-…` one [backend-core-layers].
//!
//! A refusal leaves the backend as a line of text, so a site turns its
//! message into one with `salvo_syntax::messages::rendered`, which leads it
//! with the id under `-v`.

salvo_syntax::messages! {
    // ===== emitter =====

    // [actor-monitor-handler]
    /// A plain-effect spawn of a handler with several faces.
    actor_monitor_handler_several_plain_faces = "actor-monitor-handler:several-plain-faces" (handler: &dyn std::fmt::Display) => format!(
        "handler `{handler}` implements several plain effects, and a shared instance behind several faces is not \
         supported yet"
    );

    // [backend-companion]
    /// A companion file at the path of the file a module generates.
    backend_companion_collides_with_module = "backend-companion:collides-with-module" (path: &dyn std::fmt::Display, module: &dyn std::fmt::Display) => format!(
        "companion file `{path}` collides with the generated file of module `{module}`: a companion cannot \
         replace a module Salvo emits"
    );

    // [effect-member-generics]
    /// An effect member with generic parameters of its own, which a trait object cannot dispatch.
    effect_member_generics_no_dynamic_dispatch = "effect-member-generics:no-dynamic-dispatch" (member: &dyn std::fmt::Display) => format!(
        "effect member `{member}` has its own generic parameters, which the rust backend cannot dispatch \
         dynamically yet"
    );

    // [platform-intrinsic-fn]
    /// An intrinsic fn with no Rust lowering.
    platform_intrinsic_fn_no_rust_lowering = "platform-intrinsic-fn:no-rust-lowering" (name: &dyn std::fmt::Display) => format!(
        "intrinsic fn `{name}` has no rust lowering"
    );

    // [ref-anchor]
    /// A `ref(c)` argument that is not a handle into the argument passed as `c`.
    ref_anchor_not_into_anchor = "ref-anchor:not-into-anchor" () =>
        "a `ref(c)` argument that is not a handle into the argument passed as `c`";

    // [rs-actor]
    /// The IR met an addr instance whose type is not a named one.
    rs_actor_addr_non_effect_type = "rs-actor:addr-non-effect-type" () =>
        "an addr instance of a non-effect type";
    /// An addr instance of a type that is not an effect.
    rs_actor_addr_not_an_effect = "rs-actor:addr-not-an-effect" (effect: &dyn std::fmt::Display) => format!(
        "`{effect}` is not an effect, so an addr is not an instance of it"
    );
    /// A generic actor effect, which has no protocol yet.
    rs_actor_generic_protocol = "rs-actor:generic-protocol" (effect: &dyn std::fmt::Display) => format!(
        "effect `{effect}` is generic, which the rust backend cannot make an actor protocol of yet"
    );
    /// A `spawn … in group` in a program without std `net`.
    rs_actor_group_needs_net = "rs-actor:group-needs-net" () =>
        "`spawn … in group` needs std `net`";
    /// std's `ActorGroup` without the `join` member a group spawn sends.
    rs_actor_group_no_join = "rs-actor:group-no-join" () =>
        "`ActorGroup` has no `join` member";
    /// A `replyto` naming a member its handler does not have.
    rs_actor_replyto_no_member = "rs-actor:replyto-no-member" (member: &dyn std::fmt::Display) => format!(
        "`replyto {member}`: no such member"
    );
    /// A `replyto` outside a handler.
    rs_actor_replyto_outside_handler = "rs-actor:replyto-outside-handler" () =>
        "`replyto` outside a handler";
    /// A `replyto` whose task the IR does not declare.
    rs_actor_replyto_unknown_task = "rs-actor:replyto-unknown-task" () =>
        "`replyto` of an unknown task";
    /// A `@self` send outside an actor.
    rs_actor_self_send_outside_actor = "rs-actor:self-send-outside-actor" () =>
        "`@self` send outside an actor";
    /// A send through an effect the IR does not declare.
    rs_actor_send_unknown_interface = "rs-actor:send-unknown-interface" () =>
        "a send to an unknown interface";
    /// A spawn of a generic handler.
    rs_actor_spawn_generic_handler = "rs-actor:spawn-generic-handler" (handler: &dyn std::fmt::Display) => format!(
        "spawning generic handler `{handler}` is not supported yet"
    );
    /// A `spawn` of something whose type is not a named one.
    rs_actor_spawn_non_handler = "rs-actor:spawn-non-handler" () =>
        "`spawn` of something that is not a handler";
    /// A `spawn` of a handler the IR does not declare.
    rs_actor_spawn_unknown_handler = "rs-actor:spawn-unknown-handler" (handler: &dyn std::fmt::Display) => format!(
        "`spawn` of an unknown handler `{handler}`"
    );
    /// A task target without the parameter its answer arrives in.
    rs_actor_task_no_answer_param = "rs-actor:task-no-answer-param" () =>
        "a task target has no answer parameter";
    /// A `waitfor` whose binding is not a `Reply<T>`.
    rs_actor_waitfor_not_reply = "rs-actor:waitfor-not-reply" () =>
        "`waitfor` binds a `Reply<T>`";

    // [rs-actor-ir]
    /// An actor handler without the dispatch function the IR builder generates for a face.
    rs_actor_ir_no_dispatcher = "rs-actor-ir:no-dispatcher" (handler: &dyn std::fmt::Display, effect: &dyn std::fmt::Display) => format!(
        "handler `{handler}` has no dispatcher for `{effect}`"
    );
    /// An unpack of a type that is not one of the enums the IR builder generates.
    rs_actor_ir_unpack_non_enum = "rs-actor-ir:unpack-non-enum" () =>
        "an unpack of something that is not a generated enum";

    // [rs-intrinsic-handler]
    /// An intrinsic handler with no Rust lowering.
    rs_intrinsic_handler_no_lowering = "rs-intrinsic-handler:no-lowering" (handler: &dyn std::fmt::Display) => format!(
        "intrinsic handler `{handler}` has no rust lowering"
    );

    // [rs-ir]
    /// A call to a fn the IR does not declare.
    rs_ir_call_no_declaration = "rs-ir:call-no-declaration" (name: &dyn std::fmt::Display) => format!(
        "a call to `{name}`, which has no declaration in the IR"
    );
    /// A construction whose type is not a named one.
    rs_ir_construct_non_named = "rs-ir:construct-non-named" () =>
        "a construct of a non-named type";
    /// A handler whose face the IR does not declare.
    rs_ir_face_not_found = "rs-ir:face-not-found" (handler: &dyn std::fmt::Display, face: &dyn std::fmt::Display) => format!(
        "handler `{handler}`: face `{face}` not found"
    );
    /// A fn value whose type is not a fn type.
    rs_ir_fn_value_non_fn = "rs-ir:fn-value-non-fn" () =>
        "a fn value of a non-fn type";
    /// A handler call on a type the IR declares no handler for.
    rs_ir_handler_call_not_handler = "rs-ir:handler-call-not-handler" (name: &dyn std::fmt::Display) => format!(
        "a handler call on `{name}`, which is not a handler"
    );
    /// A handler call on something whose type is not a named one.
    rs_ir_handler_call_non_handler = "rs-ir:handler-call-non-handler" () =>
        "a handler call on something that is not a handler";
    /// A handler call of a member the IR does not declare.
    rs_ir_handler_call_unknown_member = "rs-ir:handler-call-unknown-member" () =>
        "a handler call of an unknown member";
    /// A handler constructed without one of its constructor parameters.
    rs_ir_handler_missing_param = "rs-ir:handler-missing-param" (handler: &dyn std::fmt::Display, param: &dyn std::fmt::Display) => format!(
        "handler `{handler}` constructed without `{param}`"
    );
    /// A handler without a body for a member of its face.
    rs_ir_handler_no_member = "rs-ir:handler-no-member" (handler: &dyn std::fmt::Display, member: &dyn std::fmt::Display) => format!(
        "handler `{handler}` has no member `{member}`"
    );
    /// A member call of a member the IR does not declare.
    rs_ir_unknown_member = "rs-ir:unknown-member" () =>
        "a member call to an unknown member";
    /// A value of a type no Rust spelling exists for.
    rs_ir_unrenderable_type = "rs-ir:unrenderable-type" (ty: &dyn std::fmt::Display) => format!(
        "a value of type `{ty}` reached rust code generation"
    );
    /// An IR node the builder left unsupported.
    rs_ir_unsupported_node = "rs-ir:unsupported-node" (what: &dyn std::fmt::Display) => format!(
        "unsupported IR node: {what}"
    );

    // [rs-iter-pass]
    /// A `for` over a type with no Rust iteration.
    rs_iter_pass_no_iteration = "rs-iter-pass:no-iteration" (subject: &dyn std::fmt::Display) => format!(
        "`for` over `{subject}`, which has no rust iteration"
    );

    // [rs-loc]
    /// A locator answering a handle into something other than its anchor.
    rs_loc_handle_outside_anchor = "rs-loc:handle-outside-anchor" () =>
        "a locator answers a handle into something other than its anchor";
    /// A lending callback called on something that is not a place.
    rs_loc_lending_callback_not_place = "rs-loc:lending-callback-not-place" () =>
        "a lending callback called on something that is not a place";
    /// A locator returning something other than a handle of its anchor.
    rs_loc_returns_non_anchor_handle = "rs-loc:returns-non-anchor-handle" () =>
        "a locator returns something other than a handle of its anchor";

    // [rs-mixed]
    /// A spawn of a mixed handler that is not its construction.
    rs_mixed_spawn_no_construction = "rs-mixed:spawn-no-construction" () =>
        "`spawn` of a mixed handler needs its construction";
    /// A handler implementing actor and plain effects in a shape the mixed lowering does not cover.
    rs_mixed_unrendered = "rs-mixed:unrendered" (handler: &dyn std::fmt::Display) => format!(
        "handler `{handler}` mixes actor and plain effects, which the rust backend does not render"
    );

    // [rs-mod-use]
    /// A module-level `use` whose IR has no binding.
    rs_mod_use_no_binding = "rs-mod-use:no-binding" () =>
        "a module-level `use` with no binding";

    // [rs-mut-arg-hoist]
    /// A read of mutable data an argument after it mutates; `at` is where.
    rs_mut_arg_hoist_read_then_mutated = "rs-mut-arg-hoist:read-then-mutated" (place: &dyn std::fmt::Display, at: &dyn std::fmt::Display) => format!(
        "cannot lower this call: `{place}` is read here while a later argument mutates it, and it is mutable data \
         — a copy would be a snapshot on one backend and a live handle on the other, so the program has to \
         choose: `copy({place})` for the snapshot, or give the mutating call its own statement first (at {at})"
    );

    // [rs-narrow-mut]
    /// A mutable use whose value is a temporary of its own expression.
    rs_narrow_mut_mutable_temporary = "rs-narrow-mut:mutable-temporary" () =>
        "cannot lower this mutable use: the value it would mutate is a temporary of its own expression";
    /// A mutation through the union arm of a read-only root.
    rs_narrow_mut_read_only_arm = "rs-narrow-mut:read-only-arm" (root: &dyn std::fmt::Display) => format!(
        "`{root}` is read-only here, so a `Mut` value cannot be mutated through its union arm: the arm's `Mut` \
         is a claim about the arm, not about `{root}`, so this frame received it borrowed. Take the payload as \
         its own parameter (`Mut List<T>`) and check the arm at the call site."
    );

    // [rs-opt-borrow]
    /// A union of borrows with no arm map onto the target union.
    rs_opt_borrow_cannot_lend = "rs-opt-borrow:cannot-lend" (from: &dyn std::fmt::Display, to: &dyn std::fmt::Display) => format!(
        "cannot lend a `{from}` as a `{to}`"
    );

    // [rs-path]
    /// A handle into a generic container where another path type is expected.
    rs_path_generic_where_other_expected = "rs-path:generic-where-other-expected" () =>
        "a handle into a generic container where another path type is expected";
    /// A handle into a container that is a type parameter.
    rs_path_generic_container = "rs-path:generic-container" (container: &dyn std::fmt::Display) => format!(
        "a handle into the generic container `{container}` has no path on the Rust backend yet"
    );
    /// A handle type whose `ref` names no container.
    rs_path_handle_names_no_container = "rs-path:handle-names-no-container" (handle: &dyn std::fmt::Display) => format!(
        "internal: the handle type `{handle}` names no container"
    );
    /// A lending effect member called on something that is not a place.
    rs_path_lending_member_not_place = "rs-path:lending-member-not-place" () =>
        "a lending effect member called on something that is not a place";
    /// A locator rendered without its path type.
    rs_path_locator_no_path_type = "rs-path:locator-no-path-type" () =>
        "internal: a locator with no path type";
    /// Two live handles where one contains the other.
    rs_path_nested_live_handles = "rs-path:nested-live-handles" () =>
        "two live handles where one contains the other";
    /// No storage of a container holds the element type.
    rs_path_no_storage_holds = "rs-path:no-storage-holds" (container: &dyn std::fmt::Display, element: &dyn std::fmt::Display) => format!(
        "no storage of `{container}` holds a `{element}`"
    );
    /// A path type with several shapes not declared ahead of its use.
    rs_path_not_declared_ahead = "rs-path:not-declared-ahead" (container: &dyn std::fmt::Display, element: &dyn std::fmt::Display) => format!(
        "internal: the path type of `{container}` to `{element}` was not declared ahead"
    );
    /// A field of an optional handle read before the handle was tested.
    rs_path_optional_handle_untested = "rs-path:optional-handle-untested" () =>
        "a field of an optional handle read before it was tested";
    /// A handle whose path is not one its container's type allows.
    rs_path_path_not_allowed = "rs-path:path-not-allowed" () =>
        "a handle whose path is not one its container's type allows";
    /// Two handles whose paths differ in kind.
    rs_path_paths_differ_in_kind = "rs-path:paths-differ-in-kind" () =>
        "two handles whose paths differ in kind";
    /// A container holding more of itself, whose handles have unbounded paths.
    rs_path_recursive_container = "rs-path:recursive-container" (container: &dyn std::fmt::Display) => format!(
        "`{container}` contains itself, so its handles have no finite path yet"
    );

    // [rs-platform-check]
    /// A platform check naming a qualifier its module does not declare.
    rs_platform_check_qualifier_not_found = "rs-platform-check:qualifier-not-found" (qualifier: &dyn std::fmt::Display, module: &dyn std::fmt::Display) => format!(
        "internal: qualifier `{qualifier}` of module `{module}` not found"
    );

    // [rs-platform-factory]
    /// A union's factories named like a type of its module.
    rs_platform_factory_named_like_type = "rs-platform-factory:named-like-type" (target: &dyn std::fmt::Display) => format!(
        "the factories of `{target}` would be named like the type `{target}`: rename one"
    );

    // [rs-platform-handler]
    /// A platform handler whose effect is not a declared one (the skeleton).
    rs_platform_handler_effect_not_declared = "rs-platform-handler:effect-not-declared" (handler: &dyn std::fmt::Display, effect: &dyn std::fmt::Display) => format!(
        "platform handler `{handler}` implements `{effect}`, which is not a declared effect"
    );

    // [rs-wire]
    /// A `protocol<E>` of an effect without a wire form.
    rs_wire_protocol_no_wire_form = "rs-wire:protocol-no-wire-form" () =>
        "`protocol<E>`: `E` has no wire form, so it cannot be a group's protocol";

    // [runtime-sched]
    /// A program using actors whose runtime module was not reached.
    runtime_sched_runtime_not_reached = "runtime-sched:runtime-not-reached" () =>
        "internal: this program uses actors but the runtime module (`std/runtime.sv`) was not reached";

    // [throw]
    /// A throw with neither a `try` nor a throwing fn to leave to.
    throw_outside_try = "throw:outside-try" () =>
        "a throw outside a `try` in a fn that does not throw";

    // ===== build =====

    // [rs-cargo]
    /// Crates declared, but the Cargo manifest was not written.
    rs_cargo_manifest_missing = "rs-cargo:manifest-missing" (path: &dyn std::fmt::Display) => format!(
        "crates are declared but `{path}` was not written"
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
            assert!(salvo_syntax::messages::id_well_formed(id), "`{id}` is not `label:name`");
            assert!(salvo_syntax::messages::name_matches_id(name, id), "`{name}` is not named after `{id}`");
        }
    }
}
