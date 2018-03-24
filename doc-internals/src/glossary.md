# Glossary

Term                       | Meaning
---------------------------|--------
GI                         | Short for GObject Introspection.
Glib                       | A C library with common utilities like hash tables, linked lists, and portability aids.  Also contains the [GObject] system.
Glib-rs                    | The Rust bindings for [Glib] and [GObject].  They include macros and wrappers for the [GType] system.
GObject                    | An [object system][GObject] for C, used by [GTK+] and [GNOME] programs.  It adds classes to C.
GObject Introspection (GI) | A system which generates machine-readable descriptions of the API in libraries which contain GObjects.  These descriptions can be used to generate language bindings automatically.  [Overview of GObject Introspection][gi]
GType                      | A [dynamic type system for C][GType], which is the foundation for [GObject].
procedural macro           | User-supplied code that runs in the Rust compiler; it lets one extend the language with a custom parser and code generator.


[GObject]: https://developer.gnome.org/gobject/stable/
[GNOME]: https://www.gnome.org
[GTK+]: https://www.gtk.org
[Glib]: https://developer.gnome.org/glib/stable/
[GType]: https://developer.gnome.org/gobject/stable/chapter-gtype.html
[gi]: https://people.gnome.org/~federico/blog/magic-of-gobject-introspection.html
