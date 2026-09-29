// Streams: the byte streams every producer in a program hands out — files,
// network bodies, buffers — and, as the module grows, the one effect that reads
// and writes all of them.
//
// Not part of `core`, so it arrives by asking: `import stream`.

// [stream-handle] A handle for a new stream, unique in this process: every
// stream table — the host's and the in-memory ones alike — draws from this one
// counter, so a handle handed to the wrong table is *unknown* there, never
// another stream's. Mixing providers is still a mistake, but it can no longer
// silently close or read somebody else's stream.
export intrinsic fn fresh_handle() [] -> Long
