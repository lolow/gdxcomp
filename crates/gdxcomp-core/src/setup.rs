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

/// How to combine values that fall into the same (x, series) cell, i.e. across
/// dimensions that are neither the x-axis nor the series.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Aggregation {
    #[default]
    Sum,
    Mean,
    Min,
    Max,
    Count,
}

impl Aggregation {
    /// Aggregate finite values. `NaN` inputs (Undefined/NA) are ignored; an
    /// empty input yields `NaN`.
    pub fn apply(self, values: &[f64]) -> f64 {
        let finite: Vec<f64> = values.iter().copied().filter(|v| !v.is_nan()).collect();
        if matches!(self, Aggregation::Count) {
            return finite.len() as f64;
        }
        if finite.is_empty() {
            return f64::NAN;
        }
        match self {
            Aggregation::Sum => finite.iter().sum(),
            Aggregation::Mean => finite.iter().sum::<f64>() / finite.len() as f64,
            Aggregation::Min => finite.iter().copied().fold(f64::INFINITY, f64::min),
            Aggregation::Max => finite.iter().copied().fold(f64::NEG_INFINITY, f64::max),
            Aggregation::Count => unreachable!(),
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
    /// How to combine values over unmapped dimensions.
    #[serde(default)]
    pub aggregate: Aggregation,
    #[serde(default)]
    pub chart: ChartKind,
}

impl DisplaySetup {
    /// A minimal setup for a symbol: x = dimension 0, no series, sum-aggregate.
    pub fn for_symbol(symbol: impl Into<String>) -> Self {
        DisplaySetup {
            files: Vec::new(),
            symbol: symbol.into(),
            field: Field::Level,
            x_dim: 0,
            series_dim: None,
            filters: BTreeMap::new(),
            aggregate: Aggregation::Sum,
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
