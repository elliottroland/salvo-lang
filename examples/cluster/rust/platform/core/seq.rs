// [platform-value-type] `core.seq`'s list fast path: `filter` over a `List`
// keeps a copy of each element that passes (ROADMAP 0.7).

pub fn filter<T: Clone>(list: &Vec<T>, keep: &mut dyn FnMut(&T) -> bool) -> Vec<T> {
    list.iter().filter(|x| keep(x)).cloned().collect()
}
