use serde::Serialize;

use crate::error::{CoreError, Result};
use crate::model::{LoadedFile, Rec, SymbolKind, SymbolMeta};
use crate::setup::{ChartKind, DisplaySetup, Field};

/// Hard limit on the number of Plotly traces returned in a single view.
///
/// With `refine_setup` auto-picking the first series value this is rarely hit,
/// but it catches cases where the user has manually set many filter values across
/// many files. The frontend surfaces the error so the user can add filters.
const MAX_TRACES: usize = 30;

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

/// Fill in sensible defaults for a setup that has no filter on the series dimension.
///
/// When `series_dim` is set but carries no filter, this picks the first distinct
/// UEL value from the series dimension so the initial plot shows one series per
/// file rather than potentially hundreds. The caller (Tauri command) applies this
/// before `build_view` and returns the effective setup to the UI so the filter
/// panel reflects the auto-selection.
pub fn refine_setup(files: &[LoadedFile], setup: &DisplaySetup) -> Result<DisplaySetup> {
    let Some(sd) = setup.series_dim else {
        return Ok(setup.clone());
    };

    // Respect an existing filter.
    if setup.filters.get(&sd).is_some_and(|f| !f.is_empty()) {
        return Ok(setup.clone());
    }

    // Read distinct series values from the first file that has the symbol.
    let keys = files
        .iter()
        .find(|f| f.symbol(&setup.symbol).is_some())
        .map(|f| f.distinct_keys(&setup.symbol, sd))
        .transpose()?
        .unwrap_or_default();

    // Only one value anyway; the filter adds nothing.
    if keys.len() <= 1 {
        return Ok(setup.clone());
    }

    let mut refined = setup.clone();
    refined
        .filters
        .insert(sd, vec![keys.into_iter().next().unwrap()]);
    Ok(refined)
}

/// Build the chart + table for `setup` across the given files.
///
/// Records are read from disk lazily here — only the symbol named in `setup` is
/// read. Call [`refine_setup`] first to ensure the series dimension has a
/// reasonable default filter; without it this will likely hit [`CoreError::TooManyTraces`].
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

    let mut groups: Vec<SeriesGroup> = Vec::new();
    let mut table = Vec::new();

    for (fi, file) in files.iter().enumerate() {
        if file.symbol(&setup.symbol).is_none() {
            continue;
        }
        let records = file.read_records(&setup.symbol)?;
        for rec in &records {
            if !passes_filters(rec, setup) {
                continue;
            }
            let value = rec.values[setup.field.index()];
            let x = x_key(rec, setup.x_dim);
            let series = setup
                .series_dim
                .map(|sd| rec.keys.get(sd).cloned().unwrap_or_default());

            group_for(&mut groups, fi, &series).push(x, value);

            table.push(TableRow {
                file: file.label.clone(),
                keys: rec.keys.clone(),
                value,
            });
        }
    }

    let traces: Vec<Trace> = groups
        .into_iter()
        .map(|g| Trace {
            name: trace_name(&files[g.file_index].label, g.series.as_deref(), multi_file),
            x: g.x,
            y: g.y,
        })
        .collect();

    if traces.len() > MAX_TRACES {
        return Err(CoreError::TooManyTraces {
            traces: traces.len(),
            max: MAX_TRACES,
        });
    }

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

struct SeriesGroup {
    file_index: usize,
    series: Option<String>,
    x: Vec<String>,
    y: Vec<f64>,
}

impl SeriesGroup {
    fn push(&mut self, x: String, value: f64) {
        self.x.push(x);
        self.y.push(value);
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
        x: Vec::new(),
        y: Vec::new(),
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
