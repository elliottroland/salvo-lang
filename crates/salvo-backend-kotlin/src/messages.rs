//! [diag-id] The text of every diagnostic the Kotlin backend reports, one
//! function per diagnostic, named after its id — the backend's own
//! `messages!`, beside salvo-syntax's (TESTS.md T2). A backend owns its
//! texts: an id here is a `kt-…` label, or a core label the code implements;
//! a core crate never names a `kt-…` one [backend-core-layers].
//!
//! A refusal leaves the backend as a line of text, so a site turns its
//! message into one with `salvo_syntax::messages::rendered`, which leads it
//! with the id under `-v`.

salvo_syntax::messages! {
    // ===== emitter =====

    // [backend-companion]
    /// A companion file at the path of a file the backend generates.
    backend_companion_collides_with_generated = "backend-companion:collides-with-generated" (path: &dyn std::fmt::Display) => format!(
        "companion file `{path}` collides with a generated file"
    );

    // [kt-actor]
    /// The IR met an addr instance whose type is not a named one.
    kt_actor_addr_non_effect_type = "kt-actor:addr-non-effect-type" () =>
        "an addr instance of a non-effect type";
    /// An addr instance of a type that is not an effect.
    kt_actor_addr_not_an_effect = "kt-actor:addr-not-an-effect" (effect: &dyn std::fmt::Display) => format!(
        "`{effect}` is not an effect, so an addr is not an instance of it"
    );
    /// An actor effect generic over values, which has no protocol yet.
    kt_actor_generic_protocol = "kt-actor:generic-protocol" (effect: &dyn std::fmt::Display) => format!(
        "actor effect `{effect}` is generic over values, which the IR emitter cannot make a protocol of yet"
    );
    /// A `spawn … in group` in a program without std `net`.
    kt_actor_group_needs_net = "kt-actor:group-needs-net" () =>
        "`spawn … in group` needs std `net`";
    /// std's `ActorGroup` without the `join` member a group spawn sends.
    kt_actor_group_no_join = "kt-actor:group-no-join" () =>
        "`ActorGroup` has no `join` member";
    /// A `replyto` outside a handler.
    kt_actor_replyto_outside_handler = "kt-actor:replyto-outside-handler" () =>
        "`replyto` outside a handler";
    /// A `replyto` whose task the IR does not declare.
    kt_actor_replyto_unknown_task = "kt-actor:replyto-unknown-task" () =>
        "`replyto` of an unknown task";
    /// A `@self` send outside an actor.
    kt_actor_self_send_outside_actor = "kt-actor:self-send-outside-actor" () =>
        "`@self` send outside an actor";
    /// A self-send whose enclosing handler the IR does not declare.
    kt_actor_self_send_no_impl = "kt-actor:self-send-no-impl" (handler: &dyn std::fmt::Display) => format!(
        "internal: no impl `{handler}` for a self-send"
    );
    /// A send through an effect the IR does not declare.
    kt_actor_send_unknown_interface = "kt-actor:send-unknown-interface" () =>
        "send to an unknown interface";
    /// A `spawn` of something whose type is not a named one.
    kt_actor_spawn_non_handler = "kt-actor:spawn-non-handler" () =>
        "`spawn` of something that is not a handler";
    /// A spawn of a handler with more faces than `Pair`/`Triple` hold.
    kt_actor_spawn_too_many_faces = "kt-actor:spawn-too-many-faces" (handler: &dyn std::fmt::Display, n: usize) => format!(
        "handler `{handler}` implements {n} effects, and the kotlin backend renders a spawn's addr tuple as \
         `Pair`/`Triple` — at most three faces"
    );
    /// A `spawn` of a handler the IR does not declare.
    kt_actor_spawn_unknown_handler = "kt-actor:spawn-unknown-handler" (handler: &dyn std::fmt::Display) => format!(
        "`spawn` of an unknown handler `{handler}`"
    );
    /// A task target without the parameter its answer arrives in.
    kt_actor_task_no_answer_param = "kt-actor:task-no-answer-param" () =>
        "a task target has no answer parameter";
    /// A `waitfor` whose binding is not a `Reply<T>`.
    kt_actor_waitfor_not_reply = "kt-actor:waitfor-not-reply" () =>
        "`waitfor` binds a `Reply<T>`";

    // [kt-actor-ir]
    /// An actor handler without the dispatch function the IR builder generates for a face.
    kt_actor_ir_no_dispatcher = "kt-actor-ir:no-dispatcher" (handler: &dyn std::fmt::Display, effect: &dyn std::fmt::Display) => format!(
        "handler `{handler}` has no dispatcher for `{effect}`"
    );

    // [kt-copy]
    /// A `copy` of a type core's copy plan has no plan for.
    kt_copy_no_plan = "kt-copy:no-plan" (ty: &dyn std::fmt::Display) => format!(
        "cannot `copy` a value of type `{ty}`"
    );

    // [kt-field-canbe-mut]
    /// A `canbe Mut` field whose type's `Mut` rendering is not a subtype of its plain one.
    kt_field_canbe_mut_mut_not_subtype = "kt-field-canbe-mut:mut-not-subtype" (field: &dyn std::fmt::Display) => format!(
        "the kotlin backend cannot store `canbe Mut` field `{field}` yet: its type's `Mut` form is not a subtype \
         of the plain one"
    );

    // [kt-intrinsic-handler]
    /// An intrinsic handler, which has no Kotlin lowering.
    kt_intrinsic_handler_unsupported = "kt-intrinsic-handler:unsupported" (handler: &dyn std::fmt::Display) => format!(
        "intrinsic handler `{handler}` is not supported by the kotlin backend"
    );

    // [kt-ir]
    /// An IR node the builder left unsupported.
    kt_ir_unsupported_node = "kt-ir:unsupported-node" (what: &dyn std::fmt::Display) => format!(
        "unsupported IR node: {what}"
    );

    // [kt-mixed]
    /// A spawn of a mixed handler that is not its construction.
    kt_mixed_spawn_no_construction = "kt-mixed:spawn-no-construction" () =>
        "`spawn` of a mixed handler needs its construction";
    /// A handler implementing actor and plain effects in a shape the mixed lowering does not cover.
    kt_mixed_unrendered = "kt-mixed:unrendered" (handler: &dyn std::fmt::Display) => format!(
        "handler `{handler}` mixes actor and plain effects, which the IR emitter does not render yet"
    );

    // [kt-monitor]
    /// A plain-effect spawn of a handler with several faces.
    kt_monitor_several_plain_faces = "kt-monitor:several-plain-faces" (handler: &dyn std::fmt::Display) => format!(
        "handler `{handler}` implements several plain effects, and a shared instance behind several faces is not \
         supported yet"
    );

    // [kt-platform-factory]
    /// A union's factory object named like a type of its module.
    kt_platform_factory_named_like_type = "kt-platform-factory:named-like-type" (object: &dyn std::fmt::Display) => format!(
        "the factories of `{object}` would be named like the type `{object}`: rename one"
    );

    // [kt-platform-handler]
    /// A platform handler whose effect is not a declared one (the skeleton).
    kt_platform_handler_effect_not_declared = "kt-platform-handler:effect-not-declared" (handler: &dyn std::fmt::Display, effect: &dyn std::fmt::Display) => format!(
        "platform handler `{handler}` implements `{effect}`, which is not a declared effect"
    );
    /// A platform handler whose effect's declaration the emitter cannot find.
    kt_platform_handler_effect_not_located = "kt-platform-handler:effect-not-located" (handler: &dyn std::fmt::Display, effect: &dyn std::fmt::Display) => format!(
        "platform handler `{handler}` implements `{effect}`, whose declaration could not be located"
    );

    // [kt-wire]
    /// A type with no wire codec.
    kt_wire_no_codec = "kt-wire:no-codec" (name: &dyn std::fmt::Display) => format!(
        "no wire codec for `{name}`"
    );
    /// A tuple wider than the `Pair`/`Triple` codecs.
    kt_wire_no_tuple_codec = "kt-wire:no-tuple-codec" (n: usize) => format!(
        "no wire codec for a {n}-tuple"
    );
    /// A `protocol` call whose type argument is not an effect.
    kt_wire_protocol_not_effect = "kt-wire:protocol-not-effect" () =>
        "`protocol` needs an effect as its type argument";
    /// A `protocol<E>` of an effect without a wire form.
    kt_wire_protocol_no_wire_form = "kt-wire:protocol-no-wire-form" (effect: &dyn std::fmt::Display) => format!(
        "`protocol<{effect}>`: `{effect}` has no wire form, so it cannot be a group's protocol"
    );
    /// A `watch_control` without the handler whose `control` it feeds.
    kt_wire_watch_control_no_handler = "kt-wire:watch-control-no-handler" () =>
        "`watch_control` is called from a handler's `init`";
    /// A `watch_control` in a handler without a private `control(from, data)`.
    kt_wire_watch_control_no_control = "kt-wire:watch-control-no-control" (handler: &dyn std::fmt::Display) => format!(
        "`watch_control`: handler `{handler}` has no private `control(from, data)` member"
    );

    // [platform-intrinsic-fn]
    /// An intrinsic fn with no Kotlin lowering.
    platform_intrinsic_fn_unsupported_by_kotlin = "platform-intrinsic-fn:unsupported-by-kotlin" (name: &dyn std::fmt::Display) => format!(
        "intrinsic fn `{name}` is not supported by the kotlin backend"
    );

    // ===== build =====

    // [kt-build]
    /// No `kotlin-stdlib.jar` beside `kotlinc` for a persisted artifact's classpath.
    kt_build_no_kotlin_stdlib = "kt-build:no-kotlin-stdlib" () =>
        "could not find `kotlin-stdlib.jar` beside the `kotlinc` on PATH; it is needed on a persisted \
         artifact's classpath";

    // [kt-gradle]
    /// Artifacts declared, but the Gradle build file was not written.
    kt_gradle_build_file_missing = "kt-gradle:build-file-missing" (path: &dyn std::fmt::Display) => format!(
        "artifacts are declared but `{path}` was not written"
    );
    /// Gradle failed resolving the declared artifacts.
    kt_gradle_failed = "kt-gradle:failed" (code: &dyn std::fmt::Display) => format!(
        "Gradle failed with exit code {code} resolving `[kotlin] artifacts`"
    );
    /// Gradle ran, but its classpath file cannot be read.
    kt_gradle_no_classpath = "kt-gradle:no-classpath" (path: &dyn std::fmt::Display, error: &dyn std::fmt::Display) => format!(
        "Gradle ran but wrote no classpath to `{path}`: {error}"
    );

    // [kt-std-library]
    /// `kotlinc` failed compiling the standard library; `stderr` is its output.
    kt_std_library_kotlinc_failed = "kt-std-library:kotlinc-failed" (stderr: &dyn std::fmt::Display) => format!(
        "kotlinc failed compiling the standard library:\n{stderr}"
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
