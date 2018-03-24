#![feature(proc_macro)]

extern crate glib_sys;
extern crate gobject_gen;

#[macro_use]
extern crate glib;
use gobject_gen::gobject_gen;

extern crate gobject_sys;

// glib_wrapper! wants these bindings
use glib_sys as glib_ffi;
use gobject_sys as gobject_ffi;
use std::mem;
use std::ptr;

use glib::{IsA, Object};
use glib::translate::*;

// The glib crate doesn't bind GInitiallyUnowned, so let's bind it here.
glib_wrapper! {
    pub struct InitiallyUnowned(
        Object<gobject_sys::GInitiallyUnowned, gobject_sys::GInitiallyUnownedClass>
    );

    match fn {
        get_type => || gobject_sys::g_initially_unowned_get_type(),
    }
}

gobject_gen! {
    // Make an instantiable class out of InitiallyUnowned
    class Floating: InitiallyUnowned {
    }

    impl Floating {
        virtual fn frob(&self) {
            println!("hello!");
        }
    }

    // This will just have a method that doesn't take ownership of a Floating object
    class Foo {
    }

    impl Foo {
        virtual fn blah(&self, x: &Floating) {
            x.frob();
        }
    }
}

fn is_floating<T: IsA<Object>>(obj: &T) -> bool {
    from_glib(unsafe { gobject_sys::g_object_is_floating(obj.to_glib_none().0) })
}

#[test]
fn initially_unowned_is_floating() {
    let floating = Floating::new();

    assert!(is_floating(&floating));

    let foo = Foo::new();
    let foo_is_floating: bool =
        unsafe { from_glib(gobject_sys::g_object_is_floating(foo.to_glib_none().0)) };

    assert!(!is_floating(&foo));

    foo.blah(&floating);

    assert!(is_floating(&floating));
}
