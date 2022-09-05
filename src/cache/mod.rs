use std::time::{Duration, Instant};
use dashmap::DashMap;


pub trait Cache<K, V> where
    K: std::hash::Hash + Eq + Clone,
    V: Clone,
{
    fn get(&self, key: &K) -> Option<V>;
    fn set(&self, key: K, value: V, ttl: Option<Duration>);
    fn del(&self, key: &K);
    fn clear(&self);
}

#[derive(Clone)]
struct InternalEntry<V> {
    value: V,
    expiration: Instant,
}

impl<V> InternalEntry<V> {
    fn new(v: V, duration: Duration) -> Self {
        InternalEntry {
            value: v,
            expiration: Instant::now() + duration,
        }
    }

    fn is_expired(&self) -> bool {
        Instant::now() > self.expiration
    }
}

#[derive(Clone, Debug, Default)]
pub struct LocalCache<K, V> {
    pub cache: DashMap<K, V>,
}

impl<K, V> LocalCache<K, V> where
    K: std::hash::Hash + Eq + Clone,
    V: Clone,
{
    pub fn new() -> Self {
        LocalCache {
            cache: DashMap::new(),
        }
    }
}

impl<K, V> Cache<K, V> for LocalCache<K, V> where
    K: std::hash::Hash + Eq + Clone,
    V: Clone,
{
    fn get(&self, key: &K) -> Option<V> {
        self.cache.get(key).map(|v| v.value().clone())
    }

    fn set(&self, key: K, value: V, ttl: Option<Duration>) {
        if let Some(ttl) = ttl {
            self.cache.insert(key, InternalEntry::new(value, ttl));
        } else {
            self.cache.insert(key, value);
        }
    }

    fn del(&self, key: &K) {
        self.cache.remove(key);
    }

    fn clear(&self) {
        self.cache.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_cache() {
        let cache = LocalCache::new();
        cache.set("key", "value", None);
        assert_eq!(cache.get(&"key"), Some("value".to_string()));
        cache.set("key", "value", Some(Duration::from_secs(1)));
        assert_eq!(cache.get(&"key"), Some("value".to_string()));
    }
}