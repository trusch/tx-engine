use serde::Serialize;
use std::hash::Hash;

use crate::error::{Error, Result};

pub trait KVStore {
    type Key;
    type Value;

    fn get(&self, key: Self::Key) -> Result<&Self::Value>;
    fn set(&mut self, key: Self::Key, value: Self::Value) -> Result<()>;
}

#[derive(Debug, Clone, Default)]
pub struct InMemoryKVStore<K, T: Serialize> {
    store: std::collections::HashMap<K, T>,
}

impl<K: Eq + Hash, T: Serialize> InMemoryKVStore<K, T> {
    pub fn new() -> Result<Self> {
        Ok(Self {
            store: std::collections::HashMap::new(),
        })
    }
}

impl<K: Eq + Hash, T: Serialize> KVStore for InMemoryKVStore<K, T> {
    type Key = K;
    type Value = T;

    fn get(&self, key: Self::Key) -> Result<&Self::Value> {
        self.store.get(&key).ok_or(Error::NotFound)
    }

    fn set(&mut self, key: Self::Key, value: Self::Value) -> Result<()> {
        self.store.insert(key, value);
        Ok(())
    }
}

impl<K, T: Serialize> IntoIterator for InMemoryKVStore<K, T> {
    type Item = (K, T);
    type IntoIter = std::collections::hash_map::IntoIter<K, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.store.into_iter()
    }
}
