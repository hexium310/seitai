use std::{collections::HashMap, hash::Hash, time::{Duration, Instant}};

#[derive(Debug, Default)]
pub struct TimeKeeper<K: Eq + Hash> {
    inner: HashMap<K, Instant>,
}

impl<K: Eq + Hash> TimeKeeper<K> {
    pub fn new() -> Self {
        Self { inner: HashMap::new() }
    }

    pub fn is_elapsed(&self, key: &K, interval: Duration) -> bool {
        self.inner.get(key).map(|v| v.elapsed() < interval).unwrap_or(false)
    }

    pub fn record(&mut self, key: K) {
        let now = Instant::now();
        *self.inner.entry(key).or_insert(now) = now;
    }
}
