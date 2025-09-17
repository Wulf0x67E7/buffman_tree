#![cfg(test)]

use std::{
    borrow::Borrow,
    collections::{BTreeMap, BTreeSet},
    fmt::Debug,
    ops::{Index, RangeTo},
};

use quickcheck::{Arbitrary, Gen, TestResult, empty_shrinker, single_shrinker};
use quickcheck_macros::quickcheck;
use slab::Slab;

use crate::{
    trie2::Trie,
    util::{BTrie, debug_fn},
};

#[derive(Debug, Clone, Copy)]
pub enum Op {
    Insert,
    Get,
    GetDeepest,
    Remove,
}
impl Op {
    const WEIGHTED: &[Self] = [
        [Self::Insert; 1],
        [Self::Get; 1],
        [Self::GetDeepest; 1],
        [Self::Remove; 1],
    ]
    .as_flattened();
}
impl Arbitrary for Op {
    fn arbitrary(g: &mut Gen) -> Self {
        g.choose(Self::WEIGHTED).copied().unwrap()
    }
    fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
        match self {
            Op::Insert => single_shrinker(Op::GetDeepest),
            Op::Get => empty_shrinker(),
            Op::GetDeepest => single_shrinker(Op::Get),
            Op::Remove => single_shrinker(Op::GetDeepest),
        }
    }
}
#[derive(Debug, Clone, Copy)]
pub struct Action<T> {
    op: Op,
    item: T,
}
impl<T: Arbitrary> Arbitrary for Action<T> {
    fn arbitrary(g: &mut Gen) -> Self {
        Self {
            op: Op::arbitrary(g),
            item: T::arbitrary(g),
        }
    }
    fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
        let item = self.item.clone();
        Box::new(self.op.shrink().map(move |op| Action {
            op,
            item: item.clone(),
        }))
    }
}
impl<T> Action<T> {
    pub fn map_item<U>(self, f: impl FnOnce(T) -> U) -> Action<U> {
        Action {
            op: self.op,
            item: f(self.item),
        }
    }
}
impl<T: Clone> Action<&T> {
    pub fn cloned(self) -> Action<T> {
        Action {
            op: self.op,
            item: self.item.clone(),
        }
    }
}
#[derive(Clone)]
pub struct Procedure<T> {
    actions: Vec<Action<usize>>,
    items: Vec<T>,
}
impl<T: Debug> Debug for Procedure<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Procedure")
            .field(&debug_fn(|f: &mut std::fmt::Formatter<'_>| {
                f.debug_list()
                    .entries(self.actions.iter().map(|action| {
                        debug_fn(|f: &mut std::fmt::Formatter<'_>| {
                            f.debug_struct("Action")
                                .field("op", &action.op)
                                .field("item", &self.items.get(action.item))
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
impl<T: Arbitrary> Procedure<T> {
    fn gen_action(&mut self, g: &mut Gen) -> Action<usize> {
        let op = Op::arbitrary(g);
        let end = self.items.len();
        let item = {
            let index = usize::arbitrary(g) % (2 * end);
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
    pub fn run<O, S>(&self) -> TestResult
    where
        O: 'static + Default + Consumer<T>,
        for<'a> S: 'static + Default + Consumer<T, U<'a> = O::U<'a>>,
    {
        let mut oracle = O::default();
        let mut student = S::default();
        for action in self
            .actions
            .iter()
            .map(|action| action.map_item(|index| &self.items[index]))
        {
            if oracle.consume(action.cloned()) != student.consume(action.cloned()) {
                return TestResult::failed();
            }
        }
        TestResult::passed()
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
        let Self { actions, items } = self.clone();
        Box::new(actions.shrink().map(move |actions| {
            Self {
                actions,
                items: items.clone(),
            }
            .pack_items()
        }))
    }
}

pub trait Consumer<T> {
    type U<'a>: 'a + Debug + PartialEq
    where
        Self: 'a;
    fn consume(&mut self, action: Action<T>) -> Self::U<'_>;
}

impl<K: IntoIterator<Item: Ord>, V: Debug + Clone + PartialEq> Consumer<(K, V)> for Trie<K::Item, V>
where
    for<'a> &'a K: IntoIterator<Item = &'a K::Item>,
{
    type U<'a>
        = Option<V>
    where
        Self: 'a;

    fn consume(&mut self, action: Action<(K, V)>) -> Self::U<'_> {
        let Action {
            op,
            item: (key, value),
        } = action;
        match op {
            Op::Insert => self.insert(key, value),
            Op::Get => self.get(&key).cloned(),
            Op::GetDeepest => self.get_deepest(&key).cloned(),
            Op::Remove => self.remove(&key),
        }
    }
}

impl<
    K: Ord + Index<RangeTo<usize>, Output = K> + Index<usize, Output: PartialEq>,
    V: Debug + Clone + PartialEq,
> Consumer<(K, V)> for BTreeMap<K, V>
where
    Self: BTrie<K, V>,
    for<'a> &'a K: IntoIterator<Item = &'a <K as Index<usize>>::Output>,
{
    type U<'a>
        = Option<V>
    where
        Self: 'a;

    fn consume(&mut self, action: Action<(K, V)>) -> Self::U<'_> {
        let Action {
            op,
            item: (key, value),
        } = action;
        match op {
            Op::Insert => self.insert(key, value),
            Op::Get => self.get(&key).cloned(),
            Op::GetDeepest => self.get_deepest(&key).cloned(),
            Op::Remove => self.remove(&key),
        }
    }
}
