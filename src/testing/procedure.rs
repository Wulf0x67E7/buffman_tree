use crate::{
    testing::{Action, Consumer, Op},
    util::debug_fn,
};
use quickcheck::{Arbitrary, Gen, TestResult, empty_shrinker};
use std::{
    collections::{BTreeMap, HashMap},
    fmt::Debug,
    hash::Hash,
};

#[derive(Clone)]
pub struct Procedure<T> {
    actions: Vec<Action<usize>>,
    items: Vec<T>,
}
impl<T: Debug> Debug for Procedure<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Procedure")
            .field(&debug_fn(|f| {
                f.debug_list()
                    .entries(self.actions.iter().map(|action| {
                        debug_fn(|f| {
                            f.debug_struct("Action")
                                .field("op", action.op())
                                .field("item", &self.items[action.item])
                                .finish()
                        })
                    }))
                    .finish()
            }))
            .finish()
    }
}
impl<T> Default for Procedure<T> {
    fn default() -> Self {
        Self {
            actions: Default::default(),
            items: Default::default(),
        }
    }
}
impl<T: Eq + Hash> FromIterator<Action<T>> for Procedure<T> {
    fn from_iter<Iter: IntoIterator<Item = Action<T>>>(iter: Iter) -> Self {
        let mut index = HashMap::new();
        let actions = iter
            .into_iter()
            .map(|action| {
                action.map_item(|item| {
                    let len = index.len();
                    *index.entry(item).or_insert(len)
                })
            })
            .collect();
        let mut items = index.into_iter().collect::<Vec<_>>();
        items.sort_by_key(|(_, idx)| *idx);
        let items = items.into_iter().map(|(item, _)| item).collect();
        Self { actions, items }
    }
}
impl<T: Arbitrary> Procedure<T> {
    fn gen_action(&mut self, g: &mut Gen) -> Action<usize> {
        let op = Op::arbitrary(g);
        let end = self.items.len();
        let item = if end == 0 {
            self.items.push(T::arbitrary(g));
            end
        } else {
            let index: usize = usize::arbitrary(g) % (2 * end);
            if let Op::Insert = op
                && index >= end
            {
                self.items.push(T::arbitrary(g));
                end
            } else {
                index % end
            }
        };
        Action { op, item }
    }
    fn pack_items(&mut self) -> Self {
        let mut packed_actions = vec![];
        let mut packed_items = vec![];
        let mut indices_map = BTreeMap::new();
        for Action {
            op,
            item: old_index,
        } in self.actions.iter().cloned()
        {
            let new_index = *indices_map
                .entry(old_index)
                .or_insert_with_key(|&old_index| {
                    let new_index = packed_items.len();
                    packed_items.push(self.items[old_index].clone());
                    new_index
                });
            packed_actions.push(Action {
                op,
                item: new_index,
            });
        }
        Self {
            actions: packed_actions,
            items: packed_items,
        }
    }
    fn shrink_actions(&self) -> impl use<T> + Iterator<Item = Self> {
        let Self { actions, items } = self.clone();
        actions.shrink().map({
            move |actions| {
                Self {
                    actions,
                    items: items.clone(),
                }
                .pack_items()
            }
        })
    }
    fn shrink_items(&self) -> impl use<T> + Iterator<Item = Self> {
        let Self { actions, items } = self.clone();
        let mut idx = 0;
        let mut es = if let Some(es) = items.get(idx) {
            es.shrink()
        } else {
            empty_shrinker()
        };
        std::iter::from_fn(move || {
            loop {
                match es.next() {
                    Some(e) => {
                        let mut items = items.clone();
                        items[idx] = e;
                        break Some(Self {
                            actions: actions.clone(),
                            items,
                        });
                    }
                    None => {
                        idx += 1;
                        es = items.get(idx)?.shrink();
                    }
                }
            }
        })
    }
    pub fn run<O, S>(&self) -> TestResult
    where
        O: Default + Consumer<T>,
        S: Default + Consumer<T>,
        for<'a> O::U<'a>: PartialEq<S::U<'a>>,
    {
        let mut oracle = O::default();
        let mut student = S::default();
        for action in self
            .actions
            .iter()
            .map(|action| action.map_item(|index| &self.items[index]))
        {
            let (oracle, student) = (
                oracle.consume(action.cloned()),
                student.consume(action.cloned()),
            );
            if oracle != student {
                return TestResult::error(format!("oracle != student : {oracle:?} != {student:?}"));
            }
        }
        TestResult::passed()
    }
    pub fn len(&self) -> usize {
        self.actions.len()
    }
    pub fn actions(&self) -> impl Iterator<Item = Action<&T>> {
        self.actions
            .iter()
            .map(|action| action.map_item(|item| &self.items[item]))
    }
}
impl<T: Arbitrary> Arbitrary for Procedure<T> {
    fn arbitrary(g: &mut Gen) -> Self {
        let len = g.size();
        let mut this = Self::default();
        for _ in 0..len {
            let action = this.gen_action(g);
            this.actions.push(action);
        }
        this
    }
    fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
        Box::new(Iterator::chain(self.shrink_actions(), self.shrink_items()))
    }
}

#[test]
fn procedure_shrinking() {
    fn test(proc: Procedure<(Vec<u8>, usize)>) -> TestResult {
        if proc.actions().any(|action| action.item().0.len() > 3) {
            TestResult::failed()
        } else {
            TestResult::passed()
        }
    }
    let ret = quickcheck::QuickCheck::new()
        .quicktest(test as fn(Procedure<(Vec<u8>, usize)>) -> TestResult)
        .unwrap_err();
    let proc = "TestResult { status: Fail, arguments: [\"Procedure([Action { op: Empty, item: ([0, 0, 0, 0], 0) }])\"], err: None }";
    assert_eq!(format!("{ret:?}"), format!("{proc}"));
}
