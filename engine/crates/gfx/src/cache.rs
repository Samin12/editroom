//! Small thread-safe LRU caches. Values are built outside the lock, so builders may use other
//! caches; two threads racing on the same key both build the same (deterministic) value.

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Arc, Mutex};

pub struct Cache<K, V> {
    inner: Mutex<Lru<K, V>>,
}

struct Lru<K, V> {
    map: HashMap<K, (Arc<V>, u64)>,
    tick: u64,
    cap: usize,
}

impl<K: Hash + Eq + Clone, V> Cache<K, V> {
    pub fn new(cap: usize) -> Self {
        Cache { inner: Mutex::new(Lru { map: HashMap::new(), tick: 0, cap: cap.max(1) }) }
    }

    pub fn get(&self, k: &K) -> Option<Arc<V>> {
        let mut g = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        g.tick += 1;
        let tick = g.tick;
        g.map.get_mut(k).map(|e| {
            e.1 = tick;
            e.0.clone()
        })
    }

    pub fn get_or(&self, k: K, build: impl FnOnce() -> V) -> Arc<V> {
        if let Some(v) = self.get(&k) {
            return v;
        }
        let v = Arc::new(build());
        let mut g = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        g.tick += 1;
        let tick = g.tick;
        if g.map.len() >= g.cap {
            // Evict the oldest eighth in one go so the scan stays rare.
            let mut ages: Vec<u64> = g.map.values().map(|e| e.1).collect();
            ages.sort_unstable();
            let cut = ages[(ages.len() / 8).min(ages.len() - 1)];
            g.map.retain(|_, e| e.1 > cut);
        }
        g.map.insert(k, (v.clone(), tick));
        v
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.inner.lock().unwrap().map.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evicts_oldest() {
        let c: Cache<u32, u32> = Cache::new(8);
        for i in 0..8 {
            c.get_or(i, || i * 10);
        }
        c.get(&0);
        c.get_or(100, || 1);
        assert!(c.len() <= 8);
        assert!(c.get(&0).is_some(), "recently used entry survives");
        assert_eq!(*c.get_or(100, || 2), 1);
    }
}
