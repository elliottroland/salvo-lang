package salvo.path

import salvo.core.list.addPlatform
import salvo.core.string.endsWithPlatform
import salvo.core.string.isEmpty
import salvo.core.string.sizePlatform
import salvo.core.string.splitLast
import salvo.core.string.splitPlatform
import salvo.core.string.startsWithPlatform
import salvo.core.string.trimSuffixPlatform

data class Path(
    val text: String,
)

object __Codec_Path : salvo.WireCodec<Path> {
    override fun enc(v: Path, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.text, out)
    }
    override fun dec(inp: salvo.WireIn): Path = Path(salvo.StrCodec.dec(inp))
}

fun path(text: String): Path {
    return Path(text = text)
}

fun toStr(p: Path): String {
    return p.text
}

fun isAbsolute(p: Path): Boolean {
    return startsWithPlatform(p.text, "/")
}

fun join__Path_Str(p: Path, child: String): Path {
    if (startsWithPlatform(child, "/") || isEmpty(p.text)) {
        return path(child)
    }
    if (endsWithPlatform(p.text, "/")) {
        return path("${p.text}$child")
    }
    return path("${p.text}/$child")
}

fun join__Path_Path(p: Path, child: Path): Path {
    return join__Path_Str(p, child.text)
}

fun parent(p: Path): Path? {
    val text = trimTrailingSlashes(p.text)
    val cut = splitLast(text, "/")
    if (cut == null) {
        return null
    }
    val (dir, _name) = cut
    if (isEmpty(dir)) {
        if (startsWithPlatform(text, "/") && sizePlatform(text) > 1) {
            return path("/")
        }
        return null
    }
    return path(dir)
}

fun fileName(p: Path): String? {
    val text = trimTrailingSlashes(p.text)
    val cut = splitLast(text, "/")
    if (cut == null) {
        if (isEmpty(text)) {
            return null
        }
        return text
    }
    val (_dir, name) = cut
    if (isEmpty(name)) {
        return null
    }
    return name
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun extension(p: Path): String? {
    val name = fileName(p)
    if (name != null) {
        val n = name as String
        val cut = splitLast(n, ".")
        if (cut == null) {
            return null
        }
        val (stem, ext) = cut
        if (isEmpty(stem)) {
            return null
        }
        return ext
    }
    return null
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun stem(p: Path): String? {
    val name = fileName(p)
    if (name != null) {
        val n = name as String
        val cut = splitLast(n, ".")
        if (cut == null) {
            return n
        }
        val (stem, _ext) = cut
        if (isEmpty(stem)) {
            return n
        }
        return stem
    }
    return null
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST", "UNNECESSARY_SAFE_CALL")
fun withExtension(p: Path, ext: String): Path {
    val s = stem(p)
    if (s != null) {
        val base = s as String
        val name = if (isEmpty(ext)) {
            base
        } else {
            "$base.$ext"
        }
        val up = parent(p)
        if (up != null) {
            val dir = up as Path
            return join__Path_Str(dir, name)
        }
        return path(name)
    }
    return path(p.text)
}

fun segments(p: Path): salvo.platform.core.list.MutList<String> {
    val out = mutableListOf<String>()
    for (part in salvo.platform.core.list.each(splitPlatform(p.text, "/"))) {
        if (!isEmpty(part)) {
            addPlatform(out, part)
        }
    }
    return out
}

fun trimTrailingSlashes(text: String): String {
    var t = text
    while (sizePlatform(t) > 1 && endsWithPlatform(t, "/")) {
        t = trimSuffixPlatform(t, "/")
    }
    return t
}

fun hash(value: Path): Long {
    var h = 17L
    h = ((h) * 31L + ((value.text).hashCode().toLong()))
    return h
}

fun eq(a: Path, b: Path): Boolean {
    if (!((a.text) == (b.text))) {
        return false
    }
    return true
}
