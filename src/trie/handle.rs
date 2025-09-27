use slab::Slab;
use std::{
    fmt::{Debug, Display},
    hash::Hash,
    marker::PhantomData,
    usize,
};
pub type Shared<T> = Slab<T>;
pub struct Handle<T>(usize, PhantomData<for<'a> fn(&'a T) -> &'a T>);
impl<T> Handle<T> {
    pub(crate) fn from(vacant: usize) -> Self {
        Self(vacant, PhantomData)
    }
    pub(crate) fn _unwrap(self) -> usize {
        self.0
    }
    pub fn new_shared() -> Shared<T> {
        Slab::new()
    }
    pub fn new_shared_with_capacity(capacity: usize) -> Shared<T> {
        Slab::with_capacity(capacity)
    }
    pub fn new_null() -> Self {
        Self::from(usize::MAX)
    }
    pub fn _set(self, shared: &mut Slab<T>, val: T) -> Self {
        assert!(self._is_null());
        Self::from(shared.insert(val))
    }
    pub fn new(shared: &mut Slab<T>, val: T) -> Self {
        Self::from(shared.insert(val))
    }
    pub fn new_with(shared: &mut Slab<T>, f: impl FnOnce(Self) -> T) -> Self {
        let this = Self::from(shared.vacant_key());
        let ret = Self::new(shared, f(this.leak()));
        debug_assert_eq!(this, ret);
        ret
    }
    pub fn _new_default(shared: &mut Slab<T>) -> Self
    where
        T: Default,
    {
        Self::new(shared, T::default())
    }
    pub fn _is_null(&self) -> bool {
        self.0 == usize::MAX
    }
    pub fn leak(&self) -> Self {
        Self::from(self.0)
    }
    pub fn _get_null<'a>(&self, shared: &'a Slab<T>) -> Option<&'a T> {
        (!self._is_null()).then(|| self.get(shared))
    }
    pub fn get<'a>(&self, shared: &'a Slab<T>) -> &'a T {
        &shared[self.0]
    }
    pub fn _get_mut_null<'a>(&self, shared: &'a mut Slab<T>) -> Option<&'a mut T> {
        (!self._is_null()).then(|| self.get_mut(shared))
    }
    pub fn get_mut<'a>(&self, shared: &'a mut Slab<T>) -> &'a mut T {
        &mut shared[self.0]
    }
    pub fn _replace_null(&self, shared: &mut Slab<T>, val: T) -> Result<T, T> {
        if self._is_null() {
            Err(val)
        } else {
            Ok(self._replace(shared, val))
        }
    }
    pub fn _replace(&self, shared: &mut Slab<T>, val: T) -> T {
        std::mem::replace(self.get_mut(shared), val)
    }
    pub fn _remove_null(self, shared: &mut Slab<T>) -> Option<T> {
        (!self._is_null()).then(|| self.remove(shared))
    }
    pub fn remove(self, shared: &mut Slab<T>) -> T {
        shared.remove(self.0)
    }
}
impl<T> Debug for Handle<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Handle").field(&self.0).finish()
    }
}
impl<T> Display for Handle<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <usize as Display>::fmt(&self.0, f)
    }
}
impl<T> PartialEq for Handle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
impl<T> Eq for Handle<T> {}
impl<T> PartialOrd for Handle<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(&other.0)
    }
}
impl<T> Ord for Handle<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}
impl<T> Hash for Handle<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}
