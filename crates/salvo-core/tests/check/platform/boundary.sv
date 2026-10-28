// [platform-check] What crosses from host to Salvo.

// [platform-check] A constructive qualifier — no `qualifies`, so nothing can
// prove it of a host value — is trusted when the platform declaration sits in
// its module, as its constructors are (here), and refused from another
// (`other.sv`) (ABI D7).
qualifier Clean of Str

platform fn scrub(s: Str) [] -> Clean Str => s

// [platform-check] A check that walks a collection warns on the declaration
// (D10 C3); one that does not, does not.
//~ warning[platform-check:element-cost]
platform fn tiers() [] -> List<"a" | "b">
platform fn tier() [] -> "a" | "b"

// [platform-check] D10 C5: a platform fn's result may not borrow from its
// parameters — the host has no Salvo lifetimes to hand back.
struct Box { v: Str }

//~ error[platform-check:borrowed-return]
platform fn peek(b: Box) [] -> proj(b) Str => b

// FILE: lib.sv
export qualifier Clean of Str

// FILE: other.sv
import lib.Clean

//~ error[platform-check:foreign-constructive]
platform fn scrub(s: Str) [] -> Clean Str => s
