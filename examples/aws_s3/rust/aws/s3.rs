use crate::aws::AwsError;
use crate::core_checked::Checked;
use crate::stream::InStream;
use crate::time::Instant;
use crate::stream::InvalidUtf8;
use crate::core_map::Map;
use crate::stream::StreamFailed;
use crate::stream::Streams;
use crate::core_list::add_platform;
use crate::core_bytes::bytes_of;
use crate::core_checked::checked;
use crate::core_result::err;
use crate::core_checked::ignore;
use crate::core_result::ok;


#[derive(Clone, Debug, PartialEq)]
pub struct PutObjectInput {
    pub acl: Option<String>,
    pub body: crate::stream::InStream,
    pub bucket: String,
    pub cache_control: Option<String>,
    pub content_disposition: Option<String>,
    pub content_encoding: Option<String>,
    pub content_language: Option<String>,
    pub content_length: Option<i64>,
    pub content_md5: Option<String>,
    pub content_type: Option<String>,
    pub checksum_algorithm: Option<String>,
    pub checksum_crc32: Option<String>,
    pub checksum_crc32_c: Option<String>,
    pub checksum_crc64_nvme: Option<String>,
    pub checksum_sha1: Option<String>,
    pub checksum_sha256: Option<String>,
    pub checksum_sha512: Option<String>,
    pub checksum_md5: Option<String>,
    pub checksum_xxhash64: Option<String>,
    pub checksum_xxhash3: Option<String>,
    pub checksum_xxhash128: Option<String>,
    pub if_match: Option<String>,
    pub if_none_match: Option<String>,
    pub grant_full_control: Option<String>,
    pub grant_read: Option<String>,
    pub grant_read_acp: Option<String>,
    pub grant_write_acp: Option<String>,
    pub key: String,
    pub write_offset_bytes: Option<i64>,
    pub metadata: Option<crate::core_map::Map<String, String>>,
    pub server_side_encryption: Option<String>,
    pub storage_class: Option<String>,
    pub website_redirect_location: Option<String>,
    pub sse_customer_algorithm: Option<String>,
    pub sse_customer_key: Option<String>,
    pub sse_customer_key_md5: Option<String>,
    pub ssekms_key_id: Option<String>,
    pub ssekms_encryption_context: Option<String>,
    pub bucket_key_enabled: Option<bool>,
    pub request_payer: Option<String>,
    pub tagging: Option<String>,
    pub object_lock_mode: Option<String>,
    pub object_lock_retain_until_date: Option<crate::time::Instant>,
    pub object_lock_legal_hold_status: Option<String>,
    pub object_lock_event_hold: Option<String>,
    pub object_lock_event_hold_duration_days: Option<i32>,
    pub object_lock_event_hold_duration_years: Option<i32>,
    pub expected_bucket_owner: Option<String>,
}

pub fn close__PutObjectInput(streams: &crate::stream::Streams, mut value: crate::aws_s3::PutObjectInput) -> crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> {
    return streams.close__InStream(value.body);
}

#[derive(Clone, Debug, PartialEq)]
pub struct PutObjectOutput {
    pub expiration: Option<String>,
    pub e_tag: Option<String>,
    pub checksum_crc32: Option<String>,
    pub checksum_crc32_c: Option<String>,
    pub checksum_crc64_nvme: Option<String>,
    pub checksum_sha1: Option<String>,
    pub checksum_sha256: Option<String>,
    pub checksum_sha512: Option<String>,
    pub checksum_md5: Option<String>,
    pub checksum_xxhash64: Option<String>,
    pub checksum_xxhash3: Option<String>,
    pub checksum_xxhash128: Option<String>,
    pub checksum_type: Option<String>,
    pub server_side_encryption: Option<String>,
    pub version_id: Option<String>,
    pub sse_customer_algorithm: Option<String>,
    pub sse_customer_key_md5: Option<String>,
    pub ssekms_key_id: Option<String>,
    pub ssekms_encryption_context: Option<String>,
    pub bucket_key_enabled: Option<bool>,
    pub size: Option<i64>,
    pub request_charged: Option<String>,
}

impl crate::wire::__Wire for PutObjectOutput {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.expiration, out);
        crate::wire::__Wire::__enc(&self.e_tag, out);
        crate::wire::__Wire::__enc(&self.checksum_crc32, out);
        crate::wire::__Wire::__enc(&self.checksum_crc32_c, out);
        crate::wire::__Wire::__enc(&self.checksum_crc64_nvme, out);
        crate::wire::__Wire::__enc(&self.checksum_sha1, out);
        crate::wire::__Wire::__enc(&self.checksum_sha256, out);
        crate::wire::__Wire::__enc(&self.checksum_sha512, out);
        crate::wire::__Wire::__enc(&self.checksum_md5, out);
        crate::wire::__Wire::__enc(&self.checksum_xxhash64, out);
        crate::wire::__Wire::__enc(&self.checksum_xxhash3, out);
        crate::wire::__Wire::__enc(&self.checksum_xxhash128, out);
        crate::wire::__Wire::__enc(&self.checksum_type, out);
        crate::wire::__Wire::__enc(&self.server_side_encryption, out);
        crate::wire::__Wire::__enc(&self.version_id, out);
        crate::wire::__Wire::__enc(&self.sse_customer_algorithm, out);
        crate::wire::__Wire::__enc(&self.sse_customer_key_md5, out);
        crate::wire::__Wire::__enc(&self.ssekms_key_id, out);
        crate::wire::__Wire::__enc(&self.ssekms_encryption_context, out);
        crate::wire::__Wire::__enc(&self.bucket_key_enabled, out);
        crate::wire::__Wire::__enc(&self.size, out);
        crate::wire::__Wire::__enc(&self.request_charged, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            expiration: crate::wire::__Wire::__dec(r)?,
            e_tag: crate::wire::__Wire::__dec(r)?,
            checksum_crc32: crate::wire::__Wire::__dec(r)?,
            checksum_crc32_c: crate::wire::__Wire::__dec(r)?,
            checksum_crc64_nvme: crate::wire::__Wire::__dec(r)?,
            checksum_sha1: crate::wire::__Wire::__dec(r)?,
            checksum_sha256: crate::wire::__Wire::__dec(r)?,
            checksum_sha512: crate::wire::__Wire::__dec(r)?,
            checksum_md5: crate::wire::__Wire::__dec(r)?,
            checksum_xxhash64: crate::wire::__Wire::__dec(r)?,
            checksum_xxhash3: crate::wire::__Wire::__dec(r)?,
            checksum_xxhash128: crate::wire::__Wire::__dec(r)?,
            checksum_type: crate::wire::__Wire::__dec(r)?,
            server_side_encryption: crate::wire::__Wire::__dec(r)?,
            version_id: crate::wire::__Wire::__dec(r)?,
            sse_customer_algorithm: crate::wire::__Wire::__dec(r)?,
            sse_customer_key_md5: crate::wire::__Wire::__dec(r)?,
            ssekms_key_id: crate::wire::__Wire::__dec(r)?,
            ssekms_encryption_context: crate::wire::__Wire::__dec(r)?,
            bucket_key_enabled: crate::wire::__Wire::__dec(r)?,
            size: crate::wire::__Wire::__dec(r)?,
            request_charged: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GetObjectInput {
    pub bucket: String,
    pub if_match: Option<String>,
    pub if_modified_since: Option<crate::time::Instant>,
    pub if_none_match: Option<String>,
    pub if_unmodified_since: Option<crate::time::Instant>,
    pub key: String,
    pub range: Option<String>,
    pub response_cache_control: Option<String>,
    pub response_content_disposition: Option<String>,
    pub response_content_encoding: Option<String>,
    pub response_content_language: Option<String>,
    pub response_content_type: Option<String>,
    pub response_expires: Option<crate::time::Instant>,
    pub version_id: Option<String>,
    pub sse_customer_algorithm: Option<String>,
    pub sse_customer_key: Option<String>,
    pub sse_customer_key_md5: Option<String>,
    pub request_payer: Option<String>,
    pub part_number: Option<i32>,
    pub expected_bucket_owner: Option<String>,
    pub checksum_mode: Option<String>,
}

impl crate::wire::__Wire for GetObjectInput {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.bucket, out);
        crate::wire::__Wire::__enc(&self.if_match, out);
        crate::wire::__Wire::__enc(&self.if_modified_since, out);
        crate::wire::__Wire::__enc(&self.if_none_match, out);
        crate::wire::__Wire::__enc(&self.if_unmodified_since, out);
        crate::wire::__Wire::__enc(&self.key, out);
        crate::wire::__Wire::__enc(&self.range, out);
        crate::wire::__Wire::__enc(&self.response_cache_control, out);
        crate::wire::__Wire::__enc(&self.response_content_disposition, out);
        crate::wire::__Wire::__enc(&self.response_content_encoding, out);
        crate::wire::__Wire::__enc(&self.response_content_language, out);
        crate::wire::__Wire::__enc(&self.response_content_type, out);
        crate::wire::__Wire::__enc(&self.response_expires, out);
        crate::wire::__Wire::__enc(&self.version_id, out);
        crate::wire::__Wire::__enc(&self.sse_customer_algorithm, out);
        crate::wire::__Wire::__enc(&self.sse_customer_key, out);
        crate::wire::__Wire::__enc(&self.sse_customer_key_md5, out);
        crate::wire::__Wire::__enc(&self.request_payer, out);
        crate::wire::__Wire::__enc(&self.part_number, out);
        crate::wire::__Wire::__enc(&self.expected_bucket_owner, out);
        crate::wire::__Wire::__enc(&self.checksum_mode, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            bucket: crate::wire::__Wire::__dec(r)?,
            if_match: crate::wire::__Wire::__dec(r)?,
            if_modified_since: crate::wire::__Wire::__dec(r)?,
            if_none_match: crate::wire::__Wire::__dec(r)?,
            if_unmodified_since: crate::wire::__Wire::__dec(r)?,
            key: crate::wire::__Wire::__dec(r)?,
            range: crate::wire::__Wire::__dec(r)?,
            response_cache_control: crate::wire::__Wire::__dec(r)?,
            response_content_disposition: crate::wire::__Wire::__dec(r)?,
            response_content_encoding: crate::wire::__Wire::__dec(r)?,
            response_content_language: crate::wire::__Wire::__dec(r)?,
            response_content_type: crate::wire::__Wire::__dec(r)?,
            response_expires: crate::wire::__Wire::__dec(r)?,
            version_id: crate::wire::__Wire::__dec(r)?,
            sse_customer_algorithm: crate::wire::__Wire::__dec(r)?,
            sse_customer_key: crate::wire::__Wire::__dec(r)?,
            sse_customer_key_md5: crate::wire::__Wire::__dec(r)?,
            request_payer: crate::wire::__Wire::__dec(r)?,
            part_number: crate::wire::__Wire::__dec(r)?,
            expected_bucket_owner: crate::wire::__Wire::__dec(r)?,
            checksum_mode: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GetObjectOutput {
    pub body: crate::stream::InStream,
    pub delete_marker: Option<bool>,
    pub accept_ranges: Option<String>,
    pub expiration: Option<String>,
    pub restore: Option<String>,
    pub last_modified: Option<crate::time::Instant>,
    pub content_length: Option<i64>,
    pub e_tag: Option<String>,
    pub checksum_crc32: Option<String>,
    pub checksum_crc32_c: Option<String>,
    pub checksum_crc64_nvme: Option<String>,
    pub checksum_sha1: Option<String>,
    pub checksum_sha256: Option<String>,
    pub checksum_sha512: Option<String>,
    pub checksum_md5: Option<String>,
    pub checksum_xxhash64: Option<String>,
    pub checksum_xxhash3: Option<String>,
    pub checksum_xxhash128: Option<String>,
    pub checksum_type: Option<String>,
    pub missing_meta: Option<i32>,
    pub version_id: Option<String>,
    pub cache_control: Option<String>,
    pub content_disposition: Option<String>,
    pub content_encoding: Option<String>,
    pub content_language: Option<String>,
    pub content_range: Option<String>,
    pub content_type: Option<String>,
    pub website_redirect_location: Option<String>,
    pub server_side_encryption: Option<String>,
    pub metadata: Option<crate::core_map::Map<String, String>>,
    pub sse_customer_algorithm: Option<String>,
    pub sse_customer_key_md5: Option<String>,
    pub ssekms_key_id: Option<String>,
    pub bucket_key_enabled: Option<bool>,
    pub storage_class: Option<String>,
    pub request_charged: Option<String>,
    pub replication_status: Option<String>,
    pub parts_count: Option<i32>,
    pub tag_count: Option<i32>,
    pub object_lock_mode: Option<String>,
    pub object_lock_retain_until_date: Option<crate::time::Instant>,
    pub object_lock_legal_hold_status: Option<String>,
    pub object_lock_event_hold: Option<String>,
    pub object_lock_event_hold_duration_days: Option<i32>,
    pub object_lock_event_hold_duration_years: Option<i32>,
}

pub fn close__GetObjectOutput(streams: &crate::stream::Streams, mut value: crate::aws_s3::GetObjectOutput) -> crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> {
    return streams.close__InStream(value.body);
}

#[derive(Clone, Debug, PartialEq)]
pub struct S3Error {
    pub code: String,
    pub message: String,
    pub status: i32,
    pub request_id: Option<String>,
    pub storage_class: Option<String>,
    pub access_tier: Option<String>,
}

impl crate::wire::__Wire for S3Error {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.code, out);
        crate::wire::__Wire::__enc(&self.message, out);
        crate::wire::__Wire::__enc(&self.status, out);
        crate::wire::__Wire::__enc(&self.request_id, out);
        crate::wire::__Wire::__enc(&self.storage_class, out);
        crate::wire::__Wire::__enc(&self.access_tier, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            code: crate::wire::__Wire::__dec(r)?,
            message: crate::wire::__Wire::__dec(r)?,
            status: crate::wire::__Wire::__dec(r)?,
            request_id: crate::wire::__Wire::__dec(r)?,
            storage_class: crate::wire::__Wire::__dec(r)?,
            access_tier: crate::wire::__Wire::__dec(r)?,
        })
    }
}

pub type S3Failure = crate::unions::Union2<crate::aws_s3::S3Error, crate::aws::AwsError>;

/// Factories for the host: one per arm of the union [platform-factory].
impl S3Failure {
    pub fn s3_error(value: crate::aws_s3::S3Error) -> Self {
        crate::unions::Union2::U1(value)
    }
    pub fn aws_error(value: crate::aws::AwsError) -> Self {
        crate::unions::Union2::U2(value)
    }
}

pub trait __Stateless_S3: Send + Sync {
    fn put_object(&self, input: crate::aws_s3::PutObjectInput, reply: crate::scheduler::SalvoReply);
    fn get_object(&self, input: crate::aws_s3::GetObjectInput, reply: crate::scheduler::SalvoReply);
}

pub trait __Stateful_S3: Send {
    fn put_object(&mut self, input: crate::aws_s3::PutObjectInput, reply: crate::scheduler::SalvoReply);
    fn get_object(&mut self, input: crate::aws_s3::GetObjectInput, reply: crate::scheduler::SalvoReply);
}

pub struct S3 {
    inner: __Inner_S3,
}

pub enum __Inner_S3 {
    Shared(std::sync::Arc<dyn __Stateless_S3>),
    Locked(std::sync::Arc<std::sync::Mutex<dyn __Stateful_S3>>),
}

impl Clone for S3 {
    fn clone(&self) -> Self {
        Self { inner: match &self.inner {
            __Inner_S3::Shared(h) => __Inner_S3::Shared(h.clone()),
            __Inner_S3::Locked(h) => __Inner_S3::Locked(h.clone()),
        } }
    }
}

impl S3 {
    pub fn shared<__H: __Stateless_S3 + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_S3::Shared(std::sync::Arc::new(inner)) }
    }
    pub fn share_shared(inner: std::sync::Arc<dyn __Stateless_S3>) -> Self {
        Self { inner: __Inner_S3::Shared(inner) }
    }
    pub fn locked<__H: __Stateful_S3 + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_S3::Locked(std::sync::Arc::new(std::sync::Mutex::new(inner))) }
    }
    pub fn share_locked(inner: std::sync::Arc<std::sync::Mutex<dyn __Stateful_S3>>) -> Self {
        Self { inner: __Inner_S3::Locked(inner) }
    }
    pub fn put_object(&self, input: crate::aws_s3::PutObjectInput, reply: crate::scheduler::SalvoReply) {
        match &self.inner {
            __Inner_S3::Shared(h) => h.put_object(input, reply),
            __Inner_S3::Locked(h) => h.lock().unwrap().put_object(input, reply),
        }
    }
    pub fn get_object(&self, input: crate::aws_s3::GetObjectInput, reply: crate::scheduler::SalvoReply) {
        match &self.inner {
            __Inner_S3::Shared(h) => h.get_object(input, reply),
            __Inner_S3::Locked(h) => h.lock().unwrap().get_object(input, reply),
        }
    }
}

pub trait __Stateless_S3Calls: Send + Sync {
    fn calls(&self) -> Vec<String>;
}

pub trait __Stateful_S3Calls: Send {
    fn calls(&mut self) -> Vec<String>;
}

pub struct S3Calls {
    inner: __Inner_S3Calls,
}

pub enum __Inner_S3Calls {
    Shared(std::sync::Arc<dyn __Stateless_S3Calls>),
    Locked(std::sync::Arc<std::sync::Mutex<dyn __Stateful_S3Calls>>),
}

impl Clone for S3Calls {
    fn clone(&self) -> Self {
        Self { inner: match &self.inner {
            __Inner_S3Calls::Shared(h) => __Inner_S3Calls::Shared(h.clone()),
            __Inner_S3Calls::Locked(h) => __Inner_S3Calls::Locked(h.clone()),
        } }
    }
}

impl S3Calls {
    pub fn shared<__H: __Stateless_S3Calls + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_S3Calls::Shared(std::sync::Arc::new(inner)) }
    }
    pub fn share_shared(inner: std::sync::Arc<dyn __Stateless_S3Calls>) -> Self {
        Self { inner: __Inner_S3Calls::Shared(inner) }
    }
    pub fn locked<__H: __Stateful_S3Calls + 'static>(inner: __H) -> Self {
        Self { inner: __Inner_S3Calls::Locked(std::sync::Arc::new(std::sync::Mutex::new(inner))) }
    }
    pub fn share_locked(inner: std::sync::Arc<std::sync::Mutex<dyn __Stateful_S3Calls>>) -> Self {
        Self { inner: __Inner_S3Calls::Locked(inner) }
    }
    pub fn calls(&self) -> Vec<String> {
        match &self.inner {
            __Inner_S3Calls::Shared(h) => h.calls(),
            __Inner_S3Calls::Locked(h) => h.lock().unwrap().calls(),
        }
    }
}

pub struct FakeS3 {
    __dep0: crate::stream::Streams,
    recorded: Vec<String>,
}

impl FakeS3 {
    pub fn new(__dep0: crate::stream::Streams) -> Self {
        Self {
            __dep0,
            recorded: vec![]
        }
    }
}

impl crate::aws_s3::__Stateful_S3 for FakeS3 {
    fn put_object(&mut self, input: crate::aws_s3::PutObjectInput, reply: crate::scheduler::SalvoReply) {
        crate::core_list::add_platform::<String>(&mut self.recorded, String::from("put_object"));
        let mut r#unsized: bool = input.content_length.is_none();
        let mut closed: crate::unions::Union2<(), crate::core_checked::Checked<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>> = crate::aws_s3::close__PutObjectInput(&self.__dep0, input);
        if matches!(closed, crate::unions::Union2::U2(_)) {
            let mut closed_1 = match closed { crate::unions::Union2::U2(__v) => __v, _ => unreachable!() };
            crate::core_checked::ignore::<crate::unions::Union2<crate::stream::InvalidUtf8, crate::stream::StreamFailed>>(closed_1);
        };
        if r#unsized {
            crate::scheduler::salvo_reply_wire::<crate::unions::Union2<crate::aws_s3::PutObjectOutput, crate::core_checked::Checked<crate::unions::Union2<crate::aws_s3::S3Error, crate::aws::AwsError>>>>(reply, crate::unions::Union2::U2(crate::core_result::err::<crate::core_checked::Checked<crate::unions::Union2<crate::aws_s3::S3Error, crate::aws::AwsError>>>(crate::core_checked::checked::<crate::unions::Union2<crate::aws_s3::S3Error, crate::aws::AwsError>>(crate::unions::Union2::U2(crate::aws::AwsError { code: String::from("MissingContentLength"), message: String::from("S3 PutObject streams its body, so the input needs content_length: the body's length in bytes") })))));
            return;
        };
        crate::scheduler::salvo_reply_wire::<crate::unions::Union2<crate::aws_s3::PutObjectOutput, crate::core_checked::Checked<crate::unions::Union2<crate::aws_s3::S3Error, crate::aws::AwsError>>>>(reply, crate::unions::Union2::U1(crate::core_result::ok::<crate::aws_s3::PutObjectOutput>(crate::aws_s3::PutObjectOutput { expiration: None, e_tag: None, checksum_crc32: None, checksum_crc32_c: None, checksum_crc64_nvme: None, checksum_sha1: None, checksum_sha256: None, checksum_sha512: None, checksum_md5: None, checksum_xxhash64: None, checksum_xxhash3: None, checksum_xxhash128: None, checksum_type: None, server_side_encryption: None, version_id: None, sse_customer_algorithm: None, sse_customer_key_md5: None, ssekms_key_id: None, ssekms_encryption_context: None, bucket_key_enabled: None, size: None, request_charged: None })));
    }
    fn get_object(&mut self, input: crate::aws_s3::GetObjectInput, reply: crate::scheduler::SalvoReply) {
        crate::core_list::add_platform::<String>(&mut self.recorded, String::from("get_object"));
        (reply).send(std::boxed::Box::<crate::unions::Union2<crate::aws_s3::GetObjectOutput, crate::core_checked::Checked<crate::unions::Union2<crate::aws_s3::S3Error, crate::aws::AwsError>>>>::new(crate::unions::Union2::U1(crate::core_result::ok::<crate::aws_s3::GetObjectOutput>(crate::aws_s3::GetObjectOutput { body: self.__dep0.from_bytes(crate::core_bytes::bytes_of(vec![])), delete_marker: None, accept_ranges: None, expiration: None, restore: None, last_modified: None, content_length: None, e_tag: None, checksum_crc32: None, checksum_crc32_c: None, checksum_crc64_nvme: None, checksum_sha1: None, checksum_sha256: None, checksum_sha512: None, checksum_md5: None, checksum_xxhash64: None, checksum_xxhash3: None, checksum_xxhash128: None, checksum_type: None, missing_meta: None, version_id: None, cache_control: None, content_disposition: None, content_encoding: None, content_language: None, content_range: None, content_type: None, website_redirect_location: None, server_side_encryption: None, metadata: None, sse_customer_algorithm: None, sse_customer_key_md5: None, ssekms_key_id: None, bucket_key_enabled: None, storage_class: None, request_charged: None, replication_status: None, parts_count: None, tag_count: None, object_lock_mode: None, object_lock_retain_until_date: None, object_lock_legal_hold_status: None, object_lock_event_hold: None, object_lock_event_hold_duration_days: None, object_lock_event_hold_duration_years: None }))));
    }
}

impl crate::aws_s3::__Stateful_S3Calls for FakeS3 {
    fn calls(&mut self) -> Vec<String> {
        return (self.recorded).clone();
    }
}
