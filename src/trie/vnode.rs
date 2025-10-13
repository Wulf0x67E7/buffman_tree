use crate::trie::{
    LeafHandle, Trie,
    branch::{BTreeBranch, Branch, BranchHandle},
    handle::{Handle, Shared},
    node::{Node, NodeHandle},
};
use std::{
    borrow::Borrow,
    cmp::Ordering,
    fmt::Debug,
    iter::{self, Peekable},
    mem::{replace, take, transmute},
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
    pub fn _make_next(&self, trie: &mut Trie<K, V>, key: K) -> Self {
        if let Some(next) = self.next(trie, &key) {
            return next;
        }
        let node = self.handle.get_mut(&mut trie.nodes);
        if node.is_empty() {
            node.prefix_mut().push(key);
            Self {
                prefix_len: node.prefix().len(),
                handle: self.handle.leak(),
            }
        } else {
            Self::start(
                self.make_branch(trie)
                    .get_mut(&mut trie.branches)
                    .get_or_insert(self.handle.leak(), &mut trie.nodes, key),
            )
        }
    }
    pub fn make_descend(&self, trie: &mut Trie<K, V>, key: impl IntoIterator<Item = K>) -> Self {
        let mut vnode = self.leak();
        let mut key = key.into_iter().peekable();
        loop {
            if vnode.empty_node(&trie.nodes) {
                vnode
                    .handle
                    .get_mut(&mut trie.nodes)
                    .prefix_mut()
                    .extend(key);
                break vnode.skip_prefix(trie);
            }
            vnode = vnode.try_skip_prefix(trie, &mut key, K::eq);
            let Some(key) = key.next() else {
                break vnode;
            };
            vnode = Self::start(
                vnode
                    .make_branch(trie)
                    .get_mut(&mut trie.branches)
                    .get_or_insert(vnode.handle.leak(), &mut trie.nodes, key),
            );
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
        mut inspect: impl FnMut(Self, &Trie<K, V>, &'a Q) -> bool,
    ) -> Result<Self, Self>
    where
        K: Borrow<Q>,
    {
        let mut key = key.into_iter().peekable();
        let mut node = self.leak();
        loop {
            node = node.try_skip_prefix(trie, &mut key, |k, q| k.borrow() == *q);
            let Some(key) = key.next() else {
                break Ok(node);
            };
            let (true, Some(next)) = (inspect(node.leak(), trie, key), node.next(trie, key)) else {
                break Err(node);
            };
            node = next;
        }
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
        mut inspect: impl FnMut(Self, &mut Trie<K, V>, &'a Q) -> bool,
    ) -> Result<Self, Self>
    where
        K: Borrow<Q>,
    {
        let mut key = key.into_iter().peekable();
        let mut node = self.leak();
        loop {
            node = node.try_skip_prefix(trie, &mut key, |k, q| k.borrow() == *q);
            let Some(key) = key.next() else {
                break Ok(node);
            };
            let (true, Some(next)) = (inspect(node.leak(), trie, key), node.next(trie, key)) else {
                break Err(node);
            };
            node = next;
        }
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
        let mut key = key.into_iter().peekable();
        loop {
            node = node.try_skip_prefix(trie, &mut key, |k, q| k.borrow() == *q);
            match f(node.leak(), trie) {
                Ok(t) => break Ok(t),
                Err(t @ Some(_)) => backup = t,
                Err(None) => (),
            }
            if let Some(k) = key.next()
                && let Some(next) = node.next(trie, k)
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
        mut inspect_descend: impl FnMut(Self, &mut Trie<K, V>, &'a Q) -> bool,
        inspect_target: impl FnOnce(Self, &mut Trie<K, V>) -> Option<T>,
        mut inspect_ascend: impl FnMut(Self, &mut Trie<K, V>, &'a Q) -> bool,
    ) -> Result<T, Self>
    where
        K: Borrow<Q>,
    {
        let mut stack = vec![];
        let target = self.descend_mut(trie, key, |node, trie, key| {
            stack.push((node.leak(), key));
            inspect_descend(node.leak(), trie, key)
        })?;
        stack.reverse();
        let ret = inspect_target(target.leak(), trie).ok_or(target)?;
        for (node, key) in stack {
            if !inspect_ascend(node, trie, key) {
                break;
            }
        }
        Ok(ret)
    }
    pub fn snap_prefix(&self, trie: &Trie<K, V>) -> Self {
        let handle = self.handle.leak();
        let prefix_len = self.prefix_len.min(handle.get(&trie.nodes).prefix().len());
        Self { prefix_len, handle }
    }
    pub fn skip_prefix(&self, trie: &Trie<K, V>) -> Self {
        let handle = self.handle.leak();
        let prefix_len = handle.get(&trie.nodes).prefix().len();
        Self { prefix_len, handle }
    }
    pub fn try_skip_prefix<Q: PartialEq>(
        &self,
        trie: &Trie<K, V>,
        key: &mut Peekable<impl Iterator<Item = Q>>,
        eq: impl Fn(&K, &Q) -> bool,
    ) -> Self {
        let handle = self.handle.leak();
        let remaining_prefix = &handle.get(&trie.nodes).prefix()[self.prefix_len..];
        let mut match_len = 0;
        while let Some(k) = remaining_prefix.get(match_len)
            && let Some(_) = key.next_if(|key| eq(k, key))
        {
            match_len += 1;
        }
        Self {
            prefix_len: self.prefix_len + match_len,
            handle,
        }
    }
    pub fn into_iter(&self, mut trie: Trie<K, V>) -> impl use<K, V> + Iterator<Item = V> {
        let mut stack = vec![self.leak()];
        iter::from_fn(move || {
            loop {
                let node = stack.pop()?.skip_prefix(&trie);
                if let Some(branch) = node.branch(&trie) {
                    stack.extend(branch.values().map(|node| Self {
                        prefix_len: 0,
                        handle: node.leak(),
                    }));
                }
                if let Some((_, leaf)) = node.take_leaf(&mut trie) {
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
                    stack.extend(branch.values().map(|node| Self {
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
                    stack.extend(branch.values().map(|node| Self {
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
    pub fn empty_node(&self, nodes: &Shared<Node<K, V>>) -> bool {
        self.handle.get(nodes).is_empty()
    }
    pub fn is_node_handle(&self, nodes: &Shared<Node<K, V>>) -> NodeHandle<K, V> {
        self.as_node_handle(nodes)
            .expect("VNode isn't actual Node.")
    }
    pub fn is_node<'a>(&self, nodes: &'a Shared<Node<K, V>>) -> &'a Node<K, V> {
        self.is_node_handle(nodes).get(nodes)
    }
    pub fn is_node_mut<'a>(&self, nodes: &'a mut Shared<Node<K, V>>) -> &'a mut Node<K, V> {
        self.is_node_handle(nodes).get_mut(nodes)
    }
    pub fn as_node_handle(&self, nodes: &Shared<Node<K, V>>) -> Option<NodeHandle<K, V>> {
        match self.prefix_len.cmp(&self.handle.get(nodes).prefix().len()) {
            Ordering::Less => None,
            Ordering::Equal => Some(self.handle.leak()),
            Ordering::Greater => panic!("Invalid VNode"),
        }
    }
    pub fn as_node<'a>(&self, nodes: &'a Shared<Node<K, V>>) -> Option<&'a Node<K, V>> {
        Some(self.as_node_handle(nodes)?.get(nodes))
    }
    pub fn as_node_mut<'a>(&self, nodes: &'a mut Shared<Node<K, V>>) -> Option<&'a mut Node<K, V>> {
        Some(self.as_node_handle(nodes)?.get_mut(nodes))
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
                let (value, new_node) =
                    node.make_leaf_at(self.handle.leak(), branches, leaves, value, self.prefix_len);
                if let Some((key, mut new_node)) = new_node {
                    debug_assert!(!new_node.is_empty());
                    let new_node = Handle::new_with(nodes, |_this| {
                        #[cfg(feature = "testing")]
                        new_node.set_this(_this, branches, leaves);
                        new_node
                    });
                    self.handle
                        .get_mut(nodes)
                        .get_branch_mut(branches)
                        .unwrap()
                        .insert(key, new_node);
                }
                value
            }
            Ordering::Equal => node.make_leaf(
                #[cfg(feature = "testing")]
                self.handle.leak(),
                leaves,
                value,
            ),
            Ordering::Greater => unreachable!(),
        }
    }
    pub fn leaf_handle(&self, trie: &Trie<K, V>) -> Option<LeafHandle<V>> {
        self.as_node(&trie.nodes)?.leaf()
    }
    pub fn leaf<'a>(&self, trie: &'a Trie<K, V>) -> Option<&'a V> {
        Some(self.leaf_handle(trie)?.get(&trie.leaves).get())
    }
    pub fn leaf_mut<'a>(&self, trie: &'a mut Trie<K, V>) -> Option<&'a mut V> {
        Some(self.leaf_handle(trie)?.get_mut(&mut trie.leaves).get_mut())
    }
    pub fn take_leaf(&self, trie: &mut Trie<K, V>) -> Option<(Self, V)> {
        let ret = self
            .as_node_mut(&mut trie.nodes)?
            .take_leaf(&mut trie.leaves)?;
        Some((self.snap_prefix(trie), ret))
    }
    pub fn make_branch(&self, trie: &mut Trie<K, V>) -> BranchHandle<K, V> {
        let Trie {
            root: _,
            nodes,
            branches,
            leaves: _leaves,
        } = trie;
        let node = self.handle.get_mut(nodes);
        match self.prefix_len.cmp(&node.prefix().len()) {
            Ordering::Less => {
                let (branch, new_node) =
                    node.make_branch_at(self.handle.leak(), branches, self.prefix_len);
                if let Some((key, mut new_node)) = new_node {
                    debug_assert!(!new_node.is_empty());
                    let new_node = Handle::new_with(nodes, |_this| {
                        #[cfg(feature = "testing")]
                        new_node.set_this(_this, branches, _leaves);
                        new_node
                    });
                    self.handle
                        .get_mut(nodes)
                        .get_branch_mut(branches)
                        .unwrap()
                        .insert(key, new_node);
                }
                branch
            }
            Ordering::Equal => node.make_branch(
                #[cfg(feature = "testing")]
                self.handle.leak(),
                &mut trie.branches,
            ),
            Ordering::Greater => unreachable!(),
        }
    }
    pub fn _branch_handle(&self, trie: &Trie<K, V>) -> Option<BranchHandle<K, V>> {
        self.as_node(&trie.nodes)?.branch()
    }
    pub fn branch<'a>(&self, trie: &'a Trie<K, V>) -> Option<&'a BTreeBranch<K, V>> {
        self.as_node(&trie.nodes)?.get_branch(&trie.branches)
    }
    pub fn _branch_mut<'a>(
        &mut self,
        trie: &'a mut Trie<K, V>,
    ) -> Option<&'a mut BTreeBranch<K, V>> {
        self.as_node_mut(&mut trie.nodes)?
            .get_branch_mut(&mut trie.branches)
    }
    fn prune_messy(
        &self,
        trie: &mut Trie<K, V>,
    ) -> Option<(
        Option<()>,
        BranchHandle<K, V>,
        Option<(K, NodeHandle<K, V>)>,
    )> {
        let (leaf, branch) = self.is_node(&trie.nodes).leaf_branch();
        // only prune if self is branch
        let branch = branch?;
        Some((
            leaf.map(|_| ()),
            branch.leak(),
            // try to prune
            branch.get_mut(&mut trie.branches).prune(&mut trie.nodes)?,
        ))
    }
    fn prune_cleanup(
        &self,
        trie: &mut Trie<K, V>,
        leaf: Option<()>,
        branch: BranchHandle<K, V>,
        displaced: Option<(K, NodeHandle<K, V>)>,
    ) {
        match (leaf, displaced) {
            // self is now empty, contract displaced into self
            (None, Some((key, displaced))) => {
                self.prune_contract(trie, key, displaced);
            }
            // leaf means self is not empty, have to restore displaced as single child
            (Some(_), Some((key, displaced))) => {
                let old = branch.get_mut(&mut trie.branches).insert(key, displaced);
                debug_assert_eq!(old, None);
            }
            // no displaced node to clean up, take empty branch
            (_, None) => {
                let branch = self
                    .is_node_mut(&mut trie.nodes)
                    .take_branch()
                    .unwrap()
                    .remove(&mut trie.branches);
                debug_assert!(branch.is_empty());
            }
        }
    }
    fn prune_contract(&self, trie: &mut Trie<K, V>, key: K, displaced: NodeHandle<K, V>) {
        let Trie {
            root: _,
            nodes,
            branches,
            leaves: _leaves,
        } = trie;
        // get node to contract into self
        let mut displaced = displaced.remove(nodes);
        let node = self.is_node_mut(nodes);
        // update displaced previous and this node to own
        displaced.set_previous(node.previous().leak());
        #[cfg(feature = "testing")]
        displaced.set_this(self.handle.leak(), branches, _leaves);
        // update displaced prefix to [own_prefix.., key, ..displaced_prefix]
        let (tmp, prefix) = (node.prefix_mut(), displaced.prefix_mut());
        tmp.push(key);
        tmp.extend(prefix.drain(..));
        *prefix = take(tmp);
        // replace empty self with displaced node
        let old = replace(node, displaced).branch().unwrap().remove(branches);
        debug_assert!(old.is_empty());
    }
    pub fn prune_branch(&self, trie: &mut Trie<K, V>) -> bool {
        if let Some((leaf, branch, displaced)) = self.prune_messy(trie) {
            self.prune_cleanup(trie, leaf, branch, displaced);
        }
        self.empty_node(&trie.nodes)
    }
}
