use std::ffi::CString;
use std::os::raw::c_char;
use std::path::Path;
use std::ptr;

use gdx_sys as ffi;

use crate::error::{GdxError, Result};
use crate::reader::error_message;
use crate::types::{Record, SymbolType};

/// A GDX file opened for writing.
///
/// Primarily used to build self-contained test fixtures, but also a complete
/// writer for ordinary (finite) numeric data. Call [`finish`](GdxWriter::finish)
/// to close cleanly; dropping without finishing still closes the file.
pub struct GdxWriter {
    obj: *mut ffi::GdxObj,
}

impl GdxWriter {
    /// Create `path`, overwriting any existing file. `producer` is recorded in
    /// the file's audit metadata.
    pub fn create(path: impl AsRef<Path>, producer: &str) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let cpath = CString::new(path.to_string_lossy().as_bytes())
            .map_err(|_| GdxError::InvalidPath(path.clone()))?;
        let cproducer = CString::new(producer).unwrap_or_default();

        let _guard = crate::lock::lock();
        unsafe {
            let mut obj: *mut ffi::GdxObj = ptr::null_mut();
            let mut msg = [0 as c_char; ffi::GMS_SSSIZE];
            if ffi::gdxcreate(&mut obj, msg.as_mut_ptr(), ffi::GMS_SSSIZE as i32) == 0
                || obj.is_null()
            {
                return Err(GdxError::Create(crate::reader::buf_to_string(&msg)));
            }
            let mut errnr = 0;
            if ffi::c__gdxopenwrite(obj, cpath.as_ptr(), cproducer.as_ptr(), &mut errnr) == 0 {
                let message = error_message(obj, errnr);
                ffi::gdxfree(&mut obj);
                return Err(GdxError::OpenWrite {
                    path,
                    code: errnr,
                    message,
                });
            }
            Ok(GdxWriter { obj })
        }
    }

    /// Write one symbol with all of its records.
    ///
    /// `dim` is the index dimension; every record's `keys` must have this length.
    pub fn write_symbol(
        &mut self,
        name: &str,
        text: &str,
        dim: usize,
        kind: SymbolType,
        subtype: i32,
        records: &[Record],
    ) -> Result<()> {
        let cname = CString::new(name).map_err(|_| GdxError::InvalidPath(name.into()))?;
        let ctext = CString::new(text).unwrap_or_default();

        let _guard = crate::lock::lock();
        unsafe {
            if ffi::c__gdxdatawritestrstart(
                self.obj,
                cname.as_ptr(),
                ctext.as_ptr(),
                dim as i32,
                kind.to_raw(),
                subtype,
            ) == 0
            {
                return Err(self.op_error("gdxDataWriteStrStart"));
            }

            for rec in records {
                debug_assert_eq!(rec.keys.len(), dim, "record key count must equal dim");
                let ckeys: Vec<CString> = rec
                    .keys
                    .iter()
                    .map(|k| CString::new(&**k).unwrap_or_default())
                    .collect();
                let keyptrs: Vec<*const c_char> = ckeys.iter().map(|k| k.as_ptr()).collect();
                if ffi::c__gdxdatawritestr(self.obj, keyptrs.as_ptr(), rec.values.as_ptr()) == 0 {
                    return Err(self.op_error("gdxDataWriteStr"));
                }
            }

            ffi::c__gdxdatawritedone(self.obj);
        }
        Ok(())
    }

    /// Close the file, flushing all data.
    pub fn finish(mut self) -> Result<()> {
        let guard = crate::lock::lock();
        unsafe {
            ffi::c__gdxclose(self.obj);
            ffi::gdxfree(&mut self.obj); // sets self.obj to null
        }
        drop(guard);
        // self drops here; its Drop sees a null obj and is a no-op.
        Ok(())
    }

    unsafe fn op_error(&self, op: &'static str) -> GdxError {
        let code = ffi::c__gdxgetlasterror(self.obj);
        GdxError::Operation {
            op,
            code,
            message: error_message(self.obj, code),
        }
    }
}

impl Drop for GdxWriter {
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
