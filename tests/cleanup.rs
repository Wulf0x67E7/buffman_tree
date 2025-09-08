use std::{collections::BTreeSet, iter::repeat};

use quickcheck::TestResult;
use quickcheck_macros::quickcheck;
use rand::{SeedableRng, seq::SliceRandom};
use rand_xoshiro::Xoshiro256PlusPlus as Rng;

use buffman_tree::Leaf;
use buffman_tree::Trie;

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

pub type Condition =
    fn(&Box<[Box<[u8]>]>, &Trie<Box<[u8]>, ()>, &Box<[Box<[u8]>]>) -> Option<String>;
macro_rules! cond {
    (|$arg:ident, $trie:ident, $res:ident| $x:stmt; !$pred:expr => $err:expr) => {
        #[allow(unused_variables)]
        |$arg: &Box<[Box<[u8]>]>, $trie: &Trie<Box<[u8]>, ()>, $res: &Box<[Box<[u8]>]>| {
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
                .flat_map(|v| trie.remove(v.clone()))
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
