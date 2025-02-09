
/// created this to represents cache statistics for statistics tracking.

#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    pub(crate) hits: u64,
    pub(crate) misses: u64,
    pub(crate) evictions: u64,
}

impl CacheStats {
    
    pub fn hits(&self) -> u64 {
        self.hits /// this here returns the number of cache hits
    }

    pub fn misses(&self) -> u64 {
        self.misses /// this here returns the number of cache misses
    }

    pub fn evictions(&self) -> u64 {
        self.evictions /// this here returns the number of evicted entries
    }

    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            (self.hits as f64 / total as f64) * 100.0 /// this here returns the hit rate as a percentage
        }
    }
}