// [fs-surface] [type-checked] The filesystem's failures carry the library's one
// obligation: a member answers `Err Checked<FsError>`, so a result that is never
// looked at does not compile, and the ways out are the generic ones — `ignore`,
// `detach`, or narrowing to `Ok`. Against *real* std, because what is being
// tested is the wiring between two std modules rather than the linear rule
// itself (that is `linear_group_tests`').

//--- an_fs_failure_must_be_acknowledged
// Looking at the `Err` arm and doing nothing with it leaves the
// obligation. The `Ok` arm consumes `text` and the `Err` arm drops it, so
// the surviving fate check reports it as consumed on some paths but not
// others (the frame-drop message moved to the IR analysis [ir-analyze],
// but this `when` leaves `text` owned on one path, which the fate-conflict
// check — kept in the AST checker — catches first).
import std.fs
import std.fs.path
import std.fs.host
import std.stream.host

fn main() [use] -> None {
    use StdConsole()
    use HostRawStreams()
    use DefaultStreams()
    use HostRawFs()
    use DefaultFs()
    let text = read_to_str(path("nope.txt"))
    //~ error[linear-obligation:some-paths]
    when text {
        is Ok { println(text) }
        is Err { println("failed") }
    }
}

//--- an_ignored_fs_failure_is_acknowledged
// `ignore` says the not-looking was deliberate.
import std.fs
import std.fs.path
import std.fs.host
import std.stream.host

fn main() [use] -> None {
    use StdConsole()
    use HostRawStreams()
    use DefaultStreams()
    use HostRawFs()
    use DefaultFs()
    let text = read_to_str(path("nope.txt"))
    when text {
        is Ok { println(text) }
        is Err { ignore(text) }
    }
}

//--- a_detached_fs_failure_is_acknowledged
// `detach` hands back the bare `FsError`, which `to_str` renders — the
// shape that replaced a read of the old wrapper.
import std.fs
import std.fs.path
import std.fs.host
import std.stream.host

fn main() [use] -> None {
    use StdConsole()
    use HostRawStreams()
    use DefaultStreams()
    use HostRawFs()
    use DefaultFs()
    let text = read_to_str(path("nope.txt"))
    when text {
        is Ok { println(text) }
        is Err { println(to_str(detach(text))) }
    }
}
