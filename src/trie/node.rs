use crate::{
    trie::{
        Handle, LeafHandle, NodeDebug, Trie,
        branch::{BTreeBranch, BranchHandle},
        handle::Shared,
        leaf::Leaf,
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
    previous: NodeHandle<K, V>,
    prefix: Vec<K>,
    data: DataHandle<K, V>,
    #[cfg(feature = "testing")]
    this: NodeHandle<K, V>,
}
impl<K, V> NodeDebug<K, V> for Node<K, V> {
    fn default_with_owner(#[cfg(feature = "testing")] owner: NodeHandle<K, V>) -> Self {
        Self::from(owner, Handle::new_null(), Vec::default(), ())
    }
    fn debug<'a>(&'a self, trie: &'a Trie<K, V>) -> impl 'a + Debug
    where
        K: Debug,
        V: Debug,
    {
        debug_fn(|f| {
            let mut f = f.debug_struct(match self.data {
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
                f.field("branch", &branch.get(&trie.branches).debug(trie));
            }
            f.finish()
        })
    }
    fn set_owner(&mut self, owner: NodeHandle<K, V>) -> NodeHandle<K, V> {
        use std::mem::replace;
        replace(&mut self.this, owner)
    }
}
impl<K, V> Node<K, V> {
    pub fn _from_null<T: Into<DataHandle<K, V>>>(prefix: Vec<K>, handle: T) -> Self {
        Self::from(
            #[cfg(feature = "testing")]
            Handle::new_null(),
            Handle::new_null(),
            prefix,
            handle,
        )
    }
    pub fn from<T: Into<DataHandle<K, V>>>(
        #[cfg(feature = "testing")] this: NodeHandle<K, V>,
        previous: NodeHandle<K, V>,
        prefix: Vec<K>,
        handle: T,
    ) -> Self {
        Self {
            previous,
            prefix,
            data: handle.into(),
            #[cfg(feature = "testing")]
            this,
        }
    }
    pub fn is_empty(&self) -> bool {
        match &self.data {
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
    pub(super) fn previous(&self) -> NodeHandle<K, V> {
        self.previous.leak()
    }
    #[cfg(feature = "testing")]
    pub(super) fn set_this(
        &mut self,
        this: NodeHandle<K, V>,
        branches: &mut Shared<BTreeBranch<K, V>>,
        leaves: &mut Shared<Leaf<V>>,
    ) -> NodeHandle<K, V> {
        let (old_a, old_b, old_c) = (
            replace(&mut self.this, this.leak()),
            self.get_branch_mut(branches)
                .map(|branch| branch.set_owner(this.leak())),
            self._get_leaf_mut(leaves).map(|leaf| leaf.set_owner(this)),
        );
        if let Some(old_b) = old_b
            && !old_a._is_null()
            && !old_b._is_null()
        {
            assert_eq!(old_a, old_b);
        }
        if let Some(old_c) = old_c
            && !old_a._is_null()
            && !old_c._is_null()
        {
            assert_eq!(old_a, old_c);
        }
        old_a
    }
    pub(super) fn set_previous(&mut self, previous: NodeHandle<K, V>) -> NodeHandle<K, V> {
        replace(&mut self.previous, previous)
    }
    pub fn branch(&self) -> Option<BranchHandle<K, V>> {
        self.data.branch()
    }
    pub fn get_branch<'a>(
        &self,
        branches: &'a Shared<BTreeBranch<K, V>>,
    ) -> Option<&'a BTreeBranch<K, V>> {
        Some(self.data.branch()?.get(branches))
    }
    pub fn get_branch_mut<'a>(
        &self,
        branches: &'a mut Shared<BTreeBranch<K, V>>,
    ) -> Option<&'a mut BTreeBranch<K, V>> {
        Some(self.data.branch()?.get_mut(branches))
    }
    pub fn leaf(&self) -> Option<LeafHandle<V>> {
        self.data.leaf()
    }
    pub fn _get_leaf<'a>(&self, leaves: &'a Shared<Leaf<V>>) -> Option<&'a Leaf<V>> {
        Some(self.data.leaf()?.get(leaves))
    }
    pub fn _get_leaf_mut<'a>(&self, leaves: &'a mut Shared<Leaf<V>>) -> Option<&'a mut Leaf<V>> {
        Some(self.data.leaf()?.get_mut(leaves))
    }
    pub fn leaf_branch(&self) -> (Option<LeafHandle<V>>, Option<BranchHandle<K, V>>) {
        self.data.leaf_branch()
    }
    pub fn _get_leaf_branch<'a, 'b>(
        &self,
        leaves: &'a Shared<Leaf<V>>,
        branches: &'b Shared<BTreeBranch<K, V>>,
    ) -> (Option<&'a V>, Option<&'b BTreeBranch<K, V>>) {
        let (leaf, branch) = self.leaf_branch();
        (
            leaf.map(|leaf| leaf.get(leaves).get()),
            branch.map(|branch| branch.get(branches)),
        )
    }
    pub fn _get_leaf_branch_mut<'a, 'b>(
        &self,
        leaves: &'a mut Shared<Leaf<V>>,
        branches: &'b mut Shared<BTreeBranch<K, V>>,
    ) -> (Option<&'a mut V>, Option<&'b mut BTreeBranch<K, V>>) {
        let (leaf, branch) = self.leaf_branch();
        (
            leaf.map(|leaf| leaf.get_mut(leaves).get_mut()),
            branch.map(|branch| branch.get_mut(branches)),
        )
    }
}
impl<K: Ord, V> Node<K, V> {
    pub fn make_leaf(
        &mut self,
        #[cfg(feature = "testing")] this: NodeHandle<K, V>,
        leaves: &mut Shared<Leaf<V>>,
        value: V,
    ) -> Option<V> {
        match self.data.leak() {
            DataHandle::Empty => {
                self.data = DataHandle::Leaf(Handle::new(
                    leaves,
                    Leaf::new(
                        #[cfg(feature = "testing")]
                        this,
                        value,
                    ),
                ));
                None
            }
            DataHandle::Leaf(leaf) | DataHandle::Full { leaf, .. } => {
                Some(leaf.get_mut(leaves).replace(value))
            }
            DataHandle::Branch(handle) => {
                self.data = DataHandle::Full {
                    leaf: Handle::new(
                        leaves,
                        Leaf::new(
                            #[cfg(feature = "testing")]
                            this,
                            value,
                        ),
                    ),
                    branch: handle,
                };
                None
            }
        }
    }
    pub fn make_leaf_at(
        &mut self,
        this: NodeHandle<K, V>,
        branches: &mut Shared<BTreeBranch<K, V>>,
        leaves: &mut Shared<Leaf<V>>,
        value: V,
        leaf_at: usize,
    ) -> (Option<V>, Option<(K, Node<K, V>)>) {
        assert!(leaf_at <= self.prefix.len());
        #[cfg(feature = "testing")]
        assert_eq!(self.this, this);
        let node = (leaf_at < self.prefix.len()).then_some(()).and_then(|()| {
            let node = self.make_branch_at(this.leak(), branches, leaf_at).1;
            debug_assert_eq!(self.leaf(), None);
            node
        });
        debug_assert_eq!(leaf_at, self.prefix.len());
        (
            self.make_leaf(
                #[cfg(feature = "testing")]
                this,
                leaves,
                value,
            ),
            node,
        )
    }
    pub fn take_leaf(&mut self, leaves: &mut Shared<Leaf<V>>) -> Option<V> {
        match self.data.leak() {
            DataHandle::Empty | DataHandle::Branch(_) => None,
            DataHandle::Leaf(leaf) => {
                self.data = DataHandle::Empty;
                self.prefix.clear();
                Some(leaf.remove(leaves).unwrap())
            }
            DataHandle::Full { leaf, branch } => {
                self.data = DataHandle::Branch(branch);
                Some(leaf.remove(leaves).unwrap())
            }
        }
    }
    pub fn make_branch(
        &mut self,
        #[cfg(feature = "testing")] _this: NodeHandle<K, V>,
        branches: &mut Shared<BTreeBranch<K, V>>,
    ) -> BranchHandle<K, V> {
        #[cfg(feature = "testing")]
        assert_eq!(self.this, _this);
        let branch_handle: BranchHandle<K, V>;
        self.data = match &self.data {
            DataHandle::Empty => {
                branch_handle = Handle::new(
                    branches,
                    BTreeBranch::default_with_owner(
                        #[cfg(feature = "testing")]
                        _this,
                    ),
                )
                .into();
                branch_handle.leak().into()
            }
            DataHandle::Leaf(leaf) => {
                branch_handle = Handle::new(
                    branches,
                    BTreeBranch::default_with_owner(
                        #[cfg(feature = "testing")]
                        _this,
                    ),
                )
                .into();
                (leaf.leak(), branch_handle.leak()).into()
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
        this: NodeHandle<K, V>,
        branches: &mut Shared<BTreeBranch<K, V>>,
        branch_at: usize,
    ) -> (BranchHandle<K, V>, Option<(K, Node<K, V>)>) {
        assert!(branch_at <= self.prefix().len());
        #[cfg(feature = "testing")]
        assert_eq!(self.this, this);
        let node = (branch_at < self.prefix().len()).then(|| {
            let mut drain = self.prefix.drain(branch_at..);
            let key = drain.next().unwrap();
            let prefix = drain.collect();
            let node = Node::from(
                #[cfg(feature = "testing")]
                Handle::new_null(),
                this.leak(),
                prefix,
                replace(&mut self.data, DataHandle::Empty),
            );
            (key, node)
        });
        debug_assert_eq!(branch_at, self.prefix.len());
        (
            self.make_branch(
                #[cfg(feature = "testing")]
                this,
                branches,
            ),
            node,
        )
    }
    pub fn take_branch(&mut self) -> Option<BranchHandle<K, V>> {
        match self.data.leak() {
            DataHandle::Empty | DataHandle::Leaf(_) => None,
            DataHandle::Branch(branch) => {
                self.data = ().into();
                self.prefix.clear();
                Some(branch)
            }
            DataHandle::Full { leaf, branch } => {
                self.data = leaf.into();
                Some(branch)
            }
        }
    }
}
pub type NodeHandle<K, V> = Handle<Node<K, V>>;
