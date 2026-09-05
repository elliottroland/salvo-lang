fn max(...ints: Int[]) -> Int? {
    let max: Int? = None
    for i in ints {
        if max is None || i > max {
            max = i
        }
    }
    return max
}

fn max(first: Int, ...rest: Int[]) -> Int {
    let rest_max: Int? = max(rest)
    if rest_max is Int && rest_max > first {
        return rest_max
    }
    return first
}

fn map<S, T>(list: List<S>, mapper: (S) -> T) -> List<T> {
    let result: Mut List<T> = mutable_list()
    for s in list {
        add(result, mapper(s))
    }
    return result
}

fn to_string(int: Int) -> Str {
    return "${int}"
}

fn do_something() {
    let list: List<Int> = list(1, 2, 3)
    map(list, to_string)
    map(list, i -> "${i}")
    map(list, i -> { return "${i}" })
}

fn range(start: Int, end: Int) -> Iter<Int> {
    let i = start
    while i++ < end {
        yield i
    }
}

fn rangeIncl(start: Int, end: Int) -> Iter<Int> {
    if start > end {
        return
    }
    for i in range(start, end) {
        yield i
    }
    yield end
}

fn arrays() {
    let numbers: Int[] = [1, 2, 3]
    let generated: Int[] = Int[5] { i: Int -> 0 }
    let size = numbers.size()
    let first = numbers[0]
}

fn remove_first<T>(list: Mut NonEmpty List<T>) -> [list: Mut] T

fn consume<T>(list: List<T>) -> [] None

fn maybe_remove_first<T>(list: Mut NonEmpty List<T>) [Random<Int>] -> T? {
    if next_random() > 0 {
        return remove_first(list)
    }
    return None
}

fn generic_calls() [Random<Int>, Random<Double>] -> None {
    let int: Int = next_random()
    let double: Double = next_random()
    let number = next_random<Int>()
}

// [fn-effects] A fn type may declare the effects a call of the value
// performs; the function taking it inherits them.
fn run_it(f: (s: Str) [Console] -> [s] Str, value: Str) -> [value] Str {
    return f(value)
}

fn use_it() [Console] -> None {
    let shouted = run_it(s -> {
        println("shouting ${s}")
        return "${s}!"
    }, "hello")
    println(shouted)
}
