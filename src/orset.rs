//! ORSet (Observed-Remove Set) CRDT implementation.
//!
//! An ORSet supports both adding and removing elements. Unlike a GSet,
//! elements can be removed, but removal only affects elements that were
//! observed (added) before the removal. This is achieved by tagging each
//! element addition with a unique tag (typically a pair of node ID and
//! a local counter).
//!
//! # Properties
//!
//! - Supports both add and remove operations
//! - **Commutative**: Order of operations doesn't matter
//! - **Associative**: Merging order doesn't matter
//! - **Idempotent**: Merging the same state multiple times has no effect
//!
//! # How It Works
//!
//! Each element addition is tagged with a unique identifier (node_id, counter).
//! When removing an element, all tags currently associated with that element
//! are "observed" and recorded in a tombstone set. An element is considered
//! present if it has at least one tag that is not in the tombstone set.
//!
//! # Example
//!
//! ```rust
//! use mini_crdt::{ORSet, Merge, CrdtValue};
//!
//! let mut set = ORSet::new();
//! set.insert("node1", "a").unwrap();
//! set.insert("node1", "b").unwrap();
//! assert!(set.contains(&"a"));
//!
//! set.remove(&"node1", &"a").unwrap();
//! assert!(!set.contains(&"a"));
//! assert!(set.contains(&"b"));
//!
//! // Merge with another set
//! let mut other = ORSet::new();
//! other.insert("node2", "c").unwrap();
//! set.merge(&other);
//! assert!(set.contains(&"c"));
//! ```

use core::fmt;
use core::hash::Hash;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{HashMap, HashSet, Merge, CrdtValue, NodeAware, GCounter, Vec};

/// A unique tag for an element addition.
/// Consists of a node identifier and a local counter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Tag<N> {
    /// The node that created this tag.
    pub node: N,
    /// A local counter unique to this node.
    pub counter: u64,
}

impl<N: Clone> Tag<N> {
    /// Creates a new tag.
    pub fn new(node: N, counter: u64) -> Self {
        Self { node, counter }
    }
}

/// Error type for ORSet operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ORSetError {
    /// The element is invalid.
    InvalidElement,
    /// The node ID is invalid.
    InvalidNodeId,
    /// The element was not found (for remove operations).
    ElementNotFound,
}

impl fmt::Display for ORSetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ORSetError::InvalidElement => write!(f, "invalid element"),
            ORSetError::InvalidNodeId => write!(f, "invalid node ID"),
            ORSetError::ElementNotFound => write!(f, "element not found"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ORSetError {}

/// An Observed-Remove Set (ORSet) CRDT.
///
/// Supports both adding and removing elements. Each addition is tagged
/// with a unique identifier, and removals observe all current tags for
/// an element.
///
/// # Type Parameters
///
/// - `T`: The element type. Must implement `Clone + Eq + Hash`.
/// - `N`: The node identifier type. Must implement `Clone + Eq + Hash`.
///
/// # Invariants
///
/// - An element is present iff it has at least one tag not in the tombstone set
/// - Tags are never reused (monotonically increasing per node)
/// - Tombstones only grow
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ORSet<T, N>
where
    T: Clone + Eq + Hash,
    N: Clone + Eq + Hash,
{
    /// Map from element to its set of tags.
    elements: HashMap<T, HashSet<Tag<N>>>,
    /// Tombstone set: tags that have been observed as removed.
    tombstones: HashSet<Tag<N>>,
    /// Per-node counter for generating unique tags.
    counters: GCounter<N>,
}

impl<T: Clone + Eq + Hash, N: Clone + Eq + Hash> PartialEq for ORSet<T, N> {
    fn eq(&self, other: &Self) -> bool {
        self.elements == other.elements
            && self.tombstones == other.tombstones
            && self.counters == other.counters
    }
}

impl<T: Clone + Eq + Hash, N: Clone + Eq + Hash> Eq for ORSet<T, N> {}

impl<T: Clone + Eq + Hash, N: Clone + Eq + Hash> ORSet<T, N> {
    /// Creates a new empty ORSet.
    pub fn new() -> Self {
        Self {
            elements: HashMap::new(),
            tombstones: HashSet::new(),
            counters: GCounter::new(),
        }
    }

    /// Creates a new ORSet with pre-existing state.
    pub fn with_state(
        elements: HashMap<T, HashSet<Tag<N>>>,
        tombstones: HashSet<Tag<N>>,
        counters: GCounter<N>,
    ) -> Self {
        Self {
            elements,
            tombstones,
            counters,
        }
    }

    /// Inserts an element with a new unique tag.
    ///
    /// Returns the tag that was created for this insertion.
    pub fn insert(&mut self, node: N, element: T) -> Result<Tag<N>, ORSetError> {
        self.validate_node(&node)?;
        self.validate_element(&element)?;

        // Get next counter for this node
        let counter = self.counters.count(&node) + 1;
        self.counters.increment_by(node.clone(), 1).map_err(|_| ORSetError::InvalidNodeId)?;

        let tag = Tag::new(node, counter);

        // Add the tag to the element's tag set
        self.elements.entry(element).or_default().insert(tag.clone());

        Ok(tag)
    }

    /// Removes an element by observing all its current tags.
    ///
    /// All tags currently associated with the element are moved to the tombstone set.
    /// Returns the number of tags that were observed (moved to tombstones).
    pub fn remove(&mut self, node: &N, element: &T) -> Result<usize, ORSetError> {
        self.validate_node(node)?;
        self.validate_element(element)?;

        let tags = self.elements.get(element);
        let Some(tags) = tags else {
            return Err(ORSetError::ElementNotFound);
        };

        let count = tags.len();
        for tag in tags {
            self.tombstones.insert(tag.clone());
        }

        // Remove the element entirely since all its tags are now tombstoned
        self.elements.remove(element);

        Ok(count)
    }

    /// Removes an element using specific tags (for more precise control).
    ///
    /// Only the specified tags are moved to the tombstone set.
    /// Returns the number of tags that were actually tombstoned.
    pub fn remove_tags(&mut self, tags: &[Tag<N>]) -> usize {
        let mut count = 0;
        for tag in tags {
            if self.tombstones.insert(tag.clone()) {
                count += 1;
            }
        }
        // Clean up elements that have no remaining live tags
        self.elements.retain(|_, element_tags| {
            element_tags.retain(|t| !self.tombstones.contains(t));
            !element_tags.is_empty()
        });
        count
    }

    /// Checks if an element is present (has at least one live tag).
    pub fn contains(&self, element: &T) -> bool {
        self.elements.get(element).map_or(false, |tags| {
            tags.iter().any(|t| !self.tombstones.contains(t))
        })
    }

    /// Returns the number of distinct elements in the set.
    pub fn len(&self) -> usize {
        self.elements.len()
    }

    /// Checks if the set is empty.
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    /// Returns an iterator over all present elements.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.elements.keys()
    }

    /// Returns a vector of all present elements (cloned).
    pub fn to_vec(&self) -> Vec<T> {
        self.elements.keys().cloned().collect()
    }

    /// Returns all tags for a specific element (including tombstoned ones).
    pub fn tags(&self, element: &T) -> Option<&HashSet<Tag<N>>> {
        self.elements.get(element)
    }

    /// Returns the live (non-tombstoned) tags for a specific element.
    pub fn live_tags(&self, element: &T) -> Vec<Tag<N>> {
        self.elements
            .get(element)
            .map(|tags| tags.iter().filter(|t| !self.tombstones.contains(*t)).cloned().collect())
            .unwrap_or_default()
    }

    /// Returns all tombstone tags.
    pub fn tombstones(&self) -> &HashSet<Tag<N>> {
        &self.tombstones
    }

    /// Returns the counter state.
    pub fn counters(&self) -> &GCounter<N> {
        &self.counters
    }

    /// Returns the next tag that would be generated for a node.
    pub fn next_tag(&self, node: &N) -> Tag<N> {
        let counter = self.counters.count(node) + 1;
        Tag::new(node.clone(), counter)
    }

    /// Validates a node ID. Override to add custom validation.
    fn validate_node(&self, _node: &N) -> Result<(), ORSetError> {
        Ok(())
    }

    /// Validates an element. Override to add custom validation.
    fn validate_element(&self, _element: &T) -> Result<(), ORSetError> {
        Ok(())
    }
}

impl<T: Clone + Eq + Hash, N: Clone + Eq + Hash> Default for ORSet<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone + Eq + Hash, N: Clone + Eq + Hash> CrdtValue for ORSet<T, N> {
    type Value = Vec<T>;

    /// Returns all present elements as a vector.
    fn value(&self) -> Vec<T> {
        self.to_vec()
    }
}

impl<T: Clone + Eq + Hash, N: Clone + Eq + Hash> NodeAware for ORSet<T, N> {
    type NodeId = N;

    fn nodes(&self) -> impl Iterator<Item = &N> {
        self.counters.nodes()
    }
}

impl<T: Clone + Eq + Hash, N: Clone + Eq + Hash> Merge for ORSet<T, N> {
    /// Merges another ORSet into this one.
    ///
    /// - Elements: union of tag sets
    /// - Tombstones: union of tombstone sets
    /// - Counters: merge of GCounters
    fn merge(&mut self, other: &Self) {
        // Merge elements (union of tag sets)
        for (element, tags) in &other.elements {
            self.elements
                .entry(element.clone())
                .or_default()
                .extend(tags.iter().cloned());
        }

        // Merge tombstones (union)
        self.tombstones.extend(other.tombstones.iter().cloned());

        // Merge counters
        self.counters.merge(&other.counters);

        // Clean up elements that have no live tags after merge
        self.elements.retain(|_, element_tags| {
            element_tags.retain(|t| !self.tombstones.contains(t));
            !element_tags.is_empty()
        });
    }
}

impl<T: Clone + Eq + Hash + fmt::Debug, N: Clone + Eq + Hash + fmt::Debug> fmt::Display for ORSet<T, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ORSet {{ len: {}, tombstones: {}, nodes: {} }}",
            self.len(),
            self.tombstones.len(),
            self.counters.node_count()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_orset_basic() {
        let mut set = ORSet::new();
        assert_eq!(set.len(), 0);
        assert!(set.is_empty());

        set.insert("node1", "a").unwrap();
        assert_eq!(set.len(), 1);
        assert!(set.contains(&"a"));

        set.insert("node1", "b").unwrap();
        assert_eq!(set.len(), 2);
        assert!(set.contains(&"b"));
    }

    #[test]
    fn test_orset_remove() {
        let mut set = ORSet::new();
        set.insert("node1", "a").unwrap();
        set.insert("node1", "b").unwrap();

        assert!(set.contains(&"a"));
        assert!(set.contains(&"b"));

        set.remove(&"node1", &"a").unwrap();
        assert!(!set.contains(&"a"));
        assert!(set.contains(&"b"));
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn test_orset_remove_nonexistent() {
        let mut set = ORSet::new();
        set.insert("node1", "a").unwrap();

        let result = set.remove(&"node1", &"b");
        assert!(matches!(result, Err(ORSetError::ElementNotFound)));
    }

    #[test]
    fn test_orset_reinsert_after_remove() {
        let mut set = ORSet::new();
        set.insert("node1", "a").unwrap();
        set.remove(&"node1", &"a").unwrap();
        assert!(!set.contains(&"a"));

        // Re-insert should work (new tag)
        set.insert("node1", "a").unwrap();
        assert!(set.contains(&"a"));
    }

    #[test]
    fn test_orset_concurrent_add_remove() {
        // Node1 adds "a"
        let mut set1 = ORSet::new();
        let tag1 = set1.insert("node1", "a").unwrap();

        // Node2 adds "a" concurrently
        let mut set2 = ORSet::new();
        let tag2 = set2.insert("node2", "a").unwrap();

        // Merge - both adds should be present
        set1.merge(&set2);
        assert!(set1.contains(&"a"));
        assert_eq!(set1.live_tags(&"a").len(), 2);

        // Node1 removes "a" by observing only its own tag
        set1.remove_tags(&[tag1]);
        // Should still be present because node2's add is not observed
        assert!(set1.contains(&"a"));
        assert_eq!(set1.live_tags(&"a").len(), 1);

        // Node2 removes "a" by observing its own tag
        set1.remove_tags(&[tag2]);
        // Now should be gone
        assert!(!set1.contains(&"a"));
    }

    #[test]
    fn test_orset_merge() {
        let mut set1 = ORSet::new();
        set1.insert("node1", "a").unwrap();
        set1.insert("node1", "b").unwrap();

        let mut set2 = ORSet::new();
        set2.insert("node2", "b").unwrap();
        set2.insert("node2", "c").unwrap();

        set1.merge(&set2);

        assert!(set1.contains(&"a"));
        assert!(set1.contains(&"b"));
        assert!(set1.contains(&"c"));
        assert_eq!(set1.len(), 3);
    }

    #[test]
    fn test_orset_merge_with_remove() {
        let mut set1 = ORSet::new();
        set1.insert("node1", "a").unwrap();
        set1.insert("node1", "b").unwrap();

        let mut set2 = ORSet::new();
        set2.insert("node2", "b").unwrap();
        set2.insert("node2", "c").unwrap();
        set2.remove(&"node2", &"b").unwrap(); // node2 removes "b"

        set1.merge(&set2);

        // "a" only in set1 -> present
        // "b" added by both, removed by node2 -> node1's add still live
        // "c" only in set2 -> present
        assert!(set1.contains(&"a"));
        assert!(set1.contains(&"b"));
        assert!(set1.contains(&"c"));
    }

    #[test]
    fn test_orset_merge_idempotent() {
        let mut set1 = ORSet::new();
        set1.insert("node1", "a").unwrap();
        set1.insert("node1", "b").unwrap();

        let set2 = set1.clone();
        set1.merge(&set2);

        assert_eq!(set1.len(), 2);
        assert!(set1.contains(&"a"));
        assert!(set1.contains(&"b"));
    }

    #[test]
    fn test_orset_merge_commutative() {
        let mut set1 = ORSet::new();
        set1.insert("node1", "a").unwrap();

        let mut set2 = ORSet::new();
        set2.insert("node2", "b").unwrap();

        let mut merged1 = set1.clone();
        merged1.merge(&set2);

        let mut merged2 = set2.clone();
        merged2.merge(&set1);

        assert_eq!(merged1.len(), merged2.len());
        assert_eq!(merged1.contains(&"a"), merged2.contains(&"a"));
        assert_eq!(merged1.contains(&"b"), merged2.contains(&"b"));
    }

    #[test]
    fn test_orset_merged() {
        let mut set1 = ORSet::new();
        set1.insert("node1", "a").unwrap();

        let mut set2 = ORSet::new();
        set2.insert("node2", "b").unwrap();

        let merged = set1.merged(&set2);
        assert_eq!(merged.len(), 2);
        // Originals unchanged
        assert_eq!(set1.len(), 1);
        assert_eq!(set2.len(), 1);
    }

    #[test]
    fn test_orset_counters() {
        let mut set = ORSet::new();
        set.insert("node1", "a").unwrap();
        set.insert("node1", "b").unwrap();
        set.insert("node2", "c").unwrap();

        assert_eq!(set.counters().count(&"node1"), 2);
        assert_eq!(set.counters().count(&"node2"), 1);
    }

    #[test]
    fn test_orset_remove_tags() {
        let mut set = ORSet::new();
        let tag1 = set.insert("node1", "a").unwrap();
        let tag2 = set.insert("node1", "a").unwrap();
        let tag3 = set.insert("node2", "a").unwrap();

        assert_eq!(set.live_tags(&"a").len(), 3);

        // Remove only tag1
        set.remove_tags(&[tag1]);
        assert_eq!(set.live_tags(&"a").len(), 2);
        assert!(set.contains(&"a"));

        // Remove remaining tags
        set.remove_tags(&[tag2, tag3]);
        assert!(!set.contains(&"a"));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_orset_serde() {
        let mut set = ORSet::new();
        set.insert("node1", "a").unwrap();
        set.insert("node2", "b").unwrap();
        set.remove(&"node1", &"a").unwrap();

        let json = serde_json::to_string(&set).unwrap();
        let deserialized: ORSet<&str, &str> = serde_json::from_str(&json).unwrap();

        assert_eq!(set.len(), deserialized.len());
        assert_eq!(set.contains(&"a"), deserialized.contains(&"a"));
        assert_eq!(set.contains(&"b"), deserialized.contains(&"b"));
        assert_eq!(set.tombstones().len(), deserialized.tombstones().len());
    }
}