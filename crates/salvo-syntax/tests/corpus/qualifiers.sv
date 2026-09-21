qualifier Surname of Person with Old

qualifier Old of Person with Surname

qualifier Ok<T> of T
qualifier Err<T> of T

qualifier Something<S, T> of Pair<S, T>
qualifier FirstStr<T> of Pair<Str, T>
qualifier Ints of Pair<Int, Int>

type Result<S, T> = Ok S | Err T

qualifier Positive of Int {
    fn qualifies(int: Int) -> Bool {
        return int > 0
    }
}

qualifier NonNegative of Int with Positive {
    fn qualifies(int: Int) -> Bool {
        return int >= 0
    }
}

qualifier HasSurname of Person {
    surname: Str

    fn qualifies(person: Person) -> Bool {
        return person.surname is Str
    }
}

qualifier RandomPositive of Int

// Constructor function: `-> Int as RandomPositive` marks it; return points
// return plain Int values which gain the qualifier by construction.
fn random_positive_int() [Random<Int>] -> Int as RandomPositive {
    let num = next_random()
    while num <= 0 {
        num = next_random()
    }
    return num
}

fn checks(person: Person) -> None {
    let p: Old Surname Person | None = check_old_surname(person)
    if p is Surname {
        println("has surname")
    } elif p is Person {
        println("plain person")
    } else {
        println("nothing")
    }
    let nested: Ok (Ok Str | Err Int) | Err Bool = some_function()
    let pair = (person, check_surname(person))
}

// [qual-lift] `is ^Q` lifts a qualifier: the same arm test, after which the
// subject reads with the claim *removed*, which is what opens a qualified
// union.
fn describe(outcome: Ok (Ok Int | Err Str) | Err Str) [Console] -> None {
    when outcome {
        is ^Ok {
            when outcome {
                is Ok {
                    println("value ${outcome}")
                }
                is Err {
                    println("inner error ${outcome}")
                }
            }
        }
        is Err {
            println("outer error ${outcome}")
        }
    }
}

fn read_only(list: Mut List<Int>) [Console] -> None => list: Mut {
    if list is ^Mut {
        println("size ${size(list)}")
    }
}

// [qual-refn] A qualifier states what functions it does not own do to *its*
// claim. `add` mutates its list, so [deduce-syntax] forbids it from
// promising `NonEmpty` back — but appending can never empty a list, and the
// qualifier that owns the claim may say so.
qualifier NonEmpty<T> of List<T> {
    fn qualifies(list: List<T>) -> Bool {
        return size(list) > 0
    }

    // Adding an element makes the list non-empty.
    refn add(list: Mut List<T>, elem: T) => list: +NonEmpty
}

// [qual-refn-reconcile] A top-level refinement is the consumer's own word on
// a function, and replaces the qualifiers' refinements for that parameter.
refn remove_first<T>(list: Mut NonEmpty List<T>) => list: -NonEmpty
