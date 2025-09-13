use slab::Slab;

use crate::{Branch, Key, Leaf, handle::Handle};
use std::mem::{replace, take};

pub type NodeId<K, V> = Handle<Node<K, V>>;

#[derive(Debug, PartialEq)]
pub enum Node<K: Key, V> {
    None,
    Leaf(bool, Leaf<K, V>),
    Branch(Branch<K, V>),
    Full(Leaf<K, V>, Branch<K, V>),
}
impl<K: Key, V> Default for Node<K, V> {
    fn default() -> Self {
        Self::None
    }
}
impl<K: Key, V> Node<K, V> {
    pub fn is_none(&self) -> bool {
        matches!(self, Node::None)
    }
    pub fn is_empty(&self) -> bool {
        match self {
            Node::None => true,
            Node::Leaf(_, _) | Node::Full(_, _) => false,
            Node::Branch(branch) => branch.is_empty(),
        }
    }
    pub fn as_branch(&self) -> Option<&Branch<K, V>> {
        if let Self::Branch(branch) | Self::Full(_, branch) = self {
            Some(branch)
        } else {
            None
        }
    }
    pub fn as_branch_mut(&mut self) -> Option<&mut Branch<K, V>> {
        if let Self::Branch(branch) | Self::Full(_, branch) = self {
            Some(branch)
        } else {
            None
        }
    }
    pub fn make_branch(&mut self) -> (&mut Branch<K, V>, Option<Leaf<K, V>>) {
        match self {
            Node::None => {
                *self = Self::Branch(Branch::default());
                let Node::Branch(branch) = self else {
                    unreachable!()
                };
                (branch, None)
            }
            Node::Leaf(_, _) => {
                let Node::Leaf(shallow, leaf) = take(self) else {
                    unreachable!();
                };
                let leaf = if shallow {
                    *self = Self::Branch(Branch::default());
                    Some(leaf)
                } else {
                    *self = Self::Full(leaf, Branch::default());
                    None
                };
                let (Node::Branch(branch) | Node::Full(_, branch)) = self else {
                    unreachable!()
                };
                (branch, leaf)
            }
            Node::Branch(branch) | Node::Full(_, branch) => (branch, None),
        }
    }
    pub fn insert_child_handle<'a>(
        &mut self,
        key: K::Piece,
        child: Handle<Self>,
    ) -> (Option<Handle<Self>>, Option<Leaf<K, V>>) {
        let (branch, leaf) = self.make_branch();
        (branch.insert_handle(key, child), leaf)
    }
    pub fn insert_child<'a>(
        &mut self,
        key: K::Piece,
        shared: &'a mut Slab<Self>,
    ) -> (&'a mut Node<K, V>, Option<Leaf<K, V>>) {
        let (branch, leaf) = self.make_branch();
        (branch.get_or_insert(key, shared), leaf)
    }
    pub fn get_child_handle(&self, key: K::Piece) -> Option<&Handle<Self>> {
        self.as_branch()?.get_handle(&key)
    }
    pub fn get_child<'a>(&self, key: K::Piece, shared: &'a Slab<Self>) -> Option<&'a Node<K, V>> {
        self.as_branch()?.get(key, shared)
    }
    pub fn get_child_mut<'a>(
        &mut self,
        key: K::Piece,
        shared: &'a mut Slab<Self>,
    ) -> Option<&'a mut Node<K, V>> {
        self.as_branch_mut()?.get_mut(key, shared)
    }
    pub fn as_leaf(&self) -> Option<&Leaf<K, V>> {
        if let Self::Leaf(_, leaf) | Self::Full(leaf, _) = self {
            Some(leaf)
        } else {
            None
        }
    }
    pub fn as_leaf_mut(&mut self) -> Option<&mut Leaf<K, V>> {
        if let Self::Leaf(_, leaf) | Self::Full(leaf, _) = self {
            Some(leaf)
        } else {
            None
        }
    }
    pub fn make_leaf(
        &mut self,
        shallow: bool,
        key: K,
        value: V,
    ) -> Option<Result<Leaf<K, V>, Leaf<K, V>>> {
        let new = Leaf::from(key, value);
        match self {
            Node::None => {
                *self = Node::Leaf(shallow, new);
                None
            }
            Node::Branch(_) => {
                let Node::Branch(branch) = take(self) else {
                    unreachable!();
                };
                *self = Self::Full(new, branch);
                None
            }
            Node::Leaf(shallow, leaf) => {
                if *shallow {
                    debug_assert!(leaf.key().len() > new.key().len());
                    Some(Err(replace(leaf, new)))
                } else {
                    debug_assert!(leaf.key().equal(new.key()));
                    Some(Ok(replace(leaf, new)))
                }
            }
            Node::Full(leaf, _) => {
                debug_assert!(leaf.key().equal(new.key()));
                Some(Ok(replace(leaf, new)))
            }
        }
    }
    pub fn take_leaf(&mut self) -> Option<Leaf<K, V>> {
        match self {
            Node::None => {
                debug_assert!(false);
                None
            }
            Node::Leaf(_, _) => {
                let Node::Leaf(_, leaf) = take(self) else {
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
    pub fn as_leaf_branch(&self) -> (Option<&Leaf<K, V>>, Option<&Branch<K, V>>) {
        match self {
            Node::None => (None, None),
            Node::Leaf(_, leaf) => (Some(leaf), None),
            Node::Branch(branch) => (None, Some(branch)),
            Node::Full(leaf, branch) => (Some(leaf), Some(branch)),
        }
    }
    pub fn as_leaf_branch_mut(&mut self) -> (Option<&mut Leaf<K, V>>, Option<&mut Branch<K, V>>) {
        match self {
            Node::None => (None, None),
            Node::Leaf(_, leaf) => (Some(leaf), None),
            Node::Branch(branch) => (None, Some(branch)),
            Node::Full(leaf, branch) => (Some(leaf), Some(branch)),
        }
    }
    pub fn as_leaf_child_handle(
        &self,
        key: Option<K::Piece>,
    ) -> (Option<&Leaf<K, V>>, Option<&Handle<Self>>) {
        let (leaf, branch) = self.as_leaf_branch();
        (
            leaf,
            branch
                .zip(key)
                .and_then(|(branch, key)| branch.get_handle(&key)),
        )
    }
    pub fn as_leaf_child_handle_mut(
        &mut self,
        key: Option<K::Piece>,
    ) -> (Option<&mut Leaf<K, V>>, Option<&Handle<Self>>) {
        let (leaf, branch) = self.as_leaf_branch_mut();
        (
            leaf,
            branch
                .zip(key)
                .and_then(|(branch, key)| branch.get_handle(&key)),
        )
    }
    pub fn as_leaf_child<'a>(
        &self,
        key: Option<K::Piece>,
        shared: &'a Slab<Self>,
    ) -> (Option<&Leaf<K, V>>, Option<&'a Node<K, V>>) {
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
        key: Option<K::Piece>,
        shared: &'a mut Slab<Self>,
    ) -> (Option<&mut Leaf<K, V>>, Option<&'a mut Node<K, V>>) {
        let (leaf, branch) = self.as_leaf_branch_mut();
        (
            leaf,
            branch
                .zip(key)
                .and_then(|(branch, key)| branch.get_mut(key, shared)),
        )
    }
}
