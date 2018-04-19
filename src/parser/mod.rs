use proc_macro::{Diagnostic, Level, TokenStream};

use proc_macro2::Term;
use syn::buffer::Cursor;
use syn::punctuated::Punctuated;
use syn::synom::{PResult, Synom};
use syn::{self, parse_error, FieldsNamed, Ident, Path};

use ast;
use errors::*;

pub fn parse_program(token_stream: TokenStream) -> Result<ast::Program> {
    syn::parse(token_stream).map_err(|e| e.into())
}

impl Synom for ast::Program {
    named!(parse -> Self, do_parse!(
        items: many0!(syn!(ast::Item)) >>
        (ast::Program {
            items: items
        })
    ));

    fn description() -> Option<&'static str> {
        Some("gobject_gen program")
    }
}

impl Synom for ast::Item {
    named!(parse -> Self, alt!(
        syn!(ast::Class) => { |x| ast::Item::Class(x) }
        |
        syn!(ast::Impl) => { |x| ast::Item::Impl(x) }
        |
        syn!(ast::Interface) => { |x| ast::Item::Interface(x) }
    ));

    fn description() -> Option<&'static str> {
        Some("item")
    }
}

// class Foo [: SuperClass [, ImplementsIface]*] {
//     struct FooPrivate {
//         ...
//     }
//
//     private_init() -> FooPrivate {
//         ...
//     }
// }
impl Synom for ast::Class {
    named!(parse -> Self, do_parse!(
        call!(keyword("class"))                                  >>
        name: syn!(Ident)                                        >>
        extends: option!(do_parse!(
            punct!(:)                                            >>
            superclass: syn!(Path)                               >>
            // FIXME: interfaces
            (superclass)))                                       >>
        fields: syn!(FieldsNamed)                                >>
        (ast::Class {
            name:    name,
            extends: extends,
            fields:   fields
        })
    ));

    fn description() -> Option<&'static str> {
        Some("class item")
    }
}

impl Synom for ast::Interface {
    named!(parse -> Self, do_parse!(
        call!(keyword("interface")) >>
        name: syn!(Ident) >>
        items_and_braces: braces!(many0!(syn!(ast::ImplItem)))  >>
        (ast::Interface {
            name: name,
            items: items_and_braces.1,
        })
    ));

    fn description() -> Option<&'static str> {
        Some("interface item")
    }
}

impl Synom for ast::Impl {
    named!(parse -> Self, do_parse!(
        keyword!(impl) >>
        interface: option!(call!(keyword("interface"))) >>
        trait_: option!(do_parse!(
            path: syn!(Ident) >>
            keyword!(for) >>
            (path)
        )) >>
        self_path: syn!(Ident) >>
        body: braces!(many0!(syn!(ast::ImplItem))) >>
        (ast::Impl {
            is_interface: interface.is_some(),
            trait_: trait_,
            self_path: self_path,
            items: body.1
        })
    ));

    fn description() -> Option<&'static str> {
        Some("impl item")
    }
}

impl Synom for ast::ImplItem {
    named!(parse -> Self, do_parse!(
        attrs: many0!(call!(syn::Attribute::parse_outer)) >>
        node: syn!(ast::ImplItemKind) >>
        (ast::ImplItem { attrs, node })
    ));

    fn description() -> Option<&'static str> {
        Some("item inside impl")
    }
}

impl Synom for ast::ImplItemKind {
    named!(parse -> Self, alt!(
        syn!(ast::ImplItemMethod) => { |x| ast::ImplItemKind::Method(x) }
        |
        syn!(ast::ImplProp) => { |x| ast::ImplItemKind::Prop(x) }
        |
        do_parse!(
            call!(keyword("reserve_slots")) >>
            slots: parens!(syn!(syn::Lit)) >>
            (ast::ImplItemKind::ReserveSlots(slots.1))
        )
    ));

    fn description() -> Option<&'static str> {
        Some("item inside impl")
    }
}

macro_rules! error_not_ident {
    ($i:expr,) => {{
        let mut i = $i;
        if let Some((token, tts)) = i.token_tree() {
            i = tts;
            loop {
                if i.eof() {
                    break;
                }
                if let Some((tok, tts)) = i.token_tree() {
                    println!("token: {}", tok);
                    i = tts;
                }
            }
            let span = token.span().unstable();
            Diagnostic::spanned(
                span.clone(),
                Level::Error,
                format!("expected identifier, found `{}`", token),
            ).emit();
        }
        Ok(((), i))
    }};
}

named!{parse_ident -> syn::Ident,
       alt!(syn!(syn::Ident)
            | do_parse!(
                error_not_ident!() >>
                map!(reject!(), |()| ()) >>
            (Ident::from("__foo"))))
}

impl Synom for ast::ImplItemMethod {
    named!(parse -> Self, do_parse!(
        public: option!(call!(keyword("pub"))) >>
        virtual_: option!(call!(keyword("virtual"))) >>
        signal: option!(call!(keyword("signal"))) >>
        keyword!(fn) >>
        name: call!(parse_ident) >>
        params: parens!(Punctuated::parse_terminated) >>
        output: syn!(syn::ReturnType) >>
        body: alt!(
            syn!(syn::Block) => { Some }
            |
            punct!(;) => { |_| None }
        ) >>
        (ast::ImplItemMethod {
            public,
            virtual_,
            signal,
            name,
            inputs: params.1,
            output,
            body,
        })
    ));

    fn description() -> Option<&'static str> {
        Some("method or signal")
    }
}

impl Synom for ast::ImplProp {
    named!(parse -> Self, do_parse!(
        call!(keyword("property")) >>

        name: syn!(syn::Ident) >>

        punct!(:) >>
        call!(keyword("T")) >>
        call!(keyword("where")) >>
        call!(keyword("T")) >>
        punct!(:) >>

        type_: syn!(syn::Type) >>
        items_and_braces: braces!(many0!(alt!(
            do_parse!(
                call!(keyword("get")) >>
                parens!(do_parse!(
                    punct!(&) >>
                    call!(keyword("self")) >>
                    ()
                )) >>
                punct!(->) >>
                call!(keyword("T")) >>
                getter: syn!(syn::Block) >>
                (ast::ImplPropBlock::Getter(getter))
            )
            |
            do_parse!(
                call!(keyword("set")) >>
                param: parens!(do_parse!(
                    punct!(&) >>
                    call!(keyword("self")) >>
                    punct!(,) >>
                    param: syn!(syn::Ident) >>
                    punct!(:) >>
                    call!(keyword("T")) >>
                    (param)
                )) >>
                block: syn!(syn::Block) >>
                (ast::ImplPropBlock::Setter(
                    ast::ImplPropSetter{
                        param: param.1,
                        block: block
                    }
                ))
            )
        ))) >>
        (ast::ImplProp {
            name: name,
            type_: type_,
            items: items_and_braces.1,
        })
    ));

    fn description() -> Option<&'static str> {
        Some("property definition")
    }
}

/// Creates a parsing function for use with synom's call!().  For
/// example, if you need to parse a keyword "foo" as part of a bigger
/// parser, you could do this:
///
/// ```norun
/// call!(keyword("foo"))
/// ```
fn keyword<'a>(name: &'static str) -> impl Fn(Cursor<'a>) -> PResult<Term> {
    move |input: Cursor<'a>| {
        if let Some((term, rest)) = input.term() {
            if term.as_str() == name {
                return Ok((term, rest));
            }
        }
        parse_error() // FIXME: use a meaningful error message when synom allows for it
    }
}

pub mod tests {
    use super::*;
    use quote;
    use quote::ToTokens;
    use syn::parse_str;

    pub fn run() {
        parses_class_with_no_superclass();
        parses_class_with_superclass();
        parses_class_item();
        parses_plain_impl_item();
        parses_impl_item_with_trait();
        parses_class_with_private_field();
        parses_impl_interface();
        parses_interface();
    }

    fn assert_tokens_equal<T: ToTokens>(x: &T, s: &str) {
        let mut tokens = quote::Tokens::new();
        x.to_tokens(&mut tokens);
        assert_eq!(tokens.to_string(), s);
    }

    fn parses_class_with_no_superclass() {
        let raw = "class Foo {}";
        let class = parse_str::<ast::Class>(raw).unwrap();

        assert_eq!(class.name.as_ref(), "Foo");
        assert!(class.extends.is_none());
    }

    fn parses_class_with_private_field() {
        let raw = "class Foo {
          foo : u32,
          bar : u32,
          baz : u32
        \
                   }";
        let class = parse_str::<ast::Class>(raw).unwrap();

        assert_eq!(class.fields.named.len(), 3);
    }

    fn parses_class_with_superclass() {
        let raw = "class Foo: Bar {}";
        let class = parse_str::<ast::Class>(raw).unwrap();

        assert_eq!(class.name.as_ref(), "Foo");
        assert_tokens_equal(&class.extends, "Bar");
    }

    fn parses_class_item() {
        let raw = "class Foo {}";
        let item = parse_str::<ast::Item>(raw).unwrap();

        if let ast::Item::Class(class) = item {
            assert_eq!(class.name.as_ref(), "Foo");
            assert!(class.extends.is_none());
        } else {
            unreachable!();
        }
    }

    fn test_parsing_impl_item(
        raw: &str,
        trait_name: Option<&str>,
        self_name: &str,
        is_interface: bool,
    ) {
        let item = parse_str::<ast::Item>(raw).unwrap();

        if let ast::Item::Impl(ref impl_) = item {
            assert_eq!(impl_.is_interface, is_interface);

            if let Some(trait_path) = impl_.trait_ {
                assert_tokens_equal(&trait_path, trait_name.as_ref().unwrap());
            } else {
                assert!(trait_name.is_none());
            }

            assert_tokens_equal(&impl_.self_path, self_name);
        } else {
            unreachable!();
        }
    }

    fn parses_plain_impl_item() {
        test_parsing_impl_item("impl Foo {}", None, "Foo", false);
    }

    fn parses_impl_item_with_trait() {
        test_parsing_impl_item("impl Foo for Bar {}", Some("Foo"), "Bar", false);
    }

    fn parses_impl_interface() {
        test_parsing_impl_item("impl interface Foo for Bar {}", Some("Foo"), "Bar", true);
    }

    fn parses_interface() {
        let raw = "interface Foo { virtual fn bar(&self); }";
        let iface = parse_str::<ast::Interface>(raw).unwrap();

        assert_eq!(iface.name.as_ref(), "Foo");
    }
}
