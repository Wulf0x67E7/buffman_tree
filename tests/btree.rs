use std::collections::BTreeMap;

use buffman_tree::{Trie, testing::Procedure};
use quickcheck::TestResult;
use quickcheck_macros::quickcheck;

#[quickcheck]
fn btree(proc: Procedure<(Vec<u8>, usize)>) -> TestResult {
    proc.run::<BTreeMap<_, _>, Trie<_, _>>()
}
