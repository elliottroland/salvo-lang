import random.Random
import list.add
import list.ext.add as ext_add
import core.Str

external type LinkedList<T>

external fn chars(str: Str) -> Char[]

external fn complicated_func<T>(list: List<T>) -> Str

// [internal-fn] A compiler-intrinsic fn: declared for its signature and
// deduction; each backend lowers calls to it directly (no define files).
internal fn copy<T>(value: T) -> [value] T
