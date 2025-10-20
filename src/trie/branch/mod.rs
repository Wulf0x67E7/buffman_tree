mod btree;
mod byte;
mod hash;
use crate::{NodeDebug, trie::node::NodeHandle};
pub use btree::*;
pub use byte::*;
pub use hash::*;

pub trait Branch<K, V, Q = K>: Sized + Default + NodeDebug<K, V, Self> {
    fn is_empty(&self) -> bool;
    fn insert(&mut self, key: K, node: NodeHandle<K, V, Self>) -> Option<NodeHandle<K, V, Self>>;
    fn get_or_insert_with(
        &mut self,
        key: K,
        f: impl FnOnce() -> NodeHandle<K, V, Self>,
    ) -> NodeHandle<K, V, Self>;
    fn get(&self, key: &Q) -> Option<NodeHandle<K, V, Self>>;
    fn cleanup(&mut self, f: impl FnMut(&mut NodeHandle<K, V, Self>) -> bool) -> usize;
    fn prune(
        &mut self,
        f: impl FnMut(&mut NodeHandle<K, V, Self>) -> bool,
    ) -> Option<Option<(K, NodeHandle<K, V, Self>)>>;

    fn iter<'a>(&'a self) -> impl Iterator<Item = (&'a K, NodeHandle<K, V, Self>)>
    where
        K: 'a;
    fn keys<'a>(&'a self) -> impl Iterator<Item = &'a K>
    where
        K: 'a,
    {
        self.iter().map(|(k, _)| k)
    }
    fn values<'a>(&'a self) -> impl Iterator<Item = NodeHandle<K, V, Self>>
    where
        K: 'a,
    {
        self.iter().map(|(_, v)| v)
    }
}
