// [fmt-width] What a line too long for 120 columns becomes: each construct
// written on one line, printed broken the canonical way
// (`widths.out.sv`), but for a union and a chain of a tier-3 operator
// (`+`, `is A | B`), which the width does not break [expr-wrap]: their
// lines stay long. Parsed only (`print_tests.rs`), never checked.
type Small = Int
type Wide = FirstLongArmName | SecondLongArmName | ThirdLongArmName | FourthLongArmName | FifthLongArmName | SixthLongArmName
type After = Int

fn long_signature(
    first_parameter: Int,
    second_parameter: Str,
    third_parameter: List<Int>,
    fourth: Bool
) -> Int => first_parameter {
    return first_parameter
}

fn long_clause() -> AReturnTypeWithAVeryLongNameThatGoesOnAndOn
=> first_deduced, second_deduced, third_deduced, fourth_deduced {
    return 0
}

fn long_parameters(
    first_parameter_of_many: Int,
    second_parameter_of_many: Str,
    third_parameter_of_many: List<Int>
) [Console] -> Int {
    return 0
}

effect Store {
    fn long_member(
        first_key_of_the_store: Str,
        second_key_of_the_store: Str,
        third_key_of_store: Str
    ) -> Int => first_key_of_the_store
}

fn calls() {
    let result = a_function_with_a_long_name(
        the_first_argument,
        the_second_argument,
        the_third_argument_xxxxxxxxxxxxxxxx
    )
    let nested = outer_function(
        inner_function(first_argument_value_long, second_argument_value_long, third_argument_value_long),
        fourth
    )
    let nested = outer_function(
        inner_function(
            first_argument_value_of_a_very_long_call,
            second_argument_value_of_a_very_long_call,
            third_argument
        ),
        fourth
    )
    let typed = convert_between_representations<
        FirstRepresentationType,
        SecondRepresentationType,
        ThirdRepresentation
    >(1)
    let p = Point {
        x: a_long_coordinate_computation(first_value),
        y: another_long_coordinate_computation(second_value)
    }
    let f = (
        first_parameter: Int,
        second_parameter: Int,
        third_parameter: Int,
        fourth_parameter: Int
    ) -> first_parameter
    let (
        first_component_of_the_tuple,
        second_component_of_the_tuple,
        third_component_of_the_tuple,
        fourth_component
    ) = t
    let t = (
        first_component_of_the_tuple,
        second_component_of_the_tuple,
        third_component_of_the_tuple,
        fourth_component_xx
    )
    println(
        "a string longer than the width is never broken, and a list around it breaks only if that helps the line fit"
    )
    println("a string longer than the width on a line of its own too is never broken, and the call around it is not broken either")
}

fn chains(m: Int?, xs: List<Int>, ok: Bool, done: Bool) {
    let a = first_lookup_of_the_value(m)
        ?: second_lookup_of_the_value(m)
        ?: third_lookup_of_the_value(m)
        ?: fourth_lookup(m)
        ?: 0
    let b = xs
        .map((element) -> element + 1)
        .filter((element) -> element > 2)
        .fold(0, (total, element) -> total + element)
    let b = some_receiver_object.a_method_with_a_long_name(
        the_first_argument_value,
        the_second_argument_value_xxxxxxxxxx
    )
    let c = m
        |> first_transformation_of_the_value(_)
        |> second_transformation_of_the_value(_)
        ?> third_transformation_of_it(_)
    if first_condition_of_the_test(ok)
        && second_condition_of_the_test(done)
        && third_condition_of_the_test(ok)
        && fourth(ok) {
        return
    }
    let sum = first_summand_of_the_total(xs) + second_summand_of_the_total(xs) + third_summand_of_the_total(xs) + 4 + 5 + 6 + 7
    let in_brackets = check(
        m is FirstLongArmName | SecondLongArmName | ThirdLongArmName | FourthLongArmName | FifthLongArmName
    )
    let in_brackets = check(m is FirstLongArmName | SecondLongArmName | ThirdLongArmName | FourthLongArmName | FifthLongArmName | SixthLongArmName | Seventh)
    total(first_argument_of_the_total)
        replyto got(first_argument_of_the_total)
        replyto done(second_argument_xxxxxxxxxxxx)
    return first_lookup_of_the_value(m)
        ?: second_lookup_of_the_value(m)
        ?: third_lookup_of_the_value(m)
        ?: fourth_lookup(xs)
}

handler Memory(
    the_size_of_the_memory: Int,
    the_name_of_the_memory: Str,
    the_owner_of_the_memory: Str,
    the_age: Int
) for Store {
    fn get(key: Str) -> Int {
        return 0
    }
}

actor effect Counter {
    send fn add(
        the_amount_to_add_to_the_count: Int,
        a_note_about_the_addition: Str,
        the_time_of_it: Int,
        the_place: Str
    )
}

fn more(m: Thing?, xs: List<Int>) {
    let a = m?.lookup(
        the_first_key_of_the_lookup,
        the_second_key_of_the_lookup,
        the_third_key_of_it_xxxxxxxxxxxxxxxxxxxx
    )
    let b = f(
        m?.first_lookup_of_the_thing()?.second_lookup_of_the_thing()?.third_lookup_of_the_thing_xxxxxxxxxxxxxxxx()
    )
    let w: (
        FirstLongTypeNameOfTheTuple,
        SecondLongTypeNameOfTheTuple,
        ThirdLongTypeNameOfTheTuple,
        FourthLongTypeName
    ) = t
    total(1)
        replyto got(
            the_first_capture_of_the_mint,
            the_second_capture_of_the_mint,
            the_third_capture_of_the_mint_xxx,
            the_fourth_capture
        )
    let s = the_receiver
        .first_method_of_the_chain(argument)
        .second_method_of_the_chain(argument)
        .third_method_of_the_chain()
}
