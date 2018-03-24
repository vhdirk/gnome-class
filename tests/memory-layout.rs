#![deny(warnings)]
#![feature(proc_macro)]

extern crate gobject_gen;

#[macro_use]
extern crate glib;
use gobject_gen::gobject_gen;

extern crate gobject_sys;

use std::mem;

gobject_gen! {
    class ZeroSlots {
    }

    class OneSlot {
    }

    impl OneSlot {
        virtual fn foo(&self) {
        }

        pub fn static_method(&self) {
        }
    }

    class TwoSlots {
    }

    impl TwoSlots {
        virtual fn foo(&self) {
        }

        pub fn static_method(&self) {
        }

        signal fn bar(&self);
    }

    class ThreeSlots: TwoSlots {
    }

    impl ThreeSlots {
        signal fn baz(&self);
    }
}

#[test]
fn zero_slots() {
    assert_eq!(
        mem::size_of::<<ZeroSlots as glib::wrapper::Wrapper>::GlibType>(),
        mem::size_of::<gobject_sys::GObject>()
    );
    assert_eq!(
        mem::size_of::<<ZeroSlots as glib::wrapper::Wrapper>::GlibClassType>(),
        mem::size_of::<gobject_sys::GObjectClass>()
    );
}

#[test]
fn one_slot() {
    assert_eq!(
        mem::size_of::<<OneSlot as glib::wrapper::Wrapper>::GlibType>(),
        mem::size_of::<gobject_sys::GObject>()
    );
    assert_eq!(
        mem::size_of::<<OneSlot as glib::wrapper::Wrapper>::GlibClassType>(),
        mem::size_of::<gobject_sys::GObjectClass>() + mem::size_of::<usize>()
    );
}

#[test]
fn two_slots() {
    assert_eq!(
        mem::size_of::<<TwoSlots as glib::wrapper::Wrapper>::GlibType>(),
        mem::size_of::<gobject_sys::GObject>()
    );
    assert_eq!(
        mem::size_of::<<TwoSlots as glib::wrapper::Wrapper>::GlibClassType>(),
        mem::size_of::<gobject_sys::GObjectClass>() + 2 * mem::size_of::<usize>()
    );
}

#[test]
fn three_slots() {
    assert_eq!(
        mem::size_of::<<ThreeSlots as glib::wrapper::Wrapper>::GlibType>(),
        mem::size_of::<gobject_sys::GObject>()
    );
    assert_eq!(
        mem::size_of::<<ThreeSlots as glib::wrapper::Wrapper>::GlibClassType>(),
        mem::size_of::<gobject_sys::GObjectClass>() + 3 * mem::size_of::<usize>()
    );
}
