# High-level Internal Representation

## Constraining Rust features to GObject features

GObject's methods may look like normal function definitions, but they
do not support all the features that full-fledged Rust functions (or
trait methods) have:  GObject doesn't support generics or attributes,
and it supports a limited set of argument types — specifically, only
types that can be represented by GObject Introspection.

So, while the AST directly uses `syn::FnArg` for function arguments in
`ast::ImplItemMethod`, we "limit" their features by creating a custom
`hir::FnArg` type that only supports the following:

```
// this is in hir/mod.rs
pub enum FnArg<'ast> {
    SelfRef(Token!(&), Token!(self)),
    Arg {
        mutbl: Option<Token![mut]>,
        name: Ident,
        ty: Ty<'ast>,
    }
}

pub enum Ty<'ast> {
    Unit,
    Char(Ident),
    Bool(Ident),
    Borrowed(Box<Ty<'ast>>),
    Integer(Ident),
    Owned(&'ast syn::Path),
}
```

That is, a function argument is either `&self` or a named argument
of a limited set of possible types, and no attributes/generics/etc.

Similarly, `hir::FnSig` only supports what GObject function signatures
support, and not everything that is present in a Rust `syn::FnSig`.


