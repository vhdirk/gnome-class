// use lalrpop_intern::InternedString;
// use quote::Tokens;
use proc_macro2::Term;
use syn::punctuated::Punctuated;
use syn::{Attribute, Lit};
use syn::{Block, FieldsNamed, FnArg, Ident, Path, ReturnType, Type};

pub struct Program {
    pub items: Vec<Item>,
}

impl Program {
    pub fn classes<'a>(&'a self) -> impl Iterator<Item = &'a Class> + 'a {
        self.items.iter().filter_map(|item| match *item {
            Item::Class(ref c) => Some(c),
            _ => None,
        })
    }

    pub fn impls<'a>(&'a self) -> impl Iterator<Item = &'a Impl> + 'a {
        self.items.iter().filter_map(|item| match *item {
            Item::Impl(ref i) => Some(i),
            _ => None,
        })
    }
}

pub enum Item {
    Class(Class),
    Impl(Impl),
}

pub fn get_program_classes<'a>(program: &'a Program) -> Vec<&'a Class> {
    program
        .items
        .iter()
        .filter_map(|item| {
            if let Item::Class(ref c) = *item {
                Some(c)
            } else {
                None
            }
        })
        .collect()
}

pub struct Class {
    pub name: Ident,
    pub extends: Option<Path>,
    pub fields: FieldsNamed,
}

// similar to syn::ItemImpl
pub struct Impl {
    pub trait_: Option<Ident>,
    pub self_path: Ident,
    pub items: Vec<ImplItem>,
}

pub struct ImplItem {
    pub attrs: Vec<Attribute>,
    pub node: ImplItemKind,
}

pub enum ImplItemKind {
    Method(ImplItemMethod),
    Prop(ImplProp),
    ReserveSlots(Lit),
}

pub struct ImplItemMethod {
    pub public: Option<Term>,   // requires body
    pub virtual_: Option<Term>, // implies public, doesn't need body
    pub signal: Option<Term>,   // ignore
    pub name: Ident,
    pub inputs: Punctuated<FnArg, Token!(,)>, // must start with &self
    pub output: ReturnType,
    pub body: Option<Block>,
}

pub struct ImplProp {
    pub name: Ident,
    pub type_: Type,
    pub items: Vec<ImplPropBlock>,
}

impl ImplProp {
    pub fn getter(&self) -> Option<&ImplPropBlock> {
        self.items.iter().find(|item| match *item {
            ImplPropBlock::Getter(_) => true,
            _ => false,
        })
    }

    pub fn setter(&self) -> Option<&ImplPropBlock> {
        self.items.iter().find(|item| match *item {
            ImplPropBlock::Setter(_) => true,
            _ => false,
        })
    }
}

pub enum ImplPropBlock {
    Getter(Block),
    Setter(ImplPropSetter),
}

pub struct ImplPropSetter {
    pub param: Ident,
    pub block: Block,
}

// Mostly copied from syn's ImplItemType
pub struct InstancePrivateItem {
    pub type_token: Token!(type),
    pub eq_token: Token!(=),
    pub path: Path,
    pub semi_token: Token!(;),
}
