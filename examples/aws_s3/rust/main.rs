#![allow(non_snake_case, non_camel_case_types, unused_mut, unused_parens, unused_imports, dead_code, unreachable_code, unused_variables, path_statements, unused_must_use, suspicious_double_ref_op, non_upper_case_globals, unused_braces)]
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
#[path = "aws.rs"]
pub mod aws;
#[path = "aws/s3.rs"]
pub mod aws_s3;
#[path = "core/actor.rs"]
pub mod core_actor;
#[path = "core/basic.rs"]
pub mod core_basic;
#[path = "core/buffer.rs"]
pub mod core_buffer;
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
#[path = "core/other.rs"]
pub mod core_other;
#[path = "core/range.rs"]
pub mod core_range;
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
#[path = "fs/path.rs"]
pub mod fs_path;
#[path = "runtime.rs"]
pub mod runtime;
#[path = "runtime/routing.rs"]
pub mod runtime_routing;
#[path = "runtime/streams.rs"]
pub mod runtime_streams;
#[path = "runtime/timers.rs"]
pub mod runtime_timers;
#[path = "stream.rs"]
pub mod stream;
#[path = "time.rs"]
pub mod time;
#[path = "platform/core/buffer.rs"]
pub mod platform_core_buffer;
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
#[path = "platform/runtime/routing.rs"]
pub mod platform_runtime_routing;
#[path = "platform/runtime/streams.rs"]
pub mod platform_runtime_streams;
#[path = "platform/runtime.rs"]
pub mod platform_runtime;
#[path = "platform/time.rs"]
pub mod platform_time;

use crate::fs::AlreadyExists;
use crate::aws::AwsError;
use crate::core_bytes::Bytes;
use crate::core_checked::Checked;
use crate::core_console::Console;
use crate::fs::FileInfo;
use crate::fs::Fs;
use crate::aws_s3::GetObjectInput;
use crate::aws_s3::GetObjectOutput;
use crate::stream::InStream;
use crate::stream::InvalidUtf8;
use crate::fs::IoError;
use crate::core_map::Map;
use crate::fs::NotADirectory;
use crate::fs::NotFound;
use crate::stream::OutStream;
use crate::fs_path::Path;
use crate::fs::PathEscapes;
use crate::fs::PermissionDenied;
use crate::aws_s3::PutObjectInput;
use crate::aws_s3::PutObjectOutput;
use crate::aws_s3::S3;
use crate::aws_s3::S3Error;
use crate::stream::StreamFailed;
use crate::fs::Streaming;
use crate::stream::Streams;
use crate::core_checked::checked;
use crate::core_checked::detach;
use crate::core_result::err;
use crate::stream::fill_from;
use crate::core_map::get_platform;
use crate::core_checked::ignore;
use crate::core_bytes::mut_bytes;
use crate::core_map::mut_map_of_platform;
use crate::core_result::ok;
use crate::fs_path::path;
use crate::stream::pipe;
use crate::core_console::println;
use crate::core_map::put_platform;
use crate::fs::read_to_str;
use crate::fs::write_str;


pub fn describe(e: &crate::unions::Union2<crate::aws_s3::S3Error, crate::aws::AwsError>) -> String {
    if matches!(e, crate::unions::Union2::U2(_)) {
        let mut e_1 = match &e { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        return format!("{}: {}", e_1.code, e_1.message);
    };
    let mut e_2 = match &e { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
    if (match &e_2.code { __v => __v == &"NoSuchKey", #[allow(unreachable_patterns)] _ => false }) {
        return String::from("no such key");
    };
    let mut e_3 = match &e { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
    return format!("the service refused: {}", e_3.code.clone());
}

pub fn size_of(fs: &crate::fs::Fs, streams: &crate::stream::Streams, path: &crate::fs_path::Path) -> Option<i64> {
    let mut info: crate::unions::Union2<crate::fs::FileInfo, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> = fs.metadata(path);
    return if matches!(info, crate::unions::Union2::U1(_)) {
        let mut info_1 = match &info { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
        return Some(info_1.size);
    } else {
        let mut info_2 = match info { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        crate::core_checked::ignore::<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>(info_2);
        return None;
    };
}

pub fn upload(s3: &crate::aws_s3::S3, fs: &crate::fs::Fs, console: &crate::core_console::Console, streams: &crate::stream::Streams, bucket: &String, key: &String, path: &crate::fs_path::Path, mut length: Option<i64>) {
    let mut opened: crate::unions::Union2<crate::stream::InStream, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> = fs.open_read(path);
    if matches!(opened, crate::unions::Union2::U2(_)) {
        let mut opened_1 = match opened { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        crate::core_console::println(console, &format!("open {}: {}", crate::fs_path::to_str(path), crate::fs::to_str(&crate::core_checked::detach::<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>(opened_1))));
        return;
    };
    let mut put: crate::unions::Union2<crate::aws_s3::PutObjectOutput, crate::core_checked::Checked<crate::unions::Union2<crate::aws_s3::S3Error, crate::aws::AwsError>>> = {
        let (mut r, __wid) = crate::scheduler::salvo_waiter();
        crate::scheduler::salvo_waiter_decoder(__wid, (|__b: &[u8]| crate::wire::salvo_decode::<crate::unions::Union2<crate::aws_s3::PutObjectOutput, crate::core_checked::Checked<crate::unions::Union2<crate::aws_s3::S3Error, crate::aws::AwsError>>>>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg)));
        let mut opened_2 = match opened { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
        s3.put_object(crate::aws_s3::PutObjectInput { acl: None, body: opened_2, bucket: (bucket).clone(), cache_control: None, content_disposition: None, content_encoding: None, content_language: None, content_length: length.clone(), content_md5: None, content_type: None, checksum_algorithm: None, checksum_crc32: None, checksum_crc32_c: None, checksum_crc64_nvme: None, checksum_sha1: None, checksum_sha256: None, checksum_sha512: None, checksum_md5: None, checksum_xxhash64: None, checksum_xxhash3: None, checksum_xxhash128: None, if_match: None, if_none_match: None, grant_full_control: None, grant_read: None, grant_read_acp: None, grant_write_acp: None, key: (key).clone(), write_offset_bytes: None, metadata: None, server_side_encryption: None, storage_class: None, website_redirect_location: None, sse_customer_algorithm: None, sse_customer_key: None, sse_customer_key_md5: None, ssekms_key_id: None, ssekms_encryption_context: None, bucket_key_enabled: None, request_payer: None, tagging: None, object_lock_mode: None, object_lock_retain_until_date: None, object_lock_legal_hold_status: None, object_lock_event_hold: None, object_lock_event_hold_duration_days: None, object_lock_event_hold_duration_years: None, expected_bucket_owner: None }, r);
        *crate::scheduler::salvo_wait(__wid).downcast::<crate::unions::Union2<crate::aws_s3::PutObjectOutput, crate::core_checked::Checked<crate::unions::Union2<crate::aws_s3::S3Error, crate::aws::AwsError>>>>().expect("the awaited answer")
    };
    if matches!(put, crate::unions::Union2::U1(_)) {
        let mut put_3 = match &put { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
        crate::core_console::println(console, &format!("put {}: etag {}", key, {
            let mut __elv_4 = &put_3.e_tag;
            if __elv_4.is_none() {
                String::from("?")
            } else {
                let mut __some_5 = __elv_4.as_ref().unwrap();
                __some_5.clone()
            }
        }));
    } else {
        let mut put_6 = match put { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        crate::core_console::println(console, &format!("put {}: {}", key, crate::describe(&crate::core_checked::detach::<crate::unions::Union2<crate::aws_s3::S3Error, crate::aws::AwsError>>(put_6))));
    };
}

pub fn download(s3: &crate::aws_s3::S3, fs: &crate::fs::Fs, console: &crate::core_console::Console, streams: &crate::stream::Streams, bucket: &String, key: &String, path: &crate::fs_path::Path) {
    let mut got: crate::unions::Union2<crate::aws_s3::GetObjectOutput, crate::core_checked::Checked<crate::unions::Union2<crate::aws_s3::S3Error, crate::aws::AwsError>>> = {
        let (mut r, __wid) = crate::scheduler::salvo_waiter();
        crate::scheduler::salvo_waiter_decoder(__wid, (|_| None));
        s3.get_object(crate::aws_s3::GetObjectInput { bucket: (bucket).clone(), if_match: None, if_modified_since: None, if_none_match: None, if_unmodified_since: None, key: (key).clone(), range: None, response_cache_control: None, response_content_disposition: None, response_content_encoding: None, response_content_language: None, response_content_type: None, response_expires: None, version_id: None, sse_customer_algorithm: None, sse_customer_key: None, sse_customer_key_md5: None, request_payer: None, part_number: None, expected_bucket_owner: None, checksum_mode: None }, r);
        *crate::scheduler::salvo_wait(__wid).downcast::<crate::unions::Union2<crate::aws_s3::GetObjectOutput, crate::core_checked::Checked<crate::unions::Union2<crate::aws_s3::S3Error, crate::aws::AwsError>>>>().expect("the awaited answer")
    };
    if matches!(got, crate::unions::Union2::U2(_)) {
        let mut got_1 = match got { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        crate::core_console::println(console, &format!("get {}: {}", key, crate::describe(&crate::core_checked::detach::<crate::unions::Union2<crate::aws_s3::S3Error, crate::aws::AwsError>>(got_1))));
        return;
    };
    let mut got_2 = match got { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
    let mut __destructured_3: crate::aws_s3::GetObjectOutput = got_2;
    let mut body: crate::stream::InStream = __destructured_3.body;
    let mut content_length: Option<i64> = __destructured_3.content_length;
    crate::core_console::println(console, &format!("get {}: {} bytes", key, {
        let mut __elv_4 = &content_length;
        if __elv_4.is_none() {
            i64::wrapping_neg(1i64)
        } else {
            let mut __some_5 = __elv_4.unwrap();
            __some_5
        }
    }));
    let mut target: crate::unions::Union2<crate::stream::OutStream, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> = fs.open_write(path);
    if matches!(target, crate::unions::Union2::U2(_)) {
        let mut target_6 = match target { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        crate::core_console::println(console, &format!("open {}: {}", crate::fs_path::to_str(path), crate::fs::to_str(&crate::core_checked::detach::<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>(target_6))));
        let mut closed: crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> = streams.close__InStream(body);
        if matches!(closed, crate::unions::Union2::U2(_)) {
            let mut closed_7 = match closed { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            crate::core_checked::ignore::<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>(closed_7);
        };
        return;
    };
    let mut copied: crate::unions::Union2<i64, crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> = {
        let (mut r, __wid) = crate::scheduler::salvo_waiter();
        crate::scheduler::salvo_waiter_decoder(__wid, (|__b: &[u8]| crate::wire::salvo_decode::<crate::unions::Union2<i64, crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>>>(__b).map(|__v| std::boxed::Box::new(__v) as crate::scheduler::SalvoMsg)));
        let mut target_8 = match target { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
        crate::stream::pipe(streams, body, target_8, r);
        *crate::scheduler::salvo_wait(__wid).downcast::<crate::unions::Union2<i64, crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>>>().expect("the awaited answer")
    };
    if matches!(copied, crate::unions::Union2::U1(_)) {
        let mut copied_9 = match &copied { crate::unions::Union2::U1(__v) => *__v, _ => unreachable!() };
        crate::core_console::println(console, &format!("piped {} bytes into {}", copied_9, crate::fs_path::to_str(path)));
    } else {
        let mut copied_10 = match copied { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        crate::core_console::println(console, &format!("pipe: {}", crate::stream::to_str(&crate::core_checked::detach::<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>(copied_10))));
    };
}

pub fn round_trip(s3: &crate::aws_s3::S3, fs: &crate::fs::Fs, console: &crate::core_console::Console, streams: &crate::stream::Streams, key: &String) {
    crate::upload(s3, fs, console, streams, &String::from("notes"), &(key).clone(), &crate::fs_path::path(&String::from("notes.txt")), crate::size_of(fs, streams, &crate::fs_path::path(&String::from("notes.txt"))));
    crate::download(s3, fs, console, streams, &String::from("notes"), &(key).clone(), &crate::fs_path::path(&String::from("back.txt")));
    let mut back: crate::unions::Union2<String, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> = crate::fs::read_to_str(fs, streams, &crate::fs_path::path(&String::from("back.txt")));
    if matches!(back, crate::unions::Union2::U1(_)) {
        let mut back_1 = match &back { crate::unions::Union2::U1(__v) => __v, _ => unreachable!() };
        crate::core_console::println(console, &format!("back.txt: {} bytes", crate::core_string::size_platform(back_1)));
        console.print(back_1);
    } else {
        let mut back_2 = match back { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        crate::core_console::println(console, &format!("back.txt: {}", crate::fs::to_str(&crate::core_checked::detach::<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>(back_2))));
    };
}

pub struct MemS3 {
    __dep0: crate::stream::Streams,
    objects: crate::core_map::Map<String, crate::core_bytes::Bytes>,
}

impl MemS3 {
    pub fn new(__dep0: crate::stream::Streams) -> Self {
        Self {
            __dep0,
            objects: crate::core_map::mut_map_of_platform::<String, crate::core_bytes::Bytes>(vec![], &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..]))
        }
    }
}

impl crate::aws_s3::__Stateful_S3 for MemS3 {
    fn put_object(&mut self, input: crate::aws_s3::PutObjectInput, reply: crate::scheduler::SalvoReply) {
        let mut __destructured_1: crate::aws_s3::PutObjectInput = input;
        let mut bucket: String = __destructured_1.bucket;
        let mut key: String = __destructured_1.key;
        let mut body: crate::stream::InStream = __destructured_1.body;
        let mut buf: crate::core_bytes::Bytes = crate::core_bytes::mut_bytes(vec![]);
        let mut filled: crate::unions::Union2<i64, crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> = crate::stream::fill_from(&self.__dep0, &body, &mut buf);
        let mut closed: crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> = self.__dep0.close__InStream(body);
        if matches!(closed, crate::unions::Union2::U2(_)) {
            let mut closed_2 = match closed { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            crate::core_checked::ignore::<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>(closed_2);
        };
        if matches!(filled, crate::unions::Union2::U2(_)) {
            let mut filled_3 = match filled { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            crate::scheduler::salvo_reply_wire::<crate::unions::Union2<crate::aws_s3::PutObjectOutput, crate::core_checked::Checked<crate::unions::Union2<crate::aws_s3::S3Error, crate::aws::AwsError>>>>(reply, crate::unions::Union2::U2(crate::core_result::err::<crate::core_checked::Checked<crate::unions::Union2<crate::aws_s3::S3Error, crate::aws::AwsError>>>(crate::core_checked::checked::<crate::unions::Union2<crate::aws_s3::S3Error, crate::aws::AwsError>>(crate::unions::Union2::U2(crate::aws::AwsError { code: String::from("StreamFailed"), message: format!("{}", crate::stream::to_str(&crate::core_checked::detach::<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>(filled_3))) })))));
            return;
        };
        let mut data: crate::core_bytes::Bytes = buf.clone();
        let mut tag: String = format!("\"{}\"", crate::core_bytes::size_platform(&data));
        crate::core_map::put_platform::<String, crate::core_bytes::Bytes>(&mut self.objects, format!("{}/{}", bucket, key), data, &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..]));
        crate::scheduler::salvo_reply_wire::<crate::unions::Union2<crate::aws_s3::PutObjectOutput, crate::core_checked::Checked<crate::unions::Union2<crate::aws_s3::S3Error, crate::aws::AwsError>>>>(reply, crate::unions::Union2::U1(crate::core_result::ok::<crate::aws_s3::PutObjectOutput>(crate::aws_s3::PutObjectOutput { expiration: None, e_tag: Some(tag.clone()), checksum_crc32: None, checksum_crc32_c: None, checksum_crc64_nvme: None, checksum_sha1: None, checksum_sha256: None, checksum_sha512: None, checksum_md5: None, checksum_xxhash64: None, checksum_xxhash3: None, checksum_xxhash128: None, checksum_type: None, server_side_encryption: None, version_id: None, sse_customer_algorithm: None, sse_customer_key_md5: None, ssekms_key_id: None, ssekms_encryption_context: None, bucket_key_enabled: None, size: None, request_charged: None })));
    }
    fn get_object(&mut self, input: crate::aws_s3::GetObjectInput, reply: crate::scheduler::SalvoReply) {
        let mut __tmp1 = format!("{}/{}", input.bucket, input.key);
        let mut found: Option<&crate::core_bytes::Bytes> = crate::core_map::get_platform::<String, crate::core_bytes::Bytes>(&self.objects, &__tmp1, &mut |__a0: &String| { let mut __h = std::hash::DefaultHasher::new(); std::hash::Hash::hash(&__a0[..], &mut __h); (std::hash::Hasher::finish(&__h) as i64) }, &mut |__a0: &String, __a1: &String| (&__a0[..] == &__a1[..]));
        if found.is_none() {
            (reply).send(std::boxed::Box::<crate::unions::Union2<crate::aws_s3::GetObjectOutput, crate::core_checked::Checked<crate::unions::Union2<crate::aws_s3::S3Error, crate::aws::AwsError>>>>::new(crate::unions::Union2::U2(crate::core_result::err::<crate::core_checked::Checked<crate::unions::Union2<crate::aws_s3::S3Error, crate::aws::AwsError>>>(crate::core_checked::checked::<crate::unions::Union2<crate::aws_s3::S3Error, crate::aws::AwsError>>(crate::unions::Union2::U1(crate::aws_s3::S3Error { code: String::from("NoSuchKey"), message: String::from("The specified key does not exist."), status: 404i32, request_id: None, storage_class: None, access_tier: None }))))));
            return;
        };
        let mut found_1 = found.unwrap();
        let mut data: crate::core_bytes::Bytes = (found_1).clone();
        let mut length: i64 = ((crate::core_bytes::size_platform(&data)) as i64);
        (reply).send(std::boxed::Box::<crate::unions::Union2<crate::aws_s3::GetObjectOutput, crate::core_checked::Checked<crate::unions::Union2<crate::aws_s3::S3Error, crate::aws::AwsError>>>>::new(crate::unions::Union2::U1(crate::core_result::ok::<crate::aws_s3::GetObjectOutput>(crate::aws_s3::GetObjectOutput { body: self.__dep0.from_bytes(data), delete_marker: None, accept_ranges: None, expiration: None, restore: None, last_modified: None, content_length: Some(length), e_tag: None, checksum_crc32: None, checksum_crc32_c: None, checksum_crc64_nvme: None, checksum_sha1: None, checksum_sha256: None, checksum_sha512: None, checksum_md5: None, checksum_xxhash64: None, checksum_xxhash3: None, checksum_xxhash128: None, checksum_type: None, missing_meta: None, version_id: None, cache_control: None, content_disposition: None, content_encoding: None, content_language: None, content_range: None, content_type: None, website_redirect_location: None, server_side_encryption: None, metadata: None, sse_customer_algorithm: None, sse_customer_key_md5: None, ssekms_key_id: None, bucket_key_enabled: None, storage_class: None, request_charged: None, replication_status: None, parts_count: None, tag_count: None, object_lock_mode: None, object_lock_retain_until_date: None, object_lock_legal_hold_status: None, object_lock_event_hold: None, object_lock_event_hold_duration_days: None, object_lock_event_hold_duration_years: None }))));
    }
}

pub fn main() {
    crate::scheduler::salvo_set_protocols(vec![("Faults".to_string(), crate::core_actor::__PROTO_Faults.to_string()), ("Timer".to_string(), crate::time::__PROTO_Timer.to_string()), ("TimerCtl".to_string(), crate::time::__PROTO_TimerCtl.to_string()), ("Wheel".to_string(), crate::runtime_timers::__PROTO_Wheel.to_string())]);
    let mut __use_1: crate::core_console::__Platform_StdOutConsole = crate::core_console::__Platform_StdOutConsole::new();
    let __handle_2 = crate::core_console::Console::shared(__use_1);
    let __use_3 = std::sync::Arc::new(std::sync::Mutex::new(crate::fs_mem::MemFs::new()));
    let __handle_4 = crate::fs::Fs::share_locked(__use_3.clone());
    let __handle_5 = crate::stream::Streams::share_locked(__use_3.clone());
    let mut written: crate::unions::Union2<i64, crate::core_checked::Checked<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>> = crate::fs::write_str(&__handle_4, &__handle_5, &crate::fs_path::path(&String::from("notes.txt")), &String::from("hello from Salvo\nsecond line\n"));
    if matches!(written, crate::unions::Union2::U2(_)) {
        let mut written_6 = match written { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
        crate::core_console::println(&__handle_2, &format!("write: {}", crate::fs::to_str(&crate::core_checked::detach::<crate::unions::Union7<crate::fs::NotFound, crate::fs::PermissionDenied, crate::fs::AlreadyExists, crate::fs::NotADirectory, crate::fs::PathEscapes, crate::fs::IoError, crate::fs::Streaming>>(written_6))));
        return;
    };
    crate::core_console::println(&__handle_2, &String::from("-- FakeS3 --"));
    {
        let __use_7 = std::sync::Arc::new(std::sync::Mutex::new(crate::aws_s3::FakeS3::new(__handle_5.clone())));
        let __handle_8 = crate::aws_s3::S3::share_locked(__use_7.clone());
        let __handle_9 = crate::aws_s3::S3Calls::share_locked(__use_7.clone());
        crate::round_trip(&__handle_8, &__handle_4, &__handle_2, &__handle_5, &String::from("greeting.txt"));
        crate::upload(&__handle_8, &__handle_4, &__handle_2, &__handle_5, &String::from("notes"), &String::from("unsized.txt"), &crate::fs_path::path(&String::from("notes.txt")), None);
        crate::core_console::println(&__handle_2, &format!("calls: {}", crate::core_list::to_str(&__handle_9.calls(), &mut |__a0| format!("{}", __a0))));
    };
    crate::core_console::println(&__handle_2, &String::from("-- MemS3 --"));
    {
        let mut __use_10: crate::MemS3 = crate::MemS3::new(__handle_5.clone());
        let __handle_11 = crate::aws_s3::S3::locked(__use_10);
        crate::round_trip(&__handle_11, &__handle_4, &__handle_2, &__handle_5, &String::from("greeting.txt"));
        crate::download(&__handle_11, &__handle_4, &__handle_2, &__handle_5, &String::from("notes"), &String::from("missing.txt"), &crate::fs_path::path(&String::from("missing.txt")));
    };
}
