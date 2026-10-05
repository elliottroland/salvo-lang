use crate::unions::*;
use crate::core_iterator::Finished;
use crate::core_iterator::emitted;
use crate::core_iterator::finished;

#[derive(Clone, Debug, PartialEq)]
pub struct __Iter_range_Int_Int_Int {
    pub start: i32,
    pub end: i32,
    pub step: i32,
    pub i: i32,
}

impl crate::wire::__Wire for __Iter_range_Int_Int_Int {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.start, out);
        crate::wire::__Wire::__enc(&self.end, out);
        crate::wire::__Wire::__enc(&self.step, out);
        crate::wire::__Wire::__enc(&self.i, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            start: crate::wire::__Wire::__dec(r)?,
            end: crate::wire::__Wire::__dec(r)?,
            step: crate::wire::__Wire::__dec(r)?,
            i: crate::wire::__Wire::__dec(r)?,
        })
    }
}

pub fn range__Int_Int_Int(start: i32, end: i32, step: i32) -> __Iter_range_Int_Int_Int {
    return __Iter_range_Int_Int_Int { start: start, end: end, step: step, i: start };
}

pub fn next(__p: &mut __Iter_range_Int_Int_Int) -> Union2<i32, Finished> {
    let mut next = __p.i;
    return (match if __p.step == 0 {
        Union2::<Finished, i32>::U1(finished())
    } else if __p.step > 0 && __p.i >= __p.end {
        Union2::<Finished, i32>::U1(finished())
    } else if __p.step < 0 && __p.i <= __p.end {
        Union2::<Finished, i32>::U1(finished())
    } else {
        __p.i = __p.i + __p.step;
        Union2::<Finished, i32>::U2(emitted(next))
    } { Union2::U1(__v) => Union2::<i32, Finished>::U2(__v), Union2::U2(__v) => Union2::<i32, Finished>::U1(__v), });
}

pub fn range__Int_Int(start: i32, end: i32) -> __Iter_range_Int_Int_Int {
    let mut step = if start < end {
        1
    } else if start > end {
        -1
    } else {
        0
    };
    return range__Int_Int_Int(start, end, step);
}

pub fn range__Int(end: i32) -> __Iter_range_Int_Int_Int {
    return range__Int_Int(0, end);
}

pub fn InRange__Int_qualifies(n: i32, lo: i32, hi: i32) -> bool {
    return n >= lo && n <= hi;
}
