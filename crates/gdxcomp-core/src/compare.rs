use serde::Serialize;

use crate::error::{CoreError, Result};
use crate::model::{LoadedFile, Rec, SymbolKind, SymbolMeta};
use crate::setup::{ChartKind, DimAgg, DisplaySetup, Field};

const MAX_TRACES: usize = 30;

/// One plotted series — one per file.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Trace {
    pub name: String,
    pub x: Vec<String>,
    /// Non-finite values serialize as JSON `null` (a gap in the line).
    pub y: Vec<f64>,
}

/// One row of the underlying filtered data table.
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
    pub x_label: String,
    pub traces: Vec<Trace>,
    pub dim_names: Vec<String>,
    pub table: Vec<TableRow>,
}

/// Symbols present with the same dimension and kind in *every* file. Sorted by name.
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

/// Fill in sensible defaults before `build_view`:
/// - x-axis limited to first 5 UELs if unfiltered
/// - uninitialized non-x dims defaulted to sum aggregation
pub fn refine_setup(files: &[LoadedFile], setup: &DisplaySetup) -> Result<DisplaySetup> {
    let mut refined = setup.clone();
    let first_file = files.iter().find(|f| f.symbol(&setup.symbol).is_some());
    let meta = first_file.and_then(|f| f.symbol(&setup.symbol));

    // Limit x-axis to first 5 UELs when no filter is set yet.
    if setup.filters.get(&setup.x_dim).is_none_or(|f| f.is_empty()) {
        let keys = first_file
            .map(|f| f.distinct_keys(&setup.symbol, setup.x_dim))
            .transpose()?
            .unwrap_or_default();
        if keys.len() > 5 {
            refined
                .filters
                .insert(setup.x_dim, keys.into_iter().take(5).collect());
        }
    }

    // Default uninitialized non-x dims to sum aggregation.
    if let Some(m) = meta {
        for d in 0..m.dim {
            if d == setup.x_dim {
                continue;
            }
            if setup.filters.get(&d).is_some_and(|f| !f.is_empty()) {
                continue;
            }
            if setup.dim_agg.contains_key(&d) {
                continue;
            }
            refined.dim_agg.insert(d, DimAgg::Sum);
        }
    }

    Ok(refined)
}

/// Build the chart + table for `setup` across the given files.
/// Each file produces exactly one trace. Call [`refine_setup`] first.
pub fn build_view(files: &[LoadedFile], setup: &DisplaySetup) -> Result<PlotView> {
    let meta = files
        .iter()
        .find_map(|f| f.symbol(&setup.symbol))
        .ok_or_else(|| CoreError::SymbolMissing(setup.symbol.clone()))?
        .clone();

    validate_dim(&meta, setup.x_dim)?;

    let dim_names = dimension_names(&meta);
    let x_label = axis_label(&meta, &dim_names, setup.x_dim);

    let needs_agg = !setup.dim_agg.is_empty();
    let agg_method = setup.dim_agg.values().copied().next().unwrap_or(DimAgg::Sum);

    let mut x_order: Vec<String> = Vec::new();
    let mut groups: Vec<FileGroup> = Vec::new();
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

            if needs_agg && !x_order.contains(&x) {
                x_order.push(x.clone());
            }
            group_for(&mut groups, fi).push(x, value);

            table.push(TableRow {
                file: file.label.clone(),
                keys: rec.keys.clone(),
                value,
            });
        }
    }

    let traces: Vec<Trace> = groups
        .into_iter()
        .map(|g| {
            let name = files[g.file_index].label.clone();
            let (x, y) = if needs_agg {
                let y = x_order.iter().map(|x| g.aggregate(x, agg_method)).collect();
                (x_order.clone(), y)
            } else {
                g.into_pairs()
            };
            Trace { name, x, y }
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
        traces,
        dim_names,
        table,
    })
}

fn validate_dim(meta: &SymbolMeta, dim: usize) -> Result<()> {
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
        if allowed.is_empty() || setup.dim_agg.contains_key(dim) {
            return true;
        }
        rec.keys.get(*dim).is_some_and(|k| allowed.contains(k))
    })
}

struct FileGroup {
    file_index: usize,
    cells: Vec<(String, Vec<f64>)>,
}

impl FileGroup {
    fn push(&mut self, x: String, value: f64) {
        match self.cells.iter_mut().find(|(cx, _)| cx == &x) {
            Some((_, vals)) => vals.push(value),
            None => self.cells.push((x, vec![value])),
        }
    }

    fn aggregate(&self, x: &str, how: DimAgg) -> f64 {
        self.cells
            .iter()
            .find(|(cx, _)| cx == x)
            .map(|(_, vals)| how.apply(vals))
            .unwrap_or(f64::NAN)
    }

    fn into_pairs(self) -> (Vec<String>, Vec<f64>) {
        let mut xs = Vec::new();
        let mut ys = Vec::new();
        for (x, vals) in self.cells {
            for v in vals {
                xs.push(x.clone());
                ys.push(v);
            }
        }
        (xs, ys)
    }
}

fn group_for(groups: &mut Vec<FileGroup>, file_index: usize) -> &mut FileGroup {
    if let Some(pos) = groups.iter().position(|g| g.file_index == file_index) {
        return &mut groups[pos];
    }
    groups.push(FileGroup { file_index, cells: Vec::new() });
    groups.last_mut().unwrap()
}
