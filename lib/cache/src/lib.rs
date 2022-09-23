use std::collections::HashMap;
use std::fmt::{Debug, Display};
use std::time::{Duration, Instant};

use dashmap::DashMap;

pub trait Cache<V>
    where
        V: Clone + Sized + Display + Debug + PartialEq,
{
    fn get(&self, key: &str) -> Option<Entry<V>>;
    fn set(&self, key: String, value: Entry<V>, ttl: Option<Duration>);
    fn del(&self, key: &str);
    fn clear(&self);
}

#[derive(Debug, Clone, PartialEq)]
pub enum Entry<V: Clone + Sized + Display + Debug + PartialEq> {
    Value(V),
    List(Vec<V>),
    Map(HashMap<String, V>),
}

#[derive(Clone, Debug)]
struct InternalEntry<V: Clone + Sized + Display + Debug + PartialEq> {
    value: Entry<V>,
    expiration: Option<Instant>,
}

impl<V> InternalEntry<V> where V: Clone + Sized + Display + Debug + PartialEq {
    fn new(v: Entry<V>, duration: Option<Duration>) -> Self {
        InternalEntry {
            value: v,
            expiration: duration.map(|x|Instant::now() + x),
        }
    }

    fn is_expired(&self) -> bool {
         self.expiration.map(|x| Instant::now() > x).unwrap_or(false)
    }
}

#[derive(Clone, Debug, Default)]
pub struct LocalCache<V: Clone + Sized + Display + Debug + PartialEq> {
    cache: DashMap<String, InternalEntry<V>>,
}

impl<V> LocalCache<V>
    where
        V: Clone + Sized + Display + Debug + PartialEq,
{
    pub fn new() -> Self {
        LocalCache {
            cache: DashMap::new(),
        }
    }
}

impl<V> Cache<V> for LocalCache<V>
    where
        V: Clone + Sized + Display + Debug + PartialEq,
{
    fn get(&self, key: &str) -> Option<Entry<V>> {
        let value  = self.cache.get(key);
        if value.is_none() {
            return None;
        }
        let entry = value.unwrap().clone();
        if entry.is_expired() {
            self.del(key);
            None
        } else {
            Some(entry.value)
        }
    }

    fn set(&self, key: String, value: Entry<V>, ttl: Option<Duration>) {
        self.cache.insert(key, InternalEntry::new(value, ttl));
    }

    fn del(&self, key: &str) {
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
    fn test_local_cache_none_ttl() {
        let cache = LocalCache::new();
        cache.set("key".to_string(), Entry::Value("value".to_string()), None);
        std::thread::sleep(Duration::from_secs(1));
        assert_eq!(cache.get("key"), Some(Entry::Value("value".to_string())));
    }

    #[test]
    fn test_local_cache_ttl() {
        let cache = LocalCache::new();
        cache.set("key".to_string(), Entry::Value("value".to_string()), Some(Duration::from_secs(1)));
        std::thread::sleep(Duration::from_secs(1));
        assert_ne!(cache.get("key"), Some(Entry::Value("value".to_string())));
    }

    #[test]
    fn test_local_cache() {
        let cache = LocalCache::new();
        cache.set("key".to_string(), Entry::Value("value".to_string()), Some(Duration::from_secs(10)));
        assert_eq!(cache.get("key"), Some(Entry::Value("value".to_string())));
    }
}

