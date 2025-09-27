use std::mem::{replace, take};

use crate::trie::handle::Handle;

pub type LeafHandle<V> = Handle<Leaf<V>>;

#[derive(Debug)]
pub struct Leaf<V> {
    value: V,
    #[cfg(feature = "testing")]
    this: usize,
}
impl<V: Default> Default for Leaf<V> {
    fn default() -> Self {
        Self {
            value: Default::default(),
            #[cfg(feature = "testing")]
            this: Handle::<()>::new_null()._unwrap(),
        }
    }
}

impl<V> Leaf<V> {
    pub fn new<#[cfg(feature = "testing")] K>(
        value: V,
        #[cfg(feature = "testing")] this: super::node::NodeHandle<K, V>,
    ) -> Self {
        Self {
            value,
            #[cfg(feature = "testing")]
            this: this._unwrap(),
        }
    }
    #[cfg(feature = "testing")]
    pub fn _this<K>(&self) -> super::node::NodeHandle<K, V> {
        Handle::from(self.this)
    }
    #[cfg(feature = "testing")]
    pub fn set_this<K>(
        &mut self,
        this: super::node::NodeHandle<K, V>,
    ) -> super::node::NodeHandle<K, V> {
        use std::mem::replace;
        Handle::from(replace(&mut self.this, this._unwrap()))
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
