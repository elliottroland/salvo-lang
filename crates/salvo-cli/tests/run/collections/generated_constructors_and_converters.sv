// generated constructors and converters [col-by] [col-convert]
//
// Compile-and-run data ([test-run-data]), moved from the codegen tests.

//--- generated_constructors

fn main() [use] -> None {
    use StdConsole()
    let a = array_by(3, i -> i * 2)
    println("array_by ${a[0]} ${a[1]} ${a[2]}")
    let xs = list_by(4, i -> i + 10)
    println("list_by ${to_str(xs)}")
    let s = set_by(5, i -> i % 3)
    println("set_by ${to_str(s)}")
    let m = map_by(3, i -> (i, i * i))
    println("map_by ${to_str(m)}")
    let dups = List(1, 2, 2, 3)
    let uniq = to_set(dups)
    println("to_set ${to_str(uniq)}")
    let pairs = List(("a", 1), ("b", 2))
    let m2 = to_map(pairs)
    println("to_map pairs ${to_str(m2)}")
    let words = List("alpha", "be")
    let m3 = to_map(words, w -> (w, size(w)))
    println("to_map rule ${to_str(m3)}")
    let arr = array_of(1, 2, 3)
    arr[0] = 9
    println("assigned ${arr[0]} size ${size(arr)}")
}
//= stdout
//= array_by 0 2 4
//= list_by [10, 11, 12, 13]
//= set_by {0, 1, 2}
//= map_by {0: 0, 1: 1, 2: 4}
//= to_set {1, 2, 3}
//= to_map pairs {a: 1, b: 2}
//= to_map rule {alpha: 5, be: 2}
//= assigned 9 size 3
