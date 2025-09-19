use crate::{
    trie::{
        branch::Branch,
        handle::{Handle, Shared},
        node::{Node, NodeHandle},
        vnode::VNode,
    },
    util::OptExt as _,
};
pub(self) mod branch;
pub(self) mod handle;
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
        let mut nodes = Handle::new_shared();
        Self {
            root: Handle::new_default(&mut nodes),
            nodes,
            branches: Handle::new_shared(),
            leaves: Handle::new_shared(),
        }
    }
}
impl<K, V> Trie<K, V> {
    pub fn with_capacity(capacity: usize) -> Self {
        let mut nodes = Handle::new_shared_with_capacity(capacity);
        Self {
            root: Handle::new_default(&mut nodes),
            nodes,
            branches: Handle::new_shared_with_capacity(capacity),
            leaves: Handle::new_shared_with_capacity(capacity),
        }
    }
}
impl<K: IntoIterator<Item: Ord>, V> FromIterator<(K, V)> for Trie<K::Item, V> {
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        let iter = iter.into_iter();
        let capacity = {
            let hint = iter.size_hint();
            hint.1.unwrap_or(hint.0)
        };
        let mut this = Self::with_capacity(capacity);
        for (k, v) in iter {
            this.insert(k, v);
        }
        this
    }
}
impl<K: Clone + IntoIterator<Item: Ord>, V> FromIterator<(K, V)> for Trie<K::Item, (K, V)> {
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        let iter = iter.into_iter();
        let capacity = {
            let hint = iter.size_hint();
            hint.1.unwrap_or(hint.0)
        };
        let mut this = Self::with_capacity(capacity);
        for (k, v) in iter {
            this.insert(k.clone(), (k, v));
        }
        this
    }
}
impl<K: Ord, V: PartialEq> PartialEq for Trie<K, V> {
    fn eq(&self, other: &Self) -> bool {
        self.iter().eq(other.iter())
    }
}
impl<K: PartialEq + Ord, V> Trie<K, V> {
    pub fn is_empty(&self) -> bool {
        let empty_shallow = self.root.get(&self.nodes).is_empty();
        debug_assert_eq!(
            empty_shallow,
            self.branches.is_empty() && self.leaves.is_empty() && self.nodes.len() == 1,
            "{} == 0 && {} == 0 && {} == 1",
            self.branches.len(),
            self.leaves.len(),
            self.nodes.len()
        );
        empty_shallow
    }
    pub fn insert(&mut self, key: impl IntoIterator<Item = K>, value: V) -> Option<V> {
        VNode::start(self.root.leak())
            .make_descend(self, key)
            .make_leaf(self, value)
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
        match self.try_get_handle(key) {
            Ok(node) => Ok(node.get_mut(&mut self.leaves)),
            Err(Some(node)) => Err(Some(node.get_mut(&mut self.leaves))),
            Err(None) => Err(None),
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
        VNode::start(self.root.leak())
            .dive(
                self,
                key,
                |node, this, _| Some(node.as_node(&this.nodes).is_some()),
                |node, this| {
                    let (node, ret) = node.take_leaf(this)?;
                    node.prune_branch(this);
                    Some(ret)
                },
                |node, this, _| node.prune_branch(this).is_some(),
            )
            .ok()
    }
    pub fn into_iter(self) -> impl Iterator<Item = V>
    where
        (K, V): 'static,
    {
        VNode::start(self.root.leak()).into_iter(self)
    }
    pub fn iter(&self) -> impl Iterator<Item = &V> {
        VNode::start(self.root.leak()).iter(self)
    }
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut V> {
        VNode::start(self.root.leak()).iter_mut(self)
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
            .descend(self, key, |_, _, _| true)
            .ok()?
            .leaf_handle(self)
    }
    fn try_get_handle<'a, Q: 'a + PartialEq + Ord>(
        &self,
        key: impl IntoIterator<Item = &'a Q>,
    ) -> Result<LeafHandle<V>, Option<LeafHandle<V>>>
    where
        K: Borrow<Q>,
    {
        VNode::start(self.root.leak())
            .find(self, key, |node, this| Err(node.leaf_handle(this)))
            .map_err(|node| node.leaf_handle(self))
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
        let values = BTreeMap::from_iter([
            (vec![], "_".into()),
            (vec![1], "1".into()),
            (vec![1, 0], "10".into()),
        ]);
        let searches = vec![vec![]];
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
        for (a, b) in btree.values().zip(trie.iter()) {
            assert_eq!(a, b, "failed iter {a:?} != {b:?}\n{trie:?}");
        }
        for key in &searches {
            assert_eq!(btree.remove(key), trie.remove(key), "failed remove {key:?}");
            assert_eq!(
                btree.is_empty(),
                trie.is_empty(),
                "failed is empty\n{trie:?}"
            );
        }

        assert!(trie.is_empty(), "failed is empty\n{trie:?}");
        assert_eq!(trie, Trie::default());
    }
}
