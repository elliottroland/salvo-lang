// [test-file] The tests of module `fs`, which takes paths as `Path`s [path-type].

import fs.mem
import fs.path

test "a Path reaches the filesystem without spelling its text" {
    use MemFs()
    let dir = path("/data/board")
    let file = join(dir, "notices.db")
    let parent_of = parent(file)
    if parent_of is Path up {
        expect(create_dirs(up) is Ok, "created the directory")
    }
    let wrote = write_str(file, "hello")
    if wrote is Err {
        ignore(wrote)
        throw(Failure { message: "write failed" })
    }
    // `MemFs` directories are implicit: one exists once a file is in it.
    expect(exists(dir), "the directory exists")
    let back = read_to_str(file)
    if back is Ok {
        expect_eq(back, "hello")
    } else {
        ignore(back)
        throw(Failure { message: "read failed" })
    }
}
