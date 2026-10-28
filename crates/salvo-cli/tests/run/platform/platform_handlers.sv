// platform handlers [platform-handler]
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

// [platform-handler] End to end under rustc with the *generated* skeleton,
// one stub filled in. The asserted stdout is byte-identical to the Kotlin
// backend's run of the same program.
//--- a_platform_handler

effect RawClock {
    fn raw_now() [] -> Int
}

// The host implements this one, in Kotlin or Rust: bodyless here, and
// constructed at its `use` like any handler.
platform handler HostRawClock(offset: Int) of RawClock

effect Clock {
    fn stamp(label: Str) -> Str => label
}

handler DefaultClock [RawClock] of Clock {
    fn stamp(label: Str) -> Str => label {
        return "${label}@${raw_now()}"
    }
}

fn main() [use] {
    use StdConsole()
    use HostRawClock(35)
    use DefaultClock()
    println(stamp("boot"))
}
// FILE: salvo.toml
[project]
name = "case"

[build]
platform = "platform"
// FILE: platform/main.rs
// Host implementation of the platform declarations of Salvo module `main`.
//
// Generated once by `salvo platform generate`; the compiler never writes
// this file again — it is yours. Nothing here is checked by Salvo: rustc
// checks it, against the traits the backend generates from the
// `platform handler` declarations.

use crate::*;

// `platform handler HostRawClock` — the compiler SERIALIZES this instance: every
// member runs under one lock on both backends, so the receivers are
// `&mut self` and plain fields are fine; the struct must be `Send`.
// If the host synchronizes internally, declare it `threadsafe platform
// handler` in Salvo to say so [platform-threadsafe].
pub struct HostRawClock {
    offset: i32,
}

impl HostRawClock {
    pub fn new(offset: i32) -> Self {
        Self {
            offset,
        }
    }
}

impl crate::RawClockPlatform for HostRawClock {
    fn raw_now(&mut self) -> i32 {
        self.offset + 7
    }
}
// FILE: platform/main.kt
// Host implementation of the platform declarations of Salvo module `main`.
//
// Generated once by `salvo platform generate`; the compiler never writes
// this file again — it is yours. Nothing here is checked by Salvo: the
// Kotlin compiler checks it, against the interfaces the backend generates
// from the `platform handler` declarations.
package salvo.platform.main

import salvo.main.*

// `platform handler HostRawClock` — the compiler SERIALIZES this instance: every
// member runs under one `synchronized` monitor on both backends, so
// plain fields are fine. If the host synchronizes internally and wants
// to run concurrently, declare it `threadsafe platform handler` in
// Salvo and regenerate [platform-threadsafe].
class HostRawClock(private val offset: Int) : RawClockPlatform {
    override fun rawNow(): Int {
        return offset + 7
    }
}
//= stdout
//= boot@42
