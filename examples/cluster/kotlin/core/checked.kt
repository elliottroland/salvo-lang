package salvo.core.checked

data class Checked<T>(
    val value: T,
)

fun<T> checked(value: T): Checked<T> {
    return Checked(value = value)
}

fun<T> ignore(checked: Checked<T>) {
    (checked).let {}
}

fun<T> detach(checked: Checked<T>): T {
    return checked.value
}
