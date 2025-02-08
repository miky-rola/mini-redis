use std::collections::{BinaryHeap, HashMap};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use std::cmp::Ordering;

// Helper struct for the expiration queue
#[derive(Eq, PartialEq)]
struct ExpirationEntry {
    expiration: Instant,
    key: String,
}

impl Ord for ExpirationEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse ordering to make BinaryHeap a min-heap
        other.expiration.cmp(&self.expiration)
    }
}

impl PartialOrd for ExpirationEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone)]
struct Cache {
    data: Arc<Mutex<HashMap<String, (String, Option<Instant>)>>>,
    expiration_queue: Arc<Mutex<BinaryHeap<ExpirationEntry>>>,
}

impl Cache {
    fn new() -> Self {
        let cache = Cache {
            data: Arc::new(Mutex::new(HashMap::new())),
            expiration_queue: Arc::new(Mutex::new(BinaryHeap::new())),
        };

        // Start background cleanup thread
        let cleanup_cache = cache.clone();
        thread::spawn(move || {
            loop {
                cleanup_cache.cleanup_expired();
                thread::sleep(Duration::from_secs(1));
            }
        });

        cache
    }

    fn set(&self, key: String, value: String, ttl: Option<Duration>) {
        let expiration = ttl.map(|duration| Instant::now() + duration);
        
        // Add to expiration queue if TTL is set
        if let Some(exp) = expiration {
            let mut queue = self.expiration_queue.lock().unwrap();
            queue.push(ExpirationEntry {
                expiration: exp,
                key: key.clone(),
            });
        }

        let mut data = self.data.lock().unwrap();
        data.insert(key, (value, expiration));
    }

    fn get(&self, key: &str) -> Option<String> {
        let mut data = self.data.lock().unwrap();
        if let Some((value, expiration)) = data.get(key) {
            if let Some(exp) = expiration {
                if Instant::now() > *exp {
                    data.remove(key);
                    return None;
                }
            }
            return Some(value.clone());
        }
        None
    }

    fn exists(&self, key: &str) -> bool {
        self.get(key).is_some()
    }

    fn incr(&self, key: &str) -> Result<i64, String> {
        let mut data = self.data.lock().unwrap();
        match data.get(key) {
            Some((value, expiration)) => {
                if let Some(exp) = expiration {
                    if Instant::now() > *exp {
                        data.remove(key);
                        return Err("Key expired".to_string());
                    }
                }
                match value.parse::<i64>() {
                    Ok(num) => {
                        let new_value = num + 1;
                        data.insert(key.to_string(), (new_value.to_string(), *expiration));
                        Ok(new_value)
                    }
                    Err(_) => Err("Value is not an integer".to_string()),
                }
            }
            None => {
                data.insert(key.to_string(), ("1".to_string(), None));
                Ok(1)
            }
        }
    }

    fn decr(&self, key: &str) -> Result<i64, String> {
        let mut data = self.data.lock().unwrap();
        match data.get(key) {
            Some((value, expiration)) => {
                if let Some(exp) = expiration {
                    if Instant::now() > *exp {
                        data.remove(key);
                        return Err("Key expired".to_string());
                    }
                }
                match value.parse::<i64>() {
                    Ok(num) => {
                        let new_value = num - 1;
                        data.insert(key.to_string(), (new_value.to_string(), *expiration));
                        Ok(new_value)
                    }
                    Err(_) => Err("Value is not an integer".to_string()),
                }
            }
            None => {
                data.insert(key.to_string(), ("-1".to_string(), None));
                Ok(-1)
            }
        }
    }

    fn cleanup_expired(&self) {
        let mut queue = self.expiration_queue.lock().unwrap();
        let mut data = self.data.lock().unwrap();
        let now = Instant::now();

        while let Some(entry) = queue.peek() {
            if entry.expiration > now {
                break;
            }
            
            // Remove expired entry
            queue.pop();
            data.remove(&entry.key);
        }
    }

    fn remove(&self, key: &str) -> Option<String> {
        let mut data = self.data.lock().unwrap();
        data.remove(key).map(|(value, _)| value)
    }

    fn clear(&self) {
        let mut data = self.data.lock().unwrap();
        let mut queue = self.expiration_queue.lock().unwrap();
        data.clear();
        queue.clear();
    }
}

fn main() {
    let cache = Cache::new();

    // Test Redis-like commands
    cache.set("counter".to_string(), "5".to_string(), None);
    
    match cache.incr("counter") {
        Ok(value) => println!("Incremented counter: {}", value),
        Err(e) => println!("Error: {}", e),
    }

    match cache.decr("counter") {
        Ok(value) => println!("Decremented counter: {}", value),
        Err(e) => println!("Error: {}", e),
    }

    println!("Counter exists: {}", cache.exists("counter"));

    // Test TTL functionality
    cache.set("temp_key".to_string(), "temporary".to_string(), Some(Duration::from_secs(2)));
    println!("temp_key exists: {}", cache.exists("temp_key"));
    
    thread::sleep(Duration::from_secs(3));
    println!("temp_key exists after expiration: {}", cache.exists("temp_key"));

    cache.clear();
    println!("Cache cleared");
}