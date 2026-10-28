// How a multi-module program lays out as a crate.

//--- crate_layout_mounts_only_used_modules
// [mod-used-only] [rs-crate] [rs-imports]
// The main module is the crate root: attributes + mounted mods (the
// attributes coming first, and `unused.rs` not being emitted at all, are
// `crate_layout_mounts_only_used_modules` in `codegen_tests.rs`).
//~ emits(main.rs): #[path = "geometry.rs"]
//~| pub mod geometry;
//~ emits(main.rs): #[path = "std/core/console.rs"]
//~| pub mod std_core_console;
//~ !emits(main.rs): pub mod unused;
// Generated imports: one per name used [rs-imports].
//~ emits(main.rs): use crate::geometry::area;
//~ emits(main.rs): use crate::std_core_console::println;
//~ !emits(main.rs): use crate::geometry::*;
import geometry.area

fn main() [use] -> None {
    use StdConsole
    println("area: ${area(3, 4)}")
}
// FILE: geometry.sv
//~ emits(geometry.rs): pub fn area(
export fn area(w: Int, h: Int) -> Int {
    return w * h
}
// FILE: unused.sv
export fn never_called() -> Int {
    return 42
}
