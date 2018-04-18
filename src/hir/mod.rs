// High-level Internal Representation of GObject artifacts
//
// Here we provide a view of the world in terms of what GObject knows:
// classes, interfaces, signals, etc.
//
// We construct this view of the world from the raw Abstract Syntax
// Tree (AST) from the previous stage.

use std::collections::HashMap;

use proc_macro::TokenStream;
use proc_macro2::{Delimiter, Group, Span, TokenTree};
use quote::{ToTokens, Tokens};
use syn::buffer::TokenBuffer;
use syn::punctuated::Punctuated;
use syn::synom::Synom;
use syn::{self, parse_str, Block, Ident, Path, ReturnType};

use super::ast;
use super::checking::*;
use super::errors::*;
use super::glib_utils::*;

pub struct Program<'ast> {
    pub classes: Classes<'ast>,
}

pub struct Classes<'ast> {
    items: HashMap<Ident, Class<'ast>>,
}
#[cfg_attr(rustfmt, rustfmt_skip)]
pub struct Class<'ast> {
    pub name: Ident, // Foo
    pub gobject_parent: bool,
    pub parent: Tokens,           // Parent
    pub parent_ffi: Tokens,       // ffi::Parent
    pub parent_class_ffi: Tokens, // ffi::ParentClass
    pub implements: Vec<Path>,    // names of GTypeInterfaces

    // pub class_private: Option<&'ast ast::PrivateStruct>

    pub private_fields: Vec<&'ast ast::Field>,

    // The order of these is important; it's the order of the slots in FooClass
    pub slots: Vec<Slot<'ast>>,
    // pub n_reserved_slots: usize,
    //
    // pub properties: Vec<Property>,
    pub overrides: HashMap<Ident, Vec<Method<'ast>>>,
}

pub enum Slot<'ast> {
    Method(Method<'ast>),
    VirtualMethod(VirtualMethod<'ast>),
    Signal(Signal<'ast>),
}

pub struct Method<'ast> {
    pub public: bool,
    pub sig: FnSig<'ast>,
    pub body: &'ast Block,
}

pub struct VirtualMethod<'ast> {
    pub sig: FnSig<'ast>,
    pub body: Option<&'ast Block>,
}

/// Represents a slot signature (method or signal).
///
/// This is different from syn::FnDecl because GObject slots only support a subset
/// of the things that Rust does.  This is encoded in our `FnArg` type for arguments
/// and the `Ty` for the return type.
///
/// `FnSig` has a number of convenience methods that return an `impl
/// ToTokens`, for when you need to emit code for different aspects of
/// the `FnSig`:  the Glib type that corresponds to the function's
/// return value, the input arguments with Glib types, the input
/// arguments converted *from* Glib types, and so on.
pub struct FnSig<'ast> {
    pub name: Ident,
    pub inputs: Vec<FnArg<'ast>>,
    pub output: Ty<'ast>,
}

pub enum FnArg<'ast> {
    SelfRef(Token!(&), Token!(self)),
    Arg {
        mutbl: Option<Token![mut]>,
        name: Ident,
        ty: Ty<'ast>,
    },
}

pub struct Signal<'ast> {
    // FIXME: signal flags
    pub sig: FnSig<'ast>,
    pub body: Option<&'ast Block>,
}

pub enum Ty<'ast> {
    Unit,
    Char(Ident),
    Bool(Ident),
    Borrowed(Box<Ty<'ast>>),
    Integer(Ident),
    Owned(&'ast syn::Path),
}

impl<'ast> Ty<'ast> {
    pub fn to_gtype_string(&self) -> &'static str {
        match *self {
            Ty::Unit => "gobject_sys::G_TYPE_NONE",
            Ty::Char(_) => "gobject_sys::G_TYPE_UINT", // <char as ToGlib>::GlibType = u32
            Ty::Bool(_) => "gobject_sys::G_TYPE_BOOLEAN",
            Ty::Borrowed(_) => unimplemented!(),

            Ty::Integer(ref ident) => match ident.as_ref() {
                "i8" => "gobject_sys::G_TYPE_CHAR",
                "i16" => unimplemented!("should we promote i16 to i32?"),
                "i32" => "gobject_sys::G_TYPE_INT",
                "i64" => "gobject_sys::G_TYPE_INT64",
                "isize" => unimplemented!(),

                "u8" => "gobject_sys::G_TYPE_UCHAR",
                "u16" => unimplemented!("should we promote u16 to u32?"),
                "u32" => "gobject_sys::G_TYPE_UINT",
                "u64" => "gobject_sys::G_TYPE_UINT64",
                "usize" => unimplemented!(),

                _ => unreachable!(),
            },

            Ty::Owned(_) => unimplemented!(),
        }
    }

    pub fn to_gtype_path(&self) -> Path {
        path_from_string(self.to_gtype_string())
    }
}

fn path_from_string(s: &str) -> Path {
    parse_str::<Path>(s).unwrap()
}

impl<'ast> Program<'ast> {
    pub fn from_ast_program(ast: &'ast ast::Program) -> Result<Program<'ast>> {
        check_program(ast)?;

        let mut classes = Classes::new();
        for class in ast.classes() {
            classes.add(class)?;
        }
        for impl_ in ast.impls() {
            classes.add_impl(impl_)?;
        }

        Ok(Program { classes })
    }
}

impl<'ast> Classes<'ast> {
    fn new() -> Classes<'ast> {
        Classes {
            items: HashMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn get(&self, name: &str) -> &Class {
        self.items.iter().find(|c| c.1.name == name).unwrap().1
    }

    fn add(&mut self, ast_class: &'ast ast::Class) -> Result<()> {
        let prev = self.items.insert(
            ast_class.name,
            Class {
                name: ast_class.name,
                gobject_parent: ast_class.extends.is_none(),
                parent: tokens_ParentInstance(ast_class),
                parent_ffi: tokens_ParentInstanceFfi(ast_class),
                parent_class_ffi: tokens_ParentClassFfi(ast_class),
                implements: Vec::new(),
                private_fields: ast_class
                    .items
                    .iter()
                    .filter_map(|i| match *i {
                        ast::ClassItem::PrivateField(ref field) => Some(field)
                    }).collect(),
                slots: Vec::new(),
                overrides: HashMap::new(),
            },
        );
        if prev.is_some() {
            bail!("redefinition of class `{}`", ast_class.name);
        }
        Ok(())
    }

    fn add_impl(&mut self, impl_: &'ast ast::Impl) -> Result<()> {
        let class = match self.items.get_mut(&impl_.self_path) {
            Some(class) => class,
            None => bail!("impl for class that doesn't exist: {}", impl_.self_path),
        };
        match *impl_ {
            ast::Impl { is_interface: false, trait_: Some(parent_class), .. } => {
                for item in impl_.items.iter() {
                    let item = match item.node {
                        ast::ImplItemKind::Method(ref m) => m,
                        ast::ImplItemKind::ReserveSlots(_) => {
                            bail!("can't reserve slots in a parent class impl");
                        }
                    };
                    if item.signal.is_some() {
                        bail!("can't implement signals for parent classes")
                    }
                    if !item.virtual_.is_some() {
                        bail!("can only implement virtual functions for parent classes")
                    }
                    if item.public.is_some() {
                        bail!("overrides are always public, no `pub` needed")
                    }
                    let method = match class.translate_method(item)? {
                        Slot::VirtualMethod(VirtualMethod {
                            sig,
                            body: Some(body),
                        }) => Method {
                            public: false,
                            sig,
                            body,
                        },
                        Slot::VirtualMethod(VirtualMethod { .. }) => {
                            bail!("overrides must provide a body for virtual methods");
                        }
                        _ => unreachable!(),
                    };
                    class
                        .overrides
                        .entry(parent_class)
                        .or_insert(Vec::new())
                        .push(method);
                }
            }

            ast::Impl { is_interface: false, trait_: None, .. } => {
                for item in impl_.items.iter() {
                    let slot = class.translate_slot(item)?;
                    class.slots.push(slot);
                }
            }

            ast::Impl { is_interface: true, trait_: Some(parent_class), .. } => {
                unimplemented!()
            }

            _ => unreachable!()
        }

        Ok(())
    }

    pub fn iter<'a>(&'a self) -> impl Iterator<Item = &'a Class> + 'a {
        self.items.values()
    }
}

impl<'ast> Class<'ast> {
    fn translate_slot(&mut self, item: &'ast ast::ImplItem) -> Result<Slot<'ast>> {
        assert_eq!(item.attrs.len(), 0); // attributes unimplemented
        match item.node {
            ast::ImplItemKind::Method(ref method) => self.translate_method(method),
            ast::ImplItemKind::ReserveSlots(ref _slots) => {
                panic!("reserve slots not implemented");
            }
        }
    }

    fn translate_method(&mut self, method: &'ast ast::ImplItemMethod) -> Result<Slot<'ast>> {
        if method.signal.is_some() {
            if method.public.is_some() {
                bail!(
                    "function `{}` is a signal so it doesn't need to be public",
                    method.name
                )
            }

            if method.virtual_.is_some() {
                bail!(
                    "function `{}` is a signal so it doesn't need to be virtual",
                    method.name
                )
            }

            let sig = self.extract_sig(method)?;
            Ok(Slot::Signal(Signal {
                // FIXME: signal flags
                sig,
                body: method.body.as_ref(),
            }))
        } else if method.virtual_.is_some() {
            if method.public.is_some() {
                bail!(
                    "function `{}` is virtual so it doesn't need to be public",
                    method.name
                )
            }
            let sig = self.extract_sig(method)?;
            Ok(Slot::VirtualMethod(VirtualMethod {
                sig,
                body: method.body.as_ref(),
            }))
        } else {
            let sig = self.extract_sig(method)?;
            Ok(Slot::Method(Method {
                sig,
                public: method.public.is_some(),
                body: method
                    .body
                    .as_ref()
                    .ok_or_else(|| format!("function `{}` requires a body", method.name))?,
            }))
        }
    }

    fn extract_sig(&mut self, method: &'ast ast::ImplItemMethod) -> Result<FnSig<'ast>> {
        Ok(FnSig {
            output: self.extract_output(&method.output)?,
            inputs: self.extract_inputs(&method.inputs)?,
            name: method.name,
        })
    }

    fn extract_output(&mut self, output: &'ast ReturnType) -> Result<Ty<'ast>> {
        match *output {
            ReturnType::Type(_, ref boxt) => self.extract_ty(boxt),
            ReturnType::Default => Ok(Ty::Unit),
        }
    }

    fn extract_inputs(
        &mut self,
        punc: &'ast Punctuated<syn::FnArg, Token!(,)>,
    ) -> Result<Vec<FnArg<'ast>>> {
        punc.iter()
            .map(|arg| match *arg {
                syn::FnArg::Captured(syn::ArgCaptured {
                    ref pat, ref ty, ..
                }) => {
                    let (name, mutbl) = match *pat {
                        syn::Pat::Ident(syn::PatIdent {
                            by_ref: None,
                            mutability: m,
                            ident,
                            subpat: None,
                        }) => (ident, m),
                        _ => bail!("only bare identifiers are allowed as argument patterns"),
                    };

                    Ok(FnArg::Arg {
                        mutbl,
                        name,
                        ty: self.extract_ty(ty)?,
                    })
                }
                syn::FnArg::SelfRef(syn::ArgSelfRef {
                    and_token,
                    lifetime: None,
                    mutability: None,
                    self_token,
                }) => Ok(FnArg::SelfRef(and_token, self_token)),
                syn::FnArg::SelfRef(syn::ArgSelfRef {
                    mutability: Some(..),
                    ..
                }) => bail!("&mut self not implemented yet"),
                syn::FnArg::SelfRef(syn::ArgSelfRef {
                    lifetime: Some(..), ..
                }) => bail!("lifetime arguments on self not implemented yet"),
                syn::FnArg::SelfValue(_) => bail!("by-value self not implemented"),
                syn::FnArg::Inferred(_) => bail!("cannot have inferred function arguments"),
                syn::FnArg::Ignored(_) => bail!("cannot have ignored function arguments"),
            })
            .collect()
    }

    fn extract_ty(&mut self, t: &'ast syn::Type) -> Result<Ty<'ast>> {
        match *t {
            syn::Type::Slice(_) => bail!("slice types not implemented yet"),
            syn::Type::Array(_) => bail!("array types not implemented yet"),
            syn::Type::Ptr(_) => bail!("ptr types not implemented yet"),
            syn::Type::Reference(syn::TypeReference {
                lifetime: Some(_), ..
            }) => bail!("borrowed types with lifetimes not implemented yet"),
            syn::Type::Reference(syn::TypeReference {
                lifetime: None,
                ref elem,
                ref mutability,
                ..
            }) => {
                if let Some(_) = *mutability {
                    bail!("mutable borrowed pointers not implemented");
                }
                let path = match **elem {
                    syn::Type::Path(syn::TypePath {
                        qself: None,
                        ref path,
                    }) => path,
                    _ => bail!("only borrowed pointers to paths supported"),
                };
                let ty = self.extract_ty_path(path)?;
                Ok(Ty::Borrowed(Box::new(ty)))
            }
            syn::Type::BareFn(_) => bail!("function pointer types not implemented yet"),
            syn::Type::Never(_) => bail!("never not implemented yet"),
            syn::Type::Tuple(syn::TypeTuple { ref elems, .. }) => {
                if elems.len() == 0 {
                    Ok(Ty::Unit)
                } else {
                    bail!("tuple types not implemented yet")
                }
            }
            syn::Type::Path(syn::TypePath { qself: Some(_), .. }) => {
                bail!("path types with qualified self (`as` syntax) not allowed")
            }
            syn::Type::Path(syn::TypePath {
                qself: None,
                ref path,
            }) => self.extract_ty_path(path),
            syn::Type::TraitObject(_) => bail!("trait objects not implemented yet"),
            syn::Type::ImplTrait(_) => bail!("trait objects not implemented yet"),
            syn::Type::Paren(syn::TypeParen { ref elem, .. }) => self.extract_ty(elem),
            syn::Type::Group(syn::TypeGroup { ref elem, .. }) => self.extract_ty(elem),
            syn::Type::Infer(_) => bail!("underscore types not allowed"),
            syn::Type::Macro(_) => bail!("type macros not allowed"),
            syn::Type::Verbatim(_) => bail!("type macros not allowed"),
        }
    }

    fn extract_ty_path(&mut self, t: &'ast syn::Path) -> Result<Ty<'ast>> {
        if t.segments.iter().any(|segment| match segment.arguments {
            syn::PathArguments::None => false,
            _ => true,
        }) {
            bail!("type or lifetime parameters not allowed")
        }
        if t.leading_colon.is_some() || t.segments.len() > 1 {
            return Ok(Ty::Owned(t));
        }

        // let ident = t.segments.get(0).item().ident;
        let ident = t.segments.first().unwrap().value().ident;

        match ident.as_ref() {
            "char" => Ok(Ty::Char(ident)),
            "bool" => Ok(Ty::Bool(ident)),
            "i8" | "i16" | "i32" | "i64" | "isize" | "u8" | "u16" | "u32" | "u64" | "usize" => {
                Ok(Ty::Integer(ident))
            }
            _other => Ok(Ty::Owned(t)),
        }
    }
}

fn make_path_glib_object() -> Path {
    let tokens = quote_cs! { glib::Object };
    let token_stream = TokenStream::from(tokens);
    let buffer = TokenBuffer::new(token_stream);
    let cursor = buffer.begin();
    Path::parse(cursor).unwrap().0
}

impl<'a> ToTokens for FnArg<'a> {
    fn to_tokens(&self, tokens: &mut Tokens) {
        match *self {
            FnArg::SelfRef(and, self_) => {
                and.to_tokens(tokens);
                self_.to_tokens(tokens);
            }
            FnArg::Arg {
                name,
                ref ty,
                mutbl,
            } => {
                mutbl.to_tokens(tokens);
                name.to_tokens(tokens);
                Token!(:)([Span::call_site()]).to_tokens(tokens);
                ty.to_tokens(tokens);
            }
        }
    }
}

impl<'a> ToTokens for Ty<'a> {
    fn to_tokens(&self, tokens: &mut Tokens) {
        match *self {
            Ty::Unit => tokens.append(TokenTree::Group(Group::new(
                Delimiter::Parenthesis,
                quote!{ () }.into(),
            ))),
            Ty::Char(tok) => tok.to_tokens(tokens),
            Ty::Bool(tok) => tok.to_tokens(tokens),
            Ty::Integer(t) => t.to_tokens(tokens),
            Ty::Borrowed(ref t) => {
                Token!(&)([Span::call_site()]).to_tokens(tokens);
                t.to_tokens(tokens)
            }
            Ty::Owned(t) => t.to_tokens(tokens),
        }
    }
}

pub mod tests {
    use super::*;

    pub fn run() {
        creates_trivial_class();
        creates_class_with_superclass();
        maps_ty_to_gtype();
    }

    fn test_class_and_superclass(raw: &str, class_name: &str, superclass_name: &str) {
        let token_stream = raw.parse::<TokenStream>().unwrap();
        let buffer = TokenBuffer::new(token_stream);
        let cursor = buffer.begin();
        let ast_program = ast::Program::parse(cursor).unwrap().0;

        let program = Program::from_ast_program(&ast_program).unwrap();

        assert!(program.classes.len() == 1);

        let class = program.classes.get(class_name);
        assert_eq!(class.name.as_ref(), class_name);
        assert_eq!(class.parent.to_string(), superclass_name);
    }

    fn creates_trivial_class() {
        let raw = "class Foo {}";

        test_class_and_superclass(raw, "Foo", "glib :: Object");
    }

    fn creates_class_with_superclass() {
        let raw = "class Foo: Bar {}";

        test_class_and_superclass(raw, "Foo", "Bar");
    }

    fn maps_ty_to_gtype() {
        assert_eq!(Ty::Unit.to_gtype_string(), "gobject_sys::G_TYPE_NONE");
        assert_eq!(
            Ty::Char(Ident::new("char", Span::call_site())).to_gtype_string(),
            "gobject_sys::G_TYPE_UINT"
        );
        assert_eq!(
            Ty::Bool(Ident::new("bool", Span::call_site())).to_gtype_string(),
            "gobject_sys::G_TYPE_BOOLEAN"
        );

        // assert_eq!(Ty::Borrowed(...).to_gtype_string(), ...);

        assert_eq!(
            Ty::Integer(Ident::new("i8", Span::call_site())).to_gtype_string(),
            "gobject_sys::G_TYPE_CHAR"
        );
        // assert_eq!(Ty::Integer(Ident::new("i16", Span::call_site())).to_gtype(), ...);
        assert_eq!(
            Ty::Integer(Ident::new("i32", Span::call_site())).to_gtype_string(),
            "gobject_sys::G_TYPE_INT"
        );
        assert_eq!(
            Ty::Integer(Ident::new("i64", Span::call_site())).to_gtype_string(),
            "gobject_sys::G_TYPE_INT64"
        );
        // assert_eq!(Ty::Integer(Ident::new("isize", Span::call_site())).to_gtype_string(), ...);

        assert_eq!(
            Ty::Integer(Ident::new("u8", Span::call_site())).to_gtype_string(),
            "gobject_sys::G_TYPE_UCHAR"
        );
        // assert_eq!(Ty::Integer(Ident::new("u16", Span::call_site())).to_gtype_string(), ...);
        assert_eq!(
            Ty::Integer(Ident::new("u32", Span::call_site())).to_gtype_string(),
            "gobject_sys::G_TYPE_UINT"
        );
        assert_eq!(
            Ty::Integer(Ident::new("u64", Span::call_site())).to_gtype_string(),
            "gobject_sys::G_TYPE_UINT64"
        );
        // assert_eq!(Ty::Integer(Ident::new("usize", Span::call_site())).to_gtype_string(), ...);

        // assert_eq!(Ty::Owned(...).to_gtype_string(), ...);
    }
}
