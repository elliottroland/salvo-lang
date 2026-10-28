// Provenance arms whose payload is itself a union [qual-group] [qual-arm-infer].

//--- a_provenance_arm_holding_a_union
// `ok(ok(1))` carries one flat `Ok` (a qualifier applied twice
// deduplicates), which arm inference reads as the payload of the outer `Ok`
// arm; that payload is the union `Ok Int | Err Str`, so the value is wrapped
// into it first. Found 2026-10-28: both backends were handed the value
// wrapped once, and rustc and kotlinc refused it.
fn step(n: Int) [] -> Ok (Ok Int | Err Str) | Err Str {
    if n == 0 {
        return ok(ok(1))
    }
    return err("outer")
}

fn show(s: Ok (Ok Int | Err Str) | Err Str) -> Str {
    when s {
        is Ok {
            let inner: Ok Int | Err Str = s
            when inner {
                is Ok { return "ok ok ${inner}" }
                is Err { return "ok err ${inner}" }
            }
        }
        is Err { return "err ${s}" }
    }
}

fn main() [use] -> None {
    use StdConsole()
    println(show(step(0)))
    println(show(step(1)))
}
//= stdout
//= ok ok 1
//= err outer
