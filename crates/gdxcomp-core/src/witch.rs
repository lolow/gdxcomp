use std::collections::HashMap;

use crate::model::LoadedFile;

/// Maps WITCH time-period UELs (e.g. `"t10"`) to calendar years (e.g. `2050.0`).
///
/// Source priority:
///  1. The `year(t)` 1-dim parameter from the GDX files.
///  2. Fallback formula: `2000 + 5 × val(t)`, where `val(t)` is the numeric
///     suffix of the UEL label (e.g. `"t10"` → 10 → 2050).
pub struct YearMapper {
    explicit: HashMap<String, f64>,
}

impl YearMapper {
    pub fn new(files: &[LoadedFile]) -> Self {
        for file in files {
            if let Some(sym) = file.symbols.iter().find(|s| s.name == "year" && s.dim == 1) {
                if let Ok(records) = file.read_records_arc(&sym.name) {
                    let explicit: HashMap<String, f64> = records
                        .iter()
                        .filter_map(|r| {
                            let key = r.keys.first().map(|k| k.to_string())?;
                            let val = r.values[0];
                            val.is_finite().then_some((key, val))
                        })
                        .collect();
                    if !explicit.is_empty() {
                        return YearMapper { explicit };
                    }
                }
            }
        }
        YearMapper {
            explicit: HashMap::new(),
        }
    }

    /// Returns the calendar year for `uel`, or `None` if the suffix cannot be parsed.
    pub fn get(&self, uel: &str) -> Option<f64> {
        if let Some(&y) = self.explicit.get(uel) {
            return Some(y);
        }
        // Fallback: strip leading non-digits, parse the numeric suffix.
        let digits: String = uel.chars().skip_while(|c| !c.is_ascii_digit()).collect();
        let val: f64 = digits.parse().ok()?;
        Some(2000.0 + 5.0 * val)
    }
}
