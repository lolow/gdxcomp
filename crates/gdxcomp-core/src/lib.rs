//! UI-independent domain logic for gdxcomp.
//!
//! - [`LoadedFile`] reads a GDX file into an owned, thread-safe snapshot.
//! - [`common_symbols`] finds symbols comparable across files (same name/dim/kind).
//! - [`build_view`] turns a [`DisplaySetup`] into a [`PlotView`] (chart traces + table).
//! - [`DisplaySetup`] is the serializable display configuration (JSON import/export).

mod compare;
mod error;
mod model;
mod setup;
mod witch;

pub use compare::{build_view, common_symbols, refine_setup, PlotView, TableRow, Trace, XValue};
pub use error::{CoreError, Result};
pub use model::{LoadedFile, Rec, SymbolKind, SymbolMeta};
pub use setup::{AppMode, DimAgg, DisplaySetup, Field};
