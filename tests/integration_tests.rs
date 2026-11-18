//! Integration tests for mini-crdt.
//!
//! These tests verify the CRDT properties (associativity, commutativity, idempotency)
//! and test realistic distributed scenarios.

use mini_crdt::*;

#[test]
fn test_gcounter_crdt_properties() {
    // Test associativity: merge(merge(a, b), c) == merge(a, merge(b, c))
    let mut a = GCounter::new();
    a.increment("n1").unwrap();
    a.increment("n2").unwrap();

    let mut b = GCounter::new();
    b.increment("n2").unwrap();
    b.increment("n3").unwrap();

    let mut c = GCounter::new();
    c.increment("n3").unwrap();
    c.increment("n4").unwrap();

    let mut ab_c = a.clone();
    ab_c.merge(&b);
    ab_c.merge(&c);

    let mut a_bc = a.clone();
    let mut bc = b.clone();
    bc.merge(&c);
    a_bc.merge(&bc);

    assert_eq!(ab_c.value(), a_bc.value());
    assert_eq!(ab_c.count(&"n1"), a_bc.count(&"n1"));
    assert_eq!(ab_c.count(&"n2"), a_bc.count(&"n2"));
    assert_eq!(ab_c.count(&"n3"), a_bc.count(&"n3"));
    assert_eq!(ab_c.count(&"n4"), a_bc.count(&"n4"));

    // Test commutativity: merge(a, b) == merge(b, a)
    let mut ab = a.clone();
    ab.merge(&b);

    let mut ba = b.clone();
    ba.merge(&a);

    assert_eq!(ab.value(), ba.value());

    // Test idempotency: merge(a, a) == a
    let mut aa = a.clone();
    aa.merge(&a);
    assert_eq!(aa.value(), a.value());
}

#[test]
fn test_pncounter_crdt_properties() {
    let mut a = PNCounter::new();
    a.increment("n1").unwrap();
    a.increment("n1").unwrap();
    a.decrement("n2").unwrap();

    let mut b = PNCounter::new();
    b.increment("n2").unwrap();
    b.decrement("n1").unwrap();
    b.decrement("n3").unwrap();

    let mut c = PNCounter::new();
    c.increment("n3").unwrap();
    c.increment("n4").unwrap();

    // Associativity
    let mut ab_c = a.clone();
    ab_c.merge(&b);
    ab_c.merge(&c);

    let mut a_bc = a.clone();
    let mut bc = b.clone();
    bc.merge(&c);
    a_bc.merge(&bc);

    assert_eq!(ab_c.value(), a_bc.value());
    assert_eq!(ab_c.node_value(&"n1"), a_bc.node_value(&"n1"));
    assert_eq!(ab_c.node_value(&"n2"), a_bc.node_value(&"n2"));
    assert_eq!(ab_c.node_value(&"n3"), a_bc.node_value(&"n3"));
    assert_eq!(ab_c.node_value(&"n4"), a_bc.node_value(&"n4"));

    // Commutativity
    let mut ab = a.clone();
    ab.merge(&b);

    let mut ba = b.clone();
    ba.merge(&a);

    assert_eq!(ab.value(), ba.value());
    assert_eq!(ab.node_value(&"n1"), ba.node_value(&"n1"));
    assert_eq!(ab.node_value(&"n2"), ba.node_value(&"n2"));
    assert_eq!(ab.node_value(&"n3"), ba.node_value(&"n3"));

    // Idempotency
    let mut aa = a.clone();
    aa.merge(&a);
    assert_eq!(aa.value(), a.value());
}

#[test]
fn test_gset_crdt_properties() {
    let mut a = GSet::new();
    a.insert("a").unwrap();
    a.insert("b").unwrap();

    let mut b = GSet::new();
    b.insert("b").unwrap();
    b.insert("c").unwrap();

    let mut c = GSet::new();
    c.insert("c").unwrap();
    c.insert("d").unwrap();

    // Associativity
    let mut ab_c = a.clone();
    ab_c.merge(&b);
    ab_c.merge(&c);

    let mut a_bc = a.clone();
    let mut bc = b.clone();
    bc.merge(&c);
    a_bc.merge(&bc);

    assert_eq!(ab_c.len(), a_bc.len());
    assert_eq!(ab_c.contains(&"a"), a_bc.contains(&"a"));
    assert_eq!(ab_c.contains(&"b"), a_bc.contains(&"b"));
    assert_eq!(ab_c.contains(&"c"), a_bc.contains(&"c"));
    assert_eq!(ab_c.contains(&"d"), a_bc.contains(&"d"));

    // Commutativity
    let mut ab = a.clone();
    ab.merge(&b);

    let mut ba = b.clone();
    ba.merge(&a);

    assert_eq!(ab.len(), ba.len());

    // Idempotency
    let mut aa = a.clone();
    aa.merge(&a);
    assert_eq!(aa.len(), a.len());
}

#[test]
fn test_orset_crdt_properties() {
    let mut a = ORSet::new();
    a.insert("n1", "a").unwrap();
    a.insert("n1", "b").unwrap();

    let mut b = ORSet::new();
    b.insert("n2", "b").unwrap();
    b.insert("n2", "c").unwrap();

    let mut c = ORSet::new();
    c.insert("n3", "c").unwrap();
    c.insert("n3", "d").unwrap();

    // Associativity
    let mut ab_c = a.clone();
    ab_c.merge(&b);
    ab_c.merge(&c);

    let mut a_bc = a.clone();
    let mut bc = b.clone();
    bc.merge(&c);
    a_bc.merge(&bc);

    assert_eq!(ab_c.len(), a_bc.len());
    assert_eq!(ab_c.contains(&"a"), a_bc.contains(&"a"));
    assert_eq!(ab_c.contains(&"b"), a_bc.contains(&"b"));
    assert_eq!(ab_c.contains(&"c"), a_bc.contains(&"c"));
    assert_eq!(ab_c.contains(&"d"), a_bc.contains(&"d"));

    // Commutativity
    let mut ab = a.clone();
    ab.merge(&b);

    let mut ba = b.clone();
    ba.merge(&a);

    assert_eq!(ab.len(), ba.len());
    assert_eq!(ab.contains(&"a"), ba.contains(&"a"));
    assert_eq!(ab.contains(&"b"), ba.contains(&"b"));
    assert_eq!(ab.contains(&"c"), ba.contains(&"c"));

    // Idempotency
    let mut aa = a.clone();
    aa.merge(&a);
    assert_eq!(aa.len(), a.len());
}

#[test]
fn test_lww_register_crdt_properties() {
    let mut a = LWWRegister::new();
    a.set("n1", "val_a", 100).unwrap();

    let mut b = LWWRegister::new();
    b.set("n2", "val_b", 200).unwrap();

    let mut c = LWWRegister::new();
    c.set("n3", "val_c", 150).unwrap();

    // Associativity
    let mut ab_c = a.clone();
    ab_c.merge(&b);
    ab_c.merge(&c);

    let mut a_bc = a.clone();
    let mut bc = b.clone();
    bc.merge(&c);
    a_bc.merge(&bc);

    assert_eq!(ab_c.value(), a_bc.value());
    assert_eq!(ab_c.time(), a_bc.time());

    // Commutativity
    let mut ab = a.clone();
    ab.merge(&b);

    let mut ba = b.clone();
    ba.merge(&a);

    assert_eq!(ab.value(), ba.value());
    assert_eq!(ab.time(), ba.time());

    // Idempotency
    let mut aa = a.clone();
    aa.merge(&a);
    assert_eq!(aa.value(), a.value());
}

#[test]
fn test_lww_map_crdt_properties() {
    let mut a = LWWMap::new();
    a.insert("n1", "k1", "v1", 100).unwrap();
    a.insert("n1", "k2", "v2", 100).unwrap();

    let mut b = LWWMap::new();
    b.insert("n2", "k2", "v2_new", 200).unwrap();
    b.insert("n2", "k3", "v3", 100).unwrap();

    let mut c = LWWMap::new();
    c.insert("n3", "k3", "v3_new", 150).unwrap();
    c.insert("n3", "k4", "v4", 100).unwrap();

    // Associativity
    let mut ab_c = a.clone();
    ab_c.merge(&b);
    ab_c.merge(&c);

    let mut a_bc = a.clone();
    let mut bc = b.clone();
    bc.merge(&c);
    a_bc.merge(&bc);

    assert_eq!(ab_c.len(), a_bc.len());
    assert_eq!(ab_c.get(&"k1"), a_bc.get(&"k1"));
    assert_eq!(ab_c.get(&"k2"), a_bc.get(&"k2"));
    assert_eq!(ab_c.get(&"k3"), a_bc.get(&"k3"));
    assert_eq!(ab_c.get(&"k4"), a_bc.get(&"k4"));

    // Commutativity
    let mut ab = a.clone();
    ab.merge(&b);

    let mut ba = b.clone();
    ba.merge(&a);

    assert_eq!(ab.len(), ba.len());
    assert_eq!(ab.get(&"k1"), ba.get(&"k1"));
    assert_eq!(ab.get(&"k2"), ba.get(&"k2"));
    assert_eq!(ab.get(&"k3"), ba.get(&"k3"));

    // Idempotency
    let mut aa = a.clone();
    aa.merge(&a);
    assert_eq!(aa.len(), a.len());
}

#[test]
fn test_distributed_scenario_gcounter() {
    // Simulate 3 nodes incrementing a counter independently
    let mut node1 = GCounter::new();
    let mut node2 = GCounter::new();
    let mut node3 = GCounter::new();

    // Node 1 increments 5 times
    for _ in 0..5 {
        node1.increment("node1").unwrap();
    }

    // Node 2 increments 3 times
    for _ in 0..3 {
        node2.increment("node2").unwrap();
    }

    // Node 3 increments 7 times
    for _ in 0..7 {
        node3.increment("node3").unwrap();
    }

    // Merge all (order shouldn't matter)
    let mut merged = node1.clone();
    merged.merge(&node2);
    merged.merge(&node3);

    assert_eq!(merged.value(), 15);
    assert_eq!(merged.count(&"node1"), 5);
    assert_eq!(merged.count(&"node2"), 3);
    assert_eq!(merged.count(&"node3"), 7);

    // Verify different merge order gives same result
    let mut merged2 = node3.clone();
    merged2.merge(&node1);
    merged2.merge(&node2);
    assert_eq!(merged.value(), merged2.value());
}

#[test]
fn test_distributed_scenario_orset() {
    // Simulate collaborative editing scenario
    let mut alice = ORSet::new();
    let mut bob = ORSet::new();
    let mut charlie = ORSet::new();

    // Alice adds "hello" and "world"
    let alice_hello = alice.insert("alice", "hello").unwrap();
    let alice_world = alice.insert("alice", "world").unwrap();

    // Bob adds "world" and "!"
    let bob_world = bob.insert("bob", "world").unwrap();
    let bob_bang = bob.insert("bob", "!").unwrap();

    // Charlie adds "hello" and "crdt"
    let charlie_hello = charlie.insert("charlie", "hello").unwrap();
    let charlie_crdt = charlie.insert("charlie", "crdt").unwrap();

    // Merge all
    let mut merged = alice.clone();
    merged.merge(&bob);
    merged.merge(&charlie);

    // All elements should be present
    assert!(merged.contains(&"hello"));
    assert!(merged.contains(&"world"));
    assert!(merged.contains(&"!"));
    assert!(merged.contains(&"crdt"));
    assert_eq!(merged.len(), 4);

    // Alice removes "world" by observing only her tag
    merged.remove_tags(&[alice_world]);
    // But Bob's "world" should still be there
    assert!(merged.contains(&"world"));

    // Bob also removes "world" by observing his tag
    merged.remove_tags(&[bob_world]);
    // Now it should be gone
    assert!(!merged.contains(&"world"));
}

#[test]
fn test_distributed_scenario_lww_map() {
    // Simulate a distributed key-value store
    let mut dc1 = LWWMap::new(); // Data center 1
    let mut dc2 = LWWMap::new(); // Data center 2
    let mut dc3 = LWWMap::new(); // Data center 3

    // DC1 writes initial values
    dc1.insert("dc1", "user:1", "Alice", 1000).unwrap();
    dc1.insert("dc1", "user:2", "Bob", 1000).unwrap();

    // DC2 updates user:1 and adds user:3
    dc2.insert("dc2", "user:1", "Alice Smith", 2000).unwrap(); // Later timestamp
    dc2.insert("dc2", "user:3", "Charlie", 1500).unwrap();

    // DC3 updates user:2
    dc3.insert("dc3", "user:2", "Robert", 2500).unwrap(); // Latest timestamp

    // Merge all
    let mut merged = dc1.clone();
    merged.merge(&dc2);
    merged.merge(&dc3);

    assert_eq!(merged.get(&"user:1"), Some(&"Alice Smith")); // DC2 wins
    assert_eq!(merged.get(&"user:2"), Some(&"Robert")); // DC3 wins
    assert_eq!(merged.get(&"user:3"), Some(&"Charlie")); // Only DC2
    assert_eq!(merged.len(), 3);
}

#[test]
fn test_concurrent_updates_same_key_lww_register() {
    // Two nodes update the same register concurrently
    let mut node1 = LWWRegister::new();
    node1.set("node1", "value_from_node1", 100).unwrap();

    let mut node2 = LWWRegister::new();
    node2.set("node2", "value_from_node2", 100).unwrap(); // Same timestamp!

    // Merge - tiebreaker should be deterministic (node ID comparison)
    let mut merged = node1.clone();
    merged.merge(&node2);

    // node2 > node1 lexicographically, so node2's value should win
    assert_eq!(merged.value(), Some(&"value_from_node2"));
}

#[test]
fn test_gcounter_overflow_handling() {
    let mut counter = GCounter::new();
    // Test large increments
    counter.increment_by("node1", u64::MAX / 2).unwrap();
    counter.increment_by("node1", u64::MAX / 2).unwrap();

    // Should not panic, value should wrap or saturate depending on implementation
    // Our implementation uses u64 which wraps, but the total can overflow
    // This test just ensures no panic
    let _ = counter.value();
}

#[test]
fn test_pncounter_negative_values() {
    let mut counter = PNCounter::new();
    counter.increment("n1").unwrap(); // +1
    counter.decrement("n1").unwrap(); // -1
    counter.decrement("n1").unwrap(); // -1

    assert_eq!(counter.value(), -1);
    assert_eq!(counter.node_value(&"n1"), -1);
    assert_eq!(counter.positive_count(&"n1"), 1);
    assert_eq!(counter.negative_count(&"n1"), 2);
}

#[cfg(feature = "serde")]
#[test]
fn test_all_crdts_serde_roundtrip() {
    // GCounter
    let mut gc = GCounter::new();
    gc.increment("n1").unwrap();
    gc.increment("n2").unwrap();
    let json = serde_json::to_string(&gc).unwrap();
    let decoded: GCounter<&str> = serde_json::from_str(&json).unwrap();
    assert_eq!(gc.value(), decoded.value());

    // PNCounter
    let mut pnc = PNCounter::new();
    pnc.increment("n1").unwrap();
    pnc.decrement("n2").unwrap();
    let json = serde_json::to_string(&pnc).unwrap();
    let decoded: PNCounter<&str> = serde_json::from_str(&json).unwrap();
    assert_eq!(pnc.value(), decoded.value());

    // GSet
    let mut gs = GSet::new();
    gs.insert("a").unwrap();
    gs.insert("b").unwrap();
    let json = serde_json::to_string(&gs).unwrap();
    let decoded: GSet<&str> = serde_json::from_str(&json).unwrap();
    assert_eq!(gs.len(), decoded.len());

    // ORSet
    let mut os = ORSet::new();
    os.insert("n1", "a").unwrap();
    os.insert("n2", "b").unwrap();
    let json = serde_json::to_string(&os).unwrap();
    let decoded: ORSet<&str, &str> = serde_json::from_str(&json).unwrap();
    assert_eq!(os.len(), decoded.len());

    // LWWRegister
    let mut lww = LWWRegister::new();
    lww.set("n1", "value", 100).unwrap();
    let json = serde_json::to_string(&lww).unwrap();
    let decoded: LWWRegister<&str, &str> = serde_json::from_str(&json).unwrap();
    assert_eq!(lww.value(), decoded.value());

    // LWWMap
    let mut lwwm = LWWMap::new();
    lwwm.insert("n1", "k1", "v1", 100).unwrap();
    let json = serde_json::to_string(&lwwm).unwrap();
    let decoded: LWWMap<&str, &str, &str> = serde_json::from_str(&json).unwrap();
    assert_eq!(lwwm.len(), decoded.len());
}