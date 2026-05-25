//! Process-global serialization of GDX FFI calls.
//!
//! The vendored GDX library is built without internal mutexes (`-DGC_NO_MUTEX`)
//! and is not safe to call from multiple threads concurrently. Every public
//! operation in this crate holds this lock for the full duration of its FFI
//! work (including multi-call read/write sequences), so the wrapper is safe to
//! use from any thread — at the cost of serializing GDX access.

use std::sync::{Mutex, MutexGuard, OnceLock};

static GDX_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

/// Acquire the global GDX lock. Poisoning is ignored: a panic mid-operation
/// may leave a GDX object in an odd state, but the lock itself stays usable.
pub(crate) fn lock() -> MutexGuard<'static, ()> {
    GDX_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
