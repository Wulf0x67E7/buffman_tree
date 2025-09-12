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
    fn find(
        &mut self,
        branch: Option<&Branch<K, V>>,
    ) -> Option<impl IntoIterator<Item = NodeId<K, V>>>;
}
pub struct Ordered;
impl<K: Key, V> Way<K, V> for Ordered {
    fn find(
        &mut self,
        branch: Option<&Branch<K, V>>,
    ) -> Option<impl IntoIterator<Item = NodeId<K, V>>> {
        Some(
            branch
                .into_iter()
                .flat_map(|branch| branch.children().rev().map(Handle::leak)),
        )
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
    fn find(
        &mut self,
        branch: Option<&Branch<K, V>>,
    ) -> Option<impl IntoIterator<Item = NodeId<K, V>>> {
        let Some(piece) = self.0.next() else {
            return Some(None);
        };
        Some(Some(branch?.get_handle(&piece)?.leak()))
    }
}

pub struct Predicated<P>(P);
impl<P> From<P> for Predicated<P> {
    fn from(value: P) -> Self {
        Self(value)
    }
}
impl<K: Key, V, P: FnMut(Option<&Branch<K, V>>) -> Option<I>, I: IntoIterator<Item = NodeId<K, V>>>
    Way<K, V> for Predicated<P>
{
    fn find(
        &mut self,
        branch: Option<&Branch<K, V>>,
    ) -> Option<impl IntoIterator<Item = NodeId<K, V>>> {
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
    pub fn next(&mut self, shared: &Slab<Node<K, V>>) -> Option<(NodeId<K, V>, bool)> {
        let node = self.stack.pop()?;
        let branch = node.get(&shared).as_branch();
        let err = if let Some(way) = self.way.find(branch) {
            for x in way {
                #[cfg(debug_assertions)]
                debug_assert!(self.unique.insert(x.leak()));
                self.stack.push(x);
            }
            false
        } else {
            true
        };
        Some((node, err))
    }
}
