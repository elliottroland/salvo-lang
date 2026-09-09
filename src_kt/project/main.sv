import random.Random
import random.DefaultRandom

qualifier NonEmpty<T> of List<T> {
    fn qualifies(list: List<T>) -> Bool {
        return list.size() > 0
    }

    // Refinements allow us to update deductions without having to run the
    // `qualifies` function each time.
    refn add(list: Mut List<T>, elem: T) -> [list: +NonEmpty]
}

fn remove_first<T>(list: List<T>) -> T? {
    return list.get(0)
}

fn remove_first<T>(list: NonEmpty List<T>) -> T {
    return list.get(0)!
}

provenance qualifier ThreadId of Int

fn thread_id(int: Int) -> Int as ThreadId {
    return int
}

platform effect ThreadEff {
    fn spawn(block: () [Console] -> [] None) -> [] ThreadId Int
    fn join(id: ThreadId Int) -> [] None
}

qualifier Positive of Int {
    fn qualifies(int: Int) -> Bool {
        return int > 0
    }
}

handler CyclicRandom(arr: Double[]) of Random {
    i: Int = 0

    fn random() -> [] Double {
        return arr[i++]
    }
}

fn main() [use, ThreadEff] {
    use StdOutConsole
    use CyclicRandom([1.2, 5.7, 2.4])

    let list = mutable_list(1,2,3)
    list.add(3)
    list

    random_number()

    println("Starting thread...")
    let id = spawn(() -> {
        println("Hello world")
    })
    join(id)
    println("Done")
}

fn random_number() [Random] -> Double {
    return random()
}