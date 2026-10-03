// `test.actor`: what a `test actor` declaration runs on [test-actor]. The
// harness `salvo test` writes calls [begin_actor_test] before each one; a
// program has no reason to.

import runtime
import runtime.timers

// [test-actor] Starts an actor test: the virtual runtime, as a fresh
// scheduler with no pending deadline, its randomness from [seed] and its
// clock at zero.
export fn begin_actor_test(seed: Long) [] -> None => !seed {
    enter_virtual(seed)
    reset_timers()
}
