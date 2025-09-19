mod trie;
pub use trie::*;
pub mod testing;
#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn nop() {}
}

#[doc(hidden)]
#[allow(dead_code)]
#[macro_use]
pub mod util {
    use std::{
        marker::PhantomData,
        time::{Duration, Instant},
    };

    pub fn debug_fn<F: Fn(&mut std::fmt::Formatter<'_>) -> std::fmt::Result>(
        f: F,
    ) -> impl std::fmt::Debug {
        struct DebugFn<F>(F);
        impl<F: Fn(&mut std::fmt::Formatter<'_>) -> std::fmt::Result> std::fmt::Debug for DebugFn<F> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0(f)
            }
        }
        DebugFn(f)
    }

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
    pub trait OptExt<T>: Sized {
        fn invert<U>(self, u: U) -> Option<U>;
        fn invert_with<U>(self, f: impl FnOnce() -> U) -> Option<U>;
        fn remap<U>(f: impl FnOnce(T) -> U) -> impl FnOnce(Self) -> Option<U>;
    }
    impl<T> OptExt<T> for Option<T> {
        fn invert<U>(self, u: U) -> Option<U> {
            self.ok_or(u).err()
        }
        fn invert_with<U>(self, f: impl FnOnce() -> U) -> Option<U> {
            self.ok_or_else(f).err()
        }
        fn remap<U>(f: impl FnOnce(T) -> U) -> impl FnOnce(Self) -> Option<U> {
            |this| this.map(f)
        }
    }
    pub trait ResExt<T, U>: Sized {
        fn invert(self) -> Result<U, T>;
        fn remap<V>(f: impl FnOnce(T) -> V) -> impl FnOnce(Self) -> Result<V, U>;
        fn remap_err<V>(f: impl FnOnce(U) -> V) -> impl FnOnce(Self) -> Result<T, V>;
    }
    impl<T, U> ResExt<T, U> for Result<T, U> {
        fn invert(self) -> Result<U, T> {
            match self {
                Ok(t) => Err(t),
                Err(u) => Ok(u),
            }
        }
        fn remap<V>(f: impl FnOnce(T) -> V) -> impl FnOnce(Self) -> Result<V, U> {
            |this| this.map(f)
        }
        fn remap_err<V>(f: impl FnOnce(U) -> V) -> impl FnOnce(Self) -> Result<T, V> {
            |this| this.map_err(f)
        }
    }
    pub trait TryExt: Sized {
        type T;
        type U;
        type InvertArg;
        type Inverted: TryExt<T = Self::U, U = Self::T>;
        type RemapOk<V>: TryExt<T = V, U = Self::U>;
        type RemapErr<V>: TryExt<T = Self::T, U = V>;
        fn invert(self, arg: Self::InvertArg) -> Self::Inverted;
        fn invert_with(self, f: impl FnOnce() -> Self::InvertArg) -> Self::Inverted;
        fn remap<V>(f: impl FnOnce(Self::T) -> V) -> impl FnOnce(Self) -> Self::RemapOk<V>;
        fn remap_err<V>(f: impl FnOnce(Self::U) -> V) -> impl FnOnce(Self) -> Self::RemapErr<V>;
    }
    impl<T, U> TryExt for Result<T, U> {
        type T = T;
        type U = U;
        type InvertArg = ();
        type Inverted = Result<U, T>;
        type RemapOk<V> = Result<V, U>;
        type RemapErr<V> = Result<T, V>;
        fn invert(self, (): ()) -> Result<U, T> {
            ResExt::invert(self)
        }
        fn invert_with(self, f: impl FnOnce() -> ()) -> Result<U, T> {
            () = f();
            ResExt::invert(self)
        }
        fn remap<V>(f: impl FnOnce(T) -> V) -> impl FnOnce(Self) -> Result<V, U> {
            <Result<T, U> as ResExt<T, U>>::remap(f)
        }
        fn remap_err<V>(f: impl FnOnce(U) -> V) -> impl FnOnce(Self) -> Result<T, V> {
            <Result<T, U> as ResExt<T, U>>::remap_err(f)
        }
    }

    impl<T, U> TryExt for Option<(T, PhantomData<U>)> {
        type T = T;
        type U = U;
        type InvertArg = U;
        type Inverted = Option<(U, PhantomData<T>)>;
        type RemapOk<V> = Option<(V, PhantomData<U>)>;
        type RemapErr<V> = Option<(T, PhantomData<V>)>;
        fn invert(self, u: U) -> Self::Inverted {
            OptExt::invert(self, (u, PhantomData))
        }
        fn invert_with(self, f: impl FnOnce() -> Self::InvertArg) -> Self::Inverted {
            OptExt::invert_with(self, || (f(), PhantomData))
        }
        fn remap<V>(f: impl FnOnce(Self::T) -> V) -> impl FnOnce(Self) -> Self::RemapOk<V> {
            <Option<(T, PhantomData<U>)> as OptExt<(T, PhantomData<U>)>>::remap(
                |(t, PhantomData)| (f(t), PhantomData),
            )
        }
        fn remap_err<V>(_: impl FnOnce(U) -> V) -> impl FnOnce(Self) -> Self::RemapErr<V> {
            |this| this.map(|(t, PhantomData)| (t, PhantomData))
        }
    }
    pub struct PhantomOption<T, U>(Option<T>, PhantomData<U>);
    impl<T, U> From<Option<T>> for PhantomOption<T, U> {
        fn from(value: Option<T>) -> Self {
            Self(value, PhantomData)
        }
    }
    impl<T, U> Into<Option<T>> for PhantomOption<T, U> {
        fn into(self) -> Option<T> {
            self.0
        }
    }
    impl<T, V> OptExt<T> for PhantomOption<T, V> {
        fn invert<U>(self, u: U) -> Option<U> {
            self.0.invert(u).into()
        }
        fn invert_with<U>(self, f: impl FnOnce() -> U) -> Option<U> {
            self.0.invert_with(f).into()
        }
        fn remap<U>(f: impl FnOnce(T) -> U) -> impl FnOnce(Self) -> Option<U> {
            |Self(o, PhantomData)| o.map(f).into()
        }
    }
    impl<T, U> TryExt for PhantomOption<T, U> {
        type T = T;
        type U = U;
        type InvertArg = U;
        type Inverted = PhantomOption<U, T>;
        type RemapOk<V> = PhantomOption<V, U>;
        type RemapErr<V> = PhantomOption<T, V>;
        fn invert(self, u: U) -> Self::Inverted {
            OptExt::invert(self, u).into()
        }
        fn invert_with(self, f: impl FnOnce() -> Self::InvertArg) -> Self::Inverted {
            OptExt::invert_with(self, f).into()
        }
        fn remap<V>(f: impl FnOnce(Self::T) -> V) -> impl FnOnce(Self) -> Self::RemapOk<V> {
            |this| <PhantomOption<T, U> as OptExt<T>>::remap(f)(this).into()
        }
        fn remap_err<V>(_: impl FnOnce(U) -> V) -> impl FnOnce(Self) -> Self::RemapErr<V> {
            |Self(this, _)| this.into()
        }
    }
    pub fn remap<T: TryExt, V>(f: impl FnOnce(T::T) -> V) -> impl FnOnce(T) -> T::RemapOk<V> {
        T::remap(f)
    }
    pub fn time<T>(f: impl FnOnce() -> T) -> (Duration, T) {
        let start = Instant::now();
        let ret = f();
        let time = start.elapsed();
        (time, ret)
    }
}
