use std::{hash::Hash, marker::PhantomData};

use indexmap::{Equivalent, IndexMap};
use serde::{de, ser::SerializeMap, Deserialize, Serialize};

/// A wrapper around [`IndexMap`] with custom implementation of [`PartialEq`], [`Eq`],
/// [`PartialOrd`], [`Ord`], and [`Hash`].
///
/// Only a selected list of methods are re-exported for convenience.
#[derive(Debug, Clone, Default)]
pub struct OrderedMap<K, V>(IndexMap<K, V>);

impl<K, V> From<IndexMap<K, V>> for OrderedMap<K, V> {
    fn from(map: IndexMap<K, V>) -> Self {
        Self(map)
    }
}

impl<K, V> OrderedMap<K, V> {
    /// Creates a new [`OrderedMap`]
    pub fn new() -> Self {
        Self(IndexMap::new())
    }

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
        Q: Hash + Equivalent<K>,
    {
        self.0.get(key)
    }

    /// Calls [`IndexMap::get_mut`] internally
    pub fn get_mut<Q: ?Sized>(&mut self, key: &Q) -> Option<&mut V>
    where
        Q: Hash + Equivalent<K>,
    {
        self.0.get_mut(key)
    }

    /// Calls [`IndexMap::remove`] internally
    pub fn remove<Q: ?Sized>(&mut self, key: &Q) -> Option<V>
    where
        Q: Hash + Equivalent<K>,
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
        let len = self.0.len();
        let mut map = serializer.serialize_map(Some(len))?;
        for (key, value) in self {
            map.serialize_entry(key, value)?;
        }
        map.end()
    }
}

struct Visitor<K, V> {
    key_marker: PhantomData<K>,
    value_marker: PhantomData<V>,
}

impl<'de, K, V> de::Visitor<'de> for Visitor<K, V>
where
    K: Deserialize<'de> + Hash + Eq,
    V: Deserialize<'de>,
{
    type Value = OrderedMap<K, V>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("A sequence of map entries")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: de::MapAccess<'de>,
    {
        let mut inner = IndexMap::new();
        while let Some((key, value)) = map.next_entry()? {
            inner.insert(key, value);
        }
        Ok(OrderedMap(inner))
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
        deserializer.deserialize_map(Visitor::<K, V> {
            key_marker: PhantomData,
            value_marker: PhantomData,
        })
    }
}

impl<K, V> PartialEq for OrderedMap<K, V>
where
    K: PartialEq,
    V: PartialEq,
{
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.0.len() == other.0.len() && self.0.iter().zip(&other.0).all(|(a, b)| a == b)
    }
}

impl<K, V> Eq for OrderedMap<K, V>
where
    K: Eq,
    V: Eq,
{
}

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
    #[inline]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.iter().cmp(other.0.iter())
    }
}

impl<K, V> Hash for OrderedMap<K, V>
where
    K: Hash,
    V: Hash,
{
    #[inline]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_usize(self.0.len());
        for entry in &self.0 {
            entry.hash(state)
        }
    }
}

impl<'a, K, V> IntoIterator for &'a OrderedMap<K, V> {
    type Item = (&'a K, &'a V);

    type IntoIter = indexmap::map::Iter<'a, K, V>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<'a, K, V> IntoIterator for &'a mut OrderedMap<K, V> {
    type Item = (&'a K, &'a mut V);

    type IntoIter = indexmap::map::IterMut<'a, K, V>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter_mut()
    }
}

impl<K, V> IntoIterator for OrderedMap<K, V> {
    type Item = (K, V);

    type IntoIter = indexmap::map::IntoIter<K, V>;
    
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<K, V> FromIterator<(K, V)> for OrderedMap<K, V> 
where
    K: Hash + Eq,
{
    #[inline]
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        let index_map = IndexMap::from_iter(iter);
        Self(index_map)
    }
}
