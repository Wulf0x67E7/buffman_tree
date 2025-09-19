use crate::{
    handle::{Handle, Shared},
    trie2::{
        LeafHandle, Trie,
        branch::{Branch, BranchHandle},
        node::{Node, NodeHandle},
    },
};
use std::{
    borrow::Borrow,
    cmp::Ordering,
    fmt::Debug,
    iter,
    mem::{transmute, transmute_copy},
};

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
impl<K, V> PartialEq for VNode<K, V> {
    fn eq(&self, other: &Self) -> bool {
        self.prefix_len == other.prefix_len && self.handle == other.handle
    }
}
/// Navigation methods
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
    pub fn next<Q: PartialEq + Ord>(&self, trie: &Trie<K, V>, key: &Q) -> Option<Self>
    where
        K: Borrow<Q>,
    {
        match self.as_node(&trie.nodes) {
            None if self.handle.get(&trie.nodes).prefix()[self.prefix_len].borrow() == key => {
                Some(Self {
                    prefix_len: self.prefix_len + 1,
                    handle: self.handle.leak(),
                })
            }
            Some(node) => Some(Self {
                prefix_len: 0,
                handle: node.get_branch(&trie.branches)?.get(key)?,
            }),
            None => None,
        }
    }
    pub fn make_next(&self, trie: &mut Trie<K, V>, key: K) -> Self {
        if let Some(next) = self.next(trie, &key) {
            return next;
        }
        let node = self.handle.get_mut(&mut trie.nodes);
        if node.is_empty_node() {
            node.prefix_mut().push(key);
            Self {
                prefix_len: node.prefix().len(),
                handle: self.handle.leak(),
            }
        } else {
            Self::start(
                self.make_branch(trie)
                    .get_mut(&mut trie.branches)
                    .get_or_insert(&mut trie.nodes, key),
            )
        }
    }
    pub fn make_descend(&self, trie: &mut Trie<K, V>, key: impl IntoIterator<Item = K>) -> Self {
        let mut node = self.leak();
        let mut key = key.into_iter();
        loop {
            if let Some(key) = key.next() {
                node = node.make_next(trie, key);
            } else {
                break node;
            }
        }
    }
    /// Descends down [Trie] following 'key' while having immutable access.
    /// 'inspect' will be called with all [VNode]s encountered along the way,
    /// excluding the target returned inside [Result::Ok].
    /// A returned [Result::Err] will contain the [VNode] that has either been rejected
    /// by inspect, or where 'key' pointed towards a non-existent branch.
    pub fn descend<'a, Q: 'a + PartialEq + Ord>(
        &self,
        trie: &Trie<K, V>,
        key: impl IntoIterator<Item = &'a Q>,
        mut inspect: impl FnMut(Self, &Trie<K, V>) -> bool,
    ) -> Result<Self, Self>
    where
        K: Borrow<Q>,
    {
        let mut node = self.leak();
        for key in key {
            node = inspect(node.leak(), trie)
                .then_some(())
                .and_then(|()| node.next(trie, key))
                .ok_or(node)?;
        }
        Ok(node)
    }
    /// Descends down [Trie] following 'key' while having mutable access.
    /// 'inspect' will be called with all [VNode]s encountered along the way,
    /// excluding the target returned inside [Result::Ok].
    /// A returned [Result::Err] will contain the [VNode] that has either been rejected
    /// by inspect, or where 'key' pointed towards a non-existent branch.
    pub fn descend_mut<'a, Q: 'a + PartialEq + Ord>(
        &self,
        trie: &mut Trie<K, V>,
        key: impl IntoIterator<Item = &'a Q>,
        mut inspect: impl FnMut(Self, &mut Trie<K, V>) -> bool,
    ) -> Result<Self, Self>
    where
        K: Borrow<Q>,
    {
        let mut node = self.leak();
        for key in key {
            node = inspect(node.leak(), trie)
                .then_some(())
                .and_then(|()| node.next(trie, key))
                .ok_or(node)?;
        }
        Ok(node)
    }
    /// Searches for a value inside [Trie] along a path determined by 'key'.
    /// 'f' will be called with all [VNode]s encountered along the way.
    /// When this returns [Result::Ok] it will be returned immediately.
    /// When [Result::Err] is returned instead, search will continue,
    /// with the value inside [Option::Some] being kept as a backup,
    /// should the search otherwise fail and [Option::None] keeping the previous one.
    /// When 'f' never succeeds nor gives a backup, [Result::Err] will contain the [VNode] where 'key'
    /// ran out or pointed towards a non-existent branch.
    pub fn find<'a, Q: 'a + PartialEq + Ord, T>(
        &self,
        trie: &Trie<K, V>,
        key: impl IntoIterator<Item = &'a Q>,
        mut f: impl FnMut(Self, &Trie<K, V>) -> Result<T, Option<T>>,
    ) -> Result<T, Self>
    where
        K: Borrow<Q>,
    {
        let mut node = self.leak();
        let mut backup = None;
        let mut key = key.into_iter();
        loop {
            match f(node.leak(), trie) {
                Ok(t) => break Ok(t),
                Err(t @ Some(_)) => backup = t,
                Err(None) => (),
            }
            if let Some(key) = key.next()
                && let Some(next) = node.next(trie, key)
            {
                node = next;
            } else {
                break backup.ok_or(node.leak());
            }
        }
    }
    /// Dive into [Trie] following 'key' while having mutable access.
    /// 'inspect_descend' will be called on all [VNode]s encountered along the descend,
    /// excluding the target passed into 'inspect_target',
    /// where the [bool] value inside [Option::Some] determines whether it will be revisited during the ascend,
    /// while a [Option::None] will have it be rejected.
    /// During ascend 'inspect_ascend' will be called on all [VNode]s thus remembered,
    /// being cut short if 'false' was returned.
    /// The value inside [Result::Ok] will be the one inside
    /// the [Option::Some] returned by 'inspect_target'.
    /// A returned [Result::Err] will contain the [VNode] that has either been rejected
    /// by 'inspect_descend', where 'key' pointed towards a non-existent branch,
    /// or 'inspect_target' returned [Option::None].
    pub fn dive<'a, T, Q: 'a + PartialEq + Ord>(
        &self,
        trie: &mut Trie<K, V>,
        key: impl IntoIterator<Item = &'a Q>,
        mut inspect_descend: impl FnMut(Self, &mut Trie<K, V>) -> Option<bool>,
        inspect_target: impl FnOnce(Self, &mut Trie<K, V>) -> Option<T>,
        mut inspect_ascend: impl FnMut(Self, &mut Trie<K, V>) -> bool,
    ) -> Result<T, Self>
    where
        K: Borrow<Q>,
    {
        let mut stack = vec![];
        let target = self.descend_mut(trie, key, |node, trie| {
            inspect_descend(node.leak(), trie).map_or(false, |remember| {
                if remember {
                    stack.push(node.leak());
                }
                true
            })
        })?;
        stack.reverse();
        let ret = inspect_target(target.leak(), trie).ok_or(target)?;
        for node in stack {
            if !inspect_ascend(node, trie) {
                break;
            }
        }
        Ok(ret)
    }
    pub fn skip_prefix(&self, trie: &Trie<K, V>) -> Self {
        let handle = self.handle.leak();
        let prefix_len = handle.get(&trie.nodes).prefix().len();
        Self { prefix_len, handle }
    }
    pub fn into_iter(&self, mut trie: Trie<K, V>) -> impl use<K, V> + Iterator<Item = V> {
        let mut stack = vec![self.leak()];
        iter::from_fn(move || {
            loop {
                let node = stack.pop()?.skip_prefix(&trie);
                if let Some(branch) = node.branch(&trie) {
                    stack.extend(branch.values().rev().map(|node| Self {
                        prefix_len: 0,
                        handle: node.leak(),
                    }));
                }
                if let Some(leaf) = node.take_leaf(&mut trie) {
                    break Some(leaf);
                }
            }
        })
    }
    pub fn iter<'a>(&self, trie: &'a Trie<K, V>) -> impl use<'a, K, V> + Iterator<Item = &'a V> {
        let mut stack = vec![self.leak()];
        iter::from_fn(move || {
            loop {
                let node = stack.pop()?.skip_prefix(trie);
                if let Some(branch) = node.branch(trie) {
                    stack.extend(branch.values().rev().map(|node| Self {
                        prefix_len: 0,
                        handle: node.leak(),
                    }));
                }
                if let Some(leaf) = node.leaf(trie) {
                    break Some(leaf);
                }
            }
        })
    }

    pub fn iter_mut<'a>(
        &self,
        trie: &'a mut Trie<K, V>,
    ) -> impl use<'a, K, V> + Iterator<Item = &'a mut V> {
        let mut stack = vec![self.leak()];
        iter::from_fn(move || {
            loop {
                let node = stack.pop()?.skip_prefix(trie);
                if let Some(branch) = node.branch(trie) {
                    stack.extend(branch.values().rev().map(|node| Self {
                        prefix_len: 0,
                        handle: node.leak(),
                    }));
                }
                if let Some(leaf) = node.leaf_mut(trie) {
                    // SAFETY (lifetime extension):
                    //      each yielded node is distinct and we only return
                    //      a mutable reference to the leaf directly tied to it,
                    //      which are therefore also distinct.
                    break Some(unsafe { transmute(leaf) });
                }
            }
        })
    }
}

/// Manipulation methods
impl<K: Ord, V> VNode<K, V> {
    pub fn as_node<'a>(&self, nodes: &'a Shared<Node<K, V>>) -> Option<&'a Node<K, V>> {
        match self.prefix_len.cmp(&self.handle.get(nodes).prefix().len()) {
            Ordering::Less => None,
            Ordering::Equal => Some(self.handle.get(nodes)),
            Ordering::Greater => panic!("Invalid VNode"),
        }
    }
    pub fn as_node_mut<'a>(&self, nodes: &'a mut Shared<Node<K, V>>) -> Option<&'a mut Node<K, V>> {
        match self.prefix_len.cmp(&self.handle.get(nodes).prefix().len()) {
            Ordering::Less => None,
            Ordering::Equal => Some(self.handle.get_mut(nodes)),
            Ordering::Greater => panic!("Invalid VNode"),
        }
    }
    pub fn make_leaf(&self, trie: &mut Trie<K, V>, value: V) -> Option<V> {
        let Trie {
            root: _,
            nodes,
            branches,
            leaves,
        } = trie;
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
    pub fn leaf_handle(&self, trie: &Trie<K, V>) -> Option<LeafHandle<V>> {
        self.as_node(&trie.nodes)?.leaf()
    }
    pub fn leaf<'a>(&self, trie: &'a Trie<K, V>) -> Option<&'a V> {
        Some(self.leaf_handle(trie)?.get(&trie.leaves))
    }
    pub fn leaf_mut<'a>(&self, trie: &'a mut Trie<K, V>) -> Option<&'a mut V> {
        Some(self.leaf_handle(trie)?.get_mut(&mut trie.leaves))
    }
    pub fn take_leaf(&self, trie: &mut Trie<K, V>) -> Option<V> {
        self.as_node_mut(&mut trie.nodes)?
            .take_leaf(&mut trie.leaves)
    }
    pub fn make_branch(&self, trie: &mut Trie<K, V>) -> BranchHandle<K, V> {
        let node = self.handle.get_mut(&mut trie.nodes);
        match self.prefix_len.cmp(&node.prefix().len()) {
            Ordering::Less => {
                let (branch, new_node) = node.make_branch_at(&mut trie.branches, self.prefix_len);
                if let Some((key, new_node)) = new_node {
                    node.get_branch_mut(&mut trie.branches)
                        .unwrap()
                        .insert(key, Handle::new(&mut trie.nodes, new_node));
                }
                branch
            }
            Ordering::Equal => node.make_branch(&mut trie.branches),
            Ordering::Greater => unreachable!(),
        }
    }
    pub fn branch_handle(&self, trie: &Trie<K, V>) -> Option<BranchHandle<K, V>> {
        self.as_node(&trie.nodes)?.branch()
    }
    pub fn branch<'a>(&self, trie: &'a Trie<K, V>) -> Option<&'a Branch<K, V>> {
        self.as_node(&trie.nodes)?.get_branch(&trie.branches)
    }
    pub fn prune_branch(&self, trie: &mut Trie<K, V>) -> bool {
        if let Some(branch) = self
            .as_node(&trie.nodes)
            .expect("VNode isn't actual Node and cannot be pruned.")
            .branch()
            && branch.get_mut(&mut trie.branches).prune(&mut trie.nodes)
            && let Some(branch) = self.handle.get_mut(&mut trie.nodes).take_branch()
        {
            let branch = branch.remove(&mut trie.branches);
            debug_assert!(branch.is_empty());
        }
        self.handle.get(&trie.nodes).is_empty_node()
    }
}
