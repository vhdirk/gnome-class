// We give `ClassName` variables an identifier that uses upper-case.
#![allow(non_snake_case)]

use proc_macro2::{Delimiter, Group, Span, TokenTree};
use quote::{ToTokens, Tokens};
use syn::{Ident, Path};

use errors::*;
use hir::*;

mod boilerplate;
mod cstringident;
mod imp;
mod instance_ext;
mod signals;

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
    instance_private: Option<&'ast Path>,
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

        let instance_private = class.instance_private;

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
            instance_private,
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

impl<'ast> FnSig<'ast> {
    /// Generates the Glib type name of the function's return value
    ///
    /// For example, if the `FnSig` represents a `fn foo(...) ->
    /// bool`, then this function will generate something that
    /// resolves to `glib_sys::gboolean`.
    fn output_glib_type<'a>(&'a self) -> impl ToTokens + 'a {
        ToGlibType(&self.output, self)
    }

    /// Generates an argument list just with Rust types, suitable for `Fn` signatures, without
    /// `&Self`
    ///
    /// For example, if the `FnSig` represents a `fn foo(&self, a: bool, b: i32)`, then this
    /// function will generate tokens for `bool, i32`.  This is useful when generating
    /// an `Fn(&Self, bool, i32)` signature.
    ///
    /// Note that the first parameter `&Self` is omitted.  This is so that callers can
    /// emit it themselves.
    fn input_arg_types<'a>(&'a self) -> impl ToTokens + 'a {
        ArgTypes(self)
    }

    /// Generates an argument list with Glib types suitable for function prototypes, without the
    /// `&self`
    ///
    /// For example, if the `FnSig` represents a `fn foo(&self, a: bool, b: i32)`, then this
    /// function will generate tokens for `a: glib_sys::boolean, b: i32,`.  This is useful when
    /// generating a prototype for an `unsafe extern "C" fn` callable from C.
    ///
    /// Note that the first parameter `&self` is omitted.  This is so that callers
    /// can emit a suitable C pointer instead of a Rust `&self`.
    fn input_args_with_glib_types<'a>(&'a self) -> impl ToTokens + 'a {
        FnArgsWithGlibTypes(self)
    }

    /// Generates an argument list with values converted from Glib types, without the `&self`
    ///
    /// For example, if the `FnSig` represents a `fn foo(&self, a:
    /// bool, b: i32)`, then this function will generate tokens for
    /// `<bool as FromGlib<_>>::from_glib(a), b,`.  Presumably the
    /// generated tokens are being used in a function call from C to
    /// Rust.
    ///
    /// Note that the first parameter `&self` is omitted.  This is so that the caller
    /// can emit the tokens for the first argument as appropriate.
    fn input_args_from_glib_types<'a>(&'a self) -> impl ToTokens + 'a {
        ArgNamesFromGlib(&self.inputs[1..])
    }

    /// Generates an argument list with values converted to Glib types, without the `&self`
    ///
    /// For example, if the `FnSig` represents a `fn foo(&self, a:
    /// bool, b: i32)`, then this function will generate tokens for
    /// `<bool as ToGlib>::to_glib(&a), b,`.  Presumably the generated
    /// tokens are being used in a function call from Rust to C.
    ///
    /// Note that the first parameter `&self` is omitted.  This is so that the caller
    /// can emit the tokens for the first argument as appropriate.
    fn input_args_to_glib_types<'a>(&'a self) -> impl ToTokens + 'a {
        ArgNamesToGlib(&self.inputs[1..])
    }

    /// Generates a list of argument names with no type conversions, without the `&self`
    ///
    /// For example, if the `FnSig` represents a `fn foo(&self, a:
    /// bool, b: i32)`, then this function will generate tokens for
    /// `a, b,`.  This is just to pass through arguments from inside a
    /// wrapper function.
    ///
    /// Note that the first parameter `&self` is omitted.  This is so that the caller
    /// can emit the tokens for the first argument as appropriate.
    fn input_arg_names<'a>(&'a self) -> impl ToTokens + 'a {
        ArgNames(&self.inputs[1..])
    }

    /// Generates the conversion from a Rust return value into a Glib value
    ///
    /// For example, if the `FnSig` has an `output` type of `bool`,
    /// and the `tokens` correspond to `true`, this function will
    /// generate `<bool as ToGlib>::to_glib(&true)`.  This can be used
    /// by code which generates a function callable from C that wraps
    /// Rust code.
    fn ret_to_glib<'a, T: ToTokens + 'a>(&'a self, tokens: T) -> impl ToTokens + 'a {
        ToGlib(&self.output, tokens)
    }

    fn ret_from_glib_fn<'a, V: ToTokens>(&'a self, v: &'a V) -> impl ToTokens + 'a {
        let mut tokens = Tokens::new();
        v.to_tokens(&mut tokens);
        FromGlib(&self.output, tokens)
    }
}

struct ToGlibType<'ast>(&'ast Ty<'ast>, &'ast FnSig<'ast>);

impl<'ast> ToTokens for ToGlibType<'ast> {
    fn to_tokens(&self, tokens: &mut Tokens) {
        match *self.0 {
            Ty::Unit => self.0.to_tokens(tokens),
            Ty::Char(i) | Ty::Bool(i) => {
                (quote_cs! {
                    <#i as ToGlib>::GlibType
                }).to_tokens(tokens);
            }
            Ty::Borrowed(ref t) => {
                (quote_cs! {
                    <#t as GlibPtrDefault>::GlibType
                }).to_tokens(tokens);
            }
            Ty::Integer(i) => i.to_tokens(tokens),
            Ty::Owned(_) => panic!("unimplemented glib type for owned types"),
        }
    }
}

struct ToGlib<'ast, T>(&'ast Ty<'ast>, T);

impl<'ast, T: ToTokens> ToTokens for ToGlib<'ast, T> {
    fn to_tokens(&self, tokens: &mut Tokens) {
        let expr = &self.1;
        match *self.0 {
            // no conversion necessary
            Ty::Unit | Ty::Integer(_) => self.1.to_tokens(tokens),

            Ty::Char(i) | Ty::Bool(i) => {
                (quote_cs! {
                    <#i as ToGlib>::to_glib(&#expr)
                }).to_tokens(tokens);
            }
            Ty::Borrowed(ref t) => {
                (quote_cs! {
                    <#t as ToGlibPtr<_>>::to_glib_none(#expr).0
                }).to_tokens(tokens);
            }
            Ty::Owned(_) => panic!("unimplemented to glib type for owned types"),
        }
    }
}

struct FromGlib<'ast>(&'ast Ty<'ast>, Tokens);

impl<'ast> ToTokens for FromGlib<'ast> {
    fn to_tokens(&self, tokens: &mut Tokens) {
        let needs_conversion = match *self.0 {
            Ty::Unit => false, // no conversion necessary
            Ty::Char(i) | Ty::Bool(i) => {
                (quote_cs! {
                    <#i as FromGlib<_>>::from_glib
                }).to_tokens(tokens);
                true
            }
            Ty::Borrowed(ref t) => {
                (quote_cs! {
                    &<#t as FromGlibPtrBorrow<_>>::from_glib_borrow
                }).to_tokens(tokens);
                true
            }
            Ty::Integer(_) => false, // no conversion necessary
            Ty::Owned(_) => panic!("unimplemented from glib on owned types"),
        };

        if needs_conversion {
            tokens.append(TokenTree::Group(Group::new(
                Delimiter::Parenthesis,
                self.1.clone().into_tokens().into(),
            )));
        } else {
            self.1.to_tokens(tokens);
        }
    }
}

struct ArgTypes<'ast>(&'ast FnSig<'ast>);

impl<'ast> ToTokens for ArgTypes<'ast> {
    fn to_tokens(&self, tokens: &mut Tokens) {
        for arg in self.0.inputs[1..].iter() {
            match *arg {
                FnArg::Arg { ref ty, .. } => {
                    ty.to_tokens(tokens);
                    Token!(,)([Span::call_site()]).to_tokens(tokens);
                }
                FnArg::SelfRef(..) => unreachable!(),
            }
        }
    }
}

struct FnArgsWithGlibTypes<'ast>(&'ast FnSig<'ast>);

impl<'ast> ToTokens for FnArgsWithGlibTypes<'ast> {
    fn to_tokens(&self, tokens: &mut Tokens) {
        for arg in self.0.inputs[1..].iter() {
            match *arg {
                FnArg::Arg {
                    name,
                    ref ty,
                    mutbl: _,
                } => {
                    name.to_tokens(tokens);
                    Token!(:)([Span::call_site()]).to_tokens(tokens);
                    ToGlibType(ty, self.0).to_tokens(tokens);
                    Token!(,)([Span::call_site()]).to_tokens(tokens);
                }
                FnArg::SelfRef(..) => unreachable!(),
            }
        }
    }
}

struct ArgNamesFromGlib<'ast>(&'ast [FnArg<'ast>]);

impl<'ast> ToTokens for ArgNamesFromGlib<'ast> {
    fn to_tokens(&self, tokens: &mut Tokens) {
        for arg in self.0 {
            match *arg {
                FnArg::Arg {
                    ref name,
                    ref ty,
                    mutbl: _,
                } => {
                    let mut name_tokens = Tokens::new();
                    name.to_tokens(&mut name_tokens);
                    FromGlib(ty, name_tokens).to_tokens(tokens);
                    Token!(,)([Span::call_site()]).to_tokens(tokens);
                }
                FnArg::SelfRef(..) => unreachable!(),
            }
        }
    }
}

struct ArgNamesToGlib<'ast>(&'ast [FnArg<'ast>]);

impl<'ast> ToTokens for ArgNamesToGlib<'ast> {
    fn to_tokens(&self, tokens: &mut Tokens) {
        for arg in self.0 {
            match *arg {
                FnArg::Arg {
                    ref ty,
                    name,
                    mutbl: _,
                } => {
                    ToGlib(ty, name).to_tokens(tokens);
                    Token!(,)([Span::call_site()]).to_tokens(tokens);
                }
                FnArg::SelfRef(..) => unreachable!(),
            }
        }
    }
}

struct ArgNames<'ast>(&'ast [FnArg<'ast>]);

impl<'ast> ToTokens for ArgNames<'ast> {
    fn to_tokens(&self, tokens: &mut Tokens) {
        for arg in self.0 {
            match *arg {
                FnArg::Arg { name, .. } => {
                    name.to_tokens(tokens);
                    Token!(,)([Span::call_site()]).to_tokens(tokens);
                }
                FnArg::SelfRef(..) => unreachable!(),
            }
        }
    }
}
