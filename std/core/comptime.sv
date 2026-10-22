// What a `comptime fn` may know about a type — declared here so the surface
// is readable, greppable and hoverable, and so a `[when field.kind]` is an
// ordinary `when` over a union rather than a rule of its own (user decisions
// 2026-09-29, comptime round 8; the kind split from the type 2026-10-20).
//
// [comptime-fields] These types exist at **compile time only**. The compiler
// fills them for the body of a `comptime fn` — `T` is a `Struct` or a `Union`
// by its bound, `T.fields` is its `List<Field>`, a `[for field in T.fields]`
// binder is a `Field` — and nothing constructs one or holds one as a value; a
// `comptime struct` in an ordinary type position is refused where it is
// written. Their shape is the compiler's: this module mirrors it, and a test
// keeps the two in step.
//
// A binder's `.type` is the type itself, spliced **as a type** in brackets
// wherever one is written inside the body (`let x: [field.type] = …`,
// `of_json<[field.type]>(…)`) and compared with `field.type is Str`
// [comptime-inline]. What kind of type it is — a struct, a
// union, … — is a separate projection, `.kind`, so a program type that happens
// to be called `Union` is never mistaken for the kind [comptime-kind].

// A struct: its declared fields, in declaration order.
export comptime struct Struct {
    // The declared name, as a `Str` literal in each copy (`T.name`).
    name: Str,
    // The fields, in declaration order — what `[for field in T.fields]` unrolls.
    fields: List<Field>,
    // Whether the declaration says `canbe Mut`. A structural `hash` refuses a
    // mutable struct [col-hashed-ordered].
    mutable: Bool,
    // The doc comment above the declaration, as text, or `None` (`T.docs`,
    // tested with `[if T.docs is None]`) [comptime-docs].
    docs: Str?
}

// A named union (`type Source = A | B | None`): its declared arms, `None`
// included, in declaration order [union-arm-identity].
export comptime struct Union {
    name: Str,
    // The arms — what `[for arm in T.arms]` unrolls, and what a
    // `[when value] { [arm] { … } }` dispatches over.
    arms: List<Arm>,
    // The doc comment above the declaration, or `None` [comptime-docs].
    docs: Str?
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

// Every kind — what `.kind` is. A `[when field.kind] { is Struct { … } … }` is
// exhaustive over these arms, with no `else` [when-exhaustive], so
// a kind added to the language is an error in every comptime fn that did not
// consider it.
export comptime type Kind = Struct | Union | Tuple | FnType | Opaque

// A type itself, as a binder's `.type` holds it: usable as a type inside the
// body, compared with `is <Type>`; its description is the binder's `.kind`.
export comptime struct Type {}

// A value a declaration writes, as a field's `.default` holds it: in a copy it
// is that expression, evaluated where it was declared [comptime-default].
export comptime struct Value {}

// One field of a `Struct`.
export comptime struct Field {
    // The declared name: a literal of the union of the struct's field names
    // (`"x" | "y"` for `Point`), tested with `field.name is "x"` or an
    // exhaustive `[when field.name]`, and a `Str` literal in each copy
    // [comptime-name].
    name: Str,
    // The declared type's kind [comptime-kind].
    kind: Kind,
    // The declared type; usable as a type inside the body.
    type: Type,
    // The declared position, from zero.
    index: Int,
    first: Bool,
    last: Bool,
    // The declared default (`port: Int = 8080`), or `None` when the field has
    // none: read as an expression in a copy (`field.default`), evaluated
    // where the struct is declared, and tested with `[if field.default is
    // None]` [comptime-default]. A declared `= None` is a default.
    default: Value?,
    // The doc comment above the field, its lines joined, or `None`: a `Str`
    // literal in a copy that has one, tested with `[if field.docs is None]`
    // [comptime-docs].
    docs: Str?
}

// One arm of a `Union`. `name` is the struct's name for a struct arm, the
// type's text otherwise (`None`) — a literal of the union of the arm names.
export comptime struct Arm {
    name: Str,
    kind: Kind,
    type: Type,
    index: Int,
    first: Bool,
    last: Bool,
    // The doc comment of the declaration the arm names (a struct, or a
    // `type`), or `None` [comptime-docs].
    docs: Str?
}
