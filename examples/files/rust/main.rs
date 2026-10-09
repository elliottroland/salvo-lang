#![allow(non_snake_case, non_camel_case_types, unused_mut, unused_parens, unused_imports, dead_code, unreachable_code, unused_variables, path_statements, unused_must_use, suspicious_double_ref_op, non_upper_case_globals, unused_braces)]
#[path = "unions/mod.rs"]
pub mod unions;
#[path = "seq.rs"]
pub mod seq;
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
#[path = "core/basic.rs"]
pub mod core_basic;
#[path = "core/bytes.rs"]
pub mod core_bytes;
#[path = "core/checked.rs"]
pub mod core_checked;
#[path = "core/compare.rs"]
pub mod core_compare;
#[path = "core/console.rs"]
pub mod core_console;
#[path = "core/deque.rs"]
pub mod core_deque;
#[path = "core/index.rs"]
pub mod core_index;
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
#[path = "fs/path.rs"]
pub mod fs_path;
#[path = "fs/restricted.rs"]
pub mod fs_restricted;
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
#[path = "platform/core/deque.rs"]
pub mod platform_core_deque;
#[path = "platform/core/list.rs"]
pub mod platform_core_list;
#[path = "platform/core/map.rs"]
pub mod platform_core_map;
#[path = "platform/core/set.rs"]
pub mod platform_core_set;
#[path = "platform/core/sorted.rs"]
pub mod platform_core_sorted;
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

use crate::fs::AlreadyExists;
use crate::core_bytes::Bytes;
use crate::core_checked::Checked;
use crate::stream::Chunks;
use crate::core_console::Console;
use crate::fs_host::DefaultFs;
use crate::stream_host::DefaultStreams;
use crate::core_iterator::Finished;
use crate::fs::Fs;
use crate::stream_host::HostRawStreams;
use crate::stream::InStream;
use crate::stream::InvalidUtf8;
use crate::fs::IoError;
use crate::stream::Lines;
use crate::fs::NotADirectory;
use crate::fs::NotFound;
use crate::stream::OutStream;
use crate::fs_path::Path;
use crate::fs::PathEscapes;
use crate::fs::PermissionDenied;
use crate::fs_restricted::RestrictedFs;
use crate::stream::StreamFailed;
use crate::fs::Streaming;
use crate::stream::Streams;
use crate::core_list::add_platform;
use crate::core_string::byte_size_platform;
use crate::core_bytes::bytes_of;
use crate::stream::close__Chunks;
use crate::stream::close__Lines;
use crate::fs::copy_file;
use crate::core_checked::detach;
use crate::stream::lines;
use crate::core_bytes::mut_bytes;
use crate::core_string::mut_str;
use crate::stream::next__Chunks;
use crate::stream::next__Lines;
use crate::fs::open_chunks;
use crate::fs_path::path;
use crate::core_console::println;
use crate::fs::read_to_bytes;
use crate::fs::read_to_str;
use crate::core_bytes::to_hex_platform;
use crate::core_list::to_str;
use crate::core_bytes::to_str_platform;
use crate::fs::write_str;


pub fn kind_name__FsError(kind: &crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>) -> String {
    if matches!(kind, crate::unions::Union7::U1(_)) {
        let mut kind_1 = match &kind { crate::unions::Union7::U1(__v) => __v, _ => unreachable!() };
        return String::from("not found");
    };
    if matches!(kind, crate::unions::Union7::U4(_)) {
        let mut kind_2 = match &kind { crate::unions::Union7::U4(__v) => __v, _ => unreachable!() };
        return String::from("not a directory");
    };
    if matches!(kind, crate::unions::Union7::U5(_)) {
        let mut kind_3 = match &kind { crate::unions::Union7::U5(__v) => __v, _ => unreachable!() };
        return String::from("escapes the sandbox");
    };
    if matches!(kind, crate::unions::Union7::U7(_)) {
        let mut kind_4 = match &kind { crate::unions::Union7::U7(__v) => __v, _ => unreachable!() };
        return crate::kind_name__StreamError(&kind_4.error);
    };
    return String::from("other");
}

pub fn kind_name__StreamError(kind: &crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>) -> String {
    if matches!(kind, crate::unions::Union2::U1(_)) {
        let mut kind_1 = match &kind { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
        return String::from("not valid UTF-8");
    };
    return String::from("other");
}

pub fn workflow(fs: &crate::fs::Fs, console: &crate::core_console::Console, streams: &crate::stream::Streams) {
    let mut wrote: crate::unions::Union2<i64, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> = crate::fs::write_str(fs, streams, &crate::fs_path::path(&String::from("notes.txt")), &String::from("alpha\nbeta\ngamma\n"));
    if matches!(wrote, crate::unions::Union2::U1(_)) {
        let mut wrote_1 = match &wrote { crate::unions::Union2::U1(__v) => *__v, _ => unreachable!() };
        crate::core_console::println(console, &format!("wrote {} bytes", wrote_1));
    } else {
        let mut wrote_2 = match wrote { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        crate::core_console::println(console, &format!("write failed: {}", crate::kind_name__FsError(&crate::core_checked::detach::<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>(wrote_2))));
    };
    let mut text: crate::unions::Union2<String, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> = crate::fs::read_to_str(fs, streams, &crate::fs_path::path(&String::from("notes.txt")));
    if matches!(text, crate::unions::Union2::U1(_)) {
        let mut text_3 = match &text { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
        crate::core_console::println(console, &format!("read back {} bytes", crate::core_string::byte_size_platform(text_3)));
    } else {
        let mut text_4 = match text { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        crate::core_console::println(console, &format!("read failed: {}", crate::kind_name__FsError(&crate::core_checked::detach::<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>(text_4))));
    };
    let mut opened: crate::unions::Union2<crate::stream::InStream, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> = fs.open_read(&crate::fs_path::path(&String::from("notes.txt")));
    if matches!(opened, crate::unions::Union2::U1(_)) {
        let mut opened_5 = match opened { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
        let mut p: crate::stream::Lines = crate::stream::lines(opened_5);
        loop {
            let mut __step_7: crate::unions::Union2<String, crate::core_iterator::Finished> = crate::stream::next__Lines(streams, &mut p);
            if matches!(__step_7, crate::unions::Union2::U1(_)) {
                let mut __emitted_8 = match __step_7 { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
                let mut line: String = __emitted_8;
                crate::core_console::println(console, &format!("line: {}", line));
            } else {
                break;
            };
        }
        let mut closed: crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> = crate::stream::close__Lines(streams, p);
        if matches!(closed, crate::unions::Union2::U2(_)) {
            let mut closed_9 = match closed { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            crate::core_console::println(console, &format!("close failed: {}", crate::kind_name__StreamError(&crate::core_checked::detach::<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>(closed_9))));
        };
    } else {
        let mut opened_10 = match opened { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        crate::core_console::println(console, &format!("open failed: {}", crate::kind_name__FsError(&crate::core_checked::detach::<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>(opened_10))));
    };
    let mut out: crate::unions::Union2<crate::stream::OutStream, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> = fs.open_append(&crate::fs_path::path(&String::from("notes.txt")));
    if matches!(out, crate::unions::Union2::U1(_)) {
        let mut out_11 = match out { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
        let mut w: crate::stream::OutStream = out_11;
        let mut at: i64 = streams.position__OutStream(&w);
        let mut n: i64 = streams.write_line(&w, &String::from("delta"));
        crate::core_console::println(console, &format!("appended {} bytes at offset {}", n, at));
        let mut shut: crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> = streams.close__OutStream(w);
        if matches!(shut, crate::unions::Union2::U2(_)) {
            let mut shut_12 = match shut { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            crate::core_console::println(console, &format!("close failed: {}", crate::kind_name__StreamError(&crate::core_checked::detach::<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>(shut_12))));
        };
        let mut resumed: crate::unions::Union2<crate::stream::InStream, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> = fs.open_read_at(&crate::fs_path::path(&String::from("notes.txt")), at);
        if matches!(resumed, crate::unions::Union2::U1(_)) {
            let mut resumed_13 = match resumed { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
            let mut s: crate::stream::InStream = resumed_13;
            let mut line: Option<String> = streams.read_line(&s);
            if line.is_some() {
                let mut line_14 = line.as_ref().unwrap();
                crate::core_console::println(console, &format!("at {}: {}", at, line_14));
            } else {
                crate::core_console::println(console, &format!("at {}: end of file", at));
            };
            let mut done: crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> = streams.close__InStream(s);
            if matches!(done, crate::unions::Union2::U2(_)) {
                let mut done_15 = match done { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
                crate::core_console::println(console, &format!("close failed: {}", crate::kind_name__StreamError(&crate::core_checked::detach::<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>(done_15))));
            };
        } else {
            let mut resumed_16 = match resumed { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            crate::core_console::println(console, &format!("reopen failed: {}", crate::kind_name__FsError(&crate::core_checked::detach::<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>(resumed_16))));
        };
    } else {
        let mut out_17 = match out { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        crate::core_console::println(console, &format!("append failed: {}", crate::kind_name__FsError(&crate::core_checked::detach::<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>(out_17))));
    };
    let mut bin: crate::unions::Union2<crate::stream::OutStream, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> = fs.open_write(&crate::fs_path::path(&String::from("raw.bin")));
    if matches!(bin, crate::unions::Union2::U1(_)) {
        let mut bin_18 = match bin { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
        let mut w: crate::stream::OutStream = bin_18;
        let mut data: crate::core_bytes::Bytes = crate::core_bytes::bytes_of(vec![(((0i32) as i32) as u8), (((255i32) as i32) as u8), (((200i32) as i32) as u8)]);
        let mut n: i64 = streams.write_bytes(&w, &data);
        let mut m: i64 = streams.write(&w, &String::from("hé"));
        crate::core_console::println(console, &format!("wrote {} raw bytes and {} encoded", n, m));
        let mut shut: crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> = streams.close__OutStream(w);
        if matches!(shut, crate::unions::Union2::U2(_)) {
            let mut shut_19 = match shut { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            crate::core_console::println(console, &format!("close failed: {}", crate::kind_name__StreamError(&crate::core_checked::detach::<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>(shut_19))));
        };
    } else {
        let mut bin_20 = match bin { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        crate::core_console::println(console, &format!("raw open failed: {}", crate::kind_name__FsError(&crate::core_checked::detach::<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>(bin_20))));
    };
    let mut raw: crate::unions::Union2<crate::stream::InStream, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> = fs.open_read(&crate::fs_path::path(&String::from("raw.bin")));
    if matches!(raw, crate::unions::Union2::U1(_)) {
        let mut raw_21 = match raw { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
        let mut s: crate::stream::InStream = raw_21;
        let mut head: crate::unions::Union2<crate::core_bytes::Bytes, crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> = streams.read_bytes(&s, 3i32);
        if matches!(head, crate::unions::Union2::U1(_)) {
            let mut head_22 = match &head { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
            crate::core_console::println(console, &format!("first three: {} = {}", crate::core_bytes::to_str_platform(head_22), crate::core_bytes::to_hex_platform(head_22)));
        } else {
            let mut head_23 = match head { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            crate::core_console::println(console, &format!("byte read failed: {}", crate::kind_name__StreamError(&crate::core_checked::detach::<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>(head_23))));
        };
        let mut tail: crate::unions::Union2<String, crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> = streams.read_all(&s);
        if matches!(tail, crate::unions::Union2::U1(_)) {
            let mut tail_24 = match &tail { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
            crate::core_console::println(console, &format!("the rest, as text: {}", tail_24));
        } else {
            let mut tail_25 = match tail { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            crate::core_console::println(console, &format!("decode failed: {}", crate::kind_name__StreamError(&crate::core_checked::detach::<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>(tail_25))));
        };
        let mut done: crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> = streams.close__InStream(s);
        if matches!(done, crate::unions::Union2::U2(_)) {
            let mut done_26 = match done { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            crate::core_console::println(console, &format!("close failed: {}", crate::kind_name__StreamError(&crate::core_checked::detach::<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>(done_26))));
        };
    } else {
        let mut raw_27 = match raw { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        crate::core_console::println(console, &format!("raw read failed: {}", crate::kind_name__FsError(&crate::core_checked::detach::<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>(raw_27))));
    };
    let mut split: crate::unions::Union2<crate::stream::InStream, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> = fs.open_read_at(&crate::fs_path::path(&String::from("raw.bin")), 5i64);
    if matches!(split, crate::unions::Union2::U1(_)) {
        let mut split_28 = match split { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
        let mut s: crate::stream::InStream = split_28;
        let mut broken: crate::unions::Union2<String, crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> = streams.read_all(&s);
        if matches!(broken, crate::unions::Union2::U1(_)) {
            let mut broken_29 = match &broken { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
            crate::core_console::println(console, &format!("unexpected: {} decoded", broken_29));
        } else {
            let mut broken_30 = match broken { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            crate::core_console::println(console, &format!("mid-character: {}", crate::kind_name__StreamError(&crate::core_checked::detach::<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>(broken_30))));
        };
        let mut done: crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> = streams.close__InStream(s);
        if matches!(done, crate::unions::Union2::U1(_)) {
            let mut done_31 = match &done { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
            crate::core_console::println(console, &String::from("unexpected: the failure was not recorded"));
        } else {
            let mut done_32 = match done { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            crate::core_console::println(console, &format!("and again at close: {}", crate::kind_name__StreamError(&crate::core_checked::detach::<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>(done_32))));
        };
    } else {
        let mut split_33 = match split { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        crate::core_console::println(console, &format!("split open failed: {}", crate::kind_name__FsError(&crate::core_checked::detach::<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>(split_33))));
    };
    let mut held: crate::unions::Union2<crate::stream::InStream, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> = fs.open_read(&crate::fs_path::path(&String::from("raw.bin")));
    if matches!(held, crate::unions::Union2::U1(_)) {
        let mut held_34 = match held { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
        let mut s: crate::stream::InStream = held_34;
        let mut buf: crate::core_bytes::Bytes = crate::core_bytes::mut_bytes(vec![]);
        let mut steps: i32 = 0i32;
        let mut moved: i32 = 0i32;
        let mut reading: bool = true;
        loop {
            if !(reading) {
                break;
            };
            crate::core_bytes::clear_platform(&mut buf);
            let mut got: crate::unions::Union2<i32, crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> = streams.read_to__InStream_Bytes_Int(&s, &mut buf, 4i32);
            if matches!(got, crate::unions::Union2::U1(_)) {
                let mut got_35 = match &got { crate::unions::Union2::U1(__v) => *__v, _ => unreachable!() };
                let mut n: i32 = got_35;
                if (n == 0i32) {
                    reading = false;
                } else {
                    steps = i32::wrapping_add(steps, 1i32);
                    moved = i32::wrapping_add(moved, n);
                };
            } else {
                let mut got_36 = match got { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
                crate::core_console::println(console, &format!("fill failed: {}", crate::kind_name__StreamError(&crate::core_checked::detach::<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>(got_36))));
                reading = false;
            };
        }
        crate::core_console::println(console, &format!("filled {} bytes in {} reads, one buffer", moved, steps));
        let mut done: crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> = streams.close__InStream(s);
        if matches!(done, crate::unions::Union2::U2(_)) {
            let mut done_37 = match done { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            crate::core_console::println(console, &format!("close failed: {}", crate::kind_name__StreamError(&crate::core_checked::detach::<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>(done_37))));
        };
    } else {
        let mut held_38 = match held { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        crate::core_console::println(console, &format!("fill open failed: {}", crate::kind_name__FsError(&crate::core_checked::detach::<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>(held_38))));
    };
    let mut lined: crate::unions::Union2<crate::stream::InStream, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> = fs.open_read(&crate::fs_path::path(&String::from("notes.txt")));
    if matches!(lined, crate::unions::Union2::U1(_)) {
        let mut lined_39 = match lined { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
        let mut s: crate::stream::InStream = lined_39;
        let mut line: String = crate::core_string::mut_str(vec![]);
        let mut longest: i32 = 0i32;
        let mut reading: bool = true;
        loop {
            if !(reading) {
                break;
            };
            crate::core_string::clear_platform(&mut line);
            if streams.read_line_to(&s, &mut line) {
                if (crate::core_string::size_platform(&line) > longest) {
                    longest = crate::core_string::size_platform(&line);
                };
            } else {
                reading = false;
            };
        }
        crate::core_console::println(console, &format!("longest line: {} characters", longest));
        let mut done: crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> = streams.close__InStream(s);
        if matches!(done, crate::unions::Union2::U2(_)) {
            let mut done_40 = match done { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            crate::core_console::println(console, &format!("close failed: {}", crate::kind_name__StreamError(&crate::core_checked::detach::<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>(done_40))));
        };
    } else {
        let mut lined_41 = match lined { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        crate::core_console::println(console, &format!("lines open failed: {}", crate::kind_name__FsError(&crate::core_checked::detach::<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>(lined_41))));
    };
    let mut ch: crate::unions::Union2<crate::stream::Chunks, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> = crate::fs::open_chunks(fs, streams, &crate::fs_path::path(&String::from("raw.bin")), 4i32);
    if matches!(ch, crate::unions::Union2::U1(_)) {
        let mut ch_42 = match ch { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
        let mut p: crate::stream::Chunks = ch_42;
        let mut seen: i32 = 0i32;
        loop {
            let mut __step_44: crate::unions::Union2<crate::core_bytes::Bytes, crate::core_iterator::Finished> = crate::stream::next__Chunks(streams, &mut p);
            if matches!(__step_44, crate::unions::Union2::U1(_)) {
                let mut __emitted_45 = match __step_44 { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
                let mut chunk: crate::core_bytes::Bytes = __emitted_45;
                seen = i32::wrapping_add(seen, crate::core_bytes::size_platform(&chunk));
            } else {
                break;
            };
        }
        crate::core_console::println(console, &format!("chunks saw {} bytes", seen));
        let mut done: crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> = crate::stream::close__Chunks(streams, p);
        if matches!(done, crate::unions::Union2::U2(_)) {
            let mut done_46 = match done { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            crate::core_console::println(console, &format!("close failed: {}", crate::kind_name__StreamError(&crate::core_checked::detach::<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>(done_46))));
        };
    } else {
        let mut ch_47 = match ch { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        crate::core_console::println(console, &format!("chunks failed: {}", crate::kind_name__FsError(&crate::core_checked::detach::<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>(ch_47))));
    };
    let mut copied: crate::unions::Union2<i64, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> = crate::fs::copy_file(fs, streams, &crate::fs_path::path(&String::from("notes.txt")), &crate::fs_path::path(&String::from("notes-copy.txt")));
    if matches!(copied, crate::unions::Union2::U1(_)) {
        let mut copied_48 = match &copied { crate::unions::Union2::U1(__v) => *__v, _ => unreachable!() };
        crate::core_console::println(console, &format!("copied {} bytes", copied_48));
    } else {
        let mut copied_49 = match copied { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        crate::core_console::println(console, &format!("copy failed: {}", crate::kind_name__FsError(&crate::core_checked::detach::<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>(copied_49))));
    };
    let mut whole: crate::unions::Union2<crate::core_bytes::Bytes, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> = crate::fs::read_to_bytes(fs, streams, &crate::fs_path::path(&String::from("raw.bin")));
    if matches!(whole, crate::unions::Union2::U1(_)) {
        let mut whole_50 = match &whole { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
        crate::core_console::println(console, &format!("raw.bin is {} bytes: {}", crate::core_bytes::size_platform(whole_50), crate::core_bytes::to_hex_platform(whole_50)));
    } else {
        let mut whole_51 = match whole { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        crate::core_console::println(console, &format!("byte read failed: {}", crate::kind_name__FsError(&crate::core_checked::detach::<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>(whole_51))));
    };
    let mut failures: Vec<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>> = vec![];
    let mut missing: crate::unions::Union2<String, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> = crate::fs::read_to_str(fs, streams, &crate::fs_path::path(&String::from("nope.txt")));
    if matches!(missing, crate::unions::Union2::U1(_)) {
        let mut missing_52 = match &missing { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
        crate::core_console::println(console, &format!("unexpected: {}", missing_52));
    } else {
        let mut missing_53 = match missing { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        crate::core_list::add_platform::<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>(&mut failures, crate::core_checked::detach::<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>(missing_53));
    };
    let mut not_a_dir: crate::unions::Union2<Vec<String>, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> = fs.list_dir(&crate::fs_path::path(&String::from("notes.txt")));
    if matches!(not_a_dir, crate::unions::Union2::U1(_)) {
        let mut not_a_dir_54 = match &not_a_dir { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
        crate::core_console::println(console, &format!("unexpected: {}", crate::core_list::to_str(not_a_dir_54, &mut |__a0| format!("{}", __a0))));
    } else {
        let mut not_a_dir_55 = match not_a_dir { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        crate::core_list::add_platform::<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>(&mut failures, crate::core_checked::detach::<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>(not_a_dir_55));
    };
    crate::core_console::println(console, &format!("failures: {}", crate::core_list::size_platform::<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>(&failures)));
    for mut kind in failures.iter() {
        crate::core_console::println(console, &format!("  {}", crate::kind_name__FsError(&(kind).clone())));
    }
    for mut name in vec![String::from("notes.txt"), String::from("notes-copy.txt"), String::from("raw.bin")].into_iter() {
        let mut gone: crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> = fs.delete(&crate::fs_path::path(&name));
        if matches!(gone, crate::unions::Union2::U2(_)) {
            let mut gone_56 = match gone { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            crate::core_console::println(console, &format!("delete failed: {}", crate::kind_name__FsError(&crate::core_checked::detach::<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>(gone_56))));
        };
    }
    crate::core_console::println(console, &String::from("cleaned up"));
}

pub fn sandbox_edges(fs: &crate::fs::Fs, console: &crate::core_console::Console, streams: &crate::stream::Streams) {
    let mut inside: crate::unions::Union2<i64, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> = crate::fs::write_str(fs, streams, &crate::fs_path::path(&String::from("sub/../probe.txt")), &String::from("inside\n"));
    if matches!(inside, crate::unions::Union2::U1(_)) {
        let mut inside_1 = match &inside { crate::unions::Union2::U1(__v) => *__v, _ => unreachable!() };
        crate::core_console::println(console, &format!("through `..`: wrote {} bytes", inside_1));
    } else {
        let mut inside_2 = match inside { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        crate::core_console::println(console, &format!("through `..`: {}", crate::kind_name__FsError(&crate::core_checked::detach::<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>(inside_2))));
    };
    let mut up: crate::unions::Union2<String, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> = crate::fs::read_to_str(fs, streams, &crate::fs_path::path(&String::from("../secret.txt")));
    if matches!(up, crate::unions::Union2::U1(_)) {
        let mut up_3 = match &up { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
        crate::core_console::println(console, &String::from("unexpected: read outside the sandbox"));
    } else {
        let mut up_4 = match up { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        crate::core_console::println(console, &format!("climbing out: {}", crate::kind_name__FsError(&crate::core_checked::detach::<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>(up_4))));
    };
    let mut absolute: crate::unions::Union2<String, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> = crate::fs::read_to_str(fs, streams, &crate::fs_path::path(&String::from("/etc/hosts")));
    if matches!(absolute, crate::unions::Union2::U1(_)) {
        let mut absolute_5 = match &absolute { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
        crate::core_console::println(console, &String::from("unexpected: an absolute path resolved"));
    } else {
        let mut absolute_6 = match absolute { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        crate::core_console::println(console, &format!("absolute path: {}", crate::kind_name__FsError(&crate::core_checked::detach::<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>(absolute_6))));
    };
    let mut probe: crate::fs_path::Path = crate::fs_path::path(&String::from("probe.txt"));
    crate::core_console::println(console, &format!("probe still there: {}", fs.exists(&probe)));
    let mut gone: crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> = fs.delete(&probe);
    if matches!(gone, crate::unions::Union2::U2(_)) {
        let mut gone_7 = match gone { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        crate::core_console::println(console, &format!("delete failed: {}", crate::kind_name__FsError(&crate::core_checked::detach::<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>(gone_7))));
    };
}

pub fn main() {
    crate::scheduler::salvo_set_protocols(vec![("Faults".to_string(), crate::core_actor::__PROTO_Faults.to_string())]);
    let mut __use_1: crate::core_console::__Platform_StdOutConsole = crate::core_console::__Platform_StdOutConsole::new();
    let __handle_2 = crate::core_console::Console::shared(__use_1);
    let mut __use_3: crate::stream_host::HostRawStreams = crate::stream_host::HostRawStreams::new();
    let __handle_4 = crate::stream_host::RawStreams::shared(__use_3);
    let mut __use_5: crate::stream_host::DefaultStreams = crate::stream_host::DefaultStreams::new(__handle_4.clone());
    let __handle_6 = crate::stream::Streams::shared(__use_5);
    let mut __use_7: crate::fs_host::__Platform_HostRawFs = crate::fs_host::__Platform_HostRawFs::new();
    let __handle_8 = crate::fs_host::RawFs::locked(__use_7);
    let mut __use_9: crate::fs_host::DefaultFs = crate::fs_host::DefaultFs::new(__handle_8.clone(), __handle_6.clone());
    let __handle_10 = crate::fs::Fs::shared(__use_9);
    let mut root: crate::fs_path::Path = crate::fs_path::path(&String::from("tmp/files-example"));
    let mut made: crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> = __handle_10.create_dirs(&root);
    if matches!(made, crate::unions::Union2::U2(_)) {
        let mut made_11 = match made { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        crate::core_console::println(&__handle_2, &format!("cannot create the working directory: {}", crate::kind_name__FsError(&crate::core_checked::detach::<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>(made_11))));
        return;
    };
    crate::core_console::println(&__handle_2, &String::from("-- the real filesystem, scoped to one directory --"));
    {
        let mut __use_12: crate::fs_restricted::RestrictedFs = crate::fs_restricted::RestrictedFs::new((root).clone(), __handle_10.clone(), __handle_6.clone());
        let __handle_13 = crate::fs::Fs::shared(__use_12);
        crate::workflow(&__handle_13, &__handle_2, &__handle_6);
        crate::sandbox_edges(&__handle_13, &__handle_2, &__handle_6);
    };
    let mut gone: crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> = __handle_10.delete(&root);
    if matches!(gone, crate::unions::Union2::U2(_)) {
        let mut gone_14 = match gone { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        crate::core_console::println(&__handle_2, &format!("cleanup failed: {}", crate::kind_name__FsError(&crate::core_checked::detach::<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>(gone_14))));
    };
    crate::core_console::println(&__handle_2, &String::from("-- the same code, with no disk at all --"));
    {
        let __use_15 = std::sync::Arc::new(std::sync::Mutex::new(crate::fs_mem::MemFs::new()));
        let __handle_16 = crate::fs::Fs::share_locked(__use_15.clone());
        let __handle_17 = crate::stream::Streams::share_locked(__use_15.clone());
        crate::workflow(&__handle_16, &__handle_2, &__handle_17);
    };
}
