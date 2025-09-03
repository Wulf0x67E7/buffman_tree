use slab::Slab;

use crate::{Branch, Node, handle::Handle};

pub struct Walk<K, S, V, F> {
    stack: Vec<Handle<Node<K, S, V>>>,
    filter: F,
    #[cfg(debug_assertions)]
    unique: std::collections::HashSet<Handle<Node<K, S, V>>, std::hash::RandomState>,
}
impl<K, S, V> Walk<K, S, V, ()> {
    pub fn next(&mut self, shared: &Slab<Node<K, S, V>>) -> Option<Handle<Node<K, S, V>>> {
        let node = self.stack.pop()?;
        let branch = node.get(&shared).as_branch();
        for x in branch
            .into_iter()
            .flat_map(|branch| branch.children().rev().map(Handle::leak))
        {
            debug_assert!(self.unique.insert(x.leak()));
            self.stack.push(x);
        }
        Some(node)
    }
}
impl<K, S, V, F> Walk<K, S, V, F> {
    pub fn start(root: &Option<Handle<Node<K, S, V>>>, filter: F) -> Self {
        Self {
            stack: Vec::from_iter(root.as_ref().map(Handle::leak)),
            filter,
            #[cfg(debug_assertions)]
            unique: std::collections::HashSet::from_iter(root.as_ref().map(Handle::leak)),
        }
    }
    pub fn peek(&self) -> Option<Handle<Node<K, S, V>>> {
        self.stack.last().map(Handle::leak)
    }
    pub fn next_by_key(&mut self, shared: &Slab<Node<K, S, V>>) -> Option<Handle<Node<K, S, V>>>
    where
        F: Iterator<Item = S>,
        S: Ord,
    {
        let node = self.stack.pop()?;
        let branch = node.get(&shared).as_branch();
        for x in branch.into_iter().flat_map(|branch| {
            self.filter
                .next()
                .and_then(|key| branch.get_handle(key).map(Handle::leak))
        }) {
            debug_assert!(self.unique.insert(x.leak()));
            self.stack.push(x);
        }
        Some(node)
    }

    pub fn next_by_filter<I: IntoIterator<Item = Handle<Node<K, S, V>>>>(
        &mut self,
        shared: &Slab<Node<K, S, V>>,
    ) -> Option<Handle<Node<K, S, V>>>
    where
        F: for<'a> FnMut(&'a Branch<K, S, V>) -> I,
    {
        let node = self.stack.pop()?;
        let branch = node.get(&shared).as_branch();
        for x in branch.into_iter().flat_map(&mut self.filter) {
            debug_assert!(self.unique.insert(x.leak()));
            self.stack.push(x);
        }
        Some(node)
    }
}
