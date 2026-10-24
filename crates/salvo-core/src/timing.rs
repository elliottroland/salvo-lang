//! [phase-timings] Per-phase wall-clock timings, printed to stderr when the
//! environment variable `SALVO_TIMINGS` is set (PIPELINE.md §7 step 1: the
//! case for each pipeline change rests on numbers). Off by default, and then
//! a phase costs one cached environment check.
//!
//! ```text
//! let _t = salvo_core::timing::phase("resolve");
//! ```
//! prints `timing: resolve  12.3ms` when `_t` is dropped.

use std::sync::OnceLock;
use std::time::Instant;

/// Whether timings are being printed (`SALVO_TIMINGS` is set).
pub fn enabled() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("SALVO_TIMINGS").is_some())
}

/// A running phase: reports its duration when dropped.
pub struct Phase {
    name: String,
    start: Instant,
}

/// Starts timing `name`, or `None` when timings are off.
pub fn phase(name: impl Into<String>) -> Option<Phase> {
    enabled().then(|| Phase { name: name.into(), start: Instant::now() })
}

impl Drop for Phase {
    fn drop(&mut self) {
        let ms = self.start.elapsed().as_secs_f64() * 1000.0;
        eprintln!("timing: {:<28} {:>9.1}ms", self.name, ms);
    }
}
