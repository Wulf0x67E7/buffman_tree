use slab::Slab;

use crate::{Node, NodeId, handle::Handle, util::unzipped};
use std::{
    collections::{BTreeMap, btree_map::Entry},
    fmt::Debug,
};

#[derive(PartialEq)]
pub struct Branch<K, S, V>(BTreeMap<S, NodeId<K, S, V>>);
impl<K, S, V> Default for Branch<K, S, V> {
    fn default() -> Self {
        Self(Default::default())
    }
}
impl<K: Debug, S: Debug, V: Debug> Debug for Branch<K, S, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
impl<K, S, V> FromIterator<(S, NodeId<K, S, V>)> for Branch<K, S, V>
where
    S: Ord,
{
    fn from_iter<T: IntoIterator<Item = (S, NodeId<K, S, V>)>>(iter: T) -> Self {
        let mut ret = Self::default();
        for (key, value) in iter {
            ret.0.insert(key, value);
        }
        ret
    }
}
impl<K, S, V> Branch<K, S, V> {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn insert_handle(&mut self, key: S, child: NodeId<K, S, V>) -> Option<NodeId<K, S, V>>
    where
        S: Ord,
    {
        self.0.insert(key, child)
    }
    pub fn get_or_insert<'a>(
        &mut self,
        key: S,
        shared: &'a mut Slab<Node<K, S, V>>,
    ) -> &'a mut Node<K, S, V>
    where
        S: Ord,
    {
        self.0
            .entry(key)
            .or_insert_with(|| Handle::new_default(shared))
            .get_mut(shared)
    }
    pub fn get_handle(&self, key: S) -> Option<&NodeId<K, S, V>>
    where
        S: Ord,
    {
        self.0.get(&key)
    }
    pub fn get<'a>(&self, key: S, shared: &'a Slab<Node<K, S, V>>) -> Option<&'a Node<K, S, V>>
    where
        S: Ord,
    {
        self.get_handle(key)
            .zip(Some(shared))
            .map(unzipped(Handle::get))
    }
    pub fn get_mut<'a>(
        &self,
        key: S,
        shared: &'a mut Slab<Node<K, S, V>>,
    ) -> Option<&'a mut Node<K, S, V>>
    where
        S: Ord,
    {
        self.get_handle(key)
            .zip(Some(shared))
            .map(unzipped(Handle::get_mut))
    }
    pub fn remove(&mut self, key: S) -> Option<NodeId<K, S, V>>
    where
        S: Ord,
    {
        self.0.remove(&key)
    }
    pub fn remove_if<'a>(
        &mut self,
        key: S,
        shared: &'a mut Slab<Node<K, S, V>>,
        f: impl FnOnce(&mut Slab<Node<K, S, V>>, &NodeId<K, S, V>) -> bool,
    ) -> Option<Node<K, S, V>>
    where
        S: Ord,
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
    pub fn children(&self) -> Children<K, S, V> {
        self.0.values()
    }
    pub fn iter(&self) -> Iter<K, S, V> {
        self.0.iter()
    }
}
pub type Children<'a, K, B, V> = std::collections::btree_map::Values<'a, B, Handle<Node<K, B, V>>>;
pub type Iter<'a, K, B, V> = std::collections::btree_map::Iter<'a, B, Handle<Node<K, B, V>>>;
