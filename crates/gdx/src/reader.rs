use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::Path;
use std::ptr;

use gdx_sys as ffi;

use crate::error::{GdxError, Result};
use crate::types::{Record, SymbolInfo, SymbolType};

/// A GDX file opened for reading.
///
/// On [`open`](GdxFile::open) the symbol table and special-value sentinels are
/// loaded eagerly; record data is read on demand via [`read`](GdxFile::read).
/// The underlying GDX object is closed and freed on drop.
///
/// Not thread-safe: the type is intentionally `!Send`/`!Sync` (it holds a raw
/// pointer). Read the data you need into owned [`Record`]s, then drop the file.
pub struct GdxFile {
    obj: *mut ffi::GdxObj,
    special: [f64; ffi::GMS_SVIDX_MAX],
    symbols: Vec<SymbolInfo>,
}

impl GdxFile {
    /// Open `path` for reading.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let cpath = CString::new(path.to_string_lossy().as_bytes())
            .map_err(|_| GdxError::InvalidPath(path.clone()))?;

        let _guard = crate::lock::lock();
        unsafe {
            let mut obj: *mut ffi::GdxObj = ptr::null_mut();
            let mut msg = [0 as c_char; ffi::GMS_SSSIZE];
            if ffi::gdxcreate(&mut obj, msg.as_mut_ptr(), ffi::GMS_SSSIZE as i32) == 0
                || obj.is_null()
            {
                return Err(GdxError::Create(buf_to_string(&msg)));
            }

            let mut errnr = 0;
            if ffi::c__gdxopenread(obj, cpath.as_ptr(), &mut errnr) == 0 {
                let message = error_message(obj, errnr);
                ffi::gdxfree(&mut obj);
                return Err(GdxError::OpenRead {
                    path,
                    code: errnr,
                    message,
                });
            }

            let mut special = [0.0f64; ffi::GMS_SVIDX_MAX];
            ffi::c__gdxgetspecialvalues(obj, special.as_mut_ptr());

            let symbols = match read_symbol_table(obj) {
                Ok(s) => s,
                Err(e) => {
                    ffi::c__gdxclose(obj);
                    ffi::gdxfree(&mut obj);
                    return Err(e);
                }
            };

            Ok(GdxFile {
                obj,
                special,
                symbols,
            })
        }
    }

    /// The file's symbol table (cached at open time).
    pub fn symbols(&self) -> &[SymbolInfo] {
        &self.symbols
    }

    /// Look up a symbol by name (case-sensitive, as stored in the file).
    pub fn symbol(&self, name: &str) -> Option<&SymbolInfo> {
        self.symbols.iter().find(|s| s.name == name)
    }

    /// Read all records of the named symbol.
    pub fn read(&self, name: &str) -> Result<Vec<Record>> {
        let info = self
            .symbol(name)
            .ok_or_else(|| GdxError::SymbolNotFound(name.to_string()))?;
        self.read_info(info)
    }

    /// Read all records of a symbol described by `info`.
    pub fn read_info(&self, info: &SymbolInfo) -> Result<Vec<Record>> {
        let _guard = crate::lock::lock();
        unsafe { read_records(self.obj, info.number, info.dim, &self.special) }
    }
}

impl Drop for GdxFile {
    fn drop(&mut self) {
        if self.obj.is_null() {
            return;
        }
        let _guard = crate::lock::lock();
        unsafe {
            ffi::c__gdxclose(self.obj);
            ffi::gdxfree(&mut self.obj);
        }
    }
}

/// Map a GDX special-value sentinel onto an ordinary `f64`.
fn map_special(v: f64, special: &[f64; ffi::GMS_SVIDX_MAX]) -> f64 {
    if v == special[ffi::GMS_SVIDX_UNDEF] || v == special[ffi::GMS_SVIDX_NA] {
        f64::NAN
    } else if v == special[ffi::GMS_SVIDX_PINF] {
        f64::INFINITY
    } else if v == special[ffi::GMS_SVIDX_MINF] {
        f64::NEG_INFINITY
    } else if v == special[ffi::GMS_SVIDX_EPS] {
        0.0
    } else {
        v
    }
}

unsafe fn read_symbol_table(obj: *mut ffi::GdxObj) -> Result<Vec<SymbolInfo>> {
    let (mut symcnt, mut uelcnt) = (0, 0);
    if ffi::c__gdxsysteminfo(obj, &mut symcnt, &mut uelcnt) == 0 {
        return Err(op_error(obj, "gdxSystemInfo"));
    }

    let mut out = Vec::with_capacity(symcnt.max(0) as usize);
    for number in 1..=symcnt {
        let mut id = [0 as c_char; ffi::GMS_SSSIZE];
        let (mut dim, mut typ) = (0, 0);
        if ffi::c__gdxsymbolinfo(obj, number, id.as_mut_ptr(), &mut dim, &mut typ) == 0 {
            return Err(op_error(obj, "gdxSymbolInfo"));
        }
        let mut expl = [0 as c_char; ffi::GMS_SSSIZE];
        let (mut recs, mut userinfo) = (0, 0);
        ffi::c__gdxsymbolinfox(obj, number, &mut recs, &mut userinfo, expl.as_mut_ptr());

        let Some(kind) = SymbolType::from_raw(typ) else {
            continue; // skip unknown symbol categories
        };
        let dim = dim.max(0) as usize;

        out.push(SymbolInfo {
            number: number as usize,
            name: buf_to_string(&id),
            dim,
            kind,
            subtype: userinfo,
            records: recs.max(0) as usize,
            text: buf_to_string(&expl),
            domains: read_domains(obj, number, dim),
        });
    }
    Ok(out)
}

unsafe fn read_domains(obj: *mut ffi::GdxObj, number: i32, dim: usize) -> Vec<String> {
    if dim == 0 {
        return Vec::new();
    }
    let mut bufs = vec![[0 as c_char; ffi::GMS_SSSIZE]; dim];
    let mut ptrs: Vec<*mut c_char> = bufs.iter_mut().map(|b| b.as_mut_ptr()).collect();
    if ffi::c__gdxsymbolgetdomainx(obj, number, ptrs.as_mut_ptr()) == 0 {
        return vec!["*".to_string(); dim];
    }
    bufs.iter().map(|b| buf_to_string(b)).collect()
}

unsafe fn read_records(
    obj: *mut ffi::GdxObj,
    number: usize,
    dim: usize,
    special: &[f64; ffi::GMS_SVIDX_MAX],
) -> Result<Vec<Record>> {
    let mut nrecs = 0;
    if ffi::c__gdxdatareadstrstart(obj, number as i32, &mut nrecs) == 0 {
        return Err(op_error(obj, "gdxDataReadStrStart"));
    }

    // Key buffers: the API may write any of the symbol's `dim` keys.
    let mut keybufs = vec![[0 as c_char; ffi::GMS_SSSIZE]; ffi::GMS_MAX_INDEX_DIM];
    let mut keyptrs: Vec<*mut c_char> = keybufs.iter_mut().map(|b| b.as_mut_ptr()).collect();
    let mut values = [0.0f64; ffi::GMS_VAL_MAX];
    let mut dimfrst = 0;

    let mut records = Vec::with_capacity(nrecs.max(0) as usize);
    while ffi::c__gdxdatareadstr(obj, keyptrs.as_mut_ptr(), values.as_mut_ptr(), &mut dimfrst) == 1
    {
        let keys = (0..dim).map(|d| buf_to_string(&keybufs[d])).collect();
        let mut mapped = [0.0f64; ffi::GMS_VAL_MAX];
        for (i, v) in values.iter().enumerate() {
            mapped[i] = map_special(*v, special);
        }
        records.push(Record {
            keys,
            values: mapped,
        });
    }
    ffi::c__gdxdatareaddone(obj);
    Ok(records)
}

pub(crate) fn buf_to_string(buf: &[c_char]) -> String {
    unsafe { CStr::from_ptr(buf.as_ptr()) }
        .to_string_lossy()
        .into_owned()
}

pub(crate) unsafe fn error_message(obj: *mut ffi::GdxObj, code: i32) -> String {
    let mut msg = [0 as c_char; ffi::GMS_SSSIZE];
    if ffi::c__gdxerrorstr(obj, code, msg.as_mut_ptr()) == 1 {
        buf_to_string(&msg)
    } else {
        format!("gdx error {code}")
    }
}

unsafe fn op_error(obj: *mut ffi::GdxObj, op: &'static str) -> GdxError {
    let code = ffi::c__gdxgetlasterror(obj);
    GdxError::Operation {
        op,
        code,
        message: error_message(obj, code),
    }
}
