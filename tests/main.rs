use buffman_tree::{ByteString, Trie};
use quickcheck::{Arbitrary, Gen};
use std::{
    collections::{BTreeMap, HashMap},
    hash::RandomState,
};

#[test]
fn performance() {
    let mut g = Gen::new(2 ^ 16);
    let kvs = Vec::<(String, ())>::arbitrary(&mut g);

    let btree = BTreeMap::from_iter(kvs.clone());
    let hash: HashMap<String, (), RandomState> = HashMap::from_iter(kvs.clone());
    let trie: Trie<ByteString, ()> =
        Trie::from_iter(kvs.iter().map(|(s, ())| (s.clone().into(), ())));
}
