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

fn transform<S, T>(list: List<S>, mapper: (S) -> T) -> List<T> {
    let result: Mut List<T> = mut_list_of()
    for s in list {
        add(result, mapper(s))
    }
    return result
}

fn to_string(int: Int) -> Str {
    return "${int}"
}

fn do_something() {
    let list: List<Int> = list_of(1, 2, 3)
    transform(list, to_string)
    transform(list, i -> "${i}")
    transform(list, i -> { return "${i}" })
}

// A pass is an ordinary struct with an ordinary `next` [iter-protocol] —
// or, when it needs no name, an `iter fn` whose pass struct the compiler
// writes [iter-fn].
struct Range {
    start: Int,
    end: Int
}

fn range(start: Int, end: Int) -> Range {
    return Range {start: start, end: end}
}

iter fn next(range: Range) -> Emitted Int | Finished {
    state {
        at: Int = range.start
    }
    if at >= range.end {
        return finished()
    }
    let v = copy(at)
    at = at + 1
    return emitted(v)
}

// The written-out form, for a pass a program has to name.
struct Countdown : Yield<self, Int> canbe Mut {
    at: Int
}

fn countdown(from: Int) -> Mut Countdown {
    return Mut Countdown {at: from}
}

fn next(p: Mut Countdown) -> Emitted Int | Finished => p: Mut {
    if p.at <= 0 {
        return finished()
    }
    let now = copy(p.at)
    p.at = p.at - 1
    return emitted(now)
}

fn arrays() {
    let numbers: Int[] = array_of(1, 2, 3)
    let generated: Int[] = array_by(5, i -> 0)
    let size = numbers.size()
    let first = numbers[0]
}

fn remove_first<T>(list: Mut NonEmpty List<T>) -> T => list: Mut {
    return list.get(0)!
}

fn consume<T>(list: List<T>) -> None {
}

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
fn run_it(f: (s: Str) [Console] -> Str, value: Str) -> Str =>[f] s => value {
    return f(value)
}

fn use_it() [Console] -> None {
    let shouted = run_it(s -> {
        println("shouting ${s}")
        return "${s}!"
    }, "hello")
    println(shouted)
}

// [implicit-group] A named bundle of implicit parameters, spread with `?`.
params Field<T> {
    fn add(a: T, b: T) -> T
    fn zero() -> T
}

// [implicit-param] `?cmp` is resolved at the call site by name and type;
// `?Field<T>` spreads the group's members as implicit parameters of their
// own, with no binder — they are called unqualified here and overridden by
// their own names at the call.
fn total<T>(xs: List<T>, ?Field<T>) -> T => xs {
    let acc = zero()
    for x in xs {
        acc = add(acc, x)
    }
    return acc
}

fn ordered<T>(list: List<T>, ?cmp: (T, T) -> Int) -> List<T> => list {
    return list
}

// [implicit-override] The caller supplies one implicit by name; the rest
// still resolve.
fn totals() [Console] -> None {
    println("${total(list_of(1, 2, 3))}")
    println("${total(list_of(2, 3, 4), add = times)}")
}
