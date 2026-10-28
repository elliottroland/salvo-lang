// [mod-collision] Name collisions are errors instead of last-win.

// [mod-collision] Two imports bringing the same visible name collide.

// FILE: a.sv
export struct Person {
    name: Str
}

// FILE: b.sv
export struct Person {
    age: Int
}

// FILE: main.sv
import a.Person
//~ error[mod-collision:import-conflict]
import b.Person

fn main() -> None {
}

//--- an import colliding with an own declaration
// [mod-collision] An import colliding with an own-module declaration.

// FILE: a.sv
export struct Person {
    name: Str
}

// FILE: main.sv
//~ error[mod-collision:import-conflict]
import a.Person

struct Person {
    age: Int
}

fn main() -> None {
}

//--- an alias avoids the collision
// [mod-collision] Aliasing one of the imports resolves the conflict.

// FILE: a.sv
export struct Person {
    name: Str
}

// FILE: b.sv
export struct Person {
    age: Int
}

// FILE: main.sv
import a.Person
import b.Person as AgedPerson

fn main() -> None {
}

//--- a duplicate in one module
// [mod-collision] A same-kind duplicate within one module is an error.

struct Person {
    name: Str
}

//~ error[mod-collision:duplicate]
struct Person {
    age: Int
}

//--- same-name fns are overloads
// [mod-collision] [fn-overload] Same-name fns are overloads, never
// collisions.

// FILE: a.sv
export fn describe(x: Int) -> Str {
    return "int"
}

// FILE: main.sv
import a.describe

fn describe(x: Str) -> Str {
    return "str"
}
