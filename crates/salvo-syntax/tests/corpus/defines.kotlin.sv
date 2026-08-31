define type LinkedList<T> {
    imports: ``
    import java.util.LinkedList
    ``

    inline: ``
    LinkedList<${T}>
    ``
}

define fn complicated_func<T>(list: List<T>) -> Str {
    imports: ``
    import salvo.complicatedFunc
    ``

    inline: ``
    complicatedFunc(${list})
    ``
}

define fn list<T>(...elems: T[]) -> List<T> {
    inline: ``
    listOf(${...elems})
    ``
}

define handler StdOutConsole of Console {
    define fn print(message: Str) {
        inline: ``
        print(${message})
        ``
    }
}
