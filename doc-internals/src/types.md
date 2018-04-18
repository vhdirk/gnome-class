# Type conversions between Rust and Glib

## Conversions in methods

Consider a method like this:

```rust
class Foo {
}

impl Foo {
    virtual pub fn my_method(&self, an_int: u32, a_string: &str) -> bool;
}
```

If this were C code, we would be using a prototype like this:

```C
gboolean my_method(Foo *foo, guint an_int, const char *a_string);
```

Here, the Rust types are not the same as the C-side Glib types:

* `bool` / `gboolean`
* `u32` / `guint`
* `&str` / `char *`

These conversions of values can be done with the `ToGlib` and
`FromGlib` family of traits in glib-rs.  However, we need to convert
the *types* as well, so that we can generate trampolines.

## Extern functions for methods

What does a virtual method look like in GObject?  It is a function
pointer inside a class structure.  The method above would be something
like

```C
struct FooClass {
    GObjectClass parent_class;
    
    gboolean (* my_method)(Foo *foo, guint an_int, const char *a_string);
}
```

By convention, C code implements a public function that calls this
virtual method by dereferencing the function pointer:

```C
gboolean
foo_my_method (Foo *foo, guint an_int, const char *a_string) 
{
    FooClass *klass = FOO_GET_CLASS(foo);
    
    (* klass->my_method) (foo, an_int, a_string);
}
```

This function does the following:

* Given a `foo` instance, find its class structure.

* Dereference the `klass->my_method` function pointer and call into it.
  
Language bindings expect this public function to be present: they call
into it so that the function can do its own argument checking and so
on.  The gnome-class code generator must generate an ABI-compatible
function as a `pub unsafe extern "C" fn`.  We do this in
`imp_extern_methods`:

```
#[no_mangle]
pub unsafe extern "C" fn #ffi_name(this: *mut #InstanceNameFfi,
                                   #inputs)
    -> #output
{
    #callback_guard

    let klass = (*this).get_class();
    // We unwrap() because klass.method_name is always set to a method_trampoline
    (klass.#name.as_ref().unwrap())(this, #args)
}
```

Note that this function: a) takes a raw pointer to an FFI struct for
the instance on which the method is being called; b) calls a function
pointer inside the `klass` vtable, with C types for arguments.  In
effect, this is as if we had written a C function that just calls the
function pointer inside the vtable.

# Trampolines

So far, we have a Rust method function callable from C, that calls a
function pointer with C types.  We need to do a few things to glue
this nicely to Rust code:

* Go from the `this` (a raw pointer to an FFI instance structure) in
  the function above, to a Rust `&self`.
  
* Convert C types from arguments into Rust types, with glib-rs.

This is what a **trampoline** does:  it converts the arguments and
obtains the `&self`.  We generate trampolines in
`instance_slot_trampolines`:

# Argument conversions

Given a function signature like

```
virtual pub fn my_method(&self, an_int: u32, a_string: &str) -> bool;
```

we need to generate a few things:

* The return type as a Glib type, i.e. `bool` gets translated to
  `gboolean`.
  
* Input arguments but with Glib types, i.e. `an_int: u32, a_string:
  &str` gets translated to `an_int: u32, a_string: *const libc::c_char`.
  
* Just the list of Rust types, i.e. `u32, &str` for use in a `Fn`
  closure declaration: `Fn(&Self, u32, &str)` that doesn't have
  argument names.
  
* Each argument value converted from a Rust type to a Glib type:
  `<u32 as ToGlib>::to_glib(&an_int), <&str as
  ToGlibPtr>::to_glib_none(a_string)`.
  
* Each argument value converted from a Glib type to a Rust type: `<u32
  as FromGlib<_>>::from_glib(an_int), <&str as
  FromGlibPtrBorrow<_>>::from_glib_borrow(a_string)`.
  
* etc.

Our representation of a method or signal signature is `hir::FnSig`.
It provides methods like `output_glib_type()` or
`input_args_to_glib_types()` that generate the conversions above.
This is done by wrapping the `FnSig`'s fields into helper types, and
then those helper types have `impl ToTokens`; those implementations
generate the appropriate code.
