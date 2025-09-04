use slab::Slab;

use crate::{Branch, Leaf, handle::Handle};
use std::mem::{replace, take};

#[derive(Debug, PartialEq)]
pub enum Node<K, B, V> {
    None,
    Leaf(Leaf<K, V>),
    Branch(Branch<K, B, V>),
    Full(Leaf<K, V>, Branch<K, B, V>),
}
impl<K, B, V> Default for Node<K, B, V> {
    fn default() -> Self {
        Self::None
    }
}
impl<K, B, V> Node<K, B, V> {
    pub fn is_none(&self) -> bool {
        matches!(self, Node::None)
    }
    pub fn is_empty(&self) -> bool {
        match self {
            Node::None => true,
            Node::Leaf(_) | Node::Full(_, _) => false,
            Node::Branch(branch) => branch.is_empty(),
        }
    }
    pub fn as_branch(&self) -> Option<&Branch<K, B, V>> {
        if let Self::Branch(branch) | Self::Full(_, branch) = self {
            Some(branch)
        } else {
            None
        }
    }
    pub fn as_branch_mut(&mut self) -> Option<&mut Branch<K, B, V>> {
        if let Self::Branch(branch) | Self::Full(_, branch) = self {
            Some(branch)
        } else {
            None
        }
    }
    pub fn make_branch(&mut self) -> &mut Branch<K, B, V> {
        match self {
            Node::None => {
                *self = Self::Branch(Branch::default());
                let Node::Branch(branch) = self else {
                    unreachable!()
                };
                branch
            }
            Node::Leaf(_) => {
                let Node::Leaf(leaf) = take(self) else {
                    unreachable!();
                };
                *self = Self::Full(leaf, Branch::default());
                let Node::Full(_, branch) = self else {
                    unreachable!()
                };
                branch
            }
            Node::Branch(branch) | Node::Full(_, branch) => branch,
        }
    }
    pub fn insert_child_handle<'a>(&mut self, key: B, child: Handle<Self>) -> Option<Handle<Self>>
    where
        B: Ord,
    {
        self.make_branch().insert_handle(key, child)
    }
    pub fn insert_child<'a>(&mut self, key: B, shared: &'a mut Slab<Self>) -> &'a mut Node<K, B, V>
    where
        B: Ord,
    {
        self.make_branch().get_or_insert(key, shared)
    }
    pub fn get_child_handle(&self, key: B) -> Option<&Handle<Self>>
    where
        B: Ord,
    {
        self.as_branch()?.get_handle(key)
    }
    pub fn get_child<'a>(&self, key: B, shared: &'a Slab<Self>) -> Option<&'a Node<K, B, V>>
    where
        B: Ord,
    {
        self.as_branch()?.get(key, shared)
    }
    pub fn get_child_mut<'a>(
        &mut self,
        key: B,
        shared: &'a mut Slab<Self>,
    ) -> Option<&'a mut Node<K, B, V>>
    where
        B: Ord,
    {
        self.as_branch_mut()?.get_mut(key, shared)
    }
    pub fn as_leaf(&self) -> Option<&Leaf<K, V>> {
        if let Self::Leaf(leaf) | Self::Full(leaf, _) = self {
            Some(leaf)
        } else {
            None
        }
    }
    pub fn as_leaf_mut(&mut self) -> Option<&mut Leaf<K, V>> {
        if let Self::Leaf(leaf) | Self::Full(leaf, _) = self {
            Some(leaf)
        } else {
            None
        }
    }
    pub fn make_leaf(&mut self, key: K, value: V) -> Option<Leaf<K, V>>
    where
        K: PartialEq,
    {
        let new = Leaf::from(key, value);
        match self {
            Node::None => {
                *self = Node::Leaf(new);
                None
            }
            Node::Branch(_) => {
                let Node::Branch(branch) = take(self) else {
                    unreachable!();
                };
                *self = Self::Full(new, branch);
                None
            }
            Node::Leaf(leaf) | Node::Full(leaf, _) => {
                debug_assert!(leaf.key() == new.key());
                Some(replace(leaf, new))
            }
        }
    }
    pub fn take_leaf(&mut self) -> Option<Leaf<K, V>> {
        match self {
            Node::None => {
                debug_assert!(false);
                None
            }
            Node::Leaf(_) => {
                let Node::Leaf(leaf) = take(self) else {
                    unreachable!();
                };
                Some(leaf)
            }
            Node::Branch(_) => None,
            Node::Full(_, _) => {
                let Node::Full(leaf, branch) = take(self) else {
                    unreachable!();
                };
                *self = Node::Branch(branch);
                Some(leaf)
            }
        }
    }
    pub fn as_leaf_branch(&self) -> (Option<&Leaf<K, V>>, Option<&Branch<K, B, V>>) {
        match self {
            Node::None => (None, None),
            Node::Leaf(leaf) => (Some(leaf), None),
            Node::Branch(branch) => (None, Some(branch)),
            Node::Full(leaf, branch) => (Some(leaf), Some(branch)),
        }
    }
    pub fn as_leaf_branch_mut(
        &mut self,
    ) -> (Option<&mut Leaf<K, V>>, Option<&mut Branch<K, B, V>>) {
        match self {
            Node::None => (None, None),
            Node::Leaf(leaf) => (Some(leaf), None),
            Node::Branch(branch) => (None, Some(branch)),
            Node::Full(leaf, branch) => (Some(leaf), Some(branch)),
        }
    }
    pub fn as_leaf_child_handle(
        &self,
        key: Option<B>,
    ) -> (Option<&Leaf<K, V>>, Option<&Handle<Self>>)
    where
        B: Ord,
    {
        let (leaf, branch) = self.as_leaf_branch();
        (
            leaf,
            branch
                .zip(key)
                .and_then(|(branch, key)| branch.get_handle(key)),
        )
    }
    pub fn as_leaf_child_handle_mut(
        &mut self,
        key: Option<B>,
    ) -> (Option<&mut Leaf<K, V>>, Option<&Handle<Self>>)
    where
        B: Ord,
    {
        let (leaf, branch) = self.as_leaf_branch_mut();
        (
            leaf,
            branch
                .zip(key)
                .and_then(|(branch, key)| branch.get_handle(key)),
        )
    }
    pub fn as_leaf_child<'a>(
        &self,
        key: Option<B>,
        shared: &'a Slab<Self>,
    ) -> (Option<&Leaf<K, V>>, Option<&'a Node<K, B, V>>)
    where
        B: Ord,
    {
        let (leaf, branch) = self.as_leaf_branch();
        (
            leaf,
            branch
                .zip(key)
                .and_then(|(branch, key)| branch.get(key, shared)),
        )
    }
    pub fn as_leaf_child_mut<'a>(
        &mut self,
        key: Option<B>,
        shared: &'a mut Slab<Self>,
    ) -> (Option<&mut Leaf<K, V>>, Option<&'a mut Node<K, B, V>>)
    where
        B: Ord,
    {
        let (leaf, branch) = self.as_leaf_branch_mut();
        (
            leaf,
            branch
                .zip(key)
                .and_then(|(branch, key)| branch.get_mut(key, shared)),
        )
    }
}
