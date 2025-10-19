use std::mem::{replace, take};

use crate::{NodeDebug, trie::handle::Handle};

pub type LeafHandle<V> = Handle<Leaf<V>>;

#[derive(Debug)]
pub struct Leaf<V> {
    value: V,
    #[cfg(feature = "testing")]
    owner: usize,
}
impl<V: Default> Default for Leaf<V> {
    fn default() -> Self {
        Self {
            value: Default::default(),
            #[cfg(feature = "testing")]
            owner: Handle::<()>::new_null()._unwrap(),
        }
    }
}
impl<K, V, B> NodeDebug<K, V, B> for Leaf<V> {
    fn default_with_owner(
        #[cfg(feature = "testing")] owner: super::node::NodeHandle<K, V, B>,
    ) -> Self
    where
        Self: Default,
    {
        Self {
            #[cfg(feature = "testing")]
            owner: owner._unwrap(),
            ..Default::default()
        }
    }
    fn debug<'a>(&'a self, _: &'a super::Trie<K, V, B>) -> impl 'a + std::fmt::Debug
    where
        K: std::fmt::Debug,
        V: std::fmt::Debug,
    {
        &self.value
    }
    fn set_owner(
        &mut self,
        owner: super::node::NodeHandle<K, V, B>,
    ) -> super::node::NodeHandle<K, V, B> {
        use std::mem::replace;
        Handle::from(replace(&mut self.owner, owner._unwrap()))
    }
}
impl<V> Leaf<V> {
    pub fn new<#[cfg(feature = "testing")] K, #[cfg(feature = "testing")] B>(
        #[cfg(feature = "testing")] owner: super::node::NodeHandle<K, V, B>,
        value: V,
    ) -> Self {
        Self {
            value,
            #[cfg(feature = "testing")]
            owner: owner._unwrap(),
        }
    }
    pub fn get(&self) -> &V {
        &self.value
    }
    pub fn get_mut(&mut self) -> &mut V {
        &mut self.value
    }
    pub fn replace(&mut self, value: V) -> V {
        replace(&mut self.value, value)
    }
    pub fn unwrap(self) -> V {
        self.value
    }
    pub fn _take(&mut self) -> V
    where
        V: Default,
    {
        take(&mut self.value)
    }
}
