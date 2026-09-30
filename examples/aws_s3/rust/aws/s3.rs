use crate::aws::*;
use crate::collections::*;
use crate::core_actor::*;
use crate::core_bytes::*;
use crate::core_checked::*;
use crate::core_iterator::*;
use crate::core_list::*;
use crate::core_map::*;
use crate::core_result::*;
use crate::core_set::*;
use crate::core_sorted::*;
use crate::core_string::*;
use crate::stream::*;
use crate::time::*;
use crate::unions::*;

#[derive(Clone, Debug, PartialEq)]
pub struct ObjectCannedACLPrivate {
}

impl crate::wire::__Wire for ObjectCannedACLPrivate {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ObjectCannedACLPublicRead {
}

impl crate::wire::__Wire for ObjectCannedACLPublicRead {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ObjectCannedACLPublicReadWrite {
}

impl crate::wire::__Wire for ObjectCannedACLPublicReadWrite {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ObjectCannedACLAuthenticatedRead {
}

impl crate::wire::__Wire for ObjectCannedACLAuthenticatedRead {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ObjectCannedACLAwsExecRead {
}

impl crate::wire::__Wire for ObjectCannedACLAwsExecRead {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ObjectCannedACLBucketOwnerRead {
}

impl crate::wire::__Wire for ObjectCannedACLBucketOwnerRead {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ObjectCannedACLBucketOwnerFullControl {
}

impl crate::wire::__Wire for ObjectCannedACLBucketOwnerFullControl {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ObjectCannedACLUnknown {
    pub value: String,
}

impl crate::wire::__Wire for ObjectCannedACLUnknown {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.value, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            value: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChecksumAlgorithmCrc32 {
}

impl crate::wire::__Wire for ChecksumAlgorithmCrc32 {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChecksumAlgorithmCrc32C {
}

impl crate::wire::__Wire for ChecksumAlgorithmCrc32C {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChecksumAlgorithmSha1 {
}

impl crate::wire::__Wire for ChecksumAlgorithmSha1 {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChecksumAlgorithmSha256 {
}

impl crate::wire::__Wire for ChecksumAlgorithmSha256 {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChecksumAlgorithmCrc64Nvme {
}

impl crate::wire::__Wire for ChecksumAlgorithmCrc64Nvme {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChecksumAlgorithmSha512 {
}

impl crate::wire::__Wire for ChecksumAlgorithmSha512 {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChecksumAlgorithmMd5 {
}

impl crate::wire::__Wire for ChecksumAlgorithmMd5 {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChecksumAlgorithmXxhash64 {
}

impl crate::wire::__Wire for ChecksumAlgorithmXxhash64 {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChecksumAlgorithmXxhash3 {
}

impl crate::wire::__Wire for ChecksumAlgorithmXxhash3 {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChecksumAlgorithmXxhash128 {
}

impl crate::wire::__Wire for ChecksumAlgorithmXxhash128 {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChecksumAlgorithmUnknown {
    pub value: String,
}

impl crate::wire::__Wire for ChecksumAlgorithmUnknown {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.value, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            value: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ServerSideEncryptionAes256 {
}

impl crate::wire::__Wire for ServerSideEncryptionAes256 {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ServerSideEncryptionAwsFsx {
}

impl crate::wire::__Wire for ServerSideEncryptionAwsFsx {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ServerSideEncryptionAwsBackup {
}

impl crate::wire::__Wire for ServerSideEncryptionAwsBackup {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ServerSideEncryptionAwsKms {
}

impl crate::wire::__Wire for ServerSideEncryptionAwsKms {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ServerSideEncryptionAwsKmsDsse {
}

impl crate::wire::__Wire for ServerSideEncryptionAwsKmsDsse {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ServerSideEncryptionUnknown {
    pub value: String,
}

impl crate::wire::__Wire for ServerSideEncryptionUnknown {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.value, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            value: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StorageClassStandard {
}

impl crate::wire::__Wire for StorageClassStandard {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StorageClassReducedRedundancy {
}

impl crate::wire::__Wire for StorageClassReducedRedundancy {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StorageClassStandardIa {
}

impl crate::wire::__Wire for StorageClassStandardIa {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StorageClassOnezoneIa {
}

impl crate::wire::__Wire for StorageClassOnezoneIa {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StorageClassIntelligentTiering {
}

impl crate::wire::__Wire for StorageClassIntelligentTiering {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StorageClassGlacier {
}

impl crate::wire::__Wire for StorageClassGlacier {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StorageClassDeepArchive {
}

impl crate::wire::__Wire for StorageClassDeepArchive {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StorageClassOutposts {
}

impl crate::wire::__Wire for StorageClassOutposts {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StorageClassGlacierIr {
}

impl crate::wire::__Wire for StorageClassGlacierIr {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StorageClassSnow {
}

impl crate::wire::__Wire for StorageClassSnow {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StorageClassExpressOnezone {
}

impl crate::wire::__Wire for StorageClassExpressOnezone {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StorageClassFsxOpenzfs {
}

impl crate::wire::__Wire for StorageClassFsxOpenzfs {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StorageClassFsxOntap {
}

impl crate::wire::__Wire for StorageClassFsxOntap {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StorageClassAwsBackupWarm {
}

impl crate::wire::__Wire for StorageClassAwsBackupWarm {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StorageClassAwsBackupLowCostWarm {
}

impl crate::wire::__Wire for StorageClassAwsBackupLowCostWarm {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StorageClassUnknown {
    pub value: String,
}

impl crate::wire::__Wire for StorageClassUnknown {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.value, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            value: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RequestPayerRequester {
}

impl crate::wire::__Wire for RequestPayerRequester {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RequestPayerUnknown {
    pub value: String,
}

impl crate::wire::__Wire for RequestPayerUnknown {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.value, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            value: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ObjectLockModeGovernance {
}

impl crate::wire::__Wire for ObjectLockModeGovernance {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ObjectLockModeCompliance {
}

impl crate::wire::__Wire for ObjectLockModeCompliance {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ObjectLockModeUnknown {
    pub value: String,
}

impl crate::wire::__Wire for ObjectLockModeUnknown {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.value, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            value: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ObjectLockLegalHoldStatusOn {
}

impl crate::wire::__Wire for ObjectLockLegalHoldStatusOn {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ObjectLockLegalHoldStatusOff {
}

impl crate::wire::__Wire for ObjectLockLegalHoldStatusOff {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ObjectLockLegalHoldStatusUnknown {
    pub value: String,
}

impl crate::wire::__Wire for ObjectLockLegalHoldStatusUnknown {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.value, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            value: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ObjectLockEventHoldOn {
}

impl crate::wire::__Wire for ObjectLockEventHoldOn {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ObjectLockEventHoldOff {
}

impl crate::wire::__Wire for ObjectLockEventHoldOff {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ObjectLockEventHoldUnknown {
    pub value: String,
}

impl crate::wire::__Wire for ObjectLockEventHoldUnknown {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.value, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            value: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChecksumTypeComposite {
}

impl crate::wire::__Wire for ChecksumTypeComposite {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChecksumTypeFullObject {
}

impl crate::wire::__Wire for ChecksumTypeFullObject {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChecksumTypeUnknown {
    pub value: String,
}

impl crate::wire::__Wire for ChecksumTypeUnknown {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.value, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            value: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RequestChargedRequester {
}

impl crate::wire::__Wire for RequestChargedRequester {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RequestChargedUnknown {
    pub value: String,
}

impl crate::wire::__Wire for RequestChargedUnknown {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.value, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            value: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChecksumModeEnabled {
}

impl crate::wire::__Wire for ChecksumModeEnabled {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChecksumModeUnknown {
    pub value: String,
}

impl crate::wire::__Wire for ChecksumModeUnknown {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.value, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            value: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReplicationStatusComplete {
}

impl crate::wire::__Wire for ReplicationStatusComplete {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReplicationStatusPending {
}

impl crate::wire::__Wire for ReplicationStatusPending {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReplicationStatusFailed {
}

impl crate::wire::__Wire for ReplicationStatusFailed {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReplicationStatusReplica {
}

impl crate::wire::__Wire for ReplicationStatusReplica {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReplicationStatusCompleted {
}

impl crate::wire::__Wire for ReplicationStatusCompleted {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReplicationStatusUnknown {
    pub value: String,
}

impl crate::wire::__Wire for ReplicationStatusUnknown {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.value, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            value: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct IntelligentTieringAccessTierArchiveAccess {
}

impl crate::wire::__Wire for IntelligentTieringAccessTierArchiveAccess {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct IntelligentTieringAccessTierDeepArchiveAccess {
}

impl crate::wire::__Wire for IntelligentTieringAccessTierDeepArchiveAccess {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct IntelligentTieringAccessTierUnknown {
    pub value: String,
}

impl crate::wire::__Wire for IntelligentTieringAccessTierUnknown {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.value, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            value: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PutObjectInput {
    pub acl: Option<Union8<ObjectCannedACLPrivate, ObjectCannedACLPublicRead, ObjectCannedACLPublicReadWrite, ObjectCannedACLAuthenticatedRead, ObjectCannedACLAwsExecRead, ObjectCannedACLBucketOwnerRead, ObjectCannedACLBucketOwnerFullControl, ObjectCannedACLUnknown>>,
    pub body: InStream,
    pub bucket: String,
    pub cache_control: Option<String>,
    pub content_disposition: Option<String>,
    pub content_encoding: Option<String>,
    pub content_language: Option<String>,
    pub content_length: Option<i64>,
    pub content_md5: Option<String>,
    pub content_type: Option<String>,
    pub checksum_algorithm: Option<Union11<ChecksumAlgorithmCrc32, ChecksumAlgorithmCrc32C, ChecksumAlgorithmSha1, ChecksumAlgorithmSha256, ChecksumAlgorithmCrc64Nvme, ChecksumAlgorithmSha512, ChecksumAlgorithmMd5, ChecksumAlgorithmXxhash64, ChecksumAlgorithmXxhash3, ChecksumAlgorithmXxhash128, ChecksumAlgorithmUnknown>>,
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
    pub metadata: Option<SalvoMap<String, String>>,
    pub server_side_encryption: Option<Union6<ServerSideEncryptionAes256, ServerSideEncryptionAwsFsx, ServerSideEncryptionAwsBackup, ServerSideEncryptionAwsKms, ServerSideEncryptionAwsKmsDsse, ServerSideEncryptionUnknown>>,
    pub storage_class: Option<Union16<StorageClassStandard, StorageClassReducedRedundancy, StorageClassStandardIa, StorageClassOnezoneIa, StorageClassIntelligentTiering, StorageClassGlacier, StorageClassDeepArchive, StorageClassOutposts, StorageClassGlacierIr, StorageClassSnow, StorageClassExpressOnezone, StorageClassFsxOpenzfs, StorageClassFsxOntap, StorageClassAwsBackupWarm, StorageClassAwsBackupLowCostWarm, StorageClassUnknown>>,
    pub website_redirect_location: Option<String>,
    pub sse_customer_algorithm: Option<String>,
    pub sse_customer_key: Option<String>,
    pub sse_customer_key_md5: Option<String>,
    pub ssekms_key_id: Option<String>,
    pub ssekms_encryption_context: Option<String>,
    pub bucket_key_enabled: Option<bool>,
    pub request_payer: Option<Union2<RequestPayerRequester, RequestPayerUnknown>>,
    pub tagging: Option<String>,
    pub object_lock_mode: Option<Union3<ObjectLockModeGovernance, ObjectLockModeCompliance, ObjectLockModeUnknown>>,
    pub object_lock_retain_until_date: Option<Instant>,
    pub object_lock_legal_hold_status: Option<Union3<ObjectLockLegalHoldStatusOn, ObjectLockLegalHoldStatusOff, ObjectLockLegalHoldStatusUnknown>>,
    pub object_lock_event_hold: Option<Union3<ObjectLockEventHoldOn, ObjectLockEventHoldOff, ObjectLockEventHoldUnknown>>,
    pub object_lock_event_hold_duration_days: Option<i32>,
    pub object_lock_event_hold_duration_years: Option<i32>,
    pub expected_bucket_owner: Option<String>,
}

pub fn close__4(streams: &crate::stream::Streams, value: PutObjectInput) -> Union2<(), Checked<Union2<InvalidUtf8, StreamFailed>>> {
    return streams.close(value.body);
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
    pub checksum_type: Option<Union3<ChecksumTypeComposite, ChecksumTypeFullObject, ChecksumTypeUnknown>>,
    pub server_side_encryption: Option<Union6<ServerSideEncryptionAes256, ServerSideEncryptionAwsFsx, ServerSideEncryptionAwsBackup, ServerSideEncryptionAwsKms, ServerSideEncryptionAwsKmsDsse, ServerSideEncryptionUnknown>>,
    pub version_id: Option<String>,
    pub sse_customer_algorithm: Option<String>,
    pub sse_customer_key_md5: Option<String>,
    pub ssekms_key_id: Option<String>,
    pub ssekms_encryption_context: Option<String>,
    pub bucket_key_enabled: Option<bool>,
    pub size: Option<i64>,
    pub request_charged: Option<Union2<RequestChargedRequester, RequestChargedUnknown>>,
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
    pub if_modified_since: Option<Instant>,
    pub if_none_match: Option<String>,
    pub if_unmodified_since: Option<Instant>,
    pub key: String,
    pub range: Option<String>,
    pub response_cache_control: Option<String>,
    pub response_content_disposition: Option<String>,
    pub response_content_encoding: Option<String>,
    pub response_content_language: Option<String>,
    pub response_content_type: Option<String>,
    pub response_expires: Option<Instant>,
    pub version_id: Option<String>,
    pub sse_customer_algorithm: Option<String>,
    pub sse_customer_key: Option<String>,
    pub sse_customer_key_md5: Option<String>,
    pub request_payer: Option<Union2<RequestPayerRequester, RequestPayerUnknown>>,
    pub part_number: Option<i32>,
    pub expected_bucket_owner: Option<String>,
    pub checksum_mode: Option<Union2<ChecksumModeEnabled, ChecksumModeUnknown>>,
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
    pub body: InStream,
    pub delete_marker: Option<bool>,
    pub accept_ranges: Option<String>,
    pub expiration: Option<String>,
    pub restore: Option<String>,
    pub last_modified: Option<Instant>,
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
    pub checksum_type: Option<Union3<ChecksumTypeComposite, ChecksumTypeFullObject, ChecksumTypeUnknown>>,
    pub missing_meta: Option<i32>,
    pub version_id: Option<String>,
    pub cache_control: Option<String>,
    pub content_disposition: Option<String>,
    pub content_encoding: Option<String>,
    pub content_language: Option<String>,
    pub content_range: Option<String>,
    pub content_type: Option<String>,
    pub website_redirect_location: Option<String>,
    pub server_side_encryption: Option<Union6<ServerSideEncryptionAes256, ServerSideEncryptionAwsFsx, ServerSideEncryptionAwsBackup, ServerSideEncryptionAwsKms, ServerSideEncryptionAwsKmsDsse, ServerSideEncryptionUnknown>>,
    pub metadata: Option<SalvoMap<String, String>>,
    pub sse_customer_algorithm: Option<String>,
    pub sse_customer_key_md5: Option<String>,
    pub ssekms_key_id: Option<String>,
    pub bucket_key_enabled: Option<bool>,
    pub storage_class: Option<Union16<StorageClassStandard, StorageClassReducedRedundancy, StorageClassStandardIa, StorageClassOnezoneIa, StorageClassIntelligentTiering, StorageClassGlacier, StorageClassDeepArchive, StorageClassOutposts, StorageClassGlacierIr, StorageClassSnow, StorageClassExpressOnezone, StorageClassFsxOpenzfs, StorageClassFsxOntap, StorageClassAwsBackupWarm, StorageClassAwsBackupLowCostWarm, StorageClassUnknown>>,
    pub request_charged: Option<Union2<RequestChargedRequester, RequestChargedUnknown>>,
    pub replication_status: Option<Union6<ReplicationStatusComplete, ReplicationStatusPending, ReplicationStatusFailed, ReplicationStatusReplica, ReplicationStatusCompleted, ReplicationStatusUnknown>>,
    pub parts_count: Option<i32>,
    pub tag_count: Option<i32>,
    pub object_lock_mode: Option<Union3<ObjectLockModeGovernance, ObjectLockModeCompliance, ObjectLockModeUnknown>>,
    pub object_lock_retain_until_date: Option<Instant>,
    pub object_lock_legal_hold_status: Option<Union3<ObjectLockLegalHoldStatusOn, ObjectLockLegalHoldStatusOff, ObjectLockLegalHoldStatusUnknown>>,
    pub object_lock_event_hold: Option<Union3<ObjectLockEventHoldOn, ObjectLockEventHoldOff, ObjectLockEventHoldUnknown>>,
    pub object_lock_event_hold_duration_days: Option<i32>,
    pub object_lock_event_hold_duration_years: Option<i32>,
}

pub fn close__5(streams: &crate::stream::Streams, value: GetObjectOutput) -> Union2<(), Checked<Union2<InvalidUtf8, StreamFailed>>> {
    return streams.close(value.body);
}

#[derive(Clone, Debug, PartialEq)]
pub struct EncryptionTypeMismatch {
}

impl crate::wire::__Wire for EncryptionTypeMismatch {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct InvalidObjectState {
    pub storage_class: Option<Union16<StorageClassStandard, StorageClassReducedRedundancy, StorageClassStandardIa, StorageClassOnezoneIa, StorageClassIntelligentTiering, StorageClassGlacier, StorageClassDeepArchive, StorageClassOutposts, StorageClassGlacierIr, StorageClassSnow, StorageClassExpressOnezone, StorageClassFsxOpenzfs, StorageClassFsxOntap, StorageClassAwsBackupWarm, StorageClassAwsBackupLowCostWarm, StorageClassUnknown>>,
    pub access_tier: Option<Union3<IntelligentTieringAccessTierArchiveAccess, IntelligentTieringAccessTierDeepArchiveAccess, IntelligentTieringAccessTierUnknown>>,
}

impl crate::wire::__Wire for InvalidObjectState {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.storage_class, out);
        crate::wire::__Wire::__enc(&self.access_tier, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            storage_class: crate::wire::__Wire::__dec(r)?,
            access_tier: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct InvalidRequest {
}

impl crate::wire::__Wire for InvalidRequest {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct InvalidWriteOffset {
}

impl crate::wire::__Wire for InvalidWriteOffset {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct NoSuchKey {
}

impl crate::wire::__Wire for NoSuchKey {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TooManyParts {
}

impl crate::wire::__Wire for TooManyParts {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

pub trait __Stateless_S3: Send + Sync {
    fn put_object(&self, input: PutObjectInput, reply: crate::scheduler::SalvoReply);
    fn get_object(&self, input: GetObjectInput, reply: crate::scheduler::SalvoReply);
}

pub trait __Stateful_S3: Send {
    fn put_object(&mut self, input: PutObjectInput, reply: crate::scheduler::SalvoReply);
    fn get_object(&mut self, input: GetObjectInput, reply: crate::scheduler::SalvoReply);
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
    pub fn put_object(&self, input: PutObjectInput, reply: crate::scheduler::SalvoReply) {
        match &self.inner {
            __Inner_S3::Shared(h) => h.put_object(input, reply),
            __Inner_S3::Locked(h) => h.lock().unwrap().put_object(input, reply),
        }
    }
    pub fn get_object(&self, input: GetObjectInput, reply: crate::scheduler::SalvoReply) {
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
    recorded: Vec<String>,
    __dep_Streams: crate::stream::Streams,
}

impl FakeS3 {
    pub fn new(__dep_Streams: crate::stream::Streams) -> Self {
        Self {
            recorded: vec![],
            __dep_Streams,
        }
    }
}

impl crate::aws_s3::__Stateful_S3 for FakeS3 {

    fn put_object(&mut self, input: PutObjectInput, reply: crate::scheduler::SalvoReply) {
        self.recorded.push("put_object".to_string());
        let mut r#unsized = input.content_length.is_none();
        let mut closed = close__4(&self.__dep_Streams, input);
        if matches!(closed, Union2::U2(_)) {
            ignore(closed.u2().clone());
        }
        if r#unsized {
            crate::scheduler::salvo_reply_wire::<Union2<PutObjectOutput, Checked<Union7<EncryptionTypeMismatch, InvalidObjectState, InvalidRequest, InvalidWriteOffset, NoSuchKey, TooManyParts, AwsError>>>>(reply, Union2::<PutObjectOutput, Checked<Union7<EncryptionTypeMismatch, InvalidObjectState, InvalidRequest, InvalidWriteOffset, NoSuchKey, TooManyParts, AwsError>>>::U2(err(checked(Union7::<EncryptionTypeMismatch, InvalidObjectState, InvalidRequest, InvalidWriteOffset, NoSuchKey, TooManyParts, AwsError>::U7(AwsError { code: "MissingContentLength".to_string(), message: "S3 PutObject streams its body, so the input needs content_length: the body's length in bytes".to_string() })))));
            return;
        }
        crate::scheduler::salvo_reply_wire::<Union2<PutObjectOutput, Checked<Union7<EncryptionTypeMismatch, InvalidObjectState, InvalidRequest, InvalidWriteOffset, NoSuchKey, TooManyParts, AwsError>>>>(reply, Union2::<PutObjectOutput, Checked<Union7<EncryptionTypeMismatch, InvalidObjectState, InvalidRequest, InvalidWriteOffset, NoSuchKey, TooManyParts, AwsError>>>::U1(ok(PutObjectOutput { expiration: None, e_tag: None, checksum_crc32: None, checksum_crc32_c: None, checksum_crc64_nvme: None, checksum_sha1: None, checksum_sha256: None, checksum_sha512: None, checksum_md5: None, checksum_xxhash64: None, checksum_xxhash3: None, checksum_xxhash128: None, checksum_type: None, server_side_encryption: None, version_id: None, sse_customer_algorithm: None, sse_customer_key_md5: None, ssekms_key_id: None, ssekms_encryption_context: None, bucket_key_enabled: None, size: None, request_charged: None })));
    }

    fn get_object(&mut self, input: GetObjectInput, reply: crate::scheduler::SalvoReply) {
        self.recorded.push("get_object".to_string());
        (reply).send(Box::new(Union2::<GetObjectOutput, Checked<Union7<EncryptionTypeMismatch, InvalidObjectState, InvalidRequest, InvalidWriteOffset, NoSuchKey, TooManyParts, AwsError>>>::U1(ok(GetObjectOutput { body: self.__dep_Streams.from_bytes(vec![]), delete_marker: None, accept_ranges: None, expiration: None, restore: None, last_modified: None, content_length: None, e_tag: None, checksum_crc32: None, checksum_crc32_c: None, checksum_crc64_nvme: None, checksum_sha1: None, checksum_sha256: None, checksum_sha512: None, checksum_md5: None, checksum_xxhash64: None, checksum_xxhash3: None, checksum_xxhash128: None, checksum_type: None, missing_meta: None, version_id: None, cache_control: None, content_disposition: None, content_encoding: None, content_language: None, content_range: None, content_type: None, website_redirect_location: None, server_side_encryption: None, metadata: None, sse_customer_algorithm: None, sse_customer_key_md5: None, ssekms_key_id: None, bucket_key_enabled: None, storage_class: None, request_charged: None, replication_status: None, parts_count: None, tag_count: None, object_lock_mode: None, object_lock_retain_until_date: None, object_lock_legal_hold_status: None, object_lock_event_hold: None, object_lock_event_hold_duration_days: None, object_lock_event_hold_duration_years: None }))));
    }
}

impl crate::aws_s3::__Stateful_S3Calls for FakeS3 {

    fn calls(&mut self) -> Vec<String> {
        return self.recorded.clone();
    }
}
