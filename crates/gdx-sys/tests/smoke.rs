//! Phase-1 de-risk: exercise the raw FFI against a real GDX file.
//!
//! Gated behind the `GDX_TEST_FILE` env var so the suite stays green where no
//! sample file is available (e.g. CI without a GAMS install). Run locally with:
//!   GDX_TEST_FILE=/opt/gams40/apifiles/GAMS/trnsport.gdx cargo test -p gdx-sys -- --ignored

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

use gdx_sys::*;

#[test]
#[ignore = "requires a GDX file via GDX_TEST_FILE"]
fn reads_symbols_and_records() {
    let path =
        std::env::var("GDX_TEST_FILE").expect("set GDX_TEST_FILE to a .gdx path to run this test");

    unsafe {
        let mut obj: *mut GdxObj = ptr::null_mut();
        let mut msg = [0 as c_char; GMS_SSSIZE];
        assert_eq!(gdxcreate(&mut obj, msg.as_mut_ptr(), GMS_SSSIZE as i32), 1);
        assert!(!obj.is_null());

        let cpath = CString::new(path).unwrap();
        let mut errnr = 0;
        assert_eq!(
            c__gdxopenread(obj, cpath.as_ptr(), &mut errnr),
            1,
            "open err {errnr}"
        );

        let (mut symcnt, mut uelcnt) = (0, 0);
        c__gdxsysteminfo(obj, &mut symcnt, &mut uelcnt);
        assert!(symcnt > 0, "expected at least one symbol");

        // Find a parameter and read at least one record.
        let mut read_any = false;
        for i in 1..=symcnt {
            let mut id = [0 as c_char; GMS_SSSIZE];
            let (mut dim, mut typ) = (0, 0);
            c__gdxsymbolinfo(obj, i, id.as_mut_ptr(), &mut dim, &mut typ);
            if typ != GMS_DT_PAR {
                continue;
            }
            let mut nrecs = 0;
            assert_eq!(c__gdxdatareadstrstart(obj, i, &mut nrecs), 1);

            let mut keybufs = vec![[0 as c_char; GMS_SSSIZE]; GMS_MAX_INDEX_DIM];
            let mut keys: Vec<*mut c_char> = keybufs.iter_mut().map(|b| b.as_mut_ptr()).collect();
            let mut vals = [0.0f64; GMS_VAL_MAX];
            let mut dimfrst = 0;

            if c__gdxdatareadstr(obj, keys.as_mut_ptr(), vals.as_mut_ptr(), &mut dimfrst) == 1 {
                read_any = true;
                let name = CStr::from_ptr(id.as_ptr()).to_string_lossy();
                let first_key = CStr::from_ptr(keys[0]).to_string_lossy().into_owned();
                eprintln!(
                    "read param {name}: key0={first_key} value={}",
                    vals[GMS_VAL_LEVEL]
                );
                assert!(!first_key.is_empty() || dim == 0);
            }
            c__gdxdatareaddone(obj);
            break;
        }
        assert!(read_any, "no parameter record was read");

        c__gdxclose(obj);
        gdxfree(&mut obj);
    }
}
