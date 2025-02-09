# Mini-Redis

A lightweight, thread-safe in-memory cache system implemented in Rust, inspired by Redis. This implementation provides a subset of Redis-like functionality with a focus on simplicity and performance. It's designed to be a lightweight, in-memory cache suitable for small to medium-sized applications that need Redis-like functionality without the full Redis deployment.


## Features

- Thread-safe operations using `RwLock` for optimal read/write concurrency
- TTL (Time-To-Live) support for automatic key expiration
- LRU (Least Recently Used) eviction policy
- Bulk operations for efficient multiple key/value handling
- Compare-and-swap operations for atomic updates
- Statistics tracking (hits, misses, evictions)
- Configurable cache size limits
- Clean and modular Rust implementation

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
mini_redis = "0.1.0"
```

Basic usage example:

```rust
use mini_redis::{Cache, CacheConfig};
use std::time::Duration;

fn main() {
    // Create a new cache with default configuration
    let cache = Cache::new(CacheConfig::default());

    // Set a value with TTL
    cache.set(
        "key".to_string(),
        "value".to_string(),
        Some(Duration::from_secs(60))
    ).unwrap();

    // Get a value
    if let Ok(Some(value)) = cache.get("key") {
        println!("Value: {}", value);
    }
}
```

## Features

### Bulk Operations

```rust
let items = vec![
    ("key1".to_string(), "value1".to_string()),
    ("key2".to_string(), "value2".to_string())
];
cache.bulk_set(items).unwrap();

let keys = vec!["key1", "key2"];
let values = cache.bulk_get(keys).unwrap();
```

### Compare and Swap

```rust
let success = cache.compare_and_swap(
    "key",
    "old_value",
    "new_value".to_string()
).unwrap();
```

### Statistics

```rust
let stats = cache.get_stats().unwrap();
println!("Cache hit rate: {}%", stats.hit_rate());
```

## Configuration

```rust
use std::num::NonZeroUsize;

let config = CacheConfig::default()
    .with_max_size(NonZeroUsize::new(1000).unwrap())
    .with_default_ttl(Duration::from_secs(30))
    .with_cleanup_interval(Duration::from_secs(5));

let cache = Cache::new(config);
```

## Project Structure

- `src/lib.rs` - Main library entry point
- `src/cache.rs` - Core cache implementation
- `src/config.rs` - Configuration handling
- `src/error.rs` - Error types
- `src/stats.rs` - Statistics tracking
- `src/types.rs` - Internal type definitions
- `benches/` - Performance benchmarks

## Performance

The cache is designed with performance in mind:
- Uses `RwLock` for better read concurrency
- Efficient bulk operations
- Background cleanup thread for expired entries
- LRU eviction for memory management

## Inspiration

This project takes inspiration from the Redis source code but implements a minimal subset of the features. It's designed to be a lightweight, in-memory cache suitable for small to medium-sized applications that need Redis-like functionality without the full Redis deployment.


## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.



## Author

miky-rola  mikyrola8@gmail.com