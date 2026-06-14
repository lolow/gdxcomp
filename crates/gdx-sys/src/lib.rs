//! Raw FFI bindings to the GAMS GDX C library (`libgdxcclib64`).
//!
//! These bind directly to the library's exported entry points: the
//! object-management functions (`gdxcreate`/`gdxfree`) and the `c__gdx*`
//! "explicit object" wrappers, which take the GDX object pointer as their
//! first argument. This avoids the dynamic-loading wrapper (`gdxcc.c`) and its
//! global function-pointer table entirely.
//!
//! Symbol names are the lowercase forms exported on Linux/macOS (the upstream
//! header lowercases them via macros for the no-leading-underscore convention).
//!
//! Everything here is `unsafe`; see the `gdx` crate for the safe wrapper.
#![allow(non_snake_case)]

use std::os::raw::{c_char, c_double, c_int, c_void};

/// Short-string buffer size used throughout the GDX API.
pub const GMS_SSSIZE: usize = 256;
/// Maximum number of index dimensions for a symbol.
pub const GMS_MAX_INDEX_DIM: usize = 20;
/// Number of value fields per record (level, marginal, lower, upper, scale).
pub const GMS_VAL_MAX: usize = 5;

pub const GMS_VAL_LEVEL: usize = 0;
pub const GMS_VAL_MARGINAL: usize = 1;
pub const GMS_VAL_LOWER: usize = 2;
pub const GMS_VAL_UPPER: usize = 3;
pub const GMS_VAL_SCALE: usize = 4;

pub const GMS_DT_SET: c_int = 0;
pub const GMS_DT_PAR: c_int = 1;
pub const GMS_DT_VAR: c_int = 2;
pub const GMS_DT_EQU: c_int = 3;
pub const GMS_DT_ALIAS: c_int = 4;

/// Length of the special-value array returned by [`c__gdxgetspecialvalues`].
pub const GMS_SVIDX_MAX: usize = 7;
pub const GMS_SVIDX_UNDEF: usize = 0;
pub const GMS_SVIDX_NA: usize = 1;
pub const GMS_SVIDX_PINF: usize = 2;
pub const GMS_SVIDX_MINF: usize = 3;
pub const GMS_SVIDX_EPS: usize = 4;

/// Opaque GDX object handle (`TGXFileRec_t`).
pub type GdxObj = c_void;

extern "C" {
    /// Create a GDX object. Returns 1 on success and writes a message into `msg`.
    pub fn gdxcreate(pobj: *mut *mut GdxObj, msg: *mut c_char, msglen: c_int) -> c_int;
    /// Destroy a GDX object created by [`gdxcreate`].
    pub fn gdxfree(pobj: *mut *mut GdxObj) -> c_int;

    /// Open a GDX file for reading. Returns 1 on success; otherwise `errnr` is set.
    pub fn c__gdxopenread(obj: *mut GdxObj, filename: *const c_char, errnr: *mut c_int) -> c_int;
    /// Open a GDX file for writing. Returns 1 on success; otherwise `errnr` is set.
    pub fn c__gdxopenwrite(
        obj: *mut GdxObj,
        filename: *const c_char,
        producer: *const c_char,
        errnr: *mut c_int,
    ) -> c_int;
    /// Close the currently open GDX file.
    pub fn c__gdxclose(obj: *mut GdxObj) -> c_int;

    /// Number of symbols and unique elements in the open file.
    pub fn c__gdxsysteminfo(obj: *mut GdxObj, symcnt: *mut c_int, uelcnt: *mut c_int) -> c_int;
    /// Name, dimension and type (`GMS_DT_*`) of symbol number `synr` (1-based).
    pub fn c__gdxsymbolinfo(
        obj: *mut GdxObj,
        synr: c_int,
        syid: *mut c_char,
        dim: *mut c_int,
        typ: *mut c_int,
    ) -> c_int;
    /// Record count, user info (subtype) and explanatory text of symbol `synr`.
    pub fn c__gdxsymbolinfox(
        obj: *mut GdxObj,
        synr: c_int,
        reccnt: *mut c_int,
        userinfo: *mut c_int,
        expl: *mut c_char,
    ) -> c_int;
    /// Domain identifiers for symbol `synr`. `domainids` must hold `dim` `char*`.
    pub fn c__gdxsymbolgetdomainx(
        obj: *mut GdxObj,
        synr: c_int,
        domainids: *mut *mut c_char,
    ) -> c_int;

    /// Begin reading symbol `synr` in string mode; sets `nrecs`.
    pub fn c__gdxdatareadstrstart(obj: *mut GdxObj, synr: c_int, nrecs: *mut c_int) -> c_int;
    /// Read one record: fills `keystr` (array of `dim` C strings) and `values[5]`.
    /// Returns 1 while records remain, 0 when exhausted.
    pub fn c__gdxdatareadstr(
        obj: *mut GdxObj,
        keystr: *mut *mut c_char,
        values: *mut c_double,
        dimfrst: *mut c_int,
    ) -> c_int;
    /// Finish the current read.
    pub fn c__gdxdatareaddone(obj: *mut GdxObj) -> c_int;

    /// Begin writing a symbol in string mode.
    pub fn c__gdxdatawritestrstart(
        obj: *mut GdxObj,
        syid: *const c_char,
        expltxt: *const c_char,
        dimen: c_int,
        typ: c_int,
        userinfo: c_int,
    ) -> c_int;
    /// Write one record: `keystr` (array of `dim` C strings) and `values[5]`.
    pub fn c__gdxdatawritestr(
        obj: *mut GdxObj,
        keystr: *const *const c_char,
        values: *const c_double,
    ) -> c_int;
    /// Finish the current write.
    pub fn c__gdxdatawritedone(obj: *mut GdxObj) -> c_int;

    /// Special-value array (EPS/NA/+Inf/-Inf/Undef) used in records.
    pub fn c__gdxgetspecialvalues(obj: *mut GdxObj, avals: *mut c_double) -> c_int;
    /// Number of the last error, or 0.
    pub fn c__gdxgetlasterror(obj: *mut GdxObj) -> c_int;
    /// Human-readable message for error number `errnr`.
    pub fn c__gdxerrorstr(obj: *mut GdxObj, errnr: c_int, errmsg: *mut c_char) -> c_int;

    /// Begin reading symbol `synr` in raw (integer-key) mode; sets `nrecs`.
    /// Keys arrive as 1-based global UEL indices; use [`c__gdxumuelget`] to resolve them.
    pub fn c__gdxdatareadrawstart(obj: *mut GdxObj, synr: c_int, nrecs: *mut c_int) -> c_int;
    /// Read one record in raw mode: fills `keyint` (array of `dim` UEL indices) and `values[5]`.
    /// Returns 1 while records remain, 0 when exhausted.
    pub fn c__gdxdatareadraw(
        obj: *mut GdxObj,
        keyint: *mut c_int,
        values: *mut c_double,
        dimfrst: *mut c_int,
    ) -> c_int;

    /// UEL count and highest mapped index for the open file.
    pub fn c__gdxumuelinfo(obj: *mut GdxObj, uelcnt: *mut c_int, highmap: *mut c_int) -> c_int;
    /// Resolve UEL number `uelnr` to its label string.
    /// Writes into `uel` (caller provides [`GMS_SSSIZE`] buffer); sets `uelmap`.
    /// Returns 1 on success, 0 if `uelnr` is out of range.
    pub fn c__gdxumuelget(
        obj: *mut GdxObj,
        uelnr: c_int,
        uel: *mut c_char,
        uelmap: *mut c_int,
    ) -> c_int;
}
