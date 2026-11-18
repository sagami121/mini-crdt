//! Benchmarks for mini-crdt.
//!
//! Run with: `cargo bench`

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use mini_crdt::*;

fn bench_gcounter_increment(c: &mut Criterion) {
    let mut group = c.benchmark_group("gcounter");

    group.bench_function("increment_1000", |b| {
        b.iter(|| {
            let mut counter = GCounter::new();
            for i in 0..1000 {
                counter.increment(black_box(format!("node{}", i % 10))).unwrap();
            }
            black_box(counter.value())
        })
    });

    group.bench_function("merge_1000", |b| {
        let mut counter1 = GCounter::new();
        for i in 0..500 {
            counter1.increment(format!("node{}", i % 10)).unwrap();
        }

        let mut counter2 = GCounter::new();
        for i in 500..1000 {
            counter2.increment(format!("node{}", i % 10)).unwrap();
        }

        b.iter(|| {
            let mut merged = counter1.clone();
            merged.merge(black_box(&counter2));
            black_box(merged.value())
        })
    });

    group.finish();
}

fn bench_pncounter_increment(c: &mut Criterion) {
    let mut group = c.benchmark_group("pncounter");

    group.bench_function("increment_decrement_1000", |b| {
        b.iter(|| {
            let mut counter = PNCounter::new();
            for i in 0..1000 {
                if i % 2 == 0 {
                    counter.increment(black_box(format!("node{}", i % 10))).unwrap();
                } else {
                    counter.decrement(black_box(format!("node{}", i % 10))).unwrap();
                }
            }
            black_box(counter.value())
        })
    });

    group.bench_function("merge_1000", |b| {
        let mut counter1 = PNCounter::new();
        for i in 0..500 {
            counter1.increment(format!("node{}", i % 10)).unwrap();
        }

        let mut counter2 = PNCounter::new();
        for i in 500..1000 {
            counter2.decrement(format!("node{}", i % 10)).unwrap();
        }

        b.iter(|| {
            let mut merged = counter1.clone();
            merged.merge(black_box(&counter2));
            black_box(merged.value())
        })
    });

    group.finish();
}

fn bench_gset_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("gset");

    group.bench_function("insert_1000", |b| {
        b.iter(|| {
            let mut set = GSet::new();
            for i in 0..1000 {
                set.insert(black_box(format!("key{}", i))).unwrap();
            }
            black_box(set.len())
        })
    });

    group.bench_function("merge_1000", |b| {
        let mut set1 = GSet::new();
        for i in 0..500 {
            set1.insert(format!("key{}", i)).unwrap();
        }

        let mut set2 = GSet::new();
        for i in 500..1000 {
            set2.insert(format!("key{}", i)).unwrap();
        }

        b.iter(|| {
            let mut merged = set1.clone();
            merged.merge(black_box(&set2));
            black_box(merged.len())
        })
    });

    group.finish();
}

fn bench_orset_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("orset");

    group.bench_function("insert_1000", |b| {
        b.iter(|| {
            let mut set = ORSet::new();
            for i in 0..1000 {
                set.insert(black_box(format!("node{}", i % 10)), black_box(format!("key{}", i))).unwrap();
            }
            black_box(set.len())
        })
    });

    group.bench_function("insert_remove_1000", |b| {
        b.iter(|| {
            let mut set = ORSet::new();
            for i in 0..500 {
                set.insert(format!("node{}", i % 10), format!("key{}", i)).unwrap();
            }
            for i in 0..250 {
                set.remove(&format!("node{}", i % 10), &format!("key{}", i)).unwrap();
            }
            black_box(set.len())
        })
    });

    group.bench_function("merge_1000", |b| {
        let mut set1 = ORSet::new();
        for i in 0..500 {
            set1.insert(format!("node{}", i % 10), format!("key{}", i)).unwrap();
        }

        let mut set2 = ORSet::new();
        for i in 500..1000 {
            set2.insert(format!("node{}", i % 10), format!("key{}", i)).unwrap();
        }

        b.iter(|| {
            let mut merged = set1.clone();
            merged.merge(black_box(&set2));
            black_box(merged.len())
        })
    });

    group.finish();
}

fn bench_lww_register(c: &mut Criterion) {
    let mut group = c.benchmark_group("lww_register");

    group.bench_function("set_1000", |b| {
        b.iter(|| {
            let mut reg = LWWRegister::new();
            for i in 0..1000 {
                reg.set(black_box(format!("node{}", i % 10)), black_box(format!("value{}", i)), black_box(i as u64)).unwrap();
            }
            black_box(reg.value())
        })
    });

    group.bench_function("merge_1000", |b| {
        let mut reg1 = LWWRegister::new();
        for i in 0..500 {
            reg1.set(format!("node{}", i % 10), format!("value{}", i), i as u64).unwrap();
        }

        let mut reg2 = LWWRegister::new();
        for i in 500..1000 {
            reg2.set(format!("node{}", i % 10), format!("value{}", i), i as u64).unwrap();
        }

        b.iter(|| {
            let mut merged = reg1.clone();
            merged.merge(black_box(&reg2));
            black_box(merged.value())
        })
    });

    group.finish();
}

fn bench_lww_map(c: &mut Criterion) {
    let mut group = c.benchmark_group("lww_map");

    group.bench_function("insert_1000", |b| {
        b.iter(|| {
            let mut map = LWWMap::new();
            for i in 0..1000 {
                map.insert(black_box(format!("node{}", i % 10)), black_box(format!("key{}", i)), black_box(format!("value{}", i)), black_box(i as u64)).unwrap();
            }
            black_box(map.len())
        })
    });

    group.bench_function("merge_1000", |b| {
        let mut map1 = LWWMap::new();
        for i in 0..500 {
            map1.insert(format!("node{}", i % 10), format!("key{}", i), format!("value{}", i), i as u64).unwrap();
        }

        let mut map2 = LWWMap::new();
        for i in 500..1000 {
            map2.insert(format!("node{}", i % 10), format!("key{}", i), format!("value{}", i), i as u64).unwrap();
        }

        b.iter(|| {
            let mut merged = map1.clone();
            merged.merge(black_box(&map2));
            black_box(merged.len())
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_gcounter_increment,
    bench_pncounter_increment,
    bench_gset_insert,
    bench_orset_insert,
    bench_lww_register,
    bench_lww_map
);
criterion_main!(benches);