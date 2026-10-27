// [platform-value-type] `std.core.seq`'s list fast path: `filter` over a `List`
// answers a view [rs-proj-generic]: a borrow of each element that passes,
// nothing copied (ROADMAP 0.7; a borrow since 0v).

pub fn filter<'a, T>(list: &'a Vec<T>, keep: &mut dyn FnMut(&T) -> bool) -> Vec<&'a T> {
    list.iter().filter(|x| keep(x)).collect()
}
