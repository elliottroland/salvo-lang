// A handler constructor with no parameters is written without brackets, of
// every kind; one with parameters keeps them, and so does a `use`.
handler Plain for Greeter {
    fn greet(name: Str) -> Str => name {
        return name
    }
}

export handler Exported [Console] for Greeter

handler Generic<T> for Greeter

handler Counting for Ordered Counter {
    mailbox { capacity: 8 }
}

handler Taking(prefix: Str) for Greeter

platform handler Host for Raw

threadsafe platform handler Threaded for Raw

intrinsic handler Built for Raw

fn main() [use] {
    use Plain()
    use Taking("a")
}
