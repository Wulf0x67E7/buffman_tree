mod node;
pub use node::*;
#[derive(Debug, PartialEq)]
pub struct Trie<K, B, V> {
    root: Option<Node<K, B, V>>,
}
impl<K, B, V> Default for Trie<K, B, V> {
    fn default() -> Self {
        Self { root: None }
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
        debug_assert!(!self.root.as_ref().map(Node::is_empty).unwrap_or(false));
        self.root.is_none()
    }
    pub fn insert(&mut self, key: K, value: V) -> Option<Leaf<K, V>>
    where
        K: PartialEq,
        for<'a> &'a K: IntoIterator<Item = &'a B>,
        B: Ord + Clone,
    {
        let mut node = self.root.get_or_insert_default();
        for key in &key {
            node = node.insert_child(key.clone());
        }
        node.make_leaf(key, value)
    }
    pub fn get<Q>(&self, key: Q) -> Option<Leaf<&K, &V>>
    where
        Q: IntoIterator<Item = B>,
        B: Ord,
    {
        let mut node = self.root.as_ref()?;
        debug_assert!(!node.is_empty());
        for key in key {
            node = node.get_child(key)?;
            debug_assert!(!node.is_empty());
        }
        node.as_leaf().map(Leaf::as_ref)
    }
    pub fn get_mut<Q>(&mut self, key: Q) -> Option<Leaf<&K, &mut V>>
    where
        Q: IntoIterator<Item = B>,
        B: Ord,
    {
        let mut node = self.root.as_mut()?;
        debug_assert!(!node.is_empty());
        for key in key {
            node = node.get_child_mut(key)?;
            debug_assert!(!node.is_empty());
        }
        node.as_leaf_mut().map(Leaf::as_mut)
    }
    pub fn get_deepest<Q>(&self, key: Q) -> Option<&Node<K, B, V>>
    where
        Q: IntoIterator<Item = B>,
        B: Ord,
    {
        let mut node = self.root.as_ref()?;
        debug_assert!(!node.is_empty());
        for key in key {
            let Some(child) = node.get_child(key) else {
                break;
            };
            node = child;
            debug_assert!(!node.is_empty());
        }
        Some(node)
    }
    pub fn get_deepest_leaf<Q>(&self, key: Q) -> Option<Leaf<&K, &V>>
    where
        Q: IntoIterator<Item = B>,
        B: Ord,
    {
        let mut node = self.root.as_ref()?;
        let mut key = key.into_iter();
        let mut ret = node.as_leaf();
        debug_assert!(!node.is_empty());
        loop {
            let (leaf, child) = node.as_leaf_child(key.next());
            ret = leaf.or(ret);
            if let Some(child) = child {
                node = child;
                debug_assert!(!node.is_empty());
            } else {
                break;
            }
        }
        ret.map(Leaf::as_ref)
    }
    pub fn get_deepest_leaf_mut<Q>(&mut self, key: Q) -> Option<Leaf<&K, &mut V>>
    where
        Q: IntoIterator<Item = B>,
        B: Ord,
    {
        let mut node = self.root.as_mut()?;
        let mut key = key.into_iter();
        let mut ret = None;
        debug_assert!(!node.is_empty());
        loop {
            let (leaf, child) = node.as_leaf_child_mut(key.next());
            ret = leaf.or(ret);
            if let Some(child) = child {
                node = child;
                debug_assert!(!node.is_empty());
            } else {
                break;
            }
        }
        ret.map(Leaf::as_mut)
    }
    pub fn remove<Q>(&mut self, key: Q) -> Option<Leaf<K, V>>
    where
        Q: IntoIterator<Item = B>,
        B: Ord,
    {
        fn remove<Q, K, B, V>(node: &mut Node<K, B, V>, mut key: Q::IntoIter) -> Option<Leaf<K, V>>
        where
            Q: IntoIterator<Item = B>,
            B: Ord,
        {
            let Some(k) = key.next() else {
                debug_assert!(!node.is_empty());
                return node.take_leaf();
            };
            let mut ret = None;
            node.as_branch_mut()?.remove_if(k, |child| {
                ret = remove::<Q, K, B, V>(child, key);
                child.is_empty()
            });
            ret
        }
        let ret = remove::<Q, K, B, V>(self.root.as_mut()?, key.into_iter())?;
        self.root.take_if(|node| node.is_empty());
        Some(ret)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty() {
        let trie: Trie<(), (), ()> = Trie::default();
        assert!(trie.is_empty());
        assert_eq!(trie, Trie { root: None });
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
                root: Some(Node::Leaf(Leaf::new(vec![], ' ')))
            }
        );
        assert_eq!(trie.insert(vec![], '_'), Some(Leaf::new(vec![], ' ')));
        assert_eq!(trie.get([]), Some(Leaf::new(&vec![], &'_')));
        assert_eq!(
            trie,
            Trie {
                root: Some(Node::Leaf(Leaf::new(vec![], '_')))
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
                root: Some(Node::Full(
                    Leaf::new(vec![], '_'),
                    Branch::from_iter([
                        (0, Node::Leaf(Leaf::new(vec![0], '0'))),
                        (1, Node::Leaf(Leaf::new(vec![1], '1')))
                    ])
                ))
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
}
