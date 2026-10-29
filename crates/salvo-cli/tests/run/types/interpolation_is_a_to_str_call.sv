// [interp-to-str] An interpolated part is the call `to_str(part)` (PIPELINE.md
// §II.10 P4, option (a), user decision 2026-10-28; built in B3.2), resolved as
// a written `to_str` call is.
//
// Compile-and-run data ([test-run-data]).

//--- an_enclosing_implicit_takes_a_scalar_and_a_str
// [implicit-resolve-body] Census D1: `${n}` inside a fn with `?to_str` covering
// `Int` goes through the implicit, as the call `to_str(n)` does (before, the
// interpolation printed `7` and the call `#7`). A `Str` part likewise.

fn hex(x: Int) [] -> Str => x { return "#${x}" }

fn quoted(s: Str) [] -> Str => s { return "'${s}'" }

fn show(n: Int, ?to_str: (x: Int) -> Str) [Console] -> None => n, to_str {
    println("interp: ${n}")
    println("call:   ${to_str(n)}")
}

fn say(s: Str, ?to_str: (x: Str) -> Str) [Console] -> None => s, to_str {
    println("says ${s}")
}

fn main() [use] {
    use StdConsole()
    show(7, to_str = hex)
    say("hi", to_str = quoted)
    say("hi")
}
//= stdout
//= interp: #7
//= call:   #7
//= says 'hi'
//= says hi

//--- an_effect_member_renders_a_part
// [effect-member-overload] R10 D3: an available effect's `to_str` takes the
// part, through the handler in scope.

struct Point { x: Int }

effect Fmt {
    fn to_str(p: Point) -> Str => p
}

handler Angle of Fmt {
    fn to_str(p: Point) -> Str => p { return "<${p.x}>" }
}

fn main() [use] {
    use StdConsole()
    use Angle()
    let p = Point { x: 3 }
    println("point ${p}")
}
//= stdout
//= point <3>
