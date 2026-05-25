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


#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ChartKind {
    #[default]
    Line,
    Bar,
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
    /// Symbol to plot.
    pub symbol: String,
    /// Value field (Variables/Equations); ignored for Parameters/Sets.
    #[serde(default)]
    pub field: Field,
    /// Dimension index mapped to the x-axis.
    #[serde(default)]
    pub x_dim: usize,
    /// Optional dimension index mapped to series (within each file).
    #[serde(default)]
    pub series_dim: Option<usize>,
    /// Per-dimension allow-lists of UELs. A missing/empty entry means "all".
    #[serde(default)]
    pub filters: BTreeMap<usize, Vec<String>>,
    #[serde(default)]
    pub chart: ChartKind,
}

impl DisplaySetup {
    pub fn for_symbol(symbol: impl Into<String>) -> Self {
        DisplaySetup {
            files: Vec::new(),
            symbol: symbol.into(),
            field: Field::Level,
            x_dim: 0,
            series_dim: None,
            filters: BTreeMap::new(),
            chart: ChartKind::Line,
        }
    }

    /// Serialize to pretty JSON.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Parse from JSON.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}
