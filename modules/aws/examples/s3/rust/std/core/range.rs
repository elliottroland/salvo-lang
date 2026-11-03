use crate::std_core_iterator::Finished;
use crate::std_core_iterator::emitted;
use crate::std_core_iterator::finished;


#[derive(Clone, Debug, PartialEq)]
pub struct RangeIter {
    pub end: i32,
    pub step: i32,
    pub i: i32,
}

impl crate::wire::__Wire for RangeIter {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.end, out);
        crate::wire::__Wire::__enc(&self.step, out);
        crate::wire::__Wire::__enc(&self.i, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            end: crate::wire::__Wire::__dec(r)?,
            step: crate::wire::__Wire::__dec(r)?,
            i: crate::wire::__Wire::__dec(r)?,
        })
    }
}

pub fn range__Int_Int_Int(mut start: i32, mut end: i32, mut step: i32) -> crate::std_core_range::RangeIter {
    return crate::std_core_range::RangeIter { end: end, step: step, i: start };
}

pub fn next(p: &mut crate::std_core_range::RangeIter) -> crate::unions::Union2<i32, crate::std_core_iterator::Finished> {
    let mut value: i32 = p.i;
    if (((p.step == 0i32) || ((p.step > 0i32) && (p.i >= p.end))) || ((p.step < 0i32) && (p.i <= p.end))) {
        return crate::unions::Union2::U2(crate::std_core_iterator::finished());
    };
    p.i = i32::wrapping_add(p.i, p.step);
    return crate::unions::Union2::U1(crate::std_core_iterator::emitted::<i32>(value));
}

pub fn range__Int_Int(mut start: i32, mut end: i32) -> crate::std_core_range::RangeIter {
    let mut step: i32 = if (start < end) {
        1i32
    } else if (start > end) {
        i32::wrapping_neg(1i32)
    } else {
        0i32
    };
    return crate::std_core_range::range__Int_Int_Int(start, end, step);
}

pub fn range__Int(mut end: i32) -> crate::std_core_range::RangeIter {
    return crate::std_core_range::range__Int_Int(0i32, end);
}

pub fn InRange__Int_qualifies(mut n: i32, mut lo: i32, mut hi: i32) -> bool {
    return ((n >= lo) && (n <= hi));
}
