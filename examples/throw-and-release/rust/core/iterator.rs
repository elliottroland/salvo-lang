
#[derive(Clone, Debug)]
pub struct Finished {
}

pub fn emitted<T: Clone>(value: T) -> T {
    return value;
}

pub fn finished() -> Finished {
    return Finished {  };
}
