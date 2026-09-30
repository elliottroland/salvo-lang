package salvo.aws.s3

import salvo.*
import salvo.aws.*
import salvo.core.actor.*
import salvo.core.bytes.*
import salvo.core.checked.*
import salvo.core.list.*
import salvo.core.map.*
import salvo.core.result.*
import salvo.core.set.*
import salvo.core.sorted.*
import salvo.core.string.*
import salvo.stream.*
import salvo.time.*

object ObjectCannedACL {

    class Private

    class PublicRead

    class PublicReadWrite

    class AuthenticatedRead

    class AwsExecRead

    class BucketOwnerRead

    class BucketOwnerFullControl

    data class Unknown(
        val value: String,
    )
}

object __Codec_ObjectCannedACL_Private : salvo.WireCodec<ObjectCannedACL.Private> {
    override fun enc(v: ObjectCannedACL.Private, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): ObjectCannedACL.Private = ObjectCannedACL.Private()
}

object __Codec_ObjectCannedACL_PublicRead : salvo.WireCodec<ObjectCannedACL.PublicRead> {
    override fun enc(v: ObjectCannedACL.PublicRead, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): ObjectCannedACL.PublicRead = ObjectCannedACL.PublicRead()
}

object __Codec_ObjectCannedACL_PublicReadWrite : salvo.WireCodec<ObjectCannedACL.PublicReadWrite> {
    override fun enc(v: ObjectCannedACL.PublicReadWrite, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): ObjectCannedACL.PublicReadWrite = ObjectCannedACL.PublicReadWrite()
}

object __Codec_ObjectCannedACL_AuthenticatedRead : salvo.WireCodec<ObjectCannedACL.AuthenticatedRead> {
    override fun enc(v: ObjectCannedACL.AuthenticatedRead, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): ObjectCannedACL.AuthenticatedRead = ObjectCannedACL.AuthenticatedRead()
}

object __Codec_ObjectCannedACL_AwsExecRead : salvo.WireCodec<ObjectCannedACL.AwsExecRead> {
    override fun enc(v: ObjectCannedACL.AwsExecRead, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): ObjectCannedACL.AwsExecRead = ObjectCannedACL.AwsExecRead()
}

object __Codec_ObjectCannedACL_BucketOwnerRead : salvo.WireCodec<ObjectCannedACL.BucketOwnerRead> {
    override fun enc(v: ObjectCannedACL.BucketOwnerRead, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): ObjectCannedACL.BucketOwnerRead = ObjectCannedACL.BucketOwnerRead()
}

object __Codec_ObjectCannedACL_BucketOwnerFullControl : salvo.WireCodec<ObjectCannedACL.BucketOwnerFullControl> {
    override fun enc(v: ObjectCannedACL.BucketOwnerFullControl, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): ObjectCannedACL.BucketOwnerFullControl = ObjectCannedACL.BucketOwnerFullControl()
}

object __Codec_ObjectCannedACL_Unknown : salvo.WireCodec<ObjectCannedACL.Unknown> {
    override fun enc(v: ObjectCannedACL.Unknown, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.value, out)
    }
    override fun dec(inp: salvo.WireIn): ObjectCannedACL.Unknown = ObjectCannedACL.Unknown(salvo.StrCodec.dec(inp))
}

object ChecksumAlgorithm {

    class Crc32

    class Crc32C

    class Sha1

    class Sha256

    class Crc64Nvme

    class Sha512

    class Md5

    class Xxhash64

    class Xxhash3

    class Xxhash128

    data class Unknown(
        val value: String,
    )
}

object __Codec_ChecksumAlgorithm_Crc32 : salvo.WireCodec<ChecksumAlgorithm.Crc32> {
    override fun enc(v: ChecksumAlgorithm.Crc32, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): ChecksumAlgorithm.Crc32 = ChecksumAlgorithm.Crc32()
}

object __Codec_ChecksumAlgorithm_Crc32C : salvo.WireCodec<ChecksumAlgorithm.Crc32C> {
    override fun enc(v: ChecksumAlgorithm.Crc32C, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): ChecksumAlgorithm.Crc32C = ChecksumAlgorithm.Crc32C()
}

object __Codec_ChecksumAlgorithm_Sha1 : salvo.WireCodec<ChecksumAlgorithm.Sha1> {
    override fun enc(v: ChecksumAlgorithm.Sha1, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): ChecksumAlgorithm.Sha1 = ChecksumAlgorithm.Sha1()
}

object __Codec_ChecksumAlgorithm_Sha256 : salvo.WireCodec<ChecksumAlgorithm.Sha256> {
    override fun enc(v: ChecksumAlgorithm.Sha256, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): ChecksumAlgorithm.Sha256 = ChecksumAlgorithm.Sha256()
}

object __Codec_ChecksumAlgorithm_Crc64Nvme : salvo.WireCodec<ChecksumAlgorithm.Crc64Nvme> {
    override fun enc(v: ChecksumAlgorithm.Crc64Nvme, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): ChecksumAlgorithm.Crc64Nvme = ChecksumAlgorithm.Crc64Nvme()
}

object __Codec_ChecksumAlgorithm_Sha512 : salvo.WireCodec<ChecksumAlgorithm.Sha512> {
    override fun enc(v: ChecksumAlgorithm.Sha512, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): ChecksumAlgorithm.Sha512 = ChecksumAlgorithm.Sha512()
}

object __Codec_ChecksumAlgorithm_Md5 : salvo.WireCodec<ChecksumAlgorithm.Md5> {
    override fun enc(v: ChecksumAlgorithm.Md5, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): ChecksumAlgorithm.Md5 = ChecksumAlgorithm.Md5()
}

object __Codec_ChecksumAlgorithm_Xxhash64 : salvo.WireCodec<ChecksumAlgorithm.Xxhash64> {
    override fun enc(v: ChecksumAlgorithm.Xxhash64, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): ChecksumAlgorithm.Xxhash64 = ChecksumAlgorithm.Xxhash64()
}

object __Codec_ChecksumAlgorithm_Xxhash3 : salvo.WireCodec<ChecksumAlgorithm.Xxhash3> {
    override fun enc(v: ChecksumAlgorithm.Xxhash3, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): ChecksumAlgorithm.Xxhash3 = ChecksumAlgorithm.Xxhash3()
}

object __Codec_ChecksumAlgorithm_Xxhash128 : salvo.WireCodec<ChecksumAlgorithm.Xxhash128> {
    override fun enc(v: ChecksumAlgorithm.Xxhash128, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): ChecksumAlgorithm.Xxhash128 = ChecksumAlgorithm.Xxhash128()
}

object __Codec_ChecksumAlgorithm_Unknown : salvo.WireCodec<ChecksumAlgorithm.Unknown> {
    override fun enc(v: ChecksumAlgorithm.Unknown, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.value, out)
    }
    override fun dec(inp: salvo.WireIn): ChecksumAlgorithm.Unknown = ChecksumAlgorithm.Unknown(salvo.StrCodec.dec(inp))
}

object ServerSideEncryption {

    class Aes256

    class AwsFsx

    class AwsBackup

    class AwsKms

    class AwsKmsDsse

    data class Unknown(
        val value: String,
    )
}

object __Codec_ServerSideEncryption_Aes256 : salvo.WireCodec<ServerSideEncryption.Aes256> {
    override fun enc(v: ServerSideEncryption.Aes256, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): ServerSideEncryption.Aes256 = ServerSideEncryption.Aes256()
}

object __Codec_ServerSideEncryption_AwsFsx : salvo.WireCodec<ServerSideEncryption.AwsFsx> {
    override fun enc(v: ServerSideEncryption.AwsFsx, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): ServerSideEncryption.AwsFsx = ServerSideEncryption.AwsFsx()
}

object __Codec_ServerSideEncryption_AwsBackup : salvo.WireCodec<ServerSideEncryption.AwsBackup> {
    override fun enc(v: ServerSideEncryption.AwsBackup, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): ServerSideEncryption.AwsBackup = ServerSideEncryption.AwsBackup()
}

object __Codec_ServerSideEncryption_AwsKms : salvo.WireCodec<ServerSideEncryption.AwsKms> {
    override fun enc(v: ServerSideEncryption.AwsKms, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): ServerSideEncryption.AwsKms = ServerSideEncryption.AwsKms()
}

object __Codec_ServerSideEncryption_AwsKmsDsse : salvo.WireCodec<ServerSideEncryption.AwsKmsDsse> {
    override fun enc(v: ServerSideEncryption.AwsKmsDsse, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): ServerSideEncryption.AwsKmsDsse = ServerSideEncryption.AwsKmsDsse()
}

object __Codec_ServerSideEncryption_Unknown : salvo.WireCodec<ServerSideEncryption.Unknown> {
    override fun enc(v: ServerSideEncryption.Unknown, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.value, out)
    }
    override fun dec(inp: salvo.WireIn): ServerSideEncryption.Unknown = ServerSideEncryption.Unknown(salvo.StrCodec.dec(inp))
}

object StorageClass {

    class Standard

    class ReducedRedundancy

    class StandardIa

    class OnezoneIa

    class IntelligentTiering

    class Glacier

    class DeepArchive

    class Outposts

    class GlacierIr

    class Snow

    class ExpressOnezone

    class FsxOpenzfs

    class FsxOntap

    class AwsBackupWarm

    class AwsBackupLowCostWarm

    data class Unknown(
        val value: String,
    )
}

object __Codec_StorageClass_Standard : salvo.WireCodec<StorageClass.Standard> {
    override fun enc(v: StorageClass.Standard, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): StorageClass.Standard = StorageClass.Standard()
}

object __Codec_StorageClass_ReducedRedundancy : salvo.WireCodec<StorageClass.ReducedRedundancy> {
    override fun enc(v: StorageClass.ReducedRedundancy, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): StorageClass.ReducedRedundancy = StorageClass.ReducedRedundancy()
}

object __Codec_StorageClass_StandardIa : salvo.WireCodec<StorageClass.StandardIa> {
    override fun enc(v: StorageClass.StandardIa, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): StorageClass.StandardIa = StorageClass.StandardIa()
}

object __Codec_StorageClass_OnezoneIa : salvo.WireCodec<StorageClass.OnezoneIa> {
    override fun enc(v: StorageClass.OnezoneIa, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): StorageClass.OnezoneIa = StorageClass.OnezoneIa()
}

object __Codec_StorageClass_IntelligentTiering : salvo.WireCodec<StorageClass.IntelligentTiering> {
    override fun enc(v: StorageClass.IntelligentTiering, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): StorageClass.IntelligentTiering = StorageClass.IntelligentTiering()
}

object __Codec_StorageClass_Glacier : salvo.WireCodec<StorageClass.Glacier> {
    override fun enc(v: StorageClass.Glacier, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): StorageClass.Glacier = StorageClass.Glacier()
}

object __Codec_StorageClass_DeepArchive : salvo.WireCodec<StorageClass.DeepArchive> {
    override fun enc(v: StorageClass.DeepArchive, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): StorageClass.DeepArchive = StorageClass.DeepArchive()
}

object __Codec_StorageClass_Outposts : salvo.WireCodec<StorageClass.Outposts> {
    override fun enc(v: StorageClass.Outposts, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): StorageClass.Outposts = StorageClass.Outposts()
}

object __Codec_StorageClass_GlacierIr : salvo.WireCodec<StorageClass.GlacierIr> {
    override fun enc(v: StorageClass.GlacierIr, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): StorageClass.GlacierIr = StorageClass.GlacierIr()
}

object __Codec_StorageClass_Snow : salvo.WireCodec<StorageClass.Snow> {
    override fun enc(v: StorageClass.Snow, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): StorageClass.Snow = StorageClass.Snow()
}

object __Codec_StorageClass_ExpressOnezone : salvo.WireCodec<StorageClass.ExpressOnezone> {
    override fun enc(v: StorageClass.ExpressOnezone, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): StorageClass.ExpressOnezone = StorageClass.ExpressOnezone()
}

object __Codec_StorageClass_FsxOpenzfs : salvo.WireCodec<StorageClass.FsxOpenzfs> {
    override fun enc(v: StorageClass.FsxOpenzfs, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): StorageClass.FsxOpenzfs = StorageClass.FsxOpenzfs()
}

object __Codec_StorageClass_FsxOntap : salvo.WireCodec<StorageClass.FsxOntap> {
    override fun enc(v: StorageClass.FsxOntap, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): StorageClass.FsxOntap = StorageClass.FsxOntap()
}

object __Codec_StorageClass_AwsBackupWarm : salvo.WireCodec<StorageClass.AwsBackupWarm> {
    override fun enc(v: StorageClass.AwsBackupWarm, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): StorageClass.AwsBackupWarm = StorageClass.AwsBackupWarm()
}

object __Codec_StorageClass_AwsBackupLowCostWarm : salvo.WireCodec<StorageClass.AwsBackupLowCostWarm> {
    override fun enc(v: StorageClass.AwsBackupLowCostWarm, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): StorageClass.AwsBackupLowCostWarm = StorageClass.AwsBackupLowCostWarm()
}

object __Codec_StorageClass_Unknown : salvo.WireCodec<StorageClass.Unknown> {
    override fun enc(v: StorageClass.Unknown, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.value, out)
    }
    override fun dec(inp: salvo.WireIn): StorageClass.Unknown = StorageClass.Unknown(salvo.StrCodec.dec(inp))
}

object RequestPayer {

    class Requester

    data class Unknown(
        val value: String,
    )
}

object __Codec_RequestPayer_Requester : salvo.WireCodec<RequestPayer.Requester> {
    override fun enc(v: RequestPayer.Requester, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): RequestPayer.Requester = RequestPayer.Requester()
}

object __Codec_RequestPayer_Unknown : salvo.WireCodec<RequestPayer.Unknown> {
    override fun enc(v: RequestPayer.Unknown, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.value, out)
    }
    override fun dec(inp: salvo.WireIn): RequestPayer.Unknown = RequestPayer.Unknown(salvo.StrCodec.dec(inp))
}

object ObjectLockMode {

    class Governance

    class Compliance

    data class Unknown(
        val value: String,
    )
}

object __Codec_ObjectLockMode_Governance : salvo.WireCodec<ObjectLockMode.Governance> {
    override fun enc(v: ObjectLockMode.Governance, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): ObjectLockMode.Governance = ObjectLockMode.Governance()
}

object __Codec_ObjectLockMode_Compliance : salvo.WireCodec<ObjectLockMode.Compliance> {
    override fun enc(v: ObjectLockMode.Compliance, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): ObjectLockMode.Compliance = ObjectLockMode.Compliance()
}

object __Codec_ObjectLockMode_Unknown : salvo.WireCodec<ObjectLockMode.Unknown> {
    override fun enc(v: ObjectLockMode.Unknown, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.value, out)
    }
    override fun dec(inp: salvo.WireIn): ObjectLockMode.Unknown = ObjectLockMode.Unknown(salvo.StrCodec.dec(inp))
}

object ObjectLockLegalHoldStatus {

    class On

    class Off

    data class Unknown(
        val value: String,
    )
}

object __Codec_ObjectLockLegalHoldStatus_On : salvo.WireCodec<ObjectLockLegalHoldStatus.On> {
    override fun enc(v: ObjectLockLegalHoldStatus.On, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): ObjectLockLegalHoldStatus.On = ObjectLockLegalHoldStatus.On()
}

object __Codec_ObjectLockLegalHoldStatus_Off : salvo.WireCodec<ObjectLockLegalHoldStatus.Off> {
    override fun enc(v: ObjectLockLegalHoldStatus.Off, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): ObjectLockLegalHoldStatus.Off = ObjectLockLegalHoldStatus.Off()
}

object __Codec_ObjectLockLegalHoldStatus_Unknown : salvo.WireCodec<ObjectLockLegalHoldStatus.Unknown> {
    override fun enc(v: ObjectLockLegalHoldStatus.Unknown, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.value, out)
    }
    override fun dec(inp: salvo.WireIn): ObjectLockLegalHoldStatus.Unknown = ObjectLockLegalHoldStatus.Unknown(salvo.StrCodec.dec(inp))
}

object ObjectLockEventHold {

    class On

    class Off

    data class Unknown(
        val value: String,
    )
}

object __Codec_ObjectLockEventHold_On : salvo.WireCodec<ObjectLockEventHold.On> {
    override fun enc(v: ObjectLockEventHold.On, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): ObjectLockEventHold.On = ObjectLockEventHold.On()
}

object __Codec_ObjectLockEventHold_Off : salvo.WireCodec<ObjectLockEventHold.Off> {
    override fun enc(v: ObjectLockEventHold.Off, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): ObjectLockEventHold.Off = ObjectLockEventHold.Off()
}

object __Codec_ObjectLockEventHold_Unknown : salvo.WireCodec<ObjectLockEventHold.Unknown> {
    override fun enc(v: ObjectLockEventHold.Unknown, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.value, out)
    }
    override fun dec(inp: salvo.WireIn): ObjectLockEventHold.Unknown = ObjectLockEventHold.Unknown(salvo.StrCodec.dec(inp))
}

object ChecksumType {

    class Composite

    class FullObject

    data class Unknown(
        val value: String,
    )
}

object __Codec_ChecksumType_Composite : salvo.WireCodec<ChecksumType.Composite> {
    override fun enc(v: ChecksumType.Composite, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): ChecksumType.Composite = ChecksumType.Composite()
}

object __Codec_ChecksumType_FullObject : salvo.WireCodec<ChecksumType.FullObject> {
    override fun enc(v: ChecksumType.FullObject, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): ChecksumType.FullObject = ChecksumType.FullObject()
}

object __Codec_ChecksumType_Unknown : salvo.WireCodec<ChecksumType.Unknown> {
    override fun enc(v: ChecksumType.Unknown, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.value, out)
    }
    override fun dec(inp: salvo.WireIn): ChecksumType.Unknown = ChecksumType.Unknown(salvo.StrCodec.dec(inp))
}

object RequestCharged {

    class Requester

    data class Unknown(
        val value: String,
    )
}

object __Codec_RequestCharged_Requester : salvo.WireCodec<RequestCharged.Requester> {
    override fun enc(v: RequestCharged.Requester, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): RequestCharged.Requester = RequestCharged.Requester()
}

object __Codec_RequestCharged_Unknown : salvo.WireCodec<RequestCharged.Unknown> {
    override fun enc(v: RequestCharged.Unknown, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.value, out)
    }
    override fun dec(inp: salvo.WireIn): RequestCharged.Unknown = RequestCharged.Unknown(salvo.StrCodec.dec(inp))
}

object ChecksumMode {

    class Enabled

    data class Unknown(
        val value: String,
    )
}

object __Codec_ChecksumMode_Enabled : salvo.WireCodec<ChecksumMode.Enabled> {
    override fun enc(v: ChecksumMode.Enabled, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): ChecksumMode.Enabled = ChecksumMode.Enabled()
}

object __Codec_ChecksumMode_Unknown : salvo.WireCodec<ChecksumMode.Unknown> {
    override fun enc(v: ChecksumMode.Unknown, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.value, out)
    }
    override fun dec(inp: salvo.WireIn): ChecksumMode.Unknown = ChecksumMode.Unknown(salvo.StrCodec.dec(inp))
}

object ReplicationStatus {

    class Complete

    class Pending

    class Failed

    class Replica

    class Completed

    data class Unknown(
        val value: String,
    )
}

object __Codec_ReplicationStatus_Complete : salvo.WireCodec<ReplicationStatus.Complete> {
    override fun enc(v: ReplicationStatus.Complete, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): ReplicationStatus.Complete = ReplicationStatus.Complete()
}

object __Codec_ReplicationStatus_Pending : salvo.WireCodec<ReplicationStatus.Pending> {
    override fun enc(v: ReplicationStatus.Pending, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): ReplicationStatus.Pending = ReplicationStatus.Pending()
}

object __Codec_ReplicationStatus_Failed : salvo.WireCodec<ReplicationStatus.Failed> {
    override fun enc(v: ReplicationStatus.Failed, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): ReplicationStatus.Failed = ReplicationStatus.Failed()
}

object __Codec_ReplicationStatus_Replica : salvo.WireCodec<ReplicationStatus.Replica> {
    override fun enc(v: ReplicationStatus.Replica, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): ReplicationStatus.Replica = ReplicationStatus.Replica()
}

object __Codec_ReplicationStatus_Completed : salvo.WireCodec<ReplicationStatus.Completed> {
    override fun enc(v: ReplicationStatus.Completed, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): ReplicationStatus.Completed = ReplicationStatus.Completed()
}

object __Codec_ReplicationStatus_Unknown : salvo.WireCodec<ReplicationStatus.Unknown> {
    override fun enc(v: ReplicationStatus.Unknown, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.value, out)
    }
    override fun dec(inp: salvo.WireIn): ReplicationStatus.Unknown = ReplicationStatus.Unknown(salvo.StrCodec.dec(inp))
}

object IntelligentTieringAccessTier {

    class ArchiveAccess

    class DeepArchiveAccess

    data class Unknown(
        val value: String,
    )
}

object __Codec_IntelligentTieringAccessTier_ArchiveAccess : salvo.WireCodec<IntelligentTieringAccessTier.ArchiveAccess> {
    override fun enc(v: IntelligentTieringAccessTier.ArchiveAccess, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): IntelligentTieringAccessTier.ArchiveAccess = IntelligentTieringAccessTier.ArchiveAccess()
}

object __Codec_IntelligentTieringAccessTier_DeepArchiveAccess : salvo.WireCodec<IntelligentTieringAccessTier.DeepArchiveAccess> {
    override fun enc(v: IntelligentTieringAccessTier.DeepArchiveAccess, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): IntelligentTieringAccessTier.DeepArchiveAccess = IntelligentTieringAccessTier.DeepArchiveAccess()
}

object __Codec_IntelligentTieringAccessTier_Unknown : salvo.WireCodec<IntelligentTieringAccessTier.Unknown> {
    override fun enc(v: IntelligentTieringAccessTier.Unknown, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.value, out)
    }
    override fun dec(inp: salvo.WireIn): IntelligentTieringAccessTier.Unknown = IntelligentTieringAccessTier.Unknown(salvo.StrCodec.dec(inp))
}

data class PutObjectInput(
    val acl: Union8<ObjectCannedACL.Private, ObjectCannedACL.PublicRead, ObjectCannedACL.PublicReadWrite, ObjectCannedACL.AuthenticatedRead, ObjectCannedACL.AwsExecRead, ObjectCannedACL.BucketOwnerRead, ObjectCannedACL.BucketOwnerFullControl, ObjectCannedACL.Unknown>? = null,
    val body: InStream,
    val bucket: String,
    val cache_control: String? = null,
    val content_disposition: String? = null,
    val content_encoding: String? = null,
    val content_language: String? = null,
    val content_length: Long? = null,
    val content_md5: String? = null,
    val content_type: String? = null,
    val checksum_algorithm: Union11<ChecksumAlgorithm.Crc32, ChecksumAlgorithm.Crc32C, ChecksumAlgorithm.Sha1, ChecksumAlgorithm.Sha256, ChecksumAlgorithm.Crc64Nvme, ChecksumAlgorithm.Sha512, ChecksumAlgorithm.Md5, ChecksumAlgorithm.Xxhash64, ChecksumAlgorithm.Xxhash3, ChecksumAlgorithm.Xxhash128, ChecksumAlgorithm.Unknown>? = null,
    val checksum_crc32: String? = null,
    val checksum_crc32_c: String? = null,
    val checksum_crc64_nvme: String? = null,
    val checksum_sha1: String? = null,
    val checksum_sha256: String? = null,
    val checksum_sha512: String? = null,
    val checksum_md5: String? = null,
    val checksum_xxhash64: String? = null,
    val checksum_xxhash3: String? = null,
    val checksum_xxhash128: String? = null,
    val if_match: String? = null,
    val if_none_match: String? = null,
    val grant_full_control: String? = null,
    val grant_read: String? = null,
    val grant_read_acp: String? = null,
    val grant_write_acp: String? = null,
    val key: String,
    val write_offset_bytes: Long? = null,
    val metadata: Map<String, String>? = null,
    val server_side_encryption: Union6<ServerSideEncryption.Aes256, ServerSideEncryption.AwsFsx, ServerSideEncryption.AwsBackup, ServerSideEncryption.AwsKms, ServerSideEncryption.AwsKmsDsse, ServerSideEncryption.Unknown>? = null,
    val storage_class: Union16<StorageClass.Standard, StorageClass.ReducedRedundancy, StorageClass.StandardIa, StorageClass.OnezoneIa, StorageClass.IntelligentTiering, StorageClass.Glacier, StorageClass.DeepArchive, StorageClass.Outposts, StorageClass.GlacierIr, StorageClass.Snow, StorageClass.ExpressOnezone, StorageClass.FsxOpenzfs, StorageClass.FsxOntap, StorageClass.AwsBackupWarm, StorageClass.AwsBackupLowCostWarm, StorageClass.Unknown>? = null,
    val website_redirect_location: String? = null,
    val sse_customer_algorithm: String? = null,
    val sse_customer_key: String? = null,
    val sse_customer_key_md5: String? = null,
    val ssekms_key_id: String? = null,
    val ssekms_encryption_context: String? = null,
    val bucket_key_enabled: Boolean? = null,
    val request_payer: Union2<RequestPayer.Requester, RequestPayer.Unknown>? = null,
    val tagging: String? = null,
    val object_lock_mode: Union3<ObjectLockMode.Governance, ObjectLockMode.Compliance, ObjectLockMode.Unknown>? = null,
    val object_lock_retain_until_date: Instant? = null,
    val object_lock_legal_hold_status: Union3<ObjectLockLegalHoldStatus.On, ObjectLockLegalHoldStatus.Off, ObjectLockLegalHoldStatus.Unknown>? = null,
    val object_lock_event_hold: Union3<ObjectLockEventHold.On, ObjectLockEventHold.Off, ObjectLockEventHold.Unknown>? = null,
    val object_lock_event_hold_duration_days: Int? = null,
    val object_lock_event_hold_duration_years: Int? = null,
    val expected_bucket_owner: String? = null,
)

fun close__4(streams: Streams, value: PutObjectInput): Union2<Unit, Checked<Union2<InvalidUtf8, StreamFailed>>> {
    return streams.close(value.body)
}

data class PutObjectOutput(
    val expiration: String? = null,
    val e_tag: String? = null,
    val checksum_crc32: String? = null,
    val checksum_crc32_c: String? = null,
    val checksum_crc64_nvme: String? = null,
    val checksum_sha1: String? = null,
    val checksum_sha256: String? = null,
    val checksum_sha512: String? = null,
    val checksum_md5: String? = null,
    val checksum_xxhash64: String? = null,
    val checksum_xxhash3: String? = null,
    val checksum_xxhash128: String? = null,
    val checksum_type: Union3<ChecksumType.Composite, ChecksumType.FullObject, ChecksumType.Unknown>? = null,
    val server_side_encryption: Union6<ServerSideEncryption.Aes256, ServerSideEncryption.AwsFsx, ServerSideEncryption.AwsBackup, ServerSideEncryption.AwsKms, ServerSideEncryption.AwsKmsDsse, ServerSideEncryption.Unknown>? = null,
    val version_id: String? = null,
    val sse_customer_algorithm: String? = null,
    val sse_customer_key_md5: String? = null,
    val ssekms_key_id: String? = null,
    val ssekms_encryption_context: String? = null,
    val bucket_key_enabled: Boolean? = null,
    val size: Long? = null,
    val request_charged: Union2<RequestCharged.Requester, RequestCharged.Unknown>? = null,
)

object __Codec_PutObjectOutput : salvo.WireCodec<PutObjectOutput> {
    override fun enc(v: PutObjectOutput, out: salvo.WireOut) {
        salvo.OptCodec(salvo.StrCodec).enc(v.expiration, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.e_tag, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.checksum_crc32, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.checksum_crc32_c, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.checksum_crc64_nvme, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.checksum_sha1, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.checksum_sha256, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.checksum_sha512, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.checksum_md5, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.checksum_xxhash64, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.checksum_xxhash3, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.checksum_xxhash128, out)
        salvo.OptCodec(salvo.Union3Codec(__Codec_ChecksumType_Composite, __Codec_ChecksumType_FullObject, __Codec_ChecksumType_Unknown)).enc(v.checksum_type, out)
        salvo.OptCodec(salvo.Union6Codec(__Codec_ServerSideEncryption_Aes256, __Codec_ServerSideEncryption_AwsFsx, __Codec_ServerSideEncryption_AwsBackup, __Codec_ServerSideEncryption_AwsKms, __Codec_ServerSideEncryption_AwsKmsDsse, __Codec_ServerSideEncryption_Unknown)).enc(v.server_side_encryption, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.version_id, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.sse_customer_algorithm, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.sse_customer_key_md5, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.ssekms_key_id, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.ssekms_encryption_context, out)
        salvo.OptCodec(salvo.BoolCodec).enc(v.bucket_key_enabled, out)
        salvo.OptCodec(salvo.LongCodec).enc(v.size, out)
        salvo.OptCodec(salvo.Union2Codec(__Codec_RequestCharged_Requester, __Codec_RequestCharged_Unknown)).enc(v.request_charged, out)
    }
    override fun dec(inp: salvo.WireIn): PutObjectOutput = PutObjectOutput(salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.Union3Codec(__Codec_ChecksumType_Composite, __Codec_ChecksumType_FullObject, __Codec_ChecksumType_Unknown)).dec(inp), salvo.OptCodec(salvo.Union6Codec(__Codec_ServerSideEncryption_Aes256, __Codec_ServerSideEncryption_AwsFsx, __Codec_ServerSideEncryption_AwsBackup, __Codec_ServerSideEncryption_AwsKms, __Codec_ServerSideEncryption_AwsKmsDsse, __Codec_ServerSideEncryption_Unknown)).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.BoolCodec).dec(inp), salvo.OptCodec(salvo.LongCodec).dec(inp), salvo.OptCodec(salvo.Union2Codec(__Codec_RequestCharged_Requester, __Codec_RequestCharged_Unknown)).dec(inp))
}

data class GetObjectInput(
    val bucket: String,
    val if_match: String? = null,
    val if_modified_since: Instant? = null,
    val if_none_match: String? = null,
    val if_unmodified_since: Instant? = null,
    val key: String,
    val range: String? = null,
    val response_cache_control: String? = null,
    val response_content_disposition: String? = null,
    val response_content_encoding: String? = null,
    val response_content_language: String? = null,
    val response_content_type: String? = null,
    val response_expires: Instant? = null,
    val version_id: String? = null,
    val sse_customer_algorithm: String? = null,
    val sse_customer_key: String? = null,
    val sse_customer_key_md5: String? = null,
    val request_payer: Union2<RequestPayer.Requester, RequestPayer.Unknown>? = null,
    val part_number: Int? = null,
    val expected_bucket_owner: String? = null,
    val checksum_mode: Union2<ChecksumMode.Enabled, ChecksumMode.Unknown>? = null,
)

object __Codec_GetObjectInput : salvo.WireCodec<GetObjectInput> {
    override fun enc(v: GetObjectInput, out: salvo.WireOut) {
        salvo.StrCodec.enc(v.bucket, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.if_match, out)
        salvo.OptCodec(__Codec_Instant).enc(v.if_modified_since, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.if_none_match, out)
        salvo.OptCodec(__Codec_Instant).enc(v.if_unmodified_since, out)
        salvo.StrCodec.enc(v.key, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.range, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.response_cache_control, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.response_content_disposition, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.response_content_encoding, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.response_content_language, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.response_content_type, out)
        salvo.OptCodec(__Codec_Instant).enc(v.response_expires, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.version_id, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.sse_customer_algorithm, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.sse_customer_key, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.sse_customer_key_md5, out)
        salvo.OptCodec(salvo.Union2Codec(__Codec_RequestPayer_Requester, __Codec_RequestPayer_Unknown)).enc(v.request_payer, out)
        salvo.OptCodec(salvo.IntCodec).enc(v.part_number, out)
        salvo.OptCodec(salvo.StrCodec).enc(v.expected_bucket_owner, out)
        salvo.OptCodec(salvo.Union2Codec(__Codec_ChecksumMode_Enabled, __Codec_ChecksumMode_Unknown)).enc(v.checksum_mode, out)
    }
    override fun dec(inp: salvo.WireIn): GetObjectInput = GetObjectInput(salvo.StrCodec.dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(__Codec_Instant).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(__Codec_Instant).dec(inp), salvo.StrCodec.dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(__Codec_Instant).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.Union2Codec(__Codec_RequestPayer_Requester, __Codec_RequestPayer_Unknown)).dec(inp), salvo.OptCodec(salvo.IntCodec).dec(inp), salvo.OptCodec(salvo.StrCodec).dec(inp), salvo.OptCodec(salvo.Union2Codec(__Codec_ChecksumMode_Enabled, __Codec_ChecksumMode_Unknown)).dec(inp))
}

data class GetObjectOutput(
    val body: InStream,
    val delete_marker: Boolean? = null,
    val accept_ranges: String? = null,
    val expiration: String? = null,
    val restore: String? = null,
    val last_modified: Instant? = null,
    val content_length: Long? = null,
    val e_tag: String? = null,
    val checksum_crc32: String? = null,
    val checksum_crc32_c: String? = null,
    val checksum_crc64_nvme: String? = null,
    val checksum_sha1: String? = null,
    val checksum_sha256: String? = null,
    val checksum_sha512: String? = null,
    val checksum_md5: String? = null,
    val checksum_xxhash64: String? = null,
    val checksum_xxhash3: String? = null,
    val checksum_xxhash128: String? = null,
    val checksum_type: Union3<ChecksumType.Composite, ChecksumType.FullObject, ChecksumType.Unknown>? = null,
    val missing_meta: Int? = null,
    val version_id: String? = null,
    val cache_control: String? = null,
    val content_disposition: String? = null,
    val content_encoding: String? = null,
    val content_language: String? = null,
    val content_range: String? = null,
    val content_type: String? = null,
    val website_redirect_location: String? = null,
    val server_side_encryption: Union6<ServerSideEncryption.Aes256, ServerSideEncryption.AwsFsx, ServerSideEncryption.AwsBackup, ServerSideEncryption.AwsKms, ServerSideEncryption.AwsKmsDsse, ServerSideEncryption.Unknown>? = null,
    val metadata: Map<String, String>? = null,
    val sse_customer_algorithm: String? = null,
    val sse_customer_key_md5: String? = null,
    val ssekms_key_id: String? = null,
    val bucket_key_enabled: Boolean? = null,
    val storage_class: Union16<StorageClass.Standard, StorageClass.ReducedRedundancy, StorageClass.StandardIa, StorageClass.OnezoneIa, StorageClass.IntelligentTiering, StorageClass.Glacier, StorageClass.DeepArchive, StorageClass.Outposts, StorageClass.GlacierIr, StorageClass.Snow, StorageClass.ExpressOnezone, StorageClass.FsxOpenzfs, StorageClass.FsxOntap, StorageClass.AwsBackupWarm, StorageClass.AwsBackupLowCostWarm, StorageClass.Unknown>? = null,
    val request_charged: Union2<RequestCharged.Requester, RequestCharged.Unknown>? = null,
    val replication_status: Union6<ReplicationStatus.Complete, ReplicationStatus.Pending, ReplicationStatus.Failed, ReplicationStatus.Replica, ReplicationStatus.Completed, ReplicationStatus.Unknown>? = null,
    val parts_count: Int? = null,
    val tag_count: Int? = null,
    val object_lock_mode: Union3<ObjectLockMode.Governance, ObjectLockMode.Compliance, ObjectLockMode.Unknown>? = null,
    val object_lock_retain_until_date: Instant? = null,
    val object_lock_legal_hold_status: Union3<ObjectLockLegalHoldStatus.On, ObjectLockLegalHoldStatus.Off, ObjectLockLegalHoldStatus.Unknown>? = null,
    val object_lock_event_hold: Union3<ObjectLockEventHold.On, ObjectLockEventHold.Off, ObjectLockEventHold.Unknown>? = null,
    val object_lock_event_hold_duration_days: Int? = null,
    val object_lock_event_hold_duration_years: Int? = null,
)

fun close__5(streams: Streams, value: GetObjectOutput): Union2<Unit, Checked<Union2<InvalidUtf8, StreamFailed>>> {
    return streams.close(value.body)
}

class EncryptionTypeMismatch

object __Codec_EncryptionTypeMismatch : salvo.WireCodec<EncryptionTypeMismatch> {
    override fun enc(v: EncryptionTypeMismatch, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): EncryptionTypeMismatch = EncryptionTypeMismatch()
}

data class InvalidObjectState(
    val storage_class: Union16<StorageClass.Standard, StorageClass.ReducedRedundancy, StorageClass.StandardIa, StorageClass.OnezoneIa, StorageClass.IntelligentTiering, StorageClass.Glacier, StorageClass.DeepArchive, StorageClass.Outposts, StorageClass.GlacierIr, StorageClass.Snow, StorageClass.ExpressOnezone, StorageClass.FsxOpenzfs, StorageClass.FsxOntap, StorageClass.AwsBackupWarm, StorageClass.AwsBackupLowCostWarm, StorageClass.Unknown>? = null,
    val access_tier: Union3<IntelligentTieringAccessTier.ArchiveAccess, IntelligentTieringAccessTier.DeepArchiveAccess, IntelligentTieringAccessTier.Unknown>? = null,
)

object __Codec_InvalidObjectState : salvo.WireCodec<InvalidObjectState> {
    override fun enc(v: InvalidObjectState, out: salvo.WireOut) {
        salvo.OptCodec(salvo.Union16Codec(__Codec_StorageClass_Standard, __Codec_StorageClass_ReducedRedundancy, __Codec_StorageClass_StandardIa, __Codec_StorageClass_OnezoneIa, __Codec_StorageClass_IntelligentTiering, __Codec_StorageClass_Glacier, __Codec_StorageClass_DeepArchive, __Codec_StorageClass_Outposts, __Codec_StorageClass_GlacierIr, __Codec_StorageClass_Snow, __Codec_StorageClass_ExpressOnezone, __Codec_StorageClass_FsxOpenzfs, __Codec_StorageClass_FsxOntap, __Codec_StorageClass_AwsBackupWarm, __Codec_StorageClass_AwsBackupLowCostWarm, __Codec_StorageClass_Unknown)).enc(v.storage_class, out)
        salvo.OptCodec(salvo.Union3Codec(__Codec_IntelligentTieringAccessTier_ArchiveAccess, __Codec_IntelligentTieringAccessTier_DeepArchiveAccess, __Codec_IntelligentTieringAccessTier_Unknown)).enc(v.access_tier, out)
    }
    override fun dec(inp: salvo.WireIn): InvalidObjectState = InvalidObjectState(salvo.OptCodec(salvo.Union16Codec(__Codec_StorageClass_Standard, __Codec_StorageClass_ReducedRedundancy, __Codec_StorageClass_StandardIa, __Codec_StorageClass_OnezoneIa, __Codec_StorageClass_IntelligentTiering, __Codec_StorageClass_Glacier, __Codec_StorageClass_DeepArchive, __Codec_StorageClass_Outposts, __Codec_StorageClass_GlacierIr, __Codec_StorageClass_Snow, __Codec_StorageClass_ExpressOnezone, __Codec_StorageClass_FsxOpenzfs, __Codec_StorageClass_FsxOntap, __Codec_StorageClass_AwsBackupWarm, __Codec_StorageClass_AwsBackupLowCostWarm, __Codec_StorageClass_Unknown)).dec(inp), salvo.OptCodec(salvo.Union3Codec(__Codec_IntelligentTieringAccessTier_ArchiveAccess, __Codec_IntelligentTieringAccessTier_DeepArchiveAccess, __Codec_IntelligentTieringAccessTier_Unknown)).dec(inp))
}

class InvalidRequest

object __Codec_InvalidRequest : salvo.WireCodec<InvalidRequest> {
    override fun enc(v: InvalidRequest, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): InvalidRequest = InvalidRequest()
}

class InvalidWriteOffset

object __Codec_InvalidWriteOffset : salvo.WireCodec<InvalidWriteOffset> {
    override fun enc(v: InvalidWriteOffset, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): InvalidWriteOffset = InvalidWriteOffset()
}

class NoSuchKey

object __Codec_NoSuchKey : salvo.WireCodec<NoSuchKey> {
    override fun enc(v: NoSuchKey, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): NoSuchKey = NoSuchKey()
}

class TooManyParts

object __Codec_TooManyParts : salvo.WireCodec<TooManyParts> {
    override fun enc(v: TooManyParts, out: salvo.WireOut) {
    }
    override fun dec(inp: salvo.WireIn): TooManyParts = TooManyParts()
}

interface S3 {
    fun put_object(input: PutObjectInput, reply: salvo.SalvoReply)
    fun get_object(input: GetObjectInput, reply: salvo.SalvoReply)
}

class __Mon_S3(private val inner: S3) : S3 {
    override fun put_object(input: PutObjectInput, reply: salvo.SalvoReply) =
        synchronized(inner) { inner.put_object(input, reply) }
    override fun get_object(input: GetObjectInput, reply: salvo.SalvoReply) =
        synchronized(inner) { inner.get_object(input, reply) }
}

interface S3Calls {
    fun calls(): List<String>
}

class __Mon_S3Calls(private val inner: S3Calls) : S3Calls {
    override fun calls(): List<String> =
        synchronized(inner) { inner.calls() }
}

class FakeS3(private val __dep_Streams: Streams) : S3, S3Calls {
    private var recorded: MutableList<String> = mutableListOf<String>()

    @Suppress("UNCHECKED_CAST", "USELESS_CAST")
    override fun put_object(input: PutObjectInput, reply: salvo.SalvoReply) {
        recorded.add("put_object")
        val unsized = input.content_length == null
        val closed = close__4(__dep_Streams, input)
        if (closed is U2_2<*, *>) {
            ignore((closed.value as Checked<Union2<InvalidUtf8, StreamFailed>>))
        }
        if (unsized) {
            salvo.SalvoSched.replyWire(reply, U2_2<PutObjectOutput, Checked<Union7<EncryptionTypeMismatch, InvalidObjectState, InvalidRequest, InvalidWriteOffset, NoSuchKey, TooManyParts, AwsError>>>(err(checked<Union7<EncryptionTypeMismatch, InvalidObjectState, InvalidRequest, InvalidWriteOffset, NoSuchKey, TooManyParts, AwsError>>(U7_7<EncryptionTypeMismatch, InvalidObjectState, InvalidRequest, InvalidWriteOffset, NoSuchKey, TooManyParts, AwsError>(AwsError(code = "MissingContentLength", message = "S3 PutObject streams its body, so the input needs content_length: the body's length in bytes"))))), salvo.Union2Codec(__Codec_PutObjectOutput, __Codec_Checked(salvo.Union7Codec(__Codec_EncryptionTypeMismatch, __Codec_InvalidObjectState, __Codec_InvalidRequest, __Codec_InvalidWriteOffset, __Codec_NoSuchKey, __Codec_TooManyParts, __Codec_AwsError))))
            return
        }
        salvo.SalvoSched.replyWire(reply, U2_1<PutObjectOutput, Checked<Union7<EncryptionTypeMismatch, InvalidObjectState, InvalidRequest, InvalidWriteOffset, NoSuchKey, TooManyParts, AwsError>>>(ok(PutObjectOutput())), salvo.Union2Codec(__Codec_PutObjectOutput, __Codec_Checked(salvo.Union7Codec(__Codec_EncryptionTypeMismatch, __Codec_InvalidObjectState, __Codec_InvalidRequest, __Codec_InvalidWriteOffset, __Codec_NoSuchKey, __Codec_TooManyParts, __Codec_AwsError))))
    }

    override fun get_object(input: GetObjectInput, reply: salvo.SalvoReply) {
        recorded.add("get_object")
        reply.send(U2_1<GetObjectOutput, Checked<Union7<EncryptionTypeMismatch, InvalidObjectState, InvalidRequest, InvalidWriteOffset, NoSuchKey, TooManyParts, AwsError>>>(ok(GetObjectOutput(body = __dep_Streams.from_bytes(salvo.SalvoBytes.of(arrayOf<UByte>()))))))
    }

    override fun calls(): List<String> {
        return recorded.toMutableList()
    }
}
