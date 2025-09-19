use std::{
    borrow::Borrow,
    collections::BTreeMap,
    ops::{Bound, Index, RangeTo},
};

pub trait BTrie<K, V> {
    fn get_deepest<
        I: ?Sized + Ord + Index<RangeTo<usize>, Output = I> + Index<usize, Output: PartialEq>,
    >(
        &self,
        key: &I,
    ) -> Option<&V>
    where
        K: Borrow<I>,
        for<'a> &'a I: IntoIterator<Item = &'a <I as Index<usize>>::Output>;
    fn get_deepest_mut<
        I: ?Sized + Ord + Index<RangeTo<usize>, Output = I> + Index<usize, Output: PartialEq>,
    >(
        &mut self,
        key: &I,
    ) -> Option<&mut V>
    where
        K: Borrow<I>,
        for<'a> &'a I: IntoIterator<Item = &'a <I as Index<usize>>::Output>;
}
impl<K: Ord, V> BTrie<K, V> for BTreeMap<K, V> {
    fn get_deepest<
        I: ?Sized + Ord + Index<RangeTo<usize>, Output = I> + Index<usize, Output: PartialEq>,
    >(
        &self,
        mut key: &I,
    ) -> Option<&V>
    where
        K: Borrow<I>,
        for<'a> &'a I: IntoIterator<Item = &'a <I as Index<usize>>::Output>,
    {
        loop {
            let (k, v) = self
                .range((Bound::Unbounded, Bound::Included(key)))
                .last()?;
            if k.borrow() == key {
                break Some(v);
            }
            key = &key[..k
                .borrow()
                .into_iter()
                .zip(key)
                .take_while(|(a, b)| a == b)
                .count()];
        }
    }
    fn get_deepest_mut<
        I: ?Sized + Ord + Index<RangeTo<usize>, Output = I> + Index<usize, Output: PartialEq>,
    >(
        &mut self,
        mut key: &I,
    ) -> Option<&mut V>
    where
        K: Borrow<I>,
        for<'a> &'a I: IntoIterator<Item = &'a <I as Index<usize>>::Output>,
    {
        loop {
            let (k, _) = self
                .range((Bound::Unbounded, Bound::Included(key)))
                .last()?;
            if k.borrow() == key {
                break Some(self.get_mut(key).unwrap());
            }
            key = &key[..k
                .borrow()
                .into_iter()
                .zip(key)
                .take_while(|(a, b)| a == b)
                .count()];
        }
    }
}
