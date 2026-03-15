use criterion::{Criterion, criterion_group, criterion_main};
use learn_std::HashMap;
use rand::prelude::*;
use std::collections::HashMap as StdHashMap;
use std::hint::black_box;

fn bench_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("Insert");
    let size = 10000;
    let mut rng = StdRng::seed_from_u64(42);
    let keys: Vec<u64> = (0..size).map(|_| rng.next_u64()).collect();

    group.bench_function("Custom HashMap", |b| {
        b.iter(|| {
            let mut map = HashMap::with_capacity(size as usize);
            for &key in &keys {
                map.insert(key, key);
            }
            black_box(map);
        })
    });

    group.bench_function("Std HashMap", |b| {
        b.iter(|| {
            let mut map = StdHashMap::with_capacity(size as usize);
            for &key in &keys {
                map.insert(key, key);
            }
            black_box(map);
        })
    });
    group.finish();
}

fn bench_get(c: &mut Criterion) {
    let mut group = c.benchmark_group("Get");
    let size = 10000;
    let mut rng = StdRng::seed_from_u64(42);
    let keys: Vec<u64> = (0..size).map(|_| rng.next_u64()).collect();

    let mut my_map = HashMap::with_capacity(size as usize);
    let mut std_map = StdHashMap::with_capacity(size as usize);
    for &key in &keys {
        my_map.insert(key, key);
        std_map.insert(key, key);
    }

    group.bench_function("Custom Get", |b| {
        b.iter(|| {
            for &key in &keys {
                black_box(my_map.get(&key));
            }
        })
    });

    group.bench_function("Std Get", |b| {
        b.iter(|| {
            for &key in &keys {
                black_box(std_map.get(&key));
            }
        })
    });
    group.finish();
}

criterion_group!(benches, bench_insert, bench_get);
criterion_main!(benches);
