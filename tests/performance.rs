use buffman_tree::{Key, Leaf, Trie, util::time};
use quickcheck::{Arbitrary, Gen};
use rand::{RngCore, SeedableRng, seq::SliceRandom};
use rand_xoshiro::Xoshiro256PlusPlus;
use std::{
    borrow::Borrow,
    collections::{BTreeMap, HashMap},
    hash::{Hash, RandomState},
    hint::black_box,
    ops::Bound,
    time::Duration,
};

trait MapExt<K, Q: ?Sized, V> {
    fn get_longest_prefix(&self, key: &Q) -> Option<(&K, &V)>;
}
impl<K: Key, Q, V> MapExt<K, Q, V> for Trie<K, V>
where
    Q: Key<Piece = K::Piece>,
{
    fn get_longest_prefix(&self, key: &Q) -> Option<(&K, &V)> {
        self.get_deepest_leaf(key).map(Leaf::unwrap)
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
impl<K: Eq + Hash + Borrow<[Q]>, Q: Eq + Hash, V> MapExt<K, [Q], V> for HashMap<K, V> {
    fn get_longest_prefix(&self, mut key: &[Q]) -> Option<(&K, &V)> {
        loop {
            if let Some(kv) = self.get_key_value(key) {
                return Some(kv);
            }
            key = key.split_last()?.1;
        }
    }
}

fn bench<'a, T: FromIterator<(K, V)> + MapExt<K, Q, V>, K, Q: 'a + ?Sized, V: Clone>(
    entries: impl IntoIterator<Item = (K, V)>,
    searches: impl IntoIterator<Item = &'a Q>,
    reduce: impl FnMut(V, V) -> V,
) -> (Duration, Duration, Option<V>) {
    let (init, map) = time(|| T::from_iter(entries));
    let (run, ret) = time(|| {
        searches
            .into_iter()
            .flat_map(|key| map.get_longest_prefix(key).map(|(_, v)| v.clone()))
            .reduce(reduce)
    });
    (init, run, ret)
}

#[test]
fn performance() {
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

    let btree = bench::<BTreeMap<Box<[char]>, usize>, _, _, _>(
        entries.clone(),
        searches.iter().map(|k| &**k),
        usize::wrapping_add,
    );
    //let hash = time(|| {
    //    bench::<HashMap<Box<[char]>, usize>, _, _, _>(
    //        entries.clone(),
    //        searches.iter().map(|k| &**k),
    //        usize::wrapping_add,
    //    )
    //});
    let trie =
        bench::<Trie<Box<[char]>, usize>, _, _, _>(entries.clone(), &searches, usize::wrapping_add);

    println!("btree:    {btree:?}");
    //println!("hash:     {}mys sum {:?}", hash.0.as_micros(), hash.1);
    println!("trie:     {trie:?}");
    assert_eq!(btree.2, trie.2);
}
