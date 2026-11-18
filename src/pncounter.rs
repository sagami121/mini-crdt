//! PNCounter (Positive-Negative Counter) CRDT implementation.
//!
//! A PNCounter is a counter that supports both increment and decrement operations.
//! It is implemented as a pair of GCounters: one for positive increments (P)
//! and one for negative increments (N). The value is P - N.
//!
//! # Properties
//!
//! - Supports both increment and decrement
//! - **Commutative**: Order of operations doesn't matter
//! - **Associative**: Merging order doesn't matter
//! - **Idempotent**: Merging the same state multiple times has no effect
//!
//! # Example
//!
//! ```rust
//! use mini_crdt::{PNCounter, Merge, CrdtValue};
//!
//! let mut counter = PNCounter::new();
//! counter.increment("node1").unwrap();
//! counter.increment("node1").unwrap();
//! counter.decrement("node1").unwrap();
//! assert_eq!(counter.value(), 1);
//!
//! // Merge with another counter
//! let mut other = PNCounter::new();
//! other.increment("node2").unwrap();
//! other.decrement("node2").unwrap();
//! other.decrement("node2").unwrap();
//! counter.merge(&other);
//! assert_eq!(counter.value(), 0); // node1: +1, node2: -1
//! ```

use core::fmt;
use core::hash::Hash;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{HashMap, Merge, CrdtValue, NodeAware, GCounter, Vec};

/// Error type for PNCounter operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum PNCounterError {
    /// The node ID is invalid.
    InvalidNodeId,
}

impl fmt::Display for PNCounterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PNCounterError::InvalidNodeId => write!(f, "invalid node ID"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for PNCounterError {}

/// A Positive-Negative Counter (PNCounter) CRDT.
///
/// Implemented as two GCounters: one for increments (P) and one for decrements (N).
/// The value is P - N.
///
/// # Type Parameters
///
/// - `N`: The node identifier type. Must implement `Clone + Eq + Hash`.
///
/// # Invariants
///
/// - Both P and N are GCounters (only grow)
/// - Value = P.value() - N.value()
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct PNCounter<N>
where
    N: Clone + Eq + Hash,
{
    /// Positive increments (GCounter)
    pub(crate) positive: GCounter<N>,
    /// Negative increments (GCounter)
    pub(crate) negative: GCounter<N>,
}

impl<N: Clone + Eq + Hash> PartialEq for PNCounter<N> {
    fn eq(&self, other: &Self) -> bool {
        self.positive == other.positive && self.negative == other.negative
    }
}

impl<N: Clone + Eq + Hash> Eq for PNCounter<N> {}

impl<N: Clone + Eq + Hash> PNCounter<N> {
    /// Creates a new empty PNCounter.
    pub fn new() -> Self {
        Self {
            positive: GCounter::new(),
            negative: GCounter::new(),
        }
    }

    /// Creates a new PNCounter with pre-existing counts.
    pub fn with_counts(positive: GCounter<N>, negative: GCounter<N>) -> Self {
        Self { positive, negative }
    }

    /// Increments the counter for the given node.
    pub fn increment(&mut self, node: N) -> Result<(), PNCounterError> {
        self.positive.increment(node).map_err(|_| PNCounterError::InvalidNodeId)
    }

    /// Increments the counter for the given node by a specific amount.
    pub fn increment_by(&mut self, node: N, amount: u64) -> Result<(), PNCounterError> {
        self.positive.increment_by(node, amount).map_err(|_| PNCounterError::InvalidNodeId)
    }

    /// Decrements the counter for the given node.
    pub fn decrement(&mut self, node: N) -> Result<(), PNCounterError> {
        self.negative.increment(node).map_err(|_| PNCounterError::InvalidNodeId)
    }

    /// Decrements the counter for the given node by a specific amount.
    pub fn decrement_by(&mut self, node: N, amount: u64) -> Result<(), PNCounterError> {
        self.negative.increment_by(node, amount).map_err(|_| PNCounterError::InvalidNodeId)
    }

    /// Returns the positive count for a specific node.
    pub fn positive_count(&self, node: &N) -> u64 {
        self.positive.count(node)
    }

    /// Returns the negative count for a specific node.
    pub fn negative_count(&self, node: &N) -> u64 {
        self.negative.count(node)
    }

    /// Returns the net count for a specific node (positive - negative).
    pub fn node_value(&self, node: &N) -> i64 {
        self.positive.count(node) as i64 - self.negative.count(node) as i64
    }

    /// Returns an iterator over all nodes with their positive and negative counts.
    pub fn counts(&self) -> impl Iterator<Item = (&N, u64, u64)> {
        // Collect all unique nodes from both positive and negative
        let mut nodes = Vec::new();
        for node in self.positive.nodes() {
            nodes.push(node);
        }
        for node in self.negative.nodes() {
            if !self.positive.counts().any(|(n, _)| n == node) {
                nodes.push(node);
            }
        }
        nodes.into_iter().map(move |node| {
            let p = self.positive.count(node);
            let n = self.negative.count(node);
            (node, p, n)
        })
    }

    /// Returns the number of nodes that have contributed to this counter.
    pub fn node_count(&self) -> usize {
        let mut count = self.positive.node_count();
        for node in self.negative.nodes() {
            if self.positive.count(node) == 0 {
                count += 1;
            }
        }
        count
    }

    /// Checks if the counter is empty (no operations yet).
    pub fn is_empty(&self) -> bool {
        self.positive.is_empty() && self.negative.is_empty()
    }

    /// Returns a reference to the internal positive GCounter.
    pub fn positive(&self) -> &GCounter<N> {
        &self.positive
    }

    /// Returns a reference to the internal negative GCounter.
    pub fn negative(&self) -> &GCounter<N> {
        &self.negative
    }
}

impl<N: Clone + Eq + Hash> Default for PNCounter<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<N: Clone + Eq + Hash> CrdtValue for PNCounter<N> {
    type Value = i64;

    /// Returns the total value of the counter (sum of positive - sum of negative).
    fn value(&self) -> i64 {
        self.positive.value() as i64 - self.negative.value() as i64
    }
}

impl<N: Clone + Eq + Hash> NodeAware for PNCounter<N> {
    type NodeId = N;

    fn nodes(&self) -> impl Iterator<Item = &N> {
        // We need to collect all unique nodes
        let mut seen = Vec::new();
        for node in self.positive.nodes() {
            seen.push(node);
        }
        for node in self.negative.nodes() {
            if !seen.iter().any(|n| *n == node) {
                seen.push(node);
            }
        }
        seen.into_iter()
    }
}

impl<N: Clone + Eq + Hash> Merge for PNCounter<N> {
    /// Merges another PNCounter into this one.
    ///
    /// Merges both the positive and negative GCounters independently.
    fn merge(&mut self, other: &Self) {
        self.positive.merge(&other.positive);
        self.negative.merge(&other.negative);
    }
}

impl<N: Clone + Eq + Hash + fmt::Debug> fmt::Display for PNCounter<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "PNCounter {{ value: {}, positive: {}, negative: {}, nodes: {} }}",
            self.value(),
            self.positive.value(),
            self.negative.value(),
            self.node_count()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pncounter_basic() {
        let mut counter = PNCounter::new();
        assert_eq!(counter.value(), 0);
        assert!(counter.is_empty());

        counter.increment("node1").unwrap();
        assert_eq!(counter.value(), 1);

        counter.increment("node1").unwrap();
        assert_eq!(counter.value(), 2);

        counter.decrement("node1").unwrap();
        assert_eq!(counter.value(), 1);
    }

    #[test]
    fn test_pncounter_increment_by() {
        let mut counter = PNCounter::new();
        counter.increment_by("node1", 5).unwrap();
        assert_eq!(counter.value(), 5);

        counter.decrement_by("node1", 2).unwrap();
        assert_eq!(counter.value(), 3);
    }

    #[test]
    fn test_pncounter_node_value() {
        let mut counter = PNCounter::new();
        counter.increment_by("node1", 5).unwrap();
        counter.decrement_by("node1", 2).unwrap();

        assert_eq!(counter.node_value(&"node1"), 3);
        assert_eq!(counter.positive_count(&"node1"), 5);
        assert_eq!(counter.negative_count(&"node1"), 2);
    }

    #[test]
    fn test_pncounter_merge() {
        let mut counter1 = PNCounter::new();
        counter1.increment("node1").unwrap();
        counter1.increment("node1").unwrap();
        counter1.decrement("node1").unwrap(); // node1: +1
        counter1.increment("node2").unwrap();  // node2: +1

        let mut counter2 = PNCounter::new();
        counter2.increment("node1").unwrap();   // node1: +1
        counter2.decrement("node2").unwrap();   // node2: -1
        counter2.decrement("node3").unwrap();   // node3: -1
        counter2.decrement("node3").unwrap();   // node3: -2

        counter1.merge(&counter2);

        // node1: max(1,1)=1 pos, max(0,0)=0 neg = +1
        // node2: max(1,0)=1 pos, max(0,1)=1 neg = 0
        // node3: max(0,0)=0 pos, max(0,2)=2 neg = -2
        // Total: 1 + 0 - 2 = -1
        assert_eq!(counter1.value(), -1);
        assert_eq!(counter1.node_value(&"node1"), 1);
        assert_eq!(counter1.node_value(&"node2"), 0);
        assert_eq!(counter1.node_value(&"node3"), -2);
    }

    #[test]
    fn test_pncounter_merge_idempotent() {
        let mut counter1 = PNCounter::new();
        counter1.increment("node1").unwrap();
        counter1.decrement("node2").unwrap();

        let counter2 = counter1.clone();
        counter1.merge(&counter2);

        assert_eq!(counter1.value(), 0);
        assert_eq!(counter1.node_value(&"node1"), 1);
        assert_eq!(counter1.node_value(&"node2"), -1);
    }

    #[test]
    fn test_pncounter_merge_commutative() {
        let mut counter1 = PNCounter::new();
        counter1.increment("node1").unwrap();
        counter1.decrement("node2").unwrap();

        let mut counter2 = PNCounter::new();
        counter2.increment("node2").unwrap();
        counter2.increment("node3").unwrap();

        let mut merged1 = counter1.clone();
        merged1.merge(&counter2);

        let mut merged2 = counter2.clone();
        merged2.merge(&counter1);

        assert_eq!(merged1.value(), merged2.value());
        assert_eq!(merged1.node_value(&"node1"), merged2.node_value(&"node1"));
        assert_eq!(merged1.node_value(&"node2"), merged2.node_value(&"node2"));
        assert_eq!(merged1.node_value(&"node3"), merged2.node_value(&"node3"));
    }

    #[test]
    fn test_pncounter_merged() {
        let mut counter1 = PNCounter::new();
        counter1.increment("node1").unwrap();

        let mut counter2 = PNCounter::new();
        counter2.decrement("node2").unwrap();

        let merged = counter1.merged(&counter2);
        assert_eq!(merged.value(), 0);
        // Originals unchanged
        assert_eq!(counter1.value(), 1);
        assert_eq!(counter2.value(), -1);
    }

    #[test]
    fn test_pncounter_counts_iterator() {
        let mut counter = PNCounter::new();
        counter.increment_by("node1", 3).unwrap();
        counter.decrement_by("node1", 1).unwrap();
        counter.decrement_by("node2", 2).unwrap();

        let counts: Vec<_> = counter.counts().collect();
        assert_eq!(counts.len(), 2);

        let node1 = counts.iter().find(|(n, _, _)| **n == "node1").unwrap();
        assert_eq!(node1.1, 3); // positive
        assert_eq!(node1.2, 1); // negative

        let node2 = counts.iter().find(|(n, _, _)| **n == "node2").unwrap();
        assert_eq!(node2.1, 0); // positive
        assert_eq!(node2.2, 2); // negative
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_pncounter_serde() {
        let mut counter = PNCounter::new();
        counter.increment("node1").unwrap();
        counter.decrement("node2").unwrap();
        counter.increment("node1").unwrap();

        let json = serde_json::to_string(&counter).unwrap();
        let deserialized: PNCounter<&str> = serde_json::from_str(&json).unwrap();

        assert_eq!(counter.value(), deserialized.value());
        assert_eq!(counter.positive_count(&"node1"), deserialized.positive_count(&"node1"));
        assert_eq!(counter.negative_count(&"node2"), deserialized.negative_count(&"node2"));
    }
}