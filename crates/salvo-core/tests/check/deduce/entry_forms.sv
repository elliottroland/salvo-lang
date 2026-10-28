// [deduce-syntax] Syntax rules for the new entry forms (the old
// `deduction_entry_forms_are_validated`).

//--- exhaustive_and_delta_do_not_mix
// Mixing exhaustive and delta in one entry.
qualifier A of Int

//~ error[deduce-syntax:exhaustive-and-delta]
fn f(x: A Int) -> None => x: A -A {
}

//--- an_establishing_entry_is_legal_in_the_qualifiers_file
// [deduce-reapply] `+Qual` says the function **establishes** the claim
// (user decisions 2026-09-22): legal in the qualifier's own file, whether or
// not the parameter already declares it, and trusted there.
qualifier A of Int

fn f(x: Int) -> None
=> x: +A {
}

//--- an_establishing_entry_must_apply_to_the_parameters_type
// …but the claim still has to make sense for the parameter's type.
qualifier A of Int

fn f(s: Str) -> None
//~ error[deduce-reapply:does-not-apply]
=> s: +A {
}

//--- type_narrowing_in_a_deduction_is_not_supported
// A type other than `Never` is not supported yet (D1b).
//~ error[deduce-syntax:type-narrowing]
fn f(x: List<Int>) -> None => x: List<Int> {
}

//--- an_effect_member_must_say_what_happens_to_each_parameter
// [decl-explicit] Bodyless declarations carry no inference: an effect
// member must state its deductions and return type (effects are forbidden
// there [effect-member-no-effects]). The declaration is then a real
// contract — a member that takes ownership consumes its argument at the
// call site. (The other bodyless form, `intrinsic`, is the compiler's and
// only std may write it [platform-intrinsic-std-only], so it never appears in a
// user source like these.)
//
// An effect member missing its clause (effects are forbidden there
// [effect-member-no-effects], so they are not asked for, and a missing
// return type is `None` [effect-member-none]). [deduce-syntax] The clause
// is required per parameter (Copy scalars exempt), not as a whole.
effect Sink {
    //~ error[deduce-syntax:bodiless-clause]
    fn eat(x: Str)
}

//--- an_effect_members_declared_deductions_are_enforced
// The member's declared deductions are enforced: `[]` moves `h`, so
// the later read is an error (this was silently accepted before).
struct Handle {
    id: Int
}

effect Sink {
    fn eat(h: Handle) -> None => !h
}

handler Bin of Sink {
    fn eat(h: Handle) -> None => !h {
        discard(h)
    }
}

fn main() [use] -> None {
    use StdConsole
    use Bin
    let h = Handle {id: 1}
    eat(h)
    //~ error[flow-use-after-move:consumed]
    println("${h.id}")
}
