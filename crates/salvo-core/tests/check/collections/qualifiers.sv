// [qual-overload] One qualifier name over several subject types. That std
// really declares `NonEmpty` five times over is asserted end to end in each
// backend's `compiles_and_runs_list_claims` case.

// Two qualifiers may share a name when their subjects differ, and each
// applies to its own. Before overloading the second declaration was a
// "duplicate qualifier" error and only one subject could have the name.
qualifier Filled<T> of List<T>
qualifier Filled<T> of Set<T>

fn several_subjects(a: Filled List<Int>, b: Filled Set<Int>) [] -> None => a, b {
}

// …and the applies-to check is still on: a subject no declaration of that
// name accepts is an error, so overloading did not turn it off.
//~ error[qual-of:does-not-apply]
fn not_a_subject(n: Filled Int) [] -> None => n {
}

//--- one_name_twice_over_one_subject_is_a_duplicate
// Two declarations of one name over **one** subject are a duplicate, not an
// overload: nothing at a use site could tell them apart.
qualifier Twice<T> of List<T>
//~ error[mod-collision:duplicate]
qualifier Twice<T> of List<T>
