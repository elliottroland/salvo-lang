#![allow(non_snake_case, non_camel_case_types, unused_mut, unused_parens, unused_imports, dead_code, unreachable_code, unused_variables, path_statements, unused_must_use, suspicious_double_ref_op, non_upper_case_globals, unused_braces, unused_assignments)]
#[path = "unions/mod.rs"]
pub mod unions;
#[path = "scheduler.rs"]
pub mod scheduler;
#[path = "hoststreams.rs"]
pub mod hoststreams;
#[path = "hosttime.rs"]
pub mod hosttime;
#[path = "wire.rs"]
pub mod wire;
#[path = "std/core/actor.rs"]
pub mod std_core_actor;
#[path = "std/core/basic.rs"]
pub mod std_core_basic;
#[path = "std/core/bytes.rs"]
pub mod std_core_bytes;
#[path = "std/core/checked.rs"]
pub mod std_core_checked;
#[path = "std/core/compare.rs"]
pub mod std_core_compare;
#[path = "std/core/console.rs"]
pub mod std_core_console;
#[path = "std/core/deque.rs"]
pub mod std_core_deque;
#[path = "std/core/index.rs"]
pub mod std_core_index;
#[path = "std/core/iterator.rs"]
pub mod std_core_iterator;
#[path = "std/core/list.rs"]
pub mod std_core_list;
#[path = "std/core/map.rs"]
pub mod std_core_map;
#[path = "std/core/result.rs"]
pub mod std_core_result;
#[path = "std/core/set.rs"]
pub mod std_core_set;
#[path = "std/core/sorted.rs"]
pub mod std_core_sorted;
#[path = "std/core/string.rs"]
pub mod std_core_string;
#[path = "std/fs.rs"]
pub mod std_fs;
#[path = "std/fs/host.rs"]
pub mod std_fs_host;
#[path = "std/fs/mem.rs"]
pub mod std_fs_mem;
#[path = "std/fs/path.rs"]
pub mod std_fs_path;
#[path = "std/fs/restricted.rs"]
pub mod std_fs_restricted;
#[path = "std/runtime.rs"]
pub mod std_runtime;
#[path = "std/runtime/routing.rs"]
pub mod std_runtime_routing;
#[path = "std/runtime/streams.rs"]
pub mod std_runtime_streams;
#[path = "std/stream.rs"]
pub mod std_stream;
#[path = "std/stream/host.rs"]
pub mod std_stream_host;
#[path = "platform/std/core/bytes.rs"]
pub mod platform_std_core_bytes;
#[path = "platform/std/core/console.rs"]
pub mod platform_std_core_console;
#[path = "platform/std/core/deque.rs"]
pub mod platform_std_core_deque;
#[path = "platform/std/core/list.rs"]
pub mod platform_std_core_list;
#[path = "platform/std/core/map.rs"]
pub mod platform_std_core_map;
#[path = "platform/std/core/set.rs"]
pub mod platform_std_core_set;
#[path = "platform/std/core/sorted.rs"]
pub mod platform_std_core_sorted;
#[path = "platform/std/core/string.rs"]
pub mod platform_std_core_string;
#[path = "platform/std/fs/host.rs"]
pub mod platform_std_fs_host;
#[path = "platform/std/runtime/routing.rs"]
pub mod platform_std_runtime_routing;
#[path = "platform/std/runtime/streams.rs"]
pub mod platform_std_runtime_streams;
#[path = "platform/std/runtime.rs"]
pub mod platform_std_runtime;

use crate::std_fs::AlreadyExists;
use crate::std_core_bytes::Bytes;
use crate::std_core_checked::Checked;
use crate::std_stream::Chunks;
use crate::std_core_console::Console;
use crate::std_fs_host::DefaultFs;
use crate::std_stream_host::DefaultStreams;
use crate::std_core_iterator::Finished;
use crate::std_fs::Fs;
use crate::std_stream_host::HostRawStreams;
use crate::std_stream::InStream;
use crate::std_stream::InvalidUtf8;
use crate::std_fs::IoError;
use crate::std_stream::Lines;
use crate::std_core_string::Mut_Str;
use crate::std_fs::NotADirectory;
use crate::std_fs::NotFound;
use crate::std_stream::OutStream;
use crate::std_fs_path::Path;
use crate::std_fs::PathEscapes;
use crate::std_fs::PermissionDenied;
use crate::std_fs_restricted::RestrictedFs;
use crate::std_stream::StreamFailed;
use crate::std_fs::Streaming;
use crate::std_stream::Streams;
use crate::std_core_list::add_platform;
use crate::std_core_string::byte_size_platform;
use crate::std_core_bytes::bytes_of;
use crate::std_stream::close__Chunks;
use crate::std_stream::close__Lines;
use crate::std_fs::copy_file;
use crate::std_core_checked::detach;
use crate::std_stream::lines;
use crate::std_core_bytes::mut_bytes;
use crate::std_stream::next__Chunks;
use crate::std_stream::next__Lines;
use crate::std_fs::open_chunks;
use crate::std_fs_path::path;
use crate::std_core_console::println;
use crate::std_fs::read_to_bytes;
use crate::std_fs::read_to_str;
use crate::std_core_bytes::to_hex_platform;
use crate::std_core_list::to_str;
use crate::std_core_bytes::to_str_platform;
use crate::std_fs::write_str;


pub fn kind_name__FsError(mut kind: crate::unions::Union7<&crate::std_fs::NotFound, &crate::std_fs::PermissionDenied, &crate::std_fs::AlreadyExists, &crate::std_fs::NotADirectory, &crate::std_fs::PathEscapes, &crate::std_fs::IoError, &crate::std_fs::Streaming>) -> String {
    if matches!(kind, crate::unions::Union7::U1(_)) {
        let mut kind_1 = match kind { crate::unions::Union7::U1(__v) => __v, _ => unreachable!() };
        return String::from("not found");
    };
    if matches!(kind, crate::unions::Union7::U4(_)) {
        let mut kind_2 = match kind { crate::unions::Union7::U4(__v) => __v, _ => unreachable!() };
        return String::from("not a directory");
    };
    if matches!(kind, crate::unions::Union7::U5(_)) {
        let mut kind_3 = match kind { crate::unions::Union7::U5(__v) => __v, _ => unreachable!() };
        return String::from("escapes the sandbox");
    };
    if matches!(kind, crate::unions::Union7::U7(_)) {
        let mut kind_4 = match kind { crate::unions::Union7::U7(__v) => __v, _ => unreachable!() };
        return crate::kind_name__StreamError((&kind_4.error).as_ref());
    };
    return String::from("other");
}

pub fn kind_name__StreamError(mut kind: crate::unions::Union2<&crate::std_stream::InvalidUtf8, &crate::std_stream::StreamFailed>) -> String {
    if matches!(kind, crate::unions::Union2::U1(_)) {
        let mut kind_1 = match kind { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
        return String::from("not valid UTF-8");
    };
    return String::from("other");
}

pub fn workflow(fs: &crate::std_fs::Fs, console: &crate::std_core_console::Console, streams: &crate::std_stream::Streams) {
    let mut wrote: crate::unions::Union2<i64, crate::std_core_checked::Checked<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>> = crate::std_fs::write_str(fs, streams, &crate::std_fs_path::path(&String::from("notes.txt")), &String::from("alpha\nbeta\ngamma\n"));
    if matches!(wrote, crate::unions::Union2::U1(_)) {
        let mut wrote_1 = match &wrote { crate::unions::Union2::U1(__v) => *__v, _ => unreachable!() };
        crate::std_core_console::println(console, &format!("wrote {} bytes", wrote_1));
    } else if matches!(wrote, crate::unions::Union2::U2(_)) {
        let mut wrote_2 = match wrote { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        crate::std_core_console::println(console, &format!("write failed: {}", crate::kind_name__FsError((&(crate::std_core_checked::detach::<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>(wrote_2))).as_ref())));
    } else {
        panic!("salvo: unhandled union arm at main:70:5");
    };
    let mut text: crate::unions::Union2<String, crate::std_core_checked::Checked<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>> = crate::std_fs::read_to_str(fs, streams, &crate::std_fs_path::path(&String::from("notes.txt")));
    if matches!(text, crate::unions::Union2::U1(_)) {
        let mut text_3 = match &text { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
        crate::std_core_console::println(console, &format!("read back {} bytes", crate::std_core_string::byte_size_platform(text_3)));
    } else if matches!(text, crate::unions::Union2::U2(_)) {
        let mut text_4 = match text { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        crate::std_core_console::println(console, &format!("read failed: {}", crate::kind_name__FsError((&(crate::std_core_checked::detach::<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>(text_4))).as_ref())));
    } else {
        panic!("salvo: unhandled union arm at main:75:5");
    };
    let mut opened: crate::unions::Union2<crate::std_stream::InStream, crate::std_core_checked::Checked<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>> = fs.open_read(&crate::std_fs_path::path(&String::from("notes.txt")));
    if matches!(opened, crate::unions::Union2::U1(_)) {
        let mut opened_5 = match opened { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
        let mut p: crate::std_stream::Lines = crate::std_stream::lines(opened_5);
        loop {
            let mut __step_7: crate::unions::Union2<String, crate::std_core_iterator::Finished> = crate::std_stream::next__Lines(streams, &mut p);
            if matches!(__step_7, crate::unions::Union2::U1(_)) {
                let mut __emitted_8 = match __step_7 { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
                let mut line: String = __emitted_8;
                crate::std_core_console::println(console, &format!("line: {}", line));
            } else {
                break;
            };
        }
        let mut closed: crate::unions::Union2<(), crate::std_core_checked::Checked<crate::unions::Union2<crate::std_stream::InvalidUtf8, crate::std_stream::StreamFailed>>> = crate::std_stream::close__Lines(streams, p);
        if matches!(closed, crate::unions::Union2::U2(_)) {
            let mut closed_9 = match closed { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            crate::std_core_console::println(console, &format!("close failed: {}", crate::kind_name__StreamError((&(crate::std_core_checked::detach::<crate::unions::Union2<crate::std_stream::InvalidUtf8, crate::std_stream::StreamFailed>>(closed_9))).as_ref())));
        };
    } else if matches!(opened, crate::unions::Union2::U2(_)) {
        let mut opened_10 = match opened { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        crate::std_core_console::println(console, &format!("open failed: {}", crate::kind_name__FsError((&(crate::std_core_checked::detach::<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>(opened_10))).as_ref())));
    } else {
        panic!("salvo: unhandled union arm at main:84:5");
    };
    let mut out: crate::unions::Union2<crate::std_stream::OutStream, crate::std_core_checked::Checked<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>> = fs.open_append(&crate::std_fs_path::path(&String::from("notes.txt")));
    if matches!(out, crate::unions::Union2::U1(_)) {
        let mut out_11 = match out { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
        let mut w: crate::std_stream::OutStream = out_11;
        let mut at: i64 = streams.position__OutStream(&w);
        let mut n: i64 = streams.write_line(&w, &String::from("delta"));
        crate::std_core_console::println(console, &format!("appended {} bytes at offset {}", n, at));
        let mut shut: crate::unions::Union2<(), crate::std_core_checked::Checked<crate::unions::Union2<crate::std_stream::InvalidUtf8, crate::std_stream::StreamFailed>>> = streams.close__OutStream(w);
        if matches!(shut, crate::unions::Union2::U2(_)) {
            let mut shut_12 = match shut { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            crate::std_core_console::println(console, &format!("close failed: {}", crate::kind_name__StreamError((&(crate::std_core_checked::detach::<crate::unions::Union2<crate::std_stream::InvalidUtf8, crate::std_stream::StreamFailed>>(shut_12))).as_ref())));
        };
        let mut resumed: crate::unions::Union2<crate::std_stream::InStream, crate::std_core_checked::Checked<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>> = fs.open_read_at(&crate::std_fs_path::path(&String::from("notes.txt")), at);
        if matches!(resumed, crate::unions::Union2::U1(_)) {
            let mut resumed_13 = match resumed { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
            let mut s: crate::std_stream::InStream = resumed_13;
            let mut line: Option<String> = streams.read_line(&s);
            if line.is_some() {
                let mut line_14 = line.as_ref().unwrap();
                crate::std_core_console::println(console, &format!("at {}: {}", at, line_14));
            } else if line.is_none() {
                crate::std_core_console::println(console, &format!("at {}: end of file", at));
            } else {
                panic!("salvo: unhandled union arm at main:123:21");
            };
            let mut done: crate::unions::Union2<(), crate::std_core_checked::Checked<crate::unions::Union2<crate::std_stream::InvalidUtf8, crate::std_stream::StreamFailed>>> = streams.close__InStream(s);
            if matches!(done, crate::unions::Union2::U2(_)) {
                let mut done_15 = match done { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
                crate::std_core_console::println(console, &format!("close failed: {}", crate::kind_name__StreamError((&(crate::std_core_checked::detach::<crate::unions::Union2<crate::std_stream::InvalidUtf8, crate::std_stream::StreamFailed>>(done_15))).as_ref())));
            };
        } else if matches!(resumed, crate::unions::Union2::U2(_)) {
            let mut resumed_16 = match resumed { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            crate::std_core_console::println(console, &format!("reopen failed: {}", crate::kind_name__FsError((&(crate::std_core_checked::detach::<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>(resumed_16))).as_ref())));
        } else {
            panic!("salvo: unhandled union arm at main:119:13");
        };
    } else if matches!(out, crate::unions::Union2::U2(_)) {
        let mut out_17 = match out { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        crate::std_core_console::println(console, &format!("append failed: {}", crate::kind_name__FsError((&(crate::std_core_checked::detach::<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>(out_17))).as_ref())));
    } else {
        panic!("salvo: unhandled union arm at main:102:5");
    };
    let mut bin: crate::unions::Union2<crate::std_stream::OutStream, crate::std_core_checked::Checked<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>> = fs.open_write(&crate::std_fs_path::path(&String::from("raw.bin")));
    if matches!(bin, crate::unions::Union2::U1(_)) {
        let mut bin_18 = match bin { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
        let mut w: crate::std_stream::OutStream = bin_18;
        let mut data: crate::std_core_bytes::Bytes = crate::std_core_bytes::bytes_of(vec![(((0i32) as i32) as u8), (((255i32) as i32) as u8), (((200i32) as i32) as u8)]);
        let mut n: i64 = streams.write_bytes(&w, &data);
        let mut m: i64 = streams.write(&w, &String::from("hé"));
        crate::std_core_console::println(console, &format!("wrote {} raw bytes and {} encoded", n, m));
        let mut shut: crate::unions::Union2<(), crate::std_core_checked::Checked<crate::unions::Union2<crate::std_stream::InvalidUtf8, crate::std_stream::StreamFailed>>> = streams.close__OutStream(w);
        if matches!(shut, crate::unions::Union2::U2(_)) {
            let mut shut_19 = match shut { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            crate::std_core_console::println(console, &format!("close failed: {}", crate::kind_name__StreamError((&(crate::std_core_checked::detach::<crate::unions::Union2<crate::std_stream::InvalidUtf8, crate::std_stream::StreamFailed>>(shut_19))).as_ref())));
        };
    } else if matches!(bin, crate::unions::Union2::U2(_)) {
        let mut bin_20 = match bin { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        crate::std_core_console::println(console, &format!("raw open failed: {}", crate::kind_name__FsError((&(crate::std_core_checked::detach::<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>(bin_20))).as_ref())));
    } else {
        panic!("salvo: unhandled union arm at main:143:5");
    };
    let mut raw: crate::unions::Union2<crate::std_stream::InStream, crate::std_core_checked::Checked<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>> = fs.open_read(&crate::std_fs_path::path(&String::from("raw.bin")));
    if matches!(raw, crate::unions::Union2::U1(_)) {
        let mut raw_21 = match raw { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
        let mut s: crate::std_stream::InStream = raw_21;
        let mut head: crate::unions::Union2<crate::std_core_bytes::Bytes, crate::std_core_checked::Checked<crate::unions::Union2<crate::std_stream::InvalidUtf8, crate::std_stream::StreamFailed>>> = streams.read_bytes(&s, 3i32);
        if matches!(head, crate::unions::Union2::U1(_)) {
            let mut head_22 = match &head { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
            crate::std_core_console::println(console, &format!("first three: {} = {}", crate::std_core_bytes::to_str_platform(head_22), crate::std_core_bytes::to_hex_platform(head_22)));
        } else if matches!(head, crate::unions::Union2::U2(_)) {
            let mut head_23 = match head { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            crate::std_core_console::println(console, &format!("byte read failed: {}", crate::kind_name__StreamError((&(crate::std_core_checked::detach::<crate::unions::Union2<crate::std_stream::InvalidUtf8, crate::std_stream::StreamFailed>>(head_23))).as_ref())));
        } else {
            panic!("salvo: unhandled union arm at main:164:13");
        };
        let mut tail: crate::unions::Union2<String, crate::std_core_checked::Checked<crate::unions::Union2<crate::std_stream::InvalidUtf8, crate::std_stream::StreamFailed>>> = streams.read_all(&s);
        if matches!(tail, crate::unions::Union2::U1(_)) {
            let mut tail_24 = match &tail { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
            crate::std_core_console::println(console, &format!("the rest, as text: {}", tail_24));
        } else if matches!(tail, crate::unions::Union2::U2(_)) {
            let mut tail_25 = match tail { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            crate::std_core_console::println(console, &format!("decode failed: {}", crate::kind_name__StreamError((&(crate::std_core_checked::detach::<crate::unions::Union2<crate::std_stream::InvalidUtf8, crate::std_stream::StreamFailed>>(tail_25))).as_ref())));
        } else {
            panic!("salvo: unhandled union arm at main:171:13");
        };
        let mut done: crate::unions::Union2<(), crate::std_core_checked::Checked<crate::unions::Union2<crate::std_stream::InvalidUtf8, crate::std_stream::StreamFailed>>> = streams.close__InStream(s);
        if matches!(done, crate::unions::Union2::U2(_)) {
            let mut done_26 = match done { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            crate::std_core_console::println(console, &format!("close failed: {}", crate::kind_name__StreamError((&(crate::std_core_checked::detach::<crate::unions::Union2<crate::std_stream::InvalidUtf8, crate::std_stream::StreamFailed>>(done_26))).as_ref())));
        };
    } else if matches!(raw, crate::unions::Union2::U2(_)) {
        let mut raw_27 = match raw { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        crate::std_core_console::println(console, &format!("raw read failed: {}", crate::kind_name__FsError((&(crate::std_core_checked::detach::<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>(raw_27))).as_ref())));
    } else {
        panic!("salvo: unhandled union arm at main:160:5");
    };
    let mut split: crate::unions::Union2<crate::std_stream::InStream, crate::std_core_checked::Checked<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>> = fs.open_read_at(&crate::std_fs_path::path(&String::from("raw.bin")), 5i64);
    if matches!(split, crate::unions::Union2::U1(_)) {
        let mut split_28 = match split { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
        let mut s: crate::std_stream::InStream = split_28;
        let mut broken: crate::unions::Union2<String, crate::std_core_checked::Checked<crate::unions::Union2<crate::std_stream::InvalidUtf8, crate::std_stream::StreamFailed>>> = streams.read_all(&s);
        if matches!(broken, crate::unions::Union2::U1(_)) {
            let mut broken_29 = match &broken { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
            crate::std_core_console::println(console, &format!("unexpected: {} decoded", broken_29));
        } else if matches!(broken, crate::unions::Union2::U2(_)) {
            let mut broken_30 = match broken { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            crate::std_core_console::println(console, &format!("mid-character: {}", crate::kind_name__StreamError((&(crate::std_core_checked::detach::<crate::unions::Union2<crate::std_stream::InvalidUtf8, crate::std_stream::StreamFailed>>(broken_30))).as_ref())));
        } else {
            panic!("salvo: unhandled union arm at main:191:13");
        };
        let mut done: crate::unions::Union2<(), crate::std_core_checked::Checked<crate::unions::Union2<crate::std_stream::InvalidUtf8, crate::std_stream::StreamFailed>>> = streams.close__InStream(s);
        if matches!(done, crate::unions::Union2::U1(_)) {
            let mut done_31 = match &done { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
            crate::std_core_console::println(console, &String::from("unexpected: the failure was not recorded"));
        } else if matches!(done, crate::unions::Union2::U2(_)) {
            let mut done_32 = match done { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            crate::std_core_console::println(console, &format!("and again at close: {}", crate::kind_name__StreamError((&(crate::std_core_checked::detach::<crate::unions::Union2<crate::std_stream::InvalidUtf8, crate::std_stream::StreamFailed>>(done_32))).as_ref())));
        } else {
            panic!("salvo: unhandled union arm at main:196:13");
        };
    } else if matches!(split, crate::unions::Union2::U2(_)) {
        let mut split_33 = match split { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        crate::std_core_console::println(console, &format!("split open failed: {}", crate::kind_name__FsError((&(crate::std_core_checked::detach::<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>(split_33))).as_ref())));
    } else {
        panic!("salvo: unhandled union arm at main:187:5");
    };
    let mut held: crate::unions::Union2<crate::std_stream::InStream, crate::std_core_checked::Checked<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>> = fs.open_read(&crate::std_fs_path::path(&String::from("raw.bin")));
    if matches!(held, crate::unions::Union2::U1(_)) {
        let mut held_34 = match held { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
        let mut s: crate::std_stream::InStream = held_34;
        let mut buf: crate::std_core_bytes::Bytes = crate::std_core_bytes::mut_bytes(vec![]);
        let mut steps: i32 = 0i32;
        let mut moved: i32 = 0i32;
        let mut reading: bool = true;
        loop {
            if !(reading) {
                break;
            };
            crate::std_core_bytes::clear_platform(&mut buf);
            let mut got: crate::unions::Union2<i32, crate::std_core_checked::Checked<crate::unions::Union2<crate::std_stream::InvalidUtf8, crate::std_stream::StreamFailed>>> = streams.read_to__InStream_Bytes_Int(&s, &mut buf, 4i32);
            if matches!(got, crate::unions::Union2::U1(_)) {
                let mut got_35 = match &got { crate::unions::Union2::U1(__v) => *__v, _ => unreachable!() };
                let mut n: i32 = got_35;
                if (n == 0i32) {
                    reading = false;
                } else {
                    steps = i32::wrapping_add(steps, 1i32);
                    moved = i32::wrapping_add(moved, n);
                };
            } else if matches!(got, crate::unions::Union2::U2(_)) {
                let mut got_36 = match got { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
                crate::std_core_console::println(console, &format!("fill failed: {}", crate::kind_name__StreamError((&(crate::std_core_checked::detach::<crate::unions::Union2<crate::std_stream::InvalidUtf8, crate::std_stream::StreamFailed>>(got_36))).as_ref())));
                reading = false;
            } else {
                panic!("salvo: unhandled union arm at main:219:17");
            };
        }
        crate::std_core_console::println(console, &format!("filled {} bytes in {} reads, one buffer", moved, steps));
        let mut done: crate::unions::Union2<(), crate::std_core_checked::Checked<crate::unions::Union2<crate::std_stream::InvalidUtf8, crate::std_stream::StreamFailed>>> = streams.close__InStream(s);
        if matches!(done, crate::unions::Union2::U2(_)) {
            let mut done_37 = match done { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            crate::std_core_console::println(console, &format!("close failed: {}", crate::kind_name__StreamError((&(crate::std_core_checked::detach::<crate::unions::Union2<crate::std_stream::InvalidUtf8, crate::std_stream::StreamFailed>>(done_37))).as_ref())));
        };
    } else if matches!(held, crate::unions::Union2::U2(_)) {
        let mut held_38 = match held { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        crate::std_core_console::println(console, &format!("fill open failed: {}", crate::kind_name__FsError((&(crate::std_core_checked::detach::<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>(held_38))).as_ref())));
    } else {
        panic!("salvo: unhandled union arm at main:209:5");
    };
    let mut lined: crate::unions::Union2<crate::std_stream::InStream, crate::std_core_checked::Checked<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>> = fs.open_read(&crate::std_fs_path::path(&String::from("notes.txt")));
    if matches!(lined, crate::unions::Union2::U1(_)) {
        let mut lined_39 = match lined { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
        let mut s: crate::std_stream::InStream = lined_39;
        let mut line: String = crate::std_core_string::Mut_Str(vec![]);
        let mut longest: i32 = 0i32;
        let mut reading: bool = true;
        loop {
            if !(reading) {
                break;
            };
            crate::std_core_string::clear_platform(&mut line);
            if streams.read_line_to(&s, &mut line) {
                if (crate::std_core_string::size_platform(&line) > longest) {
                    longest = crate::std_core_string::size_platform(&line);
                };
            } else {
                reading = false;
            };
        }
        crate::std_core_console::println(console, &format!("longest line: {} characters", longest));
        let mut done: crate::unions::Union2<(), crate::std_core_checked::Checked<crate::unions::Union2<crate::std_stream::InvalidUtf8, crate::std_stream::StreamFailed>>> = streams.close__InStream(s);
        if matches!(done, crate::unions::Union2::U2(_)) {
            let mut done_40 = match done { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            crate::std_core_console::println(console, &format!("close failed: {}", crate::kind_name__StreamError((&(crate::std_core_checked::detach::<crate::unions::Union2<crate::std_stream::InvalidUtf8, crate::std_stream::StreamFailed>>(done_40))).as_ref())));
        };
    } else if matches!(lined, crate::unions::Union2::U2(_)) {
        let mut lined_41 = match lined { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        crate::std_core_console::println(console, &format!("lines open failed: {}", crate::kind_name__FsError((&(crate::std_core_checked::detach::<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>(lined_41))).as_ref())));
    } else {
        panic!("salvo: unhandled union arm at main:246:5");
    };
    let mut ch: crate::unions::Union2<crate::std_stream::Chunks, crate::std_core_checked::Checked<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>> = crate::std_fs::open_chunks(fs, streams, &crate::std_fs_path::path(&String::from("raw.bin")), 4i32);
    if matches!(ch, crate::unions::Union2::U1(_)) {
        let mut ch_42 = match ch { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
        let mut p: crate::std_stream::Chunks = ch_42;
        let mut seen: i32 = 0i32;
        loop {
            let mut __step_44: crate::unions::Union2<crate::std_core_bytes::Bytes, crate::std_core_iterator::Finished> = crate::std_stream::next__Chunks(streams, &mut p);
            if matches!(__step_44, crate::unions::Union2::U1(_)) {
                let mut __emitted_45 = match __step_44 { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
                let mut chunk: crate::std_core_bytes::Bytes = __emitted_45;
                seen = i32::wrapping_add(seen, crate::std_core_bytes::size_platform(&chunk));
            } else {
                break;
            };
        }
        crate::std_core_console::println(console, &format!("chunks saw {} bytes", seen));
        let mut done: crate::unions::Union2<(), crate::std_core_checked::Checked<crate::unions::Union2<crate::std_stream::InvalidUtf8, crate::std_stream::StreamFailed>>> = crate::std_stream::close__Chunks(streams, p);
        if matches!(done, crate::unions::Union2::U2(_)) {
            let mut done_46 = match done { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            crate::std_core_console::println(console, &format!("close failed: {}", crate::kind_name__StreamError((&(crate::std_core_checked::detach::<crate::unions::Union2<crate::std_stream::InvalidUtf8, crate::std_stream::StreamFailed>>(done_46))).as_ref())));
        };
    } else if matches!(ch, crate::unions::Union2::U2(_)) {
        let mut ch_47 = match ch { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        crate::std_core_console::println(console, &format!("chunks failed: {}", crate::kind_name__FsError((&(crate::std_core_checked::detach::<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>(ch_47))).as_ref())));
    } else {
        panic!("salvo: unhandled union arm at main:275:5");
    };
    let mut copied: crate::unions::Union2<i64, crate::std_core_checked::Checked<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>> = crate::std_fs::copy_file(fs, streams, &crate::std_fs_path::path(&String::from("notes.txt")), &crate::std_fs_path::path(&String::from("notes-copy.txt")));
    if matches!(copied, crate::unions::Union2::U1(_)) {
        let mut copied_48 = match &copied { crate::unions::Union2::U1(__v) => *__v, _ => unreachable!() };
        crate::std_core_console::println(console, &format!("copied {} bytes", copied_48));
    } else if matches!(copied, crate::unions::Union2::U2(_)) {
        let mut copied_49 = match copied { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        crate::std_core_console::println(console, &format!("copy failed: {}", crate::kind_name__FsError((&(crate::std_core_checked::detach::<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>(copied_49))).as_ref())));
    } else {
        panic!("salvo: unhandled union arm at main:295:5");
    };
    let mut whole: crate::unions::Union2<crate::std_core_bytes::Bytes, crate::std_core_checked::Checked<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>> = crate::std_fs::read_to_bytes(fs, streams, &crate::std_fs_path::path(&String::from("raw.bin")));
    if matches!(whole, crate::unions::Union2::U1(_)) {
        let mut whole_50 = match &whole { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
        crate::std_core_console::println(console, &format!("raw.bin is {} bytes: {}", crate::std_core_bytes::size_platform(whole_50), crate::std_core_bytes::to_hex_platform(whole_50)));
    } else if matches!(whole, crate::unions::Union2::U2(_)) {
        let mut whole_51 = match whole { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        crate::std_core_console::println(console, &format!("byte read failed: {}", crate::kind_name__FsError((&(crate::std_core_checked::detach::<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>(whole_51))).as_ref())));
    } else {
        panic!("salvo: unhandled union arm at main:300:5");
    };
    let mut failures: Vec<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>> = vec![];
    let mut missing: crate::unions::Union2<String, crate::std_core_checked::Checked<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>> = crate::std_fs::read_to_str(fs, streams, &crate::std_fs_path::path(&String::from("nope.txt")));
    if matches!(missing, crate::unions::Union2::U1(_)) {
        let mut missing_52 = match &missing { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
        crate::std_core_console::println(console, &format!("unexpected: {}", missing_52));
    } else if matches!(missing, crate::unions::Union2::U2(_)) {
        let mut missing_53 = match missing { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        crate::std_core_list::add_platform::<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>(&mut failures, crate::std_core_checked::detach::<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>(missing_53));
    } else {
        panic!("salvo: unhandled union arm at main:310:5");
    };
    let mut not_a_dir: crate::unions::Union2<Vec<String>, crate::std_core_checked::Checked<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>> = fs.list_dir(&crate::std_fs_path::path(&String::from("notes.txt")));
    if matches!(not_a_dir, crate::unions::Union2::U1(_)) {
        let mut not_a_dir_54 = match &not_a_dir { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
        crate::std_core_console::println(console, &format!("unexpected: {}", crate::std_core_list::to_str(not_a_dir_54, &mut |__a0| format!("{}", __a0))));
    } else if matches!(not_a_dir, crate::unions::Union2::U2(_)) {
        let mut not_a_dir_55 = match not_a_dir { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        crate::std_core_list::add_platform::<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>(&mut failures, crate::std_core_checked::detach::<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>(not_a_dir_55));
    } else {
        panic!("salvo: unhandled union arm at main:315:5");
    };
    crate::std_core_console::println(console, &format!("failures: {}", crate::std_core_list::size_platform::<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>(&failures)));
    for mut kind in failures.iter() {
        crate::std_core_console::println(console, &format!("  {}", crate::kind_name__FsError((&((kind).clone())).as_ref())));
    }
    for mut name in vec![String::from("notes.txt"), String::from("notes-copy.txt"), String::from("raw.bin")].into_iter() {
        let mut gone: crate::unions::Union2<(), crate::std_core_checked::Checked<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>> = fs.delete(&crate::std_fs_path::path(&name));
        if matches!(gone, crate::unions::Union2::U2(_)) {
            let mut gone_56 = match gone { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            crate::std_core_console::println(console, &format!("delete failed: {}", crate::kind_name__FsError((&(crate::std_core_checked::detach::<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>(gone_56))).as_ref())));
        };
    }
    crate::std_core_console::println(console, &String::from("cleaned up"));
}

pub fn sandbox_edges(fs: &crate::std_fs::Fs, console: &crate::std_core_console::Console, streams: &crate::std_stream::Streams) {
    let mut inside: crate::unions::Union2<i64, crate::std_core_checked::Checked<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>> = crate::std_fs::write_str(fs, streams, &crate::std_fs_path::path(&String::from("sub/../probe.txt")), &String::from("inside\n"));
    if matches!(inside, crate::unions::Union2::U1(_)) {
        let mut inside_1 = match &inside { crate::unions::Union2::U1(__v) => *__v, _ => unreachable!() };
        crate::std_core_console::println(console, &format!("through `..`: wrote {} bytes", inside_1));
    } else if matches!(inside, crate::unions::Union2::U2(_)) {
        let mut inside_2 = match inside { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        crate::std_core_console::println(console, &format!("through `..`: {}", crate::kind_name__FsError((&(crate::std_core_checked::detach::<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>(inside_2))).as_ref())));
    } else {
        panic!("salvo: unhandled union arm at main:340:5");
    };
    let mut up: crate::unions::Union2<String, crate::std_core_checked::Checked<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>> = crate::std_fs::read_to_str(fs, streams, &crate::std_fs_path::path(&String::from("../secret.txt")));
    if matches!(up, crate::unions::Union2::U1(_)) {
        let mut up_3 = match &up { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
        crate::std_core_console::println(console, &String::from("unexpected: read outside the sandbox"));
    } else if matches!(up, crate::unions::Union2::U2(_)) {
        let mut up_4 = match up { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        crate::std_core_console::println(console, &format!("climbing out: {}", crate::kind_name__FsError((&(crate::std_core_checked::detach::<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>(up_4))).as_ref())));
    } else {
        panic!("salvo: unhandled union arm at main:345:5");
    };
    let mut absolute: crate::unions::Union2<String, crate::std_core_checked::Checked<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>> = crate::std_fs::read_to_str(fs, streams, &crate::std_fs_path::path(&String::from("/etc/hosts")));
    if matches!(absolute, crate::unions::Union2::U1(_)) {
        let mut absolute_5 = match &absolute { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
        crate::std_core_console::println(console, &String::from("unexpected: an absolute path resolved"));
    } else if matches!(absolute, crate::unions::Union2::U2(_)) {
        let mut absolute_6 = match absolute { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        crate::std_core_console::println(console, &format!("absolute path: {}", crate::kind_name__FsError((&(crate::std_core_checked::detach::<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>(absolute_6))).as_ref())));
    } else {
        panic!("salvo: unhandled union arm at main:350:5");
    };
    let mut probe: crate::std_fs_path::Path = crate::std_fs_path::path(&String::from("probe.txt"));
    crate::std_core_console::println(console, &format!("probe still there: {}", fs.exists(&probe)));
    let mut gone: crate::unions::Union2<(), crate::std_core_checked::Checked<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>> = fs.delete(&probe);
    if matches!(gone, crate::unions::Union2::U2(_)) {
        let mut gone_7 = match gone { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        crate::std_core_console::println(console, &format!("delete failed: {}", crate::kind_name__FsError((&(crate::std_core_checked::detach::<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>(gone_7))).as_ref())));
    };
}

pub fn main() {
    crate::scheduler::salvo_set_protocols(vec![("Faults".to_string(), crate::std_core_actor::__PROTO_Faults.to_string())]);
    let mut __use_1: crate::std_core_console::__Platform_StdConsole = crate::std_core_console::__Platform_StdConsole::new();
    let __handle_2 = crate::std_core_console::Console::shared(__use_1);
    let mut __use_3: crate::std_stream_host::HostRawStreams = crate::std_stream_host::HostRawStreams::new();
    let __handle_4 = crate::std_stream_host::RawStreams::shared(__use_3);
    let mut __use_5: crate::std_stream_host::DefaultStreams = crate::std_stream_host::DefaultStreams::new(__handle_4.clone());
    let __handle_6 = crate::std_stream::Streams::shared(__use_5);
    let mut __use_7: crate::std_fs_host::__Platform_HostRawFs = crate::std_fs_host::__Platform_HostRawFs::new();
    let __handle_8 = crate::std_fs_host::RawFs::locked(__use_7);
    let mut __use_9: crate::std_fs_host::DefaultFs = crate::std_fs_host::DefaultFs::new(__handle_8.clone(), __handle_6.clone());
    let __handle_10 = crate::std_fs::Fs::shared(__use_9);
    let mut root: crate::std_fs_path::Path = crate::std_fs_path::path(&String::from("tmp/files-example"));
    let mut made: crate::unions::Union2<(), crate::std_core_checked::Checked<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>> = __handle_10.create_dirs(&root);
    if matches!(made, crate::unions::Union2::U2(_)) {
        let mut made_11 = match made { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        crate::std_core_console::println(&__handle_2, &format!("cannot create the working directory: {}", crate::kind_name__FsError((&(crate::std_core_checked::detach::<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>(made_11))).as_ref())));
        return;
    };
    crate::std_core_console::println(&__handle_2, &String::from("-- the real filesystem, scoped to one directory --"));
    {
        let mut __use_12: crate::std_fs_restricted::RestrictedFs = crate::std_fs_restricted::RestrictedFs::new((root).clone(), __handle_10.clone(), __handle_6.clone());
        let __handle_13 = crate::std_fs::Fs::shared(__use_12);
        crate::workflow(&__handle_13, &__handle_2, &__handle_6);
        crate::sandbox_edges(&__handle_13, &__handle_2, &__handle_6);
    };
    let mut gone: crate::unions::Union2<(), crate::std_core_checked::Checked<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>> = __handle_10.delete(&root);
    if matches!(gone, crate::unions::Union2::U2(_)) {
        let mut gone_14 = match gone { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        crate::std_core_console::println(&__handle_2, &format!("cleanup failed: {}", crate::kind_name__FsError((&(crate::std_core_checked::detach::<crate::unions::Union7<crate::std_fs::NotFound, crate::std_fs::PermissionDenied, crate::std_fs::AlreadyExists, crate::std_fs::NotADirectory, crate::std_fs::PathEscapes, crate::std_fs::IoError, crate::std_fs::Streaming>>(gone_14))).as_ref())));
    };
    crate::std_core_console::println(&__handle_2, &String::from("-- the same code, with no disk at all --"));
    {
        let __use_15 = std::sync::Arc::new(std::sync::Mutex::new(crate::std_fs_mem::MemFs::new()));
        let __handle_16 = crate::std_fs::Fs::share_locked(__use_15.clone());
        let __handle_17 = crate::std_stream::Streams::share_locked(__use_15.clone());
        crate::workflow(&__handle_16, &__handle_2, &__handle_17);
    };
}
