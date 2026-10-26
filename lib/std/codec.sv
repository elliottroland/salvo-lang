// `codec`: Salvo's canonical encoding of a value as bytes [wire-format] —
// what actors send across the network (`net`), what a stream's
// `write_value`/`read_value` write and read (`stream`), and what a program
// may use for anything else that stores or sends values.

// [wire-format] The canonical encoding, as a value: every type has a wire
// form unless it is (or holds) something declared `noremote` [noremote], and
// the compiler generates the codec for both backends — so the bytes `encode`
// answers on a Kotlin node are the bytes a Rust node's `decode` reads. What
// the frames of step ③ carry.
//
// Refused at the call, naming the field or declaration that stops it, for a
// type with no wire form; a generic `T` is refused too until the
// instantiation is known.
export intrinsic fn encode<T>(value: T) [] -> Bytes => !value

// [wire-format] The inverse: `None` when the bytes are not a well-formed
// encoding of `T` — truncated, or a union tag out of range. Written as
// `decode<Point>(data)`, since nothing but the type argument says what to
// read.
export intrinsic fn decode<T>(data: Bytes) [] -> T? => data
