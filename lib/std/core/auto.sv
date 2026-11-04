// The structural `cmp`, `eq`, `hash` and `to_str` that `by auto` writes for
// a struct or a named union: compare, digest or print a value by its fields,
// in declaration order.
//
// A type asks for them on the obligations it declares, and gets ordinary
// functions declared on the type, exactly as if it had written them:
//
// ```
// struct Point : Ordered<self> by auto, Hashed<self> by auto, ToStr<self> by auto {
//     x: Int,
//     y: Int
// }
//
// fn main() [use] -> None {
//     use StdConsole()
//     let p = Point { x: 1, y: 2 }
//     let q = Point { x: 1, y: 3 }
//     println("${p < q} ${p == q}")                    // true false
//     println("${p}")                                  // Point { x: 1, y: 2 }
// }
// ```
//
// `: Ordered<self> by auto` writes `cmp`, `: Eq<self> by auto` writes `eq`,
// `: Hashed<self> by auto` writes `hash` and `eq`, and `: ToStr<self> by
// auto` writes `to_str`. `auto` is this module, which every file can name
// without an import. Nothing is written unless a type asks: a struct with
// no `cmp` cannot be compared, and one with no `to_str` does not
// interpolate.
//
// **One function at a time.** `by auto` also works on a single declaration,
// which is how a type mixes written and hand-written members. The
// declaration names its parameters as the group member does (`a` and `b`
// for `cmp` and `eq`, `value` for `hash` and `to_str`):
//
// ```
// struct Version : Eq<self> by auto {
//     major: Int,
//     minor: Int
//
//     fn cmp(a: Version, b: Version) -> Int by auto
// }
//
// fn to_str(v: Version) [] -> Str => v {
//     return "${v.major}.${v.minor}"
// }
// ```
//
// **What the functions mean.** For a struct, `cmp` is lexicographic in field
// declaration order, so the order of the fields is part of the meaning;
// `eq` and `hash` walk the same fields, so the three agree, as
// [std.core.compare] requires. For a union, values in different arms order
// by the arms' declared positions, and values in the same arm by that arm's
// own function. `to_str` prints a struct in the shape of its literal, the
// same text on both backends.
//
// **Where errors land.** Each function calls the same function on every
// field, so every field's type needs one. A field without it is reported at
// that field, when the type is declared:
// "in `hash` from `auto` for `Reading.value: Double`: no matching overload
// for `hash(Double)`". A `canbe Mut` struct is refused a `hash` outright: a
// key that can change while a collection holds it corrupts the collection.
//
// How a compile-time function like these is written is the Compile-time
// functions topic; what their bodies may know about a type is
// [std.core.comptime].
// @compilerrules [comptime-bound, obligation-by, fn-attached, fn-by, comptime-instantiate, cmp-hash-values]

// ===== structs =====

// Orders two struct values field by field, in declaration order: the first
// field that differs decides, and two values whose fields all tie are
// equal. Each field is compared with its own type's `cmp`.
export comptime fn cmp<T is Struct>(a: T, b: T) [] -> Int => a, b {
    [for field in T.fields] {
        let c = cmp@[site](a.[field], b.[field])
        if c != 0 {
            return c
        }
    }
    return 0
}

// Whether two struct values are equal: every field equal, by its own type's
// `eq`, checked in declaration order and stopping at the first that
// differs.
export comptime fn eq<T is Struct>(a: T, b: T) [] -> Bool => a, b {
    [for field in T.fields] {
        if !eq@[site](a.[field], b.[field]) {
            return false
        }
    }
    return true
}

// The hash of a struct value, folding its fields' own hashes in declaration
// order with `mix_hash`.
//
// A `canbe Mut` struct is refused: a key that can change while a container
// holds it corrupts the container's lookup, so only an immutable struct may
// be one. Write the `hash` by hand if you need one anyway.
// @compilerrules [col-hashed-ordered]
export comptime fn hash<T is Struct>(value: T) [] -> Long => value {
    [if T.mutable] {
        refuse!("a `Mut`-capable struct can change while a collection holds it, so it cannot be a key: drop `canbe Mut`, or hash by hand")
    }
    let h = 17L
    [for field in T.fields] {
        h = mix_hash@compare(h, hash@[site](value.[field]))
    }
    return h
}

// The text of a struct value in the shape of its literal,
// `Point { x: 1, y: 2 }`, identical on both backends.
//
// Each field is interpolated, so it renders as `"${value}"` would render it:
// the basic types natively, a nested struct by its own `to_str` (which it
// must have). The language fixes the format, rather than leaving it to each
// backend's own debug text, so that a program prints the same everywhere.
// @compilerrules [interp-to-str]
export comptime fn to_str<T is Struct>(value: T) [] -> Str => value {
    let out: Mut Str = Mut Str("${T.name} {")
    [for field in T.fields] {
        [if field.first] {
            append@string(out, " ")
        } else {
            append@string(out, ", ")
        }
        // Interpolated rather than `to_str(…)`: interpolation is what knows
        // how every type renders — the scalars natively, a float by Salvo's
        // rule [interp-float], a struct by its own `to_str` [interp-to-str].
        append@string(out, "${field.name}: ${value.[field]}")
    }
    append@string(out, " }")
    return out
}

// ===== unions =====

// Orders two values of a named union: by the position of their arms in the
// declaration, and two values in the same arm by that arm's own `cmp`. Two
// `None`s tie.
//
// ```
// struct Manual : Ordered<self> by auto, Hashed<self> by auto { note: Str }
// struct Imported : Ordered<self> by auto, Hashed<self> by auto { feed: Str, n: Int }
// type Source = Manual | Imported : Ordered<self> by auto, Hashed<self> by auto
// ```
//
// Each arm needs a `cmp` of its own (and an `eq` and a `hash` for
// `Hashed`), here also from `by auto`.
// @compilerrules [type-union-arm-identity]
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
                            return cmp@[site](a, b)
                        }
                    } else {
                        return cmp@basic(x.index, y.index)
                    }
                }
            }
        }
    }
}

// Whether two values of a named union are equal: in the same arm, and equal
// by that arm's own `eq`. Two `None`s are equal.
export comptime fn eq<T is Union>(a: T, b: T) [] -> Bool => a, b {
    [when a] {
        [x] {
            [when b] {
                [y] {
                    [if x.index == y.index] {
                        [if x.type is None] {
                            return true
                        } else {
                            return eq@[site](a, b)
                        }
                    } else {
                        return false
                    }
                }
            }
        }
    }
}

// The hash of a value of a named union: its arm's position, mixed with the
// arm's own hash of the value.
export comptime fn hash<T is Union>(value: T) [] -> Long => value {
    [when value] {
        [arm] {
            [if arm.type is None] {
                return to_long@basic(arm.index)
            } else {
                return mix_hash@compare(to_long@basic(arm.index), hash@[site](value))
            }
        }
    }
}

// The text of a value of a named union: the text of the arm it holds, by
// that arm's own `to_str`, or `None` for the `None` arm.
export comptime fn to_str<T is Union>(value: T) [] -> Str => value {
    [when value] {
        [arm] {
            [if arm.type is None] {
                return "None"
            } else {
                // The arm's own `to_str`, as the site sees it, by ordinary
                // overload ranking (the arm is more specific than the union).
                return to_str@[site](value)
            }
        }
    }
}
