use std::fmt::Debug;

#[derive(Default, PartialEq)]
pub struct Leaf<K, V> {
    key: K,
    value: V,
}
impl<K: Debug, V: Debug> Debug for Leaf<K, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("")
            .field("key", &self.key)
            .field("value", &self.value)
            .finish()
    }
}
impl<K: From<L>, V: From<W>, L, W> From<(L, W)> for Leaf<K, V> {
    fn from((key, value): (L, W)) -> Self {
        Self::from(key, value)
    }
}
impl<K, V> Leaf<K, V> {
    pub fn from<L, W>(key: L, value: W) -> Self
    where
        K: From<L>,
        V: From<W>,
    {
        Self {
            key: K::from(key),
            value: V::from(value),
        }
    }
    pub fn new(key: K, value: V) -> Self {
        Self { key, value }
    }
    pub fn key(&self) -> &K {
        &self.key
    }
    pub fn value(&self) -> &V {
        &self.value
    }
    pub fn value_mut(&mut self) -> &mut V {
        &mut self.value
    }
    pub fn as_ref(&self) -> Leaf<&K, &V> {
        let Leaf { key, value } = self;
        Leaf { key, value }
    }
    pub fn as_mut(&mut self) -> Leaf<&K, &mut V> {
        let Leaf { key, value } = self;
        Leaf { key, value }
    }
    pub fn into_key(self) -> K {
        self.key
    }
    pub fn into_value(self) -> V {
        self.value
    }
    pub fn unwrap(self) -> (K, V) {
        self.into()
    }
    pub fn into<L, W>(self) -> (L, W)
    where
        K: Into<L>,
        V: Into<W>,
    {
        let Leaf { key, value } = self;
        (key.into(), value.into())
    }
}
