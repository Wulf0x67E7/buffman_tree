use crate::{
    trie::{
        Handle, Trie,
        handle::Shared,
        node::{Node, NodeHandle},
    },
    util::debug_fn,
};
use std::{
    borrow::Borrow,
    collections::{BTreeMap, btree_map},
    fmt::Debug,
    mem::take,
};

#[derive(Debug)]
pub struct Branch<K, V>(BTreeMap<K, NodeHandle<K, V>>);
impl<K: Debug, V: Debug> Branch<K, V> {
    pub(crate) fn branch_debug<'a>(&'a self, trie: &'a Trie<K, V>) -> impl 'a + Debug {
        debug_fn(|f| {
            let mut f = f.debug_list();
            f.entries(
                self.0
                    .iter()
                    .map(|(k, v)| (k, v.get(&trie.nodes).node_debug(trie))),
            );
            f.finish()
        })
    }
}
impl<K, V> Default for Branch<K, V> {
    fn default() -> Self {
        Self(Default::default())
    }
}
impl<K, V> Branch<K, V> {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}
impl<K: Ord, V> Branch<K, V> {
    pub fn insert(&mut self, key: K, node: NodeHandle<K, V>) -> Option<NodeHandle<K, V>> {
        self.0.insert(key, node)
    }
    pub fn get_or_insert(&mut self, shared: &mut Shared<Node<K, V>>, key: K) -> NodeHandle<K, V> {
        self.0
            .entry(key)
            .or_insert_with(|| Handle::new(shared, Node::from(vec![], ())))
            .leak()
    }
    pub fn get<Q: Ord>(&self, key: &Q) -> Option<NodeHandle<K, V>>
    where
        K: Borrow<Q>,
    {
        self.0.get(key).map(Handle::leak)
    }
    pub fn cleanup(&mut self, nodes: &mut Shared<Node<K, V>>) -> usize {
        self.0.retain(|_, node| {
            if node.get(nodes).is_empty() {
                node.leak().remove(nodes);
                false
            } else {
                true
            }
        });
        self.0.len()
    }
    pub fn prune(
        &mut self,
        nodes: &mut Shared<Node<K, V>>,
    ) -> Option<Option<(K, NodeHandle<K, V>)>> {
        match self.cleanup(nodes) {
            0 => Some(None),
            1 => Some(take(&mut self.0).into_iter().last()),
            _ => None,
        }
    }

    pub fn values(&self) -> btree_map::Values<'_, K, NodeHandle<K, V>> {
        self.0.values()
    }
}
pub type BranchHandle<K, V> = Handle<Branch<K, V>>;
