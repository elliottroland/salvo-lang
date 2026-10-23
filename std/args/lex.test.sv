// [args-lex] The tests of module `args.lex`.

fn shapes(args: List<Str>) [] -> Str => args {
    let out: Mut Str = Mut Str("")
    for t in lex(args) {
        when t {
            is LongToken {
                if t.value is None {
                    append(out, "L(${t.name})")
                } else {
                    append(out, "L(${t.name}=${t.value})")
                }
            }
            is ShortToken { append(out, "S(${t.letters})") }
            is WordToken { append(out, "W(${t.value})") }
            is EndToken { append(out, "E") }
        }
    }
    return out
}

test "long flags keep an attached value" {
    expect_eq(shapes(List("--port", "--port=8080", "--name=a=b", "--x=")), "L(port)L(port=8080)L(name=a=b)L(x=)")
}

test "short flags stay bundled" {
    expect_eq(shapes(List("-jt", "-p8080", "-v")), "S(jt)S(p8080)S(v)")
}

test "words, a lone dash and negative numbers" {
    expect_eq(shapes(List("file", "-", "-3", "-.5", "-.x")), "W(file)W(-)W(-3)W(-.5)S(.x)")
}

test "everything after -- is a word" {
    expect_eq(shapes(List("-v", "--", "--port", "-x", "--")), "S(v)EW(--port)W(-x)W(--)")
}
