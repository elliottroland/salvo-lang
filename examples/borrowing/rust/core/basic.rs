
pub fn to_str__TupleAB<A: Clone, B: Clone>(value: &(A, B), to_str: &mut dyn FnMut(&A) -> String, to_str__1: &mut dyn FnMut(&B) -> String) -> String {
    return format!("({}, {})", to_str(&value.0), to_str__1(&value.1));
}

pub fn to_str__TupleABC<A: Clone, B: Clone, C: Clone>(value: &(A, B, C), to_str: &mut dyn FnMut(&A) -> String, to_str__1: &mut dyn FnMut(&B) -> String, to_str__2: &mut dyn FnMut(&C) -> String) -> String {
    return format!("({}, {}, {})", to_str(&value.0), to_str__1(&value.1), to_str__2(&value.2));
}
