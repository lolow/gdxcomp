use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::Path;
use std::ptr;
use std::sync::Arc;

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
    name_index: HashMap<String, usize>,
    /// Lazy per-file UEL cache: index → interned label (populated on first raw read).
    uel_cache: RefCell<HashMap<i32, Arc<str>>>,
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

            let name_index = symbols
                .iter()
                .enumerate()
                .map(|(i, s)| (s.name.clone(), i))
                .collect();

            Ok(GdxFile {
                obj,
                special,
                symbols,
                name_index,
                uel_cache: RefCell::new(HashMap::new()),
            })
        }
    }

    /// The file's symbol table (cached at open time).
    pub fn symbols(&self) -> &[SymbolInfo] {
        &self.symbols
    }

    /// Look up a symbol by name (case-sensitive, as stored in the file).
    pub fn symbol(&self, name: &str) -> Option<&SymbolInfo> {
        self.name_index.get(name).map(|&i| &self.symbols[i])
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
        unsafe { self.read_records_raw(info.number, info.dim) }
    }

    /// Read all records in raw mode (integer UEL indices → interned `Arc<str>` labels).
    ///
    /// Caller must hold the global FFI lock. UEL labels are resolved lazily via
    /// [`c__gdxumuelget`] and memoised in `self.uel_cache` so each unique UEL
    /// string is allocated at most once per `GdxFile` instance.
    unsafe fn read_records_raw(&self, number: usize, dim: usize) -> Result<Vec<Record>> {
        let mut nrecs = 0;
        if ffi::c__gdxdatareadrawstart(self.obj, number as i32, &mut nrecs) == 0 {
            return Err(op_error(self.obj, "gdxDataReadRawStart"));
        }

        let mut key_indices = [0i32; ffi::GMS_MAX_INDEX_DIM];
        let mut values = [0.0f64; ffi::GMS_VAL_MAX];
        let mut dimfrst = 0;

        let mut records = Vec::with_capacity(nrecs.max(0) as usize);
        let mut uel_cache = self.uel_cache.borrow_mut();

        while ffi::c__gdxdatareadraw(
            self.obj,
            key_indices.as_mut_ptr(),
            values.as_mut_ptr(),
            &mut dimfrst,
        ) == 1
        {
            let keys: Vec<Arc<str>> = (0..dim)
                .map(|d| {
                    let uelnr = key_indices[d];
                    if uelnr <= 0 {
                        return Arc::from("");
                    }
                    if let Some(s) = uel_cache.get(&uelnr) {
                        return Arc::clone(s);
                    }
                    let mut buf = [0 as c_char; ffi::GMS_SSSIZE];
                    let mut uel_map = 0;
                    let label: Arc<str> =
                        if ffi::c__gdxumuelget(self.obj, uelnr, buf.as_mut_ptr(), &mut uel_map) == 1
                        {
                            Arc::from(buf_to_string(&buf).as_str())
                        } else {
                            Arc::from(format!("<uel {uelnr}>").as_str())
                        };
                    uel_cache.insert(uelnr, Arc::clone(&label));
                    label
                })
                .collect();

            let mut mapped = [0.0f64; ffi::GMS_VAL_MAX];
            for (i, v) in values.iter().enumerate() {
                mapped[i] = map_special(*v, &self.special);
            }
            records.push(Record {
                keys,
                values: mapped,
            });
        }
        ffi::c__gdxdatareaddone(self.obj);
        Ok(records)
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

#[cfg(test)]
mod parity {
    //! Verify that raw-mode reading produces the same keys and values as the
    //! original string-mode API, using a self-written fixture.

    use std::sync::Arc;

    use crate::{GdxFile, GdxWriter, Record, SymbolType};

    fn rec(keys: &[&str], level: f64) -> Record {
        Record {
            keys: keys.iter().map(|s| Arc::from(*s)).collect(),
            values: [level, 0.0, 0.0, 0.0, 0.0],
        }
    }

    #[test]
    fn raw_matches_expected_keys_and_values() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("parity.gdx");

        let mut w = GdxWriter::create(&path, "parity-test").unwrap();
        w.write_symbol(
            "c",
            "cost",
            2,
            SymbolType::Parameter,
            0,
            &[
                rec(&["seattle", "new-york"], 0.225),
                rec(&["seattle", "chicago"], 0.153),
                rec(&["san-diego", "topeka"], 0.126),
            ],
        )
        .unwrap();
        w.finish().unwrap();

        let file = GdxFile::open(&path).unwrap();
        let records = file.read("c").unwrap();

        assert_eq!(records.len(), 3);
        assert_eq!(records[0].keys[0].as_ref(), "seattle");
        assert_eq!(records[0].keys[1].as_ref(), "new-york");
        assert!((records[0].values[0] - 0.225).abs() < 1e-12);
        assert_eq!(records[1].keys[0].as_ref(), "seattle");
        assert_eq!(records[1].keys[1].as_ref(), "chicago");
        assert!((records[1].values[0] - 0.153).abs() < 1e-12);
        assert_eq!(records[2].keys[0].as_ref(), "san-diego");
        assert_eq!(records[2].keys[1].as_ref(), "topeka");
        assert!((records[2].values[0] - 0.126).abs() < 1e-12);
    }

    #[test]
    fn shared_uel_labels_are_the_same_arc() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("shared.gdx");

        let mut w = GdxWriter::create(&path, "parity-test").unwrap();
        w.write_symbol(
            "c",
            "cost",
            2,
            SymbolType::Parameter,
            0,
            &[
                rec(&["seattle", "new-york"], 0.225),
                rec(&["seattle", "chicago"], 0.153),
            ],
        )
        .unwrap();
        w.finish().unwrap();

        let file = GdxFile::open(&path).unwrap();
        let records = file.read("c").unwrap();

        // Both records share the "seattle" UEL — the Arc pointers must be identical.
        assert!(Arc::ptr_eq(&records[0].keys[0], &records[1].keys[0]));
    }
}
