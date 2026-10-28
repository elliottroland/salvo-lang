
pub fn hash__List<T: Clone>(value: &Vec<T>, hash: &mut dyn FnMut(&T) -> i64) -> i64 {
    let mut h: i64 = 7i64;
    for mut x in value.iter() {
        h = crate::std_core_compare::mix_hash(h, hash(x));
    }
    return h;
}

pub fn hash__TupleAB<A: Clone, B: Clone>(value: &(A, B), hash: &mut dyn FnMut(&A) -> i64, hash__1: &mut dyn FnMut(&B) -> i64) -> i64 {
    return crate::std_core_compare::mix_hash(crate::std_core_compare::mix_hash(7i64, hash(&value.0)), hash__1(&value.1));
}

pub fn hash__TupleABC<A: Clone, B: Clone, C: Clone>(value: &(A, B, C), hash: &mut dyn FnMut(&A) -> i64, hash__1: &mut dyn FnMut(&B) -> i64, hash__2: &mut dyn FnMut(&C) -> i64) -> i64 {
    return crate::std_core_compare::mix_hash(crate::std_core_compare::mix_hash(crate::std_core_compare::mix_hash(7i64, hash(&value.0)), hash__1(&value.1)), hash__2(&value.2));
}

pub fn mix_hash(mut seed: i64, mut value: i64) -> i64 {
    return i64::wrapping_add(i64::wrapping_mul(seed, 31i64), value);
}
