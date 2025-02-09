use std::num::NonZeroUsize;
use std::thread;
use std::time::Duration;
use mini_redis::{Cache, CacheConfig}; 

fn main() {
   
    let config = CacheConfig::default()
        .with_max_size(NonZeroUsize::new(1000).unwrap())
        .with_default_ttl(Duration::from_secs(30))
        .with_cleanup_interval(Duration::from_secs(5));
    
    let cache = Cache::new(config);

    cache.set("key1".to_string(), "value1".to_string(), None).unwrap();
    println!("Value: {:?}", cache.get("key1").unwrap());

    cache.set(
        "temp_key".to_string(),
        "temporary".to_string(),
        Some(Duration::from_secs(2))
    ).unwrap();
    println!("Temp value exists: {:?}", cache.get("temp_key").unwrap());
    thread::sleep(Duration::from_secs(3));
    println!("After expiration: {:?}", cache.get("temp_key").unwrap());

    let items = vec![
        ("bulk1".to_string(), "value1".to_string()),
        ("bulk2".to_string(), "value2".to_string()),
        ("bulk3".to_string(), "value3".to_string()),
    ];
    cache.bulk_set(items).unwrap();

    let keys = vec!["bulk1", "bulk2", "bulk3", "nonexistent"];
    let results = cache.bulk_get(keys).unwrap();
    println!("Bulk get results: {:?}", results);

    cache.set("cas_key".to_string(), "old_value".to_string(), None).unwrap();
    let cas_result = cache.compare_and_swap(
        "cas_key",
        "old_value",
        "new_value".to_string()
    ).unwrap();
    println!("CAS operation succeeded: {}", cas_result);

    let stats = cache.get_stats().unwrap();
    println!("\nCache Statistics:");
    println!("Hits: {}", stats.hits());
    println!("Misses: {}", stats.misses());
    println!("Evictions: {}", stats.evictions());
    println!("Hit Rate: {:.2}%", stats.hit_rate());


    cache.update_ttl("key1", Duration::from_secs(60)).unwrap();
}