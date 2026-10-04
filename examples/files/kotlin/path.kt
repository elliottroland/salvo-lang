package salvo.path

import salvo.core.actor.*
import salvo.core.bytes.*
import salvo.core.deque.*
import salvo.core.list.*
import salvo.core.map.*
import salvo.core.set.*
import salvo.core.sorted.*
import salvo.core.string.*

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

fun toStr__4(p: Path): String {
    return p.text
}

fun isAbsolute(p: Path): Boolean {
    return p.text.startsWith("/")
}

fun join__2(p: Path, child: String): Path {
    if (child.startsWith("/") || isEmpty__2(p.text)) {
        return path(child)
    }
    if (p.text.endsWith("/")) {
        return path("${p.text}$child")
    }
    return path("${p.text}/$child")
}

fun join__3(p: Path, child: Path): Path {
    return join__2(p, child.text)
}

fun parent(p: Path): Path? {
    val text = trimTrailingSlashes(p.text)
    val cut = splitLast(text, "/")
    if (cut == null) {
        return null
    }
    val (dir, _name) = cut
    if (isEmpty__2(dir)) {
        if (text.startsWith("/") && text.length > 1) {
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
        if (isEmpty__2(text)) {
            return null
        }
        return text
    }
    val (_dir, name) = cut
    if (isEmpty__2(name)) {
        return null
    }
    return name
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun extension(p: Path): String? {
    val name = fileName(p)
    if (name != null) {
        val n = name as String
        val cut = splitLast(n, ".")
        if (cut == null) {
            return null
        }
        val (stem, ext) = cut
        if (isEmpty__2(stem)) {
            return null
        }
        return ext
    }
    return null
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun stem(p: Path): String? {
    val name = fileName(p)
    if (name != null) {
        val n = name as String
        val cut = splitLast(n, ".")
        if (cut == null) {
            return n
        }
        val (stem, _ext) = cut
        if (isEmpty__2(stem)) {
            return n
        }
        return stem
    }
    return null
}

@Suppress("UNCHECKED_CAST", "USELESS_CAST")
fun withExtension(p: Path, ext: String): Path {
    val s = stem(p)
    if (s != null) {
        val base = s as String
        val name = if (isEmpty__2(ext)) {
            base
        } else {
            "$base.$ext"
        }
        val up = parent(p)
        if (up != null) {
            val dir = up as Path
            return join__2(dir, name)
        }
        return path(name)
    }
    return path(p.text)
}

fun segments(p: Path): MutableList<String> {
    val out = mutableListOf<String>()
    for (part in p.text.split("/").toMutableList()) {
        if (!isEmpty__2(part)) {
            out.add(part)
        }
    }
    return out
}

fun trimTrailingSlashes(text: String): String {
    var t = text
    while (t.length > 1 && t.endsWith("/")) {
        t = t.removeSuffix("/")
    }
    return t
}

fun hash__4(value: Path): Long {
    var h = 17L
    h = ((h) * 31L + ((value.text).hashCode().toLong()))
    return h
}

fun eq__4(a: Path, b: Path): Boolean {
    if (!((a.text) == (b.text))) {
        return false
    }
    return true
}
