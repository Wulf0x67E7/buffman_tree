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
    pub trait OptExt {
        fn invert<U>(self, u: U) -> Option<U>;
    }
    impl<T> OptExt for Option<T> {
        fn invert<U>(self, u: U) -> Option<U> {
            self.ok_or(u).err()
        }
    }
    pub trait ResExt<T, U> {
        fn invert(self) -> Result<U, T>;
    }
    impl<T, U> ResExt<T, U> for Result<T, U> {
        fn invert(self) -> Result<U, T> {
            match self {
                Ok(t) => Err(t),
                Err(u) => Ok(u),
            }
        }
    }
}
