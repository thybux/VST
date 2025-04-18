use std::collections::HashMap;
use std::fmt;
use std::hash::Hash;
use std::sync::{Arc, RwLock};

pub struct ThreadSafeMap<K, V> {
    data: Arc<RwLock<HashMap<K, V>>>,
}

impl<K, V> ThreadSafeMap<K, V> {
    pub fn new() -> Self {
        ThreadSafeMap {
            data: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn insert(&self, key: K, value: V) -> Result<Option<V>, String>
    where
        K: Eq + Hash,
    {
        match self.data.write() {
            Ok(mut guard) => Ok(guard.insert(key, value)),
            Err(_) => Err("Lock poisoned".to_string()),
        }
    }

    pub fn get(&self, key: &K) -> Result<Option<V>, String>
    where
        K: Eq + Hash,
        V: Clone,
    {
        match self.data.read() {
            Ok(guard) => Ok(guard.get(key).cloned()),
            Err(_) => Err("Lock poisoned".to_string()),
        }
    }
}

impl<K: fmt::Debug, V: fmt::Debug> fmt::Display for ThreadSafeMap<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.data.read() {
            Ok(guard) => {
                let max_len = guard
                    .keys()
                    .map(|k| format!("{:?}", k).len())
                    .max()
                    .unwrap_or(0);

                for (key, value) in guard.iter() {
                    writeln!(f, "{:max_len$}: {:?}", format!("{:?}", key), value)?;
                }
                Ok(())
            }
            Err(_) => write!(f, "ThreadSafeMap {{ <poisoned> }}"),
        }
    }
}

impl<K, V> Clone for ThreadSafeMap<K, V> {
    fn clone(&self) -> Self {
        ThreadSafeMap {
            data: Arc::clone(&self.data),
        }
    }
}
