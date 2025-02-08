use std::collections::{BinaryHeap, HashMap};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use std::cmp::Ordering;

#[derive(Eq, PartialEq)]
struct ExpirationEntry {
    expiration: Instant,
    key: String,
}

impl Ord for ExpirationEntry {
    fn cmp(&self, other: &Self) -> Ordering {
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
        let result = if let Some((value, expiration)) = data.get(key).cloned() {
            if let Some(exp) = expiration {
                if Instant::now() > exp {
                    data.remove(key);
                    return Err("Key expired".to_string());
                }
            }
            match value.parse::<i64>() {
                Ok(num) => {
                    let new_value = num + 1;
                    Ok(new_value)
                }
                Err(_) => Err("Value is not an integer".to_string()),
            }
        } else {
            Ok(1)
        };

        match result {
            Ok(new_value) => {
                data.insert(key.to_string(), (new_value.to_string(), None));
                Ok(new_value)
            }
            Err(e) => Err(e),
        }
    }

    fn decr(&self, key: &str) -> Result<i64, String> {
        let mut data = self.data.lock().unwrap();
        let result = if let Some((value, expiration)) = data.get(key).cloned() {
            if let Some(exp) = expiration {
                if Instant::now() > exp {
                    data.remove(key);
                    return Err("Key expired".to_string());
                }
            }
            match value.parse::<i64>() {
                Ok(num) => {
                    let new_value = num - 1;
                    Ok(new_value)
                }
                Err(_) => Err("Value is not an integer".to_string()),
            }
        } else {
            Ok(-1)
        };

        match result {
            Ok(new_value) => {
                data.insert(key.to_string(), (new_value.to_string(), None));
                Ok(new_value)
            }
            Err(e) => Err(e),
        }
    }

    fn cleanup_expired(&self) {
        let mut queue = self.expiration_queue.lock().unwrap();
        let mut data = self.data.lock().unwrap();
        let now = Instant::now();

        let mut expired_keys = Vec::new();
        while let Some(entry) = queue.peek() {
            if entry.expiration > now {
                break;
            }
            
            expired_keys.push(entry.key.clone());
            queue.pop();
        }

        for key in expired_keys {
            data.remove(&key);
        }
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

    cache.set("temp_key".to_string(), "temporary".to_string(), Some(Duration::from_secs(2)));
    println!("temp_key exists: {}", cache.exists("temp_key"));
    
    thread::sleep(Duration::from_secs(3));
    println!("temp_key exists after expiration: {}", cache.exists("temp_key"));

    cache.clear();
    println!("Cache cleared");
}