//! Phase-1 de-risk: exercise the raw FFI, including raw-mode reading and UEL lookup.
//!
//! `raw_read_roundtrip` is self-contained (no external file needed).
//! `reads_symbols_and_records` requires a real GDX file via `GDX_TEST_FILE`.
//! Run locally with:
//!   GDX_TEST_FILE=/opt/gams40/apifiles/GAMS/trnsport.gdx cargo test -p gdx-sys -- --ignored

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

use gdx_sys::*;

/// Write a tiny GDX via raw FFI, read it back in raw mode, and verify UEL resolution.
#[test]
fn raw_read_roundtrip() {
    let path = {
        let mut p = std::env::temp_dir();
        p.push(format!("gdxcomp_smoke_{}.gdx", std::process::id()));
        p
    };
    let cpath = CString::new(path.to_string_lossy().as_bytes()).unwrap();
    let cproducer = CString::new("smoke").unwrap();

    unsafe {
        // --- write ---
        let mut obj: *mut GdxObj = ptr::null_mut();
        let mut msg = [0 as c_char; GMS_SSSIZE];
        assert_eq!(gdxcreate(&mut obj, msg.as_mut_ptr(), GMS_SSSIZE as i32), 1);
        let mut errnr = 0;
        assert_eq!(c__gdxopenwrite(obj, cpath.as_ptr(), cproducer.as_ptr(), &mut errnr), 1, "open-write err {errnr}");

        let sym = CString::new("c").unwrap();
        let txt = CString::new("cost").unwrap();
        assert_eq!(c__gdxdatawritestrstart(obj, sym.as_ptr(), txt.as_ptr(), 2, GMS_DT_PAR, 0), 1);

        let k1 = CString::new("seattle").unwrap();
        let k2 = CString::new("new-york").unwrap();
        let keyptrs: [*const c_char; 2] = [k1.as_ptr(), k2.as_ptr()];
        let values = [0.225f64, 0.0, 0.0, 0.0, 0.0];
        assert_eq!(c__gdxdatawritestr(obj, keyptrs.as_ptr(), values.as_ptr()), 1);
        c__gdxdatawritedone(obj);
        c__gdxclose(obj);
        gdxfree(&mut obj);

        // --- read raw ---
        let mut obj2: *mut GdxObj = ptr::null_mut();
        assert_eq!(gdxcreate(&mut obj2, msg.as_mut_ptr(), GMS_SSSIZE as i32), 1);
        assert_eq!(c__gdxopenread(obj2, cpath.as_ptr(), &mut errnr), 1, "open-read err {errnr}");

        let mut nrecs = 0;
        assert_eq!(c__gdxdatareadrawstart(obj2, 1, &mut nrecs), 1);
        assert_eq!(nrecs, 1);

        let mut key_indices = [0i32; GMS_MAX_INDEX_DIM];
        let mut vals = [0.0f64; GMS_VAL_MAX];
        let mut dimfrst = 0;
        assert_eq!(c__gdxdatareadraw(obj2, key_indices.as_mut_ptr(), vals.as_mut_ptr(), &mut dimfrst), 1);
        assert!((vals[GMS_VAL_LEVEL] - 0.225).abs() < 1e-12, "unexpected value {}", vals[0]);

        let uel1 = key_indices[0];
        let uel2 = key_indices[1];
        assert!(uel1 > 0 && uel2 > 0, "UEL indices must be positive; got {uel1}, {uel2}");

        let mut uel_buf = [0 as c_char; GMS_SSSIZE];
        let mut uel_map = 0;
        assert_eq!(c__gdxumuelget(obj2, uel1, uel_buf.as_mut_ptr(), &mut uel_map), 1);
        let got1 = CStr::from_ptr(uel_buf.as_ptr()).to_string_lossy().into_owned();
        assert_eq!(got1, "seattle");

        assert_eq!(c__gdxumuelget(obj2, uel2, uel_buf.as_mut_ptr(), &mut uel_map), 1);
        let got2 = CStr::from_ptr(uel_buf.as_ptr()).to_string_lossy().into_owned();
        assert_eq!(got2, "new-york");

        c__gdxdatareaddone(obj2);
        c__gdxclose(obj2);
        gdxfree(&mut obj2);
    }

    let _ = std::fs::remove_file(&path);
}

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
