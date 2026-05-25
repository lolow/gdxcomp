use serde::Serialize;

use crate::error::{CoreError, Result};
use crate::model::{LoadedFile, Rec, SymbolKind, SymbolMeta};
use crate::setup::{Aggregation, ChartKind, DisplaySetup, Field};

/// One plotted series: a file (optionally split by a series dimension).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Trace {
    pub name: String,
    pub x: Vec<String>,
    /// Aligned with `x`; non-finite values serialize as JSON `null` (a gap).
    pub y: Vec<f64>,
}

/// One row of the underlying (filtered, pre-aggregation) data table.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TableRow {
    pub file: String,
    pub keys: Vec<String>,
    pub value: f64,
}

/// Everything the frontend needs to render the chart and table.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlotView {
    pub symbol: String,
    pub kind: SymbolKind,
    pub field: Field,
    pub chart: ChartKind,
    /// Axis label for x (the mapped dimension's domain name).
    pub x_label: String,
    pub series_label: Option<String>,
    pub traces: Vec<Trace>,
    /// Column names for the data table's key columns.
    pub dim_names: Vec<String>,
    pub table: Vec<TableRow>,
}

/// Symbols present, with the same dimension and kind, in *every* file.
/// The representative metadata comes from the first file. Sorted by name.
pub fn common_symbols(files: &[LoadedFile]) -> Vec<SymbolMeta> {
    let Some((first, rest)) = files.split_first() else {
        return Vec::new();
    };
    let mut shared: Vec<SymbolMeta> = first
        .symbols
        .iter()
        .filter(|s| {
            rest.iter().all(|f| {
                f.symbol(&s.name)
                    .is_some_and(|o| o.dim == s.dim && o.kind == s.kind)
            })
        })
        .cloned()
        .collect();
    shared.sort_by(|a, b| a.name.cmp(&b.name));
    shared
}

/// Build the chart + table for `setup` across the given files.
pub fn build_view(files: &[LoadedFile], setup: &DisplaySetup) -> Result<PlotView> {
    // Representative metadata from the first file that has the symbol.
    let meta = files
        .iter()
        .find_map(|f| f.symbol(&setup.symbol))
        .ok_or_else(|| CoreError::SymbolMissing(setup.symbol.clone()))?
        .clone();

    validate_dim(&meta, setup.x_dim)?;
    if let Some(sd) = setup.series_dim {
        validate_dim(&meta, sd)?;
    }

    let dim_names = dimension_names(&meta);
    let x_label = axis_label(&meta, &dim_names, setup.x_dim);
    let series_label = setup.series_dim.map(|sd| axis_label(&meta, &dim_names, sd));

    let multi_file = files
        .iter()
        .filter(|f| f.symbol(&setup.symbol).is_some())
        .count()
        > 1;

    let mut x_order: Vec<String> = Vec::new();
    let mut groups: Vec<SeriesGroup> = Vec::new();
    let mut table = Vec::new();

    for (fi, file) in files.iter().enumerate() {
        if file.symbol(&setup.symbol).is_none() {
            continue;
        }
        for rec in file.records(&setup.symbol) {
            if !passes_filters(rec, setup) {
                continue;
            }
            let value = rec.values[setup.field.index()];
            let x = x_key(rec, setup.x_dim);
            let series = setup
                .series_dim
                .map(|sd| rec.keys.get(sd).cloned().unwrap_or_default());

            if !x_order.contains(&x) {
                x_order.push(x.clone());
            }
            group_for(&mut groups, fi, &series).add(&x, value);

            table.push(TableRow {
                file: file.label.clone(),
                keys: rec.keys.clone(),
                value,
            });
        }
    }

    let traces = groups
        .into_iter()
        .map(|g| {
            let y = x_order
                .iter()
                .map(|x| g.aggregate(x, setup.aggregate))
                .collect();
            Trace {
                name: trace_name(&files[g.file_index].label, g.series.as_deref(), multi_file),
                x: x_order.clone(),
                y,
            }
        })
        .collect();

    Ok(PlotView {
        symbol: meta.name.clone(),
        kind: meta.kind,
        field: setup.field,
        chart: setup.chart,
        x_label,
        series_label,
        traces,
        dim_names,
        table,
    })
}

fn validate_dim(meta: &SymbolMeta, dim: usize) -> Result<()> {
    // Scalars (dim 0) accept x_dim 0 as a degenerate single-category axis.
    if meta.dim == 0 {
        return Ok(());
    }
    if dim >= meta.dim {
        return Err(CoreError::DimOutOfRange {
            symbol: meta.name.clone(),
            dim,
            ndim: meta.dim,
        });
    }
    Ok(())
}

fn dimension_names(meta: &SymbolMeta) -> Vec<String> {
    (0..meta.dim)
        .map(|i| match meta.domains.get(i) {
            Some(d) if d != "*" && !d.is_empty() => d.clone(),
            _ => format!("Dim{}", i + 1),
        })
        .collect()
}

fn axis_label(meta: &SymbolMeta, dim_names: &[String], dim: usize) -> String {
    if meta.dim == 0 {
        return "value".to_string();
    }
    dim_names
        .get(dim)
        .cloned()
        .unwrap_or_else(|| format!("Dim{}", dim + 1))
}

fn x_key(rec: &Rec, x_dim: usize) -> String {
    rec.keys
        .get(x_dim)
        .cloned()
        .unwrap_or_else(|| "value".to_string())
}

fn passes_filters(rec: &Rec, setup: &DisplaySetup) -> bool {
    setup.filters.iter().all(|(dim, allowed)| {
        if allowed.is_empty() {
            return true;
        }
        rec.keys.get(*dim).is_some_and(|k| allowed.contains(k))
    })
}

/// One plotted series' accumulator: the values seen at each x label, before
/// aggregation.
struct SeriesGroup {
    file_index: usize,
    series: Option<String>,
    cells: Vec<(String, Vec<f64>)>,
}

impl SeriesGroup {
    fn add(&mut self, x: &str, value: f64) {
        match self.cells.iter_mut().find(|(cx, _)| cx == x) {
            Some((_, vals)) => vals.push(value),
            None => self.cells.push((x.to_string(), vec![value])),
        }
    }

    fn aggregate(&self, x: &str, how: Aggregation) -> f64 {
        match self.cells.iter().find(|(cx, _)| cx == x) {
            Some((_, vals)) => how.apply(vals),
            None => f64::NAN,
        }
    }
}

/// Find or create the accumulator for `(file_index, series)`.
fn group_for<'a>(
    groups: &'a mut Vec<SeriesGroup>,
    file_index: usize,
    series: &Option<String>,
) -> &'a mut SeriesGroup {
    if let Some(pos) = groups
        .iter()
        .position(|g| g.file_index == file_index && &g.series == series)
    {
        return &mut groups[pos];
    }
    groups.push(SeriesGroup {
        file_index,
        series: series.clone(),
        cells: Vec::new(),
    });
    groups.last_mut().unwrap()
}

fn trace_name(file_label: &str, series: Option<&str>, multi_file: bool) -> String {
    match series {
        Some(s) if multi_file => format!("{file_label} / {s}"),
        Some(s) => s.to_string(),
        None => file_label.to_string(),
    }
}
