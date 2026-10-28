// Dot-names [name-dot] [kt-nested-dot-name]: nested classes and the flat
// spelling of a dot-named qualifier in a mangled name.

//--- dot_names_emit_nested_classes
// [name-dot] [kt-nested-dot-name] Dot-named structs emit as *nested*
// classes (never `inner`) and are referenced with the dotted name;
// a dot-named qualifier canonicalizes to its flat spelling in a mangled
// overload name [fn-emit-name].
// A namespace struct with two dot-named members plus a dot-named
// qualifier driving overload mangling. (`DOT_NAMES` in codegen_tests.rs.)
// The namespace class gains a body holding its members, declared under
// their member segment only.
//~ emits: data class Environment(
//~ emits: ) {
//~|
// Never `inner`: that would need an outer instance to construct.
//~ !emits: inner class
// References keep the dotted spelling — valid Kotlin nested access.
//~ emits: val id: Environment.Id
struct Environment {
    id: Environment.Id,
    name: Environment.Name
}

//~ emits:     data class Id(
struct Environment.Id {
    value: Str
}

//~ emits:     data class Name(
struct Environment.Name {
    value: Str
}

qualifier Environment.Tag of Str

fn tag(value: Str) -> +Environment.Tag Str => !value {
    return value
}

fn label(t: Str) -> Str {
    return "plain ${t}"
}

// Mangling flattens the dot into a single identifier.
//~ emits: fun label__EnvironmentTagStr(
fn label(t: Environment.Tag Str) -> Str {
    return "tagged ${t}"
}

fn main() [use] -> None {
    use StdConsole()
    let env = Environment {
        //~ emits: Environment.Id(value = "prod")
        id: Environment.Id {value: "prod"},
        name: Environment.Name {value: "Production"}
    }
    println("${env.id.value} / ${env.name.value}")
    println(label(tag("t1")))
    println(label("t2"))
}
