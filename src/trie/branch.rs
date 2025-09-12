use slab::Slab;

use crate::{Key, Node, NodeId, handle::Handle, util::unzipped};
use std::{
    borrow::Borrow,
    collections::{BTreeMap, btree_map::Entry},
    fmt::Debug,
};

#[derive(PartialEq)]
pub struct Branch<K: Key, V>(BTreeMap<K::Piece, NodeId<K, V>>);
impl<K: Key, V> Default for Branch<K, V> {
    fn default() -> Self {
        Self(Default::default())
    }
}
impl<K: Key, V: Debug> Debug for Branch<K, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
impl<K: Key, V> FromIterator<(K::Piece, NodeId<K, V>)> for Branch<K, V> {
    fn from_iter<T: IntoIterator<Item = (K::Piece, NodeId<K, V>)>>(iter: T) -> Self {
        let mut ret = Self::default();
        for (key, value) in iter {
            ret.0.insert(key, value);
        }
        ret
    }
}
impl<K: Key, V> Branch<K, V> {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn insert_handle(&mut self, key: K::Piece, child: NodeId<K, V>) -> Option<NodeId<K, V>> {
        self.0.insert(key, child)
    }
    pub fn get_or_insert<'a>(
        &mut self,
        key: K::Piece,
        shared: &'a mut Slab<Node<K, V>>,
    ) -> &'a mut Node<K, V> {
        self.0
            .entry(key)
            .or_insert_with(|| Handle::new_default(shared))
            .get_mut(shared)
    }
    pub fn get_handle<Q: ?Sized>(&self, key: &Q) -> Option<&NodeId<K, V>>
    where
        Q: Ord,
        K::Piece: Borrow<Q>,
    {
        self.0.get(key)
    }
    pub fn get<'a>(&self, key: K::Piece, shared: &'a Slab<Node<K, V>>) -> Option<&'a Node<K, V>> {
        self.get_handle(&key)
            .zip(Some(shared))
            .map(unzipped(Handle::get))
    }
    pub fn get_mut<'a>(
        &self,
        key: K::Piece,
        shared: &'a mut Slab<Node<K, V>>,
    ) -> Option<&'a mut Node<K, V>> {
        self.get_handle(&key)
            .zip(Some(shared))
            .map(unzipped(Handle::get_mut))
    }
    pub fn remove<Q>(&mut self, key: &Q) -> Option<NodeId<K, V>>
    where
        Q: Ord,
        K::Piece: Borrow<Q>,
    {
        self.0.remove(key)
    }
    pub fn remove_if<'a>(
        &mut self,
        key: K::Piece,
        shared: &'a mut Slab<Node<K, V>>,
        f: impl FnOnce(&mut Slab<Node<K, V>>, &NodeId<K, V>) -> bool,
    ) -> Option<Node<K, V>> {
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
    pub fn children(&self) -> Children<K, V> {
        self.0.values()
    }
    pub fn iter(&self) -> Iter<K, V> {
        self.0.iter()
    }
}
#[allow(type_alias_bounds)]
pub type Children<'a, K: Key, V> =
    std::collections::btree_map::Values<'a, K::Piece, Handle<Node<K, V>>>;
#[allow(type_alias_bounds)]
pub type Iter<'a, K: Key, V> = std::collections::btree_map::Iter<'a, K::Piece, Handle<Node<K, V>>>;
