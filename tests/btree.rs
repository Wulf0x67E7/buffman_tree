use buffman_tree::{Trie, testing::Procedure};
use quickcheck::TestResult;
use std::collections::BTreeMap;

#[test]
fn btree_oracle() {
    fn test(proc: Procedure<(Vec<u8>, usize)>) -> TestResult {
        proc.run::<BTreeMap<_, _>, Trie<_, _>>()
    }
    quickcheck::QuickCheck::new()
        .tests(0x400)
        .quickcheck(test as fn(Procedure<(Vec<u8>, usize)>) -> TestResult);
}
