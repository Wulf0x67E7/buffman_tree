use std::{borrow::Borrow, ops::Deref};

use slab::Slab;

use crate::{Branch, Key, Node, NodeId, handle::Handle};

pub struct Walk<K: Key, V, W> {
    stack: Vec<NodeId<K, V>>,
    way: W,
    #[cfg(debug_assertions)]
    unique: std::collections::HashSet<NodeId<K, V>, std::hash::RandomState>,
}
pub trait Way<K: Key, V> {
    fn find(&mut self, branch: &Branch<K, V>) -> impl IntoIterator<Item = NodeId<K, V>>;
}
pub struct Ordered;
impl<K: Key, V> Way<K, V> for Ordered {
    fn find(&mut self, branch: &Branch<K, V>) -> impl IntoIterator<Item = NodeId<K, V>> {
        branch.children().rev().map(Handle::leak)
    }
}
pub struct Keyed<I>(I);
pub fn keyed<K: Key>(key: &K) -> Keyed<impl Iterator<Item = &K::Piece>> {
    Keyed::wrap(key.pieces())
}
impl<T> Keyed<T> {
    pub fn wrap(value: T) -> Self {
        Self(value)
    }
}
impl<
    K: Key<Piece: Borrow<<Q::Item as Deref>::Target>>,
    V,
    Q: Iterator<Item: Deref<Target: ?Sized + Ord>>,
> Way<K, V> for Keyed<Q>
{
    fn find(&mut self, branch: &Branch<K, V>) -> impl IntoIterator<Item = NodeId<K, V>> {
        Some(branch)
            .zip(self.0.next())
            .and_then(|(branch, piece)| branch.get_handle(&piece))
            .map(Handle::leak)
    }
}

pub struct Predicated<P>(P);
impl<P> From<P> for Predicated<P> {
    fn from(value: P) -> Self {
        Self(value)
    }
}
impl<K: Key, V, P: FnMut(&Branch<K, V>) -> I, I: IntoIterator<Item = NodeId<K, V>>> Way<K, V>
    for Predicated<P>
{
    fn find(&mut self, branch: &Branch<K, V>) -> impl IntoIterator<Item = NodeId<K, V>> {
        self.0(branch)
    }
}
impl<K: Key, V, W: Way<K, V>> Walk<K, V, W> {
    pub fn start(root: &Option<NodeId<K, V>>, way: W) -> Self {
        Self {
            stack: Vec::from_iter(root.as_ref().map(Handle::leak)),
            way,
            #[cfg(debug_assertions)]
            unique: std::collections::HashSet::from_iter(root.as_ref().map(Handle::leak)),
        }
    }
    pub fn next(&mut self, shared: &Slab<Node<K, V>>) -> Option<NodeId<K, V>> {
        let node = self.stack.pop()?;
        let branch = node.get(&shared).as_branch();
        for x in branch
            .map(|branch| self.way.find(branch))
            .into_iter()
            .flatten()
        {
            #[cfg(debug_assertions)]
            debug_assert!(self.unique.insert(x.leak()));
            self.stack.push(x);
        }
        Some(node)
    }
}
