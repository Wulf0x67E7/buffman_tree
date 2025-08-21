use std::{
    collections::BTreeMap,
    mem::{replace, take},
    ops::{Deref, DerefMut},
};

#[derive(Debug, Default, PartialEq)]
pub struct Leaf<K, V> {
    key: K,
    value: V,
}
impl<K, V> Leaf<K, V> {
    pub fn new(key: K, value: V) -> Self {
        Self { key, value }
    }
    pub fn as_ref(&self) -> Leaf<&K, &V> {
        let Leaf { key, value } = self;
        Leaf { key, value }
    }
    pub fn as_mut(&mut self) -> Leaf<&K, &mut V> {
        let Leaf { key, value } = self;
        Leaf { key, value }
    }
}
#[derive(Debug, PartialEq)]
pub struct Branch<K, B, V>(BTreeMap<B, Node<K, B, V>>);
impl<K, B, V> Default for Branch<K, B, V> {
    fn default() -> Self {
        Self(Default::default())
    }
}
impl<K, B, V> FromIterator<(B, Node<K, B, V>)> for Branch<K, B, V>
where
    B: Ord,
{
    fn from_iter<T: IntoIterator<Item = (B, Node<K, B, V>)>>(iter: T) -> Self {
        let mut ret = Self::default();
        for (key, value) in iter {
            ret.insert(key, value);
        }
        ret
    }
}
impl<K, B, V> Deref for Branch<K, B, V> {
    type Target = BTreeMap<B, Node<K, B, V>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<K, B, V> DerefMut for Branch<K, B, V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
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
            Node::Leaf(_) => false,
            Node::Branch(branch) | Node::Full(_, branch) => branch.is_empty(),
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
        let new = Leaf { key, value };
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
                debug_assert!(leaf.key == new.key);
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
}
