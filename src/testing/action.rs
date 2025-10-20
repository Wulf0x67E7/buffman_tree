use crate::{branch::Branch, testing::BTrie, trie::Trie};
use quickcheck::{Arbitrary, Gen, empty_shrinker, single_shrinker};
use std::{
    borrow::Borrow,
    collections::BTreeMap,
    fmt::Debug,
    ops::{Index, RangeTo},
};

#[derive(Debug, Clone, Copy)]
pub enum Op {
    Empty,
    Len,
    Insert,
    Get,
    GetDeepest,
    Iter,
    Remove,
    Clear,
}
impl Op {
    const WEIGHTED: &[Self] = [
        [Self::Empty; 1],
        [Self::Len; 1],
        [Self::Insert; 1],
        [Self::Get; 1],
        [Self::GetDeepest; 1],
        [Self::Iter; 1],
        [Self::Remove; 1],
        [Self::Clear; 1],
    ]
    .as_flattened();
}
impl Arbitrary for Op {
    fn arbitrary(g: &mut Gen) -> Self {
        g.choose(Self::WEIGHTED).copied().unwrap()
    }
    fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
        match self {
            Op::Empty => empty_shrinker(),
            Op::Len => single_shrinker(Op::Empty),
            Op::Insert => single_shrinker(Op::GetDeepest),
            Op::Get => single_shrinker(Op::Len),
            Op::GetDeepest => single_shrinker(Op::Get),
            Op::Iter => single_shrinker(Op::GetDeepest),
            Op::Remove => single_shrinker(Op::GetDeepest),
            Op::Clear => single_shrinker(Op::Remove),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Action<T> {
    pub op: Op,
    pub item: T,
}
impl<T> Action<T> {
    pub fn new(op: Op, item: T) -> Self {
        Self { op, item }
    }
    pub fn op(&self) -> &Op {
        &self.op
    }
    pub fn item(&self) -> &T {
        &self.item
    }
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

pub trait Consumer<T> {
    type U<'a>: 'a + Debug + PartialEq
    where
        Self: 'a;
    fn consume(&mut self, action: Action<T>) -> Self::U<'_>;
}

#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub enum Return<'a, V> {
    #[default]
    None,
    Bool(bool),
    Num(usize),
    Ref(Option<&'a V>),
    Val(Option<V>),
    Refs(Vec<&'a V>),
    Vals(Vec<V>),
}
impl<'a, V> From<()> for Return<'a, V> {
    fn from((): ()) -> Self {
        Self::None
    }
}
impl<'a, V> From<bool> for Return<'a, V> {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}
impl<'a, V> From<usize> for Return<'a, V> {
    fn from(value: usize) -> Self {
        Self::Num(value)
    }
}
impl<'a, V> From<Option<&'a V>> for Return<'a, V> {
    fn from(value: Option<&'a V>) -> Self {
        Self::Ref(value)
    }
}
impl<'a, V> From<Option<V>> for Return<'a, V> {
    fn from(value: Option<V>) -> Self {
        Self::Val(value)
    }
}
impl<'a, V> FromIterator<&'a V> for Return<'a, V> {
    fn from_iter<T: IntoIterator<Item = &'a V>>(iter: T) -> Self {
        Self::Refs(Vec::from_iter(iter))
    }
}
impl<'a, V> FromIterator<V> for Return<'a, V> {
    fn from_iter<T: IntoIterator<Item = V>>(iter: T) -> Self {
        Self::Vals(Vec::from_iter(iter))
    }
}
impl<K: IntoIterator<Item: PartialEq>, V: Debug + Clone + PartialEq, B: Branch<K::Item, V>>
    Consumer<(K, V)> for Trie<K::Item, V, B>
where
    for<'a> &'a K: IntoIterator<Item = &'a K::Item>,
{
    type U<'a>
        = Return<'a, V>
    where
        Self: 'a;

    fn consume(&mut self, action: Action<(K, V)>) -> Self::U<'_> {
        let Action {
            op,
            item: (key, value),
        } = action;
        match op {
            Op::Empty => self.is_empty().into(),
            Op::Len => self.len().into(),
            Op::Insert => self.insert(key, value).into(),
            Op::Get => self.get(&key).into(),
            Op::GetDeepest => self.get_deepest(&key).into(),
            Op::Iter => self.iter().collect(),
            Op::Remove => self.remove(&key).into(),
            Op::Clear => self.clear().into(),
        }
    }
}

impl<K: Ord + Index<usize, Output: PartialEq> + Index<RangeTo<usize>>, V: Debug + Clone + PartialEq>
    Consumer<(K, V)> for BTreeMap<K, V>
where
    K: Borrow<<K as Index<RangeTo<usize>>>::Output>,
    <K as Index<RangeTo<usize>>>::Output: Ord
        + Index<usize, Output = <K as Index<usize>>::Output>
        + Index<RangeTo<usize>, Output = <K as Index<RangeTo<usize>>>::Output>,
    for<'a> &'a <K as Index<RangeTo<usize>>>::Output:
        IntoIterator<Item = &'a <K as Index<usize>>::Output>,
{
    type U<'a>
        = Return<'a, V>
    where
        Self: 'a;

    fn consume(&mut self, action: Action<(K, V)>) -> Self::U<'_> {
        let Action {
            op,
            item: (key, value),
        } = action;
        match op {
            Op::Empty => self.is_empty().into(),
            Op::Len => self.len().into(),
            Op::Insert => self.insert(key, value).into(),
            Op::Get => self.get(key.borrow()).into(),
            Op::GetDeepest => self.get_deepest(key.borrow()).into(),
            Op::Iter => self.values().collect(),
            Op::Remove => self.remove(key.borrow()).into(),
            Op::Clear => self.clear().into(),
        }
    }
}
