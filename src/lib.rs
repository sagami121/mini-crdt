//! # mini-crdt
//!
//! Zero-dependency Conflict-free Replicated Data Types (CRDTs) for Rust.
//!
//! This crate provides a collection of CRDT implementations with no mandatory dependencies.
//! All types are designed to be `Send + Sync` and work in `no_std` environments when
//! the `std` feature is disabled.
//!
//! ## Supported CRDTs
//!
//! - **GCounter** - Grow-only Counter
//! - **PNCounter** - Positive-Negative Counter (supports increment and decrement)
//! - **GSet** - Grow-only Set
//! - **ORSet** - Observed-Remove Set (supports add and remove)
//! - **LWWRegister** - Last-Writer-Wins Register
//! - **LWWMap** - Last-Writer-Wins Map
//!
//! ## Example
//!
//! ```rust
//! use mini_crdt::{GCounter, PNCounter, GSet, ORSet, LWWRegister, LWWMap, Merge, CrdtValue};
//!
//! // GCounter - only increments
//! let mut counter = GCounter::new();
//! counter.increment("node1").unwrap();
//! counter.increment("node1").unwrap();
//! counter.increment("node2").unwrap();
//! assert_eq!(counter.value(), 3);
//!
//! // PNCounter - increments and decrements
//! let mut pnc = PNCounter::new();
//! pnc.increment("node1").unwrap();
//! pnc.increment("node1").unwrap();
//! pnc.decrement("node1").unwrap();
//! assert_eq!(pnc.value(), 1);
//!
//! // LWWRegister - last writer wins
//! let mut reg = LWWRegister::new();
//! reg.set("node1", 1, 100).unwrap(); // value=1, timestamp=100
//! reg.set("node2", 2, 200).unwrap(); // value=2, timestamp=200 (wins)
//! assert_eq!(reg.value(), Some(&2));
//!
//! // Merge example
//! let mut other = GCounter::new();
//! other.increment("node3").unwrap();
//! counter.merge(&other);
//! assert_eq!(counter.value(), 4);
//! ```
//!
//! ## Serialization
//!
//! Enable the `serde` feature to get `Serialize` and `Deserialize` implementations
//! for all CRDT types.
//!
//! ```toml
//! [dependencies]
//! mini-crdt = { version = "0.1", features = ["serde"] }
//! ```
//!
//! ## `no_std` Support
//!
//! Disable default features to use in `no_std` environments:
//!
//! ```toml
//! [dependencies]
//! mini-crdt = { version = "0.1", default-features = false }
//! ```
//!
//! # Correctness
//!
//! All CRDTs in this crate satisfy the mathematical properties required for
//! conflict-free replication:
//!
//! - **Associativity**: `merge(merge(a, b), c) == merge(a, merge(b, c))`
//! - **Commutativity**: `merge(a, b) == merge(b, a)`
//! - **Idempotency**: `merge(a, a) == a`
//!
//! These properties are verified through property-based tests.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "std")]
use std::collections::HashMap;
#[cfg(feature = "std")]
use std::collections::HashSet;
#[cfg(feature = "std")]
use std::vec::Vec;

#[cfg(all(not(feature = "std"), feature = "alloc"))]
extern crate alloc;
#[cfg(all(not(feature = "std"), feature = "alloc"))]
use hashbrown::{HashMap, HashSet};
#[cfg(all(not(feature = "std"), feature = "alloc"))]
use alloc::vec::Vec;

use core::hash::Hash;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

mod gcounter;
mod pncounter;
mod gset;
mod orset;
mod lww_register;
mod lww_map;

pub use gcounter::GCounter;
pub use pncounter::PNCounter;
pub use gset::GSet;
pub use orset::ORSet;
pub use lww_register::LWWRegister;
pub use lww_map::LWWMap;

/// Trait for types that can be merged (join operation in the CRDT semilattice).
///
/// All CRDTs in this crate implement this trait. The merge operation must be:
/// - **Associative**: `merge(merge(a, b), c) == merge(a, merge(b, c))`
/// - **Commutative**: `merge(a, b) == merge(b, a)`
/// - **Idempotent**: `merge(a, a) == a`
pub trait Merge: Sized + Clone {
    /// Merges `other` into `self`, returning the joined state.
    fn merge(&mut self, other: &Self);

    /// Returns a new merged value without modifying either input.
    fn merged(&self, other: &Self) -> Self {
        let mut result = self.clone();
        result.merge(other);
        result
    }
}

/// Trait for CRDTs that have a concept of "value" that can be read.
pub trait CrdtValue {
    /// The type of the value that can be read from this CRDT.
    type Value;

    /// Returns the current value of the CRDT.
    fn value(&self) -> Self::Value;
}

/// Trait for CRDTs that track which nodes have contributed updates.
pub trait NodeAware {
    /// The type used to identify nodes.
    type NodeId: Clone + Eq + Hash;

    /// Returns the set of nodes that have contributed to this CRDT.
    fn nodes(&self) -> impl Iterator<Item = &Self::NodeId>;
}

/// A timestamp type for LWW (Last-Writer-Wins) CRDTs.
///
/// Uses a tuple of (timestamp, node_id) to break ties deterministically.
/// The node_id is used as a tiebreaker when timestamps are equal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Timestamp<N> {
    /// The logical timestamp (e.g., Lamport clock, wall-clock time, or sequence number).
    pub time: u64,
    /// The node identifier that generated this timestamp.
    pub node: N,
}

impl<N: Clone> Timestamp<N> {
    /// Creates a new timestamp.
    pub fn new(time: u64, node: N) -> Self {
        Self { time, node }
    }
}

impl<N> Timestamp<N> {
    /// Returns the time component.
    pub fn time(&self) -> u64 {
        self.time
    }

    /// Returns the node component.
    pub fn node(&self) -> &N {
        &self.node
    }
}

/// Trait for generating timestamps.
///
/// Implement this for your node ID type to provide custom timestamp generation.
pub trait TimestampProvider<N> {
    /// Generates a new timestamp that is greater than any previously generated timestamp.
    fn next_timestamp(&mut self, node: &N) -> Timestamp<N>;
}

/// A simple Lamport clock implementation for generating timestamps.
///
/// Each node maintains its own counter. The timestamp is (counter, node_id).
/// This ensures causality tracking across nodes.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(bound(serialize = "N: Serialize + Eq + Hash", deserialize = "N: Deserialize<'de> + Eq + Hash")))]
pub struct LamportClock<N> {
    counters: HashMap<N, u64>,
}

impl<N: Clone + Eq + Hash> LamportClock<N> {
    /// Creates a new Lamport clock.
    pub fn new() -> Self {
        Self {
            counters: HashMap::new(),
        }
    }

    /// Creates a new Lamport clock with a pre-existing counter for a node.
    pub fn with_counter(node: N, counter: u64) -> Self {
        let mut counters = HashMap::new();
        counters.insert(node, counter);
        Self { counters }
    }
}

impl<N: Clone + Eq + Hash> Default for LamportClock<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<N: Clone + Eq + Hash> TimestampProvider<N> for LamportClock<N> {
    fn next_timestamp(&mut self, node: &N) -> Timestamp<N> {
        let counter = self.counters.entry(node.clone()).or_insert(0);
        *counter += 1;
        Timestamp::new(*counter, node.clone())
    }
}

impl<N: Clone + Eq + Hash> LamportClock<N> {
    /// Updates the clock with a received timestamp (for causality tracking).
    pub fn update(&mut self, timestamp: &Timestamp<N>) {
        let counter = self.counters.entry(timestamp.node.clone()).or_insert(0);
        *counter = (*counter).max(timestamp.time);
    }

    /// Gets the current counter for a node.
    pub fn counter(&self, node: &N) -> u64 {
        self.counters.get(node).copied().unwrap_or(0)
    }

    /// Merges another Lamport clock into this one.
    pub fn merge(&mut self, other: &Self) {
        for (node, counter) in &other.counters {
            let entry = self.counters.entry(node.clone()).or_insert(0);
            *entry = (*entry).max(*counter);
        }
    }
}

// Re-export for convenience
pub use gcounter::GCounterError;
pub use pncounter::PNCounterError;
pub use gset::GSetError;
pub use orset::ORSetError;
pub use lww_register::LWWRegisterError;
pub use lww_map::LWWMapError;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lamport_clock() {
        let mut clock: LamportClock<&str> = LamportClock::new();

        let ts1 = clock.next_timestamp(&"node1");
        assert_eq!(ts1.time, 1);
        assert_eq!(ts1.node, "node1");

        let ts2 = clock.next_timestamp(&"node1");
        assert_eq!(ts2.time, 2);

        let ts3 = clock.next_timestamp(&"node2");
        assert_eq!(ts3.time, 1);
        assert_eq!(ts3.node, "node2");

        // Test update
        clock.update(&Timestamp::new(10, "node3"));
        assert_eq!(clock.counter(&"node3"), 10);

        // Test merge
        let mut clock2: LamportClock<&str> = LamportClock::new();
        clock2.next_timestamp(&"node1");
        clock2.next_timestamp(&"node1");
        clock2.next_timestamp(&"node2");
        clock.merge(&clock2);
        assert_eq!(clock.counter(&"node1"), 2);
        assert_eq!(clock.counter(&"node2"), 1);
    }

    #[test]
    fn test_timestamp_ordering() {
        let ts1 = Timestamp::new(1, "a");
        let ts2 = Timestamp::new(2, "a");
        let ts3 = Timestamp::new(1, "b");

        assert!(ts1 < ts2);
        assert!(ts1 < ts3); // Same time, but "a" < "b"
        assert!(ts3 < ts2); // 1 < 2
    }
}