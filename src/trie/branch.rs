use std::collections::{BTreeMap, btree_map::Entry};

use crate::Node;

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
            ret.0.insert(key, value);
        }
        ret
    }
}
impl<K, B, V> Branch<K, B, V> {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn get_or_insert(&mut self, key: B) -> &mut Node<K, B, V>
    where
        B: Ord,
    {
        self.0.entry(key).or_default()
    }
    pub fn get(&self, key: B) -> Option<&Node<K, B, V>>
    where
        B: Ord,
    {
        self.0.get(&key)
    }
    pub fn get_mut(&mut self, key: B) -> Option<&mut Node<K, B, V>>
    where
        B: Ord,
    {
        self.0.get_mut(&key)
    }
    pub fn remove_if(
        &mut self,
        key: B,
        f: impl FnOnce(&mut Node<K, B, V>) -> bool,
    ) -> Option<Node<K, B, V>>
    where
        B: Ord,
    {
        match self.0.entry(key) {
            Entry::Vacant(_) => None,
            Entry::Occupied(mut child) => {
                debug_assert!(!child.get().is_empty());
                if f(child.get_mut()) {
                    Some(child.remove())
                } else {
                    None
                }
            }
        }
    }
}
