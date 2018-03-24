# 1. Introduction

[GObject][gobject] is the C-based object system for [GTK+][gtk] and
[GNOME][gnome] programs.  Gnome-class is a Rust crate that lets you
write [GObject][gobject] implementations in Rust with a convenient
syntax.

## Quick overview of GObject

While C does not have objects or classes by itself, GObject makes it
possible to write object-oriented C programs.  The GObject library
defines the GObject type system, which supports features like:

* Classes and subclasses with single inheritance.

* A class may implement multiple interfaces.

* Virtual methods and static methods.

* Signals, which are events emitted by objects (nothing to do with
  Unix signals).  For example, a Button object may emit a "clicked"
  signal.

* Properties, or getters/setters for values on objects, with
  notification of changes.
  
* Introspection — asking the type system about which classes are
  registered and what features they contain.

Writing GObjects in C normally requires that you write an
uncomfortable amount of [boilerplate code][boilerplate] to do things
like register a new class, define its methods, register object signals
and properties, etc.  Due to the nature of C, many operations are not
type-safe and depend on correct pointer casts, or on knowing the types
that you should really be passing to varargs functions, which are not
checked by the compiler.

## Why gnome-class?

Since GObject is a C library, it can be called from Rust through a
bunch of `extern "C"` functions.  One could write `#[repr(C)]` structs
in Rust that match the layout that GObject functions expect:  for
example, those structs could have fields with function pointers to
virtual method implementations.

Doing things that way is very verbose and cumbersome:  it means using
Rust as if it were C, and dealing with GObject's idiosyncrasies in a
non-native language.

The fundamental goal of this Gnome-class crate is to let you write
GObject implementations in Rust with minimal or no boilerplate, and
with compile-time type safety all along.  The goal is to require no
`unsafe` code on your part.

## How is gnome-class different from glib-rs?

[Glib-rs][glib-rs] is the fundamental building block in the
[Gtk-rs][gtk-rs] ecosystem.  It provides the basic wrappers to write a
Rust-friendly language binding to GObject-based libraries.

However, glib-rs is a **language binding** to GObject-based libraries.
It lets you **use** GObject libraries from Rust; it does not let you
**implement** new such libraries easily.  That is the purpose of
gnome-class:  we generate GObject-compatible code, from Rust, and that
has the same kind of Rust API as a "traditional" library would have if
wrapped with glib-rs.

## Goals of gnome-class

* Let users write new GObject classes completely in Rust, with no
  unsafe code, and no boilerplate.

* Generate GObject implementations that look exactly like C GObjects
  from the outside.  The generated GObjects should be callable from C
  or other languages in exactly the same way as traditional GTK+/GNOME
  libraries.

* Automatically emit [GObject Introspection][gi] information so that the
  generated objects can be consumed by language bindings.

* In the end, we aim to make it compelling for users to *not* write
  new GObject libraries in C, but rather to give them an "obvious" way
  to it in Rust.  This should ensure higher-quality, safer code for
  GNOME's general-purpose libraries, while maintaining backwards
  compatibility with all the GObject-based infrastructure we have.

## About this document

This is an overview of how gnome-class works.  It is implemented as a
Rust procedural macro that extends the Rust language with
GObject-friendly constructs:  for example, Rust does not have
"`class`" or "`signal`" keywords, but gnome-class adds them to the
language.

* It will be helpful for you to know a bit of how GObject works.  Read
  the [GObject Tutorial][gobject-tutorial] in the [GObject Reference
  Guide][gobject-reference].  You can also read the source code for
  libraries which implement GObjects, for example, [GTK+][gtk-source].

* Please read this [overview of how GObject Introspection works][gi].
  This will give you a good idea of what we want to generate at some
  point with gnome-class.

* While it will be helpful to have some basic understanding of
  compilers (parsers, analyzers, code generators), this is not
  necessary.  This document will explain what you need to know for
  gnome-class's internals.
  
If you find any issues with this document, like missing information,
unclear explanations, or anything at all, please [file an
issue][issues] in the gnome-class issue tracker, or even submit a merge
request with a correction!

[gobject]: https://developer.gnome.org/platform-overview/unstable/tech-gobject.html.en
[boilerplate]: https://developer.gnome.org/SubclassGObject/
[gtk]: https://www.gtk.org/
[gnome]: https://www.gnome.org/
[glib-rs]: http://gtk-rs.org/docs/glib/
[gtk-rs]: http://gtk-rs.org/
[gi]: https://people.gnome.org/~federico/blog/magic-of-gobject-introspection.html
[gobject-tutorial]: https://developer.gnome.org/gobject/stable/howto-gobject.html
[gobject-reference]: https://developer.gnome.org/gobject/stable/index.html
[gtk-source]: https://gitlab.gnome.org/GNOME/gtk
[issues]: https://gitlab.gnome.org/federico/gnome-class/issues
