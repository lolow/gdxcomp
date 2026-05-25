use std::collections::HashMap;
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

/// A GDX file loaded fully into memory: an owned, thread-safe snapshot.
///
/// All symbols' records are read eagerly on [`open`](LoadedFile::open), so the
/// underlying GDX handle is closed before this value is returned. Very large
/// files are therefore held entirely in memory.
#[derive(Debug, Clone)]
pub struct LoadedFile {
    pub label: String,
    pub path: PathBuf,
    pub symbols: Vec<SymbolMeta>,
    data: HashMap<String, Vec<Rec>>,
}

impl LoadedFile {
    /// Open `path` and read every symbol's records into memory.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let label = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string_lossy().into_owned());

        let file = GdxFile::open(&path)?;

        let mut symbols = Vec::with_capacity(file.symbols().len());
        let mut data = HashMap::new();
        for info in file.symbols() {
            let records = file.read_info(info)?;
            data.insert(
                info.name.clone(),
                records
                    .into_iter()
                    .map(|r| Rec {
                        keys: r.keys,
                        values: r.values,
                    })
                    .collect(),
            );
            symbols.push(SymbolMeta {
                name: info.name.clone(),
                dim: info.dim,
                kind: info.kind.into(),
                records: info.records,
                text: info.text.clone(),
                domains: info.domains.clone(),
            });
        }

        Ok(LoadedFile {
            label,
            path,
            symbols,
            data,
        })
    }

    pub fn symbol(&self, name: &str) -> Option<&SymbolMeta> {
        self.symbols.iter().find(|s| s.name == name)
    }

    /// Records for a symbol, or an empty slice if the symbol is absent.
    pub fn records(&self, name: &str) -> &[Rec] {
        self.data.get(name).map(Vec::as_slice).unwrap_or(&[])
    }

    /// Distinct UEL labels appearing in dimension `dim` of `symbol`, in first-seen order.
    pub fn distinct_keys(&self, symbol: &str, dim: usize) -> Vec<String> {
        let mut seen = Vec::new();
        for rec in self.records(symbol) {
            if let Some(k) = rec.keys.get(dim) {
                if !seen.contains(k) {
                    seen.push(k.clone());
                }
            }
        }
        seen
    }
}
