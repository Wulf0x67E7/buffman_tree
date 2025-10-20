use crate::{
    NodeDebug,
    branch::Branch,
    trie::{
        Handle, Trie,
        node::{Node, NodeHandle},
    },
    util::debug_fn,
};
use std::{borrow::Borrow, collections::BTreeMap, fmt::Debug, mem::take};
#[derive(Debug)]
pub struct BTreeBranch<K, V> {
    map: BTreeMap<K, NodeHandle<K, V, Self>>,
    #[cfg(feature = "testing")]
    owner: NodeHandle<K, V, Self>,
}
impl<K, V> Default for BTreeBranch<K, V> {
    fn default() -> Self {
        Self {
            map: Default::default(),
            #[cfg(feature = "testing")]
            owner: NodeHandle::new_null(),
        }
    }
}
impl<K, V> NodeDebug<K, V, Self> for BTreeBranch<K, V> {
    fn default_with_owner(#[cfg(feature = "testing")] owner: NodeHandle<K, V, Self>) -> Self {
        Self {
            map: Default::default(),
            #[cfg(feature = "testing")]
            owner,
        }
    }
    fn debug<'a>(&'a self, trie: &'a Trie<K, V, Self>) -> impl 'a + Debug
    where
        K: Debug,
        V: Debug,
    {
        debug_fn(|f| {
            let mut f = f.debug_list();
            f.entries(
                self.map
                    .iter()
                    .map(|(k, v)| (k, v.get(&trie.nodes).debug(trie))),
            );
            f.finish()
        })
    }
    #[cfg(feature = "testing")]
    fn set_owner(&mut self, owner: NodeHandle<K, V, Self>) -> NodeHandle<K, V, Self> {
        use std::mem::replace;
        replace(&mut self.owner, owner)
    }
}
impl<K: Ord + Borrow<Q>, V, Q: Ord> Branch<K, V, Q> for BTreeBranch<K, V> {
    fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
    fn insert(&mut self, key: K, node: NodeHandle<K, V, Self>) -> Option<NodeHandle<K, V, Self>> {
        self.map.insert(key, node)
    }
    fn get_or_insert_with(
        &mut self,
        key: K,
        f: impl FnOnce() -> NodeHandle<K, V, Self>,
    ) -> NodeHandle<K, V, Self> {
        self.map.entry(key).or_insert_with(f).leak()
    }
    fn get(&self, key: &Q) -> Option<NodeHandle<K, V, Self>> {
        self.map.get(key).map(Handle::leak)
    }
    fn cleanup(&mut self, mut f: impl FnMut(&mut NodeHandle<K, V, Self>) -> bool) -> usize {
        self.map.retain(|_, node| !f(node));
        self.map.len()
    }
    fn prune(
        &mut self,
        f: impl FnMut(&mut NodeHandle<K, V, Self>) -> bool,
    ) -> Option<Option<(K, NodeHandle<K, V, Self>)>> {
        match self.cleanup(f) {
            0 => Some(None),
            1 => Some(take(&mut self.map).into_iter().last()),
            _ => None,
        }
    }

    fn iter<'a>(&'a self) -> impl Iterator<Item = (&'a K, Handle<Node<K, V, Self>>)>
    where
        K: 'a,
    {
        self.map.iter().map(|(k, v)| (k, v.leak()))
    }
}
