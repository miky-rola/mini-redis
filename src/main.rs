use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Clone)]
struct Cache {
    data: Arc<Mutex<HashMap<String, (String, Option<Instant>)>>>,
}

impl Cache {
    fn new() -> Self {
        Cache {
            data: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn set(&self, key: String, value: String, ttl: Option<Duration>) {
        let expiration = ttl.map(|duration| Instant::now() + duration);
        let mut data = self.data.lock().unwrap();
        data.insert(key, (value, expiration));
    }

    fn get(&self, key: &str) -> Option<String> {
        let mut data = self.data.lock().unwrap();
        if let Some((value, expiration)) = data.get(key) {
            if let Some(exp) = expiration {
                if Instant::now() > *exp {
                    // Key has expired, remove it
                    data.remove(key);
                    return None;
                }
            }
            return Some(value.clone());
        }
        None
    }

    fn remove(&self, key: &str) -> Option<String> {
        let mut data = self.data.lock().unwrap();
        data.remove(key).map(|(value, _)| value)
    }

    fn clear(&self) {
        let mut data = self.data.lock().unwrap();
        data.clear();
    }
}

fn main() {
    let cache = Cache::new();

    // Set a key with a TTL of 2 seconds
    cache.set("key1".to_string(), "value1".to_string(), Some(Duration::from_secs(2)));

    // Retrieve the key immediately
    if let Some(value) = cache.get("key1") {
        println!("Found key1: {}", value);
    } else {
        println!("key1 not found or expired");
    }

    // Wait for 3 seconds to let the key expire
    thread::sleep(Duration::from_secs(3));

    // Try to retrieve the key again
    if let Some(value) = cache.get("key1") {
        println!("Found key1: {}", value);
    } else {
        println!("key1 not found or expired");
    }

    // Test removing a key
    cache.set("key2".to_string(), "value2".to_string(), None);
    if let Some(value) = cache.remove("key2") {
        println!("Removed key2: {}", value);
    } else {
        println!("key2 not found");
    }

    // Clear the cache
    cache.clear();
    println!("Cache cleared");
}