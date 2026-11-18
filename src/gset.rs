//! GSet (Grow-only Set) CRDT implementation.
//!
//! A GSet is a set that only supports adding elements.
//! Once an element is added, it cannot be removed.
//!
//! # Properties
//!
//! - **Monotonic**: Elements are only added, never removed
//! - **Commutative**: Order of additions doesn't matter
//! - **Associative**: Merging order doesn't matter
//! - **Idempotent**: Adding the same element multiple times has no effect
//! - **Merge = Union**: Merging two GSets is equivalent to set union
//!
//! # Example
//!
//! ```rust
//! use mini_crdt::{GSet, Merge, CrdtValue};
//!
//! let mut set = GSet::new();
//! set.insert("a").unwrap();
//! set.insert("b").unwrap();
//! set.insert("a").unwrap(); // Idempotent - no effect
//! assert_eq!(set.len(), 2);
//! assert!(set.contains(&"a"));
//!
//! // Merge with another set
//! let mut other = GSet::new();
//! other.insert("c").unwrap();
//! set.merge(&other);
//! assert_eq!(set.len(), 3);
//! ```

use core::fmt;
use core::hash::Hash;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{HashSet, Merge, CrdtValue, NodeAware, Vec};

/// Error type for GSet operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum GSetError {
    /// The element is invalid.
    InvalidElement,
}

impl fmt::Display for GSetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GSetError::InvalidElement => write!(f, "invalid element"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for GSetError {}

/// A Grow-only Set (GSet) CRDT.
///
/// Elements can only be added, never removed.
/// Merge is equivalent to set union.
///
/// # Type Parameters
///
/// - `T`: The element type. Must implement `Clone + Eq + Hash`.
///
/// # Invariants
///
/// - Elements are never removed
/// - Merge takes the union of both sets
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct GSet<T>
where
    T: Clone + Eq + Hash,
{
    elements: HashSet<T>,
}

impl<T: Clone + Eq + Hash> PartialEq for GSet<T> {
    fn eq(&self, other: &Self) -> bool {
        self.elements == other.elements
    }
}

impl<T: Clone + Eq + Hash> Eq for GSet<T> {}

impl<T: Clone + Eq + Hash> GSet<T> {
    /// Creates a new empty GSet.
    pub fn new() -> Self {
        Self {
            elements: HashSet::new(),
        }
    }

    /// Creates a new GSet with initial elements.
    pub fn with_elements(elements: HashSet<T>) -> Self {
        Self { elements }
    }

    /// Inserts an element into the set.
    ///
    /// Returns `true` if the element was newly inserted, `false` if it already existed.
    pub fn insert(&mut self, element: T) -> Result<bool, GSetError> {
        self.validate_element(&element)?;
        Ok(self.elements.insert(element))
    }

    /// Checks if the set contains an element.
    pub fn contains(&self, element: &T) -> bool {
        self.elements.contains(element)
    }

    /// Returns the number of elements in the set.
    pub fn len(&self) -> usize {
        self.elements.len()
    }

    /// Checks if the set is empty.
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    /// Returns an iterator over the elements.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.elements.iter()
    }

    /// Returns a vector of all elements (cloned).
    pub fn to_vec(&self) -> Vec<T> {
        self.elements.iter().cloned().collect()
    }

    /// Clears all elements. Note: This violates CRDT semantics and should only be used
    /// for testing or local state reset. In a distributed system, use a different CRDT
    /// (like ORSet) if you need removal.
    pub fn clear(&mut self) {
        self.elements.clear();
    }

    /// Validates an element. Override this to add custom validation.
    fn validate_element(&self, _element: &T) -> Result<(), GSetError> {
        Ok(())
    }
}

impl<T: Clone + Eq + Hash> Default for GSet<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone + Eq + Hash> CrdtValue for GSet<T> {
    type Value = Vec<T>;

    /// Returns all elements as a vector.
    fn value(&self) -> Vec<T> {
        self.to_vec()
    }
}

impl<T: Clone + Eq + Hash> NodeAware for GSet<T> {
    type NodeId = T;

    fn nodes(&self) -> impl Iterator<Item = &T> {
        self.elements.iter()
    }
}

impl<T: Clone + Eq + Hash> Merge for GSet<T> {
    /// Merges another GSet into this one (set union).
    fn merge(&mut self, other: &Self) {
        self.elements.extend(other.elements.iter().cloned());
    }
}

impl<T: Clone + Eq + Hash> FromIterator<T> for GSet<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self {
            elements: iter.into_iter().collect(),
        }
    }
}

impl<T: Clone + Eq + Hash + fmt::Debug> fmt::Display for GSet<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "GSet {{ len: {} }}", self.len())
    }
}

impl<T: Clone + Eq + Hash> Extend<T> for GSet<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        self.elements.extend(iter);
    }
}

impl<T: Clone + Eq + Hash> IntoIterator for GSet<T> {
    type Item = T;
    type IntoIter = <HashSet<T> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.elements.into_iter()
    }
}

impl<'a, T: Clone + Eq + Hash> IntoIterator for &'a GSet<T> {
    type Item = &'a T;
    type IntoIter = <&'a HashSet<T> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.elements.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(all(not(feature = "std"), feature = "alloc"))]
    use alloc::vec;

    #[test]
    fn test_gset_basic() {
        let mut set = GSet::new();
        assert_eq!(set.len(), 0);
        assert!(set.is_empty());

        assert!(set.insert("a").unwrap());
        assert_eq!(set.len(), 1);
        assert!(set.contains(&"a"));

        // Idempotent insert
        assert!(!set.insert("a").unwrap());
        assert_eq!(set.len(), 1);

        assert!(set.insert("b").unwrap());
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_gset_merge() {
        let mut set1 = GSet::new();
        set1.insert("a").unwrap();
        set1.insert("b").unwrap();

        let mut set2 = GSet::new();
        set2.insert("b").unwrap();
        set2.insert("c").unwrap();

        set1.merge(&set2);

        assert_eq!(set1.len(), 3);
        assert!(set1.contains(&"a"));
        assert!(set1.contains(&"b"));
        assert!(set1.contains(&"c"));
    }

    #[test]
    fn test_gset_merge_idempotent() {
        let mut set1 = GSet::new();
        set1.insert("a").unwrap();
        set1.insert("b").unwrap();

        let set2 = set1.clone();
        set1.merge(&set2);

        assert_eq!(set1.len(), 2);
    }

    #[test]
    fn test_gset_merge_commutative() {
        let mut set1 = GSet::new();
        set1.insert("a").unwrap();
        set1.insert("b").unwrap();

        let mut set2 = GSet::new();
        set2.insert("b").unwrap();
        set2.insert("c").unwrap();

        let mut merged1 = set1.clone();
        merged1.merge(&set2);

        let mut merged2 = set2.clone();
        merged2.merge(&set1);

        assert_eq!(merged1.len(), merged2.len());
        assert_eq!(merged1.contains(&"a"), merged2.contains(&"a"));
        assert_eq!(merged1.contains(&"b"), merged2.contains(&"b"));
        assert_eq!(merged1.contains(&"c"), merged2.contains(&"c"));
    }

    #[test]
    fn test_gset_merged() {
        let mut set1 = GSet::new();
        set1.insert("a").unwrap();

        let mut set2 = GSet::new();
        set2.insert("b").unwrap();

        let merged = set1.merged(&set2);
        assert_eq!(merged.len(), 2);
        // Originals unchanged
        assert_eq!(set1.len(), 1);
        assert_eq!(set2.len(), 1);
    }

    #[test]
    fn test_gset_from_iter() {
        let set: GSet<&str> = vec!["a", "b", "c"].into_iter().collect();
        assert_eq!(set.len(), 3);
        assert!(set.contains(&"a"));
        assert!(set.contains(&"b"));
        assert!(set.contains(&"c"));
    }

    #[test]
    fn test_gset_iterator() {
        let mut set = GSet::new();
        set.insert("a").unwrap();
        set.insert("b").unwrap();
        set.insert("c").unwrap();

        let elements: Vec<_> = set.iter().cloned().collect();
        assert_eq!(elements.len(), 3);
        assert!(elements.contains(&"a"));
        assert!(elements.contains(&"b"));
        assert!(elements.contains(&"c"));
    }

    #[test]
    fn test_gset_extend() {
        let mut set = GSet::new();
        set.insert("a").unwrap();
        set.extend(vec!["b", "c"]);
        assert_eq!(set.len(), 3);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_gset_serde() {
        let mut set = GSet::new();
        set.insert("a").unwrap();
        set.insert("b").unwrap();

        let json = serde_json::to_string(&set).unwrap();
        let deserialized: GSet<&str> = serde_json::from_str(&json).unwrap();

        assert_eq!(set.len(), deserialized.len());
        assert!(deserialized.contains(&"a"));
        assert!(deserialized.contains(&"b"));
    }
}