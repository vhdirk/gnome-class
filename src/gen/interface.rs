// We give `ClassName` variables an identifier that uses upper-case.
#![allow(non_snake_case)]

use quote::Tokens;

use gen::WithSuffix;
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

    pub fn gen_interface(&self) -> Tokens {
        self.gen_boilerplate()
    }

    fn gen_boilerplate(&self) -> Tokens {
        let ModuleName = self.iface.name.with_suffix("Mod");
        let Name = &self.iface.name;
        let NameIface = self.iface.name.with_suffix("Iface");

        let slots = quote_cs! {}; // FIXME
        
        quote_cs! {
            pub mod #ModuleName {
                #![allow(non_snake_case)] // "oddly" named module above

                extern crate glib_sys as glib_ffi;
                extern crate gobject_sys as gobject_ffi;

                extern crate libc;

                pub mod imp {
                    use super::glib_ffi;
                    use super::gobject_ffi;

                    use super::libc;

                    #[repr(C)]
                    pub struct #Name(libc::c_void);

                    #[repr(C)]
                    pub struct #NameIface {
                        pub parent_iface: gobject_ffi::GTypeInterface,

                        #(#slots)*
                    }
                }
            }
        }
    }
}
