// `runtime`: the scheduler every Salvo program runs on, `main` included —
// written once in Salvo over a small platform handler, instead of twice by
// hand in each backend (RUNTIME.md).
//
// This module is **std's own** [mod-std-internal]: the rest of std imports
// it, a program cannot. Its private declarations are also where the language
// makes the exceptions a runtime needs — starting threads, holding locks,
// catching faults — that Salvo code elsewhere cannot (user guidance
// 2026-10-01). It is filled in step by step (RUNTIME.md §11.5); until the
// port, the schedulers are still the backends' `scheduler.rs`/`scheduler.kt`.

// [runtime-parker] One thread's park/unpark token: how a scheduler thread
// with nothing to do sleeps until another thread changes the state it is
// waiting on (RUNTIME.md §11.2 E3, user decision 2026-10-02). Both hosts give
// `unpark` a **token**: an `unpark` that arrives before the `park` makes the
// next `park` return at once, so recording a parker in the scheduler's state
// and parking outside its lock loses no wakeup. A park may also return with
// nobody having unparked (both hosts allow it), so a waiter parks in a loop
// that re-checks its condition.
//
// `threadsafe`: the token is unparked from any thread, while only the thread
// it belongs to may park on it.
export threadsafe platform type Parker

// [runtime-parker] The calling thread's parker.
export platform fn this_parker() [] -> Parker

// [runtime-parker] Sleeps until [p] is unparked, or returns at once if it
// already was since the last park. [p] must be the calling thread's own
// parker; parking on another thread's is a runtime bug and traps.
export platform fn park(p: Parker) [] -> None => p

// [runtime-parker] The same, giving up after [nanos] nanoseconds.
export platform fn park_nanos(p: Parker, nanos: Long) [] -> None => p, nanos

// [runtime-parker] Wakes [p]'s thread, or makes its next park return at once.
export platform fn unpark(p: Parker) [] -> None => p

// [runtime-kept-fn] Runs [body] on a new **daemon** thread: the program
// ends when `main` returns, whatever this thread is doing. The host keeps
// the fn value and runs it elsewhere — the runtime's privilege, since a fn
// value is otherwise only lent for a call [platform-fn-value]. Nothing waits
// for the thread to finish; the scheduler stops one by state, not by handle.
platform fn start_thread(body: once () -> None) [] -> None => !body

// [runtime-kept-fn] Runs [body] inside a fault boundary: `None` when it
// returned, or the host's account of the fault that ended it. What an
// activation runs in, so a fault is the actor's death and not the thread's.
platform fn guarded(body: once () -> None) [] -> Str? => !body

// ===== the host [runtime-host] =====
//
// What only the host can do, behind one interface the backends implement in
// std's platform root (RUNTIME.md §11.3): a new backend writes this handler
// and the scheduler above it is Salvo. Filled in as the port needs it.
effect RuntimeHost {
    // [addr-capability] A fresh value an outsider cannot guess: OS entropy.
    fn secure_bits() -> Long
    // A line on standard error, for the runtime's named reports.
    fn report(line: Str) -> None => line
    // [time-timer] The monotonic clock, in nanoseconds: the timeline
    // `time.tick()` reads, so a deadline and a reading agree.
    fn mono_nanos() -> Long
}

threadsafe platform handler HostRuntime() of RuntimeHost

// [mod-use] The runtime's host, bound once for the process: every function
// here reaches it without declaring it.
use HostRuntime()

// [addr-capability] The bits an addr's identity carries.
fn fresh_bits() [] -> Long {
    return secure_bits()
}

// [time-timer] The monotonic clock, for the services: the timeline
// `time.tick()` reads.
export fn now_nanos() [] -> Long {
    return mono_nanos()
}
