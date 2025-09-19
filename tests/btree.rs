use std::{borrow::Borrow, collections::BTreeMap, hint::black_box, ops::Bound};

use buffman_tree::Trie;
use quickcheck::{Arbitrary, Gen};
use quickcheck_macros::quickcheck;
use rand::{RngCore as _, SeedableRng as _, seq::SliceRandom as _};
use rand_xoshiro::Xoshiro256PlusPlus;

trait MapExt<K, Q: ?Sized, V> {
    fn get_longest_prefix(&self, key: &Q) -> Option<(&K, &V)>;
}
impl<K, Q: IntoIterator<Item: Ord>, V> MapExt<K, Q, V> for Trie<K::Item, (K, V)>
where
    K: IntoIterator<Item: Ord + Borrow<Q::Item>>,
    for<'a> &'a Q: IntoIterator<Item = &'a Q::Item>,
{
    fn get_longest_prefix(&self, key: &Q) -> Option<(&K, &V)> {
        self.get_deepest(key).map(|(k, v)| (k, v))
    }
}
impl<K: Ord + Borrow<[Q]>, Q: PartialEq + Ord, V> MapExt<K, [Q], V> for BTreeMap<K, V> {
    fn get_longest_prefix(&self, mut key: &[Q]) -> Option<(&K, &V)> {
        loop {
            let (k, v) = self
                .range((Bound::Unbounded, Bound::Included(key)))
                .last()?;
            if k.borrow() == key {
                break Some((k, v));
            }
            key = &key[..k
                .borrow()
                .iter()
                .zip(key)
                .take_while(|(a, b)| a == b)
                .count()];
        }
    }
}

fn fuzz_data() -> (Vec<(Box<[char]>, usize)>, Vec<Box<[char]>>) {
    let mut rng = Xoshiro256PlusPlus::seed_from_u64(0);
    let mut g = Gen::new(256);
    let mut generate = || {
        Vec::<(Box<[char]>, usize)>::from_iter((0..1 << 14).map(|x| {
            (
                (0..rng.next_u32() % 512)
                    .map(|_| char::arbitrary(&mut g))
                    .collect(),
                black_box(x),
            )
        }))
    };
    let entries = generate();
    let searches = {
        let mut v: Vec<_> = entries.iter().map(|(k, _)| k).cloned().collect();
        for _ in 0..4 {
            v.extend(generate().into_iter().map(|(k, _)| k));
        }
        v.shuffle(&mut rng);
        v
    };
    (entries, searches)
}

#[test]
fn btree() {}
