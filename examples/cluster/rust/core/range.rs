use crate::core_array::*;
use crate::core_bytes::*;
use crate::core_iterator::*;
use crate::core_list::*;
use crate::core_map::*;
use crate::core_set::*;
use crate::core_string::*;
use crate::unions::*;

#[derive(Clone, Debug, PartialEq)]
pub struct Range {
    pub start: i32,
    pub end: i32,
    pub step: i32,
}

impl crate::wire::__Wire for Range {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.start, out);
        crate::wire::__Wire::__enc(&self.end, out);
        crate::wire::__Wire::__enc(&self.step, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            start: crate::wire::__Wire::__dec(r)?,
            end: crate::wire::__Wire::__dec(r)?,
            step: crate::wire::__Wire::__dec(r)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct __Pass_Range {
    pub step: i32,
    pub end: i32,
    pub i: i32,
}

impl crate::wire::__Wire for __Pass_Range {
    fn __enc(&self, out: &mut Vec<u8>) {
        crate::wire::__Wire::__enc(&self.step, out);
        crate::wire::__Wire::__enc(&self.end, out);
        crate::wire::__Wire::__enc(&self.i, out);
    }
    fn __dec(r: &mut crate::wire::__Reader<'_>) -> Option<Self> {
        Some(Self {
            step: crate::wire::__Wire::__dec(r)?,
            end: crate::wire::__Wire::__dec(r)?,
            i: crate::wire::__Wire::__dec(r)?,
        })
    }
}

pub fn iter__5(range: &Range) -> __Pass_Range {
    return __Pass_Range { end: range.end, step: range.step, i: range.start };
}

pub fn next__8(__p: &mut __Pass_Range) -> Union2<i32, Finished> {
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

pub fn range(start: i32, end: i32, step: i32) -> Range {
    return Range { start: start, end: end, step: step };
}

pub fn range__2(start: i32, end: i32) -> Range {
    let mut step = if start < end {
        1
    } else if start > end {
        -1
    } else {
        0
    };
    return range(start, end, step);
}

pub fn range__3(end: i32) -> Range {
    return range__2(0, end);
}

pub fn InRange_qualifies(n: i32, lo: i32, hi: i32) -> bool {
    return n >= lo && n <= hi;
}
