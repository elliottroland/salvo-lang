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

// [platform-tree] The tree mirrors the sources, per module: the implementation
// of a handler declared in `obs/telemetry.sv` is `platform/obs/telemetry.<ext>`,
// and a `use` of it from another module constructs it across modules — where
// a mistake would be a target-language error rather than anything Salvo would
// notice. The host files are the generated skeletons with the stub filled in;
// what `generate` writes, and where, is `platform_tests.rs`'s
// `the_platform_tree_mirrors_the_source_tree`.
//--- a_handler_declared_in_a_nested_module
// FILE: salvo.toml
[project]
name = "case"

[build]
platform = "platform"
// FILE: obs/telemetry.sv
export effect Telemetry {
    fn record(name: Str, value: Int) [] -> None => name, value
}

export platform handler HostTelemetry of Telemetry
// FILE: main.sv
import obs.telemetry.Telemetry
import obs.telemetry.HostTelemetry

fn main() [use] {
    use HostTelemetry()
    record("tool", 7)
}
// FILE: platform/obs/telemetry.rs
// Host implementation of the platform declarations of Salvo module `obs.telemetry`.
//
// Generated once by `salvo platform generate`; the compiler never writes
// this file again — it is yours. Nothing here is checked by Salvo: rustc
// checks it, against the traits the backend generates from the
// `platform handler` declarations.

use crate::obs_telemetry::*;

// `platform handler HostTelemetry` — the compiler SERIALIZES this instance: every
// member runs under one lock on both backends, so the receivers are
// `&mut self` and plain fields are fine; the struct must be `Send`.
// If the host synchronizes internally, declare it `threadsafe platform
// handler` in Salvo to say so [platform-threadsafe].
pub struct HostTelemetry {
}

impl HostTelemetry {
    pub fn new() -> Self {
        Self { }
    }
}

impl crate::obs_telemetry::TelemetryPlatform for HostTelemetry {
    fn record(&mut self, name: &String, value: i32) {
        println!("[t] {}={}", name, value);
    }
}
// FILE: platform/obs/telemetry.kt
// Host implementation of the platform declarations of Salvo module `obs.telemetry`.
//
// Generated once by `salvo platform generate`; the compiler never writes
// this file again — it is yours. Nothing here is checked by Salvo: the
// Kotlin compiler checks it, against the interfaces the backend generates
// from the `platform handler` declarations.
package salvo.platform.obs.telemetry

import salvo.obs.telemetry.*

// `platform handler HostTelemetry` — the compiler SERIALIZES this instance: every
// member runs under one `synchronized` monitor on both backends, so
// plain fields are fine. If the host synchronizes internally and wants
// to run concurrently, declare it `threadsafe platform handler` in
// Salvo and regenerate [platform-threadsafe].
class HostTelemetry : TelemetryPlatform {
    override fun record(name: String, value: Int) {
        kotlin.io.println("[t] $name=$value")
    }
}
//= stdout
//= [t] tool=7
