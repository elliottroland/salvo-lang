import random.Random
import list.add
import list.ext.add as ext_add
import core.Str

// [intrinsic-fn] A compiler-intrinsic fn: declared for its signature and
// deduction; each backend lowers calls to it directly from a table keyed by
// name.
intrinsic fn copy<T>(value: T) [] -> T => value

fn chars(str: Str) [] -> Char[] => str {
    return []
}

fn complicated_func<T>(list: List<T>) [] -> Str => list {
    return ""
}
