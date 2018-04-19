// We give `ClassName` variables an identifier that uses upper-case.
#![allow(non_snake_case)]

use proc_macro2::Span;
use quote::{ToTokens, Tokens};
use syn::{Ident, Path};

use errors::*;
use hir::*;

use ast;

mod boilerplate;
mod cstringident;
mod imp;
mod instance_ext;
mod signals;
mod signatures;

use glib_utils::*;

pub fn classes(program: &Program) -> Result<Tokens> {
    let class_tokens = program
        .classes
        .iter()
        .map(|class| {
            let cx = ClassContext::new(program, class);
            cx.gen_class()
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(quote_cs! { #(#class_tokens)* })
}

struct ClassContext<'ast> {
    program: &'ast Program<'ast>,
    class: &'ast Class<'ast>,
    PrivateStructName: Ident,
    ModuleName: Ident,
    InstanceName: &'ast Ident,
    InstanceNameFfi: Ident,
    ClassName: Ident,
    PrivateClassName: Ident,
    ParentInstance: &'ast ToTokens,
    ParentInstanceFfi: &'ast Tokens,
    ParentClassFfi: &'ast Tokens,
    GObject: Tokens,
    GObjectFfi: Tokens,
    GObjectClassFfi: Tokens,
    InstanceExt: Ident,
}

impl<'ast> ClassContext<'ast> {
    #[cfg_attr(rustfmt, rustfmt_skip)]
    pub fn new(program: &'ast Program, class: &'ast Class) -> Self {
        // This function creates a ClassContext by generating the
        // commonly-used symbol names for the class in question, for
        // example, "FooClass" out of "Foo".

        let InstanceName = &class.name;

        // If our instance name is "Foo" and we have a suffix "Bar", generates a
        // "FooBar" Ident.  These are used for the generated module name,
        // instance/class struct names, etc.
        //
        // Note that we switch the spans of all identifiers to be
        // `Span::call_site` which differs from what `syn` does upstream which
        // is to use `Span::default` (currently). This is sort of a...
        //
        // FIXME(rust-lang/rust#45934) we should be able to use vanilla upstream
        // `syn` ideally, but it's not clear how that would change, if at all
        let container_name = |suffix: &str| {
            Ident::new(
                &format!("{}{}", InstanceName.as_ref(), suffix),
                Span::call_site(),
            )
        };

        let InstanceNameFfi = container_name("Ffi");
        let PrivateStructName = container_name("Priv");
        let ModuleName = container_name("Mod"); // toplevel "InstanceMod" module name
        let ClassName = container_name("Class");
        let PrivateClassName = container_name("ClassPrivate");
        let InstanceExt = container_name("Ext"); // public trait with all the class's methods

        let GObject = tokens_GObject();
        let GObjectFfi = tokens_GObjectFfi();
        let GObjectClassFfi = tokens_GObjectClassFfi();

        let ParentInstance = &class.parent;
        let ParentInstanceFfi = &class.parent_ffi;
        let ParentClassFfi = &class.parent_class_ffi;

        ClassContext {
            program,
            class,
            PrivateStructName,
            ModuleName,
            InstanceName,
            ClassName,
            PrivateClassName,
            ParentInstance,
            ParentInstanceFfi,
            ParentClassFfi,
            GObject,
            GObjectFfi,
            GObjectClassFfi,
            InstanceExt,
            InstanceNameFfi,
        }
    }

    pub fn gen_class(&self) -> Result<Tokens> {
        Ok(self.gen_boilerplate())
    }

    fn exported_fn_name(&self, method_name: &str) -> Ident {
        Ident::new(
            &format!(
                "{}_{}",
                lower_case_instance_name(self.InstanceName.as_ref()),
                method_name
            ),
            Span::call_site(),
        )
    }

    fn instance_get_type_fn_name(&self) -> Ident {
        self.exported_fn_name("get_type")
    }
}
