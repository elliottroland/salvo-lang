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
threadsafe platform type Parker

// [runtime-parker] The calling thread's parker.
platform fn this_parker() [] -> Parker

// [runtime-parker] Sleeps until [p] is unparked, or returns at once if it
// already was since the last park. [p] must be the calling thread's own
// parker; parking on another thread's is a runtime bug and traps.
platform fn park(p: Parker) [] -> None => p

// [runtime-parker] The same, giving up after [nanos] nanoseconds.
platform fn park_nanos(p: Parker, nanos: Long) [] -> None => p, nanos

// [runtime-parker] Wakes [p]'s thread, or makes its next park return at once.
platform fn unpark(p: Parker) [] -> None => p
