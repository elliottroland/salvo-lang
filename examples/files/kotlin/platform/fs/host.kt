// Host implementation of the platform declarations of Salvo module `fs.host`.
//
// Generated once by `salvo platform generate`; the compiler never writes
// this file again — it is yours. Nothing here is checked by Salvo: the
// Kotlin compiler checks it, against the interfaces the backend generates
// from the `platform handler` declarations.
package salvo.platform.fs.host

import salvo.*
import salvo.fs.*
import salvo.fs.host.*

import java.io.BufferedInputStream
import java.io.BufferedOutputStream
import java.io.File
import java.io.FileInputStream
import java.io.FileOutputStream
import java.nio.file.Files
import java.nio.file.Paths
import java.nio.file.StandardCopyOption

/**
 * The droppable failure kinds, as the generated union (`fs.FsError`; its
 * `Streaming` arm is never the host's to report — a stream failure surfaces
 * through `stream.host`).
 */
private typealias Kind = Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory,
    PathEscapes, IoError, Streaming>

private fun notFound(path: String): Kind = U7_1(NotFound(path))
private fun permissionDenied(path: String): Kind = U7_2(PermissionDenied(path))
private fun alreadyExists(path: String): Kind = U7_3(AlreadyExists(path))
private fun notADirectory(path: String): Kind = U7_4(NotADirectory(path))
private fun ioError(path: String, message: String): Kind = U7_6(IoError(path, message))

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

// `platform handler HostRawFs` — the compiler SERIALIZES this instance: every
// member runs under one `synchronized` monitor on both backends, so the plain
// hash maps below are fine. If the host ever synchronizes internally and wants
// to run concurrently, declare it `threadsafe platform handler` in Salvo
// [threadsafe-platform]. Reviewed 2026-09-26: an open-file table keyed by
// handle is inherently one-writer state, so undeclared is the right call.
// `platform handler HostRawFs` holds nothing of its own now: what it opens goes
// into the runtime's stream table [stream-table], where `stream.host`'s
// `HostRawStreams` reads it.
class HostRawFs : RawFs {
    override fun raw_open_read(path: String): Union2<Long, Kind> {
        return try {
            U2_1(SalvoStreams.registerIn(path, BufferedInputStream(FileInputStream(path)), 0))
        } catch (e: Exception) {
            U2_2(kindOf(path, e))
        }
    }

    override fun raw_open_read_at(path: String, offset: Long): Union2<Long, Kind> {
        return try {
            val input = FileInputStream(path)
            input.channel.position(if (offset < 0) 0 else offset)
            U2_1(SalvoStreams.registerIn(path, BufferedInputStream(input), if (offset < 0) 0 else offset))
        } catch (e: Exception) {
            U2_2(kindOf(path, e))
        }
    }

    override fun raw_open_write(path: String): Union2<Long, Kind> {
        return try {
            U2_1(SalvoStreams.registerOut(path, BufferedOutputStream(FileOutputStream(path, false)), 0))
        } catch (e: Exception) {
            U2_2(kindOf(path, e))
        }
    }

    override fun raw_open_append(path: String): Union2<Long, Kind> {
        return try {
            val existing = File(path).let { if (it.exists()) it.length() else 0L }
            U2_1(SalvoStreams.registerOut(path, BufferedOutputStream(FileOutputStream(path, true)), existing))
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
}
