//! GCounter (Grow-only Counter) CRDT implementation.
//!
//! A GCounter is a counter that only supports increment operations.
//! It tracks increments per node and the total value is the sum of all node counts.
//!
//! # Properties
//!
//! - **Monotonic**: Values only increase
//! - **Commutative**: Order of increments doesn't matter
//! - **Associative**: Merging order doesn't matter
//! - **Idempotent**: Merging the same state multiple times has no effect
//!
//! # Example
//!
//! ```rust
//! use mini_crdt::{GCounter, Merge, CrdtValue};
//!
//! let mut counter = GCounter::new();
//! counter.increment("node1").unwrap();
//! counter.increment("node1").unwrap();
//! counter.increment("node2").unwrap();
//! assert_eq!(counter.value(), 3);
//!
//! // Merge with another counter
//! let mut other = GCounter::new();
//! other.increment("node1").unwrap();
//! other.increment("node3").unwrap();
//! counter.merge(&other);
//! assert_eq!(counter.value(), 4); // node1: max(2,1)=2, node2: 1, node3: 1
//! ```

use core::fmt;
use core::hash::Hash;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{HashMap, Merge, CrdtValue, NodeAware, Vec};

/// Error type for GCounter operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum GCounterError {
    /// The node ID is invalid (e.g., empty string when using string nodes).
    InvalidNodeId,
}

impl fmt::Display for GCounterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GCounterError::InvalidNodeId => write!(f, "invalid node ID"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for GCounterError {}

/// A Grow-only Counter (GCounter) CRDT.
///
/// Tracks increments per node. The value is the sum of all node counts.
/// Only supports increment operations - no decrements.
///
/// # Type Parameters
///
/// - `N`: The node identifier type. Must implement `Clone + Eq + Hash`.
///
/// # Invariants
///
/// - All counts are non-negative
/// - Merge takes the maximum count per node
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct GCounter<N>
where
    N: Clone + Eq + Hash,
{
    counts: HashMap<N, u64>,
}

impl<N: Clone + Eq + Hash> PartialEq for GCounter<N> {
    fn eq(&self, other: &Self) -> bool {
        self.counts == other.counts
    }
}

impl<N: Clone + Eq + Hash> Eq for GCounter<N> {}

impl<N: Clone + Eq + Hash> GCounter<N> {
    /// Creates a new empty GCounter.
    pub fn new() -> Self {
        Self {
            counts: HashMap::new(),
        }
    }

    /// Creates a new GCounter with a pre-existing count for a node.
    pub fn with_count(node: N, count: u64) -> Self {
        let mut counts = HashMap::new();
        counts.insert(node, count);
        Self { counts }
    }

    /// Increments the counter for the given node.
    ///
    /// # Arguments
    ///
    /// * `node` - The node identifier performing the increment.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or `GCounterError::InvalidNodeId` if the node ID is invalid.
    pub fn increment(&mut self, node: N) -> Result<(), GCounterError> {
        self.validate_node(&node)?;
        *self.counts.entry(node).or_insert(0) += 1;
        Ok(())
    }

    /// Increments the counter for the given node by a specific amount.
    ///
    /// # Arguments
    ///
    /// * `node` - The node identifier performing the increment.
    /// * `amount` - The amount to increment by (must be > 0).
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or `GCounterError::InvalidNodeId` if the node ID is invalid.
    pub fn increment_by(&mut self, node: N, amount: u64) -> Result<(), GCounterError> {
        if amount == 0 {
            return Ok(());
        }
        self.validate_node(&node)?;
        *self.counts.entry(node).or_insert(0) += amount;
        Ok(())
    }

    /// Returns the current count for a specific node.
    pub fn count(&self, node: &N) -> u64 {
        self.counts.get(node).copied().unwrap_or(0)
    }

    /// Returns an iterator over all node counts.
    pub fn counts(&self) -> impl Iterator<Item = (&N, &u64)> {
        self.counts.iter()
    }

    /// Returns the number of nodes that have contributed to this counter.
    pub fn node_count(&self) -> usize {
        self.counts.len()
    }

    /// Checks if the counter is empty (no increments yet).
    pub fn is_empty(&self) -> bool {
        self.counts.is_empty()
    }

    /// Validates a node ID. Override this to add custom validation.
    fn validate_node(&self, _node: &N) -> Result<(), GCounterError> {
        Ok(())
    }
}

impl<N: Clone + Eq + Hash> Default for GCounter<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<N: Clone + Eq + Hash> CrdtValue for GCounter<N> {
    type Value = u64;

    /// Returns the total value of the counter (sum of all node counts).
    fn value(&self) -> u64 {
        self.counts.values().sum()
    }
}

impl<N: Clone + Eq + Hash> NodeAware for GCounter<N> {
    type NodeId = N;

    fn nodes(&self) -> impl Iterator<Item = &N> {
        self.counts.keys()
    }
}

impl<N: Clone + Eq + Hash> Merge for GCounter<N> {
    /// Merges another GCounter into this one.
    ///
    /// For each node, takes the maximum of the two counts.
    fn merge(&mut self, other: &Self) {
        for (node, count) in &other.counts {
            let entry = self.counts.entry(node.clone()).or_insert(0);
            *entry = (*entry).max(*count);
        }
    }
}

impl<N: Clone + Eq + Hash> FromIterator<(N, u64)> for GCounter<N> {
    fn from_iter<T: IntoIterator<Item = (N, u64)>>(iter: T) -> Self {
        let mut counts = HashMap::new();
        for (node, count) in iter {
            counts.insert(node, count);
        }
        Self { counts }
    }
}

impl<N: Clone + Eq + Hash + fmt::Debug> fmt::Display for GCounter<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "GCounter {{ total: {}, nodes: {} }}", self.value(), self.node_count())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(all(not(feature = "std"), feature = "alloc"))]
    use alloc::vec;

    #[test]
    fn test_gcounter_basic() {
        let mut counter = GCounter::new();
        assert_eq!(counter.value(), 0);
        assert!(counter.is_empty());

        counter.increment("node1").unwrap();
        assert_eq!(counter.value(), 1);
        assert_eq!(counter.count(&"node1"), 1);

        counter.increment("node1").unwrap();
        assert_eq!(counter.value(), 2);
        assert_eq!(counter.count(&"node1"), 2);

        counter.increment("node2").unwrap();
        assert_eq!(counter.value(), 3);
        assert_eq!(counter.count(&"node2"), 1);
    }

    #[test]
    fn test_gcounter_increment_by() {
        let mut counter = GCounter::new();
        counter.increment_by("node1", 5).unwrap();
        assert_eq!(counter.value(), 5);
        assert_eq!(counter.count(&"node1"), 5);

        counter.increment_by("node1", 3).unwrap();
        assert_eq!(counter.value(), 8);
    }

    #[test]
    fn test_gcounter_merge() {
        let mut counter1 = GCounter::new();
        counter1.increment("node1").unwrap();
        counter1.increment("node1").unwrap();
        counter1.increment("node2").unwrap();

        let mut counter2 = GCounter::new();
        counter2.increment("node1").unwrap();
        counter2.increment("node3").unwrap();
        counter2.increment("node3").unwrap();

        counter1.merge(&counter2);

        assert_eq!(counter1.value(), 5); // node1: max(2,1)=2, node2: 1, node3: 2
        assert_eq!(counter1.count(&"node1"), 2);
        assert_eq!(counter1.count(&"node2"), 1);
        assert_eq!(counter1.count(&"node3"), 2);
    }

    #[test]
    fn test_gcounter_merge_idempotent() {
        let mut counter1 = GCounter::new();
        counter1.increment("node1").unwrap();
        counter1.increment("node2").unwrap();

        let counter2 = counter1.clone();
        counter1.merge(&counter2);

        assert_eq!(counter1.value(), 2);
        assert_eq!(counter1.count(&"node1"), 1);
        assert_eq!(counter1.count(&"node2"), 1);
    }

    #[test]
    fn test_gcounter_merge_commutative() {
        let mut counter1 = GCounter::new();
        counter1.increment("node1").unwrap();
        counter1.increment("node2").unwrap();

        let mut counter2 = GCounter::new();
        counter2.increment("node1").unwrap();
        counter2.increment("node3").unwrap();

        let mut merged1 = counter1.clone();
        merged1.merge(&counter2);

        let mut merged2 = counter2.clone();
        merged2.merge(&counter1);

        assert_eq!(merged1.value(), merged2.value());
        assert_eq!(merged1.count(&"node1"), merged2.count(&"node1"));
        assert_eq!(merged1.count(&"node2"), merged2.count(&"node2"));
        assert_eq!(merged1.count(&"node3"), merged2.count(&"node3"));
    }

    #[test]
    fn test_gcounter_merge_associative() {
        let mut a = GCounter::new();
        a.increment("node1").unwrap();

        let mut b = GCounter::new();
        b.increment("node2").unwrap();

        let mut c = GCounter::new();
        c.increment("node3").unwrap();

        let mut ab = a.clone();
        ab.merge(&b);
        ab.merge(&c);

        let mut bc = b.clone();
        bc.merge(&c);
        let mut a_bc = a.clone();
        a_bc.merge(&bc);

        assert_eq!(ab.value(), a_bc.value());
    }

    #[test]
    fn test_gcounter_merged() {
        let mut counter1 = GCounter::new();
        counter1.increment("node1").unwrap();

        let mut counter2 = GCounter::new();
        counter2.increment("node2").unwrap();

        let merged = counter1.merged(&counter2);
        assert_eq!(merged.value(), 2);
        // Originals unchanged
        assert_eq!(counter1.value(), 1);
        assert_eq!(counter2.value(), 1);
    }

    #[test]
    fn test_gcounter_from_iter() {
        let counter: GCounter<&str> = vec![("node1", 3), ("node2", 5)].into_iter().collect();
        assert_eq!(counter.value(), 8);
        assert_eq!(counter.count(&"node1"), 3);
        assert_eq!(counter.count(&"node2"), 5);
    }

    #[test]
    fn test_gcounter_nodes() {
        let mut counter = GCounter::new();
        counter.increment("node1").unwrap();
        counter.increment("node2").unwrap();
        counter.increment("node2").unwrap();

        let nodes: Vec<_> = counter.nodes().collect();
        assert_eq!(nodes.len(), 2);
        assert!(nodes.contains(&&"node1"));
        assert!(nodes.contains(&&"node2"));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_gcounter_serde() {
        let mut counter = GCounter::new();
        counter.increment("node1").unwrap();
        counter.increment("node2").unwrap();
        counter.increment("node2").unwrap();

        let json = serde_json::to_string(&counter).unwrap();
        let deserialized: GCounter<&str> = serde_json::from_str(&json).unwrap();

        assert_eq!(counter.value(), deserialized.value());
        assert_eq!(counter.count(&"node1"), deserialized.count(&"node1"));
        assert_eq!(counter.count(&"node2"), deserialized.count(&"node2"));
    }
}