use criterion:: Criterion;
use std::time::Duration;
use advanced_cache::{Cache, CacheConfig};


fn cache_benchmark(c: &mut Criterion) {
    let config = CacheConfig::default();
    let cache = Cache::new(config);

    c.bench_function("set operation", |b| {
        b.iter(|| {
            cache.set(
                black_box("bench_key".to_string()),
                black_box("bench_value".to_string()),
                None
            ).unwrap()
        })
    });

    c.bench_function("get operation", |b| {
        cache.set("bench_key".to_string(), "bench_value".to_string(), None).unwrap();
        b.iter(|| {
            black_box(cache.get(black_box("bench_key")).unwrap())
        })
    });

    c.bench_function("bulk set 100 items", |b| {
        let items: Vec<(String, String)> = (0..100)
            .map(|i| (format!("key{}", i), format!("value{}", i)))
            .collect();
        b.iter(|| {
            cache.bulk_set(black_box(items.clone())).unwrap()
        })
    });

    c.bench_function("bulk get 100 items", |b| {
        let keys: Vec<String> = (0..100)
            .map(|i| format!("key{}", i))
            .collect();
        b.iter(|| {
            black_box(cache.bulk_get(black_box(keys.clone())).unwrap())
        })
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default().measurement_time(Duration::from_secs(10));
    targets = cache_benchmark
}
criterion_main!(benches);