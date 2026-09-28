#![allow(non_snake_case, non_camel_case_types, unused_mut, unused_parens, unused_imports, dead_code, unreachable_code, unused_variables, path_statements, unused_must_use, suspicious_double_ref_op, non_upper_case_globals)]
#[path = "unions.rs"]
pub mod unions;
#[path = "seq.rs"]
pub mod seq;
#[path = "collections.rs"]
pub mod collections;
#[path = "scheduler.rs"]
pub mod scheduler;
#[path = "hosttime.rs"]
pub mod hosttime;
#[path = "wire.rs"]
pub mod wire;
#[path = "core/bytes.rs"]
pub mod core_bytes;
#[path = "core/checked.rs"]
pub mod core_checked;
#[path = "core/console.rs"]
pub mod core_console;
#[path = "core/iterator.rs"]
pub mod core_iterator;
#[path = "core/list.rs"]
pub mod core_list;
#[path = "core/map.rs"]
pub mod core_map;
#[path = "core/result.rs"]
pub mod core_result;
#[path = "core/set.rs"]
pub mod core_set;
#[path = "core/sorted.rs"]
pub mod core_sorted;
#[path = "core/string.rs"]
pub mod core_string;
#[path = "fs.rs"]
pub mod fs;
#[path = "fs/host.rs"]
pub mod fs_host;
#[path = "fs/mem.rs"]
pub mod fs_mem;
#[path = "fs/restricted.rs"]
pub mod fs_restricted;
#[path = "platform/fs/host.rs"]
pub mod platform_fs_host;

use crate::core_bytes::*;
use crate::core_checked::*;
use crate::core_console::*;
use crate::core_iterator::*;
use crate::core_list::*;
use crate::core_map::*;
use crate::core_result::*;
use crate::core_set::*;
use crate::core_sorted::*;
use crate::core_string::*;
use crate::fs::*;
use crate::fs_host::*;
use crate::fs_mem::*;
use crate::fs_restricted::*;
use crate::unions::*;

pub fn kind_name(kind: &Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>) -> String {
    if matches!(kind, Union8::U1(_)) {
        return "not found".to_string();
    }
    if matches!(kind, Union8::U4(_)) {
        return "not a directory".to_string();
    }
    if matches!(kind, Union8::U5(_)) {
        return "escapes the sandbox".to_string();
    }
    if matches!(kind, Union8::U6(_)) {
        return "not valid UTF-8".to_string();
    }
    return "other".to_string();
}

pub fn workflow(fs: &mut crate::fs::__Handle_Fs, console: &mut crate::core_console::__Handle_Console) {
    let mut wrote = write_str(fs, &("notes.txt".to_string()), &("alpha\nbeta\ngamma\n".to_string()));
    match wrote {
        Union2::U1(_) => {
            println(console, &(format!("wrote {} bytes", *wrote.u1())));
        }
        Union2::U2(_) => {
            println(console, &(format!("write failed: {}", kind_name(&(detach(wrote.u2().clone()))))));
        }
    }
    let mut text = read_to_str(fs, &("notes.txt".to_string()));
    match text {
        Union2::U1(_) => {
            println(console, &(format!("read back {} bytes", (text.u1().len() as i64))));
        }
        Union2::U2(_) => {
            println(console, &(format!("read failed: {}", kind_name(&(detach(text.u2().clone()))))));
        }
    }
    let mut opened = fs.open_read(&("notes.txt".to_string()));
    match opened {
        Union2::U1(_) => {
            let mut p = lines(opened.u1().clone());
            while let Union2::U1(mut line) = next__13(fs, &mut p) {
                println(console, &(format!("line: {}", line)));
            }
            let mut closed = close(fs, p);
            if matches!(closed, Union2::U2(_)) {
                println(console, &(format!("close failed: {}", kind_name(&(detach(closed.u2().clone()))))));
            }
        }
        Union2::U2(_) => {
            println(console, &(format!("open failed: {}", kind_name(&(detach(opened.u2().clone()))))));
        }
    }
    let mut out = fs.open_append(&("notes.txt".to_string()));
    match out {
        Union2::U1(_) => {
            let mut w: OutStream = out.u1().clone();
            let mut at = fs.position__2(&w);
            let mut n = fs.write_line(&w, &("delta".to_string()));
            println(console, &(format!("appended {} bytes at offset {}", n, at)));
            let mut shut = fs.close__2(w);
            if matches!(shut, Union2::U2(_)) {
                println(console, &(format!("close failed: {}", kind_name(&(detach(shut.u2().clone()))))));
            }
            let mut resumed = fs.open_read_at(&("notes.txt".to_string()), at);
            match resumed {
                Union2::U1(_) => {
                    let mut s: InStream = resumed.u1().clone();
                    let mut line = fs.read_line(&s);
                    match line {
                        Some(_) => {
                            println(console, &(format!("at {}: {}", at, line.as_ref().unwrap().clone())));
                        }
                        None => {
                            println(console, &(format!("at {}: end of file", at)));
                        }
                    }
                    let mut done = fs.close(s);
                    if matches!(done, Union2::U2(_)) {
                        println(console, &(format!("close failed: {}", kind_name(&(detach(done.u2().clone()))))));
                    }
                }
                Union2::U2(_) => {
                    println(console, &(format!("reopen failed: {}", kind_name(&(detach(resumed.u2().clone()))))));
                }
            }
        }
        Union2::U2(_) => {
            println(console, &(format!("append failed: {}", kind_name(&(detach(out.u2().clone()))))));
        }
    }
    let mut bin = fs.open_write(&("raw.bin".to_string()));
    match bin {
        Union2::U1(_) => {
            let mut w: OutStream = bin.u1().clone();
            let mut data = vec![(((0) as i32) as u8), (((255) as i32) as u8), (((200) as i32) as u8)];
            let mut n = fs.write_bytes(&w, &data);
            let mut m = fs.write(&w, &("hé".to_string()));
            println(console, &(format!("wrote {} raw bytes and {} encoded", n, m)));
            let mut shut = fs.close__2(w);
            if matches!(shut, Union2::U2(_)) {
                println(console, &(format!("close failed: {}", kind_name(&(detach(shut.u2().clone()))))));
            }
        }
        Union2::U2(_) => {
            println(console, &(format!("raw open failed: {}", kind_name(&(detach(bin.u2().clone()))))));
        }
    }
    let mut raw = fs.open_read(&("raw.bin".to_string()));
    match raw {
        Union2::U1(_) => {
            let mut s: InStream = raw.u1().clone();
            let mut head = fs.read_bytes(&s, 3);
            match head {
                Union2::U1(_) => {
                    println(console, &(format!("first three: {} = {}", format!("[{}]", head.u1().clone().iter().map(|__b| __b.to_string()).collect::<Vec<String>>().join(", ")), head.u1().iter().map(|__b| format!("{:02x}", __b)).collect::<String>())));
                }
                Union2::U2(_) => {
                    println(console, &(format!("byte read failed: {}", kind_name(&(detach(head.u2().clone()))))));
                }
            }
            let mut tail = fs.read_all(&s);
            match tail {
                Union2::U1(_) => {
                    println(console, &(format!("the rest, as text: {}", tail.u1().clone())));
                }
                Union2::U2(_) => {
                    println(console, &(format!("decode failed: {}", kind_name(&(detach(tail.u2().clone()))))));
                }
            }
            let mut done = fs.close(s);
            if matches!(done, Union2::U2(_)) {
                println(console, &(format!("close failed: {}", kind_name(&(detach(done.u2().clone()))))));
            }
        }
        Union2::U2(_) => {
            println(console, &(format!("raw read failed: {}", kind_name(&(detach(raw.u2().clone()))))));
        }
    }
    let mut split = fs.open_read_at(&("raw.bin".to_string()), 5i64);
    match split {
        Union2::U1(_) => {
            let mut s: InStream = split.u1().clone();
            let mut broken = fs.read_all(&s);
            match broken {
                Union2::U1(_) => {
                    println(console, &(format!("unexpected: {} decoded", broken.u1().clone())));
                }
                Union2::U2(_) => {
                    println(console, &(format!("mid-character: {}", kind_name(&(detach(broken.u2().clone()))))));
                }
            }
            let mut done = fs.close(s);
            match done {
                Union2::U1(_) => {
                    println(console, &("unexpected: the failure was not recorded".to_string()));
                }
                Union2::U2(_) => {
                    println(console, &(format!("and again at close: {}", kind_name(&(detach(done.u2().clone()))))));
                }
            }
        }
        Union2::U2(_) => {
            println(console, &(format!("split open failed: {}", kind_name(&(detach(split.u2().clone()))))));
        }
    }
    let mut held = fs.open_read(&("raw.bin".to_string()));
    match held {
        Union2::U1(_) => {
            let mut s: InStream = held.u1().clone();
            let mut buf = Vec::<u8>::new();
            let mut steps = 0;
            let mut moved = 0;
            let mut reading = true;
            while reading {
                buf.clear();
                let mut got = fs.read_to(&s, &mut buf, 4);
                match got {
                    Union2::U1(_) => {
                        let mut n: i32 = *got.u1();
                        if n == 0 {
                            reading = false;
                        } else {
                            steps = steps + 1;
                            moved = moved + n;
                        }
                    }
                    Union2::U2(_) => {
                        println(console, &(format!("fill failed: {}", kind_name(&(detach(got.u2().clone()))))));
                        reading = false;
                    }
                }
            }
            println(console, &(format!("filled {} bytes in {} reads, one buffer", moved, steps)));
            let mut done = fs.close(s);
            if matches!(done, Union2::U2(_)) {
                println(console, &(format!("close failed: {}", kind_name(&(detach(done.u2().clone()))))));
            }
        }
        Union2::U2(_) => {
            println(console, &(format!("fill open failed: {}", kind_name(&(detach(held.u2().clone()))))));
        }
    }
    let mut lined = fs.open_read(&("notes.txt".to_string()));
    match lined {
        Union2::U1(_) => {
            let mut s: InStream = lined.u1().clone();
            let mut line = String::new();
            let mut longest = 0;
            let mut reading = true;
            while reading {
                line.clear();
                if fs.read_line_to(&s, &mut line) {
                    if ((line.chars().count() as i32) > longest) {
                        longest = (line.chars().count() as i32);
                    }
                } else {
                    reading = false;
                }
            }
            println(console, &(format!("longest line: {} characters", longest)));
            let mut done = fs.close(s);
            if matches!(done, Union2::U2(_)) {
                println(console, &(format!("close failed: {}", kind_name(&(detach(done.u2().clone()))))));
            }
        }
        Union2::U2(_) => {
            println(console, &(format!("lines open failed: {}", kind_name(&(detach(lined.u2().clone()))))));
        }
    }
    let mut ch = open_chunks(fs, &("raw.bin".to_string()), 4);
    match ch {
        Union2::U1(_) => {
            let mut p = ch.u1().clone();
            let mut seen = 0;
            while let Union2::U1(mut chunk) = next__14(fs, &mut p) {
                seen = seen + (chunk.len() as i32);
            }
            println(console, &(format!("chunks saw {} bytes", seen)));
            let mut done = close__2(fs, p);
            if matches!(done, Union2::U2(_)) {
                println(console, &(format!("close failed: {}", kind_name(&(detach(done.u2().clone()))))));
            }
        }
        Union2::U2(_) => {
            println(console, &(format!("chunks failed: {}", kind_name(&(detach(ch.u2().clone()))))));
        }
    }
    let mut copied = copy_file(fs, &("notes.txt".to_string()), &("notes-copy.txt".to_string()));
    match copied {
        Union2::U1(_) => {
            println(console, &(format!("copied {} bytes", *copied.u1())));
        }
        Union2::U2(_) => {
            println(console, &(format!("copy failed: {}", kind_name(&(detach(copied.u2().clone()))))));
        }
    }
    let mut whole = read_to_bytes(fs, &("raw.bin".to_string()));
    match whole {
        Union2::U1(_) => {
            println(console, &(format!("raw.bin is {} bytes: {}", (whole.u1().len() as i32), whole.u1().iter().map(|__b| format!("{:02x}", __b)).collect::<String>())));
        }
        Union2::U2(_) => {
            println(console, &(format!("byte read failed: {}", kind_name(&(detach(whole.u2().clone()))))));
        }
    }
    let mut failures: Vec<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>> = vec![];
    let mut missing = read_to_str(fs, &("nope.txt".to_string()));
    match missing {
        Union2::U1(_) => {
            println(console, &(format!("unexpected: {}", missing.u1().clone())));
        }
        Union2::U2(_) => {
            failures.push(detach(missing.u2().clone()));
        }
    }
    let mut not_a_dir = fs.list_dir(&("notes.txt".to_string()));
    match not_a_dir {
        Union2::U1(_) => {
            println(console, &(format!("unexpected: {}", format!("[{}]", not_a_dir.u1().clone().iter().map(|__e| __e.to_string()).collect::<Vec<_>>().join(", ")))));
        }
        Union2::U2(_) => {
            failures.push(detach(not_a_dir.u2().clone()));
        }
    }
    println(console, &(format!("failures: {}", (failures.len() as i32))));
    for mut kind in failures.clone() {
        println(console, &(format!("  {}", kind_name(&(kind.clone())))));
    }
    for mut name in vec!["notes.txt".to_string(), "notes-copy.txt".to_string(), "raw.bin".to_string()] {
        let mut gone = fs.delete(&name);
        if matches!(gone, Union2::U2(_)) {
            println(console, &(format!("delete failed: {}", kind_name(&(detach(gone.u2().clone()))))));
        }
    }
    println(console, &("cleaned up".to_string()));
}

pub fn sandbox_edges(fs: &mut crate::fs::__Handle_Fs, console: &mut crate::core_console::__Handle_Console) {
    let mut inside = write_str(fs, &("sub/../probe.txt".to_string()), &("inside\n".to_string()));
    match inside {
        Union2::U1(_) => {
            println(console, &(format!("through `..`: wrote {} bytes", *inside.u1())));
        }
        Union2::U2(_) => {
            println(console, &(format!("through `..`: {}", kind_name(&(detach(inside.u2().clone()))))));
        }
    }
    let mut up = read_to_str(fs, &("../secret.txt".to_string()));
    match up {
        Union2::U1(_) => {
            println(console, &("unexpected: read outside the sandbox".to_string()));
        }
        Union2::U2(_) => {
            println(console, &(format!("climbing out: {}", kind_name(&(detach(up.u2().clone()))))));
        }
    }
    let mut absolute = read_to_str(fs, &("/etc/hosts".to_string()));
    match absolute {
        Union2::U1(_) => {
            println(console, &("unexpected: an absolute path resolved".to_string()));
        }
        Union2::U2(_) => {
            println(console, &(format!("absolute path: {}", kind_name(&(detach(absolute.u2().clone()))))));
        }
    }
    let mut probe = "probe.txt".to_string();
    println(console, &(format!("probe still there: {}", fs.exists(&probe))));
    let mut gone = fs.delete(&probe);
    if matches!(gone, Union2::U2(_)) {
        println(console, &(format!("delete failed: {}", kind_name(&(detach(gone.u2().clone()))))));
    }
}

pub fn main() {
    let mut console = crate::core_console::__Handle_Console::new(StdOutConsole::new());
    let mut raw_fs = crate::fs_host::__Handle_RawFs::new(crate::platform_fs_host::HostRawFs::new());
    let mut fs = crate::fs::__Handle_Fs::new(DefaultFs::new(raw_fs.clone()));
    let mut root = "tmp/files-example".to_string();
    let mut made = fs.create_dirs(&root);
    if matches!(made, Union2::U2(_)) {
        println(&mut console, &(format!("cannot create the working directory: {}", kind_name(&(detach(made.u2().clone()))))));
        return;
    }
    println(&mut console, &("-- the real filesystem, scoped to one directory --".to_string()));
    if true {
        let mut fs2 = crate::fs::__Handle_Fs::new(RestrictedFs::new(root.clone(), fs.clone()));
        workflow(&mut fs2, &mut console);
        sandbox_edges(&mut fs2, &mut console);
    }
    let mut gone = fs.delete(&root);
    if matches!(gone, Union2::U2(_)) {
        println(&mut console, &(format!("cleanup failed: {}", kind_name(&(detach(gone.u2().clone()))))));
    }
    println(&mut console, &("-- the same code, with no disk at all --".to_string()));
    if true {
        let mut fs3 = crate::fs::__Handle_Fs::new(MemFs::new());
        workflow(&mut fs3, &mut console);
    }
}
