use crate::{
    handle::Shared,
    trie::handle::Handle,
    trie2::{
        branch::Branch,
        node::{Node, NodeHandle},
        vnode::VNode,
    },
    util::OptExt as _,
};
pub(self) mod branch;
pub(self) mod node;
#[cfg(test)]
pub(crate) mod testing;
pub(self) mod vnode;
use std::{borrow::Borrow, convert::identity, fmt::Debug};

pub(self) type LeafHandle<V> = Handle<V>;

pub struct Trie<K, V> {
    root: NodeHandle<K, V>,
    nodes: Shared<Node<K, V>>,
    branches: Shared<Branch<K, V>>,
    leaves: Shared<V>,
}
impl<K: Debug, V: Debug> Debug for Trie<K, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Trie")
            .field(&self.root.get(&self.nodes).node_debug(&self))
            .finish()
    }
}
impl<K, V> Default for Trie<K, V> {
    fn default() -> Self {
        let mut nodes = Default::default();
        Self {
            root: Handle::new_default(&mut nodes),
            nodes,
            branches: Default::default(),
            leaves: Default::default(),
        }
    }
}

impl<K: PartialEq + Ord, V> Trie<K, V> {
    pub fn is_empty(&self) -> bool {
        let empty_shallow = self.root.get(&self.nodes).is_empty(&self.branches);
        debug_assert_eq!(
            empty_shallow,
            self.branches.is_empty() && self.leaves.is_empty() && self.nodes.len() == 1
        );
        empty_shallow
    }
    pub fn insert(&mut self, key: impl IntoIterator<Item = K>, value: V) -> Option<V> {
        VNode::start(self.root.leak())
            .make_descend(&mut self.nodes, &mut self.branches, key)
            .make_leaf(&mut self.nodes, &mut self.branches, &mut self.leaves, value)
    }
    pub fn get<'a, Q: 'a + PartialEq + Ord>(
        &self,
        key: impl IntoIterator<Item = &'a Q>,
    ) -> Option<&V>
    where
        K: Borrow<Q>,
    {
        Some(self.get_handle(key)?.get(&self.leaves))
    }
    pub fn get_mut<'a, Q: 'a + PartialEq + Ord>(
        &mut self,
        key: impl IntoIterator<Item = &'a Q>,
    ) -> Option<&mut V>
    where
        K: Borrow<Q>,
    {
        Some(self.get_handle(key)?.get_mut(&mut self.leaves))
    }
    pub fn try_get<'a, Q: 'a + PartialEq + Ord>(
        &self,
        key: impl IntoIterator<Item = &'a Q>,
    ) -> Result<&V, Option<&V>>
    where
        K: Borrow<Q>,
    {
        Ok(self
            .try_get_handle(key)
            .map_err(Option::remap(|leaf: Handle<V>| leaf.get(&self.leaves)))?
            .get(&self.leaves))
    }
    pub fn try_get_mut<'a, Q: 'a + PartialEq + Ord>(
        &mut self,
        key: impl IntoIterator<Item = &'a Q>,
    ) -> Result<&mut V, Option<&mut V>>
    where
        K: Borrow<Q>,
    {
        match VNode::start(self.root.leak()).find(&self.nodes, &self.branches, key, |node| {
            Err(node.leaf(&self.nodes, &self.leaves).map(|_| node))
        }) {
            Ok(node) => Ok(node.leaf_mut(&self.nodes, &mut self.leaves).unwrap()),
            Err(node) => Err(node.leaf_mut(&self.nodes, &mut self.leaves)),
        }
    }
    pub fn get_deepest<'a, Q: 'a + PartialEq + Ord>(
        &self,
        key: impl IntoIterator<Item = &'a Q>,
    ) -> Option<&V>
    where
        K: Borrow<Q>,
    {
        self.try_get(key).map_or_else(identity, Option::Some)
    }
    pub fn get_deepest_mut<'a, Q: 'a + PartialEq + Ord>(
        &mut self,
        key: impl IntoIterator<Item = &'a Q>,
    ) -> Option<&mut V>
    where
        K: Borrow<Q>,
    {
        self.try_get_mut(key).map_or_else(identity, Option::Some)
    }
    pub fn remove<'a, Q: 'a + PartialEq + Ord>(
        &mut self,
        key: impl IntoIterator<Item = &'a Q>,
    ) -> Option<V>
    where
        K: Borrow<Q>,
    {
        let mut stack = vec![];
        let ret = VNode::start(self.root.leak())
            .descend(&self.nodes, &self.branches, key, |node| {
                stack.push(node);
            })
            .ok()?
            .take_leaf(&mut self.nodes, &mut self.leaves)?;
        stack.reverse();
        for node in stack {
            if !node.prune_branch(&mut self.nodes, &mut self.branches) {
                break;
            }
        }
        Some(ret)
    }
}

impl<K: Ord, V> Trie<K, V> {
    fn get_handle<'a, Q: 'a + PartialEq + Ord>(
        &self,
        key: impl IntoIterator<Item = &'a Q>,
    ) -> Option<Handle<V>>
    where
        K: Borrow<Q>,
    {
        VNode::start(self.root.leak())
            .descend(&self.nodes, &self.branches, key, |_| ())
            .ok()?
            .leaf_handle(&self.nodes)
    }
    fn try_get_handle<'a, Q: 'a + PartialEq + Ord>(
        &self,
        key: impl IntoIterator<Item = &'a Q>,
    ) -> Result<LeafHandle<V>, Option<LeafHandle<V>>>
    where
        K: Borrow<Q>,
    {
        VNode::start(self.root.leak())
            .find(&self.nodes, &self.branches, key, |node| {
                Err(node.leaf_handle(&self.nodes))
            })
            .map_err(|node| node.leaf_handle(&self.nodes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::BTrie;
    use quickcheck_macros::quickcheck;
    use std::collections::BTreeMap;

    #[quickcheck]
    fn insert_get_remove_fuzz(values: BTreeMap<Vec<u8>, String>, searches: Vec<Vec<u8>>) {
        insert_get(values, searches);
    }
    #[test]
    fn insert_get_case() {
        let values = BTreeMap::from_iter([(vec![], "".into())]);
        let searches = vec![vec![0]];
        insert_get(values, searches);
    }
    fn insert_get(values: BTreeMap<Vec<u8>, String>, mut searches: Vec<Vec<u8>>) {
        let mut btree = BTreeMap::default();
        let mut trie = Trie::default();
        searches.extend(values.keys().cloned());
        for (key, value) in values {
            assert_eq!(
                btree.insert(key.clone(), value.clone()),
                trie.insert(key.clone(), value.clone()),
                "failed insert {key:?}: {value}"
            );
        }
        for search in &searches {
            assert_eq!(btree.get(search), trie.get(search), "failed get {search:?}");
            assert_eq!(
                btree.get_deepest(&**search),
                trie.get_deepest(search),
                "failed get deepest {search:?}\n{trie:?}"
            );
        }
        for key in &searches {
            assert_eq!(btree.remove(key), trie.remove(key), "failed remove {key:?}");
        }
        for search in &searches {
            assert_eq!(btree.get(search), trie.get(search), "failed get {search:?}");
            assert_eq!(
                btree.get_deepest(&**search),
                trie.get_deepest(search),
                "failed get deepest {search:?}\n{trie:?}"
            );
        }
        assert!(trie.is_empty(), "{trie:?}");
        //assert_eq!(trie, Trie::default());
    }
}
