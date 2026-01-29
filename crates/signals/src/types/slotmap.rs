use std::ops::{Deref, DerefMut, Index, IndexMut};

use slotmap::{Key, SlotMap};

pub struct UnsafeSlotMap<K: Key, V>(SlotMap<K, V>);

impl<K: Key, V> Default for UnsafeSlotMap<K, V> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<K: Key, V> Index<K> for UnsafeSlotMap<K, V> {
    type Output = V;

    #[inline]
    fn index(&self, key: K) -> &V {
        unsafe { self.0.get_unchecked(key) }
    }
}

impl<K: Key, V> IndexMut<K> for UnsafeSlotMap<K, V> {
    #[inline]
    fn index_mut(&mut self, key: K) -> &mut V {
        unsafe { self.0.get_unchecked_mut(key) }
    }
}

impl<K: Key, V> Deref for UnsafeSlotMap<K, V> {
    type Target = SlotMap<K, V>;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<K: Key, V> DerefMut for UnsafeSlotMap<K, V> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
