//! LWW-Register (Last-Writer-Wins Register) CRDT implementation.
//!
//! An LWW-Register stores a single value with a timestamp.
//! The value with the highest timestamp wins during merge.
//! Ties are broken by node ID for deterministic behavior.
//!
//! # Properties
//!
//! - **Commutative**: Order of writes doesn't matter
//! - **Associative**: Merging order doesn't matter
//! - **Idempotent**: Merging the same state multiple times has no effect
//!
//! # Example
//!
//! ```rust
//! use mini_crdt::{LWWRegister, Merge, CrdtValue};
//!
//! let mut reg = LWWRegister::new();
//! reg.set("node1", "value1", 100).unwrap();
//! assert_eq!(reg.value(), Some(&"value1"));
//!
//! // Later write with higher timestamp wins
//! reg.set("node2", "value2", 200).unwrap();
//! assert_eq!(reg.value(), Some(&"value2"));
//!
//! // Merge with another register
//! let mut other = LWWRegister::new();
//! other.set("node3", "value3", 150).unwrap();
//! reg.merge(&other);
//! assert_eq!(reg.value(), Some(&"value2")); // 200 > 150
//! ```

use core::fmt;
use core::hash::Hash;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{Merge, CrdtValue, NodeAware, Timestamp};

/// Error type for LWWRegister operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum LWWRegisterError {
    /// The node ID is invalid.
    InvalidNodeId,
    /// The value is invalid.
    InvalidValue,
}

impl fmt::Display for LWWRegisterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LWWRegisterError::InvalidNodeId => write!(f, "invalid node ID"),
            LWWRegisterError::InvalidValue => write!(f, "invalid value"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for LWWRegisterError {}

/// A Last-Writer-Wins Register (LWW-Register) CRDT.
///
/// Stores a single value with an associated timestamp.
/// The value with the highest timestamp wins.
/// Ties are broken by comparing node IDs.
///
/// # Type Parameters
///
/// - `V`: The value type. Must implement `Clone + PartialEq`.
/// - `N`: The node identifier type. Must implement `Clone + Eq + Hash + Ord`.
///
/// # Invariants
///
/// - The stored timestamp is always the maximum seen
/// - Merge takes the value with the higher timestamp (or higher node ID on tie)
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct LWWRegister<V, N>
where
    V: Clone + PartialEq,
    N: Clone + Eq + Hash + Ord,
{
    /// The current value.
    value: Option<V>,
    /// The timestamp of the current value.
    timestamp: Option<Timestamp<N>>,
}

impl<V: Clone + PartialEq, N: Clone + Eq + Hash + Ord> PartialEq for LWWRegister<V, N> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value && self.timestamp == other.timestamp
    }
}

impl<V: Clone + PartialEq, N: Clone + Eq + Hash + Ord> Eq for LWWRegister<V, N> {}

impl<V: Clone + PartialEq, N: Clone + Eq + Hash + Ord> LWWRegister<V, N> {
    /// Creates a new empty LWW-Register.
    pub fn new() -> Self {
        Self {
            value: None,
            timestamp: None,
        }
    }

    /// Creates a new LWW-Register with an initial value and timestamp.
    pub fn with_value(value: V, timestamp: Timestamp<N>) -> Self {
        Self {
            value: Some(value),
            timestamp: Some(timestamp),
        }
    }

    /// Sets the value with a given timestamp.
    ///
    /// If the timestamp is greater than the current timestamp (or equal with a higher node ID),
    /// the value is updated. Otherwise, the operation has no effect.
    ///
    /// Returns `true` if the value was updated, `false` otherwise.
    pub fn set(&mut self, node: N, value: V, time: u64) -> Result<bool, LWWRegisterError> {
        self.validate_node(&node)?;
        self.validate_value(&value)?;

        let new_timestamp = Timestamp::new(time, node);
        let should_update = match &self.timestamp {
            None => true,
            Some(current) => new_timestamp > *current,
        };

        if should_update {
            self.value = Some(value);
            self.timestamp = Some(new_timestamp);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Sets the value using a Timestamp directly.
    pub fn set_with_timestamp(&mut self, value: V, timestamp: Timestamp<N>) -> Result<bool, LWWRegisterError> {
        self.validate_value(&value)?;

        let should_update = match &self.timestamp {
            None => true,
            Some(current) => timestamp > *current,
        };

        if should_update {
            self.value = Some(value);
            self.timestamp = Some(timestamp);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Returns the current value, if any.
    pub fn value(&self) -> Option<&V> {
        self.value.as_ref()
    }

    /// Returns the current timestamp, if any.
    pub fn timestamp(&self) -> Option<&Timestamp<N>> {
        self.timestamp.as_ref()
    }

    /// Returns the node that wrote the current value.
    pub fn writer(&self) -> Option<&N> {
        self.timestamp.as_ref().map(|t| &t.node)
    }

    /// Returns the time of the current value.
    pub fn time(&self) -> Option<u64> {
        self.timestamp.as_ref().map(|t| t.time)
    }

    /// Checks if the register is empty (no value set yet).
    pub fn is_empty(&self) -> bool {
        self.value.is_none()
    }

    /// Clears the register. Note: This violates CRDT semantics and should only be used
    /// for testing or local state reset.
    pub fn clear(&mut self) {
        self.value = None;
        self.timestamp = None;
    }

    /// Validates a node ID. Override to add custom validation.
    fn validate_node(&self, _node: &N) -> Result<(), LWWRegisterError> {
        Ok(())
    }

    /// Validates a value. Override to add custom validation.
    fn validate_value(&self, _value: &V) -> Result<(), LWWRegisterError> {
        Ok(())
    }
}

impl<V: Clone + PartialEq, N: Clone + Eq + Hash + Ord> Default for LWWRegister<V, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<V: Clone + PartialEq, N: Clone + Eq + Hash + Ord> CrdtValue for LWWRegister<V, N> {
    type Value = Option<V>;

    /// Returns the current value (cloned).
    fn value(&self) -> Option<V> {
        self.value.clone()
    }
}

impl<V: Clone + PartialEq, N: Clone + Eq + Hash + Ord> NodeAware for LWWRegister<V, N> {
    type NodeId = N;

    fn nodes(&self) -> impl Iterator<Item = &N> {
        self.timestamp.as_ref().map(|t| &t.node).into_iter()
    }
}

impl<V: Clone + PartialEq, N: Clone + Eq + Hash + Ord> Merge for LWWRegister<V, N> {
    /// Merges another LWW-Register into this one.
    ///
    /// Takes the value with the higher timestamp (or higher node ID on tie).
    fn merge(&mut self, other: &Self) {
        match (&self.timestamp, &other.timestamp) {
            (None, None) => {}
            (None, Some(_)) => {
                self.value = other.value.clone();
                self.timestamp = other.timestamp.clone();
            }
            (Some(_), None) => {
                // Keep current value
            }
            (Some(current), Some(other_ts)) => {
                if *other_ts > *current {
                    self.value = other.value.clone();
                    self.timestamp = other.timestamp.clone();
                }
            }
        }
    }
}

impl<V: Clone + PartialEq + fmt::Debug, N: Clone + Eq + Hash + Ord + fmt::Debug> fmt::Display for LWWRegister<V, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (&self.value, &self.timestamp) {
            (Some(v), Some(ts)) => write!(f, "LWWRegister {{ value: {:?}, time: {}, node: {:?} }}", v, ts.time, ts.node),
            _ => write!(f, "LWWRegister {{ empty }}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lww_register_basic() {
        let mut reg = LWWRegister::new();
        assert!(reg.is_empty());
        assert_eq!(reg.value(), None);

        reg.set("node1", "value1", 100).unwrap();
        assert_eq!(reg.value(), Some(&"value1"));
        assert_eq!(reg.time(), Some(100));
        assert_eq!(reg.writer(), Some(&"node1"));
    }

    #[test]
    fn test_lww_register_later_write_wins() {
        let mut reg = LWWRegister::new();
        reg.set("node1", "value1", 100).unwrap();
        reg.set("node2", "value2", 200).unwrap();

        assert_eq!(reg.value(), Some(&"value2"));
        assert_eq!(reg.time(), Some(200));
        assert_eq!(reg.writer(), Some(&"node2"));
    }

    #[test]
    fn test_lww_register_older_write_ignored() {
        let mut reg = LWWRegister::new();
        reg.set("node1", "value1", 200).unwrap();
        reg.set("node2", "value2", 100).unwrap(); // Older, should be ignored

        assert_eq!(reg.value(), Some(&"value1"));
        assert_eq!(reg.time(), Some(200));
    }

    #[test]
    fn test_lww_register_tiebreaker() {
        let mut reg = LWWRegister::new();
        // Same timestamp, different nodes - "b" > "a" lexicographically
        reg.set("node_a", "value_a", 100).unwrap();
        reg.set("node_b", "value_b", 100).unwrap(); // Same time, but node_b > node_a

        assert_eq!(reg.value(), Some(&"value_b"));
        assert_eq!(reg.writer(), Some(&"node_b"));
    }

    #[test]
    fn test_lww_register_set_with_timestamp() {
        let mut reg = LWWRegister::new();
        let ts = Timestamp::new(150, "node1");
        reg.set_with_timestamp("value1", ts).unwrap();

        assert_eq!(reg.value(), Some(&"value1"));
        assert_eq!(reg.time(), Some(150));
    }

    #[test]
    fn test_lww_register_merge() {
        let mut reg1 = LWWRegister::new();
        reg1.set("node1", "value1", 100).unwrap();

        let mut reg2 = LWWRegister::new();
        reg2.set("node2", "value2", 200).unwrap();

        reg1.merge(&reg2);
        assert_eq!(reg1.value(), Some(&"value2"));
        assert_eq!(reg1.time(), Some(200));
    }

    #[test]
    fn test_lww_register_merge_older() {
        let mut reg1 = LWWRegister::new();
        reg1.set("node1", "value1", 200).unwrap();

        let mut reg2 = LWWRegister::new();
        reg2.set("node2", "value2", 100).unwrap();

        reg1.merge(&reg2);
        assert_eq!(reg1.value(), Some(&"value1"));
        assert_eq!(reg1.time(), Some(200));
    }

    #[test]
    fn test_lww_register_merge_tiebreaker() {
        let mut reg1 = LWWRegister::new();
        reg1.set("node_a", "value_a", 100).unwrap();

        let mut reg2 = LWWRegister::new();
        reg2.set("node_b", "value_b", 100).unwrap();

        reg1.merge(&reg2);
        assert_eq!(reg1.value(), Some(&"value_b")); // node_b > node_a
    }

    #[test]
    fn test_lww_register_merge_empty() {
        let mut reg1 = LWWRegister::new();
        reg1.set("node1", "value1", 100).unwrap();

        let reg2 = LWWRegister::new(); // Empty

        reg1.merge(&reg2);
        assert_eq!(reg1.value(), Some(&"value1"));

        // Merge empty into empty
        let mut reg3 = LWWRegister::new();
        reg3.merge(&reg2);
        assert!(reg3.is_empty());
    }

    #[test]
    fn test_lww_register_merge_idempotent() {
        let mut reg1 = LWWRegister::new();
        reg1.set("node1", "value1", 100).unwrap();

        let reg2 = reg1.clone();
        reg1.merge(&reg2);

        assert_eq!(reg1.value(), Some(&"value1"));
        assert_eq!(reg1.time(), Some(100));
    }

    #[test]
    fn test_lww_register_merge_commutative() {
        let mut reg1 = LWWRegister::new();
        reg1.set("node1", "value1", 100).unwrap();

        let mut reg2 = LWWRegister::new();
        reg2.set("node2", "value2", 200).unwrap();

        let mut merged1 = reg1.clone();
        merged1.merge(&reg2);

        let mut merged2 = reg2.clone();
        merged2.merge(&reg1);

        assert_eq!(merged1.value(), merged2.value());
        assert_eq!(merged1.time(), merged2.time());
        assert_eq!(merged1.writer(), merged2.writer());
    }

    #[test]
    fn test_lww_register_merged() {
        let mut reg1 = LWWRegister::new();
        reg1.set("node1", "value1", 100).unwrap();

        let mut reg2 = LWWRegister::new();
        reg2.set("node2", "value2", 200).unwrap();

        let merged = reg1.merged(&reg2);
        assert_eq!(merged.value(), Some(&"value2"));
        // Originals unchanged
        assert_eq!(reg1.value(), Some(&"value1"));
        assert_eq!(reg2.value(), Some(&"value2"));
    }

    #[test]
    fn test_lww_register_clear() {
        let mut reg = LWWRegister::new();
        reg.set("node1", "value1", 100).unwrap();
        assert!(!reg.is_empty());

        reg.clear();
        assert!(reg.is_empty());
        assert_eq!(reg.value(), None);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_lww_register_serde() {
        let mut reg = LWWRegister::new();
        reg.set("node1", "value1", 100).unwrap();

        let json = serde_json::to_string(&reg).unwrap();
        let deserialized: LWWRegister<&str, &str> = serde_json::from_str(&json).unwrap();

        assert_eq!(reg.value(), deserialized.value());
        assert_eq!(reg.time(), deserialized.time());
        assert_eq!(reg.writer(), deserialized.writer());
    }
}