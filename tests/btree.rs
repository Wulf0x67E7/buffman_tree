use buffman_tree::{
    Trie,
    branch::{BTreeBranch, Branch, ByteBranch},
    testing::{Action, Op, Procedure},
};
use quickcheck::TestResult;
use std::{
    collections::BTreeMap,
    panic::{catch_unwind, resume_unwind},
};

#[test]
fn btree_oracle() {
    quickcheck::QuickCheck::new()
        .tests(0x400)
        .quickcheck(test::<BTreeBranch<_, _>> as fn(Procedure<(Vec<u8>, usize)>) -> TestResult);
    quickcheck::QuickCheck::new()
        .tests(0x400)
        .quickcheck(test::<ByteBranch<_>> as fn(Procedure<(Vec<u8>, usize)>) -> TestResult);
}

#[test]
fn btree_oracle_cases() {
    let cases = [
        Procedure::from_iter([
            Action {
                op: Op::Insert,
                item: (vec![0], 0),
            },
            Action {
                op: Op::Insert,
                item: (vec![], 0),
            },
            Action {
                op: Op::Remove,
                item: (vec![], 0),
            },
            Action {
                op: Op::Insert,
                item: (vec![], 0),
            },
        ]),
        Procedure::from_iter([
            Action {
                op: Op::Insert,
                item: (vec![195], 0),
            },
            Action {
                op: Op::Insert,
                item: (vec![195, 0], 0),
            },
            Action {
                op: Op::Insert,
                item: (vec![], 0),
            },
        ]),
    ];
    for case in cases {
        test_case::<BTreeBranch<_, _>>(case.clone());
        test_case::<ByteBranch<_>>(case);
    }
}

fn test_case<B: 'static + Branch<u8, usize>>(case: Procedure<(Vec<u8>, usize)>) {
    let dbg_str = format!("{case:#?}");
    let res = catch_unwind(|| test::<B>(case));
    match res {
        Ok(res) if res.is_failure() => panic!("Failed case: {dbg_str}\nResult: {res:#?}"),
        Ok(_) => (),
        Err(err) => {
            println!("Failed case: {dbg_str}\nError: {err:#?}");
            resume_unwind(err)
        }
    }
}
fn test<B: 'static + Branch<u8, usize>>(proc: Procedure<(Vec<u8>, usize)>) -> TestResult {
    proc.run::<BTreeMap<_, _>, Trie<_, _, B>>()
}
