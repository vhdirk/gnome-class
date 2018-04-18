```rust
gnome_class! {
    class Foo: Superclass {
        field1: Cell<u32>, // this is where we declare the members of the private struct
        field2: Cell<u8>   // separated by commas
    }

    // this defines the class ABI, basically
    impl Foo {
        // various kinds of methods ----------------------------------------

        pub fn static_method(&self, ...) {
            ...
        }

        virtual fn virtual_method(&self, ...) {
            ...
        }

        fn this_private_method_is_an_implementation_detail(&self) {
            // and is not exported or put in the class slots
        }
        
        // signals ----------------------------------------

        signal fn some_signal(&self, ...);

        #[signal-flags FIXME]
        signal fn with_default_handler(&self, ...) -> Bar {
            // default handler code goes here
        }

        #[accumulator(my_accumulator)] // see my_accumulator below
        signal fn sig_with_accumulator(&self, ...);
        
        // C ABI considerations ----------------------------------------

        reserve_slots(5); // decrement this when you add a method/signal, for ABI compatibility
    
        // Properties ----------------------------------------
        // See https://wiki.gnome.org/Projects/Vala/Manual/Classes#Properties for ideas
        
        #[attributes...]
        property some_property: T where T: u32 {
            get(&self) -> T {
                ...
            }

            set(&self, value: T) {
                ...
            }
        }

        #[construct]
        // #[construct_only]
        property foo: T where T: u32 {
            default() -> T {
                // warn if a non-construct property has a default() as it won't be used?
                // require construct or construct-only properties to have a default()?
                ... 
            }

            get(&self) -> T {
                ...
            }

            set(&self, value: T) {
                ...
            }
        }
    }

    // from sig_with_accumulator above
    fn my_accumulator(/* FIXME: arguments */) -> bool {
        ...
    }

    // Override methods from a parent class

    impl Superclass for Foo {
        // with the same signature as the method in the parent class
        virtual fn overriden_virtual_method(&self, ...) {
            ...
        }

        signal fn overriden_signal_handler(&self, ...) {
            ...
        }
    }
    
    // Override methods from another of the parent classes

    impl AnotherSuperclass for Foo {
        // same as above
    }

    // See https://wiki.gnome.org/Projects/Vala/Manual/Classes#Construction for syntax ideas

    // This "impl GObject" is an alternative to putting
    // initialization/destruction functions inside the "class" block.
    impl GObject for Foo {
        fn init(&self) {
            // set up initial things
        }

        #[construct_prop(name="foo-bar", arg="foobar")]
        #[construct_prop(name="eek", arg="eek")]
        fn constructor(&self, foobar: i32, eek: Eek) {
        }

        fn dispose(&self) {
        }
    }

    // should we list SomeInterface in the "class" line above?
    // Pros: makes it obvious at a glance what interfaces are implemented
    // Cons: a little duplication
    //
    // We have "impl interface" because if we just have the user's SomeInterface,
    // we don't know if it corresponds to a class whose methods we are overriding,
    // or to an interface that we want to implement.  So, with "impl interface"
    // we make this unambiguous.
    impl interface SomeInterface for Foo {
        virtual fn blah(&self, ...) {
        }
    }
    
    // FIXME: we need syntax to define new GTypeInterfaces
}
```
