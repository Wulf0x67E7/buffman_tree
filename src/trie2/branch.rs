use crate::{
    handle::Shared,
    trie2::{
        Handle, Trie,
        node::{Node, NodeHandle},
    },
};
use std::{
    borrow::Borrow,
    collections::{BTreeMap, btree_map},
    fmt::Debug,
};

#[derive(Debug)]
pub struct Branch<K, V>(BTreeMap<K, NodeHandle<K, V>>);
impl<K: Debug, V: Debug> Branch<K, V> {
    pub(crate) fn branch_debug<'a>(&'a self, trie: &'a Trie<K, V>) -> impl 'a + Debug {
        struct BranchDebug<'a, K, V>(&'a Branch<K, V>, &'a Trie<K, V>);
        impl<'a, K: Debug, V: Debug> Debug for BranchDebug<'a, K, V> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let mut f = f.debug_list();
                f.entries(
                    self.0
                        .0
                        .iter()
                        .map(|(k, v)| (k, v.get(&self.1.nodes).node_debug(self.1))),
                );
                f.finish()
            }
        }
        BranchDebug(self, trie)
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
            .or_insert(Handle::new(shared, Node::from(vec![], ())))
            .leak()
    }
    pub fn get<Q: Ord>(&self, key: &Q) -> Option<NodeHandle<K, V>>
    where
        K: Borrow<Q>,
    {
        self.0.get(key).map(Handle::leak)
    }
    pub fn prune(&mut self, nodes: &mut Shared<Node<K, V>>) -> bool {
        self.0.retain(|_, node| {
            if node.get(nodes).is_empty_node() {
                node.leak().remove(nodes);
                false
            } else {
                true
            }
        });
        self.is_empty()
    }

    pub fn keys(&self) -> btree_map::Keys<'_, K, NodeHandle<K, V>> {
        self.0.keys()
    }
    pub fn values(&self) -> btree_map::Values<'_, K, NodeHandle<K, V>> {
        self.0.values()
    }
}
pub type BranchHandle<K, V> = Handle<Branch<K, V>>;
