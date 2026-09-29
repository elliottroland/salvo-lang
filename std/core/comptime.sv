// What a `comptime fn` may know about a type — declared here so the surface
// is readable, greppable and hoverable, and so a `[when field.type]` is an
// ordinary `when` over a union rather than a rule of its own (user decisions
// 2026-09-29, comptime round 8).
//
// [comptime-fields] These types exist at **compile time only**. The compiler
// fills them for the body of a `comptime fn` — `T` is a `Struct` or a `Union`
// by its bound, `T.fields` is its `List<Field>`, a `[for field in T.fields]`
// binder is a `Field` — and nothing constructs one or holds one as a value; a
// `comptime struct` in an ordinary type position is refused where it is
// written. Their shape is the compiler's: this module mirrors it, and a test
// keeps the two in step.
//
// A compile-time `Type` is also usable **as a type** wherever one is written
// inside the body: `let x: field.type = …`, `of_json<field.type>(…)`.

// A struct: its declared fields, in declaration order.
export comptime struct Struct {
    // The declared name, as a `Str` literal in each copy (`T.name`).
    name: Str,
    // The fields, in declaration order — what `[for field in T.fields]` unrolls.
    fields: List<Field>,
    // Whether the declaration says `canbe Mut`. A structural `hash` refuses a
    // mutable struct [col-hashed-ordered].
    mutable: Bool
}

// A named union (`type Source = A | B | None`): its declared arms, `None`
// included, in declaration order [union-arm-identity].
export comptime struct Union {
    name: Str,
    // The arms — what `[for arm in T.arms]` unrolls, and what a
    // `[when value] { [arm] { … } }` dispatches over.
    arms: List<Arm>
}

// A tuple type: its elements, in order.
export comptime struct Tuple {
    elems: List<Type>
}

// A function type. A `comptime fn` can call one, not look inside it.
export comptime struct FnType {}

// A type declared `intrinsic type` (`Int`, `Str`, `List<T>`): nothing to look
// inside; the comptime fn calls the capability at it.
export comptime struct Basic {
    name: Str
}

// A type parameter of the struct being stamped (`Wrapper<T>`'s `T`): opaque
// to the comptime fn, like a `Basic`.
export comptime struct Generic {
    name: Str
}

// The two kinds a comptime fn cannot look inside, when it does not care which.
export comptime type Opaque = Basic | Generic

// Every kind. A `[when field.type] { is Struct { … } … }` is exhaustive over
// these arms unless an `else` closes it [when-exhaustive], so a kind added to
// the language is an error in every comptime fn that did not consider it.
export comptime type Type = Struct | Union | Tuple | FnType | Opaque

// One field of a `Struct`.
export comptime struct Field {
    // The declared name, as a `Str` literal in each copy.
    name: Str,
    // The declared type; usable as a type inside the body.
    type: Type,
    // The declared position, from zero.
    index: Int,
    first: Bool,
    last: Bool
}

// One arm of a `Union`. `name` is the struct's name for a struct arm, the
// type's text otherwise (`None`).
export comptime struct Arm {
    name: Str,
    type: Type,
    index: Int,
    first: Bool,
    last: Bool
}
