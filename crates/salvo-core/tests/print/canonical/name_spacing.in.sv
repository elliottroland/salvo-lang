// A name's argument list touches the name, and a qualifier is a space away
// from what it qualifies, however many spaces the author wrote; a keyword
// is not a name, so a group after one gets its space.
fn ranged(n: InRange(0, 100)   Int, p: Ok   (1, 2), q: proj(xs)    T) -> Bool {
    if(n > 0) {
        return(true)
    }
    while(false) {
    }
    return twice(n)<5
}

handler Prefixed(prefix: Str) for Log {
    fn log(s: Str) => s {
        println(prefix)
    }
}
