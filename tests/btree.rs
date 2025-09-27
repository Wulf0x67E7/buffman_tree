use buffman_tree::{
    Trie,
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
        .quickcheck(test as fn(Procedure<(Vec<u8>, usize)>) -> TestResult);
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
        let dbg_str = format!("{case:#?}");
        let res = catch_unwind(|| test(case));
        match res {
            Ok(res) if res.is_failure() => panic!("Failed case: {dbg_str}\nResult: {res:#?}"),
            Ok(_) => (),
            Err(err) => {
                println!("Failed case: {dbg_str}\nError: {err:#?}");
                resume_unwind(err)
            }
        }
    }
}

fn test(proc: Procedure<(Vec<u8>, usize)>) -> TestResult {
    proc.run::<BTreeMap<_, _>, Trie<_, _>>()
}
