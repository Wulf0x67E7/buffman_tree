use buffman_tree::{Trie, util::unzipped};
use quickcheck_macros::quickcheck;
use std::{
    collections::BTreeSet,
    iter::{repeat, zip},
};

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
