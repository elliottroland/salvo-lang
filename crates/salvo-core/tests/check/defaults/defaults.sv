// [default-fits] A `= default` must fit the slot it fills, at every default
// site: a struct field, a parameter, a handler's state. Until 2026-10-20 the
// struct-field and parameter sites checked the expression and discarded the
// result, so `port: InRange(1, 10) Int = 8080` compiled with a false claim.
// [default-predicate] What a default may lack: a predicate qualifier with
// constant arguments, tested where the default is applied (the run test
// `a_predicate_default_is_tested_where_it_is_applied` covers the trap).

// [default-fits] A default of the wrong type is an error at every site:
// a struct field and a parameter (both unchecked before the fix) and a
// handler's state (the one site that was already checked).
//~ error[default-fits:mismatch]
struct WrongField { n: Int = "x" }

//~ error[default-fits:mismatch]
fn wrong_param(n: Int = "x") [] -> Int => n { return n }

effect WrongE { fn get() -> Int }
handler WrongState() of WrongE {
    //~ error[default-fits:mismatch]
    state { n: Int = "x" }
    fn get() -> Int { return n }
}

// [default-predicate] A predicate qualifier with constant arguments may be
// missing from a default: it is tested where the default is applied. At a
// field, a parameter and a handler's state alike.
struct RangedField { n: InRange(1, 10) Int = 8080 }

fn ranged_param(n: InRange(1, 10) Int = 8080) [] -> Int => n { return 0 }

effect RangedE { fn get() -> Int }
handler RangedState() of RangedE {
    state { n: InRange(1, 10) Int = 8080 }
    fn get() -> Int { return 0 }
}

// [default-predicate] What cannot be tested where a default is applied is
// still refused, naming the qualifier: a constructive claim (nothing tests
// it), and a predicate whose `qualifies` needs an implicit the site may not
// have.
qualifier Big of Int {
    fn qualifies(n: Int, ?limit: (n: Int) -> Bool) -> Bool { return limit(n) }
}

//~ error[default-fits:mismatch]
struct SortedField { xs: Sorted List<Int> = List(1, 2) }

//~ error[default-fits:mismatch]
struct BigField { n: Big Int = 3 }
