//! LWW-Map (Last-Writer-Wins Map) CRDT implementation.
//!
//! An LWW-Map is a map where each key has an associated LWW-Register.
//! This allows per-key conflict resolution using last-writer-wins semantics.
//!
//! # Properties
//!
//! - **Commutative**: Order of operations doesn't matter
//! - **Associative**: Merging order doesn't matter
//! - **Idempotent**: Merging the same state multiple times has no effect
//! - Per-key LWW semantics
//!
//! # Example
//!
//! ```rust
//! use mini_crdt::{LWWMap, Merge, CrdtValue};
//!
//! let mut map = LWWMap::new();
//! map.insert("node1", "key1", "value1", 100).unwrap();
//! map.insert("node1", "key2", "value2", 100).unwrap();
//! assert_eq!(map.get(&"key1"), Some(&"value1"));
//!
//! // Later write to same key wins
//! map.insert("node2", "key1", "new_value", 200).unwrap();
//! assert_eq!(map.get(&"key1"), Some(&"new_value"));
//!
//! // Merge with another map
//! let mut other = LWWMap::new();
//! other.insert("node3", "key3", "value3", 150).unwrap();
//! map.merge(&other);
//! assert_eq!(map.len(), 3);
//! ```

use core::fmt;
use core::hash::Hash;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{HashMap, Merge, CrdtValue, NodeAware, LWWRegister, LWWRegisterError, Timestamp, Vec};

/// Error type for LWWMap operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum LWWMapError {
    /// The node ID is invalid.
    InvalidNodeId,
    /// The key is invalid.
    InvalidKey,
    /// The value is invalid.
    InvalidValue,
    /// The key was not found.
    KeyNotFound,
}

impl fmt::Display for LWWMapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LWWMapError::InvalidNodeId => write!(f, "invalid node ID"),
            LWWMapError::InvalidKey => write!(f, "invalid key"),
            LWWMapError::InvalidValue => write!(f, "invalid value"),
            LWWMapError::KeyNotFound => write!(f, "key not found"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for LWWMapError {}

/// A Last-Writer-Wins Map (LWW-Map) CRDT.
///
/// Each key maps to an LWW-Register, allowing per-key conflict resolution.
///
/// # Type Parameters
///
/// - `K`: The key type. Must implement `Clone + Eq + Hash`.
/// - `V`: The value type. Must implement `Clone + PartialEq`.
/// - `N`: The node identifier type. Must implement `Clone + Eq + Hash + Ord`.
///
/// # Invariants
///
/// - Each key has its own LWW-Register
/// - Merge is performed per-key
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct LWWMap<K, V, N>
where
    K: Clone + Eq + Hash,
    V: Clone + PartialEq,
    N: Clone + Eq + Hash + Ord,
{
    registers: HashMap<K, LWWRegister<V, N>>,
}

impl<K: Clone + Eq + Hash, V: Clone + PartialEq, N: Clone + Eq + Hash + Ord> PartialEq for LWWMap<K, V, N> {
    fn eq(&self, other: &Self) -> bool {
        self.registers == other.registers
    }
}

impl<K: Clone + Eq + Hash, V: Clone + PartialEq, N: Clone + Eq + Hash + Ord> Eq for LWWMap<K, V, N> {}

impl<K: Clone + Eq + Hash, V: Clone + PartialEq, N: Clone + Eq + Hash + Ord> LWWMap<K, V, N> {
    /// Creates a new empty LWW-Map.
    pub fn new() -> Self {
        Self {
            registers: HashMap::new(),
        }
    }

    /// Creates a new LWW-Map with pre-existing registers.
    pub fn with_registers(registers: HashMap<K, LWWRegister<V, N>>) -> Self {
        Self { registers }
    }

    /// Inserts or updates a key-value pair with a timestamp.
    ///
    /// Returns `true` if the value was updated, `false` if the existing
    /// value had a higher timestamp.
    pub fn insert(&mut self, node: N, key: K, value: V, time: u64) -> Result<bool, LWWMapError> {
        self.validate_node(&node)?;
        self.validate_key(&key)?;
        self.validate_value(&value)?;

        let register = self.registers.entry(key).or_insert_with(|| LWWRegister::new());
        register.set(node, value, time).map_err(|e| match e {
            LWWRegisterError::InvalidNodeId => LWWMapError::InvalidNodeId,
            LWWRegisterError::InvalidValue => LWWMapError::InvalidValue,
        })
    }

    /// Inserts or updates a key-value pair with a Timestamp.
    pub fn insert_with_timestamp(&mut self, key: K, value: V, timestamp: Timestamp<N>) -> Result<bool, LWWMapError> {
        self.validate_key(&key)?;
        self.validate_value(&value)?;

        let register = self.registers.entry(key).or_insert_with(|| LWWRegister::new());
        register.set_with_timestamp(value, timestamp).map_err(|e| match e {
            LWWRegisterError::InvalidNodeId => LWWMapError::InvalidNodeId,
            LWWRegisterError::InvalidValue => LWWMapError::InvalidValue,
        })
    }

    /// Gets the value for a key.
    pub fn get(&self, key: &K) -> Option<&V> {
        self.registers.get(key)?.value()
    }

    /// Gets the value and timestamp for a key.
    pub fn get_with_timestamp(&self, key: &K) -> Option<(&V, &Timestamp<N>)> {
        let register = self.registers.get(key)?;
        let value = register.value()?;
        let timestamp = register.timestamp()?;
        Some((value, timestamp))
    }

    /// Removes a key from the map.
    ///
    /// Note: This creates a "tombstone" by setting the key to an empty register.
    /// In a true CRDT, removal is typically done by setting a special tombstone value
    /// with a high timestamp. This method simply removes the key locally.
    /// For distributed removal, use `insert` with a special tombstone value.
    pub fn remove(&mut self, key: &K) -> Result<(), LWWMapError> {
        self.validate_key(key)?;
        self.registers.remove(key);
        Ok(())
    }

    /// Checks if a key exists.
    pub fn contains_key(&self, key: &K) -> bool {
        self.registers.contains_key(key)
    }

    /// Returns the number of keys in the map.
    pub fn len(&self) -> usize {
        self.registers.len()
    }

    /// Checks if the map is empty.
    pub fn is_empty(&self) -> bool {
        self.registers.is_empty()
    }

    /// Returns an iterator over all key-value pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.registers.iter().filter_map(|(k, r)| r.value().map(|v| (k, v)))
    }

    /// Returns an iterator over all keys.
    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.registers.keys()
    }

    /// Returns an iterator over all values.
    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.registers.values().filter_map(|r| r.value())
    }

    /// Returns a vector of all key-value pairs (cloned).
    pub fn to_vec(&self) -> Vec<(K, V)> {
        self.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
    }

    /// Gets the underlying register for a key (for advanced use cases).
    pub fn register(&self, key: &K) -> Option<&LWWRegister<V, N>> {
        self.registers.get(key)
    }

    /// Gets a mutable reference to the underlying register for a key.
    pub fn register_mut(&mut self, key: &K) -> Option<&mut LWWRegister<V, N>> {
        self.registers.get_mut(key)
    }

    /// Validates a node ID. Override to add custom validation.
    fn validate_node(&self, _node: &N) -> Result<(), LWWMapError> {
        Ok(())
    }

    /// Validates a key. Override to add custom validation.
    fn validate_key(&self, _key: &K) -> Result<(), LWWMapError> {
        Ok(())
    }

    /// Validates a value. Override to add custom validation.
    fn validate_value(&self, _value: &V) -> Result<(), LWWMapError> {
        Ok(())
    }
}

impl<K: Clone + Eq + Hash, V: Clone + PartialEq, N: Clone + Eq + Hash + Ord> Default for LWWMap<K, V, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Clone + Eq + Hash, V: Clone + PartialEq, N: Clone + Eq + Hash + Ord> CrdtValue for LWWMap<K, V, N> {
    type Value = Vec<(K, V)>;

    /// Returns all key-value pairs as a vector.
    fn value(&self) -> Vec<(K, V)> {
        self.to_vec()
    }
}

impl<K: Clone + Eq + Hash, V: Clone + PartialEq, N: Clone + Eq + Hash + Ord> NodeAware for LWWMap<K, V, N> {
    type NodeId = N;

    fn nodes(&self) -> impl Iterator<Item = &N> {
        let mut seen = Vec::new();
        for register in self.registers.values() {
            if let Some(node) = register.writer() {
                if !seen.contains(&node) {
                    seen.push(node);
                }
            }
        }
        seen.into_iter()
    }
}

impl<K: Clone + Eq + Hash, V: Clone + PartialEq, N: Clone + Eq + Hash + Ord> Merge for LWWMap<K, V, N> {
    /// Merges another LWW-Map into this one.
    ///
    /// Performs per-key merge of the underlying LWW-Registers.
    fn merge(&mut self, other: &Self) {
        for (key, other_register) in &other.registers {
            let register = self.registers.entry(key.clone()).or_insert_with(|| LWWRegister::new());
            register.merge(other_register);
        }
    }
}

impl<K: Clone + Eq + Hash + fmt::Debug, V: Clone + PartialEq + fmt::Debug, N: Clone + Eq + Hash + Ord + fmt::Debug> fmt::Display for LWWMap<K, V, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LWWMap {{ len: {} }}", self.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lww_map_basic() {
        let mut map = LWWMap::new();
        assert_eq!(map.len(), 0);
        assert!(map.is_empty());

        map.insert("node1", "key1", "value1", 100).unwrap();
        assert_eq!(map.len(), 1);
        assert_eq!(map.get(&"key1"), Some(&"value1"));
    }

    #[test]
    fn test_lww_map_multiple_keys() {
        let mut map = LWWMap::new();
        map.insert("node1", "key1", "value1", 100).unwrap();
        map.insert("node1", "key2", "value2", 100).unwrap();
        map.insert("node1", "key3", "value3", 100).unwrap();

        assert_eq!(map.len(), 3);
        assert_eq!(map.get(&"key1"), Some(&"value1"));
        assert_eq!(map.get(&"key2"), Some(&"value2"));
        assert_eq!(map.get(&"key3"), Some(&"value3"));
    }

    #[test]
    fn test_lww_map_update_same_key() {
        let mut map = LWWMap::new();
        map.insert("node1", "key1", "value1", 100).unwrap();
        map.insert("node2", "key1", "value2", 200).unwrap(); // Later write wins

        assert_eq!(map.get(&"key1"), Some(&"value2"));
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn test_lww_map_older_write_ignored() {
        let mut map = LWWMap::new();
        map.insert("node1", "key1", "value1", 200).unwrap();
        map.insert("node2", "key1", "value2", 100).unwrap(); // Older, ignored

        assert_eq!(map.get(&"key1"), Some(&"value1"));
    }

    #[test]
    fn test_lww_map_tiebreaker() {
        let mut map = LWWMap::new();
        map.insert("node_a", "key1", "value_a", 100).unwrap();
        map.insert("node_b", "key1", "value_b", 100).unwrap(); // Same time, node_b > node_a

        assert_eq!(map.get(&"key1"), Some(&"value_b"));
    }

    #[test]
    fn test_lww_map_get_with_timestamp() {
        let mut map = LWWMap::new();
        map.insert("node1", "key1", "value1", 100).unwrap();

        let result = map.get_with_timestamp(&"key1");
        assert!(result.is_some());
        let (value, ts) = result.unwrap();
        assert_eq!(value, &"value1");
        assert_eq!(ts.time, 100);
        assert_eq!(ts.node, "node1");
    }

    #[test]
    fn test_lww_map_remove() {
        let mut map = LWWMap::new();
        map.insert("node1", "key1", "value1", 100).unwrap();
        assert_eq!(map.len(), 1);

        map.remove(&"key1").unwrap();
        assert_eq!(map.len(), 0);
        assert_eq!(map.get(&"key1"), None);
    }

    #[test]
    fn test_lww_map_remove_nonexistent() {
        let mut map: LWWMap<&str, &str, &str> = LWWMap::new();
        let result = map.remove(&"key1");
        // Note: Current implementation doesn't error on missing key
        // This is intentional for idempotency
        assert!(result.is_ok());
    }

    #[test]
    fn test_lww_map_merge() {
        let mut map1 = LWWMap::new();
        map1.insert("node1", "key1", "value1", 100).unwrap();
        map1.insert("node1", "key2", "value2", 100).unwrap();

        let mut map2 = LWWMap::new();
        map2.insert("node2", "key2", "new_value2", 200).unwrap(); // Wins for key2
        map2.insert("node2", "key3", "value3", 100).unwrap();

        map1.merge(&map2);

        assert_eq!(map1.get(&"key1"), Some(&"value1")); // Only in map1
        assert_eq!(map1.get(&"key2"), Some(&"new_value2")); // map2 wins
        assert_eq!(map1.get(&"key3"), Some(&"value3")); // Only in map2
        assert_eq!(map1.len(), 3);
    }

    #[test]
    fn test_lww_map_merge_idempotent() {
        let mut map1 = LWWMap::new();
        map1.insert("node1", "key1", "value1", 100).unwrap();

        let map2 = map1.clone();
        map1.merge(&map2);

        assert_eq!(map1.len(), 1);
        assert_eq!(map1.get(&"key1"), Some(&"value1"));
    }

    #[test]
    fn test_lww_map_merge_commutative() {
        let mut map1 = LWWMap::new();
        map1.insert("node1", "key1", "value1", 100).unwrap();

        let mut map2 = LWWMap::new();
        map2.insert("node2", "key2", "value2", 200).unwrap();

        let mut merged1 = map1.clone();
        merged1.merge(&map2);

        let mut merged2 = map2.clone();
        merged2.merge(&map1);

        assert_eq!(merged1.len(), merged2.len());
        assert_eq!(merged1.get(&"key1"), merged2.get(&"key1"));
        assert_eq!(merged1.get(&"key2"), merged2.get(&"key2"));
    }

    #[test]
    fn test_lww_map_merged() {
        let mut map1 = LWWMap::new();
        map1.insert("node1", "key1", "value1", 100).unwrap();

        let mut map2 = LWWMap::new();
        map2.insert("node2", "key2", "value2", 200).unwrap();

        let merged = map1.merged(&map2);
        assert_eq!(merged.len(), 2);
        // Originals unchanged
        assert_eq!(map1.len(), 1);
        assert_eq!(map2.len(), 1);
    }

    #[test]
    fn test_lww_map_iterator() {
        let mut map = LWWMap::new();
        map.insert("node1", "key1", "value1", 100).unwrap();
        map.insert("node1", "key2", "value2", 100).unwrap();

        let pairs: Vec<_> = map.iter().collect();
        assert_eq!(pairs.len(), 2);

        let keys: Vec<_> = map.keys().collect();
        assert_eq!(keys.len(), 2);

        let values: Vec<_> = map.values().collect();
        assert_eq!(values.len(), 2);
    }

    #[test]
    fn test_lww_map_nodes() {
        let mut map = LWWMap::new();
        map.insert("node1", "key1", "value1", 100).unwrap();
        map.insert("node2", "key2", "value2", 200).unwrap();
        map.insert("node1", "key3", "value3", 100).unwrap();

        let nodes: Vec<_> = map.nodes().collect();
        assert_eq!(nodes.len(), 2);
        assert!(nodes.contains(&&"node1"));
        assert!(nodes.contains(&&"node2"));
    }

    #[test]
    fn test_lww_map_register_access() {
        let mut map = LWWMap::new();
        map.insert("node1", "key1", "value1", 100).unwrap();

        let register = map.register(&"key1");
        assert!(register.is_some());
        assert_eq!(register.unwrap().value(), Some(&"value1"));

        let register = map.register(&"nonexistent");
        assert!(register.is_none());
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_lww_map_serde() {
        let mut map = LWWMap::new();
        map.insert("node1", "key1", "value1", 100).unwrap();
        map.insert("node2", "key2", "value2", 200).unwrap();

        let json = serde_json::to_string(&map).unwrap();
        let deserialized: LWWMap<&str, &str, &str> = serde_json::from_str(&json).unwrap();

        assert_eq!(map.len(), deserialized.len());
        assert_eq!(map.get(&"key1"), deserialized.get(&"key1"));
        assert_eq!(map.get(&"key2"), deserialized.get(&"key2"));
    }
}