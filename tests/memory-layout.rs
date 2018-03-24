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

fn assert_n_slots_bigger_than_gobject_class<T>(n: usize)
where
    T: glib::wrapper::Wrapper,
{
    assert_eq!(
        mem::size_of::<<T as glib::wrapper::Wrapper>::GlibType>(),
        mem::size_of::<gobject_sys::GObject>()
    );
    assert_eq!(
        mem::size_of::<<T as glib::wrapper::Wrapper>::GlibClassType>(),
        mem::size_of::<gobject_sys::GObjectClass>() + n * mem::size_of::<usize>()
    );
}

#[test]
fn size_of_structs() {
    assert_n_slots_bigger_than_gobject_class::<ZeroSlots>(0);
    assert_n_slots_bigger_than_gobject_class::<OneSlot>(1);
    assert_n_slots_bigger_than_gobject_class::<TwoSlots>(2);
    assert_n_slots_bigger_than_gobject_class::<ThreeSlots>(3);
}
