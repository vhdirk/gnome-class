#![deny(warnings)]
#![feature(proc_macro)]

extern crate gobject_gen;

#[macro_use]
extern crate glib;
use gobject_gen::gobject_gen;

use std::cell::Cell;

struct PropPrivate {
    p: Cell<u32>,
    p2: Cell<u32>,
}

impl Default for PropPrivate {
    fn default() -> Self {
        PropPrivate {
            p: Cell::new(0),
            p2: Cell::new(0),
        }
    }
}

gobject_gen! {
    class ClassWithProps {
        type InstancePrivate = PropPrivate;
    }

    impl ClassWithProps {
        pub fn get(&self) -> u32 {
            self.get_priv().p.get() +
            self.get_priv().p2.get()
        }

        property myprop: T where T: u32 {
            get(&self) -> T {
                let private = self.get_priv();
                return private.p.get();
            }

            set(&self, value: T) {
                let mut private = self.get_priv();
                private.p.set(value);
            }
        }

        property prop2: T where T: u32 {
            get(&self) -> T {
                let private = self.get_priv();
                return private.p2.get();
            }

            set(&self, value: T) {
                let mut private = self.get_priv();
                private.p2.set(value);
            }
        }
    }
}

#[test]
fn test_props() {
    let obj: ClassWithProps = ClassWithProps::new();
    assert_eq!(obj.get(), 0);
}
