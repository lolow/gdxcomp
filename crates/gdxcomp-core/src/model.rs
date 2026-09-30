use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use gdx::{GdxFile, SymbolType};
use serde::{Deserialize, Serialize};

use crate::cache::RecordCache;
use crate::error::Result;

/// Serializable mirror of [`gdx::SymbolType`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SymbolKind {
    Set,
    Parameter,
    Variable,
    Equation,
    Alias,
}

impl From<SymbolType> for SymbolKind {
    fn from(t: SymbolType) -> Self {
        match t {
            SymbolType::Set => SymbolKind::Set,
            SymbolType::Parameter => SymbolKind::Parameter,
            SymbolType::Variable => SymbolKind::Variable,
            SymbolType::Equation => SymbolKind::Equation,
            SymbolType::Alias => SymbolKind::Alias,
        }
    }
}

impl SymbolKind {
    /// Whether records carry the five value fields (Variables/Equations).
    pub fn has_fields(self) -> bool {
        matches!(self, SymbolKind::Variable | SymbolKind::Equation)
    }
}

/// Metadata for a symbol, as surfaced to the UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SymbolMeta {
    pub name: String,
    pub dim: usize,
    pub kind: SymbolKind,
    pub records: usize,
    pub text: String,
    pub domains: Vec<String>,
}

/// One data record: `keys.len() == dim` interned labels plus the five value fields.
#[derive(Debug, Clone, PartialEq)]
pub struct Rec {
    pub keys: Vec<Arc<str>>,
    pub values: [f64; 5],
}

/// A GDX file with only symbol metadata loaded.
///
/// Record data is read from disk on demand via [`read_records`](LoadedFile::read_records).
/// Because the GDX library opens and closes fast, we reopen for each read rather
/// than keeping an open handle (handles are not `Send`).
/// Clones share the same path/symbols metadata and the same record cache.
/// The `Arc<RecordCache>` is the reason `Clone` is hand-written rather than
/// derived: cloning a `LoadedFile` must reuse the cache so the second copy
/// benefits from reads the first one warmed.
#[derive(Debug)]
pub struct LoadedFile {
    pub label: String,
    pub path: PathBuf,
    pub symbols: Vec<SymbolMeta>,
    /// Name → index into `symbols`. Built at open time for O(1) lookup.
    name_index: HashMap<String, usize>,
    /// Shared bounded LRU. Sized via `GDXCOMP_RECORD_CACHE_SIZE` (default 32).
    cache: Arc<RecordCache>,
}

impl Clone for LoadedFile {
    fn clone(&self) -> Self {
        Self {
            label: self.label.clone(),
            path: self.path.clone(),
            symbols: self.symbols.clone(),
            name_index: self.name_index.clone(),
            cache: Arc::clone(&self.cache),
        }
    }
}

impl LoadedFile {
    /// Open `path` and load symbol metadata only. No record data is read.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let label = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string_lossy().into_owned());

        let file = GdxFile::open(&path)?;

        let symbols: Vec<SymbolMeta> = file
            .symbols()
            .iter()
            .map(|info| SymbolMeta {
                name: info.name.clone(),
                dim: info.dim,
                kind: info.kind.into(),
                records: info.records,
                text: info.text.clone(),
                domains: info.domains.clone(),
            })
            .collect();

        let name_index = symbols
            .iter()
            .enumerate()
            .map(|(i, s)| (s.name.clone(), i))
            .collect();

        Ok(LoadedFile {
            label,
            path,
            symbols,
            name_index,
            cache: Arc::new(RecordCache::new()),
        })
    }

    pub fn symbol(&self, name: &str) -> Option<&SymbolMeta> {
        self.name_index.get(name).map(|&i| &self.symbols[i])
    }

    /// Read all records for `symbol`, going through the per-file LRU cache.
    ///
    /// Hits a warm cache without re-opening the GDX file. Misses fall through
    /// to a fresh FFI read; the resulting `Arc<Vec<Rec>>` is stored for reuse.
    pub fn read_records_arc(&self, symbol: &str) -> Result<Arc<Vec<Rec>>> {
        self.cache.get_or_insert(symbol, || self.read_records_uncached(symbol))
    }

    /// Read all records for `symbol`. Convenience wrapper that clones from
    /// the cached `Arc<Vec<Rec>>`; prefer [`read_records_arc`] when the
    /// caller can take a borrowed/shared view.
    pub fn read_records(&self, symbol: &str) -> Result<Vec<Rec>> {
        self.read_records_arc(symbol).map(|arc| (*arc).clone())
    }

    fn read_records_uncached(&self, symbol: &str) -> Result<Vec<Rec>> {
        let file = GdxFile::open(&self.path)?;
        match file.symbol(symbol) {
            Some(info) => {
                let records = file.read_info(info)?;
                Ok(records
                    .into_iter()
                    .map(|r| Rec {
                        keys: r.keys,
                        values: r.values,
                    })
                    .collect())
            }
            None => Ok(Vec::new()),
        }
    }

    /// Distinct UEL labels appearing in dimension `dim` of `symbol`, in first-seen order.
    ///
    /// Uses a HashSet for O(1) membership against the borrowed key strings,
    /// keeping a parallel Vec for stable first-seen ordering. Borrows directly
    /// from the cached `Arc<Vec<Rec>>` so cache hits incur no record clone.
    pub fn distinct_keys(&self, symbol: &str, dim: usize) -> Result<Vec<String>> {
        let records = self.read_records_arc(symbol)?;
        let mut seen: HashSet<&str> = HashSet::new();
        let mut order: Vec<String> = Vec::new();
        for rec in records.iter() {
            if let Some(k) = rec.keys.get(dim) {
                if seen.insert(k.as_ref()) {
                    order.push(k.to_string());
                }
            }
        }
        Ok(order)
    }
}
