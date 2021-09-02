/// Helper to combine multiple filters together with Filter::or, possibly boxing the types in
/// the process. This significantly decreases build times.
/// Source: https://github.com/seanmonstar/warp/issues/619#issuecomment-662716377
#[macro_export]
macro_rules! or {
    ($x:expr $(,)?) => { boxed_on_debug!($x) };
    ($($x:expr),+ $(,)?) => {
        or!(@internal ; $($x),+; $($x),+)
    };
    (@internal $($left:expr),*; $head:expr, $($tail:expr),+; $a:expr $(,$b:expr)?) => {
        (or!($($left,)* $head)).or(or!($($tail),+))
    };
    (@internal $($left:expr),*; $head:expr, $($tail:expr),+; $a:expr, $b:expr, $($more:expr),+) => {
        or!(@internal $($left,)* $head; $($tail),+; $($more),+)
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
