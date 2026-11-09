// What a compile-time function may know about a type: its fields, its arms,
// their names, types and defaults.
//
// A `comptime fn` is code that runs over the *shape* of a type while the
// program is compiled, and is stamped out once per type that asks for it,
// as `by auto` does for `cmp` and `hash` ([std.core.auto]). Inside one, the
// bound type parameter `T` is described by the structs declared here: `T` is
// a [Struct], a [Union] or a [Tuple], `T.fields` is a list of [Field]s, and
// `[for field in T.fields]` unrolls its body once per field:
//
// ```
// comptime fn field_names<T is Struct>(value: T) [] -> Str => value {
//     let out: Mut Str = Mut Str("")
//     [for field in T.fields] {
//         [if !field.first] {
//             append@string(out, ", ")
//         }
//         append@string(out, field.name)
//     }
//     return out
// }
//
// struct Point { x: Int, y: Int }
//
// fn names(value: Point) [] -> Str by field_names    // answers "x, y"
// ```
//
// **Compile time only.** None of these types exists while the program runs.
// The compiler fills them in for the body of a `comptime fn`; nothing
// constructs one, and naming one in an ordinary type position is an error.
// They are declared here so that their members can be read, searched and
// shown on hover.
//
// **A type and its kind.** A binder's `.type` is the type itself: written
// in brackets, it can be used as a type inside the body
// (`let x: [field.type] = …`), and `field.type is Str` compares it. What
// *kind* of type it is (a struct, a union, a tuple…) belongs to the type, as
// its member `.kind`, a [Kind]: `field.type.kind is Struct`. The kind is a
// member apart from the type, so a program type that happens to be called
// `Union` is never mistaken for the kind. A `[when field.type.kind]` must
// handle every kind, so a kind added to the language is an error in each
// comptime fn that has not considered it.
//
// The Compile-time functions topic explains how a comptime fn is written and
// stamped.
// @compilerrules [comptime-fields, comptime-inline, comptime-kind, comptime-tuple]

// A struct type: its name, its fields in declaration order, and what it
// declares about itself.
// @compilerknown
export comptime struct Struct {
    // The declared name, as a `Str` literal in each copy (`T.name`).
    name: Str,
    // The fields, in declaration order: what `[for field in T.fields]`
    // unrolls.
    fields: List<Field>,
    // Whether the declaration says `canbe Mut`. The structural `hash` of
    // `std.core.auto` refuses a mutable struct.
    // @compilerrules [col-hashed-ordered]
    mutable: Bool,
    // The doc comment above the declaration, as text, or `None`. Tested with
    // `[if T.docs is None]`.
    // @compilerrules [comptime-docs]
    docs: Str?
}

// A named union type (`type Source = A | B | None`): its declared arms,
// `None` included, in declaration order.
// @compilerrules [type-union-arm-identity]
// @compilerknown
export comptime struct Union {
    // The declared name, as a `Str` literal in each copy.
    name: Str,
    // The arms: what `[for arm in T.arms]` unrolls, and what a
    // `[when value] { [arm] { … } }` dispatches over.
    arms: List<Arm>,
    // The doc comment above the declaration, or `None`.
    // @compilerrules [comptime-docs]
    docs: Str?
}

// A tuple type, such as `(Str, Int)`: its elements, in order.
//
// A tuple has no name and no declaration, so a comptime fn over one is
// stamped at a written tuple type (`fn show(t: (Str, Int)) -> Str by show`),
// and walks its elements by position with `[for index in T.indices]`:
//
// ```
// comptime fn show<T is Tuple>(t: T) [] -> Str => t {
//     let out: Mut Str = Mut Str("(")
//     [for index in T.indices] {
//         [if !index.first] {
//             append@string(out, ", ")
//         }
//         append@string(out, to_str@[site](t.[index]))
//     }
//     append@string(out, ")")
//     return out
// }
//
// fn show(t: (Str, Int)) [] -> Str by show           // ("a", 1) answers "(a, 1)"
// ```
// @compilerrules [comptime-tuple]
// @compilerknown
export comptime struct Tuple {
    // The element types, in order.
    elems: List<Type>,
    // The element positions, in order: what `[for index in T.indices]`
    // unrolls.
    indices: List<Index>
}

// A function type. A comptime fn can call a value of one, but not look
// inside it.
// @compilerknown
export comptime struct FnType {}

// A type declared `intrinsic type`, such as `Int`, `Str` or `List<T>`:
// there is nothing to look inside, so a comptime fn calls the functions it
// needs at it.
// @compilerknown
export comptime struct Basic {
    // The type's name.
    name: Str
}

// A type parameter of the struct being stamped, such as the `T` of
// `Wrapper<T>`: opaque to the comptime fn, like a [Basic].
// @compilerknown
export comptime struct Generic {
    // The type parameter's name.
    name: Str
}

// The two kinds a comptime fn cannot look inside, for when it does not care
// which: `is Opaque` matches a [Basic] or a [Generic].
// @compilerknown
export comptime type Opaque = Basic | Generic

// Every kind of type: what a [Type]'s `.kind` is, as `field.type.kind`
// reads it for a field's type.
//
// A `[when field.type.kind]` must handle every arm, with no `else`, so a kind
// added to the language is an error in every comptime fn that did not
// consider it, rather than a case silently skipped.
//
// ```
// comptime fn kinds<T is Struct>(value: T) [] -> Str => value {
//     let out: Mut Str = Mut Str("")
//     [for field in T.fields] {
//         [when field.type.kind] {
//             is Struct { append@string(out, "s") }
//             is Union { append@string(out, "u") }
//             is Tuple { append@string(out, "t") }
//             is FnType { append@string(out, "f") }
//             is Opaque { append@string(out, "o") }
//         }
//     }
//     return out
// }
//
// struct Row { id: Int, tags: List<Str> }
//
// fn row_kinds(value: Row) [] -> Str by kinds         // answers "oo"
// ```
// @compilerrules [when-exhaustive]
// @compilerknown
export comptime type Kind = Struct | Union | Tuple | FnType | Opaque

// A type itself, as a binder's `.type` holds it.
//
// Written in brackets it is usable as a type inside the body
// (`[field.type]`), and `field.type is Str` compares it; what kind of type it
// is, is its `.kind`. The members of a literal union (`"json" | "text"`) are
// `.literals`, a list of [Literal]s that `[for lit in field.type.literals]`
// walks.
// @compilerrules [comptime-literals]
// @compilerknown
export comptime struct Type {
    // What kind of type it is, a [Kind]: tested in a comptime head
    // (`[if field.type.kind is Struct]`) or dispatched on with an exhaustive
    // `[when field.type.kind]`, and never read as a value.
    // @compilerrules [comptime-kind]
    kind: Kind
}

// One member of a literal union, such as `"json"` of `"json" | "text"`, as
// `[for lit in field.type.literals]` walks them in declaration order.
// @compilerrules [comptime-literals]
// @compilerknown
export comptime struct Literal {
    // The member itself, a literal of the union in each copy.
    value: Value,
    // Its text: a literal of the union of the members' texts.
    name: Str,
    // Its position among the members, from zero.
    index: Int,
    // Whether it is the first member.
    first: Bool,
    // Whether it is the last member.
    last: Bool
}

// A value written in a declaration, as a field's `.default` holds it: in each
// copy it is that expression, evaluated where it was declared, so a default
// that calls a private function of its module still works.
// @compilerrules [comptime-default]
// @compilerknown
export comptime struct Value {}

// One field of a [Struct], as `[for field in T.fields]` binds it.
// @compilerknown
export comptime struct Field {
    // The declared name: a literal of the union of the struct's field names
    // (`"x" | "y"` for `Point`), tested with `field.name is "x"` or an
    // exhaustive `[when field.name]`, and a `Str` literal in each copy.
    // @compilerrules [comptime-name]
    name: Str,
    // The declared type; usable as a type inside the body as `[field.type]`,
    // and what kind of type it is as `field.type.kind`.
    type: Type,
    // The declared position, from zero.
    index: Int,
    // Whether it is the first field.
    first: Bool,
    // Whether it is the last field.
    last: Bool,
    // The declared default (`port: Int = 8080`), or `None` when the field
    // has none: read as an expression in a copy (`field.default`), evaluated
    // where the struct is declared, and tested with
    // `[if field.default is None]`. A declared `= None` is a default.
    // @compilerrules [comptime-default]
    default: Value?,
    // The doc comment above the field, its lines joined, or `None`: a `Str`
    // literal in a copy that has one, tested with `[if field.docs is None]`.
    // @compilerrules [comptime-docs]
    docs: Str?
}

// One element position of a [Tuple], as `[for index in T.indices]` binds it.
//
// `t.[index]` reads the element at this position of a tuple value `t`: in the
// copy for the second element it is `t.1`. An element has no name, so an
// `Index` has none either.
// @compilerrules [comptime-tuple]
// @compilerknown
export comptime struct Index {
    // The position, from zero: `1` in the copy that reads `t.1`.
    index: Int,
    // Whether it is the first element.
    first: Bool,
    // Whether it is the last element.
    last: Bool,
    // The element's type; usable as a type inside the body as `[index.type]`,
    // and what kind of type it is as `index.type.kind`.
    type: Type
}

// One arm of a [Union], as `[for arm in T.arms]` or a `[when value]` arm
// binds it.
// @compilerknown
export comptime struct Arm {
    // The struct's name for a struct arm, otherwise the type's text (`None`):
    // a literal of the union of the arm names.
    name: Str,
    // The arm's type; usable as a type inside the body as `[arm.type]`, and
    // what kind of type it is as `arm.type.kind`.
    type: Type,
    // The declared position, from zero. Arms order by it in the structural
    // `cmp`.
    index: Int,
    // Whether it is the first arm.
    first: Bool,
    // Whether it is the last arm.
    last: Bool,
    // The doc comment of the declaration the arm names (a struct, or a
    // `type`), or `None`.
    // @compilerrules [comptime-docs]
    docs: Str?
}

// ---- compile-time functions
//
// A comptime head (`[if …]`, `[for … in …]`, `[let … = …]`) is a
// compile-time expression over the model above, and the functions it calls
// are these: the compiler evaluates them while it stamps, and they exist
// nowhere else. A head sees no other function, so an ordinary `size` or
// `filter` in scope is not what a head means. Evaluation always ends: a head
// has no loops, and each of these walks a list the compiler built.
//
// ```
// export comptime qualifier Key for Field
//
// comptime fn key_name<T is Struct>(value: T) [] -> Str => value {
//     [let keys = filter(T.fields, (f) -> f is Key)]
//     [if size(keys) != 1] {
//         refuse!("`${T.name}` marks ${size(keys)} fields as `Key`; mark exactly one")
//     }
//     [for f in keys] ...
//     return f.name
// }
//
// struct Order { Key id: Str, qty: Int }
//
// fn order_key(value: Order) [] -> Str by key_name    // answers "id"
// ```

// The number of elements of [xs].
// @compilerrules [comptime-eval-fns]
export comptime intrinsic fn size<T>(xs: List<T>) [] -> Int => xs

// The element of [xs] at index [i], from zero, or `None` past its end.
// @compilerrules [comptime-eval-fns]
export comptime intrinsic fn get<T>(xs: List<T>, i: Int) [] -> T? => xs, i

// The elements of [xs] for which [keep] answers true, in order.
// @compilerrules [comptime-eval-fns]
export comptime intrinsic fn filter<T>(xs: List<T>, keep: (x: T) -> Bool) [] -> List<T> => xs, keep

// How many elements of [xs] [keep] answers true for: `size(filter(xs, keep))`.
// @compilerrules [comptime-eval-fns]
export comptime intrinsic fn count<T>(xs: List<T>, keep: (x: T) -> Bool) [] -> Int => xs, keep

// Whether [test] answers true for some element of [xs]; false for an empty
// list.
// @compilerrules [comptime-eval-fns]
export comptime intrinsic fn any<T>(xs: List<T>, test: (x: T) -> Bool) [] -> Bool => xs, test

// Whether [test] answers true for every element of [xs]; true for an empty
// list.
// @compilerrules [comptime-eval-fns]
export comptime intrinsic fn all<T>(xs: List<T>, test: (x: T) -> Bool) [] -> Bool => xs, test
