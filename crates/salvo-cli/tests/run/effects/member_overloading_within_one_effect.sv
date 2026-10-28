// member overloading within one effect [effect-member-overload]
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

// [effect-member-overload] End to end: which overload runs is the checker's
// answer, and rustc must have no opinion. Byte-identical stdout on Kotlin.
//--- member_overloads

struct InFile { id: Int }
struct OutFile { id: Int }

effect Vault {
    fn close(f: InFile) -> Str => !f
    fn close(f: OutFile) -> Str => !f
    fn describe(f: InFile) -> Str => f
}

handler Files of Vault {
    fn close(f: InFile) -> Str => !f {
        return "closed in ${f.id}"
    }
    fn close(f: OutFile) -> Str => !f {
        return "closed out ${f.id}"
    }
    fn describe(f: InFile) -> Str => f {
        return "file ${f.id}"
    }
}

fn main() [use] {
    use StdConsole()
    use Files()
    println(describe(InFile { id: 3 }))
    println(close(InFile { id: 1 }))
    println(close@Vault(OutFile { id: 2 }))
}
//= stdout
//= file 3
//= closed in 1
//= closed out 2
