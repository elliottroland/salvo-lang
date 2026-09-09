intrinsic type Byte
intrinsic type Int
intrinsic type Long
intrinsic type Float
intrinsic type Double
intrinsic type Char
intrinsic type Bool
intrinsic type Any
intrinsic type Nothing

type Number = Int | Long | Double | Float

// [intrinsic-fn] [copy-fn] Duplicates a value: the argument is kept
// untouched (with all its qualifiers) and the result is a fresh,
// independent value with no fate links to the source. Implemented by
// each backend directly (Kotlin: identity for transitively immutable
// types, a real copy for Mut-capable ones; Rust: `.clone()`).
intrinsic fn copy<T>(value: T) [] -> [value] T

// [linear-group] [group-obligation] What **linear** means: a type declares
// `: Linear<self>` and supplies the `close` that discharges the obligation.
// Declaring it without one is an error at the *struct* — the compiler knows
// which function to look for, because the group is designated, at most one per
// type, and has a single member.
//
// `close` **consumes**: the empty deduction list moves its parameter, so the
// value is not kept. That is what makes it the discharge rather than a mere
// convention, and why `discard` no longer satisfies a linear obligation
// [linear-discard] — dropping a handle is exactly the leak the obligation
// exists to prevent.
params Linear<It> {
    fn close(it: It) -> [] None
}

// [intrinsic-fn] [linear-discard] Deliberately drops a value, consuming
// it. **Not** an escape hatch from linearity: a linear value's discharge is
// its own `close`, and `discard` on one is refused, naming it.
intrinsic fn discard<T canbe Linear>(value: T) [] -> [] None
