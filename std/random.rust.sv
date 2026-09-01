// Rust defines for random [backend-define-handler].
//
// No external crates: a splitmix64-style hash of the current time. Not
// cryptographic — good enough for a default handler; seedable and
// reproducible generators belong in dedicated handlers.

define handler DefaultRandom of Random {
    define fn random() -> Float {
        inline: ``
        {
            let mut x = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(0x9E3779B97F4A7C15);
            x = x.wrapping_add(0x9E3779B97F4A7C15);
            x = (x ^ (x >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
            x = (x ^ (x >> 27)).wrapping_mul(0x94D049BB133111EB);
            x ^= x >> 31;
            (((x >> 11) as f64) / ((1u64 << 53) as f64)) as f32
        }
        ``
    }
}
