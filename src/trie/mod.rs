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
pub struct Trie<K, S, V> {
    root: Option<NodeId<K, S, V>>,
    shared: Slab<Node<K, S, V>>,
}
impl<K: Debug, S: Debug, V: Debug> std::fmt::Debug for Trie<K, S, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut walk = Walk::start(&self.root, Ordered);
        let mut f = &mut f.debug_struct("Trie");
        while let Some(node) = walk.next(&self.shared) {
            f = f.field(&node.to_string(), node.get(&self.shared));
        }
        f.finish()
    }
}
impl<K: PartialEq, S, V: PartialEq> PartialEq for Trie<K, S, V> {
    fn eq(&self, other: &Self) -> bool {
        self.iter().zip(other.iter()).all(|(a, b)| a == b)
    }
}
impl<K, S, V> Default for Trie<K, S, V> {
    fn default() -> Self {
        Self {
            root: None,
            shared: Slab::new(),
        }
    }
}
impl<K, S, V> FromIterator<(K, V)> for Trie<K, S, V>
where
    K: PartialEq,
    for<'a> &'a K: IntoIterator<Item = &'a S>,
    S: Clone + Ord,
{
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        let mut ret = Self::default();
        for (key, value) in iter {
            ret.insert(key, value);
        }
        ret
    }
}
impl<K, S, V> Trie<K, S, V> {
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
        for<'a> &'a K: IntoIterator<Item = &'a S>,
        S: Ord + Clone,
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
        Q: IntoIterator<Item = S>,
        S: Ord,
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
        Q: IntoIterator<Item = S>,
        S: Ord,
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
    pub fn get_deepest<Q>(&self, key: Q) -> Option<&Node<K, S, V>>
    where
        Q: IntoIterator<Item = S>,
        S: Ord,
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
        Q: IntoIterator<Item = S>,
        S: Ord,
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
        Q: IntoIterator<Item = S>,
        S: Ord,
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
        Q: IntoIterator<Item = S>,
        S: Ord + Clone,
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
        S: Ord,
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

    use super::*;
    use crate::{trie::case::Case, util::unzipped};
    use quickcheck::TestResult;
    use quickcheck_macros::quickcheck;
    use std::{
        collections::BTreeSet,
        iter::{repeat, zip},
    };

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
    fn remove_cleanup() {
        // found by remove_cleanup_fuzz, now fixed
        let cases = [Case::from((0, [[0]])), Case::from((0, [[0, 2]]))];
        let mut success = true;
        for case in cases {
            let result = case.clone().check();
            if result.is_failure() || result.is_error() {
                success = false;
                println!("crate::tests::remove_cleanup failed for case: {case:?}");
            }
        }
        assert!(success);
    }
    #[quickcheck]
    fn remove_cleanup_fuzz(data: BTreeSet<Vec<u8>>, shuffle_seed: u64) -> TestResult {
        Case::from((shuffle_seed, data)).check()
    }
}

#[cfg(test)]
mod case {
    use std::{collections::BTreeSet, iter::repeat};

    use quickcheck::TestResult;
    use rand::{SeedableRng, seq::SliceRandom};
    use rand_xoshiro::Xoshiro256PlusPlus as Rng;

    use crate::{Leaf, Trie};

    pub type Condition =
        fn(&Box<[Box<[u8]>]>, &Trie<Box<[u8]>, u8, ()>, &Box<[Box<[u8]>]>) -> Option<String>;
    macro_rules! cond {
        (|$arg:ident, $trie:ident, $res:ident| $x:stmt; !$pred:expr => $err:expr) => {
            #[allow(unused_variables)]
            |$arg: &Box<[Box<[u8]>]>, $trie: &Trie<Box<[u8]>, u8, ()>, $res: &Box<[Box<[u8]>]>| {
                $x(!$pred).then(|| format!($err))
            }
        };
    }
    macro_rules! conditions {
        (|$arg:ident, $trie:ident, $res:ident|[$( $x:stmt; !$pred:expr => $err:expr ),+$(,)?]) => {
            [$(cond!(|$arg,$trie,$res| $x; !$pred => $err)),+]
        };
    }
    #[derive(Debug, Clone)]
    pub struct Case {
        rng: Rng,
        values: BTreeSet<Box<[u8]>>,
    }
    impl<I> From<(u64, I)> for Case
    where
        I: IntoIterator,
        Box<[u8]>: From<I::Item>,
    {
        fn from((seed, values): (u64, I)) -> Self {
            Self {
                rng: Rng::seed_from_u64(seed),
                values: BTreeSet::from_iter(values.into_iter().map(I::Item::into)),
            }
        }
    }
    impl Case {
        pub fn check(mut self) -> TestResult {
            let mut arg = Box::from_iter(self.values);
            arg.shuffle(&mut self.rng);
            let mut trie = Trie::from_iter(arg.iter().cloned().zip(repeat(())));
            arg.shuffle(&mut self.rng);
            let res = Box::from_iter(
                arg.iter()
                    .flat_map(|v| trie.remove(v.iter().cloned()))
                    .map(Leaf::into_key),
            );
            let conditions = conditions!(|arg,trie,res| [
                (); !trie.is_empty() => "Trie::is_empty == false",
                let default = Trie::default(); !trie == &default => "{trie:?} != Trie::default() == {default:?}",
                let (arg_len, res_len) = (arg.len(), res.len()); !arg_len == res_len => "arg_len != res_len --- {arg_len} != {res_len}",
                (); !arg == res => "arg != res --- {arg:?} != {res:?}",
            ]);
            conditions
                .iter()
                .find_map(|predicate| predicate(&arg, &trie, &res))
                .map(TestResult::error)
                .unwrap_or_else(TestResult::passed)
        }
    }
}
