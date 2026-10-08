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

pub fn range__Int_Int_Int(mut start: i32, mut end: i32, mut step: i32) -> crate::core_range::__Iter_range_Int_Int_Int {
    return crate::core_range::__Iter_range_Int_Int_Int { start: start, end: end, step: step, i: start };
}

pub fn next(__p: &mut crate::core_range::__Iter_range_Int_Int_Int) -> crate::unions::Union2<i32, crate::core_iterator::Finished> {
    let mut next: i32 = __p.i;
    return (match if ((__p.step) == (0i32)) {
        crate::unions::Union2::U1(crate::core_iterator::finished())
    } else if ((__p.step > 0i32) && (__p.i >= __p.end)) {
        crate::unions::Union2::U1(crate::core_iterator::finished())
    } else if ((__p.step < 0i32) && (__p.i <= __p.end)) {
        crate::unions::Union2::U1(crate::core_iterator::finished())
    } else {
        __p.i = i32::wrapping_add(__p.i, __p.step);
        crate::unions::Union2::U2(crate::core_iterator::emitted::<i32>(next))
    } { crate::unions::Union2::U1(__v) => crate::unions::Union2::U2(__v), crate::unions::Union2::U2(__v) => crate::unions::Union2::U1(__v), #[allow(unreachable_patterns)] _ => unreachable!("salvo: unreachable union arm") });
}

pub fn range__Int_Int(mut start: i32, mut end: i32) -> crate::core_range::__Iter_range_Int_Int_Int {
    let mut step: i32 = if (start < end) {
        1i32
    } else if (start > end) {
        i32::wrapping_neg(1i32)
    } else {
        0i32
    };
    return crate::core_range::range__Int_Int_Int(start, end, step);
}

pub fn range__Int(mut end: i32) -> crate::core_range::__Iter_range_Int_Int_Int {
    return crate::core_range::range__Int_Int(0i32, end);
}

pub fn InRange__Int_qualifies(mut n: i32, mut lo: i32, mut hi: i32) -> bool {
    return ((n >= lo) && (n <= hi));
}
