// [time-clock] [platform-fn] `time`'s host code: the wall clock, in
// nanoseconds since the Unix epoch (ROADMAP §0j step 5; it was an intrinsic
// lowered to the runtime's `SalvoTime`). `java.time.Instant` carries seconds
// plus a nanosecond field, so this is the precise reading rather than
// `currentTimeMillis()` scaled up.
package salvo.platform.time

fun epochNanos(): Long {
    val now = java.time.Instant.now()
    return now.epochSecond * 1_000_000_000L + now.nano.toLong()
}
