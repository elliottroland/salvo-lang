# Control flow

There are minimal control flow options, and every option is an expression. All blocks (if, while, for) represent their own scopes, so that variables declared in them cannot be access from outside.

## If/elif/else blocks

As in Ruby, `if` blocks are expressions which evaluate to values. Each branch of an `if`-`elif`-`else` chain represents a possible value that the chain might resolve to, and the final type is the union of them all. The value and type of a branch are equal to the last expression in that branch:

```
// Type of `full_name` is `Str | Str` which simplifies to `Str`
let full_name = if person.surname is Str {
    "${person.name} ${person.surname}"
} else {
    person.name
}
```

The type of an unspecified `else` branch is `None`:

```
// Type of `full_name` is `Str | None` or `Str?`
let full_name = if person.surname is Str {
    "${person.name} ${person.surname}"
}
```

Conditions are boolean expressions -- `if`, `elif` and `while` accept `Bool` and nothing else. There is no truthiness: a number, a string or a possibly-absent `Bool?` is not a decision, so compare explicitly (`n != 0`, `name.size() > 0`, `flag!`). The `is` expressions we've been using for unions *are* booleans, although there is some special syntax to bind the casted value in the scope of the if block.

## When expressions

A `when` expression is always exhaustive. That is its purpose, and it happens in one of two ways depending on whether a subject is given.

**With a subject**, `when` branches on the arms of a union type. Since the arms are known at compile time, the compiler validates that every one of them is handled:

```
when [subject variable] {
    [check1] {
        [code]
    }
    [check2] {
        [code]
    }
    ...
}
```

There is no `else` branch in this form -- the arms *are* the cases, and covering them is what the compiler checks. The subject must be a union-typed variable. The subject together with a check should form a valid boolean expression that could work for an if-expression when concatenated (i.e. `[subject] [check]` should be the boolean expression). As with if-expressions, any type/qualifier checking proven in the condition allows us to refer to the subject within that block by the more specific type. The branches of the when expression each resolve to a value like the `if`-`elif`-`else` chain. For example:

```
let result: Ok Str | Err Str | Err Bool = some_function()

// `value` of type `Str` because the `Err` branch does not resolve to a value
let value = when result {
    is Ok {
        // `result` is of type `Ok Str`
        result
    }
    is Err {
        // `result` is of type `Err Str | Err Bool`
        println("Error: {result}")
        return
    }
}
```

**Without a subject**, `when` is a chain of conditions -- an `if`-`elif`-`else` chain in `when`'s shape. The branch heads are ordinary boolean expressions, and the `else` is mandatory, since with no arms to cover it is the only thing that can make the chain exhaustive:

```
// `label` is a `Str`, not a `Str?`: every path produces a value
let label = when {
    n < 0 { "negative" }
    n == 0 { "zero" }
    else { "positive" }
}
```

That mandatory `else` is the reason the form exists. An `if` chain without an `else` folds `None` into its value, so a chain of conditions that always produces something has to be written with a trailing `else` and read carefully to see that it does; a subject-less `when` says so in its grammar. Everything else follows from the heads being plain conditions: `is` and `^` work in them and narrow their branch, and a later branch sees the earlier ones ruled out.

```
when {
    value is Str s { println("a string: ${s}") }
    else { println("an int: ${value}") }   // `value` is an `Int` here
}
```

Note that a subject-less `when` gets no arm-exhaustiveness: writing `is` heads that happen to cover a union does not remove the need for the `else`. Use the subject form when that is what you mean.

## While loops

`while` loops can be thought of as repeated `if` blocks:

* `while` loops repeat until their expression evaluates to false.
* `while` loops support the same `is` expression as `if` and `elif`, where the value is rebound for each iteration of the loop.
* `while` loops evaluate to a value like `if` blocks, which is determined by either the last evaluated expression or one of the `break` statements (e.g. `break "hello, world!"`)
* `while` loops support the `continue` statement, which skips to the next iteration of the loop.
* `while` loops support an `else` block, which runs only if the loop never ran. This works the same as the `else` of an `if` block in that the last expression determines its value.

```
let numbers: Int[] = get_numbers()
let i = 0
let last = while i++ < numbers.size() { // ++ and -- work in both fixities
    numbers[i] // the last number evaluated will be the value of the while loop
} else {
    -1 // defaults to -1 if never looped
}
```

## For loops

`for` loops only work with iterators, and do not support the Java/C like `for (int i = 0; ...)` format. An iterator is a standard type and can be specified using functions (which we will see in detail in the functions section). For example:

```
// Print out a message for each age strictly less than the person's age.
// The `range` function returns an iterator which is exclusive of the end.
for i in range(0, person.age) {
    println("Person is older than ${i}...")
}
```

`for` loops evaluate to values just like `while` loops. They support `break`, `continue` and `else`.

## Lifting a qualifier: `is ^Q`

`is` narrows: a successful check means the value is *more* specific than its declared type. Marking a qualifier with `^` inside the check makes it do the dual as well — the arm is still tested, but the claim is **lifted**, so the value reads as *less* specific afterwards:

```
fn describe(p: Mut Person) [Console] {
    if p is ^Mut {
        read_only_report(p)      // `p` reads as `Person` here
    }
}
```

Its reason to exist is the qualified union. `Ok (Ok Int | Err Str)` is a claim *about* a union, so `when` cannot take its arms apart — they belong to the inner type. A `^` branch head tests the arm and removes the claim in one step:

```
let nested = try { wrapped(7) }        // Ok (Ok Int | Err Str) | Thrown Str
when nested {
    is ^Ok {
        // `nested` reads as `Ok Int | Err Str` here
        when nested {
            is Ok { println("value ${nested}") }
            is Err { println("error ${nested}") }
        }
    }
    is Thrown {
        println("thrown ${nested}")
    }
}
```

The rules:

- **It is an ordinary `is`**, so it works everywhere one does: `if`/`elif`, `&&`/`||`/`!`, and as a `when` branch head, where it consumes the arms it matched so exhaustiveness is unchanged. The runtime test is the same; `^` decides only whether the claim survives into the branch.
- **The qualifiers must be there.** Nothing to lift is an error, not a false test — `^Q` lifts a known claim, it does not test for one (that is a plain `is Q`).
- **All or none.** `is ^Mut ^NonEmpty` lifts both; a check mixing lifted and kept qualifiers (`is ^Ok NonEmpty`) is an error, since the two say opposite things about one check.
- **Some qualifiers can never be lifted**: `once` (it restricts rather than refines), `Linear` (it carries a use obligation) and `proj` (the value is derived from another). Everything else can, since dropping a claim loses only knowledge and dropping a permission loses only permission.
- **A binding is allowed**, and it is what makes a lift of *several arms at once* possible:

  ```
  let outcome: Ok Int | Ok Str | Err Str = classify(input)
  if outcome is ^Ok value {
      // `value` is `Int | Str`; `outcome` keeps its declared type here
      print_either(value)
  }
  ```

  Without a binding the lifted value has to be re-read out of the subject, and one such read cannot stand for two arms that live in different wrapper positions — so a multi-arm lift without a binding is an error naming this remedy. With a single arm, a binding is optional: the subject itself reads lifted, as `p is ^Mut` does above.
