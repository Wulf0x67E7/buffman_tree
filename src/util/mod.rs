use std::time::{Duration, Instant};
mod debug;
pub mod opt_res_ext;
pub use debug::*;

pub fn unzipped<A, B, R, F: FnMut(A, B) -> R>(mut f: F) -> impl FnMut((A, B)) -> R {
    move |(a, b)| f(a, b)
}
#[macro_export]
macro_rules! map_chain {
    ($($f:expr),*$(,)?) => {
        move |x| {
            let v = x;
            $(let v = $f(v);)*
            v
        }
    };
}
#[macro_export]
macro_rules! flat_map_chain {
    ($($f:expr),*$(,)?) => {
        move |x| {
            let v = x;
            $(let v = $f(v)?;)*
            v
        }
    };
}

#[macro_export]
macro_rules! bool_try {
    ($f:block) => {
        (|| Some($f))().unwrap_or(false)
    };
}

pub fn map_chain2<T, U, V>(
    mut m0: impl FnMut(T) -> U,
    mut m1: impl FnMut(U) -> V,
) -> impl FnMut(T) -> V {
    map_chain!(m0, m1)
}
pub fn time<T>(f: impl FnOnce() -> T) -> (Duration, T) {
    let start = Instant::now();
    let ret = f();
    let time = start.elapsed();
    (time, ret)
}
