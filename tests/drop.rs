#![deny(warnings)]
#![feature(proc_macro)]

extern crate gobject_gen;

#[macro_use]
extern crate glib;
use gobject_gen::gobject_gen;

use std::cell::RefCell;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[derive(Clone, Default)]
pub struct DropCounter {
    counter: Arc<AtomicUsize>,
}

impl DropCounter {
    pub fn new() -> Self {
        DropCounter {
            counter: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn get(&self) -> usize {
        self.counter.load(Ordering::SeqCst)
    }
}

impl Drop for DropCounter {
    fn drop(&mut self) {
        self.counter.fetch_add(1, Ordering::SeqCst);
    }
}

gobject_gen! {
    class Dummy {
        dc: RefCell<DropCounter>,
    }

    impl Dummy {
        pub fn set_dc(&self, dc: usize) {
            let mut self_dc = self.get_priv().dc.borrow_mut();
            let dc = unsafe {
                DropCounter { counter: Arc::from_raw(dc as *const _) }
            };
            *self_dc = dc;
        }
    }
}

#[test]
fn check() {
    let dc = DropCounter::new();

    {
        let c: Dummy = Dummy::new();
        c.set_dc(Arc::into_raw(dc.counter.clone()) as usize);
        println!("Drop counter has value: {}", dc.get());
        assert_eq!(dc.get(), 0);
    }

    println!("Drop counter has value: {}", dc.get());
    assert_eq!(dc.get(), 1);
}
