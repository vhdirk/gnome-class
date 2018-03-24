# 2. Overview

Gnome-class extends the Rust language to support a very particular
kind of classes with inheritance, for the GObject type system.
Gnome-class is implemented as a procedural macro in Rust; this means
that it runs as part of the Rust compiler itself, and parses the
user's Gnome-like code and generates Rust code for it.

## Stages

Gnome-class operates in various stages, similar to a compiler:

1. We parse the user's code.  The Rust compiler gives us a
   [`proc_macro::TokenStream`][TokenStream].  We use the [syn] crate
   to parse this stream of tokens into an **Abstract Syntax Tree (AST)**,
   which is a tree of structs that closely match the user's code.
   
1. We check the AST for semantic errors.  For example, there cannot be
   two classes defined with the same name.
   
1. We create a **High-level Internal Representation (HIR)** from the
   AST.  While the AST may contain separate items for `class Foo` and
   `impl Foo`, the HIR has a single `Class` struct who knows which
   methods are defined for it, which virtual methods have default
   implementations, etc.
   
1. We **generate Rust code** from the HIR.  This code contains
   `#[repr(C)]` structs for the class structures and instance
   structures that GObject expects.  It also contains the
   implementations of methods, and the necessary trampolines to call
   Rust methods and signal handlers from C and vice-versa.
   
## Code structure

The entry point for gnome-class is the `gobject_gen!` procedural
macro.  It is defined in `src/lib.rs`.

The AST structures are defined in `src/ast.rs`.

The parser is in `src/parser/mod.rs`.

Some of the AST validation code is in `src/checking.rs`.  Other checks
happen in the HIR.

The HIR is in `src/hir/mod.rs`.

Finally, code generation is in `src/gen/*.rs`.

[TokenStream]: https://doc.rust-lang.org/proc_macro/struct.TokenStream.html
[syn]: https://github.com/dtolnay/syn/
