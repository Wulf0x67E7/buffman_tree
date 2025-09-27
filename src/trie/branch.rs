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
pub struct Branch<K, V> {
    map: BTreeMap<K, NodeHandle<K, V>>,
    #[cfg(feature = "testing")]
    this: NodeHandle<K, V>,
}
impl<K: Debug, V: Debug> Branch<K, V> {
    pub(crate) fn branch_debug<'a>(&'a self, trie: &'a Trie<K, V>) -> impl 'a + Debug {
        debug_fn(|f| {
            let mut f = f.debug_list();
            f.entries(
                self.map
                    .iter()
                    .map(|(k, v)| (k, v.get(&trie.nodes).node_debug(trie))),
            );
            f.finish()
        })
    }
}
//impl<K, V> Default for Branch<K, V> {
//    fn default() -> Self {
//        Self {
//            map: Default::default(),
//            #[cfg(feature = "testing")]
//            this: Handle::new_null(),
//        }
//    }
//}
impl<K, V> Branch<K, V> {
    pub fn new(#[cfg(feature = "testing")] this: NodeHandle<K, V>) -> Self {
        Self {
            map: Default::default(),
            #[cfg(feature = "testing")]
            this,
        }
    }
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
    #[cfg(feature = "testing")]
    pub fn set_this(&mut self, this: NodeHandle<K, V>) -> NodeHandle<K, V> {
        use std::mem::replace;
        replace(&mut self.this, this)
    }
}
impl<K: Ord, V> Branch<K, V> {
    pub fn insert(&mut self, key: K, node: NodeHandle<K, V>) -> Option<NodeHandle<K, V>> {
        self.map.insert(key, node)
    }
    pub fn get_or_insert(
        &mut self,
        this: NodeHandle<K, V>,
        shared: &mut Shared<Node<K, V>>,
        key: K,
    ) -> NodeHandle<K, V> {
        #[cfg(feature = "testing")]
        assert_eq!(self.this, this);
        self.map
            .entry(key)
            .or_insert_with(|| {
                Handle::new_with(shared, |_t| {
                    Node::from(
                        #[cfg(feature = "testing")]
                        _t,
                        this,
                        vec![],
                        (),
                    )
                })
            })
            .leak()
    }
    pub fn get<Q: Ord>(&self, key: &Q) -> Option<NodeHandle<K, V>>
    where
        K: Borrow<Q>,
    {
        self.map.get(key).map(Handle::leak)
    }
    pub fn cleanup(&mut self, nodes: &mut Shared<Node<K, V>>) -> usize {
        self.map.retain(|_, node| {
            if node.get(nodes).is_empty() {
                node.leak().remove(nodes);
                false
            } else {
                true
            }
        });
        self.map.len()
    }
    pub fn prune(
        &mut self,
        nodes: &mut Shared<Node<K, V>>,
    ) -> Option<Option<(K, NodeHandle<K, V>)>> {
        match self.cleanup(nodes) {
            0 => Some(None),
            1 => Some(take(&mut self.map).into_iter().last()),
            _ => None,
        }
    }

    pub fn values(&self) -> btree_map::Values<'_, K, NodeHandle<K, V>> {
        self.map.values()
    }
}
pub type BranchHandle<K, V> = Handle<Branch<K, V>>;
