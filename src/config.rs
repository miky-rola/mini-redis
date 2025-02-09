use std::num::NonZeroUsize;
use std::time::Duration;


#[derive(Clone)]
pub struct CacheConfig {
    pub(crate) max_size: Option<NonZeroUsize>,
    pub(crate) default_ttl: Option<Duration>,
    pub(crate) cleanup_interval: Duration,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_size: None,
            default_ttl: None,
            cleanup_interval: Duration::from_secs(1),
        }
    }
}

impl CacheConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_max_size(mut self, size: NonZeroUsize) -> Self {
        self.max_size = Some(size);
        self
    }

    pub fn with_default_ttl(mut self, ttl: Duration) -> Self {
        self.default_ttl = Some(ttl);
        self
    }

    pub fn with_cleanup_interval(mut self, interval: Duration) -> Self {
        self.cleanup_interval = interval;
        self
    }
}