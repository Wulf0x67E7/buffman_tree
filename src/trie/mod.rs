mod branch;
pub mod handle;
mod leaf;
mod node;
mod walk;
use crate::handle::Handle;
pub use branch::*;
pub use leaf::*;
pub use node::*;
use slab::Slab;
use std::{cmp::Ordering, fmt::Debug};
pub(crate) use walk::*;
pub struct Trie<K, B, V> {
    root: Option<Handle<Node<K, B, V>>>,
    shared: Slab<Node<K, B, V>>,
}
impl<K: Debug, B: Debug, V: Debug> std::fmt::Debug for Trie<K, B, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut walk = Walk::start(&self.root, Ordered);
        let mut f = &mut f.debug_struct("Trie");
        while let Some(node) = walk.next(&self.shared) {
            f = f.field(&node.to_string(), node.get(&self.shared));
        }
        f.finish()
    }
}
impl<K: PartialEq, B, V: PartialEq> PartialEq for Trie<K, B, V> {
    fn eq(&self, other: &Self) -> bool {
        self.iter().zip(other.iter()).all(|(a, b)| a == b)
    }
}
impl<K, B, V> Default for Trie<K, B, V> {
    fn default() -> Self {
        Self {
            root: None,
            shared: Slab::new(),
        }
    }
}
impl<K, B, V> FromIterator<(K, V)> for Trie<K, B, V>
where
    K: PartialEq,
    for<'a> &'a K: IntoIterator<Item = &'a B>,
    B: Clone + Ord,
{
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        let mut ret = Self::default();
        for (key, value) in iter {
            ret.insert(key, value);
        }
        ret
    }
}
impl<K, B, V> Trie<K, B, V> {
    pub fn is_empty(&self) -> bool {
        debug_assert!(
            !self
                .root
                .as_ref()
                .map(|h| h.get(&self.shared))
                .map(Node::is_empty)
                .unwrap_or(false)
        );
        self.root.is_none()
    }
    pub fn insert(&mut self, key: K, value: V) -> Option<Leaf<K, V>>
    where
        K: PartialEq,
        for<'a> &'a K: IntoIterator<Item = &'a B>,
        B: Ord + Clone,
    {
        let mut node = self
            .root
            .get_or_insert_with(|| Handle::new_default(&mut self.shared))
            .leak();
        for key in key.into_iter().cloned() {
            node = node.insert_if(
                &mut self.shared,
                {
                    let key = key.clone();
                    |node| {
                        node.get_child_handle(key)
                            .map(Handle::leak)
                            .ok_or_else(|| Node::default())
                    }
                },
                |node, handle| {
                    node.insert_child_handle(key, handle.leak());
                },
            );
        }
        node.get_mut(&mut self.shared).make_leaf(key, value)
    }

    pub fn get<Q>(&self, key: Q) -> Option<Leaf<&K, &V>>
    where
        Q: IntoIterator<Item = B>,
        B: Ord,
    {
        let mut walk = Walk::start(&self.root, Keyed::from(key));
        let mut node = walk.next(&self.shared)?;
        debug_assert!(!node.get(&self.shared).is_empty());
        while let Some(n) = walk.next(&self.shared) {
            node = n;
            debug_assert!(!node.get(&self.shared).is_empty());
        }
        node.get(&self.shared).as_leaf().map(Leaf::as_ref)
    }
    pub fn get_mut<Q>(&mut self, key: Q) -> Option<Leaf<&K, &mut V>>
    where
        Q: IntoIterator<Item = B>,
        B: Ord,
    {
        let mut walk = Walk::start(&self.root, Keyed::from(key));
        let mut node = walk.next(&self.shared)?;
        debug_assert!(!node.get(&self.shared).is_empty());
        while let Some(n) = walk.next(&self.shared) {
            node = n;
            debug_assert!(!node.get(&self.shared).is_empty());
        }
        node.get_mut(&mut self.shared)
            .as_leaf_mut()
            .map(Leaf::as_mut)
    }
    pub fn get_deepest<Q>(&self, key: Q) -> Option<&Node<K, B, V>>
    where
        Q: IntoIterator<Item = B>,
        B: Ord,
    {
        let mut walk = Walk::start(&self.root, Keyed::from(key));
        let mut node = walk.next(&self.shared)?;
        debug_assert!(!node.get(&self.shared).is_empty());
        while let Some(n) = walk.next(&self.shared) {
            node = n;
            debug_assert!(!node.get(&self.shared).is_empty());
        }
        Some(node.get(&self.shared))
    }
    pub fn get_deepest_leaf<Q>(&self, key: Q) -> Option<Leaf<&K, &V>>
    where
        Q: IntoIterator<Item = B>,
        B: Ord,
    {
        let mut walk = Walk::start(&self.root, Keyed::from(key));
        let mut node = None;
        while let Some(n) = walk.next(&self.shared) {
            node = n.get(&self.shared).as_leaf().or(node);
        }
        node.map(Leaf::as_ref)
    }
    pub fn get_deepest_leaf_mut<Q>(&mut self, key: Q) -> Option<Leaf<&K, &mut V>>
    where
        Q: IntoIterator<Item = B>,
        B: Ord,
    {
        let mut walk = Walk::start(&self.root, Keyed::from(key));
        let mut node = None;
        while let Some(n) = walk.next(&self.shared) {
            node = n.get(&self.shared).as_leaf().map(|_| n).or(node);
        }
        node.map(|node| {
            node.get_mut(&mut self.shared)
                .as_leaf_mut()
                .unwrap()
                .as_mut()
        })
    }
    pub fn remove<Q>(&mut self, key: Q) -> Option<Leaf<K, V>>
    where
        Q: IntoIterator<Item = B>,
        B: Ord + Clone,
    {
        let key = Vec::from_iter(key);
        let mut walk = Walk::start(&self.root, Keyed::from(key.iter().cloned()));
        let mut track = Vec::with_capacity(key.len() + 1);
        while let Some(node) = walk.next(&self.shared) {
            track.push(node);
        }
        match track.len().cmp(&(key.len() + 1)) {
            Ordering::Less => return None,
            Ordering::Equal => (),
            Ordering::Greater => unreachable!(),
        }
        let ret = track.pop()?.get_mut(&mut self.shared).take_leaf()?;
        'early: {
            for (k, mut node) in key.into_iter().zip(track.into_iter()).rev() {
                if let None = node.remove_if(
                    &mut self.shared,
                    {
                        let k = k.clone();
                        |node, shared| {
                            let child = node.get(shared).as_branch()?.get_handle(k)?.leak();
                            child.get(shared).is_empty().then_some(child)
                        }
                    },
                    |node, shared, child| {
                        let c = node
                            .get_mut(shared)
                            .as_branch_mut()
                            .unwrap()
                            .remove(k)
                            .unwrap();
                        assert_eq!(child, c);
                    },
                ) {
                    break 'early;
                }
            }
            self.root.take_if(|node| node.get(&self.shared).is_empty());
        }
        Some(ret)
    }
    pub fn into_iter(mut self) -> impl Iterator<Item = Leaf<K, V>> {
        let mut walk = Walk::start(&self.root, Ordered);
        std::iter::from_fn(move || {
            loop {
                let node = walk.next(&self.shared)?;
                if let Some(leaf) = node.get_mut(&mut self.shared).take_leaf() {
                    break Some(leaf);
                }
            }
        })
    }
    pub fn iter(&self) -> impl Iterator<Item = Leaf<&K, &V>> {
        let mut walk = Walk::start(&self.root, Ordered);
        std::iter::from_fn(move || {
            loop {
                let node = walk.next(&self.shared)?;
                if let Some(leaf) = node.get(&self.shared).as_leaf() {
                    break Some(leaf.as_ref());
                }
            }
        })
    }
    pub fn iter_mut(&mut self) -> impl Iterator<Item = Leaf<&K, &mut V>>
    where
        B: Ord,
    {
        let mut walk = Walk::start(&self.root, Ordered);
        std::iter::from_fn(move || {
            loop {
                let node = walk.next(&self.shared)?;
                if let Some(leaf) = node.get_mut(&mut self.shared).as_leaf_mut() {
                    // SAFETY - Lifetime extension
                    // We hold exclusive access to self and therefore shared and always return disjoint entries.
                    // This invariant is explicitly checked in debug builds where it will then panic.
                    break Some(unsafe { std::mem::transmute(leaf.as_mut()) });
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {

    use std::{
        collections::{BTreeSet, HashSet},
        iter::{repeat, zip},
    };

    use quickcheck_macros::quickcheck;

    use crate::util::unzipped;

    use super::*;

    #[test]
    fn empty() {
        let trie: Trie<(), (), ()> = Trie::default();
        assert!(trie.is_empty());
        assert_eq!(
            trie,
            Trie {
                root: None,
                shared: Slab::default()
            }
        );
        assert_eq!(trie.get(Some(())), None);
    }

    #[test]
    fn insert() {
        let mut trie = Trie::default();
        assert_eq!(trie.insert(vec![], ' '), None);
        assert_eq!(trie.get([]), Some(Leaf::new(&vec![], &' ')));
        assert_eq!(
            trie,
            Trie {
                root: Some(Handle::from(0)),
                shared: Slab::from_iter([(0, Node::Leaf(Leaf::new(vec![], ' ')))])
            }
        );
        assert_eq!(trie.insert(vec![], '_'), Some(Leaf::new(vec![], ' ')));
        assert_eq!(trie.get([]), Some(Leaf::new(&vec![], &'_')));
        assert_eq!(
            trie,
            Trie {
                root: Some(Handle::from(0)),
                shared: Slab::from_iter([(0, Node::Leaf(Leaf::new(vec![], '_')))])
            }
        );
        assert_eq!(trie.insert(vec![0], 'O'), None);
        assert_eq!(trie.insert(vec![1], '1'), None);
        assert_eq!(trie.insert(vec![0], '0'), Some(Leaf::new(vec![0], 'O')));
        assert_eq!(trie.get([0]), Some(Leaf::new(&vec![0], &'0')));
        assert_eq!(trie.get([1]), Some(Leaf::new(&vec![1], &'1')));
        assert_eq!(
            trie,
            Trie {
                root: Some(Handle::from(0)),
                shared: Slab::from_iter([
                    (
                        0,
                        Node::Full(
                            Leaf::new(vec![], '_'),
                            Branch::from_iter([(0, Handle::from(1)), (1, Handle::from(2))])
                        )
                    ),
                    (1, Node::Leaf(Leaf::new(vec![0], '0'))),
                    (2, Node::Leaf(Leaf::new(vec![1], '1')))
                ])
            }
        );
    }
    #[test]
    fn remove() {
        let mut trie = Trie::default();
        trie.insert(vec![], ' ');
        trie.insert(vec![0], '0');
        trie.insert(vec![1], '1');
        assert_eq!(trie.remove(vec![2]), None);
        assert_eq!(trie.remove(vec![0, 0]), None);
        assert_eq!(trie.remove(vec![0]), Some(Leaf::new(vec![0], '0')));
        assert_eq!(trie, Trie::from_iter([(vec![], ' '), (vec![1], '1')]));
        assert_eq!(trie.remove(vec![0]), None);
        assert_eq!(trie.remove(vec![]), Some(Leaf::new(vec![], ' ')));
        assert_eq!(trie, Trie::from_iter([(vec![1], '1')]));
        assert_eq!(trie.remove(vec![]), None);
        assert_eq!(trie.remove(vec![1]), Some(Leaf::new(vec![1], '1')));
        assert_eq!(trie, Trie::default());
        assert_eq!(trie.remove(vec![1]), None);
    }
    #[test]
    fn get_deepest_leaf() {
        let mut trie = Trie::from_iter([(vec![0], "0"), (vec![0; 3], "000"), (vec![0, 1], "01")]);
        assert_eq!(trie.get_deepest_leaf([]), None);
        assert_eq!(trie.insert(vec![], ""), None);
        assert_eq!(trie.get_deepest_leaf([]), Some(Leaf::new(&vec![], &"")));
        assert_eq!(trie.get_deepest_leaf([0]), Some(Leaf::new(&vec![0], &"0")));
        assert_eq!(
            trie.get_deepest_leaf([0, 0]),
            Some(Leaf::new(&vec![0], &"0"))
        );
        assert_eq!(
            trie.get_deepest_leaf([0; 5]),
            Some(Leaf::new(&vec![0; 3], &"000"))
        );
        assert_eq!(
            trie.get_deepest_leaf([0, 1]),
            Some(Leaf::new(&vec![0, 1], &"01"))
        );
    }
    #[quickcheck]
    fn iter_ord(data: BTreeSet<Vec<u8>>) {
        let trie = Trie::from_iter(data.iter().cloned().zip(repeat(())));
        let data2 = Vec::from_iter(trie.into_iter().map(|leaf| leaf.unwrap().0));
        assert_eq!(data.len(), data2.len());
        assert!(
            zip(&data, &data2).all(unzipped(PartialEq::eq)),
            "Result is not sorted:\n{data:?}\n{data2:?}"
        );
    }

    #[test]
    fn remove_cleanup_root_full() {
        let data = HashSet::from_iter([vec![0], vec![]]);
        _remove_cleanup(data);
    }
    #[test]
    fn remove_cleanup_root_branch() {
        let data = HashSet::from_iter([vec![0, 2]]);
        _remove_cleanup(data);
    }
    #[quickcheck]
    fn remove_cleanup(data: HashSet<Vec<u8>>) {
        _remove_cleanup(data);
    }
    fn _remove_cleanup(data: HashSet<Vec<u8>>) {
        let mut trie = Trie::from_iter(data.iter().cloned().zip(repeat(())));
        let data2 = Vec::from_iter(data.iter().flat_map(|v| trie.remove(v.iter().cloned())));
        assert!(trie.is_empty());
        assert_eq!(trie, Trie::default());
        assert_eq!(data.len(), data2.len());
    }
}
