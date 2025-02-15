use std::collections::{BinaryHeap, HashMap, VecDeque};
use std::time::{Duration, Instant};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};

use crate::config::CacheConfig;
use crate::error::CacheError;
use crate::stats::CacheStats;
use crate::types::{CacheEntry, ExpirationEntry};

enum CacheCommand {
    Set { key: String, value: String, ttl: Option<Duration>, resp: Sender<Result<(), CacheError>> },
    Get { key: String, resp: Sender<Result<Option<String>, CacheError>> },
    BulkSet { items: Vec<(String, String)>, resp: Sender<Result<(), CacheError>> },
    BulkGet { keys: Vec<String>, resp: Sender<Result<HashMap<String, Option<String>>, CacheError>> },
    GetStats { resp: Sender<Result<CacheStats, CacheError>> },
    UpdateTtl { key: String, ttl: Duration, resp: Sender<Result<bool, CacheError>> },
    CompareAndSwap { key: String, expected: String, new_value: String, resp: Sender<Result<bool, CacheError>> },
    Shutdown,
}

#[derive(Clone)]
pub struct Cache {
    sender: Sender<CacheCommand>,
    event_loop_handle: Arc<Option<JoinHandle<()>>>,
    running: Arc<AtomicBool>,
}

impl Cache {
    pub fn new(config: CacheConfig) -> Self {
        let (sender, receiver) = mpsc::channel();
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = running.clone();
        
        let handle = thread::spawn(move || {
            run_event_loop(receiver, config, running_clone);
        });
        
        Cache {
            sender,
            event_loop_handle: Arc::new(Some(handle)),
            running,
        }
    }

    pub fn set(&self, key: String, value: String, ttl: Option<Duration>) -> Result<(), CacheError> {
        let (resp_sender, resp_receiver) = mpsc::channel();
        self.sender.send(CacheCommand::Set { key, value, ttl, resp: resp_sender })
            .map_err(|_| CacheError::LockError)?;
        resp_receiver.recv().map_err(|_| CacheError::LockError)?
    }

    pub fn get(&self, key: &str) -> Result<Option<String>, CacheError> {
        let (resp_sender, resp_receiver) = mpsc::channel();
        self.sender.send(CacheCommand::Get { 
            key: key.to_string(), 
            resp: resp_sender,
        })
        .map_err(|_| CacheError::LockError)?;
        resp_receiver.recv().map_err(|_| CacheError::LockError)?
    }

    pub fn bulk_set<I>(&self, items: I) -> Result<(), CacheError>
    where
        I: IntoIterator<Item = (String, String)>,
    {
        let items_vec: Vec<_> = items.into_iter().collect();
        let (resp_sender, resp_receiver) = mpsc::channel();
        self.sender.send(CacheCommand::BulkSet { 
            items: items_vec, 
            resp: resp_sender,
        })
        .map_err(|_| CacheError::LockError)?;
        resp_receiver.recv().map_err(|_| CacheError::LockError)?
    }

    pub fn bulk_get<I, T>(&self, keys: I) -> Result<HashMap<T, Option<String>>, CacheError>
    where
        I: IntoIterator<Item = T>,
        T: Into<String> + Clone + std::hash::Hash + Eq,
    {
        let keys_vec: Vec<String> = keys.into_iter().map(|k| k.into()).collect();
        let keys_clone: Vec<T> = keys.into_iter().collect();
        
        let (resp_sender, resp_receiver) = mpsc::channel();
        self.sender.send(CacheCommand::BulkGet { 
            keys: keys_vec, 
            resp: resp_sender,
        })
        .map_err(|_| CacheError::LockError)?;
        
        let result = resp_receiver.recv().map_err(|_| CacheError::LockError)?;
        
        // Convert the result back to the original key type
        let mut converted_result = HashMap::new();
        if let Ok(string_result) = result {
            for (i, key) in keys_clone.iter().enumerate() {
                let string_key = key.clone().into();
                converted_result.insert(key.clone(), string_result.get(&string_key).cloned().flatten());
            }
        }
        
        Ok(converted_result)
    }

    pub fn get_stats(&self) -> Result<CacheStats, CacheError> {
        let (resp_sender, resp_receiver) = mpsc::channel();
        self.sender.send(CacheCommand::GetStats { 
            resp: resp_sender,
        })
        .map_err(|_| CacheError::LockError)?;
        resp_receiver.recv().map_err(|_| CacheError::LockError)?
    }

    pub fn update_ttl(&self, key: &str, ttl: Duration) -> Result<bool, CacheError> {
        let (resp_sender, resp_receiver) = mpsc::channel();
        self.sender.send(CacheCommand::UpdateTtl { 
            key: key.to_string(), 
            ttl, 
            resp: resp_sender,
        })
        .map_err(|_| CacheError::LockError)?;
        resp_receiver.recv().map_err(|_| CacheError::LockError)?
    }

    pub fn compare_and_swap(&self, key: &str, expected: &str, new_value: String) -> Result<bool, CacheError> {
        let (resp_sender, resp_receiver) = mpsc::channel();
        self.sender.send(CacheCommand::CompareAndSwap { 
            key: key.to_string(), 
            expected: expected.to_string(), 
            new_value, 
            resp: resp_sender,
        })
        .map_err(|_| CacheError::LockError)?;
        resp_receiver.recv().map_err(|_| CacheError::LockError)?
    }
}

impl Drop for Cache {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        let _ = self.sender.send(CacheCommand::Shutdown);
        
        if let Some(handle) = Arc::get_mut(&mut self.event_loop_handle).and_then(|opt| opt.take()) {
            let _ = handle.join();
        }
    }
}

fn run_event_loop(receiver: Receiver<CacheCommand>, config: CacheConfig, running: Arc<AtomicBool>) {
    let mut data = HashMap::new();
    let mut expiration_queue = BinaryHeap::new();
    let mut stats = CacheStats::default();
    let mut command_queue = VecDeque::new();
    
    let mut last_cleanup = Instant::now();
    
    while running.load(Ordering::Relaxed) {
        // Process any pending commands
        while let Ok(cmd) = receiver.try_recv() {
            match cmd {
                CacheCommand::Shutdown => return,
                cmd => command_queue.push_back(cmd),
            }
        }
        
        // Process one command from the queue
        if let Some(cmd) = command_queue.pop_front() {
            match cmd {
                CacheCommand::Set { key, value, ttl, resp } => {
                    let result = handle_set(&mut data, &mut expiration_queue, &config, key, value, ttl, &mut stats);
                    let _ = resp.send(result);
                },
                CacheCommand::Get { key, resp } => {
                    let result = handle_get(&mut data, &key, &mut stats);
                    let _ = resp.send(result);
                },
                CacheCommand::BulkSet { items, resp } => {
                    let mut result = Ok(());
                    for (key, value) in items {
                        if let Err(e) = handle_set(&mut data, &mut expiration_queue, &config, key, value, None, &mut stats) {
                            result = Err(e);
                            break;
                        }
                    }
                    let _ = resp.send(result);
                },
                CacheCommand::BulkGet { keys, resp } => {
                    let mut results = HashMap::new();
                    for key in keys {
                        results.insert(key.clone(), handle_get(&mut data, &key, &mut stats)?);
                    }
                    let _ = resp.send(Ok(results));
                },
                CacheCommand::GetStats { resp } => {
                    let _ = resp.send(Ok(stats.clone()));
                },
                CacheCommand::UpdateTtl { key, ttl, resp } => {
                    let result = handle_update_ttl(&mut data, &mut expiration_queue, &key, ttl);
                    let _ = resp.send(result);
                },
                CacheCommand::CompareAndSwap { key, expected, new_value, resp } => {
                    let result = handle_cas(&mut data, &key, &expected, new_value);
                    let _ = resp.send(result);
                },
                CacheCommand::Shutdown => return,
            }
        }
        
        // Check if it's time to clean up expired entries
        let now = Instant::now();
        if now.duration_since(last_cleanup) >= config.cleanup_interval {
            cleanup_expired(&mut data, &mut expiration_queue, &mut stats);
            last_cleanup = now;
        }
        
        // Small sleep to prevent busy-waiting
        thread::sleep(Duration::from_millis(1));
    }
}

fn handle_set(
    data: &mut HashMap<String, CacheEntry>,
    expiration_queue: &mut BinaryHeap<ExpirationEntry>,
    config: &CacheConfig,
    key: String,
    value: String,
    ttl: Option<Duration>,
    stats: &mut CacheStats,
) -> Result<(), CacheError> {
    let expiration = ttl.or(config.default_ttl)
        .map(|duration| Instant::now() + duration);

    if let Some(exp) = expiration {
        expiration_queue.push(ExpirationEntry {
            expiration: exp,
            key: key.clone(),
        });
    }

    if let Some(max_size) = config.max_size {
        if data.len() >= max_size.get() && !data.contains_key(&key) {
            evict_entry(data, stats)?;
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

fn handle_get(
    data: &mut HashMap<String, CacheEntry>,
    key: &str,
    stats: &mut CacheStats,
) -> Result<Option<String>, CacheError> {
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

fn handle_update_ttl(
    data: &mut HashMap<String, CacheEntry>,
    expiration_queue: &mut BinaryHeap<ExpirationEntry>,
    key: &str,
    ttl: Duration,
) -> Result<bool, CacheError> {
    if let Some(entry) = data.get_mut(key) {
        let new_expiration = Instant::now() + ttl;
        entry.expiration = Some(new_expiration);
        
        expiration_queue.push(ExpirationEntry {
            expiration: new_expiration,
            key: key.to_string(),
        });
        
        Ok(true)
    } else {
        Ok(false)
    }
}

fn handle_cas(
    data: &mut HashMap<String, CacheEntry>,
    key: &str,
    expected: &str,
    new_value: String,
) -> Result<bool, CacheError> {
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

fn cleanup_expired(
    data: &mut HashMap<String, CacheEntry>,
    expiration_queue: &mut BinaryHeap<ExpirationEntry>,
    stats: &mut CacheStats,
) {
    let now = Instant::now();
    let mut expired_keys = Vec::new();

    while let Some(entry) = expiration_queue.peek() {
        if entry.expiration > now {
            break;
        }
        
        expired_keys.push(entry.key.clone());
        expiration_queue.pop();
    }

    for key in expired_keys {
        if data.remove(&key).is_some() {
            stats.evictions += 1;
        }
    }
}

fn evict_entry(
    data: &mut HashMap<String, CacheEntry>,
    stats: &mut CacheStats,
) -> Result<(), CacheError> {
    if let Some((key_to_remove, _)) = data.iter()
        .min_by_key(|(_, entry)| (entry.last_accessed, entry.access_count)) {
        let key_to_remove = key_to_remove.clone();
        data.remove(&key_to_remove);
        stats.evictions += 1;
    }
    Ok(())
}