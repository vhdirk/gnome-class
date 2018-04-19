# Parsing

Gnome-class obtains a `TokenStream` from the Rust compiler in the
entry point for the procedural macro, and parses that stream of tokens
into an Abstract Syntax Tree (AST).  We use the `syn` crate for the
parsing machinery:  it is able to parse arbitrary Rust code, and
allows creating new parsers for our extensions to the language.

## Overview of the Abstract Syntax Tree (AST)

The AST is defined in `src/ast.rs`.  The AST is intended to match the
user's code pretty much verbatim.  For example, consider a call like this:

```rust
gobject_gen! {
    class Counter {
        f: Cell<u32>
    }

    impl Counter {
        pub fn add(&self, x: u32) -> u32 {
            self.get_priv().f.set(self.get() + x);
            self.get()
        }

        pub fn get(&self) -> u32 {
            self.get_priv().f.get()
        }
    }
}
```

First, `f: Cell<u32>` is a member of the private structure from a GObject
point of view, hence the call to `self.get_priv()` to access it.

Then, there is the actual invocation of the `gobject_gen!` macro.  It
has two *items*, a `class` and an `impl`.  Even though Rust does not
have a `class` item by itself, we use the same terminology to indicate
that this is a toplevel thing in the user's code.  (FIXME: replace
"thing" with something more meaningful?)

The contents of the `gobject_gen!` invocation will be parsed into the
following; see `src/ast.rs` for the actual definitions of these
structs/enums:

```
Program {
    items: [
        Item::Class(
            Class {
                name: Ident("Counter"),
                extends: None,
                fields: FieldsNamed {
                  brace_token: Brace,
                  named: Punctuated {...}
                }
            }
        ),

        Item::Impl(
            Impl {
                trait_: None,
                self_path: Ident("Counter"),
                items: [
                    ImplItem {
                        attrs: [empty vector],
                        node: ImplItemKind::Method(
                            ImplItemMethod {
                                public:   true,
                                virtual_: false,
                                signal:   false,
                                name:     Ident("add"),
                                inputs:   Punctuated {...},
                                output:   ReturnType, // u32
                                body:     Some(Block {...}),
                            }
                        ),

                        node: ImplItemKind::Method(
                            ImplItemMethod {
                                public:   true,
                                virtual_: false,
                                signal:   false,
                                name:     Ident("get"),
                                inputs:   Punctuated {...},
                                output:   ReturnType, // u32
                                body:     Some(Block {...}),
                            }
                        ),
                    }
                ],
            }
        ),
    ],
}
```

Whew!  Fortunately, within the parsing functions we only need to deal
with one thing at a time, and not the entire tree of code.

In summary:  the macro call that looks like

```
gobject_gen! {
    class Counter {
        ... PrivateField definitions ...
    }

    impl Counter {
        ... two method definitions ...
    }
}
```

gets parsed into

```
Program {
    items: [
        Item::Class(
            Class {
                name: Ident("Counter"),
                items: [ 
                    ... one PrivateField declaring
                        an f member of type Cell<u32> ...
                ]
            }
        ),

        Item::Impl(
            Impl {
                self_path: Ident("Counter"),
                items: [ 
                    ... two ImplItemKind::Method ...
                ],
            }
        ),
    ],
}
```

i.e. a `Program` with two items, an `Item::Class` and an
`Item::Impl`.  In turn, each of these items has a detailed description
of the corresponding constructs.

## The parsing process

Gnome-class uses the `syn` crate to parse a `TokenStream` into our AST
structures.  To define a parser for `SomeStruct`, one creates an `impl
Synom for SomeStruct`.  The `Synom` trait has a `parse` method; Syn
provides a set of *parser combinators* that let one "fill out" the
resulting structs by recursively parsing their fields.

Parser combinators are recursive-descent parsers that let one compose
big parsers from small parsers.  Syn implements parser combinators
with macros similar to the `nom` crate.  We won't go into a full
description of how syn works here, and just focus on the peculiarities
of gnome-class. (FIXME: link to syn/nom docs)

The parsing code — the bunch of `impl Synom` and parser combinators
that gnome-class uses — is in `parser/mod.rs`.

We define parsers for the constructs in the `gobject_gen!` macro that
are not normally part of Rust, like the `class` item and the `signal`
keyword.  In the deep part of these structures, we use plain Syn
structs like `syn::FnArg` to represent function arguments, or
`syn::Ident` for identifiers.



