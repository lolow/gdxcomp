use std::collections::{HashMap, HashSet};

use serde::Serialize;

use crate::error::{CoreError, Result};
use crate::model::{LoadedFile, Rec, SymbolKind, SymbolMeta};
use crate::setup::{AppMode, DimAgg, DisplaySetup, Field};
use crate::witch::YearMapper;

const MAX_TRACES: usize = 30;

/// A single x-axis value: either a categorical string or a numeric year.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum XValue {
    Str(String),
    Num(f64),
}

impl PartialEq for XValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (XValue::Str(a), XValue::Str(b)) => a == b,
            (XValue::Num(a), XValue::Num(b)) => a == b,
            _ => false,
        }
    }
}

impl PartialEq<str> for XValue {
    fn eq(&self, other: &str) -> bool {
        matches!(self, XValue::Str(s) if s == other)
    }
}

impl PartialEq<&str> for XValue {
    fn eq(&self, other: &&str) -> bool {
        matches!(self, XValue::Str(s) if s == *other)
    }
}

/// One plotted series — one per file.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Trace {
    pub name: String,
    pub x: Vec<XValue>,
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
    pub x_label: String,
    pub traces: Vec<Trace>,
    pub dim_names: Vec<String>,
    pub table: Vec<TableRow>,
}

/// Chart-only slice of [`PlotView`]: same shape minus the `table` rows.
/// Used by `get_chart_view` to keep IPC payload small when the user is on
/// the chart tab.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChartView {
    pub symbol: String,
    pub kind: SymbolKind,
    pub field: Field,
    pub x_label: String,
    pub traces: Vec<Trace>,
    pub dim_names: Vec<String>,
}

/// Table-only slice of [`PlotView`]: just dim names + rows.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TableView {
    pub dim_names: Vec<String>,
    pub table: Vec<TableRow>,
}

impl From<PlotView> for ChartView {
    fn from(v: PlotView) -> Self {
        Self {
            symbol: v.symbol,
            kind: v.kind,
            field: v.field,
            x_label: v.x_label,
            traces: v.traces,
            dim_names: v.dim_names,
        }
    }
}

impl From<PlotView> for TableView {
    fn from(v: PlotView) -> Self {
        Self {
            dim_names: v.dim_names,
            table: v.table,
        }
    }
}

/// Chart-only build. Skips table-row collection for a cheaper IPC payload.
/// Use when the UI is on the chart tab; pair with [`build_table`] on tab switch.
pub fn build_chart(files: &[LoadedFile], setup: &DisplaySetup) -> Result<ChartView> {
    Ok(build_internal(files, setup, false)?.into())
}

/// Table-only build. Companion to [`build_chart`].
pub fn build_table(files: &[LoadedFile], setup: &DisplaySetup) -> Result<TableView> {
    Ok(build_internal(files, setup, true)?.into())
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

fn extract_unit(text: &str) -> Option<&str> {
    let start = text.rfind('[')?;
    let end = text[start..].find(']').map(|i| start + i)?;
    Some(&text[start + 1..end])
}

fn default_dim_agg(text: &str) -> DimAgg {
    let Some(unit) = extract_unit(text) else {
        return DimAgg::Sum;
    };
    if matches!(unit, "%" | "index" | "ratio" | "1" | "-") {
        return DimAgg::Mean;
    }
    if let Some(denom) = unit.split('/').nth(1) {
        const QTY: &[&str] = &[
            "gj", "mj", "tj", "ej", "kwh", "mwh", "gwh", "twh", "w", "toe", "tc", "tco2", "gtc",
            "gtonc", "ton", "cap", "person",
        ];
        let denom_lc = denom.to_ascii_lowercase();
        if QTY.iter().any(|t| denom_lc.contains(t)) {
            return DimAgg::Mean;
        }
    }
    DimAgg::Sum
}

/// Fill in sensible defaults before `build_view`:
/// - uninitialized non-x dims defaulted based on unit (intensive → mean, extensive → sum)
pub fn refine_setup(files: &[LoadedFile], setup: &DisplaySetup) -> Result<DisplaySetup> {
    let mut refined = setup.clone();
    let first_file = files.iter().find(|f| f.symbol(&setup.symbol).is_some());
    let meta = first_file.and_then(|f| f.symbol(&setup.symbol));

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
            refined.dim_agg.insert(d, default_dim_agg(&m.text));
        }
    }

    Ok(refined)
}

/// Build the chart + table for `setup` across the given files.
/// Each file produces exactly one trace. Call [`refine_setup`] first.
pub fn build_view(files: &[LoadedFile], setup: &DisplaySetup) -> Result<PlotView> {
    build_internal(files, setup, true)
}

fn build_internal(files: &[LoadedFile], setup: &DisplaySetup, want_table: bool) -> Result<PlotView> {
    let meta = files
        .iter()
        .find_map(|f| f.symbol(&setup.symbol))
        .ok_or_else(|| CoreError::SymbolMissing(setup.symbol.clone()))?
        .clone();

    validate_dim(&meta, setup.x_dim)?;

    let dim_names = dimension_names(&meta);

    // In WITCH mode, map the "t" x-axis dimension to calendar years.
    let year_mapper: Option<YearMapper> = if setup.mode == AppMode::Witch {
        dim_names
            .get(setup.x_dim)
            .filter(|n| n.as_str() == "t")
            .map(|_| YearMapper::new(files))
    } else {
        None
    };

    let x_label = if year_mapper.is_some() {
        "year".to_string()
    } else {
        axis_label(&meta, &dim_names, setup.x_dim)
    };

    // Map a raw UEL key to its display string (year as integer string, or the UEL itself).
    let year_str = |raw: &str| -> String {
        year_mapper
            .as_ref()
            .and_then(|m| m.get(raw))
            .map(|y| format!("{:.0}", y))
            .unwrap_or_else(|| raw.to_string())
    };

    let needs_agg = !setup.dim_agg.is_empty();
    let agg_method = setup
        .dim_agg
        .values()
        .copied()
        .next()
        .unwrap_or(DimAgg::Sum);

    // Precompute filter sets once — O(1) lookup per record vs O(F) Vec::contains.
    let filter_sets: HashMap<usize, HashSet<&str>> = setup
        .filters
        .iter()
        .filter(|(d, v)| !v.is_empty() && !setup.dim_agg.contains_key(*d))
        .map(|(d, v)| (*d, v.iter().map(String::as_str).collect()))
        .collect();

    // Only collect raw table rows when the caller wants a table and there is no
    // aggregation (aggregated table is built from traces after the loop).
    let collect_raw = want_table && !needs_agg;

    let mut x_order: Vec<String> = Vec::new();
    let mut groups: Vec<FileGroup> = Vec::new();
    let mut raw_table: Vec<TableRow> = Vec::new();

    for (fi, file) in files.iter().enumerate() {
        if file.symbol(&setup.symbol).is_none() {
            continue;
        }
        let records = file.read_records_arc(&setup.symbol)?;
        for rec in records.iter() {
            if !passes_filters(rec, &filter_sets) {
                continue;
            }
            let value = rec.values[setup.field.index()];
            let raw = x_key(rec, setup.x_dim);
            let x = year_str(&raw);

            if needs_agg && !x_order.contains(&x) {
                x_order.push(x.clone());
            }
            group_for(&mut groups, fi).push(x, value);

            if collect_raw {
                // Apply year mapping to the x-dim key in the table row too.
                let mut keys: Vec<String> = rec.keys.iter().map(|k| k.to_string()).collect();
                if let Some(k) = keys.get_mut(setup.x_dim) {
                    *k = year_str(k);
                }
                raw_table.push(TableRow {
                    file: file.label.clone(),
                    keys,
                    value,
                });
            }
        }
    }

    let to_xvalue = |s: String| -> XValue {
        if year_mapper.is_some() {
            if let Ok(n) = s.parse::<f64>() {
                return XValue::Num(n);
            }
        }
        XValue::Str(s)
    };

    let traces: Vec<Trace> = groups
        .into_iter()
        .map(|g| {
            let name = files[g.file_index].label.clone();
            let (xs, y) = if needs_agg {
                let y = x_order.iter().map(|x| g.aggregate(x, agg_method)).collect();
                (x_order.clone(), y)
            } else {
                g.into_pairs()
            };
            let x = xs.into_iter().map(&to_xvalue).collect();
            Trace { name, x, y }
        })
        .collect();

    if traces.len() > MAX_TRACES {
        return Err(CoreError::TooManyTraces {
            traces: traces.len(),
            max: MAX_TRACES,
        });
    }

    // When aggregating, the table mirrors the chart: one row per (file, x) with
    // the aggregated value. Each non-x dim is labelled with its agg method or
    // filter value so every column stays aligned with the header.
    let table: Vec<TableRow> = if needs_agg && want_table {
        let ndim = meta.dim;
        traces
            .iter()
            .flat_map(|t| {
                t.x.iter().zip(t.y.iter()).map(move |(x, &y)| {
                    let x_str = match x {
                        XValue::Str(s) => s.clone(),
                        XValue::Num(n) => format!("{:.0}", n),
                    };
                    let keys: Vec<String> = (0..ndim)
                        .map(|d| {
                            if d == setup.x_dim {
                                return x_str.clone();
                            }
                            if let Some(agg) = setup.dim_agg.get(&d) {
                                return match agg {
                                    DimAgg::Sum => "sum".to_string(),
                                    DimAgg::Mean => "mean".to_string(),
                                };
                            }
                            setup
                                .filters
                                .get(&d)
                                .and_then(|v| v.first())
                                .cloned()
                                .unwrap_or_default()
                        })
                        .collect();
                    TableRow {
                        file: t.name.clone(),
                        keys,
                        value: y,
                    }
                })
            })
            .collect()
    } else {
        raw_table
    };

    Ok(PlotView {
        symbol: meta.name.clone(),
        kind: meta.kind,
        field: setup.field,
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
        .map(|k| k.to_string())
        .unwrap_or_else(|| "value".to_string())
}

fn passes_filters(rec: &Rec, filter_sets: &HashMap<usize, HashSet<&str>>) -> bool {
    filter_sets
        .iter()
        .all(|(dim, allowed)| rec.keys.get(*dim).is_some_and(|k| allowed.contains(k.as_ref())))
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
    groups.push(FileGroup {
        file_index,
        cells: Vec::new(),
    });
    groups.last_mut().unwrap()
}
