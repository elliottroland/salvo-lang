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

// [qual-widen] `^` is the dual of `is`: a successful check reads the subject
// with the qualifier *removed*, which is what opens a qualified union.
fn describe(outcome: Ok (Ok Int | Err Str) | Err Str) [Console] -> None {
    when outcome {
        ^ Ok {
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

fn read_only(list: Mut List<Int>) [Console] -> [list: Mut] None {
    if list ^ Mut {
        println("size ${size(list)}")
    }
}
