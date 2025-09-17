use slab::Slab;
use std::{
    fmt::{Debug, Display},
    hash::Hash,
    marker::PhantomData,
};
pub type Shared<T> = Slab<T>;
pub struct Handle<T>(usize, PhantomData<for<'a> fn(&'a T) -> &'a T>);
impl<T> Handle<T> {
    pub(crate) fn from(vacant: usize) -> Self {
        Self(vacant, PhantomData)
    }
    pub fn new_shared() -> Shared<T> {
        Slab::new()
    }
    pub fn vacant(shared: &Slab<T>) -> Handle<PhantomData<T>> {
        Handle::from(shared.vacant_key())
    }
    pub fn insert(vacant: Handle<PhantomData<T>>, shared: &mut Slab<T>, val: T) -> Self {
        let ret = shared.vacant_entry();
        assert_eq!(vacant.0, ret.key());
        ret.insert(val);
        Self::from(vacant.0)
    }
    pub fn new(shared: &mut Slab<T>, val: T) -> Self {
        Self::from(shared.insert(val))
    }
    pub fn new_default(shared: &mut Slab<T>) -> Self
    where
        T: Default,
    {
        Self::new(shared, T::default())
    }
    pub fn leak(&self) -> Self {
        Self::from(self.0)
    }
    pub fn get<'a>(&self, shared: &'a Slab<T>) -> &'a T {
        &shared[self.0]
    }
    pub fn get_mut<'a>(&self, shared: &'a mut Slab<T>) -> &'a mut T {
        &mut shared[self.0]
    }
    pub fn insert_if<'a>(
        &mut self,
        shared: &'a mut Slab<T>,
        predicate: impl FnOnce(&mut T) -> Result<Self, T>,
        f: impl FnOnce(&mut T, &Self),
    ) -> Self {
        match predicate(self.get_mut(shared)) {
            Ok(handle) => handle,
            Err(val) => {
                let ret = Self::new(shared, val);
                f(self.get_mut(shared), &ret);
                ret
            }
        }
    }
    pub fn remove_if<'a>(
        &mut self,
        shared: &'a mut Slab<T>,
        predicate: impl FnOnce(&mut Self, &mut Slab<T>) -> Option<Self>,
        f: impl FnOnce(&mut Self, &mut Slab<T>, Self),
    ) -> Option<T> {
        let handle = predicate(self, shared)?;
        let ret = handle.leak().remove(shared);
        f(self, shared, handle);
        Some(ret)
    }
    pub fn replace<'a>(&self, shared: &'a mut Slab<T>, val: T) -> T {
        std::mem::replace(self.get_mut(shared), val)
    }
    pub fn take(&self, shared: &mut Slab<T>) -> T
    where
        T: Default,
    {
        self.replace(shared, T::default())
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
