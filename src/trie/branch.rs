use slab::Slab;

use crate::{Node, handle::Handle, util::unzipped};
use std::collections::{BTreeMap, btree_map::Entry};

#[derive(Debug, PartialEq)]
pub struct Branch<K, B, V>(BTreeMap<B, Handle<Node<K, B, V>>>);
impl<K, B, V> Default for Branch<K, B, V> {
    fn default() -> Self {
        Self(Default::default())
    }
}
impl<K, B, V> FromIterator<(B, Handle<Node<K, B, V>>)> for Branch<K, B, V>
where
    B: Ord,
{
    fn from_iter<T: IntoIterator<Item = (B, Handle<Node<K, B, V>>)>>(iter: T) -> Self {
        let mut ret = Self::default();
        for (key, value) in iter {
            ret.0.insert(key, value);
        }
        ret
    }
}
impl<K, B, V> Branch<K, B, V> {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn insert_handle(
        &mut self,
        key: B,
        child: Handle<Node<K, B, V>>,
    ) -> Option<Handle<Node<K, B, V>>>
    where
        B: Ord,
    {
        self.0.insert(key, child)
    }
    pub fn get_or_insert<'a>(
        &mut self,
        key: B,
        shared: &'a mut Slab<Node<K, B, V>>,
    ) -> &'a mut Node<K, B, V>
    where
        B: Ord,
    {
        self.0
            .entry(key)
            .or_insert_with(|| Handle::new_default(shared))
            .get_mut(shared)
    }
    pub fn get_handle(&self, key: B) -> Option<&Handle<Node<K, B, V>>>
    where
        B: Ord,
    {
        self.0.get(&key)
    }
    pub fn get<'a>(&self, key: B, shared: &'a Slab<Node<K, B, V>>) -> Option<&'a Node<K, B, V>>
    where
        B: Ord,
    {
        self.get_handle(key)
            .zip(Some(shared))
            .map(unzipped(Handle::get))
    }
    pub fn get_mut<'a>(
        &self,
        key: B,
        shared: &'a mut Slab<Node<K, B, V>>,
    ) -> Option<&'a mut Node<K, B, V>>
    where
        B: Ord,
    {
        self.get_handle(key)
            .zip(Some(shared))
            .map(unzipped(Handle::get_mut))
    }
    pub fn remove(&mut self, key: B) -> Option<Handle<Node<K, B, V>>>
    where
        B: Ord,
    {
        self.0.remove(&key)
    }
    pub fn remove_if<'a>(
        &mut self,
        key: B,
        shared: &'a mut Slab<Node<K, B, V>>,
        f: impl FnOnce(&mut Slab<Node<K, B, V>>, &Handle<Node<K, B, V>>) -> bool,
    ) -> Option<Node<K, B, V>>
    where
        B: Ord,
    {
        match self.0.entry(key) {
            Entry::Vacant(_) => None,
            Entry::Occupied(child) => {
                if f(shared, child.get()) {
                    Some(child.remove().remove(shared))
                } else {
                    None
                }
            }
        }
    }
    pub fn iter(&self) -> std::collections::btree_map::Iter<B, Handle<Node<K, B, V>>> {
        self.0.iter()
    }
}
