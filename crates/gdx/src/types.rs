use gdx_sys as ffi;

/// The kind of a GDX symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolType {
    Set,
    Parameter,
    Variable,
    Equation,
    Alias,
}

impl SymbolType {
    pub(crate) fn from_raw(raw: i32) -> Option<Self> {
        match raw {
            ffi::GMS_DT_SET => Some(Self::Set),
            ffi::GMS_DT_PAR => Some(Self::Parameter),
            ffi::GMS_DT_VAR => Some(Self::Variable),
            ffi::GMS_DT_EQU => Some(Self::Equation),
            ffi::GMS_DT_ALIAS => Some(Self::Alias),
            _ => None,
        }
    }

    pub(crate) fn to_raw(self) -> i32 {
        match self {
            SymbolType::Set => ffi::GMS_DT_SET,
            SymbolType::Parameter => ffi::GMS_DT_PAR,
            SymbolType::Variable => ffi::GMS_DT_VAR,
            SymbolType::Equation => ffi::GMS_DT_EQU,
            SymbolType::Alias => ffi::GMS_DT_ALIAS,
        }
    }

    /// Whether records of this symbol carry the five value fields
    /// (level/marginal/lower/upper/scale) rather than a single value.
    pub fn has_fields(self) -> bool {
        matches!(self, SymbolType::Variable | SymbolType::Equation)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            SymbolType::Set => "Set",
            SymbolType::Parameter => "Parameter",
            SymbolType::Variable => "Variable",
            SymbolType::Equation => "Equation",
            SymbolType::Alias => "Alias",
        }
    }
}

/// One of the five value fields stored per Variable/Equation record.
///
/// For Sets and Parameters only [`ValueField::Level`] is meaningful (it holds
/// the parameter value; for sets it is the set's numeric value, usually 0).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueField {
    Level,
    Marginal,
    Lower,
    Upper,
    Scale,
}

impl ValueField {
    pub fn index(self) -> usize {
        match self {
            ValueField::Level => ffi::GMS_VAL_LEVEL,
            ValueField::Marginal => ffi::GMS_VAL_MARGINAL,
            ValueField::Lower => ffi::GMS_VAL_LOWER,
            ValueField::Upper => ffi::GMS_VAL_UPPER,
            ValueField::Scale => ffi::GMS_VAL_SCALE,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            ValueField::Level => "Level",
            ValueField::Marginal => "Marginal",
            ValueField::Lower => "Lower",
            ValueField::Upper => "Upper",
            ValueField::Scale => "Scale",
        }
    }
}

/// Metadata describing a symbol in a GDX file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolInfo {
    /// 1-based symbol number within the file.
    pub number: usize,
    pub name: String,
    pub dim: usize,
    pub kind: SymbolType,
    /// Subtype / "user info" (e.g. variable type for Variables).
    pub subtype: i32,
    pub records: usize,
    pub text: String,
    /// Domain set names, one per dimension (`"*"` for the universe).
    pub domains: Vec<String>,
}

/// One data record: `keys.len() == dim` string indices, plus five value fields.
///
/// Special GDX values are mapped to `f64`: Undefined/NA → `NaN`,
/// ±Infinity → `f64::INFINITY`/`NEG_INFINITY`, EPS → `0.0`.
#[derive(Debug, Clone, PartialEq)]
pub struct Record {
    pub keys: Vec<String>,
    pub values: [f64; ffi::GMS_VAL_MAX],
}

impl Record {
    /// Value for the given field.
    pub fn value(&self, field: ValueField) -> f64 {
        self.values[field.index()]
    }
}
