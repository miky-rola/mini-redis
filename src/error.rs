use std::fmt;

#[derive(Debug, Clone)]
pub enum CacheError {
    KeyNotFound,
    ValueNotInteger,
    KeyExpired,
    SerializationError(String),
    LockError,
}

impl fmt::Display for CacheError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CacheError::KeyNotFound => write!(f, "Key not found in the cache"),
            CacheError::ValueNotInteger => write!(f, "Value is not an integer"),
            CacheError::KeyExpired => write!(f, "Key has expired"),
            CacheError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            CacheError::LockError => write!(f, "Failed to acquire the lock"),
        }
    }
}

impl std::error::Error for CacheError {}