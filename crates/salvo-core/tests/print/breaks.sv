// [fmt-width] [fmt-author-breaks] Line breaking, each construct three ways:
// short on one line (it stays), short and broken by the author (it stays
// broken, one item per line), and too long for 120 columns (broken the
// canonical way). Parsed only (`print_tests.rs`), never checked.

struct Point {
    x: Int
    y: Int
}

type Short = Int | Str

type Broken = (
    Int
    | Str
)

// A union too long stays long: the width does not break one.
type Long = FirstLongArmName | SecondLongArmName | ThirdLongArmName | FourthLongArmName | FifthLongArmName | SixthLongArm

fn short(a: Int, b: Int) -> Int {
    return a + b
}

fn broken(
    a: Int,
    b: Int
) -> Int {
    return a + b
}

fn long_signature(first_parameter: Int, second_parameter: Str, third_parameter: List<Int>, fourth: Bool) -> Int
=> first_parameter {
    return first_parameter
}

fn long_parameters(
    first_parameter_of_many: Int,
    second_parameter_of_many: Str,
    third_parameter_of_many: List<Int>
) [Console] -> Int {
    return 0
}

fn deduced(xs: List<Int>) -> Int
=> xs {
    return 0
}

effect Store {
    fn get(key: Str) -> Int
    fn put(
        key: Str,
        value: Int
    ) -> None
    fn long_member(first_key_of_the_store: Str, second_key_of_the_store: Str, third_key_of_store: Str) -> Int
    => first_key_of_the_store
}

fn calls() {
    short_call(1, 2)
    broken_call(
        1,
        2
    )
    let result = a_function_with_a_long_name(the_first_argument, the_second_argument, the_third_argument_xx)
    let result = a_function_with_a_long_name(
        the_first_argument,
        the_second_argument,
        the_third_argument_xxx
    )
    let nested = outer_function(inner_function(first_argument_value, second_argument_value), third_value_xx)
    let nested = outer_function(
        inner_function(first_argument_value_long, second_argument_value_long, third_argument_value_long),
        fourth
    )
    let nested = outer_function(
        inner_function(
            first_argument_value_of_a_very_long_call,
            second_argument_value_of_a_very_long_call,
            third_argument
        ),
        fourth
    )
    let typed = convert<Int, Str>(1)
    let typed = convert<
        Int,
        Str
    >(1)
    let p = Point { x: 1, y: 2 }
    let p = Point {
        x: 1,
        y: 2
    }
    let p = Point {
        x: a_long_coordinate_computation(first_value),
        y: another_long_coordinate_computation(second_value)
    }
    let t = (1, 2)
    let t = (
        1,
        2
    )
    let f = (a: Int, b: Int) -> a + b
    let f = (
        a: Int,
        b: Int
    ) -> a + b
    let (left, right) = t
    let (
        left,
        right
    ) = t
    let { x, y } = p
    let {
        x,
        y
    } = p
    println(
        "a string longer than the width is never broken, and a list around it breaks only if that helps the line fit"
    )
    println("a string longer than the width on a line of its own too is never broken, and the call around it is not broken either")
}

fn chains(m: Int?, xs: List<Int>, ok: Bool, done: Bool) {
    let a = m ?: 0
    let a = m
        ?: 0
    let a = first_lookup_of_the_value(m)
        ?: second_lookup_of_the_value(m)
        ?: third_lookup_of_the_value(m)
        ?: 0
    let b = xs.map((x) -> x + 1).filter((x) -> x > 2)
    let b = xs
        .map((x) -> x + 1)
        .filter((x) -> x > 2)
    let b = xs
        .map((element) -> element + 1)
        .filter((element) -> element > 2)
        .fold(0, (total, element) -> total + element)
    let c = m |> f(_) ?> g(_)
    let c = m
        |> f(_)
        ?> g(_)
    let c = m
        |> first_transformation_of_the_value(_)
        |> second_transformation_of_the_value(_)
        ?> third(_)
    if ok && done {
        return
    }
    if ok
        && done {
        return
    }
    if first_condition_of_the_test(ok) && second_condition_of_the_test(done)
        || third_condition_of_the_test(ok) {
        return
    }
    let sum = 1 + 2 + 3
    let sum = (1
        + 2
        + 3)
    // A sum too long stays long: the width does not break one, nor what is in it.
    let sum = first_summand_of_the_total(xs) + second_summand_of_the_total(xs) + third_summand_of_the_total(xs) + 4
    let in_brackets = check(m is Int | Str)
    let in_brackets = check(m is Int
        | Str)
    total(1) replyto got(1)
    total(1)
        replyto got(1)
        replyto done()
}

handler Memory(size: Int, name: Str) for Store {
    fn get(key: Str) -> Int {
        return 0
    }
}

handler Paged(
    size: Int,
    name: Str
) for Store {
    fn get(key: Str) -> Int {
        return 0
    }
}

actor effect Counter {
    send fn add(amount: Int, note: Str)
    send fn take(
        amount: Int,
        note: Str
    )
}

fn more(m: Thing?, xs: List<Int>) {
    let a = m?.lookup(1, 2)
    let a = m?.lookup(
        1,
        2
    )
    let b = f(m?.first()?.second())
    let b = f(m
        ?.first()
        ?.second())
    let t: (Int, Str) = (1, "a")
    let t: (
        Int,
        Str
    ) = (1, "a")
    let r = xs
        .map((x) -> outer(
            x,
            1
        ))
        .first()
    total(1) replyto got(1, 2)
    total(1) replyto got(
        1,
        2
    )
}

comptime fn describe<T is Struct>(t: T) -> Str {
    let c = [describe@main(t, t)]
    let c = [describe@main(
        t,
        t
    )]
    return c
}
