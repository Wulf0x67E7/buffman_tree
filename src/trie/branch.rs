use crate::{
    NodeDebug,
    trie::{
        Handle, Trie,
        handle::Shared,
        node::{Node, NodeHandle},
    },
    util::debug_fn,
};
use std::{borrow::Borrow, collections::BTreeMap, fmt::Debug, mem::take};

pub trait Branch<K, V, Q = K>: NodeDebug<K, V> {
    fn is_empty(&self) -> bool;
    fn insert(&mut self, key: K, node: NodeHandle<K, V>) -> Option<NodeHandle<K, V>>;
    fn get_or_insert(
        &mut self,
        this: NodeHandle<K, V>,
        shared: &mut Shared<Node<K, V>>,
        key: K,
    ) -> NodeHandle<K, V>;
    fn get(&self, key: &Q) -> Option<NodeHandle<K, V>>;
    fn cleanup(&mut self, nodes: &mut Shared<Node<K, V>>) -> usize;
    fn prune(&mut self, nodes: &mut Shared<Node<K, V>>) -> Option<Option<(K, NodeHandle<K, V>)>>;

    fn iter<'a>(&'a self) -> impl Iterator<Item = (&'a K, NodeHandle<K, V>)>
    where
        K: 'a;
    fn keys<'a>(&'a self) -> impl Iterator<Item = &'a K>
    where
        K: 'a,
    {
        self.iter().map(|(k, _)| k)
    }
    fn values<'a>(&'a self) -> impl Iterator<Item = NodeHandle<K, V>>
    where
        K: 'a,
    {
        self.iter().map(|(_, v)| v)
    }
}

#[derive(Debug)]
pub struct BTreeBranch<K, V> {
    map: BTreeMap<K, NodeHandle<K, V>>,
    #[cfg(feature = "testing")]
    owner: NodeHandle<K, V>,
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
impl<K, V> NodeDebug<K, V> for BTreeBranch<K, V> {
    fn default_with_owner(#[cfg(feature = "testing")] owner: NodeHandle<K, V>) -> Self {
        Self {
            map: Default::default(),
            #[cfg(feature = "testing")]
            owner,
        }
    }
    fn debug<'a>(&'a self, trie: &'a Trie<K, V>) -> impl 'a + Debug
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
    fn set_owner(&mut self, owner: NodeHandle<K, V>) -> NodeHandle<K, V> {
        use std::mem::replace;
        replace(&mut self.owner, owner)
    }
}
impl<K: Ord + Borrow<Q>, V, Q: Ord> Branch<K, V, Q> for BTreeBranch<K, V> {
    fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
    fn insert(&mut self, key: K, node: NodeHandle<K, V>) -> Option<NodeHandle<K, V>> {
        self.map.insert(key, node)
    }
    fn get_or_insert(
        &mut self,
        this: NodeHandle<K, V>,
        shared: &mut Shared<Node<K, V>>,
        key: K,
    ) -> NodeHandle<K, V> {
        #[cfg(feature = "testing")]
        assert_eq!(self.owner, this);
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
    fn get(&self, key: &Q) -> Option<NodeHandle<K, V>> {
        self.map.get(key).map(Handle::leak)
    }
    fn cleanup(&mut self, nodes: &mut Shared<Node<K, V>>) -> usize {
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
    fn prune(&mut self, nodes: &mut Shared<Node<K, V>>) -> Option<Option<(K, NodeHandle<K, V>)>> {
        match self.cleanup(nodes) {
            0 => Some(None),
            1 => Some(take(&mut self.map).into_iter().last()),
            _ => None,
        }
    }

    fn iter<'a>(&'a self) -> impl Iterator<Item = (&'a K, Handle<Node<K, V>>)>
    where
        K: 'a,
    {
        // .rev() needed to ensure correct iteration order of full trie.
        // see: vnode iteration pushes to node stack
        self.map.iter().rev().map(|(k, v)| (k, v.leak()))
    }
}
pub type BranchHandle<K, V> = Handle<BTreeBranch<K, V>>;
