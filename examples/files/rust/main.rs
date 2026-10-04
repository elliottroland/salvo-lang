#![allow(non_snake_case, non_camel_case_types, unused_mut, unused_parens, unused_imports, dead_code, unreachable_code, unused_variables, path_statements, unused_must_use, suspicious_double_ref_op, non_upper_case_globals)]
#[path = "unions.rs"]
pub mod unions;
#[path = "seq.rs"]
pub mod seq;
#[path = "collections.rs"]
pub mod collections;
#[path = "scheduler.rs"]
pub mod scheduler;
#[path = "hoststreams.rs"]
pub mod hoststreams;
#[path = "hosttime.rs"]
pub mod hosttime;
#[path = "wire.rs"]
pub mod wire;
#[path = "core/actor.rs"]
pub mod core_actor;
#[path = "core/bytes.rs"]
pub mod core_bytes;
#[path = "core/checked.rs"]
pub mod core_checked;
#[path = "core/console.rs"]
pub mod core_console;
#[path = "core/deque.rs"]
pub mod core_deque;
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
#[path = "path.rs"]
pub mod path;
#[path = "runtime.rs"]
pub mod runtime;
#[path = "runtime/routing.rs"]
pub mod runtime_routing;
#[path = "runtime/streams.rs"]
pub mod runtime_streams;
#[path = "stream.rs"]
pub mod stream;
#[path = "stream/host.rs"]
pub mod stream_host;
#[path = "platform/core/bytes.rs"]
pub mod platform_core_bytes;
#[path = "platform/core/console.rs"]
pub mod platform_core_console;
#[path = "platform/core/string.rs"]
pub mod platform_core_string;
#[path = "platform/fs/host.rs"]
pub mod platform_fs_host;
#[path = "platform/runtime/routing.rs"]
pub mod platform_runtime_routing;
#[path = "platform/runtime/streams.rs"]
pub mod platform_runtime_streams;
#[path = "platform/runtime.rs"]
pub mod platform_runtime;

use crate::core_bytes::*;
use crate::core_checked::*;
use crate::core_console::*;
use crate::core_deque::*;
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
use crate::stream::*;
use crate::stream_host::*;
use crate::unions::*;

pub fn kind_name(kind: &Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>) -> String {
    if matches!(kind, Union7::U1(_)) {
        return "not found".to_string();
    }
    if matches!(kind, Union7::U4(_)) {
        return "not a directory".to_string();
    }
    if matches!(kind, Union7::U5(_)) {
        return "escapes the sandbox".to_string();
    }
    if matches!(kind, Union7::U7(_)) {
        return kind_name__2(&kind.u7().clone().error);
    }
    return "other".to_string();
}

pub fn kind_name__2(kind: &Union2<InvalidUtf8, StreamFailed>) -> String {
    if matches!(kind, Union2::U1(_)) {
        return "not valid UTF-8".to_string();
    }
    return "other".to_string();
}

pub fn workflow(fs: &crate::fs::Fs, console: &crate::core_console::Console, streams: &crate::stream::Streams) {
    let mut wrote = write_str(fs, streams, &("notes.txt".to_string()), &("alpha\nbeta\ngamma\n".to_string()));
    match wrote {
        Union2::U1(_) => {
            println(console, &(format!("wrote {} bytes", *wrote.u1())));
        }
        Union2::U2(_) => {
            println(console, &(format!("write failed: {}", kind_name(&(detach((match wrote { Union2::U2(__v) => __v, _ => unreachable!() })))))));
        }
    }
    let mut text = read_to_str(fs, streams, &("notes.txt".to_string()));
    match text {
        Union2::U1(_) => {
            println(console, &(format!("read back {} bytes", crate::core_string::byte_size_platform(text.u1()))));
        }
        Union2::U2(_) => {
            println(console, &(format!("read failed: {}", kind_name(&(detach((match text { Union2::U2(__v) => __v, _ => unreachable!() })))))));
        }
    }
    let mut opened = fs.open_read(&("notes.txt".to_string()));
    match opened {
        Union2::U1(_) => {
            let mut p = lines__2((match opened { Union2::U1(__v) => __v, _ => unreachable!() }));
            while let Union2::U1(mut line) = next__21(streams, &mut p) {
                println(console, &(format!("line: {}", line)));
            }
            let mut closed = close__2(streams, p);
            if matches!(closed, Union2::U2(_)) {
                println(console, &(format!("close failed: {}", kind_name__2(&(detach((match closed { Union2::U2(__v) => __v, _ => unreachable!() })))))));
            }
        }
        Union2::U2(_) => {
            println(console, &(format!("open failed: {}", kind_name(&(detach((match opened { Union2::U2(__v) => __v, _ => unreachable!() })))))));
        }
    }
    let mut out = fs.open_append(&("notes.txt".to_string()));
    match out {
        Union2::U1(_) => {
            let mut w: OutStream = out.u1().clone();
            let mut at = streams.position__2(&w);
            let mut n = streams.write_line(&w, &("delta".to_string()));
            println(console, &(format!("appended {} bytes at offset {}", n, at)));
            let mut shut = streams.close__2(w);
            if matches!(shut, Union2::U2(_)) {
                println(console, &(format!("close failed: {}", kind_name__2(&(detach((match shut { Union2::U2(__v) => __v, _ => unreachable!() })))))));
            }
            let mut resumed = fs.open_read_at(&("notes.txt".to_string()), at);
            match resumed {
                Union2::U1(_) => {
                    let mut s: InStream = resumed.u1().clone();
                    let mut line = streams.read_line(&s);
                    match line {
                        Some(_) => {
                            println(console, &(format!("at {}: {}", at, line.as_ref().unwrap().clone())));
                        }
                        None => {
                            println(console, &(format!("at {}: end of file", at)));
                        }
                    }
                    let mut done = streams.close(s);
                    if matches!(done, Union2::U2(_)) {
                        println(console, &(format!("close failed: {}", kind_name__2(&(detach((match done { Union2::U2(__v) => __v, _ => unreachable!() })))))));
                    }
                }
                Union2::U2(_) => {
                    println(console, &(format!("reopen failed: {}", kind_name(&(detach((match resumed { Union2::U2(__v) => __v, _ => unreachable!() })))))));
                }
            }
        }
        Union2::U2(_) => {
            println(console, &(format!("append failed: {}", kind_name(&(detach((match out { Union2::U2(__v) => __v, _ => unreachable!() })))))));
        }
    }
    let mut bin = fs.open_write(&("raw.bin".to_string()));
    match bin {
        Union2::U1(_) => {
            let mut w: OutStream = bin.u1().clone();
            let mut data = bytes_of(vec![(((0) as i32) as u8), (((255) as i32) as u8), (((200) as i32) as u8)]);
            let mut n = streams.write_bytes(&w, &data);
            let mut m = streams.write(&w, &("hé".to_string()));
            println(console, &(format!("wrote {} raw bytes and {} encoded", n, m)));
            let mut shut = streams.close__2(w);
            if matches!(shut, Union2::U2(_)) {
                println(console, &(format!("close failed: {}", kind_name__2(&(detach((match shut { Union2::U2(__v) => __v, _ => unreachable!() })))))));
            }
        }
        Union2::U2(_) => {
            println(console, &(format!("raw open failed: {}", kind_name(&(detach((match bin { Union2::U2(__v) => __v, _ => unreachable!() })))))));
        }
    }
    let mut raw = fs.open_read(&("raw.bin".to_string()));
    match raw {
        Union2::U1(_) => {
            let mut s: InStream = raw.u1().clone();
            let mut head = streams.read_bytes(&s, 3);
            match head {
                Union2::U1(_) => {
                    println(console, &(format!("first three: {} = {}", crate::core_bytes::to_str_platform(&head.u1().clone()), crate::core_bytes::to_hex_platform(head.u1()))));
                }
                Union2::U2(_) => {
                    println(console, &(format!("byte read failed: {}", kind_name__2(&(detach((match head { Union2::U2(__v) => __v, _ => unreachable!() })))))));
                }
            }
            let mut tail = streams.read_all(&s);
            match tail {
                Union2::U1(_) => {
                    println(console, &(format!("the rest, as text: {}", tail.u1().clone())));
                }
                Union2::U2(_) => {
                    println(console, &(format!("decode failed: {}", kind_name__2(&(detach((match tail { Union2::U2(__v) => __v, _ => unreachable!() })))))));
                }
            }
            let mut done = streams.close(s);
            if matches!(done, Union2::U2(_)) {
                println(console, &(format!("close failed: {}", kind_name__2(&(detach((match done { Union2::U2(__v) => __v, _ => unreachable!() })))))));
            }
        }
        Union2::U2(_) => {
            println(console, &(format!("raw read failed: {}", kind_name(&(detach((match raw { Union2::U2(__v) => __v, _ => unreachable!() })))))));
        }
    }
    let mut split = fs.open_read_at(&("raw.bin".to_string()), 5i64);
    match split {
        Union2::U1(_) => {
            let mut s: InStream = split.u1().clone();
            let mut broken = streams.read_all(&s);
            match broken {
                Union2::U1(_) => {
                    println(console, &(format!("unexpected: {} decoded", broken.u1().clone())));
                }
                Union2::U2(_) => {
                    println(console, &(format!("mid-character: {}", kind_name__2(&(detach((match broken { Union2::U2(__v) => __v, _ => unreachable!() })))))));
                }
            }
            let mut done = streams.close(s);
            match done {
                Union2::U1(_) => {
                    println(console, &("unexpected: the failure was not recorded".to_string()));
                }
                Union2::U2(_) => {
                    println(console, &(format!("and again at close: {}", kind_name__2(&(detach((match done { Union2::U2(__v) => __v, _ => unreachable!() })))))));
                }
            }
        }
        Union2::U2(_) => {
            println(console, &(format!("split open failed: {}", kind_name(&(detach((match split { Union2::U2(__v) => __v, _ => unreachable!() })))))));
        }
    }
    let mut held = fs.open_read(&("raw.bin".to_string()));
    match held {
        Union2::U1(_) => {
            let mut s: InStream = held.u1().clone();
            let mut buf = mut_bytes(vec![]);
            let mut steps = 0;
            let mut moved = 0;
            let mut reading = true;
            while reading {
                crate::core_bytes::clear_platform(&mut buf);
                let mut got = streams.read_to(&s, &mut buf, 4);
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
                        println(console, &(format!("fill failed: {}", kind_name__2(&(detach((match got { Union2::U2(__v) => __v, _ => unreachable!() })))))));
                        reading = false;
                    }
                }
            }
            println(console, &(format!("filled {} bytes in {} reads, one buffer", moved, steps)));
            let mut done = streams.close(s);
            if matches!(done, Union2::U2(_)) {
                println(console, &(format!("close failed: {}", kind_name__2(&(detach((match done { Union2::U2(__v) => __v, _ => unreachable!() })))))));
            }
        }
        Union2::U2(_) => {
            println(console, &(format!("fill open failed: {}", kind_name(&(detach((match held { Union2::U2(__v) => __v, _ => unreachable!() })))))));
        }
    }
    let mut lined = fs.open_read(&("notes.txt".to_string()));
    match lined {
        Union2::U1(_) => {
            let mut s: InStream = lined.u1().clone();
            let mut line = mut_str(vec![]);
            let mut longest = 0;
            let mut reading = true;
            while reading {
                crate::core_string::clear_platform(&mut line);
                if streams.read_line_to(&s, &mut line) {
                    if crate::core_string::size_platform(&line) > longest {
                        longest = crate::core_string::size_platform(&line);
                    }
                } else {
                    reading = false;
                }
            }
            println(console, &(format!("longest line: {} characters", longest)));
            let mut done = streams.close(s);
            if matches!(done, Union2::U2(_)) {
                println(console, &(format!("close failed: {}", kind_name__2(&(detach((match done { Union2::U2(__v) => __v, _ => unreachable!() })))))));
            }
        }
        Union2::U2(_) => {
            println(console, &(format!("lines open failed: {}", kind_name(&(detach((match lined { Union2::U2(__v) => __v, _ => unreachable!() })))))));
        }
    }
    let mut ch = open_chunks(fs, streams, &("raw.bin".to_string()), 4);
    match ch {
        Union2::U1(_) => {
            let mut p = ch.u1().clone();
            let mut seen = 0;
            while let Union2::U1(mut chunk) = next__22(streams, &mut p) {
                seen = seen + crate::core_bytes::size_platform(&chunk);
            }
            println(console, &(format!("chunks saw {} bytes", seen)));
            let mut done = close__3(streams, p);
            if matches!(done, Union2::U2(_)) {
                println(console, &(format!("close failed: {}", kind_name__2(&(detach((match done { Union2::U2(__v) => __v, _ => unreachable!() })))))));
            }
        }
        Union2::U2(_) => {
            println(console, &(format!("chunks failed: {}", kind_name(&(detach((match ch { Union2::U2(__v) => __v, _ => unreachable!() })))))));
        }
    }
    let mut copied = copy_file(fs, streams, &("notes.txt".to_string()), &("notes-copy.txt".to_string()));
    match copied {
        Union2::U1(_) => {
            println(console, &(format!("copied {} bytes", *copied.u1())));
        }
        Union2::U2(_) => {
            println(console, &(format!("copy failed: {}", kind_name(&(detach((match copied { Union2::U2(__v) => __v, _ => unreachable!() })))))));
        }
    }
    let mut whole = read_to_bytes(fs, streams, &("raw.bin".to_string()));
    match whole {
        Union2::U1(_) => {
            println(console, &(format!("raw.bin is {} bytes: {}", crate::core_bytes::size_platform(whole.u1()), crate::core_bytes::to_hex_platform(whole.u1()))));
        }
        Union2::U2(_) => {
            println(console, &(format!("byte read failed: {}", kind_name(&(detach((match whole { Union2::U2(__v) => __v, _ => unreachable!() })))))));
        }
    }
    let mut failures: Vec<Union7<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, IoError, Streaming>> = vec![];
    let mut missing = read_to_str(fs, streams, &("nope.txt".to_string()));
    match missing {
        Union2::U1(_) => {
            println(console, &(format!("unexpected: {}", missing.u1().clone())));
        }
        Union2::U2(_) => {
            failures.push(detach((match missing { Union2::U2(__v) => __v, _ => unreachable!() })));
        }
    }
    let mut not_a_dir = fs.list_dir(&("notes.txt".to_string()));
    match not_a_dir {
        Union2::U1(_) => {
            println(console, &(format!("unexpected: {}", format!("[{}]", not_a_dir.u1().clone().iter().map(|__e| __e.to_string()).collect::<Vec<_>>().join(", ")))));
        }
        Union2::U2(_) => {
            failures.push(detach((match not_a_dir { Union2::U2(__v) => __v, _ => unreachable!() })));
        }
    }
    println(console, &(format!("failures: {}", (failures.len() as i32))));
    for mut kind in failures.clone() {
        println(console, &(format!("  {}", kind_name(&(kind.clone())))));
    }
    for mut name in vec!["notes.txt".to_string(), "notes-copy.txt".to_string(), "raw.bin".to_string()] {
        let mut gone = fs.delete(&name);
        if matches!(gone, Union2::U2(_)) {
            println(console, &(format!("delete failed: {}", kind_name(&(detach((match gone { Union2::U2(__v) => __v, _ => unreachable!() })))))));
        }
    }
    println(console, &("cleaned up".to_string()));
}

pub fn sandbox_edges(fs: &crate::fs::Fs, console: &crate::core_console::Console, streams: &crate::stream::Streams) {
    let mut inside = write_str(fs, streams, &("sub/../probe.txt".to_string()), &("inside\n".to_string()));
    match inside {
        Union2::U1(_) => {
            println(console, &(format!("through `..`: wrote {} bytes", *inside.u1())));
        }
        Union2::U2(_) => {
            println(console, &(format!("through `..`: {}", kind_name(&(detach((match inside { Union2::U2(__v) => __v, _ => unreachable!() })))))));
        }
    }
    let mut up = read_to_str(fs, streams, &("../secret.txt".to_string()));
    match up {
        Union2::U1(_) => {
            println(console, &("unexpected: read outside the sandbox".to_string()));
        }
        Union2::U2(_) => {
            println(console, &(format!("climbing out: {}", kind_name(&(detach((match up { Union2::U2(__v) => __v, _ => unreachable!() })))))));
        }
    }
    let mut absolute = read_to_str(fs, streams, &("/etc/hosts".to_string()));
    match absolute {
        Union2::U1(_) => {
            println(console, &("unexpected: an absolute path resolved".to_string()));
        }
        Union2::U2(_) => {
            println(console, &(format!("absolute path: {}", kind_name(&(detach((match absolute { Union2::U2(__v) => __v, _ => unreachable!() })))))));
        }
    }
    let mut probe = "probe.txt".to_string();
    println(console, &(format!("probe still there: {}", fs.exists(&probe))));
    let mut gone = fs.delete(&probe);
    if matches!(gone, Union2::U2(_)) {
        println(console, &(format!("delete failed: {}", kind_name(&(detach((match gone { Union2::U2(__v) => __v, _ => unreachable!() })))))));
    }
}

pub fn main() {
    crate::scheduler::salvo_set_protocols(vec![("Faults".to_string(), crate::core_actor::__PROTO_Faults.to_string())]);
    let console = crate::core_console::Console::shared(crate::core_console::__Platform_StdOutConsole::new());
    let raw_streams = crate::stream_host::RawStreams::shared(HostRawStreams::new());
    let streams = crate::stream::Streams::shared(DefaultStreams::new(raw_streams.clone()));
    let raw_fs = crate::fs_host::RawFs::locked(crate::fs_host::__Platform_HostRawFs::new());
    let fs = crate::fs::Fs::shared(DefaultFs::new(raw_fs.clone(), streams.clone()));
    let mut root = "tmp/files-example".to_string();
    let mut made = fs.create_dirs(&root);
    if matches!(made, Union2::U2(_)) {
        println(&console, &(format!("cannot create the working directory: {}", kind_name(&(detach((match made { Union2::U2(__v) => __v, _ => unreachable!() })))))));
        return;
    }
    println(&console, &("-- the real filesystem, scoped to one directory --".to_string()));
    if true {
        let fs2 = crate::fs::Fs::shared(RestrictedFs::new(root.clone(), fs.clone(), streams.clone()));
        workflow(&fs2, &console, &streams);
        sandbox_edges(&fs2, &console, &streams);
    }
    let mut gone = fs.delete(&root);
    if matches!(gone, Union2::U2(_)) {
        println(&console, &(format!("cleanup failed: {}", kind_name(&(detach((match gone { Union2::U2(__v) => __v, _ => unreachable!() })))))));
    }
    println(&console, &("-- the same code, with no disk at all --".to_string()));
    if true {
        let __inst = std::sync::Arc::new(std::sync::Mutex::new(MemFs::new()));
        let fs4 = crate::fs::Fs::share_locked(__inst.clone());
        let streams2 = crate::stream::Streams::share_locked(__inst.clone());
        workflow(&fs4, &console, &streams2);
    }
}
