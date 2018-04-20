// We give `ClassName` variables an identifier that uses upper-case.
#![allow(non_snake_case)]

use proc_macro2::Span;
use quote::{ToTokens, Tokens};
use syn::Ident;

use glib_utils::*;

use gen::WithSuffix;
use hir::{Class, Program};

pub struct ClassContext<'ast> {
    pub program: &'ast Program<'ast>,
    pub class: &'ast Class<'ast>,
    pub PrivateStructName: Ident,
    pub ModuleName: Ident,
    pub InstanceName: &'ast Ident,
    pub InstanceNameFfi: Ident,
    pub ClassName: Ident,
    pub PrivateClassName: Ident,
    pub ParentInstance: &'ast ToTokens,
    pub ParentInstanceFfi: &'ast Tokens,
    pub ParentClassFfi: &'ast Tokens,
    pub GObject: Tokens,
    pub GObjectFfi: Tokens,
    pub GObjectClassFfi: Tokens,
    pub InstanceExt: Ident,
}

impl<'ast> ClassContext<'ast> {
    #[cfg_attr(rustfmt, rustfmt_skip)]
    pub fn new(program: &'ast Program, class: &'ast Class) -> Self {
        // This function creates a ClassContext by generating the
        // commonly-used symbol names for the class in question, for
        // example, "FooClass" out of "Foo".

        ClassContext {
            program,
            class,
            PrivateStructName: class.name.with_suffix("Priv"),
            ModuleName: class.name.with_suffix("Mod"), // toplevel "InstanceMod" module name
            InstanceName: &class.name,
            ClassName: class.name.with_suffix("Class"),
            PrivateClassName: class.name.with_suffix("ClassPrivate"),
            ParentInstance: &class.parent,
            ParentInstanceFfi: &class.parent_ffi,
            ParentClassFfi: &class.parent_class_ffi,
            GObject: tokens_GObject(),
            GObjectFfi: tokens_GObjectFfi(),
            GObjectClassFfi: tokens_GObjectClassFfi(),
            InstanceExt: class.name.with_suffix("Ext"), // public trait with all the class's methods
            InstanceNameFfi: class.name.with_suffix("Ffi"),
        }
    }

    pub fn gen_class(&self) -> Tokens {
        self.gen_boilerplate()
    }

    pub fn exported_fn_name(&self, method_name: &str) -> Ident {
        Ident::new(
            &format!(
                "{}_{}",
                lower_case_instance_name(self.InstanceName.as_ref()),
                method_name
            ),
            Span::call_site(),
        )
    }

    pub fn instance_get_type_fn_name(&self) -> Ident {
        self.exported_fn_name("get_type")
    }
}
