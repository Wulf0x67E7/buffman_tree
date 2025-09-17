use crate::{
    handle::{Handle, Shared},
    trie2::{
        LeafHandle,
        branch::{Branch, BranchHandle},
        node::{Node, NodeHandle},
    },
};
use std::{borrow::Borrow, cmp::Ordering, fmt::Debug};

pub struct VNode<K, V> {
    prefix_len: usize,
    handle: NodeHandle<K, V>,
}
impl<K, V> Debug for VNode<K, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VNode")
            .field("prefix_len", &self.prefix_len)
            .field("handle", &self.handle)
            .finish()
    }
}
impl<K: Ord, V> VNode<K, V> {
    pub fn start(root: NodeHandle<K, V>) -> Self {
        Self {
            prefix_len: 0,
            handle: root,
        }
    }
    pub fn leak(&self) -> Self {
        Self {
            prefix_len: self.prefix_len,
            handle: self.handle.leak(),
        }
    }
    pub fn next<Q: PartialEq + Ord>(
        &self,
        nodes: &Shared<Node<K, V>>,
        branches: &Shared<Branch<K, V>>,
        key: &Q,
    ) -> Option<Self>
    where
        K: Borrow<Q>,
    {
        let node = self.handle.get(nodes);
        match self.prefix_len.cmp(&node.prefix().len()) {
            Ordering::Less => {
                if node.prefix()[self.prefix_len].borrow() == key {
                    Some(Self {
                        prefix_len: self.prefix_len + 1,
                        handle: self.handle.leak(),
                    })
                } else {
                    None
                }
            }
            Ordering::Equal => {
                let handle = node.get_branch(branches)?.get(key)?;
                Some(Self {
                    prefix_len: 0,
                    handle,
                })
            }
            Ordering::Greater => unreachable!(),
        }
    }
    pub fn make_next(
        &self,
        nodes: &mut Shared<Node<K, V>>,
        branches: &mut Shared<Branch<K, V>>,
        key: K,
    ) -> Self {
        if let Some(next) = self.next(nodes, branches, &key) {
            return next;
        }
        let node = self.handle.get_mut(nodes);
        if node.is_empty(branches) {
            debug_assert_eq!(self.prefix_len, node.prefix().len());
            node.prefix_mut().push(key);
            Self {
                prefix_len: node.prefix().len(),
                handle: self.handle.leak(),
            }
        } else {
            Self::start(
                self.make_branch(nodes, branches)
                    .get_mut(branches)
                    .get_or_insert(nodes, key),
            )
        }
    }
    pub fn descend<'a, Q: 'a + PartialEq + Ord>(
        &self,
        nodes: &Shared<Node<K, V>>,
        branches: &Shared<Branch<K, V>>,
        key: impl IntoIterator<Item = &'a Q>,
        mut inspect: impl FnMut(Self),
    ) -> Result<Self, Self>
    where
        K: Borrow<Q>,
    {
        let mut node = self.leak();
        let mut key = key.into_iter();
        loop {
            inspect(node.leak());
            if let Some(key) = key.next() {
                node = node.next(nodes, branches, key).ok_or(node)?;
            } else {
                break Ok(node);
            }
        }
    }
    pub fn make_descend(
        &self,
        nodes: &mut Shared<Node<K, V>>,
        branches: &mut Shared<Branch<K, V>>,
        key: impl IntoIterator<Item = K>,
    ) -> Self {
        key.into_iter().fold(self.leak(), |node, key| {
            node.make_next(nodes, branches, key)
        })
    }
    pub fn find<'a, Q: 'a + PartialEq + Ord, T>(
        &self,
        nodes: &Shared<Node<K, V>>,
        branches: &Shared<Branch<K, V>>,
        key: impl IntoIterator<Item = &'a Q>,
        mut f: impl FnMut(Self) -> Result<T, Option<T>>,
    ) -> Result<T, Self>
    where
        K: Borrow<Q>,
    {
        let mut node = self.leak();
        let mut backup = None;
        let mut key = key.into_iter();
        loop {
            match f(node.leak()) {
                Ok(t) => break Ok(t),
                Err(t @ Some(_)) => backup = t,
                Err(None) => (),
            }
            if let Some(key) = key.next()
                && let Some(next) = node.next(nodes, branches, key)
            {
                node = next;
            } else {
                break backup.ok_or(node.leak());
            }
        }
    }
    pub fn make_leaf(
        &self,
        nodes: &mut Shared<Node<K, V>>,
        branches: &mut Shared<Branch<K, V>>,
        leaves: &mut Shared<V>,
        value: V,
    ) -> Option<V> {
        let node = self.handle.get_mut(nodes);
        match self.prefix_len.cmp(&node.prefix().len()) {
            Ordering::Less => {
                let (value, new_node) = node.make_leaf_at(branches, leaves, value, self.prefix_len);
                if let Some((key, new_node)) = new_node {
                    node.get_branch_mut(branches)
                        .unwrap()
                        .insert(key, Handle::new(nodes, new_node));
                }
                value
            }
            Ordering::Equal => node.make_leaf(leaves, value),
            Ordering::Greater => unreachable!(),
        }
    }
    pub fn leaf_handle(&self, nodes: &Shared<Node<K, V>>) -> Option<LeafHandle<V>> {
        let node = self.handle.get(nodes);
        if self.prefix_len == node.prefix().len() {
            node.leaf()
        } else {
            None
        }
    }
    pub fn leaf<'a>(&self, nodes: &Shared<Node<K, V>>, leaves: &'a Shared<V>) -> Option<&'a V> {
        let node = self.handle.get(nodes);
        if self.prefix_len == node.prefix().len() {
            node.get_leaf(leaves)
        } else {
            None
        }
    }
    pub fn leaf_mut<'a>(
        &self,
        nodes: &Shared<Node<K, V>>,
        leaves: &'a mut Shared<V>,
    ) -> Option<&'a mut V> {
        let node = self.handle.get(nodes);
        if self.prefix_len == node.prefix().len() {
            node.get_leaf_mut(leaves)
        } else {
            None
        }
    }
    pub fn take_leaf(&self, nodes: &mut Shared<Node<K, V>>, leaves: &mut Shared<V>) -> Option<V> {
        let node = self.handle.get_mut(nodes);
        if self.prefix_len == node.prefix().len() {
            node.take_leaf(leaves)
        } else {
            None
        }
    }
    pub fn make_branch(
        &self,
        nodes: &mut Shared<Node<K, V>>,
        branches: &mut Shared<Branch<K, V>>,
    ) -> BranchHandle<K, V> {
        let node = self.handle.get_mut(nodes);
        match self.prefix_len.cmp(&node.prefix().len()) {
            Ordering::Less => {
                let (branch, new_node) = node.make_branch_at(branches, self.prefix_len);
                if let Some((key, new_node)) = new_node {
                    node.get_branch_mut(branches)
                        .unwrap()
                        .insert(key, Handle::new(nodes, new_node));
                }
                branch
            }
            Ordering::Equal => node.make_branch(branches),
            Ordering::Greater => unreachable!(),
        }
    }
    pub fn prune_branch(
        &self,
        nodes: &mut Shared<Node<K, V>>,
        branches: &mut Shared<Branch<K, V>>,
    ) -> bool {
        if self.prefix_len != self.handle.get(nodes).prefix().len() {
            return true;
        }
        if let Some(branch) = self.handle.get(nodes).branch() {
            if !branch.get_mut(branches).prune(nodes) {
                return false;
            }
        }
        if let Some(branch) = self.handle.get_mut(nodes).take_branch() {
            let branch = branch.remove(branches);
            debug_assert!(branch.is_empty())
        }
        self.handle.get(nodes).is_empty_node()
    }
}
