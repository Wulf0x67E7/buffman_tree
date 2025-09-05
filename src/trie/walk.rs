use slab::Slab;

use crate::{Branch, Node, NodeId, handle::Handle, util::unzipped};

pub struct Walk<K, S, V, W> {
    stack: Vec<NodeId<K, S, V>>,
    way: W,
    #[cfg(debug_assertions)]
    unique: std::collections::HashSet<NodeId<K, S, V>, std::hash::RandomState>,
}
pub trait Way<K, S, V> {
    fn find(&mut self, branch: &Branch<K, S, V>) -> impl IntoIterator<Item = NodeId<K, S, V>>;
}
pub struct Ordered;
impl<K, S, V> Way<K, S, V> for Ordered {
    fn find(&mut self, branch: &Branch<K, S, V>) -> impl IntoIterator<Item = NodeId<K, S, V>> {
        branch.children().rev().map(Handle::leak)
    }
}
pub struct Keyed<I>(I);
impl<T: IntoIterator> From<T> for Keyed<T::IntoIter> {
    fn from(value: T) -> Self {
        Self(value.into_iter())
    }
}
impl<K, S: Ord, V, I: Iterator<Item = S>> Way<K, S, V> for Keyed<I> {
    fn find(&mut self, branch: &Branch<K, S, V>) -> impl IntoIterator<Item = NodeId<K, S, V>> {
        Some(branch)
            .zip(self.0.next())
            .and_then(unzipped(Branch::get_handle))
            .map(Handle::leak)
    }
}

pub struct Predicated<P>(P);
impl<P> From<P> for Predicated<P> {
    fn from(value: P) -> Self {
        Self(value)
    }
}
impl<K, S, V, P: FnMut(&Branch<K, S, V>) -> I, I: IntoIterator<Item = NodeId<K, S, V>>> Way<K, S, V>
    for Predicated<P>
{
    fn find(&mut self, branch: &Branch<K, S, V>) -> impl IntoIterator<Item = NodeId<K, S, V>> {
        self.0(branch)
    }
}
impl<K, S, V, W: Way<K, S, V>> Walk<K, S, V, W> {
    pub fn start(root: &Option<NodeId<K, S, V>>, way: W) -> Self {
        Self {
            stack: Vec::from_iter(root.as_ref().map(Handle::leak)),
            way,
            #[cfg(debug_assertions)]
            unique: std::collections::HashSet::from_iter(root.as_ref().map(Handle::leak)),
        }
    }
    pub fn next(&mut self, shared: &Slab<Node<K, S, V>>) -> Option<NodeId<K, S, V>> {
        let node = self.stack.pop()?;
        let branch = node.get(&shared).as_branch();
        for x in branch
            .map(|branch| self.way.find(branch))
            .into_iter()
            .flatten()
        {
            debug_assert!(self.unique.insert(x.leak()));
            self.stack.push(x);
        }
        Some(node)
    }
}
