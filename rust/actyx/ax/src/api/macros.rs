/// Helper to combine multiple filters together with Filter::or, possibly boxing the types in the
/// process. This significantly decreases build times.
///
/// Takes a list of filter expressions and `or`s them together in a balanced tree. That is, instead
/// of `a.or(b).or(c).or(d)`, it produces `(a.or(b)).or(c.or(d))`, thus nesting the types less
/// deeply, which provides improvements in compile time.
///
/// It also applies `::warp::Filter::boxed` to each handler expression when in `debug_assertions`
/// mode, improving compile time further.
/// Source: https://github.com/seanmonstar/warp/issues/619#issuecomment-662716377
#[macro_export]
macro_rules! balanced_or {
    // Case 0: Single expression
    // RKU: switched to always boxing, halves --release compile time for node & swarm-cli
    ($x:expr $(,)?) => { ::warp::Filter::boxed($x) };
    // Case 1: Multiple expressions, recurse with three lists: left (starts empty), right, and a
    // counter (for which the initial list of expressions is abused).
    ($($x:expr),+ $(,)?) => {
        balanced_or!(@internal ; $($x),+; $($x),+)
    };
    // Case 2: Counter <= 2: move one more item and recurse on each sublist, and or them together
    (@internal $($left:expr),*; $head:expr, $($tail:expr),+; $a:expr $(,$b:expr)?) => {
        (balanced_or!($($left,)* $head)).or(balanced_or!($($tail),+))
    };
    // Case 3: Counter > 2: move one item from the right to the left and subtract two from the
    // counter
    (@internal $($left:expr),*; $head:expr, $($tail:expr),+; $a:expr, $b:expr, $($more:expr),+) => {
        balanced_or!(@internal $($left,)* $head; $($tail),+; $($more),+)
    };
}

#[cfg(debug_assertions)]
#[macro_export]
macro_rules! boxed_on_debug {
    ($x:expr) => {
        ::warp::Filter::boxed($x)
    };
}

#[cfg(not(debug_assertions))]
#[macro_export]
macro_rules! boxed_on_debug {
    ($x:expr) => {
        $x
    };
}
