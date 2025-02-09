use std::time::Instant;
use std::cmp::Ordering;

#[derive(Eq, PartialEq)]
pub(crate) struct ExpirationEntry {
    pub expiration: Instant,
    pub key: String,
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

pub(crate) struct CacheEntry {
    pub value: String,
    pub expiration: Option<Instant>,
    pub last_accessed: Instant,
    pub access_count: u64,
}