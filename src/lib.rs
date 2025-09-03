mod trie;
pub use trie::*;
#[cfg(test)]

mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn nop() {}
}

#[allow(dead_code)]
#[macro_use]
pub(crate) mod util {
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
    pub fn map_chain2<T, U, V>(
        mut m0: impl FnMut(T) -> U,
        mut m1: impl FnMut(U) -> V,
    ) -> impl FnMut(T) -> V {
        map_chain!(m0, m1)
    }
}
