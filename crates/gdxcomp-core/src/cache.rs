//! Bounded LRU record cache shared across clones of a `LoadedFile`.
//!
//! The GDX C library is not thread-safe and is serialized through a global
//! `Mutex` in `crates/gdx/src/lock.rs`. Reading the same symbol twice from
//! the same file therefore costs two `open`/`read` round-trips through the
//! lock — wasteful, since the file is immutable for the app's lifetime.
//!
//! This cache stores at most `capacity` `(symbol → Arc<Vec<Rec>>)` entries
//! per `LoadedFile`. Capacity is read from `GDXCOMP_RECORD_CACHE_SIZE` once
//! at construction; default 32. Sharing the cache across `LoadedFile`
//! clones is achieved via `Arc<RecordCache>` (see `LoadedFile::Clone`).
//!
//! Why not `RwLock`? Reads bump the LRU entry's freshness — that needs a
//! write lock. And the underlying FFI is serialized anyway, so concurrent
//! readers wouldn't gain anything.

use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};

use lru::LruCache;

use crate::error::Result;
use crate::model::Rec;

const DEFAULT_CAPACITY: usize = 32;

/// Per-file LRU record cache. Key = symbol name. Value = `Arc<Vec<Rec>>`.
#[derive(Debug)]
pub(crate) struct RecordCache {
    inner: Mutex<LruCache<String, Arc<Vec<Rec>>>>,
}

impl RecordCache {
    pub(crate) fn new() -> Self {
        let cap = std::env::var("GDXCOMP_RECORD_CACHE_SIZE")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(DEFAULT_CAPACITY)
            .max(1);
        Self {
            inner: Mutex::new(LruCache::new(NonZeroUsize::new(cap).unwrap())),
        }
    }

    /// Return cached records or fill via `read`. `read` is only called on
    /// miss; subsequent gets for the same symbol return the shared `Arc`.
    pub(crate) fn get_or_insert<F>(&self, symbol: &str, read: F) -> Result<Arc<Vec<Rec>>>
    where
        F: FnOnce() -> Result<Vec<Rec>>,
    {
        if let Some(records) = self.inner.lock().unwrap().get(symbol).cloned() {
            return Ok(records);
        }
        let records = Arc::new(read()?);
        self.inner
            .lock()
            .unwrap()
            .put(symbol.to_string(), Arc::clone(&records));
        Ok(records)
    }
}

impl Default for RecordCache {
    fn default() -> Self {
        Self::new()
    }
}
