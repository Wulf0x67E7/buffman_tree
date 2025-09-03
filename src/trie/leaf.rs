#[derive(Debug, Default, PartialEq)]
pub struct Leaf<K, V> {
    key: K,
    value: V,
}
impl<K: From<L>, V: From<W>, L, W> From<(L, W)> for Leaf<K, V> {
    fn from((key, value): (L, W)) -> Self {
        Self::new(key, value)
    }
}
impl<K, V> Leaf<K, V> {
    pub fn new<L, W>(key: L, value: W) -> Self
    where
        K: From<L>,
        V: From<W>,
    {
        Self {
            key: K::from(key),
            value: V::from(value),
        }
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
}
