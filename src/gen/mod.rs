// We give `ClassName` variables an identifier that uses upper-case.
#![allow(non_snake_case)]

use proc_macro2::Span;
use quote::Tokens;
use syn::Ident;

mod boilerplate;
mod class;
mod cstringident;
mod imp;
mod interface;
mod instance_ext;
mod signals;
mod signatures;

use self::class::ClassContext;
use self::interface::InterfaceContext;
use hir::Program;

pub fn codegen(program: &Program) -> Tokens {
    let class_tokens = program
        .classes
        .iter()
        .map(|class| {
            let cx = ClassContext::new(program, class);
            cx.gen_class()
        })
        .collect::<Vec<_>>();

    let interface_tokens = program
        .interfaces
        .iter()
        .map(|iface| {
            let cx = InterfaceContext::new(program, iface);
            cx.gen_interface()
        })
        .collect::<Vec<_>>();

    quote_cs! {
        #(#class_tokens)*

        #(#interface_tokens)*
    }
}

trait WithSuffix: AsRef<str> {
    fn with_suffix(&self, suffix: &str) -> Ident {
        Ident::new(
            &format!("{}{}", self.as_ref(), suffix),
            Span::call_site(),
        )
    }
}

impl WithSuffix for Ident {}
