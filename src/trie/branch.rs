use crate::{
    NodeDebug,
    trie::{
        Handle, Trie,
        node::{Node, NodeHandle},
    },
    util::debug_fn,
};
use std::{
    array::from_fn,
    borrow::Borrow,
    collections::BTreeMap,
    fmt::Debug,
    mem::{replace, take},
};

pub trait Branch<K, V, Q = K>: Sized + Default + NodeDebug<K, V, Self> {
    fn is_empty(&self) -> bool;
    fn insert(&mut self, key: K, node: NodeHandle<K, V, Self>) -> Option<NodeHandle<K, V, Self>>;
    fn get_or_insert_with(
        &mut self,
        key: K,
        f: impl FnOnce() -> NodeHandle<K, V, Self>,
    ) -> NodeHandle<K, V, Self>;
    fn get(&self, key: &Q) -> Option<NodeHandle<K, V, Self>>;
    fn cleanup(&mut self, f: impl FnMut(&mut NodeHandle<K, V, Self>) -> bool) -> usize;
    fn prune(
        &mut self,
        f: impl FnMut(&mut NodeHandle<K, V, Self>) -> bool,
    ) -> Option<Option<(K, NodeHandle<K, V, Self>)>>;

    fn iter<'a>(&'a self) -> impl Iterator<Item = (&'a K, NodeHandle<K, V, Self>)>
    where
        K: 'a;
    fn keys<'a>(&'a self) -> impl Iterator<Item = &'a K>
    where
        K: 'a,
    {
        self.iter().map(|(k, _)| k)
    }
    fn values<'a>(&'a self) -> impl Iterator<Item = NodeHandle<K, V, Self>>
    where
        K: 'a,
    {
        self.iter().map(|(_, v)| v)
    }
}

#[derive(Debug)]
pub struct BTreeBranch<K, V> {
    map: BTreeMap<K, NodeHandle<K, V, Self>>,
    #[cfg(feature = "testing")]
    owner: NodeHandle<K, V, Self>,
}
impl<K, V> Default for BTreeBranch<K, V> {
    fn default() -> Self {
        Self {
            map: Default::default(),
            #[cfg(feature = "testing")]
            owner: NodeHandle::new_null(),
        }
    }
}
impl<K, V> NodeDebug<K, V, Self> for BTreeBranch<K, V> {
    fn default_with_owner(#[cfg(feature = "testing")] owner: NodeHandle<K, V, Self>) -> Self {
        Self {
            map: Default::default(),
            #[cfg(feature = "testing")]
            owner,
        }
    }
    fn debug<'a>(&'a self, trie: &'a Trie<K, V, Self>) -> impl 'a + Debug
    where
        K: Debug,
        V: Debug,
    {
        debug_fn(|f| {
            let mut f = f.debug_list();
            f.entries(
                self.map
                    .iter()
                    .map(|(k, v)| (k, v.get(&trie.nodes).debug(trie))),
            );
            f.finish()
        })
    }
    #[cfg(feature = "testing")]
    fn set_owner(&mut self, owner: NodeHandle<K, V, Self>) -> NodeHandle<K, V, Self> {
        use std::mem::replace;
        replace(&mut self.owner, owner)
    }
}
impl<K: Ord + Borrow<Q>, V, Q: Ord> Branch<K, V, Q> for BTreeBranch<K, V> {
    fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
    fn insert(&mut self, key: K, node: NodeHandle<K, V, Self>) -> Option<NodeHandle<K, V, Self>> {
        self.map.insert(key, node)
    }
    fn get_or_insert_with(
        &mut self,
        key: K,
        f: impl FnOnce() -> NodeHandle<K, V, Self>,
    ) -> NodeHandle<K, V, Self> {
        self.map.entry(key).or_insert_with(f).leak()
    }
    fn get(&self, key: &Q) -> Option<NodeHandle<K, V, Self>> {
        self.map.get(key).map(Handle::leak)
    }
    fn cleanup(&mut self, mut f: impl FnMut(&mut NodeHandle<K, V, Self>) -> bool) -> usize {
        self.map.retain(|_, node| !f(node));
        self.map.len()
    }
    fn prune(
        &mut self,
        f: impl FnMut(&mut NodeHandle<K, V, Self>) -> bool,
    ) -> Option<Option<(K, NodeHandle<K, V, Self>)>> {
        match self.cleanup(f) {
            0 => Some(None),
            1 => Some(take(&mut self.map).into_iter().last()),
            _ => None,
        }
    }

    fn iter<'a>(&'a self) -> impl Iterator<Item = (&'a K, Handle<Node<K, V, Self>>)>
    where
        K: 'a,
    {
        self.map.iter().map(|(k, v)| (k, v.leak()))
    }
}

pub struct ByteBranch<V> {
    map: [NodeHandle<u8, V, Self>; 0x100],
    #[cfg(feature = "testing")]
    owner: NodeHandle<u8, V, Self>,
}
impl<V> Default for ByteBranch<V> {
    fn default() -> Self {
        Self {
            map: std::array::from_fn(|_| Handle::new_null()),
            owner: Handle::new_null(),
        }
    }
}
impl<V> NodeDebug<u8, V, Self> for ByteBranch<V> {
    fn default_with_owner(#[cfg(feature = "testing")] owner: NodeHandle<u8, V, Self>) -> Self
    where
        Self: Default,
    {
        Self {
            map: from_fn(|_| Handle::new_null()),
            owner,
        }
    }
    fn debug<'a>(&'a self, trie: &'a Trie<u8, V, Self>) -> impl 'a + Debug
    where
        u8: Debug,
        V: Debug,
    {
        debug_fn(|fmt| {
            fmt.debug_map()
                .entries(self.map.iter().enumerate().filter_map(|(k, node)| {
                    (!node.is_null()).then(|| (k, node.get(&trie.nodes).debug(trie)))
                }))
                .finish()
        })
    }
    #[cfg(feature = "testing")]
    fn set_owner(&mut self, owner: NodeHandle<u8, V, Self>) -> NodeHandle<u8, V, Self> {
        use std::mem::replace;
        replace(&mut self.owner, owner)
    }
}
impl<V> Branch<u8, V> for ByteBranch<V> {
    fn is_empty(&self) -> bool {
        self.map.iter().all(|node| node.is_null())
    }
    fn insert(
        &mut self,
        key: u8,
        node: NodeHandle<u8, V, Self>,
    ) -> Option<NodeHandle<u8, V, Self>> {
        replace(&mut self.map[key as usize], node).valid()
    }
    fn get_or_insert_with(
        &mut self,
        key: u8,
        f: impl FnOnce() -> NodeHandle<u8, V, Self>,
    ) -> NodeHandle<u8, V, Self> {
        let node = &mut self.map[key as usize];
        if let Some(node) = node.leak().valid() {
            node
        } else {
            *node = f();
            node.leak()
        }
    }
    fn get(&self, key: &u8) -> Option<NodeHandle<u8, V, Self>> {
        self.map[*key as usize].leak().valid()
    }
    fn cleanup(&mut self, mut f: impl FnMut(&mut NodeHandle<u8, V, Self>) -> bool) -> usize {
        self.map
            .iter_mut()
            .filter_map(|node| (!f(node)).then_some(()))
            .count()
    }
    fn prune(
        &mut self,
        f: impl FnMut(&mut NodeHandle<u8, V, Self>) -> bool,
    ) -> Option<Option<(u8, NodeHandle<u8, V, Self>)>> {
        match self.cleanup(f) {
            0 => Some(None),
            1 => Some(Some(
                self.map
                    .iter_mut()
                    .enumerate()
                    .find_map(|(k, node)| {
                        node.is_valid()
                            .then(|| (k as u8, replace(node, Handle::new_null())))
                    })
                    .unwrap(),
            )),
            2..0x100 => None,
            0x100.. => unreachable!(),
        }
    }
    fn iter<'a>(&'a self) -> impl Iterator<Item = (&'a u8, NodeHandle<u8, V, Self>)>
    where
        u8: 'a,
    {
        const KEYS: [u8; 0x100] = {
            let mut key = 0;
            let mut keys = [0; 0x100];
            loop {
                keys[key as usize] = key;
                if key == 0xff {
                    break;
                }
                key += 1;
            }
            keys
        };
        self.map
            .iter()
            .enumerate()
            .filter_map(|(k, node)| node.leak().valid().map(|node| (&KEYS[k], node)))
    }
}
