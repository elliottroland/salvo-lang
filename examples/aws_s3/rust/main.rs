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
#[path = "aws.rs"]
pub mod aws;
#[path = "aws/s3.rs"]
pub mod aws_s3;
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
#[path = "core/other.rs"]
pub mod core_other;
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
#[path = "fs/mem.rs"]
pub mod fs_mem;
#[path = "runtime.rs"]
pub mod runtime;
#[path = "runtime/routing.rs"]
pub mod runtime_routing;
#[path = "runtime/timers.rs"]
pub mod runtime_timers;
#[path = "stream.rs"]
pub mod stream;
#[path = "time.rs"]
pub mod time;
#[path = "platform/runtime/routing.rs"]
pub mod platform_runtime_routing;
#[path = "platform/runtime.rs"]
pub mod platform_runtime;

use crate::aws::*;
use crate::aws_s3::*;
use crate::collections::*;
use crate::core_actor::*;
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
use crate::fs_mem::*;
use crate::stream::*;
use crate::unions::*;

pub fn describe(e: &Union2<S3Error, AwsError>) -> String {
    if matches!(e, Union2::U2(_)) {
        return format!("{}: {}", e.u2().clone().code.clone(), e.u2().clone().message.clone());
    }
    if matches!(e.u1().clone().code, ref __v if (*__v == "NoSuchKey")) {
        return "no such key".to_string();
    }
    return format!("the service refused: {}", format!("{}", e.u1().clone().code.clone()));
}

pub fn size_of(fs: &crate::fs::Fs, streams: &crate::stream::Streams, path: &String) -> Option<i64> {
    let mut info = fs.metadata(path);
    match info {
        Union2::U1(_) => {
            return Some(info.u1().clone().size);
        }
        Union2::U2(_) => {
            ignore((match info { Union2::U2(__v) => __v, _ => unreachable!() }));
            return None;
        }
    }
}

pub fn upload(s3: &crate::aws_s3::S3, fs: &crate::fs::Fs, console: &crate::core_console::Console, streams: &crate::stream::Streams, bucket: &String, key: &String, path: &String, length: Option<i64>) {
    let mut opened = fs.open_read(path);
    if matches!(opened, Union2::U2(_)) {
        println(console, &(format!("open {}: {}", path.clone(), to_str(&detach((match opened { Union2::U2(__v) => __v, _ => unreachable!() }))))));
        return;
    }
    let mut put = {
        let (mut r, __wid) = crate::scheduler::salvo_waiter();
        crate::scheduler::salvo_waiter_decoder(__wid, (|__b: &[u8]| crate::wire::salvo_decode::<Union2<PutObjectOutput, Checked<Union2<S3Error, AwsError>>>>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg)));
        s3.put_object(PutObjectInput { bucket: bucket.clone(), key: key.clone(), body: (match opened { Union2::U1(__v) => __v, _ => unreachable!() }), content_length: length, acl: None, cache_control: None, content_disposition: None, content_encoding: None, content_language: None, content_md5: None, content_type: None, checksum_algorithm: None, checksum_crc32: None, checksum_crc32_c: None, checksum_crc64_nvme: None, checksum_sha1: None, checksum_sha256: None, checksum_sha512: None, checksum_md5: None, checksum_xxhash64: None, checksum_xxhash3: None, checksum_xxhash128: None, if_match: None, if_none_match: None, grant_full_control: None, grant_read: None, grant_read_acp: None, grant_write_acp: None, write_offset_bytes: None, metadata: None, server_side_encryption: None, storage_class: None, website_redirect_location: None, sse_customer_algorithm: None, sse_customer_key: None, sse_customer_key_md5: None, ssekms_key_id: None, ssekms_encryption_context: None, bucket_key_enabled: None, request_payer: None, tagging: None, object_lock_mode: None, object_lock_retain_until_date: None, object_lock_legal_hold_status: None, object_lock_event_hold: None, object_lock_event_hold_duration_days: None, object_lock_event_hold_duration_years: None, expected_bucket_owner: None }, r);
        *crate::scheduler::salvo_wait(__wid).downcast::<Union2<PutObjectOutput, Checked<Union2<S3Error, AwsError>>>>().expect("the awaited answer")
    };
    match put {
        Union2::U1(_) => {
            println(console, &(format!("put {}: etag {}", key.clone(), if put.u1().clone().e_tag.is_some() { put.u1().clone().e_tag.as_ref().unwrap().clone() } else { "?".to_string() })));
        }
        Union2::U2(_) => {
            println(console, &(format!("put {}: {}", key.clone(), describe(&(detach((match put { Union2::U2(__v) => __v, _ => unreachable!() })))))));
        }
    }
}

pub fn download(s3: &crate::aws_s3::S3, fs: &crate::fs::Fs, console: &crate::core_console::Console, streams: &crate::stream::Streams, bucket: &String, key: &String, path: &String) {
    let mut got = {
        let (mut r, __wid) = crate::scheduler::salvo_waiter();
        crate::scheduler::salvo_waiter_decoder(__wid, (|_| None));
        s3.get_object(GetObjectInput { bucket: bucket.clone(), key: key.clone(), if_match: None, if_modified_since: None, if_none_match: None, if_unmodified_since: None, range: None, response_cache_control: None, response_content_disposition: None, response_content_encoding: None, response_content_language: None, response_content_type: None, response_expires: None, version_id: None, sse_customer_algorithm: None, sse_customer_key: None, sse_customer_key_md5: None, request_payer: None, part_number: None, expected_bucket_owner: None, checksum_mode: None }, r);
        *crate::scheduler::salvo_wait(__wid).downcast::<Union2<GetObjectOutput, Checked<Union2<S3Error, AwsError>>>>().expect("the awaited answer")
    };
    if matches!(got, Union2::U2(_)) {
        println(console, &(format!("get {}: {}", key.clone(), describe(&(detach((match got { Union2::U2(__v) => __v, _ => unreachable!() })))))));
        return;
    }
    let __destructured1 = (match got { Union2::U1(__v) => __v, _ => unreachable!() });
    let mut body = __destructured1.body;
    let mut content_length = __destructured1.content_length;
    println(console, &(format!("get {}: {} bytes", key.clone(), if content_length.is_some() { content_length.unwrap() } else { -1i64 })));
    let mut target = fs.open_write(path);
    if matches!(target, Union2::U2(_)) {
        println(console, &(format!("open {}: {}", path.clone(), to_str(&detach((match target { Union2::U2(__v) => __v, _ => unreachable!() }))))));
        let mut closed = streams.close(body);
        if matches!(closed, Union2::U2(_)) {
            ignore((match closed { Union2::U2(__v) => __v, _ => unreachable!() }));
        }
        return;
    }
    let mut copied = {
        let (mut r, __wid) = crate::scheduler::salvo_waiter();
        crate::scheduler::salvo_waiter_decoder(__wid, (|__b: &[u8]| crate::wire::salvo_decode::<Union2<i64, Checked<Union2<InvalidUtf8, StreamFailed>>>>(__b).map(|__v| Box::new(__v) as crate::scheduler::SalvoMsg)));
        pipe(streams, body, (match target { Union2::U1(__v) => __v, _ => unreachable!() }), r);
        *crate::scheduler::salvo_wait(__wid).downcast::<Union2<i64, Checked<Union2<InvalidUtf8, StreamFailed>>>>().expect("the awaited answer")
    };
    match copied {
        Union2::U1(_) => {
            println(console, &(format!("piped {} bytes into {}", *copied.u1(), path.clone())));
        }
        Union2::U2(_) => {
            println(console, &(format!("pipe: {}", to_str__4(&detach((match copied { Union2::U2(__v) => __v, _ => unreachable!() }))))));
        }
    }
}

pub fn round_trip(s3: &crate::aws_s3::S3, fs: &crate::fs::Fs, console: &crate::core_console::Console, streams: &crate::stream::Streams, key: &String) {
    { let __a1 = size_of(fs, streams, &("notes.txt".to_string())); upload(s3, fs, console, streams, &("notes".to_string()), &(key.clone()), &("notes.txt".to_string()), __a1) };
    download(s3, fs, console, streams, &("notes".to_string()), &(key.clone()), &("back.txt".to_string()));
    let mut back = read_to_str(fs, streams, &("back.txt".to_string()));
    match back {
        Union2::U1(_) => {
            println(console, &(format!("back.txt: {} bytes", (back.u1().chars().count() as i32))));
            console.print(back.u1());
        }
        Union2::U2(_) => {
            println(console, &(format!("back.txt: {}", to_str(&detach((match back { Union2::U2(__v) => __v, _ => unreachable!() }))))));
        }
    }
}

pub struct MemS3 {
    objects: SalvoMap<String, Vec<u8>>,
    __dep_Streams: crate::stream::Streams,
}

impl MemS3 {
    pub fn new(__dep_Streams: crate::stream::Streams) -> Self {
        Self {
            objects: SalvoMap::from_entries::<HostHash, HostEq, _>(vec![]),
            __dep_Streams,
        }
    }
}

impl crate::aws_s3::__Stateful_S3 for MemS3 {

    fn put_object(&mut self, input: PutObjectInput, reply: crate::scheduler::SalvoReply) {
        let __destructured2 = input;
        let mut bucket = __destructured2.bucket;
        let mut key = __destructured2.key;
        let mut body = __destructured2.body;
        let mut buf = Vec::<u8>::new();
        let mut filled = fill_from(&self.__dep_Streams, &body, &mut buf);
        let mut closed = self.__dep_Streams.close(body);
        if matches!(closed, Union2::U2(_)) {
            ignore((match closed { Union2::U2(__v) => __v, _ => unreachable!() }));
        }
        if matches!(filled, Union2::U2(_)) {
            crate::scheduler::salvo_reply_wire::<Union2<PutObjectOutput, Checked<Union2<S3Error, AwsError>>>>(reply, Union2::<PutObjectOutput, Checked<Union2<S3Error, AwsError>>>::U2(err(checked(Union2::<S3Error, AwsError>::U2(AwsError { code: "StreamFailed".to_string(), message: format!("{}", to_str__4(&detach((match filled { Union2::U2(__v) => __v, _ => unreachable!() })))) })))));
            return;
        }
        let mut data: Vec<u8> = buf;
        let mut tag = format!("\"{}\"", (data.len() as i32));
        self.objects.insert(format!("{}/{}", bucket, key), data);
        crate::scheduler::salvo_reply_wire::<Union2<PutObjectOutput, Checked<Union2<S3Error, AwsError>>>>(reply, Union2::<PutObjectOutput, Checked<Union2<S3Error, AwsError>>>::U1(ok(PutObjectOutput { e_tag: Some(tag), expiration: None, checksum_crc32: None, checksum_crc32_c: None, checksum_crc64_nvme: None, checksum_sha1: None, checksum_sha256: None, checksum_sha512: None, checksum_md5: None, checksum_xxhash64: None, checksum_xxhash3: None, checksum_xxhash128: None, checksum_type: None, server_side_encryption: None, version_id: None, sse_customer_algorithm: None, sse_customer_key_md5: None, ssekms_key_id: None, ssekms_encryption_context: None, bucket_key_enabled: None, size: None, request_charged: None })));
    }

    fn get_object(&mut self, input: GetObjectInput, reply: crate::scheduler::SalvoReply) {
        let mut found = self.objects.get(&format!("{}/{}", input.bucket.clone(), input.key.clone()));
        if found.is_none() {
            (reply).send(Box::new(Union2::<GetObjectOutput, Checked<Union2<S3Error, AwsError>>>::U2(err(checked(Union2::<S3Error, AwsError>::U1(S3Error { code: "NoSuchKey".to_string(), message: "The specified key does not exist.".to_string(), status: 404, request_id: None, storage_class: None, access_tier: None }))))));
            return;
        }
        let mut data: Vec<u8> = found.unwrap().clone();
        let mut length = (((data.len() as i32)) as i64);
        (reply).send(Box::new(Union2::<GetObjectOutput, Checked<Union2<S3Error, AwsError>>>::U1(ok(GetObjectOutput { body: self.__dep_Streams.from_bytes(data), content_length: Some(length), delete_marker: None, accept_ranges: None, expiration: None, restore: None, last_modified: None, e_tag: None, checksum_crc32: None, checksum_crc32_c: None, checksum_crc64_nvme: None, checksum_sha1: None, checksum_sha256: None, checksum_sha512: None, checksum_md5: None, checksum_xxhash64: None, checksum_xxhash3: None, checksum_xxhash128: None, checksum_type: None, missing_meta: None, version_id: None, cache_control: None, content_disposition: None, content_encoding: None, content_language: None, content_range: None, content_type: None, website_redirect_location: None, server_side_encryption: None, metadata: None, sse_customer_algorithm: None, sse_customer_key_md5: None, ssekms_key_id: None, bucket_key_enabled: None, storage_class: None, request_charged: None, replication_status: None, parts_count: None, tag_count: None, object_lock_mode: None, object_lock_retain_until_date: None, object_lock_legal_hold_status: None, object_lock_event_hold: None, object_lock_event_hold_duration_days: None, object_lock_event_hold_duration_years: None }))));
    }
}

pub fn main() {
    crate::scheduler::salvo_set_protocols(vec![("Faults".to_string(), crate::core_actor::__PROTO_Faults.to_string()), ("Timer".to_string(), crate::time::__PROTO_Timer.to_string()), ("TimerCtl".to_string(), crate::time::__PROTO_TimerCtl.to_string()), ("Wheel".to_string(), crate::runtime_timers::__PROTO_Wheel.to_string())]);
    let console = crate::core_console::Console::shared(StdOutConsole::new());
    let __inst = std::sync::Arc::new(std::sync::Mutex::new(MemFs::new()));
    let fs2 = crate::fs::Fs::share_locked(__inst.clone());
    let streams = crate::stream::Streams::share_locked(__inst.clone());
    let mut written = write_str(&fs2, &streams, &("notes.txt".to_string()), &("hello from Salvo\nsecond line\n".to_string()));
    if matches!(written, Union2::U2(_)) {
        println(&console, &(format!("write: {}", to_str(&detach((match written { Union2::U2(__v) => __v, _ => unreachable!() }))))));
        return;
    }
    println(&console, &("-- FakeS3 --".to_string()));
    if true {
        let __inst2 = std::sync::Arc::new(std::sync::Mutex::new(FakeS3::new(streams.clone())));
        let s32 = crate::aws_s3::S3::share_locked(__inst2.clone());
        let s3_calls = crate::aws_s3::S3Calls::share_locked(__inst2.clone());
        round_trip(&s32, &fs2, &console, &streams, &("greeting.txt".to_string()));
        upload(&s32, &fs2, &console, &streams, &("notes".to_string()), &("unsized.txt".to_string()), &("notes.txt".to_string()), None);
        println(&console, &(format!("calls: {}", format!("[{}]", s3_calls.calls().iter().map(|__e| __e.to_string()).collect::<Vec<_>>().join(", ")))));
    }
    println(&console, &("-- MemS3 --".to_string()));
    if true {
        let s33 = crate::aws_s3::S3::locked(MemS3::new(streams.clone()));
        round_trip(&s33, &fs2, &console, &streams, &("greeting.txt".to_string()));
        download(&s33, &fs2, &console, &streams, &("notes".to_string()), &("missing.txt".to_string()), &("missing.txt".to_string()));
    }
}
