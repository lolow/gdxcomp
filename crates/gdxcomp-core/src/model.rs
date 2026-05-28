use std::collections::HashSet;
use std::path::{Path, PathBuf};

use gdx::{GdxFile, SymbolType};
use serde::{Deserialize, Serialize};

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

/// One data record: `keys.len() == dim` indices plus the five value fields.
#[derive(Debug, Clone, PartialEq)]
pub struct Rec {
    pub keys: Vec<String>,
    pub values: [f64; 5],
}

/// A GDX file with only symbol metadata loaded.
///
/// Record data is read from disk on demand via [`read_records`](LoadedFile::read_records).
/// Because the GDX library opens and closes fast, we reopen for each read rather
/// than keeping an open handle (handles are not `Send`).
#[derive(Debug, Clone)]
pub struct LoadedFile {
    pub label: String,
    pub path: PathBuf,
    pub symbols: Vec<SymbolMeta>,
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

        let symbols = file
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

        Ok(LoadedFile {
            label,
            path,
            symbols,
        })
    }

    pub fn symbol(&self, name: &str) -> Option<&SymbolMeta> {
        self.symbols.iter().find(|s| s.name == name)
    }

    /// Read all records for `symbol` from disk.
    ///
    /// Opens and closes the GDX file on every call. This is cheap because the
    /// library is fast and the symbol table is cached in the file header.
    pub fn read_records(&self, symbol: &str) -> Result<Vec<Rec>> {
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
    /// keeping a parallel Vec for stable first-seen ordering.
    pub fn distinct_keys(&self, symbol: &str, dim: usize) -> Result<Vec<String>> {
        let records = self.read_records(symbol)?;
        let mut seen: HashSet<&str> = HashSet::new();
        let mut order: Vec<String> = Vec::new();
        for rec in &records {
            if let Some(k) = rec.keys.get(dim) {
                if seen.insert(k.as_str()) {
                    order.push(k.clone());
                }
            }
        }
        Ok(order)
    }
}
