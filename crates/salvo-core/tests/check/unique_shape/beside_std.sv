// [op-unique-shape] A module's own `to_str(List<T>)` beside std's is refused
// where it is declared (user decision O1-D, 2026-10-27): `to_str` on `List` is
// interpolation's function, declared only in `List`'s file, so no module can
// put a second one beside it — whether or not anything uses it. Before, it
// sat beside std's and every interpolation and implicit of a list was an
// ambiguity (PIPELINE.md §II.13 D2, dissolved). The error names the type's
// file and the remedy.
//
// [op-unique-shape] The refused declaration is a candidate nowhere, so its
// uses report nothing more: one mistake, one diagnostic (ROADMAP 0ag,
// closed in B3.3c). Before, the interpolation and `render`'s implicit
// each reported the ambiguity it caused.
//~ error[op-unique-shape:outside-home]
fn to_str<T>(list: List<T>, ?to_str: (x: T) -> Str) [] -> Str => list { return "mine" }

fn render<T>(x: T, ?to_str: (x: T) -> Str) [] -> Str => x { return to_str(x) }

fn main() [use] {
    use StdConsole()
    println("${List(1, 2)}")
    println(render(List(1, 2)))
}
