
#[derive(Clone, Debug)]
pub struct Finished {
}

pub fn emitted<T: Clone + 'static>(value: T) -> T {
    return value;
}

pub fn finished() -> Finished {
    return Finished {  };
}
