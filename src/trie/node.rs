use crate::{
    trie::{Handle, LeafHandle, NodeDebug, Trie, handle::Shared, leaf::Leaf},
    util::debug_fn,
};
use std::{fmt::Debug, mem::replace};

#[derive(Default)]
pub enum DataHandle<V, B> {
    #[default]
    Empty,
    Leaf(LeafHandle<V>),
    Branch(Handle<B>),
    Full {
        leaf: LeafHandle<V>,
        branch: Handle<B>,
    },
}
impl<V, B> Debug for DataHandle<V, B> {
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
impl<V, B> From<()> for DataHandle<V, B> {
    fn from((): ()) -> Self {
        Self::Empty
    }
}
impl<V, B> From<(LeafHandle<V>, Handle<B>)> for DataHandle<V, B> {
    fn from((leaf, branch): (LeafHandle<V>, Handle<B>)) -> Self {
        Self::Full { leaf, branch }
    }
}
impl<V, B> DataHandle<V, B> {
    pub fn leak(&self) -> Self {
        match self {
            DataHandle::Empty => DataHandle::Empty,
            DataHandle::Leaf(handle) => DataHandle::Leaf(handle.leak()),
            DataHandle::Branch(handle) => DataHandle::Branch(handle.leak().into()),
            DataHandle::Full { leaf, branch } => DataHandle::Full {
                leaf: leaf.leak(),
                branch: branch.leak().into(),
            },
        }
    }
    pub fn leaf(&self) -> Option<LeafHandle<V>> {
        match self.leak() {
            DataHandle::Empty | DataHandle::Branch(_) => None,
            DataHandle::Leaf(leaf) | DataHandle::Full { leaf, .. } => Some(leaf),
        }
    }
    pub fn branch(&self) -> Option<Handle<B>> {
        match self.leak() {
            DataHandle::Empty | DataHandle::Leaf(_) => None,
            DataHandle::Branch(branch) | DataHandle::Full { branch, .. } => Some(branch.into()),
        }
    }
    pub fn leaf_branch(&self) -> (Option<LeafHandle<V>>, Option<Handle<B>>) {
        match self.leak() {
            DataHandle::Empty => (None, None),
            DataHandle::Leaf(leaf) => (Some(leaf), None),
            DataHandle::Branch(branch) => (None, Some(branch.into())),
            DataHandle::Full { leaf, branch } => (Some(leaf), Some(branch.into())),
        }
    }
}

#[derive(Debug)]
pub struct Node<K, V, B> {
    previous: NodeHandle<K, V, B>,
    prefix: Vec<K>,
    data: DataHandle<V, B>,
    #[cfg(feature = "testing")]
    this: NodeHandle<K, V, B>,
}
impl<K, V, B: NodeDebug<K, V, B>> NodeDebug<K, V, B> for Node<K, V, B> {
    fn default_with_owner(#[cfg(feature = "testing")] owner: NodeHandle<K, V, B>) -> Self {
        Self::from(
            #[cfg(feature = "testing")]
            owner,
            Handle::new_null(),
            Vec::default(),
            (),
        )
    }
    fn debug<'a>(&'a self, trie: &'a Trie<K, V, B>) -> impl 'a + Debug
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
    #[cfg(feature = "testing")]
    fn set_owner(&mut self, owner: NodeHandle<K, V, B>) -> NodeHandle<K, V, B> {
        use std::mem::replace;
        replace(&mut self.this, owner)
    }
}
impl<K, V, B> Node<K, V, B> {
    pub fn _from_null<T: Into<DataHandle<V, B>>>(prefix: Vec<K>, handle: T) -> Self {
        Self::from(
            #[cfg(feature = "testing")]
            Handle::new_null(),
            Handle::new_null(),
            prefix,
            handle,
        )
    }
    pub fn from<T: Into<DataHandle<V, B>>>(
        #[cfg(feature = "testing")] this: NodeHandle<K, V, B>,
        previous: NodeHandle<K, V, B>,
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
    pub(super) fn previous(&self) -> NodeHandle<K, V, B> {
        self.previous.leak()
    }
    #[cfg(feature = "testing")]
    pub(super) fn set_this(
        &mut self,
        this: NodeHandle<K, V, B>,
        branches: &mut Shared<B>,
        leaves: &mut Shared<Leaf<V>>,
    ) -> NodeHandle<K, V, B>
    where
        B: NodeDebug<K, V, B>,
    {
        let (old_a, old_b, old_c) = (
            replace(&mut self.this, this.leak()),
            self.get_branch_mut(branches)
                .map(|branch| branch.set_owner(this.leak())),
            self._get_leaf_mut(leaves).map(|leaf| leaf.set_owner(this)),
        );
        if let Some(old_b) = old_b
            && !old_a.is_null()
            && !old_b.is_null()
        {
            assert_eq!(old_a, old_b);
        }
        if let Some(old_c) = old_c
            && !old_a.is_null()
            && !old_c.is_null()
        {
            assert_eq!(old_a, old_c);
        }
        old_a
    }
    pub(super) fn set_previous(&mut self, previous: NodeHandle<K, V, B>) -> NodeHandle<K, V, B> {
        replace(&mut self.previous, previous)
    }
    pub fn branch(&self) -> Option<Handle<B>> {
        self.data.branch()
    }
    pub fn get_branch<'a>(&self, branches: &'a Shared<B>) -> Option<&'a B> {
        Some(self.data.branch()?.get(branches))
    }
    pub fn get_branch_mut<'a>(&self, branches: &'a mut Shared<B>) -> Option<&'a mut B> {
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
    pub fn leaf_branch(&self) -> (Option<LeafHandle<V>>, Option<Handle<B>>) {
        self.data.leaf_branch()
    }
    pub fn _get_leaf_branch<'a, 'b>(
        &self,
        leaves: &'a Shared<Leaf<V>>,
        branches: &'b Shared<B>,
    ) -> (Option<&'a V>, Option<&'b B>) {
        let (leaf, branch) = self.leaf_branch();
        (
            leaf.map(|leaf| leaf.get(leaves).get()),
            branch.map(|branch| branch.get(branches)),
        )
    }
    pub fn _get_leaf_branch_mut<'a, 'b>(
        &self,
        leaves: &'a mut Shared<Leaf<V>>,
        branches: &'b mut Shared<B>,
    ) -> (Option<&'a mut V>, Option<&'b mut B>) {
        let (leaf, branch) = self.leaf_branch();
        (
            leaf.map(|leaf| leaf.get_mut(leaves).get_mut()),
            branch.map(|branch| branch.get_mut(branches)),
        )
    }
}
impl<K, V, B> Node<K, V, B> {
    pub fn make_leaf(
        &mut self,
        #[cfg(feature = "testing")] this: NodeHandle<K, V, B>,
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
        this: NodeHandle<K, V, B>,
        branches: &mut Shared<B>,
        leaves: &mut Shared<Leaf<V>>,
        value: V,
        leaf_at: usize,
    ) -> (Option<V>, Option<(K, Node<K, V, B>)>)
    where
        B: Default + NodeDebug<K, V, B>,
    {
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
        #[cfg(feature = "testing")] _this: NodeHandle<K, V, B>,
        branches: &mut Shared<B>,
    ) -> Handle<B>
    where
        B: Default + NodeDebug<K, V, B>,
    {
        #[cfg(feature = "testing")]
        assert_eq!(self.this, _this);
        let branch_handle: Handle<B>;
        self.data = match &self.data {
            DataHandle::Empty => {
                branch_handle = Handle::new(
                    branches,
                    B::default_with_owner(
                        #[cfg(feature = "testing")]
                        _this,
                    ),
                )
                .into();
                DataHandle::Branch(branch_handle.leak())
            }
            DataHandle::Leaf(leaf) => {
                branch_handle = Handle::new(
                    branches,
                    B::default_with_owner(
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
        this: NodeHandle<K, V, B>,
        branches: &mut Shared<B>,
        branch_at: usize,
    ) -> (Handle<B>, Option<(K, Node<K, V, B>)>)
    where
        B: Default + NodeDebug<K, V, B>,
    {
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
    pub fn take_branch(&mut self) -> Option<Handle<B>> {
        match self.data.leak() {
            DataHandle::Empty | DataHandle::Leaf(_) => None,
            DataHandle::Branch(branch) => {
                self.data = ().into();
                self.prefix.clear();
                Some(branch)
            }
            DataHandle::Full { leaf, branch } => {
                self.data = DataHandle::Leaf(leaf.into());
                Some(branch)
            }
        }
    }
}
pub type NodeHandle<K, V, B> = Handle<Node<K, V, B>>;
