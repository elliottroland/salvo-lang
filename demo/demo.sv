import random.Random
import random.DefaultRandom

fn main() [use] {
    use StdOutConsole
    // use DefaultRandom
    use CyclicRandom([1.2, 5.7, 2.4])

    let file = get_file()

    let list = mutable_list(1,2,3)
    list.add(3)
    let num: Int = list.remove_first()
    if list is NonEmpty {
        let num2: Int = list.remove_first()
    }

    println(list)

    let i = 21
    if i is Positive {
        println(repeat(i, 100))
    }

    random_number()

    let v = maybe_fail()
    when v {
        is Ok { println("it was ok: ${v}") }
        is Err { println("it was an err: ${v}") }
    }

    if i is Positive {
        let generated = repeat(i, () -> random_int())
    }

    close(file)

    let result = try {
        if random() < 0.5 {
            maybe_throw()
        } else {
            maybe_throw2()
        }
    }
}

fn repeat(n: Positive Int, gen: () [Random] -> Int) -> NonEmpty List<Int> {
    let list = mutable_list<Int>()
    let i = 0
    while i++ < n {
        list.add(copy(value))
    }
    return list
}

fn get_file() [Console] -> FileStream {
    let file = FileStream { name: "some file" }
    return file
}

fn maybe_throw() [Random, Throw<Str>] -> Int {
    if random() < 0.5 {
        throw("Hello world")
    }
    return -1
}

fn maybe_throw2() [Random, Throw<Int>] -> Str {
    if random() < 0.5 {
        throw(10)
    }
    return ""
}

fn maybe_fail() [Random] -> Ok Mut Str | Err Int {
    if random() < 0.5 {
        let s: Mut Str = mutable_str("it worked!")
        return ok(s)
    }
    return err(1)
}

fn println(list: List<Int>) [Console] {
    print("[")
    for i in list {
        print("${i},")
    }
    println("]")
}

fn repeat(n: Positive Int, value: Int) -> NonEmpty List<Int> {
    let list: Mut List<Int> = mutable_list()
    let i = 0
    while i++ < n {
        list.add(copy(value))
    }
    return list
}

qualifier NonEmpty<T> of List<T> {
    fn qualifies(list: List<T>) -> Bool {
        return list.size() > 0
    }

    // Refinements allow us to update deductions without having to run the
    // `qualifies` function each time.
    refn add(list: Mut List<T>, elem: T) -> [list: +NonEmpty]
}

fn remove_first<T>(list: Mut List<T>) -> T? {
    return list.get(0)
}

fn remove_first<T>(list: NonEmpty Mut List<T>) -> [list: Mut] T {
    return list.get(0)!
}

qualifier Positive of Int {
    fn qualifies(int: Int) -> Bool {
        return int > 0
    }
}

handler CyclicRandom(arr: Double[]) of Random {
    i: Int = 0

    fn random() -> [] Double {
        return arr[i++ % size(arr)]
    }
}

fn random_number() [Random, Console] -> Double {
    println("I'm about to generate a random number")
    return random()
}

struct FileStream : Linear<self> {
    name: Str
}

fn close(fs: FileStream) [Console] -> [] None {
    println("Closing file: ${fs.name}")
}

fn random_int() [Random] -> Int {}