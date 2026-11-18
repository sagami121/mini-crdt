//! Property-based tests for CRDT correctness.
//!
//! These tests use proptest to verify the mathematical properties
//! of CRDTs: associativity, commutativity, and idempotency.

use proptest::prelude::*;
use mini_crdt::*;

// Strategies for generating test data

fn node_id_strategy() -> impl Strategy<Value = String> {
    "[a-z]{1,5}".prop_map(String::from)
}

fn gcounter_strategy() -> impl Strategy<Value = GCounter<String>> {
    prop::collection::vec((node_id_strategy(), 0u64..100), 0..10)
        .prop_map(|pairs| pairs.into_iter().collect())
}

fn pncounter_strategy() -> impl Strategy<Value = PNCounter<String>> {
    prop::collection::vec(
        (node_id_strategy(), -50i64..50),
        0..10
    ).prop_map(|pairs| {
        let mut counter = PNCounter::new();
        for (node, delta) in pairs {
            if delta >= 0 {
                counter.increment_by(node, delta as u64).unwrap();
            } else {
                counter.decrement_by(node, (-delta) as u64).unwrap();
            }
        }
        counter
    })
}

fn gset_strategy() -> impl Strategy<Value = GSet<String>> {
    prop::collection::vec("[a-z]{1,5}", 0..10)
        .prop_map(|elements| elements.into_iter().collect())
}

fn orset_strategy() -> impl Strategy<Value = ORSet<String, String>> {
    prop::collection::vec(
        (node_id_strategy(), "[a-z]{1,5}"),
        0..10
    ).prop_map(|pairs| {
        let mut set = ORSet::new();
        for (node, element) in pairs {
            set.insert(node, element).unwrap();
        }
        set
    })
}

fn lww_register_strategy() -> impl Strategy<Value = LWWRegister<String, String>> {
    prop::collection::vec(
        (node_id_strategy(), "[a-z]{1,5}", 0u64..1000),
        0..5
    ).prop_map(|triplets| {
        let mut reg = LWWRegister::new();
        for (node, value, time) in triplets {
            reg.set(node, value, time).unwrap();
        }
        reg
    })
}

fn lww_map_strategy() -> impl Strategy<Value = LWWMap<String, String, String>> {
    prop::collection::vec(
        (node_id_strategy(), "[a-z]{1,5}", "[a-z]{1,5}", 0u64..1000),
        0..5
    ).prop_map(|quads| {
        let mut map = LWWMap::new();
        for (node, key, value, time) in quads {
            map.insert(node, key, value, time).unwrap();
        }
        map
    })
}

// GCounter properties

proptest! {
    #[test]
    fn gcounter_associativity(a in gcounter_strategy(), b in gcounter_strategy(), c in gcounter_strategy()) {
        let mut ab_c = a.clone();
        ab_c.merge(&b);
        ab_c.merge(&c);

        let mut a_bc = a.clone();
        let mut bc = b.clone();
        bc.merge(&c);
        a_bc.merge(&bc);

        prop_assert_eq!(ab_c.value(), a_bc.value());
        for node in ["n1", "n2", "n3", "n4", "n5"].map(String::from) {
            prop_assert_eq!(ab_c.count(&node), a_bc.count(&node));
        }
    }

    #[test]
    fn gcounter_commutativity(a in gcounter_strategy(), b in gcounter_strategy()) {
        let mut ab = a.clone();
        ab.merge(&b);

        let mut ba = b.clone();
        ba.merge(&a);

        prop_assert_eq!(ab.value(), ba.value());
        for node in ["n1", "n2", "n3", "n4", "n5"].map(String::from) {
            prop_assert_eq!(ab.count(&node), ba.count(&node));
        }
    }

    #[test]
    fn gcounter_idempotency(a in gcounter_strategy()) {
        let mut aa = a.clone();
        aa.merge(&a);
        prop_assert_eq!(aa.value(), a.value());
        for node in ["n1", "n2", "n3", "n4", "n5"].map(String::from) {
            prop_assert_eq!(aa.count(&node), a.count(&node));
        }
    }
}

// PNCounter properties

proptest! {
    #[test]
    fn pncounter_associativity(a in pncounter_strategy(), b in pncounter_strategy(), c in pncounter_strategy()) {
        let mut ab_c = a.clone();
        ab_c.merge(&b);
        ab_c.merge(&c);

        let mut a_bc = a.clone();
        let mut bc = b.clone();
        bc.merge(&c);
        a_bc.merge(&bc);

        prop_assert_eq!(ab_c.value(), a_bc.value());
        for node in ["n1", "n2", "n3", "n4", "n5"].map(String::from) {
            prop_assert_eq!(ab_c.node_value(&node), a_bc.node_value(&node));
        }
    }

    #[test]
    fn pncounter_commutativity(a in pncounter_strategy(), b in pncounter_strategy()) {
        let mut ab = a.clone();
        ab.merge(&b);

        let mut ba = b.clone();
        ba.merge(&a);

        prop_assert_eq!(ab.value(), ba.value());
        for node in ["n1", "n2", "n3", "n4", "n5"].map(String::from) {
            prop_assert_eq!(ab.node_value(&node), ba.node_value(&node));
        }
    }

    #[test]
    fn pncounter_idempotency(a in pncounter_strategy()) {
        let mut aa = a.clone();
        aa.merge(&a);
        prop_assert_eq!(aa.value(), a.value());
        for node in ["n1", "n2", "n3", "n4", "n5"].map(String::from) {
            prop_assert_eq!(aa.node_value(&node), a.node_value(&node));
        }
    }
}

// GSet properties

proptest! {
    #[test]
    fn gset_associativity(a in gset_strategy(), b in gset_strategy(), c in gset_strategy()) {
        let mut ab_c = a.clone();
        ab_c.merge(&b);
        ab_c.merge(&c);

        let mut a_bc = a.clone();
        let mut bc = b.clone();
        bc.merge(&c);
        a_bc.merge(&bc);

        prop_assert_eq!(ab_c.len(), a_bc.len());
        for elem in ["a", "b", "c", "d", "e"].map(String::from) {
            prop_assert_eq!(ab_c.contains(&elem), a_bc.contains(&elem));
        }
    }

    #[test]
    fn gset_commutativity(a in gset_strategy(), b in gset_strategy()) {
        let mut ab = a.clone();
        ab.merge(&b);

        let mut ba = b.clone();
        ba.merge(&a);

        prop_assert_eq!(ab.len(), ba.len());
        for elem in ["a", "b", "c", "d", "e"].map(String::from) {
            prop_assert_eq!(ab.contains(&elem), ba.contains(&elem));
        }
    }

    #[test]
    fn gset_idempotency(a in gset_strategy()) {
        let mut aa = a.clone();
        aa.merge(&a);
        prop_assert_eq!(aa.len(), a.len());
        for elem in ["a", "b", "c", "d", "e"].map(String::from) {
            prop_assert_eq!(aa.contains(&elem), a.contains(&elem));
        }
    }
}

// ORSet properties

proptest! {
    #[test]
    fn orset_associativity(a in orset_strategy(), b in orset_strategy(), c in orset_strategy()) {
        let mut ab_c = a.clone();
        ab_c.merge(&b);
        ab_c.merge(&c);

        let mut a_bc = a.clone();
        let mut bc = b.clone();
        bc.merge(&c);
        a_bc.merge(&bc);

        prop_assert_eq!(ab_c.len(), a_bc.len());
        for elem in ["a", "b", "c", "d", "e"].map(String::from) {
            prop_assert_eq!(ab_c.contains(&elem), a_bc.contains(&elem));
        }
    }

    #[test]
    fn orset_commutativity(a in orset_strategy(), b in orset_strategy()) {
        let mut ab = a.clone();
        ab.merge(&b);

        let mut ba = b.clone();
        ba.merge(&a);

        prop_assert_eq!(ab.len(), ba.len());
        for elem in ["a", "b", "c", "d", "e"].map(String::from) {
            prop_assert_eq!(ab.contains(&elem), ba.contains(&elem));
        }
    }

    #[test]
    fn orset_idempotency(a in orset_strategy()) {
        let mut aa = a.clone();
        aa.merge(&a);
        prop_assert_eq!(aa.len(), a.len());
        for elem in ["a", "b", "c", "d", "e"].map(String::from) {
            prop_assert_eq!(aa.contains(&elem), a.contains(&elem));
        }
    }
}

// LWWRegister properties

proptest! {
    #[test]
    fn lww_register_associativity(a in lww_register_strategy(), b in lww_register_strategy(), c in lww_register_strategy()) {
        let mut ab_c = a.clone();
        ab_c.merge(&b);
        ab_c.merge(&c);

        let mut a_bc = a.clone();
        let mut bc = b.clone();
        bc.merge(&c);
        a_bc.merge(&bc);

        prop_assert_eq!(ab_c.value(), a_bc.value());
        prop_assert_eq!(ab_c.time(), a_bc.time());
    }

    #[test]
    fn lww_register_commutativity(a in lww_register_strategy(), b in lww_register_strategy()) {
        let mut ab = a.clone();
        ab.merge(&b);

        let mut ba = b.clone();
        ba.merge(&a);

        prop_assert_eq!(ab.value(), ba.value());
        prop_assert_eq!(ab.time(), ba.time());
    }

    #[test]
    fn lww_register_idempotency(a in lww_register_strategy()) {
        let mut aa = a.clone();
        aa.merge(&a);
        prop_assert_eq!(aa.value(), a.value());
        prop_assert_eq!(aa.time(), a.time());
    }
}

// LWWMap properties

proptest! {
    #[test]
    fn lww_map_associativity(a in lww_map_strategy(), b in lww_map_strategy(), c in lww_map_strategy()) {
        let mut ab_c = a.clone();
        ab_c.merge(&b);
        ab_c.merge(&c);

        let mut a_bc = a.clone();
        let mut bc = b.clone();
        bc.merge(&c);
        a_bc.merge(&bc);

        prop_assert_eq!(ab_c.len(), a_bc.len());
        for key in ["k1", "k2", "k3", "k4", "k5"].map(String::from) {
            prop_assert_eq!(ab_c.get(&key), a_bc.get(&key));
        }
    }

    #[test]
    fn lww_map_commutativity(a in lww_map_strategy(), b in lww_map_strategy()) {
        let mut ab = a.clone();
        ab.merge(&b);

        let mut ba = b.clone();
        ba.merge(&a);

        prop_assert_eq!(ab.len(), ba.len());
        for key in ["k1", "k2", "k3", "k4", "k5"].map(String::from) {
            prop_assert_eq!(ab.get(&key), ba.get(&key));
        }
    }

    #[test]
    fn lww_map_idempotency(a in lww_map_strategy()) {
        let mut aa = a.clone();
        aa.merge(&a);
        prop_assert_eq!(aa.len(), a.len());
        for key in ["k1", "k2", "k3", "k4", "k5"].map(String::from) {
            prop_assert_eq!(aa.get(&key), a.get(&key));
        }
    }
}