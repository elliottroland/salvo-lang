// [type-identity] [obligation-by] A clause stamps the declaration it is
// written on. With another module's `struct Token` in scope through a
// whole-module import, `beta`'s own union `Token` was stamped as that struct
// — the comptime expansion looked the name up program-wide, struct first —
// and reported "`Str | Int` is not a `Union`". It reads each file's names
// through the resolver's scope (PIPELINE.md step 6). Clean, warnings
// included: std's union `to_str` by auto ended in an unreachable `return ""`
// until 2026-10-28, which warned in every stamped copy.
// FILE: alpha.sv
export struct Token {
    a: Int
}
// FILE: beta.sv
import alpha

export type Token = Str | Int : ToStr<self> by auto

export fn show(t: Token) -> Str {
    return to_str(t)
}
