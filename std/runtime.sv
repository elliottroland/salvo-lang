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

