
pub fn mix_hash(seed: i64, value: i64) -> i64 {
    return i64::wrapping_add(i64::wrapping_mul(seed, 31i64), value);
}
