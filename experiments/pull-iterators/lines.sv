// I1b prototype — the Salvo source the two hand-written target programs
// implement. Not compilable today: `Once Iter<T>`, an effectful iterator fn,
// and the injected close are the design being validated.
//
// It exercises, deliberately, everything the design has to get right at once:
//   * a producer that performs effects (Console *and* FileSystem) while the
//     consumer drives it, so the interleaving is observable;
//   * a producer holding a resource, released by `defer`;
//   * a deferred block that itself performs effects (it prints *and* closes);
//   * two distinct resume points (the header `yield`, then the loop `yield`),
//     so the machine has real states rather than one loop;
//   * an unbounded `while true` that terminates only because the consumer
//     breaks — which is the path the injected `close` has to cover.

platform effect FileSystem {
    fn open(path: Str) -> Int
    fn read_line(handle: Int) -> Str?
    fn close(handle: Int) -> None
}

fn lines(path: Str) [FileSystem, Console] -> Once Iter<Str> {
    println("opening ${path}")
    let f = open(path)
    defer {
        println("closing ${path}")
        close(f)
    }
    yield "-- ${path} --"
    while true {
        let l = read_line(f)
        when l {
            is Str { yield l }
            is None { return }
        }
    }
}

fn main() [use] -> None {
    use StdOutConsole()
    use FakeFileSystem()

    let seen = 0
    for l in lines("data.txt") {
        println("line ${l}")
        seen = seen + 1
        if seen == 3 {
            break
        }
    }
    println("done")
}

// Expected stdout, both backends:
//   opening data.txt
//   line -- data.txt --
//   line alpha
//   line beta
//   closing data.txt
//   done
