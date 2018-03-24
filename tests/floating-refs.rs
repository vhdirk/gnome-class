#![feature(proc_macro)]

extern crate gobject_gen;
extern crate glib_sys;

#[macro_use]
extern crate glib;
use gobject_gen::gobject_gen;

extern crate gobject_sys;

// glib_wrapper! wants these bindings
use glib_sys as glib_ffi;
use gobject_sys as gobject_ffi;
use std::mem;
use std::ptr;

use glib::translate::*;

// The glib crate doesn't bind GInitiallyUnowned, so let's bind it here.
glib_wrapper! {
    pub struct InitiallyUnowned(Object<gobject_sys::GInitiallyUnowned, gobject_sys::GInitiallyUnownedClass>);

    match fn {
        get_type => || gobject_sys::g_initially_unowned_get_type(),
    }
}

gobject_gen! {
    class Floating: InitiallyUnowned {
    }
}

#[test]
fn initially_unowned_starts_floating() {
    let floating = Floating::new();

    let starts_floating: bool = unsafe {
        from_glib(gobject_sys::g_object_is_floating(floating.to_glib_none().0))
    };

    assert!(starts_floating);
}
