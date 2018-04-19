// We give `ClassName` variables an identifier that uses upper-case.
#![allow(non_snake_case)]

use quote::Tokens;

mod boilerplate;
mod class;
mod cstringident;
mod imp;
mod instance_ext;
mod signals;
mod signatures;

use self::class::ClassContext;
use errors::*;
use hir::Program;

pub fn codegen(program: &Program) -> Result<Tokens> {
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
