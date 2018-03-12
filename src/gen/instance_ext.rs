use super::*;
use super::signals;

impl<'ast> ClassContext<'ast> {
    /// Returns, for each method, something like
    ///
    /// ```notest
    /// fn foo(&self, arg: u32);
    /// ```
    ///
    /// Or for signals, something like
    ///
    /// ```notest
    /// fn connect_foo<F: Fn(&Self, arg: u32) + 'static>(&self, f: F) -> glib::SignalHandlerId
    /// ```
    pub fn slot_trait_fns(&self) -> Vec<Tokens> {
        self.class
            .slots
            .iter()
            .filter_map(|slot| {
                match *slot {
                    Slot::Method(Method { public: false, .. }) => None,

                    Slot::Method(Method { public: true, ref sig, .. }) |
                    Slot::VirtualMethod(VirtualMethod { ref sig, .. }) => {
                        let name = sig.name;
                        let inputs = &sig.inputs;
                        let output = &sig.output;
                        Some(quote_cs! {
                            fn #name(#(#inputs),*) -> #output;
                        })
                    }

                    Slot::Signal(ref signal) => {
                        let connect_signalname = signals::connect_signalname(signal);
                        let sig = &signal.sig;
                        let inputs = &sig.inputs[1..]; // remove the &self, because we need &Self below
                        let output = &sig.output;
                        Some(quote_cs! {
                            fn #connect_signalname<F: Fn(&Self, #(#inputs),*) -> #output + 'static>(&self, f: F) ->
                                glib::SignalHandlerId;
                        })
                    },
                }
            })
            .collect()
    }

    pub fn method_redirects(&self) -> Vec<Tokens> {
        self.class
            .slots
            .iter()
            .filter_map(|slot| {
                match *slot {
                    Slot::Method(Method { public: false, .. }) => None,

                    Slot::Method(Method { public: true, ref sig, .. }) |
                    Slot::VirtualMethod(VirtualMethod { ref sig, .. }) => {
                        let name = sig.name;
                        let ffi_name = self.method_ffi_name(name.as_ref());
                        let arg_names = sig.input_args_to_glib_types();
                        let value = quote_cs! {
                            unsafe {
                                imp::#ffi_name(self.to_glib_none().0,
                                               #arg_names)
                            }
                        };
                        let output_from = sig.ret_from_glib_fn(&value);
                        let inputs = &sig.inputs;
                        let output = &sig.output;
                        Some(quote_cs! {
                            fn #name(#(#inputs),*) -> #output {
                                #output_from
                            }
                        })
                    }

                    Slot::Signal(_) => None, // panic!("signals not implemented"),
                }
            })
            .collect()
    }

    pub fn method_ffi_name(&self, method: &str) -> Ident {
        self.exported_fn_name(method)
    }

    /*
    pub fn methods(&self) -> impl Iterator<Item = &'ast Method> {
        self.class
            .items
            .iter()
            .filter_map(|item| match *item {
                ClassItem::Method(ref m) => Some(m),
                _ => None,
            })
    }
    */
}
