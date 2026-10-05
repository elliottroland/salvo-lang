// [platform-value-type] std's list, for `core.list`'s
// `platform type List<T> canbe Mut` (ROADMAP 0.7): a `Vec`. `Mut List` is the
// same type, passed as `&mut`.

pub type List<T> = Vec<T>;
pub type MutList<T> = Vec<T>;

// [platform-iterable] The host's loop over a list, by reference.
pub fn each<T>(list: &List<T>) -> std::slice::Iter<'_, T> {
    list.iter()
}

pub fn each_mut<T>(list: &mut List<T>) -> std::slice::IterMut<'_, T> {
    list.iter_mut()
}

pub fn into_each<T>(list: List<T>) -> std::vec::IntoIter<T> {
    list.into_iter()
}

pub fn get<T>(list: &List<T>, index: i32) -> Option<&T> {
    if index < 0 { None } else { list.get(index as usize) }
}

pub fn first<T>(list: &List<T>) -> Option<&T> {
    list.first()
}

pub fn size<T>(list: &List<T>) -> i32 {
    list.len() as i32
}

pub fn add<T>(list: &mut List<T>, elem: T) {
    list.push(elem);
}

pub fn remove_first<T>(list: &mut List<T>) -> Option<T> {
    if list.is_empty() { None } else { Some(list.remove(0)) }
}

pub fn remove_last<T>(list: &mut List<T>) -> Option<T> {
    list.pop()
}

pub fn remove_at<T>(list: &mut List<T>, index: i32) -> Option<T> {
    if index < 0 || index as usize >= list.len() { None } else { Some(list.remove(index as usize)) }
}

// [col-insert] The element back when the index is outside `0..=len`.
pub fn insert_at<T>(list: &mut List<T>, index: i32, elem: T) -> Option<T> {
    if index < 0 || index as usize > list.len() {
        return Some(elem);
    }
    list.insert(index as usize, elem);
    None
}

// [col-remove-range] Clamped to the list.
pub fn remove_range<T>(list: &mut List<T>, from: i32, to: i32) -> List<T> {
    let len = list.len() as i64;
    let from = (from as i64).clamp(0, len) as usize;
    let to = (to as i64).clamp(from as i64, len) as usize;
    list.drain(from..to).collect()
}

// [col-bounds] Out of range moves nothing and answers `false`.
pub fn swap_at<T>(list: &mut List<T>, i: i32, j: i32) -> bool {
    let n = list.len() as i32;
    if i < 0 || j < 0 || i >= n || j >= n {
        return false;
    }
    list.swap(i as usize, j as usize);
    true
}

// [col-replace] The index was proven in range by the caller's `Idx` claim.
pub fn replace_at<T>(list: &mut List<T>, index: i32, value: T) -> T {
    std::mem::replace(&mut list[index as usize], value)
}

pub fn into_mut<T>(list: List<T>) -> List<T> {
    list
}

// [linear-container] Ending one that still holds elements would drop them.
pub fn end_empty<T>(list: List<T>) {
    assert!(list.is_empty(), "salvo: a list was ended with elements in it");
}

// A stable sort, in the language's ordering, of a copy.
pub fn sort_by<T: Clone>(list: &List<T>, cmp: &mut dyn FnMut(&T, &T) -> i32) -> List<T> {
    let mut out = list.clone();
    out.sort_by(|a, b| cmp(a, b).cmp(&0));
    out
}

// A **lower bound**: an equal run is entered from the front, so both backends
// land on the same index.
pub fn insert_sorted_by<T>(list: &mut List<T>, elem: T, cmp: &mut dyn FnMut(&T, &T) -> i32) {
    let at = list.partition_point(|x| cmp(x, &elem) < 0);
    list.insert(at, elem);
}

// Lower bound, then a tie in the same ordering [col-membership].
pub fn search_sorted_by<T>(list: &List<T>, elem: &T, cmp: &mut dyn FnMut(&T, &T) -> i32) -> Option<i32> {
    let at = list.partition_point(|x| cmp(x, elem) < 0);
    if at < list.len() && cmp(&list[at], elem) == 0 { Some(at as i32) } else { None }
}
