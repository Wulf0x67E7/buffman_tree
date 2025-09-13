mod branch;
pub mod handle;
mod key;
mod leaf;
mod node;
mod walk;
use crate::handle::Handle;
pub use branch::*;
pub use key::*;
pub use leaf::*;
pub use node::*;
use slab::Slab;
use std::{borrow::Borrow, fmt::Debug};
pub(crate) use walk::*;
pub struct Trie<K: Key, V> {
    root: Option<NodeId<K, V>>,
    shared: Slab<Node<K, V>>,
}
impl<K: Key, V: Debug> std::fmt::Debug for Trie<K, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut walk = Walk::start(&self.root, Ordered);
        let mut f = &mut f.debug_struct("Trie");
        while let Some((node, err)) = walk.next(&self.shared) {
            debug_assert!(!err);
            f = f.field(&node.to_string(), node.get(&self.shared));
        }
        f.finish()
    }
}
impl<K: Key, V: PartialEq> PartialEq for Trie<K, V> {
    fn eq(&self, other: &Self) -> bool {
        self.iter().zip(other.iter()).all(|(a, b)| a == b)
    }
}
impl<K: Key, V> Default for Trie<K, V> {
    fn default() -> Self {
        Self {
            root: None,
            shared: Slab::new(),
        }
    }
}
impl<K: Key, V> FromIterator<(K, V)> for Trie<K, V> {
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        let mut ret = Self::default();
        for (key, value) in iter {
            ret.insert(key, value);
        }
        ret
    }
}
impl<K: Key, V> Trie<K, V> {
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
    pub fn insert(&mut self, key: K, value: V) -> Option<Leaf<K, V>> {
        let mut ret = None;
        let mut displaced = vec![(key, value)];
        while let Some((key, value)) = displaced.pop() {
            let mut shallow = false;
            let mut node = self
                .root
                .get_or_insert_with(|| {
                    shallow = true;
                    Handle::new_default(&mut self.shared)
                })
                .leak();
            'shallow: {
                for key in key.pieces().map(|k| k.to_owned()) {
                    if shallow {
                        break 'shallow;
                    }
                    node = node.insert_if(
                        &mut self.shared,
                        {
                            let key = key.clone();
                            |node| {
                                node.get_child_handle(key).map(Handle::leak).ok_or_else(|| {
                                    shallow = true;
                                    Node::default()
                                })
                            }
                        },
                        |node, handle| {
                            displaced.extend(
                                node.insert_child_handle(key, handle.leak())
                                    .1
                                    .map(Leaf::unwrap),
                            );
                        },
                    );
                }
                shallow = false;
            }
            match node
                .get_mut(&mut self.shared)
                .make_leaf(shallow, key, value)
            {
                Some(Ok(leaf)) if ret.is_none() => {
                    ret = Some(leaf);
                }
                Some(Ok(leaf) | Err(leaf)) => displaced.push(leaf.unwrap()),
                None => (),
            }
        }
        ret
    }

    pub fn get<Q>(&self, key: &Q) -> Option<Leaf<&K, &V>>
    where
        Q: Key<Piece = K::Piece>,
    {
        let mut walk = Walk::start(&self.root, keyed(key));
        let (mut node, mut err) = walk.next(&self.shared)?;
        debug_assert!(!node.get(&self.shared).is_empty());
        while !err && let Some((n, e)) = walk.next(&self.shared) {
            node = n;
            err |= e;
            debug_assert!(!node.get(&self.shared).is_empty());
        }
        node.get(&self.shared)
            .as_leaf()
            .filter(|leaf| leaf.key().equal(key))
            .map(Leaf::as_ref)
    }
    pub fn get_mut<Q>(&mut self, key: &Q) -> Option<Leaf<&K, &mut V>>
    where
        Q: Key<Piece = K::Piece>,
    {
        let mut walk = Walk::start(&self.root, keyed(key));
        let (mut node, mut err) = walk.next(&self.shared)?;
        debug_assert!(!node.get(&self.shared).is_empty());
        while !err && let Some((n, e)) = walk.next(&self.shared) {
            node = n;
            err |= e;
            debug_assert!(!node.get(&self.shared).is_empty());
        }
        node.get_mut(&mut self.shared)
            .as_leaf_mut()
            .filter(|leaf| leaf.key().equal(key))
            .map(Leaf::as_mut)
    }
    pub fn get_deepest<Q>(&self, key: &Q) -> Option<&Node<K, V>>
    where
        Q: Key<Piece = K::Piece>,
    {
        let mut walk = Walk::start(&self.root, keyed(key));
        let (mut node, mut err) = walk.next(&self.shared)?;
        debug_assert!(!node.get(&self.shared).is_empty());
        while !err && let Some((n, e)) = walk.next(&self.shared) {
            if let Some(leaf) = n.get(&self.shared).as_leaf()
                && leaf.key().len() <= key.len()
            {
                node = n;
            }
            err |= e;
            debug_assert!(!node.get(&self.shared).is_empty());
        }
        Some(node.get(&self.shared))
    }
    pub fn get_deepest_leaf<Q>(&self, key: &Q) -> Option<Leaf<&K, &V>>
    where
        Q: Key<Piece = K::Piece>,
    {
        let mut walk = Walk::start(&self.root, keyed(key));
        let (mut node, mut err) = (None, false);
        while !err && let Some((n, e)) = walk.next(&self.shared) {
            node = n
                .get(&self.shared)
                .as_leaf()
                .filter(|leaf| leaf.key().len() == leaf.key().equal_len(key))
                .or(node);
            err |= e;
        }
        node.map(Leaf::as_ref)
    }
    pub fn get_deepest_leaf_mut<Q>(&mut self, key: &Q) -> Option<Leaf<&K, &mut V>>
    where
        Q: Key<Piece = K::Piece>,
    {
        let mut walk = Walk::start(&self.root, keyed(key));
        let (mut node, mut err) = (None, false);
        while !err && let Some((n, e)) = walk.next(&self.shared) {
            node = n
                .get(&self.shared)
                .as_leaf()
                .filter(|leaf| leaf.key().len() == leaf.key().equal_len(key))
                .map(|_| n)
                .or(node);
            err |= e;
        }
        node.map(|node| node.get_mut(&mut self.shared).as_leaf_mut().unwrap())
            .filter(|leaf| leaf.key().equal(key))
            .map(Leaf::as_mut)
    }
    pub fn remove<Q>(&mut self, key: &Q) -> Option<Leaf<K, V>>
    where
        Q: Key,
        K::Piece: Borrow<Q::Piece>,
    {
        let key = Vec::from_iter(key.pieces().cloned());
        let mut walk = Walk::start(&self.root, keyed(&key));
        let mut track = Vec::with_capacity(key.len() + 1);
        let mut err = false;
        while !err && let Some((node, e)) = walk.next(&self.shared) {
            track.push(node);
            err |= e;
        }
        if err || track.len() < key.len() + 1 {
            return None;
        }
        debug_assert_eq!(track.len(), key.len() + 1);
        let ret = track.pop()?.get_mut(&mut self.shared).take_leaf()?;
        'early: {
            for (k, mut node) in key.iter().zip(track.into_iter()).rev() {
                if let None = node.remove_if(
                    &mut self.shared,
                    |node, shared| {
                        let child = node.get(shared).as_branch()?.get_handle(&k)?.leak();
                        child.get(shared).is_empty().then_some(child)
                    },
                    |node, shared, child| {
                        let c = node
                            .get_mut(shared)
                            .as_branch_mut()
                            .unwrap()
                            .remove(&k)
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
                let (node, err) = walk.next(&self.shared)?;
                debug_assert!(!err);
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
                let (node, err) = walk.next(&self.shared)?;
                debug_assert!(!err);
                if let Some(leaf) = node.get(&self.shared).as_leaf() {
                    break Some(leaf.as_ref());
                }
            }
        })
    }
    pub fn iter_mut(&mut self) -> impl Iterator<Item = Leaf<&K, &mut V>> {
        let mut walk = Walk::start(&self.root, Ordered);
        std::iter::from_fn(move || {
            loop {
                let (node, err) = walk.next(&self.shared)?;
                debug_assert!(!err);
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
    use quickcheck::Arbitrary;

    use super::*;

    impl<K: Key, V> Arbitrary for Trie<K, V>
    where
        (K, V): Arbitrary,
        V: 'static,
        Trie<K, V>: Clone + FromIterator<(K, V)>,
    {
        fn arbitrary(g: &mut quickcheck::Gen) -> Self {
            Self::from_iter(Vec::arbitrary(g))
        }
    }

    #[test]
    fn empty() {
        let trie: Trie<[(); 0], ()> = Trie::default();
        assert!(trie.is_empty());
        assert_eq!(
            trie,
            Trie {
                root: None,
                shared: Slab::default()
            }
        );
        assert_eq!(trie.get(&Some(())), None);
    }

    #[test]
    fn insert() {
        let mut trie = Trie::default();
        assert_eq!(trie.insert(vec![], ' '), None);
        assert_eq!(trie.get(&[]), Some(Leaf::new(&vec![], &' ')));
        assert_eq!(
            trie,
            Trie {
                root: Some(Handle::from(0)),
                shared: Slab::from_iter([(0, Node::Leaf(false, Leaf::new(vec![], ' ')))])
            }
        );
        assert_eq!(trie.insert(vec![], '_'), Some(Leaf::new(vec![], ' ')));
        assert_eq!(trie.get(&[]), Some(Leaf::new(&vec![], &'_')));
        assert_eq!(
            trie,
            Trie {
                root: Some(Handle::from(0)),
                shared: Slab::from_iter([(0, Node::Leaf(false, Leaf::new(vec![], '_')))])
            }
        );
        assert_eq!(trie.insert(vec![0], 'O'), None);
        assert_eq!(trie.insert(vec![1], '1'), None);
        assert_eq!(trie.insert(vec![0], '0'), Some(Leaf::new(vec![0], 'O')));
        assert_eq!(trie.get(&[0]), Some(Leaf::new(&vec![0], &'0')));
        assert_eq!(trie.get(&[1]), Some(Leaf::new(&vec![1], &'1')));
        assert_eq!(trie.get(&[0, 0, 0]), None);
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
                    (1, Node::Leaf(false, Leaf::new(vec![0], '0'))),
                    (2, Node::Leaf(false, Leaf::new(vec![1], '1')))
                ])
            }
        );
    }
    #[test]
    fn shallow_leaf() {
        let trie = Trie::from_iter([(vec![0, 0, 0], "000")]);
        assert_eq!(
            trie.root
                .as_ref()
                .unwrap()
                .get(&trie.shared)
                .as_leaf()
                .unwrap(),
            &Leaf::from(vec![0, 0, 0], "000")
        );
        assert_eq!(trie.get(&[]), None);
    }
    #[test]
    fn remove() {
        let mut trie = Trie::default();
        trie.insert(vec![], ' ');
        trie.insert(vec![0], '0');
        trie.insert(vec![1], '1');
        assert_eq!(trie.remove(&[2]), None);
        assert_eq!(trie.remove(&[0, 0]), None);
        assert_eq!(trie.remove(&[0]), Some(Leaf::new(vec![0], '0')));
        assert_eq!(trie, Trie::from_iter([(vec![], ' '), (vec![1], '1')]));
        assert_eq!(trie.remove(&[0]), None);
        assert_eq!(trie.remove(&[]), Some(Leaf::new(vec![], ' ')));
        assert_eq!(trie, Trie::from_iter([(vec![1], '1')]));
        assert_eq!(trie.remove(&[]), None);
        assert_eq!(trie.remove(&[1]), Some(Leaf::new(vec![1], '1')));
        assert_eq!(trie, Trie::default());
        assert_eq!(trie.remove(&[1]), None);
    }
    #[test]
    fn get_deepest_leaf() {
        let mut trie = Trie::from_iter([(vec![0], "0"), (vec![0; 3], "000"), (vec![0, 1], "01")]);
        assert_eq!(trie.get_deepest_leaf(&[]), None);
        assert_eq!(trie.insert(vec![], ""), None);
        assert_eq!(trie.get_deepest_leaf(&[]), Some(Leaf::new(&vec![], &"")));
        assert_eq!(trie.get_deepest_leaf(&[0]), Some(Leaf::new(&vec![0], &"0")));
        assert_eq!(
            trie.get_deepest_leaf(&[0, 0]),
            Some(Leaf::new(&vec![0], &"0"))
        );
        assert_eq!(
            trie.get_deepest_leaf(&[0; 5]),
            Some(Leaf::new(&vec![0; 3], &"000"))
        );
        assert_eq!(
            trie.get_deepest_leaf(&[0, 1]),
            Some(Leaf::new(&vec![0, 1], &"01"))
        );
    }
}
