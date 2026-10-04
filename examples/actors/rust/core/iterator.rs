
#[derive(Clone, Debug, PartialEq)]
pub struct Finished {
}

impl crate::wire::__Wire for Finished {
    fn __enc(&self, out: &mut Vec<u8>) {
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
        })
    }
}

pub fn emitted<T>(value: T) -> T {
    return value;
}

pub fn finished() -> Finished {
    return Finished {  };
}
