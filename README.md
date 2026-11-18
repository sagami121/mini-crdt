# mini-crdt

[![Crates.io](https://img.shields.io/crates/v/mini-crdt.svg)](https://crates.io/crates/mini-crdt)
[![Documentation](https://docs.rs/mini-crdt/badge.svg)](https://docs.rs/mini-crdt)
[![License](https://img.shields.io/crates/l/mini-crdt.svg)](https://github.com/sagami121/mini-crdt/blob/main/LICENSE)
[![Build Status](https://github.com/sagami121/mini-crdt/workflows/CI/badge.svg)](https://github.com/sagami121/mini-crdt/actions)

Zero-dependency Conflict-free Replicated Data Types (CRDTs) for Rust.

## Features

- **Zero mandatory dependencies** - Works in `no_std` environments
- **Multiple CRDT types** - GCounter, PNCounter, GSet, ORSet, LWW-Register, LWW-Map
- **Mathematically correct** - All CRDTs satisfy associativity, commutativity, and idempotency
- **Property-based tested** - Verified with proptest
- **Serialization support** - Optional `serde` feature
- **Well documented** - Comprehensive docs with examples

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
mini-crdt = "0.1"
```

For serialization support:

```toml
[dependencies]
mini-crdt = { version = "0.1", features = ["serde"] }
```

For `no_std` environments:

```toml
[dependencies]
mini-crdt = { version = "0.1", default-features = false }
```

## CRDT Types

### GCounter (Grow-only Counter)
A counter that only supports increments. Tracks increments per node.

```rust
use mini_crdt::GCounter;

let mut counter = GCounter::new();
counter.increment("node1");
counter.increment("node1");
counter.increment("node2");
assert_eq!(counter.value(), 3);
```

### PNCounter (Positive-Negative Counter)
A counter supporting both increments and decrements. Implemented as two GCounters.

```rust
use mini_crdt::PNCounter;

let mut counter = PNCounter::new();
counter.increment("node1");
counter.increment("node1");
counter.decrement("node1");
assert_eq!(counter.value(), 1);
```

### GSet (Grow-only Set)
A set that only supports adding elements. Merge is set union.

```rust
use mini_crdt::GSet;

let mut set = GSet::new();
set.insert("a");
set.insert("b");
set.insert("a"); // Idempotent
assert_eq!(set.len(), 2);
```

### ORSet (Observed-Remove Set)
A set supporting both add and remove operations. Uses unique tags per addition.

```rust
use mini_crdt::ORSet;

let mut set = ORSet::new();
set.insert("node1", "a");
set.insert("node1", "b");
set.remove("node1", "a");
assert!(!set.contains("a"));
assert!(set.contains("b"));
```

### LWW-Register (Last-Writer-Wins Register)
A single value with timestamp-based conflict resolution.

```rust
use mini_crdt::LWWRegister;

let mut reg = LWWRegister::new();
reg.set("node1", "value1", 100);
reg.set("node2", "value2", 200); // Wins (higher timestamp)
assert_eq!(reg.value(), Some(&"value2"));
```

### LWW-Map (Last-Writer-Wins Map)
A map with per-key LWW-Register semantics.

```rust
use mini_crdt::LWWMap;

let mut map = LWWMap::new();
map.insert("node1", "key1", "value1", 100);
map.insert("node2", "key1", "value2", 200); // Wins for key1
assert_eq!(map.get("key1"), Some(&"value2"));
```

## Merging

All CRDTs implement the `Merge` trait:

```rust
use mini_crdt::{GCounter, Merge};

let mut counter1 = GCounter::new();
counter1.increment("node1").unwrap();

let mut counter2 = GCounter::new();
counter2.increment("node2").unwrap();

counter1.merge(&counter2);
// counter1 now has counts from both nodes
```

## Distributed Usage Example

```rust
use mini_crdt::{ORSet, LWWMap};

// Simulate a collaborative document editing session
let mut alice = ORSet::new();
let mut bob = ORSet::new();

// Alice adds paragraphs
alice.insert("alice", "Introduction").unwrap();
alice.insert("alice", "Conclusion").unwrap();

// Bob adds and removes
bob.insert("bob", "Introduction").unwrap();
bob.insert("bob", "Methodology").unwrap();
bob.remove(&"bob", &"Introduction").unwrap(); // Bob removes Introduction

// Sync: merge all changes
let mut document = alice;
document.merge(&bob);

// Introduction still exists (Alice's add not observed by Bob's remove)
// Methodology exists (Bob's add)
// Conclusion exists (Alice's add)
assert!(document.contains("Introduction"));
assert!(document.contains("Methodology"));
assert!(document.contains("Conclusion"));
```

## Lamport Clocks

For generating timestamps in distributed systems:

```rust
use mini_crdt::{LamportClock, LWWRegister};

let mut clock = LamportClock::new();

// Node 1 generates timestamps
let ts1 = clock.next_timestamp(&"node1"); // (1, "node1")
let ts2 = clock.next_timestamp(&"node1"); // (2, "node1")

// Node 2 generates timestamps
let ts3 = clock.next_timestamp(&"node2"); // (1, "node2")

// Update clock with received timestamp (causality tracking)
clock.update(&Timestamp::new(10, "node3"));

// Use with LWW-Register
let mut reg = LWWRegister::new();
reg.set_with_timestamp("value", ts1).unwrap();
```

## CRDT Properties

All CRDTs in this crate satisfy:

- **Associativity**: `merge(merge(a, b), c) == merge(a, merge(b, c))`
- **Commutativity**: `merge(a, b) == merge(b, a)`
- **Idempotency**: `merge(a, a) == a`

These properties are verified through extensive property-based testing.

## Testing

```bash
# Run all tests
cargo test

# Run with serde feature
cargo test --features serde

# Run property-based tests
cargo test --test proptests

# Run benchmarks
cargo bench
```

## no_std Support

Disable default features for `no_std` environments:

```toml
[dependencies]
mini-crdt = { version = "0.1", default-features = false }
```

Note: In `no_std`, you'll need to provide your own allocator and use `core` instead of `std`.

## License

Licensed under either of:
- MIT license

## Contributing

Contributions are welcome! Please read our [Contributing Guide](CONTRIBUTING.md) for details.

## References

- [CRDTs: An Overview](https://hal.inria.fr/inria-00555588/document) - Shapiro et al.
- [A comprehensive study of CRDTs](https://arxiv.org/abs/1806.10254) - Baquero et al.
- [Riak DT](https://github.com/basho/riak_dt) - Erlang CRDT library