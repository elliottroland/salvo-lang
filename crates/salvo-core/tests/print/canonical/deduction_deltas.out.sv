// The claims a deduction establishes (`+Q`) print before the ones it keeps,
// each in the order written.
fn add<T>(list: Heap<T>(?cmp) Mut List<T>, elem: T) -> None
=> list: +Heap<T>(?cmp) Mut, !elem {}

fn push<T>(heap: Heap<T>(?cmp) Mut List<T>) -> None
=> heap: +Heap<T>(?cmp) +Sorted NonEmpty Mut {}

fn kept<T>(heap: Heap<T>(?cmp) Mut List<T>) -> None
=> heap: +Heap<T>(?cmp) NonEmpty Mut {}

fn dropped<T>(heap: Heap<T>(?cmp) Mut List<T>) -> None
=> heap: -Heap -NonEmpty {}
