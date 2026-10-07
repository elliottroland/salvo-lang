package salvo.fs.path

import salvo.*

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
    return salvo.core.string.startsWithPlatform(p.text, "/")
}

fun join__Path_Str(p: Path, child: String): Path {
    if ((salvo.core.string.startsWithPlatform(child, "/") || salvo.core.string.isEmpty(p.text))) {
        return path(child)
    }
    if (salvo.core.string.endsWithPlatform(p.text, "/")) {
        return path("${p.text}${child}")
    }
    return path("${p.text}/${child}")
}

fun join__Path_Path(p: Path, child: Path): Path {
    return join__Path_Str(p, child.text)
}

fun parent(p: Path): Path? {
    val text: String = trimTrailingSlashes(p.text)
    val cut: Pair<String, String>? = salvo.core.string.splitLast(text, "/")
    if ((cut == null)) {
        return null
    }
    val cut_1: Pair<String, String> = cut!!
    val __destructured_2: Pair<String, String> = cut_1
    val dir: String = __destructured_2.first
    val _name: String = __destructured_2.second
    if (salvo.core.string.isEmpty(dir)) {
        if ((salvo.core.string.startsWithPlatform(text, "/") && (salvo.core.string.sizePlatform(text) > 1))) {
            return path("/")
        }
        return null
    }
    return path(dir)
}

fun fileName(p: Path): String? {
    val text: String = trimTrailingSlashes(p.text)
    val cut: Pair<String, String>? = salvo.core.string.splitLast(text, "/")
    if ((cut == null)) {
        if (salvo.core.string.isEmpty(text)) {
            return null
        }
        return text
    }
    val cut_1: Pair<String, String> = cut!!
    val __destructured_2: Pair<String, String> = cut_1
    val _dir: String = __destructured_2.first
    val name: String = __destructured_2.second
    if (salvo.core.string.isEmpty(name)) {
        return null
    }
    return name
}

fun extension(p: Path): String? {
    val name: String? = fileName(p)
    if ((name != null)) {
        val n: String = name!!
        val cut: Pair<String, String>? = salvo.core.string.splitLast(n, ".")
        if ((cut == null)) {
            return null
        }
        val cut_1: Pair<String, String> = cut!!
        val __destructured_2: Pair<String, String> = cut_1
        val stem: String = __destructured_2.first
        val ext: String = __destructured_2.second
        if (salvo.core.string.isEmpty(stem)) {
            return null
        }
        return ext
    }
    return null
}

fun stem(p: Path): String? {
    val name: String? = fileName(p)
    if ((name != null)) {
        val n: String = name!!
        val cut: Pair<String, String>? = salvo.core.string.splitLast(n, ".")
        if ((cut == null)) {
            return n
        }
        val cut_1: Pair<String, String> = cut!!
        val __destructured_2: Pair<String, String> = cut_1
        val stem: String = __destructured_2.first
        val _ext: String = __destructured_2.second
        if (salvo.core.string.isEmpty(stem)) {
            return n
        }
        return stem
    }
    return null
}

fun withExtension(p: Path, ext: String): Path {
    val s: String? = stem(p)
    if ((s != null)) {
        val base: String = s!!
        val name: String = (if (salvo.core.string.isEmpty(ext)) {
            base
        } else {
            "${base}.${ext}"
        })
        val up: Path? = parent(p)
        if ((up != null)) {
            val dir: Path = up!!
            return join__Path_Str(dir, name)
        }
        return path(name)
    }
    return path(p.text)
}

fun segments(p: Path): salvo.platform.core.list.MutList<String> {
    val out: salvo.platform.core.list.MutList<String> = mutableListOf<String>()
    for (part in salvo.platform.core.list.each(salvo.core.string.splitPlatform(p.text, "/"))) {
        if (!(salvo.core.string.isEmpty(part))) {
            salvo.core.list.addPlatform(out, part)
        }
    }
    return out
}

fun trimTrailingSlashes(text: String): String {
    var t: String = text
    while (true) {
        if (!(((salvo.core.string.sizePlatform(t) > 1) && salvo.core.string.endsWithPlatform(t, "/")))) {
            break
        }
        t = salvo.core.string.trimSuffixPlatform(t, "/")
    }
    return t
}

fun hash(value: Path): Long {
    var h: Long = 17L
    h = salvo.core.compare.mixHash(h, (value.text).hashCode().toLong())
    return h
}

fun eq(a: Path, b: Path): Boolean {
    if (!(((a.text) == (b.text)))) {
        return false
    }
    return true
}

