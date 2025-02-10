use std::collections::{BinaryHeap, HashMap};
use std::sync::{Arc, Mutex, RwLock};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::config::CacheConfig;
use crate::error::CacheError;
use crate::stats::CacheStats;
use crate::types::{CacheEntry, ExpirationEntry};

#[derive(Clone)]
pub struct Cache {
    data: Arc<RwLock<HashMap<String, CacheEntry>>>,
    expiration_queue: Arc<Mutex<BinaryHeap<ExpirationEntry>>>,
    stats: Arc<RwLock<CacheStats>>,
    config: CacheConfig,
    running: Arc<AtomicBool>,
    cleanup_thread: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl Cache {
    pub fn new(config: CacheConfig) -> Self {
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = running.clone();
        
        let cache = Cache {
            data: Arc::new(RwLock::new(HashMap::new())),
            expiration_queue: Arc::new(Mutex::new(BinaryHeap::new())),
            stats: Arc::new(RwLock::new(CacheStats::default())),
            config,
            running,
            cleanup_thread: Arc::new(Mutex::new(None)),
        };

        let cleanup_cache = cache.clone();
        let handle = thread::spawn(move || {
            while running_clone.load(Ordering::Relaxed) {
                cleanup_cache.cleanup_expired();
                thread::sleep(cleanup_cache.config.cleanup_interval);
            }
        });

        if let Ok(mut cleanup_thread) = cache.cleanup_thread.lock() {
            *cleanup_thread = Some(handle);
        }

        cache
    }

    /// Sets a value in the cache with an optional TTL
    pub fn set(&self, key: String, value: String, ttl: Option<Duration>) -> Result<(), CacheError> {
        let expiration = ttl.or(self.config.default_ttl)
            .map(|duration| Instant::now() + duration);

        if let Some(exp) = expiration {
            let mut queue = self.expiration_queue.lock()
                .map_err(|_| CacheError::LockError)?;
            queue.push(ExpirationEntry {
                expiration: exp,
                key: key.clone(),
            });
        }

        let mut data = self.data.write()
            .map_err(|_| CacheError::LockError)?;

        if let Some(max_size) = self.config.max_size {
            if data.len() >= max_size.get() && !data.contains_key(&key) {
                self.evict_entry(&mut data)?;
            }
        }

        data.insert(key, CacheEntry {
            value,
            expiration,
            last_accessed: Instant::now(),
            access_count: 0,
        });

        Ok(())
    }

    pub fn get(&self, key: &str) -> Result<Option<String>, CacheError> {
        let mut data = self.data.write()
            .map_err(|_| CacheError::LockError)?;
        
        let mut stats = self.stats.write()
            .map_err(|_| CacheError::LockError)?;

        if let Some(entry) = data.get_mut(key) {
            if let Some(exp) = entry.expiration {
                if Instant::now() > exp {
                    data.remove(key);
                    stats.misses += 1;
                    return Ok(None);
                }
            }
            
            entry.last_accessed = Instant::now();
            entry.access_count += 1;
            stats.hits += 1;
            Ok(Some(entry.value.clone()))
        } else {
            stats.misses += 1;
            Ok(None)
        }
    }

    pub fn get_stats(&self) -> Result<CacheStats, CacheError> {
        self.stats.read()
            .map_err(|_| CacheError::LockError)
            .map(|stats| stats.clone())
    }

    /// updates the TTL for an existing key
    pub fn update_ttl(&self, key: &str, ttl: Duration) -> Result<bool, CacheError> {
        let mut data = self.data.write()
            .map_err(|_| CacheError::LockError)?;
        
        if let Some(entry) = data.get_mut(key) {
            let new_expiration = Instant::now() + ttl;
            entry.expiration = Some(new_expiration);
            
            let mut queue = self.expiration_queue.lock()
                .map_err(|_| CacheError::LockError)?;
            queue.push(ExpirationEntry {
                expiration: new_expiration,
                key: key.to_string(),
            });
            
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// performs atomic compare and swap operation
    pub fn compare_and_swap(&self, key: &str, expected: &str, new_value: String) -> Result<bool, CacheError> {
        let mut data = self.data.write()
            .map_err(|_| CacheError::LockError)?;
        
        if let Some(entry) = data.get_mut(key) {
            if entry.value == expected {
                entry.value = new_value;
                Ok(true)
            } else {
                Ok(false)
            }
        } else {
            Ok(false)
        }
    }

    fn cleanup_expired(&self) {
        if let Ok(mut queue) = self.expiration_queue.lock() {
            if let Ok(mut data) = self.data.write() {
                if let Ok(mut stats) = self.stats.write() {
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
                        if data.remove(&key).is_some() {
                            stats.evictions += 1;
                        }
                    }
                }
            }
        }
    }

    fn evict_entry(&self, data: &mut HashMap<String, CacheEntry>) -> Result<(), CacheError> {
        if let Some((key_to_remove, _)) = data.iter()
            .min_by_key(|(_, entry)| (entry.last_accessed, entry.access_count)) {
            let key_to_remove = key_to_remove.clone();
            data.remove(&key_to_remove);
            
            if let Ok(mut stats) = self.stats.write() {
                stats.evictions += 1;
            }
        }
        Ok(())
    }

    pub fn bulk_set<I>(&self, items: I) -> Result<(), CacheError>
    where
        I: IntoIterator<Item = (String, String)>,
    {
        for (key, value) in items {
            self.set(key, value, None)?;
        }
        Ok(())
    }

    /// Gets multiple values at once
    pub fn bulk_get<I, T>(&self, keys: I) -> Result<HashMap<T, Option<String>>, CacheError>
    where
        I: IntoIterator<Item = T>,
        T: Into<String> + Clone + std::hash::Hash + Eq,
    {
        let mut results = HashMap::new();
        for key in keys {
            results.insert(key.clone(), self.get(&key.into())?);
        }
        Ok(results)
    }
}

impl Drop for Cache {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        
        // take ownership of the thread handle and wait for it to finish
        if let Ok(mut cleanup_thread) = self.cleanup_thread.lock() {
            if let Some(handle) = cleanup_thread.take() {
                let _ = handle.join();
            }
        }
    }
}