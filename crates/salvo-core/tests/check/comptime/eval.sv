// [comptime-eval] A head is a compile-time expression over the model:
// `[let]`, `[for x in <list>]`, and the intrinsics of `std.core.comptime`
// [comptime-eval-fns].
export comptime qualifier Key for Field

struct Order { Key id: Str, qty: Int, note: Str }
struct TwoKeys { Key a: Str, Key b: Int }
type Shape = Order | TwoKeys

comptime fn key_name<T is Struct>(value: T) [] -> Str => value {
    [let keys = filter(T.fields, (f) -> f is Key)]
    [if size(keys) != 1] {
        refuse!("`${T.name}` marks ${size(keys)} fields as `Key`; mark exactly one")
    }
    [for f in keys] ...
    return f.name
}

fn order_key(value: Order) [] -> Str by key_name
//~ error[comptime-refuse:refused]
fn two_key(value: TwoKeys) [] -> Str by key_name

// Every intrinsic, and the operators.
comptime fn counts() [] -> Int {
    let total = 0
    [let fields = Order.fields]
    [let n = count(fields, (f) -> f.type is Str)]
    [if n == 2 && size(fields) - 1 == n && any(fields, (f) -> f is Key) && !all(fields, (f) -> f is Key)] {
        total = n * 10 + 2 % 2
    }
    return total
}

// `get` answers an element, or `None` past the end.
comptime fn second() [] -> Str {
    let out = ""
    [let f = get(Order.fields, 1)]
    [if get(Order.fields, 9) is None] {
        out = f.name
    }
    return out
}

// A walk over a filtered list counts within it.
comptime fn strs() [] -> Int {
    let total = 0
    [for f in filter(Order.fields, (x) -> x.type is Str)] {
        [if f.last] {
            total = total + f.index
        }
    }
    return total
}

// The arms of a union.
comptime fn arm_count() [] -> Int {
    [let k = size(Shape.arms)]
    return k
}

// Mistakes.
comptime fn mistakes() [] -> Int {
    //~ error[comptime-eval:runtime-fn]
    [if println("x")] { }
    //~ error[comptime-eval:unknown-fn]
    [if nothing(Order.fields)] { }
    //~ error[comptime-eval:arity]
    [if size(Order.fields, 1) == 0] { }
    //~ error[comptime-eval:type]
    [if size(Order.fields)] { }
    //~ error[comptime-eval:compare]
    [if Order.fields == 1] { }
    //~ error[comptime-eval:arith]
    [if 1 + "a" == 2] { }
    //~ error[comptime-eval:overflow]
    [if 1 / 0 == 2] { }
    //~ error[comptime-eval:not-walkable]
    [for x in size(Order.fields)] { }
    //~ error[comptime-eval:lambda-arity]
    [if any(Order.fields, (a, b) -> true)] { }
    //~ error[comptime-eval:no-property]
    [if Order.fields.name == "x"] { }
    //~ error[comptime-eval:bad-is]
    [if size(Order.fields) is Int] { }
    [let m = Order.fields]
    //~ error[comptime-eval:let-shadow]
    [let m = 2]
    //~ error[comptime-eval:let-in-body]
    let _x = m
    return 0
}

comptime fn nested() [] -> Int {
    if true {
        //~ error[comptime-eval:let-nested]
        [let k = 1]
    }
    return 0
}

fn main() {
    let _a = order_key(Order { id: "a", qty: 1, note: "" })
    let _b = counts()
    let _c = second()
    let _d = strs()
    let _e = arm_count()
    let _f = mistakes()
    let _g = nested()
}

//--- intrinsic_shape
//~ error[comptime-eval-fns:intrinsic-shape]
comptime intrinsic fn size2<T>(xs: List<T>) [] -> Int => xs {
    return 0
}

fn main() {}
