#![deny(warnings)]
#![feature(proc_macro)]

extern crate gobject_gen;

#[macro_use]
extern crate glib;
use gobject_gen::gobject_gen;

use std::cell::Cell;

gobject_gen! {
    class Counter {
      f: Cell<u32>,
    }

    impl Counter {
        pub fn add(&self, x: u32) -> u32 {
            self.get_priv().f.set(self.get() + x);
            self.get()
        }

        pub fn get(&self) -> u32 {
            self.get_priv().f.get()
        }
    }
}

#[test]
fn test() {
    let c: Counter = Counter::new();

    println!("Counter has value: {}", c.get());

    c.add(2);
    c.add(20);
    assert_eq!(c.get(), 22);

    println!("Counter has value: {}", c.get());
}
