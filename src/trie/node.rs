use crate::{
    trie::{
        Handle, LeafHandle, Trie,
        branch::{Branch, BranchHandle},
        handle::Shared,
    },
    util::debug_fn,
};
use std::{fmt::Debug, mem::replace};

#[derive(Default)]
pub enum DataHandle<K, V> {
    #[default]
    Empty,
    Leaf(LeafHandle<V>),
    Branch(BranchHandle<K, V>),
    Full {
        leaf: LeafHandle<V>,
        branch: BranchHandle<K, V>,
    },
}
impl<K, V> Debug for DataHandle<K, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(f, "Empty"),
            Self::Leaf(arg0) => f.debug_tuple("Leaf").field(arg0).finish(),
            Self::Branch(arg0) => f.debug_tuple("Branch").field(arg0).finish(),
            Self::Full { leaf, branch } => f
                .debug_struct("Full")
                .field("leaf", leaf)
                .field("branch", branch)
                .finish(),
        }
    }
}
impl<K, V> From<()> for DataHandle<K, V> {
    fn from((): ()) -> Self {
        Self::Empty
    }
}
impl<K, V> From<LeafHandle<V>> for DataHandle<K, V> {
    fn from(handle: LeafHandle<V>) -> Self {
        Self::Leaf(handle)
    }
}
impl<K, V> From<BranchHandle<K, V>> for DataHandle<K, V> {
    fn from(handle: BranchHandle<K, V>) -> Self {
        Self::Branch(handle)
    }
}
impl<K, V> From<(LeafHandle<V>, BranchHandle<K, V>)> for DataHandle<K, V> {
    fn from((leaf, branch): (LeafHandle<V>, BranchHandle<K, V>)) -> Self {
        Self::Full { leaf, branch }
    }
}
impl<K, V> DataHandle<K, V> {
    pub fn leak(&self) -> Self {
        match self {
            DataHandle::Empty => DataHandle::Empty,
            DataHandle::Leaf(handle) => DataHandle::Leaf(handle.leak()),
            DataHandle::Branch(handle) => DataHandle::Branch(handle.leak()),
            DataHandle::Full { leaf, branch } => DataHandle::Full {
                leaf: leaf.leak(),
                branch: branch.leak(),
            },
        }
    }
    pub fn leaf(&self) -> Option<LeafHandle<V>> {
        match self.leak() {
            DataHandle::Empty | DataHandle::Branch(_) => None,
            DataHandle::Leaf(leaf) | DataHandle::Full { leaf, .. } => Some(leaf),
        }
    }
    pub fn branch(&self) -> Option<BranchHandle<K, V>> {
        match self.leak() {
            DataHandle::Empty | DataHandle::Leaf(_) => None,
            DataHandle::Branch(branch) | DataHandle::Full { branch, .. } => Some(branch),
        }
    }
    pub fn leaf_branch(&self) -> (Option<LeafHandle<V>>, Option<BranchHandle<K, V>>) {
        match self.leak() {
            DataHandle::Empty => (None, None),
            DataHandle::Leaf(leaf) => (Some(leaf), None),
            DataHandle::Branch(branch) => (None, Some(branch)),
            DataHandle::Full { leaf, branch } => (Some(leaf), Some(branch)),
        }
    }
}

#[derive(Debug)]
pub struct Node<K, V> {
    prefix: Vec<K>,
    handle: DataHandle<K, V>,
}
impl<K, V> Default for Node<K, V> {
    fn default() -> Self {
        Self {
            prefix: Default::default(),
            handle: Default::default(),
        }
    }
}
impl<K, V> Node<K, V> {
    pub fn from<T: Into<DataHandle<K, V>>>(prefix: Vec<K>, handle: T) -> Self {
        Self {
            prefix,
            handle: handle.into(),
        }
    }
    pub fn is_empty(&self) -> bool {
        match &self.handle {
            DataHandle::Empty => true,
            _ => false,
        }
    }
    pub fn prefix(&self) -> &Vec<K> {
        &self.prefix
    }
    pub(super) fn prefix_mut(&mut self) -> &mut Vec<K> {
        &mut self.prefix
    }
    pub fn branch(&self) -> Option<BranchHandle<K, V>> {
        self.handle.branch()
    }
    pub fn get_branch<'a>(&self, branches: &'a Shared<Branch<K, V>>) -> Option<&'a Branch<K, V>> {
        Some(self.handle.branch()?.get(branches))
    }
    pub fn get_branch_mut<'a>(
        &self,
        branches: &'a mut Shared<Branch<K, V>>,
    ) -> Option<&'a mut Branch<K, V>> {
        Some(self.handle.branch()?.get_mut(branches))
    }
    pub fn leaf(&self) -> Option<LeafHandle<V>> {
        self.handle.leaf()
    }
    pub fn get_leaf<'a>(&self, leaves: &'a Shared<V>) -> Option<&'a V> {
        Some(self.handle.leaf()?.get(leaves))
    }
    pub fn get_leaf_mut<'a>(&self, leaves: &'a mut Shared<V>) -> Option<&'a mut V> {
        Some(self.handle.leaf()?.get_mut(leaves))
    }
    pub fn leaf_branch(&self) -> (Option<LeafHandle<V>>, Option<BranchHandle<K, V>>) {
        self.handle.leaf_branch()
    }
    pub fn get_leaf_branch<'a, 'b>(
        &self,
        leaves: &'a Shared<V>,
        branches: &'b Shared<Branch<K, V>>,
    ) -> (Option<&'a V>, Option<&'b Branch<K, V>>) {
        let (leaf, branch) = self.leaf_branch();
        (
            leaf.map(|leaf| leaf.get(leaves)),
            branch.map(|branch| branch.get(branches)),
        )
    }
    pub fn get_leaf_branch_mut<'a, 'b>(
        &self,
        leaves: &'a mut Shared<V>,
        branches: &'b mut Shared<Branch<K, V>>,
    ) -> (Option<&'a mut V>, Option<&'b mut Branch<K, V>>) {
        let (leaf, branch) = self.leaf_branch();
        (
            leaf.map(|leaf| leaf.get_mut(leaves)),
            branch.map(|branch| branch.get_mut(branches)),
        )
    }
    pub fn node_debug<'a>(&'a self, trie: &'a Trie<K, V>) -> impl 'a + Debug
    where
        K: Debug,
        V: Debug,
    {
        debug_fn(|f| {
            let mut f = f.debug_struct(match self.handle {
                DataHandle::Empty => "Node::Empty",
                DataHandle::Leaf(_) => "Node::Leaf",
                DataHandle::Branch(_) => "Node::Branch",
                DataHandle::Full { .. } => "Node::Full",
            });
            if !self.prefix().is_empty() {
                f.field("prefix", &self.prefix);
            }
            if let Some(leaf) = self.leaf() {
                f.field("leaf", &leaf.get(&trie.leaves));
            }
            if let Some(branch) = self.branch() {
                f.field("branch", &branch.get(&trie.branches).branch_debug(trie));
            }
            f.finish()
        })
    }
}
impl<K: Ord, V> Node<K, V> {
    pub fn make_leaf(&mut self, leaves: &mut Shared<V>, value: V) -> Option<V> {
        match self.handle.leak() {
            DataHandle::Empty => {
                self.handle = DataHandle::Leaf(Handle::new(leaves, value));
                None
            }
            DataHandle::Leaf(leaf) | DataHandle::Full { leaf, .. } => {
                Some(leaf.replace(leaves, value))
            }
            DataHandle::Branch(handle) => {
                self.handle = DataHandle::Full {
                    leaf: Handle::new(leaves, value),
                    branch: handle,
                };
                None
            }
        }
    }
    pub fn make_leaf_at(
        &mut self,
        branches: &mut Shared<Branch<K, V>>,
        leaves: &mut Shared<V>,
        value: V,
        leaf_at: usize,
    ) -> (Option<V>, Option<(K, Node<K, V>)>) {
        assert!(leaf_at <= self.prefix.len());
        let node = if leaf_at < self.prefix.len() {
            let node = self.make_branch_at(branches, leaf_at).1;
            debug_assert_eq!(self.leaf(), None);
            node
        } else {
            None
        };
        debug_assert_eq!(leaf_at, self.prefix.len());
        (self.make_leaf(leaves, value), node)
    }
    pub fn take_leaf(&mut self, leaves: &mut Shared<V>) -> Option<V> {
        match self.handle.leak() {
            DataHandle::Empty | DataHandle::Branch(_) => None,
            DataHandle::Leaf(leaf) => {
                self.handle = DataHandle::Empty;
                self.prefix.clear();
                Some(leaf.remove(leaves))
            }
            DataHandle::Full { leaf, branch } => {
                self.handle = DataHandle::Branch(branch);
                Some(leaf.remove(leaves))
            }
        }
    }
    pub fn make_branch(&mut self, branches: &mut Shared<Branch<K, V>>) -> BranchHandle<K, V> {
        let branch_handle: BranchHandle<K, V>;
        self.handle = match &self.handle {
            DataHandle::Empty => {
                branch_handle = Handle::new_default(branches).into();
                branch_handle.leak().into()
            }
            DataHandle::Leaf(leaf) => {
                branch_handle = Handle::new_default(branches).into();
                DataHandle::Full {
                    leaf: leaf.leak(),
                    branch: branch_handle.leak(),
                }
            }
            handle @ DataHandle::Branch(branch) | handle @ DataHandle::Full { branch, .. } => {
                branch_handle = branch.leak();
                handle.leak()
            }
        };
        branch_handle
    }
    pub fn make_branch_at(
        &mut self,
        branches: &mut Shared<Branch<K, V>>,
        branch_at: usize,
    ) -> (BranchHandle<K, V>, Option<(K, Node<K, V>)>) {
        assert!(branch_at <= self.prefix().len());
        let node = if branch_at < self.prefix().len() {
            let mut drain = self.prefix.drain(branch_at..);
            let key = drain.next().unwrap();
            let prefix = drain.collect();
            let node = match replace(&mut self.handle, DataHandle::Empty) {
                DataHandle::Empty => Node::from(prefix, ()),
                DataHandle::Leaf(leaf) => Node::from(prefix, leaf),
                DataHandle::Branch(branch) => Node::from(prefix, branch),
                DataHandle::Full { leaf, branch } => Node::from(prefix, (leaf, branch)),
            };
            Some((key, node))
        } else {
            None
        };
        debug_assert_eq!(branch_at, self.prefix.len());
        (self.make_branch(branches), node)
    }
    pub fn take_branch(&mut self) -> Option<BranchHandle<K, V>> {
        match self.handle.leak() {
            DataHandle::Empty | DataHandle::Leaf(_) => None,
            DataHandle::Branch(branch) => {
                self.handle = DataHandle::Empty;
                self.prefix.clear();
                Some(branch)
            }
            DataHandle::Full { leaf, branch } => {
                self.handle = DataHandle::Leaf(leaf);
                Some(branch)
            }
        }
    }
}
pub type NodeHandle<K, V> = Handle<Node<K, V>>;
