// We give `ClassName` variables an identifier that uses upper-case.
#![allow(non_snake_case)]

use quote::Tokens;

use errors::*;
use hir::{Interface, Program};

pub struct InterfaceContext<'ast> {
    pub program: &'ast Program<'ast>,
    pub iface: &'ast Interface<'ast>,
}

impl<'ast> InterfaceContext<'ast> {
    pub fn new(program: &'ast Program, iface: &'ast Interface) -> Self {
        InterfaceContext {
            program,
            iface,
        }
    }

    pub fn gen_interface(&self) -> Result<Tokens> {
        Ok(self.gen_boilerplate())
    }

    fn gen_boilerplate(&self) -> Tokens {
        quote_cs! {
        }
    }
}
