use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Which value field to plot. Meaningful for Variables/Equations; for
/// Parameters and Sets only [`Field::Level`] carries data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Field {
    #[default]
    Level,
    Marginal,
    Lower,
    Upper,
    Scale,
}

impl Field {
    /// Index into a record's five-element value array.
    pub fn index(self) -> usize {
        match self {
            Field::Level => 0,
            Field::Marginal => 1,
            Field::Lower => 2,
            Field::Upper => 3,
            Field::Scale => 4,
        }
    }
}

/// How to combine records that share the same (x, series) cell across unmapped
/// dimensions.  Set per dimension in [`DisplaySetup::dim_agg`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DimAgg {
    Sum,
    Mean,
}

impl DimAgg {
    pub fn apply(self, values: &[f64]) -> f64 {
        let finite: Vec<f64> = values.iter().copied().filter(|v| v.is_finite()).collect();
        if finite.is_empty() {
            return f64::NAN;
        }
        match self {
            DimAgg::Sum => finite.iter().sum(),
            DimAgg::Mean => finite.iter().sum::<f64>() / finite.len() as f64,
        }
    }
}

/// Application mode: controls WITCH-specific behaviour like year mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum AppMode {
    #[default]
    Gdx,
    Witch,
}

/// A complete, serializable description of what to plot and how.
///
/// This is the unit of JSON import/export: saving it and re-importing it
/// reproduces the same view (given the same files).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplaySetup {
    /// Files this setup was built against (for reproducing the selection).
    #[serde(default)]
    pub files: Vec<PathBuf>,
    pub symbol: String,
    #[serde(default)]
    pub field: Field,
    #[serde(default)]
    pub x_dim: usize,
    /// x-axis multi-select and non-x single-value filters.
    #[serde(default)]
    pub filters: BTreeMap<usize, Vec<String>>,
    /// Per-dimension aggregation for dims not filtered to a specific value.
    #[serde(default)]
    pub dim_agg: BTreeMap<usize, DimAgg>,
    #[serde(default)]
    pub mode: AppMode,
}

impl DisplaySetup {
    pub fn for_symbol(symbol: impl Into<String>) -> Self {
        DisplaySetup {
            files: Vec::new(),
            symbol: symbol.into(),
            field: Field::Level,
            x_dim: 0,
            filters: BTreeMap::new(),
            dim_agg: BTreeMap::new(),
            mode: AppMode::Gdx,
        }
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}
