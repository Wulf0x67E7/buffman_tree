use buffman_tree::{Trie, testing::BTrie, util::time};
use quickcheck::{Arbitrary, Gen};
use rand::{RngCore, SeedableRng, seq::SliceRandom};
use rand_xoshiro::Xoshiro256PlusPlus;
use std::{borrow::Borrow, collections::BTreeMap, hint::black_box, time::Duration};

trait MapExt<Q: ?Sized, V> {
    fn get_longest_prefix(&self, key: &Q) -> Option<&V>;
}
impl<K: Ord, Q: IntoIterator<Item: Ord>, V> MapExt<Q, V> for Trie<K, V>
where
    for<'a> &'a Q: IntoIterator<Item = &'a Q::Item>,
    K: Borrow<Q::Item>,
{
    fn get_longest_prefix(&self, key: &Q) -> Option<&V> {
        self.get_deepest(key)
    }
}
impl<K: Ord + Borrow<[Q]>, Q: PartialEq + Ord, V> MapExt<[Q], V> for BTreeMap<K, V>
where
    Self: BTrie<K, V>,
{
    fn get_longest_prefix(&self, key: &[Q]) -> Option<&V> {
        self.get_deepest(key)
    }
}

fn bench<'a, T: FromIterator<(K, V)> + MapExt<Q, V>, K, Q: 'a + ?Sized, V: Clone>(
    entries: impl IntoIterator<Item = (K, V)>,
    searches: impl IntoIterator<Item = &'a Q>,
    reduce: impl FnMut(V, V) -> V,
) -> (Duration, Duration, Option<V>) {
    let (init, map) = time(|| T::from_iter(entries));
    let (run, ret) = time(|| {
        searches
            .into_iter()
            .flat_map(|key| map.get_longest_prefix(key).cloned())
            .reduce(reduce)
    });
    (init, run, ret)
}

#[test]
fn performance() {
    let mut rng = Xoshiro256PlusPlus::seed_from_u64(0);
    let mut g = Gen::new(256);
    let mut generate = || {
        Vec::<(Box<[char]>, usize)>::from_iter((0..1 << 16).map(|x| {
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
    let trie = bench::<Trie<char, usize>, _, _, _>(entries.clone(), &searches, usize::wrapping_add);

    println!("btree:    {btree:?}");
    println!("trie2:    {trie:?}");

    assert_eq!(btree.2, trie.2);
}
