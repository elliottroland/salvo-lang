// [platform-threadsafe] the thread-safety contract
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

// [platform-threadsafe] End to end under rustc with the *generated*
// threadsafe skeleton, one stub filled in. Same program, same output as
// the undeclared run — and byte-identical to the Kotlin backend's.
//--- a_threadsafe_platform_handler

effect RawClock {
    fn raw_now() [] -> Int
}

threadsafe platform handler HostRawClock(offset: Int) of RawClock

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

// `threadsafe platform handler HostRawClock` — THE CONTRACT YOU ARE SIGNING:
// every member below is safe to run concurrently with every other, so
// any mutable state has its own synchronization (`Mutex`, `RwLock`,
// atomics). Today the compiler still serializes the instance behind
// the effect's handle, so the receivers are `&mut self` and the
// struct must be `Send`; the contract is what lets a later emission
// drop the lock [platform-threadsafe].
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

impl crate::RawClockPlatformSync for HostRawClock {
    fn raw_now(&self) -> i32 {
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

// `threadsafe platform handler HostRawClock` — THE CONTRACT YOU ARE SIGNING:
// this instance is shared across every thread of the program with NO
// lock around it. Every member below may run concurrently with every
// other, so any mutable state needs its own synchronization
// (`ConcurrentHashMap`, atomics, `synchronized` blocks of your own). If
// the host cannot promise that, delete `threadsafe` from the Salvo
// declaration: the compiler then serializes the instance for you
// [platform-threadsafe].
class HostRawClock(private val offset: Int) : RawClockPlatform {
    override fun rawNow(): Int {
        return offset + 7
    }
}
//= stdout
//= boot@42
