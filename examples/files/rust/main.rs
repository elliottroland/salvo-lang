#![allow(non_snake_case, non_camel_case_types, unused_mut, unused_parens, unused_imports, dead_code, unreachable_code, unused_variables, path_statements, unused_must_use, suspicious_double_ref_op)]
#[path = "unions.rs"]
pub mod unions;
#[path = "collections.rs"]
pub mod collections;
#[path = "core/array.rs"]
pub mod core_array;
#[path = "core/bytes.rs"]
pub mod core_bytes;
#[path = "core/console.rs"]
pub mod core_console;
#[path = "core/fs.rs"]
pub mod core_fs;
#[path = "core/hostfs.rs"]
pub mod core_hostfs;
#[path = "core/iterator.rs"]
pub mod core_iterator;
#[path = "core/list.rs"]
pub mod core_list;
#[path = "core/map.rs"]
pub mod core_map;
#[path = "core/memfs.rs"]
pub mod core_memfs;
#[path = "core/nonempty.rs"]
pub mod core_nonempty;
#[path = "core/restrictedfs.rs"]
pub mod core_restrictedfs;
#[path = "core/result.rs"]
pub mod core_result;
#[path = "core/seq.rs"]
pub mod core_seq;
#[path = "core/set.rs"]
pub mod core_set;
#[path = "core/sorted.rs"]
pub mod core_sorted;
#[path = "core/string.rs"]
pub mod core_string;
#[path = "platform/core/hostfs.rs"]
pub mod platform_core_hostfs;

use crate::core_array::*;
use crate::core_bytes::*;
use crate::core_console::*;
use crate::core_fs::*;
use crate::core_hostfs::*;
use crate::core_iterator::*;
use crate::core_list::*;
use crate::core_map::*;
use crate::core_memfs::*;
use crate::core_restrictedfs::*;
use crate::core_result::*;
use crate::core_set::*;
use crate::core_sorted::*;
use crate::core_string::*;
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

pub fn workflow<__Fx: __Has_Fs + __Has_Console>(__fx: &mut __Fx) {
    let mut wrote = write_str(&mut *__fx, &("notes.txt".to_string()), &("alpha\nbeta\ngamma\n".to_string()));
    match wrote {
        Union2::U1(_) => {
            println(&mut *__fx, &(format!("wrote {} bytes", *wrote.u1())));
        }
        Union2::U2(_) => {
            println(&mut *__fx, &(format!("write failed: {}", kind_name(&(detach(wrote.u2().clone()))))));
        }
    }
    let mut text = read_to_str(&mut *__fx, &("notes.txt".to_string()));
    match text {
        Union2::U1(_) => {
            println(&mut *__fx, &(format!("read back {} bytes", (text.u1().len() as i64))));
        }
        Union2::U2(_) => {
            println(&mut *__fx, &(format!("read failed: {}", kind_name(&(detach(text.u2().clone()))))));
        }
    }
    let mut opened = __Has_Fs::__get_Fs(&mut *__fx).open_read(&("notes.txt".to_string()));
    match opened {
        Union2::U1(_) => {
            let mut p = lines(opened.u1().clone());
            while let Union2::U1(mut line) = next__3(&mut *__fx, &mut p) {
                println(&mut *__fx, &(format!("line: {}", line)));
            }
            let mut closed = close(&mut *__fx, p);
            if matches!(closed, Union2::U2(_)) {
                println(&mut *__fx, &(format!("close failed: {}", kind_name(&(detach(closed.u2().clone()))))));
            }
        }
        Union2::U2(_) => {
            println(&mut *__fx, &(format!("open failed: {}", kind_name(&(detach(opened.u2().clone()))))));
        }
    }
    let mut out = __Has_Fs::__get_Fs(&mut *__fx).open_append(&("notes.txt".to_string()));
    match out {
        Union2::U1(_) => {
            let mut w: OutStream = out.u1().clone();
            let mut at = __Has_Fs::__get_Fs(&mut *__fx).position__2(&w);
            let mut n = __Has_Fs::__get_Fs(&mut *__fx).write_line(&w, &("delta".to_string()));
            println(&mut *__fx, &(format!("appended {} bytes at offset {}", n, at)));
            let mut shut = __Has_Fs::__get_Fs(&mut *__fx).close__2(w);
            if matches!(shut, Union2::U2(_)) {
                println(&mut *__fx, &(format!("close failed: {}", kind_name(&(detach(shut.u2().clone()))))));
            }
            let mut resumed = __Has_Fs::__get_Fs(&mut *__fx).open_read_at(&("notes.txt".to_string()), at);
            match resumed {
                Union2::U1(_) => {
                    let mut s: InStream = resumed.u1().clone();
                    let mut line = __Has_Fs::__get_Fs(&mut *__fx).read_line(&s);
                    match line {
                        Some(_) => {
                            println(&mut *__fx, &(format!("at {}: {}", at, line.as_ref().unwrap().clone())));
                        }
                        None => {
                            println(&mut *__fx, &(format!("at {}: end of file", at)));
                        }
                    }
                    let mut done = __Has_Fs::__get_Fs(&mut *__fx).close(s);
                    if matches!(done, Union2::U2(_)) {
                        println(&mut *__fx, &(format!("close failed: {}", kind_name(&(detach(done.u2().clone()))))));
                    }
                }
                Union2::U2(_) => {
                    println(&mut *__fx, &(format!("reopen failed: {}", kind_name(&(detach(resumed.u2().clone()))))));
                }
            }
        }
        Union2::U2(_) => {
            println(&mut *__fx, &(format!("append failed: {}", kind_name(&(detach(out.u2().clone()))))));
        }
    }
    let mut bin = __Has_Fs::__get_Fs(&mut *__fx).open_write(&("raw.bin".to_string()));
    match bin {
        Union2::U1(_) => {
            let mut w: OutStream = bin.u1().clone();
            let mut data = vec![(((0) as i32) as u8), (((255) as i32) as u8), (((200) as i32) as u8)];
            let mut n = __Has_Fs::__get_Fs(&mut *__fx).write_bytes(&w, &data);
            let mut m = __Has_Fs::__get_Fs(&mut *__fx).write(&w, &("hé".to_string()));
            println(&mut *__fx, &(format!("wrote {} raw bytes and {} encoded", n, m)));
            let mut shut = __Has_Fs::__get_Fs(&mut *__fx).close__2(w);
            if matches!(shut, Union2::U2(_)) {
                println(&mut *__fx, &(format!("close failed: {}", kind_name(&(detach(shut.u2().clone()))))));
            }
        }
        Union2::U2(_) => {
            println(&mut *__fx, &(format!("raw open failed: {}", kind_name(&(detach(bin.u2().clone()))))));
        }
    }
    let mut raw = __Has_Fs::__get_Fs(&mut *__fx).open_read(&("raw.bin".to_string()));
    match raw {
        Union2::U1(_) => {
            let mut s: InStream = raw.u1().clone();
            let mut head = __Has_Fs::__get_Fs(&mut *__fx).read_bytes(&s, 3);
            match head {
                Union2::U1(_) => {
                    println(&mut *__fx, &(format!("first three: {} = {}", format!("[{}]", head.u1().clone().iter().map(|__b| __b.to_string()).collect::<Vec<String>>().join(", ")), head.u1().iter().map(|__b| format!("{:02x}", __b)).collect::<String>())));
                }
                Union2::U2(_) => {
                    println(&mut *__fx, &(format!("byte read failed: {}", kind_name(&(detach(head.u2().clone()))))));
                }
            }
            let mut tail = __Has_Fs::__get_Fs(&mut *__fx).read_all(&s);
            match tail {
                Union2::U1(_) => {
                    println(&mut *__fx, &(format!("the rest, as text: {}", tail.u1().clone())));
                }
                Union2::U2(_) => {
                    println(&mut *__fx, &(format!("decode failed: {}", kind_name(&(detach(tail.u2().clone()))))));
                }
            }
            let mut done = __Has_Fs::__get_Fs(&mut *__fx).close(s);
            if matches!(done, Union2::U2(_)) {
                println(&mut *__fx, &(format!("close failed: {}", kind_name(&(detach(done.u2().clone()))))));
            }
        }
        Union2::U2(_) => {
            println(&mut *__fx, &(format!("raw read failed: {}", kind_name(&(detach(raw.u2().clone()))))));
        }
    }
    let mut split = __Has_Fs::__get_Fs(&mut *__fx).open_read_at(&("raw.bin".to_string()), 5i64);
    match split {
        Union2::U1(_) => {
            let mut s: InStream = split.u1().clone();
            let mut broken = __Has_Fs::__get_Fs(&mut *__fx).read_all(&s);
            match broken {
                Union2::U1(_) => {
                    println(&mut *__fx, &(format!("unexpected: {} decoded", broken.u1().clone())));
                }
                Union2::U2(_) => {
                    println(&mut *__fx, &(format!("mid-character: {}", kind_name(&(detach(broken.u2().clone()))))));
                }
            }
            let mut done = __Has_Fs::__get_Fs(&mut *__fx).close(s);
            match done {
                Union2::U1(_) => {
                    println(&mut *__fx, &("unexpected: the failure was not recorded".to_string()));
                }
                Union2::U2(_) => {
                    println(&mut *__fx, &(format!("and again at close: {}", kind_name(&(detach(done.u2().clone()))))));
                }
            }
        }
        Union2::U2(_) => {
            println(&mut *__fx, &(format!("split open failed: {}", kind_name(&(detach(split.u2().clone()))))));
        }
    }
    let mut held = __Has_Fs::__get_Fs(&mut *__fx).open_read(&("raw.bin".to_string()));
    match held {
        Union2::U1(_) => {
            let mut s: InStream = held.u1().clone();
            let mut buf = Vec::<u8>::new();
            let mut steps = 0;
            let mut moved = 0;
            let mut reading = true;
            while reading {
                buf.clear();
                let mut got = __Has_Fs::__get_Fs(&mut *__fx).read_to(&s, &mut buf, 4);
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
                        println(&mut *__fx, &(format!("fill failed: {}", kind_name(&(detach(got.u2().clone()))))));
                        reading = false;
                    }
                }
            }
            println(&mut *__fx, &(format!("filled {} bytes in {} reads, one buffer", moved, steps)));
            let mut done = __Has_Fs::__get_Fs(&mut *__fx).close(s);
            if matches!(done, Union2::U2(_)) {
                println(&mut *__fx, &(format!("close failed: {}", kind_name(&(detach(done.u2().clone()))))));
            }
        }
        Union2::U2(_) => {
            println(&mut *__fx, &(format!("fill open failed: {}", kind_name(&(detach(held.u2().clone()))))));
        }
    }
    let mut lined = __Has_Fs::__get_Fs(&mut *__fx).open_read(&("notes.txt".to_string()));
    match lined {
        Union2::U1(_) => {
            let mut s: InStream = lined.u1().clone();
            let mut line = String::new();
            let mut longest = 0;
            let mut reading = true;
            while reading {
                line.clear();
                if __Has_Fs::__get_Fs(&mut *__fx).read_line_to(&s, &mut line) {
                    if ((line.chars().count() as i32) > longest) {
                        longest = (line.chars().count() as i32);
                    }
                } else {
                    reading = false;
                }
            }
            println(&mut *__fx, &(format!("longest line: {} characters", longest)));
            let mut done = __Has_Fs::__get_Fs(&mut *__fx).close(s);
            if matches!(done, Union2::U2(_)) {
                println(&mut *__fx, &(format!("close failed: {}", kind_name(&(detach(done.u2().clone()))))));
            }
        }
        Union2::U2(_) => {
            println(&mut *__fx, &(format!("lines open failed: {}", kind_name(&(detach(lined.u2().clone()))))));
        }
    }
    let mut ch = open_chunks(&mut *__fx, &("raw.bin".to_string()), 4);
    match ch {
        Union2::U1(_) => {
            let mut p = ch.u1().clone();
            let mut seen = 0;
            while let Union2::U1(mut chunk) = next__4(&mut *__fx, &mut p) {
                seen = seen + (chunk.len() as i32);
            }
            println(&mut *__fx, &(format!("pass saw {} bytes", seen)));
            let mut done = close__2(&mut *__fx, p);
            if matches!(done, Union2::U2(_)) {
                println(&mut *__fx, &(format!("close failed: {}", kind_name(&(detach(done.u2().clone()))))));
            }
        }
        Union2::U2(_) => {
            println(&mut *__fx, &(format!("chunks failed: {}", kind_name(&(detach(ch.u2().clone()))))));
        }
    }
    let mut copied = copy_file(&mut *__fx, &("notes.txt".to_string()), &("notes-copy.txt".to_string()));
    match copied {
        Union2::U1(_) => {
            println(&mut *__fx, &(format!("copied {} bytes", *copied.u1())));
        }
        Union2::U2(_) => {
            println(&mut *__fx, &(format!("copy failed: {}", kind_name(&(detach(copied.u2().clone()))))));
        }
    }
    let mut whole = read_to_bytes(&mut *__fx, &("raw.bin".to_string()));
    match whole {
        Union2::U1(_) => {
            println(&mut *__fx, &(format!("raw.bin is {} bytes: {}", (whole.u1().len() as i32), whole.u1().iter().map(|__b| format!("{:02x}", __b)).collect::<String>())));
        }
        Union2::U2(_) => {
            println(&mut *__fx, &(format!("byte read failed: {}", kind_name(&(detach(whole.u2().clone()))))));
        }
    }
    let mut failures: Vec<Union8<NotFound, PermissionDenied, AlreadyExists, NotADirectory, PathEscapes, InvalidUtf8, StaleHandle, IoError>> = vec![];
    let mut missing = read_to_str(&mut *__fx, &("nope.txt".to_string()));
    match missing {
        Union2::U1(_) => {
            println(&mut *__fx, &(format!("unexpected: {}", missing.u1().clone())));
        }
        Union2::U2(_) => {
            failures.push(detach(missing.u2().clone()));
        }
    }
    let mut not_a_dir = __Has_Fs::__get_Fs(&mut *__fx).list_dir(&("notes.txt".to_string()));
    match not_a_dir {
        Union2::U1(_) => {
            println(&mut *__fx, &(format!("unexpected: {}", format!("[{}]", not_a_dir.u1().clone().iter().map(|__e| __e.to_string()).collect::<Vec<_>>().join(", ")))));
        }
        Union2::U2(_) => {
            failures.push(detach(not_a_dir.u2().clone()));
        }
    }
    println(&mut *__fx, &(format!("failures: {}", (failures.len() as i32))));
    for mut kind in failures.clone() {
        println(&mut *__fx, &(format!("  {}", kind_name(&(kind.clone())))));
    }
    for mut name in vec!["notes.txt".to_string(), "notes-copy.txt".to_string(), "raw.bin".to_string()] {
        let mut gone = __Has_Fs::__get_Fs(&mut *__fx).delete(&name);
        if matches!(gone, Union2::U2(_)) {
            println(&mut *__fx, &(format!("delete failed: {}", kind_name(&(detach(gone.u2().clone()))))));
        }
    }
    println(&mut *__fx, &("cleaned up".to_string()));
}

pub fn sandbox_edges<__Fx: __Has_Fs + __Has_Console>(__fx: &mut __Fx) {
    let mut inside = write_str(&mut *__fx, &("sub/../probe.txt".to_string()), &("inside\n".to_string()));
    match inside {
        Union2::U1(_) => {
            println(&mut *__fx, &(format!("through `..`: wrote {} bytes", *inside.u1())));
        }
        Union2::U2(_) => {
            println(&mut *__fx, &(format!("through `..`: {}", kind_name(&(detach(inside.u2().clone()))))));
        }
    }
    let mut up = read_to_str(&mut *__fx, &("../secret.txt".to_string()));
    match up {
        Union2::U1(_) => {
            println(&mut *__fx, &("unexpected: read outside the sandbox".to_string()));
        }
        Union2::U2(_) => {
            println(&mut *__fx, &(format!("climbing out: {}", kind_name(&(detach(up.u2().clone()))))));
        }
    }
    let mut absolute = read_to_str(&mut *__fx, &("/etc/hosts".to_string()));
    match absolute {
        Union2::U1(_) => {
            println(&mut *__fx, &("unexpected: an absolute path resolved".to_string()));
        }
        Union2::U2(_) => {
            println(&mut *__fx, &(format!("absolute path: {}", kind_name(&(detach(absolute.u2().clone()))))));
        }
    }
    let mut probe = "probe.txt".to_string();
    { let __a1 = &(format!("probe still there: {}", __Has_Fs::__get_Fs(&mut *__fx).exists(&probe))); println(&mut *__fx, __a1) };
    let mut gone = __Has_Fs::__get_Fs(&mut *__fx).delete(&probe);
    if matches!(gone, Union2::U2(_)) {
        println(&mut *__fx, &(format!("delete failed: {}", kind_name(&(detach(gone.u2().clone()))))));
    }
}

pub fn main() {
    let mut __fx = __Fx_main_1 { __h: StdOutConsole::new() };
    let mut __bind = crate::core_hostfs::__Lock_RawFs::new(crate::platform_core_hostfs::HostRawFs::new());
    let __handle = crate::core_hostfs::__Mon_RawFs::new(Box::new(__bind.clone()));
    let mut __fx2 = __Fx_main_2 { __outer: &mut __fx, __h: __bind };
    let mut __bind2 = DefaultFs::new(__handle.clone());
    let __handle2 = crate::core_fs::__Mon_Fs::new(Box::new(__bind2.clone()));
    let mut __fx3 = __Fx_main_3 { __outer: &mut __fx2, __h: __bind2 };
    let mut root = "tmp/files-example".to_string();
    let mut made = __Has_Fs::__get_Fs(&mut __fx3).create_dirs(&root);
    if matches!(made, Union2::U2(_)) {
        println(&mut __fx3, &(format!("cannot create the working directory: {}", kind_name(&(detach(made.u2().clone()))))));
        return;
    }
    println(&mut __fx3, &("-- the real filesystem, scoped to one directory --".to_string()));
    if true {
        let mut __bind3 = RestrictedFs::new(root.clone(), __handle2.clone());
        let __handle3 = crate::core_fs::__Mon_Fs::new(Box::new(__bind3.clone()));
        let mut __fx4 = __Fx_main_4 { __outer: &mut __fx3, __h: __bind3 };
        workflow(&mut __fx4);
        sandbox_edges(&mut __fx4);
    }
    let mut gone = __Has_Fs::__get_Fs(&mut __fx3).delete(&root);
    if matches!(gone, Union2::U2(_)) {
        println(&mut __fx3, &(format!("cleanup failed: {}", kind_name(&(detach(gone.u2().clone()))))));
    }
    println(&mut __fx3, &("-- the same code, with no disk at all --".to_string()));
    if true {
        let mut __bind4 = crate::core_fs::__Lock_Fs::new(MemFs::new());
        let __handle4 = crate::core_fs::__Mon_Fs::new(Box::new(__bind4.clone()));
        let mut __fx5 = __Fx_main_4 { __outer: &mut __fx3, __h: __bind4 };
        workflow(&mut __fx5);
    }
}

pub struct __Fx_main_1<__H> {
    __h: __H,
}

impl<__H: Console> __Has_Console for __Fx_main_1<__H> {
    fn __get_Console(&mut self) -> &mut dyn Console {
        &mut self.__h
    }
}

pub struct __Fx_main_2<'a, __H> {
    __outer: &'a mut dyn __Has_Console,
    __h: __H,
}

impl<'a, __H> __Has_Console for __Fx_main_2<'a, __H> {
    fn __get_Console(&mut self) -> &mut dyn Console {
        __Has_Console::__get_Console(&mut *self.__outer)
    }
}

impl<'a, __H: RawFs> __Has_RawFs for __Fx_main_2<'a, __H> {
    fn __get_RawFs(&mut self) -> &mut dyn RawFs {
        &mut self.__h
    }
}

pub trait __Prov_Console_RawFs: __Has_Console + __Has_RawFs {}
impl<T: __Has_Console + __Has_RawFs + ?Sized> __Prov_Console_RawFs for T {}

pub struct __Fx_main_3<'a, __H> {
    __outer: &'a mut dyn __Prov_Console_RawFs,
    __h: __H,
}

impl<'a, __H> __Has_Console for __Fx_main_3<'a, __H> {
    fn __get_Console(&mut self) -> &mut dyn Console {
        __Has_Console::__get_Console(&mut *self.__outer)
    }
}

impl<'a, __H> __Has_RawFs for __Fx_main_3<'a, __H> {
    fn __get_RawFs(&mut self) -> &mut dyn RawFs {
        __Has_RawFs::__get_RawFs(&mut *self.__outer)
    }
}

impl<'a, __H: Fs> __Has_Fs for __Fx_main_3<'a, __H> {
    fn __get_Fs(&mut self) -> &mut dyn Fs {
        &mut self.__h
    }
}

pub trait __Prov_Console_Fs_RawFs: __Has_Console + __Has_Fs + __Has_RawFs {}
impl<T: __Has_Console + __Has_Fs + __Has_RawFs + ?Sized> __Prov_Console_Fs_RawFs for T {}

pub struct __Fx_main_4<'a, __H> {
    __outer: &'a mut dyn __Prov_Console_Fs_RawFs,
    __h: __H,
}

impl<'a, __H> __Has_Console for __Fx_main_4<'a, __H> {
    fn __get_Console(&mut self) -> &mut dyn Console {
        __Has_Console::__get_Console(&mut *self.__outer)
    }
}

impl<'a, __H> __Has_RawFs for __Fx_main_4<'a, __H> {
    fn __get_RawFs(&mut self) -> &mut dyn RawFs {
        __Has_RawFs::__get_RawFs(&mut *self.__outer)
    }
}

impl<'a, __H: Fs> __Has_Fs for __Fx_main_4<'a, __H> {
    fn __get_Fs(&mut self) -> &mut dyn Fs {
        &mut self.__h
    }
}
