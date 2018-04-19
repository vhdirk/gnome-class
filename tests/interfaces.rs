#![feature(proc_macro)]

extern crate gobject_gen;
extern crate gobject_sys;

#[macro_use]
extern crate glib;

extern crate glib_sys;

extern crate libc;

use gobject_gen::gobject_gen;

gobject_gen! {
    interface Foo {
        virtual fn foo(&self);
    }
}
