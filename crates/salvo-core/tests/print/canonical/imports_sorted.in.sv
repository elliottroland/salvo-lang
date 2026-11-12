// [fmt-imports] Imports sort by their line, each with the comments written
// directly above it, a trailing comment and an alias kept; the blank lines
// between them go, and nothing is removed, not even a repeated import.

import b.x
// why c
import c.y // trailing

import a.z as w

import b.x

fn f() {}
