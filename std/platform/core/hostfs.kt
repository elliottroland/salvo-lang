// Host implementation of the platform declarations of Salvo module `core.hostfs`.
//
// Generated once by `salvo platform generate`; the compiler never writes
// this file again — it is yours. Nothing here is checked by Salvo: the
// Kotlin compiler checks it, against the interfaces the backend generates
// from the `platform effect` and `platform handler` declarations.
package salvo.platform.core.hostfs

import salvo.*
import salvo.core.fs.*
import salvo.core.hostfs.*

import java.io.BufferedInputStream
import java.io.BufferedOutputStream
import java.io.ByteArrayOutputStream
import java.io.File
import java.io.FileInputStream
import java.io.FileOutputStream
import java.nio.ByteBuffer
import java.nio.charset.CodingErrorAction
import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Paths
import java.nio.file.StandardCopyOption

/** The droppable failure kinds, as the generated union. */
private typealias Kind = Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory,
    PathEscapes, InvalidUtf8, StaleHandle, IoError>

private fun notFound(path: String): Kind = U8_1(NotFound(path))
private fun permissionDenied(path: String): Kind = U8_2(PermissionDenied(path))
private fun alreadyExists(path: String): Kind = U8_3(AlreadyExists(path))
private fun notADirectory(path: String): Kind = U8_4(NotADirectory(path))
private fun invalidUtf8(path: String): Kind = U8_6(InvalidUtf8(path))
private fun staleHandle(handle: Long): Kind = U8_7(StaleHandle("<handle $handle>"))
private fun ioError(path: String, message: String): Kind = U8_8(IoError(path, message))

/**
 * The one place a JVM exception becomes a Salvo kind. The JVM reports a
 * missing file, an unreadable one and a non-directory parent as the same
 * `FileNotFoundException`, so the file itself is asked which it was — that
 * is what keeps the kinds identical to the Rust backend's.
 */
private fun kindOf(path: String, e: Exception): Kind {
    when (e) {
        is java.nio.file.NoSuchFileException -> return notFound(path)
        is java.nio.file.FileAlreadyExistsException -> return alreadyExists(path)
        is java.nio.file.NotDirectoryException -> return notADirectory(path)
        is java.nio.file.AccessDeniedException -> return permissionDenied(path)
        is java.nio.charset.CharacterCodingException -> return invalidUtf8(path)
        else -> {}
    }
    val file = File(path)
    if (e is java.io.FileNotFoundException) {
        if (!file.exists()) {
            val parent = file.parentFile
            if (parent != null && parent.exists() && !parent.isDirectory) {
                return notADirectory(path)
            }
            return notFound(path)
        }
        if (file.isDirectory) return notADirectory(path)
        return permissionDenied(path)
    }
    return ioError(path, e.message ?: e.toString())
}

/** Strict UTF-8: a malformed byte is an error, never a replacement char. */
private fun decodeStrict(bytes: ByteArray): String {
    val decoder = StandardCharsets.UTF_8.newDecoder()
        .onMalformedInput(CodingErrorAction.REPORT)
        .onUnmappableCharacter(CodingErrorAction.REPORT)
    return decoder.decode(ByteBuffer.wrap(bytes)).toString()
}

/**
 * A stream open for reading. The buffer is *bytes* — `BufferedReader` counts
 * characters, which would make the consumed-byte position unknowable — and
 * decoding is ours, above it.
 */
private class Reading(val path: String, val stream: BufferedInputStream, var position: Long) {
    var failed: Kind? = null
}

/** A stream open for writing; `position` counts bytes accepted. */
private class Writing(val path: String, val stream: BufferedOutputStream, var position: Long) {
    var failed: Kind? = null
}

class HostRawFs : RawFs {
    private var nextHandle: Long = 0
    private val reading = HashMap<Long, Reading>()
    private val writing = HashMap<Long, Writing>()

    private fun mint(): Long {
        nextHandle += 1
        return nextHandle
    }

    override fun raw_open_read(path: String): Union2<Long, Kind> {
        return try {
            val handle = mint()
            reading[handle] = Reading(path, BufferedInputStream(FileInputStream(path)), 0)
            U2_1(handle)
        } catch (e: Exception) {
            U2_2(kindOf(path, e))
        }
    }

    override fun raw_open_read_at(path: String, offset: Long): Union2<Long, Kind> {
        return try {
            val input = FileInputStream(path)
            input.channel.position(if (offset < 0) 0 else offset)
            val handle = mint()
            reading[handle] = Reading(path, BufferedInputStream(input), if (offset < 0) 0 else offset)
            U2_1(handle)
        } catch (e: Exception) {
            U2_2(kindOf(path, e))
        }
    }

    override fun raw_open_write(path: String): Union2<Long, Kind> {
        return try {
            val handle = mint()
            writing[handle] = Writing(path, BufferedOutputStream(FileOutputStream(path, false)), 0)
            U2_1(handle)
        } catch (e: Exception) {
            U2_2(kindOf(path, e))
        }
    }

    override fun raw_open_append(path: String): Union2<Long, Kind> {
        return try {
            val existing = File(path).let { if (it.exists()) it.length() else 0L }
            val handle = mint()
            writing[handle] =
                Writing(path, BufferedOutputStream(FileOutputStream(path, true)), existing)
            U2_1(handle)
        } catch (e: Exception) {
            U2_2(kindOf(path, e))
        }
    }

    override fun raw_exists(path: String): Boolean = File(path).exists()

    override fun raw_metadata(path: String): Union2<FileInfo, Kind> {
        val file = File(path)
        if (!file.exists()) return U2_2(notFound(path))
        return U2_1(FileInfo(file.length(), file.isDirectory))
    }

    override fun raw_list_dir(path: String): Union2<List<String>, Kind> {
        val file = File(path)
        if (!file.exists()) return U2_2(notFound(path))
        if (!file.isDirectory) return U2_2(notADirectory(path))
        val names = file.list() ?: return U2_2(permissionDenied(path))
        // Directory order is the host's; sorting makes the two backends agree.
        return U2_1(names.sorted())
    }

    override fun raw_create_dirs(path: String): Union2<Unit, Kind> {
        return try {
            Files.createDirectories(Paths.get(path))
            U2_1(Unit)
        } catch (e: Exception) {
            U2_2(kindOf(path, e))
        }
    }

    override fun raw_delete(path: String): Union2<Unit, Kind> {
        return try {
            Files.delete(Paths.get(path))
            U2_1(Unit)
        } catch (e: Exception) {
            U2_2(kindOf(path, e))
        }
    }

    override fun raw_rename_path(from: String, to: String): Union2<Unit, Kind> {
        return try {
            Files.move(Paths.get(from), Paths.get(to), StandardCopyOption.REPLACE_EXISTING)
            U2_1(Unit)
        } catch (e: Exception) {
            U2_2(kindOf(from, e))
        }
    }

    override fun raw_read_line(handle: Long): String? {
        val stream = reading[handle] ?: return null
        if (stream.failed != null) return null
        return try {
            val out = ByteArrayOutputStream()
            var count = 0L
            var sawAny = false
            while (true) {
                val b = stream.stream.read()
                if (b < 0) break
                sawAny = true
                count += 1
                out.write(b)
                if (b == 10) break
            }
            if (!sawAny) {
                null
            } else {
                stream.position += count
                var bytes = out.toByteArray()
                // Neither terminator is part of the line.
                if (bytes.isNotEmpty() && bytes[bytes.size - 1] == 10.toByte()) {
                    bytes = bytes.copyOf(bytes.size - 1)
                    if (bytes.isNotEmpty() && bytes[bytes.size - 1] == 13.toByte()) {
                        bytes = bytes.copyOf(bytes.size - 1)
                    }
                }
                decodeStrict(bytes)
            }
        } catch (e: Exception) {
            stream.failed = kindOf(stream.path, e)
            null
        }
    }

    override fun raw_read_all(handle: Long): Union2<String, Kind> {
        val stream = reading[handle] ?: return U2_2(staleHandle(handle))
        stream.failed?.let { return U2_2(it) }
        return try {
            val bytes = stream.stream.readBytes()
            stream.position += bytes.size.toLong()
            U2_1(decodeStrict(bytes))
        } catch (e: Exception) {
            val kind = kindOf(stream.path, e)
            stream.failed = kind
            U2_2(kind)
        }
    }

    override fun raw_read_position(handle: Long): Long = reading[handle]?.position ?: 0

    override fun raw_close_read(handle: Long): Union2<Unit, Kind> {
        val stream = reading.remove(handle) ?: return U2_2(staleHandle(handle))
        try {
            stream.stream.close()
        } catch (e: Exception) {
            if (stream.failed == null) stream.failed = kindOf(stream.path, e)
        }
        val failed = stream.failed
        return if (failed != null) U2_2(failed) else U2_1(Unit)
    }

    override fun raw_write(handle: Long, text: String): Long {
        val stream = writing[handle] ?: return 0
        if (stream.failed != null) return 0
        val bytes = text.toByteArray(StandardCharsets.UTF_8)
        return try {
            stream.stream.write(bytes)
            stream.position += bytes.size.toLong()
            bytes.size.toLong()
        } catch (e: Exception) {
            stream.failed = kindOf(stream.path, e)
            0
        }
    }

    override fun raw_write_position(handle: Long): Long = writing[handle]?.position ?: 0

    override fun raw_flush(handle: Long): Union2<Unit, Kind> {
        val stream = writing[handle] ?: return U2_2(staleHandle(handle))
        try {
            stream.stream.flush()
        } catch (e: Exception) {
            stream.failed = kindOf(stream.path, e)
        }
        val failed = stream.failed
        stream.failed = null
        return if (failed != null) U2_2(failed) else U2_1(Unit)
    }

    override fun raw_close_write(handle: Long): Union2<Unit, Kind> {
        val stream = writing.remove(handle) ?: return U2_2(staleHandle(handle))
        try {
            stream.stream.flush()
            stream.stream.close()
        } catch (e: Exception) {
            if (stream.failed == null) stream.failed = kindOf(stream.path, e)
        }
        val failed = stream.failed
        return if (failed != null) U2_2(failed) else U2_1(Unit)
    }
}
