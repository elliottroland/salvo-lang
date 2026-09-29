// The structural implementations — the ones `by auto` stamps (user decisions
// 2026-09-28, the comptime rounds). Each is a `comptime fn` [comptime-bound]:
// compile-time code over a type's fields or arms, instantiated at a concrete
// type by a `by` clause and never called as itself. What a body may know about
// `T` is `core.comptime`'s model: `T.fields` is a `List<Field>`, `T.arms` a
// `List<Arm>`, and a `[when field.type]` is a `when` over `Type`.
//
// [obligation-by] `struct Point : Ordered<self> by auto, Hashed<self> by auto`
// stamps `cmp`, `hash` and `eq` for `Point`, each declared on the type
// [fn-attached]. [fn-by] `fn cmp(a: Point, b: Point) -> Int by auto` stamps
// one member, which is how a type mixes a structural `cmp` with a
// hand-written `eq`. Nothing here applies on its own: being in `core` means
// `auto` can be named without an import, and nothing more.
//
// What a stamped body needs of the type is checked where the type is
// declared: `cmp(a.[field], b.[field])` resolves per field, so a field with
// no `cmp` in scope — a `Double`, a function — is reported at that field, as
// "in `cmp` from `auto` for `Point.y: Double`: …" [comptime-instantiate].
//
// The contracts are `core.compare`'s: `cmp` is lexicographic in declaration
// order for a struct and by arm index then payload for a union; `eq` and
// `hash` walk the same fields, so they agree by construction
// [cmp-hash-values].

// ===== structs =====

// Lexicographic by field, in declaration order — so field order is part of
// the meaning, as it always was for the structural `cmp`.
export comptime fn cmp<T is Struct>(a: T, b: T) [] -> Int => a, b {
    [for field in T.fields] {
        let c = cmp(a.[field], b.[field])
        if c != 0 {
            return c
        }
    }
    return 0
}

export comptime fn eq<T is Struct>(a: T, b: T) [] -> Bool => a, b {
    [for field in T.fields] {
        if !eq(a.[field], b.[field]) {
            return false
        }
    }
    return true
}

// A `canbe Mut` struct is refused: a key that can change while a container
// holds it corrupts the container's lookup, so only an immutable struct may
// be one [col-hashed-ordered].
export comptime fn hash<T is Struct>(value: T) [] -> Long => value {
    [if T.mutable] {
        refuse!("a `Mut`-capable struct can change while a collection holds it, so it cannot be a key: drop `canbe Mut`, or hash by hand")
    }
    let h = 17L
    [for field in T.fields] {
        h = mix_hash(h, hash(value.[field]))
    }
    return h
}

// The literal's own shape, `Point { x: 1, y: 2 }` — identical on both
// backends, which is the whole reason the language fixes it rather than
// deferring to `Debug` or `toString` [interp-to-str].
export comptime fn to_str<T is Struct>(value: T) [] -> Str => value {
    let out: Mut Str = mut_str("${T.name} {")
    [for field in T.fields] {
        [if field.first] {
            append(out, " ")
        } else {
            append(out, ", ")
        }
        // Interpolated rather than `to_str(…)`: interpolation is what knows
        // how every type renders — the scalars natively, a float by Salvo's
        // rule [interp-float], a struct by its own `to_str` [interp-to-str].
        append(out, "${field.name}: ${value.[field]}")
    }
    append(out, " }")
    return out
}

// ===== unions =====

// Arms order by their declared position [union-arm-identity]; two values in
// the same arm order by the arm's own `cmp`.
export comptime fn cmp<T is Union>(a: T, b: T) [] -> Int => a, b {
    [when a] {
        [x] {
            [when b] {
                [y] {
                    [if x.index == y.index] {
                        // `None` has no `cmp`, and two of them tie.
                        [if x.type is None] {
                            return 0
                        } else {
                            return cmp(a, b)
                        }
                    } else {
                        return cmp(x.index, y.index)
                    }
                }
            }
        }
    }
    return 0
}

export comptime fn eq<T is Union>(a: T, b: T) [] -> Bool => a, b {
    [when a] {
        [x] {
            [when b] {
                [y] {
                    [if x.index == y.index] {
                        [if x.type is None] {
                            return true
                        } else {
                            return eq(a, b)
                        }
                    } else {
                        return false
                    }
                }
            }
        }
    }
    return false
}

export comptime fn hash<T is Union>(value: T) [] -> Long => value {
    [when value] {
        [arm] {
            [if arm.type is None] {
                return to_long(arm.index)
            } else {
                return mix_hash(to_long(arm.index), hash(value))
            }
        }
    }
    return 0L
}

export comptime fn to_str<T is Union>(value: T) [] -> Str => value {
    [when value] {
        [arm] {
            [if arm.type is None] {
                return "None"
            } else {
                // The arm's own `to_str`, by ordinary overload ranking (the
                // arm is more specific than the union).
                return to_str(value)
            }
        }
    }
    return ""
}
