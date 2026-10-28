// [mod-std-internal] std's own module, `std.runtime`, as a program sees it.
// (That std may import it, and the runtime's own layering, need std files of
// a test's own: `export_tests.rs`.)

// [mod-std-internal] A program cannot import the runtime module, whole or by
// name, even what it exports.
//~ error[mod-std-internal:import]
import std.runtime
//~ error[mod-std-internal:import]
import std.runtime.now_nanos

fn f() [] -> Int {
    return 1
}

//--- the bare spelling
// [mod-std-internal] [std-root] The bare spelling is refused for what it
// would have named: std's own module, not a path to correct towards (no
// `std-root` diagnostic).
//~ error[mod-std-internal:import]
import runtime

fn f() [] -> Int {
    return 1
}

//--- a module-level use outside the runtime module
// [mod-use] A module-level `use` binds for every function of the module
// without being declared — in the runtime module only; anywhere else it is
// refused, naming why.
effect Ticks {
    fn tick() -> Int
}

handler Ticking() of Ticks {
    fn tick() -> Int {
        return 1
    }
}

//~ error[mod-use:module-level]
use Ticking()

fn twice() [] -> Int {
    return tick()
}
