use std::{
    hash::Hash,
};

use indexmap::{IndexMap, Equivalent};
use serde::{Deserialize, Serialize};

/// A wrapper around [`IndexMap`] with custom implementation of [`PartialEq`], [`Eq`],
/// [`PartialOrd`], [`Ord`], and [`Hash`].
/// 
/// Only a selected list of methods are re-exported for convenience.
#[derive(Debug, Default)]
pub struct OrderedMap<K, V>(IndexMap<K, V>);

impl<K, V> From<IndexMap<K, V>> for OrderedMap<K, V> {
    fn from(map: IndexMap<K, V>) -> Self {
        Self(map)
    }
}

impl<K, V> OrderedMap<K, V> {
    /// Get a reference to the inner [`IndexMap`]
    /// 
    /// It is intentional to NOT implement the `AsRef<IndexMap>` trait to avoid potential 
    /// misuse
    pub fn as_inner(&self) -> &IndexMap<K, V> {
        &self.0
    }

    /// Get a mutable reference to the inner [`IndexMap`]
    /// 
    /// It is intentional to NOT implement the `AsMut<IndexMap>` trait to avoid potential 
    /// misuse
    pub fn as_inner_mut(&mut self) -> &mut IndexMap<K, V> {
        &mut self.0
    }

    /// Consumes the wrapper and returns the inner [`IndexMap`]
    pub fn into_inner(self) -> IndexMap<K, V> {
        self.0
    }
}

impl<K, V> OrderedMap<K, V> 
where
    K: Hash + Eq,
{
    /// Calls [`IndexMap::insert`] internally
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        self.0.insert(key, value)
    }

    /// Calls [`IndexMap::get`] internally
    pub fn get<Q: ?Sized>(&self, key: &Q) -> Option<&V> 
    where
        Q: Hash + Equivalent<K>
    {
        self.0.get(key)
    }

    /// Calls [`IndexMap::get_mut`] internally
    pub fn get_mut<Q: ?Sized>(&mut self, key: &Q) -> Option<&mut V> 
    where
        Q: Hash + Equivalent<K>
    {
        self.0.get_mut(key)
    }

    /// Calls [`IndexMap::remove`] internally
    pub fn remove<Q: ?Sized>(&mut self, key: &Q) -> Option<V> 
    where
        Q: Hash + Equivalent<K>
    {
        self.0.remove(key)
    }
}

impl<K, V> Serialize for OrderedMap<K, V>
where
    K: Serialize + Eq + Hash,
    V: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        indexmap::serde_seq::serialize(&self.0, serializer)
    }
}

impl<'de, K, V> Deserialize<'de> for OrderedMap<K, V>
where
    K: Deserialize<'de> + Eq + Hash,
    V: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let map: IndexMap<K, V> = indexmap::serde_seq::deserialize(deserializer)?;
        Ok(Self(map))
    }
}

impl<K, V> PartialEq for OrderedMap<K, V>
where
    K: PartialEq,
    V: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.0.len() == other.0.len() && self.0.iter().zip(&other.0).all(|(a, b)| a == b)
    }
}

impl<K, V> Eq for OrderedMap<K, V>
where
    K: Eq,
    V: Eq,
{}

impl<K, V> PartialOrd for OrderedMap<K, V> 
where
    K: PartialOrd,
    V: PartialOrd,
{
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.0.iter().partial_cmp(other.0.iter())
    }
}

impl<K, V> Ord for OrderedMap<K, V> 
where
    K: Ord,
    V: Ord,
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.iter().cmp(other.0.iter())
    }
}

impl<K, V> Hash for OrderedMap<K, V> 
where
    K: Hash,
    V: Hash,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_usize(self.0.len());
        for entry in &self.0 {
            entry.hash(state)
        }
    }
}
