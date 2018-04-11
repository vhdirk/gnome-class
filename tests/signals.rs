#![feature(proc_macro)]

extern crate gobject_gen;
extern crate gobject_sys;

#[macro_use]
extern crate glib;

extern crate glib_sys;

extern crate libc;

use gobject_gen::gobject_gen;
use std::cell::Cell;
use std::ffi::CStr;
use std::mem;
use std::slice;

use glib::object::*;
use glib::translate::*;

struct SignalerPrivate {
    val: Cell<u32>,
}

impl Default for SignalerPrivate {
    fn default() -> Self {
        SignalerPrivate { val: Cell::new(0) }
    }
}

gobject_gen! {
    class Signaler {
        type InstancePrivate = SignalerPrivate;
    }

    impl Signaler {
        signal fn value_changed(&self);
        signal fn value_changed_to(&self, v: u32);

        pub fn set_value(&self, v: u32) {
            let private = self.get_priv();
            private.val.set(v);
            self.emit_value_changed();
        }

        pub fn get_value(&self) -> u32 {
            let private = self.get_priv();
            private.val.get()
        }
    }
}

#[cfg(test)]
fn check_signal(
    query: &gobject_sys::GSignalQuery,
    obj_type: glib_sys::GType,
    signal_id: libc::c_uint,
    signal_name: &str,
) {
    assert_eq!(query.itype, obj_type);
    assert_eq!(query.signal_id, signal_id);

    let name = unsafe { CStr::from_ptr(query.signal_name) };
    assert_eq!(name.to_str().unwrap(), signal_name);
}

#[test]
fn has_signals() {
    let obj = Signaler::new();
    let obj_type = obj.get_type().to_glib();

    unsafe {
        let mut n_ids: libc::c_uint = 0;

        let raw_signal_ids = gobject_sys::g_signal_list_ids(obj_type, &mut n_ids);
        assert_eq!(n_ids, 2);

        let n_ids = n_ids as usize;

        let signal_ids = slice::from_raw_parts(raw_signal_ids, n_ids);

        // value-changed

        let mut query: gobject_sys::GSignalQuery = mem::zeroed();
        gobject_sys::g_signal_query(signal_ids[0], &mut query);

        check_signal(&query, obj_type, signal_ids[0], "value-changed");

        assert_eq!(query.n_params, 0);
        assert!(query.param_types.is_null());

        // value-changed-to

        let mut query: gobject_sys::GSignalQuery = mem::zeroed();
        gobject_sys::g_signal_query(signal_ids[1], &mut query);

        check_signal(&query, obj_type, signal_ids[1], "value-changed-to");
    }
}

#[test]
fn connects_to_signal() {
    let obj = Signaler::new();

    static mut EMITTED: bool = false;

    let _id: glib::SignalHandlerId = obj.connect_value_changed(|_| unsafe {
        EMITTED = true;
    });

    obj.set_value(42);

    unsafe {
        assert!(EMITTED);
    }
}
