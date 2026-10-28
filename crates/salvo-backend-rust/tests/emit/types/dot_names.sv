// Dot-names [name-dot].

//--- dot_names_flatten
// The same namespace-struct program the Kotlin backend nests: Rust
// flattens the names instead.
// [name-dot] [rs-fn-mangling] Rust concatenates a dot-name
// (`Environment.Id` → `EnvironmentId`): modules and structs share one
// type namespace, so a nested `mod Environment` beside
// `struct Environment` would be E0428. Mangled overload names use the
// same flat spelling.
struct Environment {
    //~ emits(main.rs): pub id: crate::EnvironmentId,
    id: Environment.Id,
    name: Environment.Name
}

// No dots survive in emitted Rust identifiers.
//~ !emits(main.rs): Environment.Id
//~ emits(main.rs): pub struct EnvironmentId {
struct Environment.Id {
    value: Str
}

//~ emits(main.rs): pub struct EnvironmentName {
struct Environment.Name {
    value: Str
}

qualifier Environment.Tag of Str

fn tag(value: Str) -> +Environment.Tag Str
=> !value {
    return value
}

fn label(t: Str) -> Str {
    return "plain ${t}"
}

//~ emits(main.rs): pub fn label__EnvironmentTagStr(
fn label(t: Environment.Tag Str) -> Str {
    return "tagged ${t}"
}

fn main() [use] -> None {
    use StdConsole()
    let env = Environment {
        id: Environment.Id {value: "prod"},
        name: Environment.Name {value: "Production"}
    }
    println("${env.id.value} / ${env.name.value}")
    println(label(tag("t1")))
    println(label("t2"))
}
