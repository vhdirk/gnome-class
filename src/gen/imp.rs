use quote::Tokens;
use syn::{Block, Ident};

use glib_utils::*;
use hir::{FnSig, Method, Signal, Slot, VirtualMethod};

use super::class::ClassContext;

impl<'ast> ClassContext<'ast> {
    pub fn slots(&self) -> Vec<Tokens> {
        // ABI: we are generating the imp::FooClass with the parent_class, and the slots to
        // signals/methods. This defines the C ABI for the class structure.
        //
        // FIXME: we should check that the extern "C" signatures only have types representable by C.

        self.class
            .slots
            .iter()
            .filter_map(|slot| {
                let InstanceNameFfi = &self.InstanceNameFfi;

                match *slot {
                    Slot::Method(_) => None,

                    Slot::VirtualMethod(VirtualMethod { ref sig, .. }) => {
                        let output = sig.output_glib_type();
                        let inputs = sig.input_args_with_glib_types();
                        let name = sig.name;
                        Some(quote_cs! {
                            pub #name: Option<unsafe extern "C" fn(
                                this: *mut #InstanceNameFfi,
                                #inputs
                            ) -> #output>,

                        })
                    }

                    Slot::Signal(ref signal) => {
                        let output = signal.sig.output_glib_type();
                        let inputs = signal.sig.input_args_with_glib_types();
                        let signalname = signal.sig.name;

                        Some(quote_cs! {
                            pub #signalname: Option<unsafe extern "C" fn(
                                this: *mut #InstanceNameFfi,
                                #inputs
                            ) -> #output>,
                        })
                    }
                }
            })
            .collect()
    }

    pub fn imp_slot_default_handlers(&self) -> Vec<Tokens> {
        let method = |sig: &FnSig, body: &Block, name: Option<Ident>| {
            let name = name.unwrap_or(Self::slot_impl_name(&sig.name));
            let inputs = &sig.inputs;
            let output = &sig.output;
            quote_cs! {
                fn #name(#(#inputs),*) -> #output #body
            }
        };
        let mut ret = self.class
            .slots
            .iter()
            .map(|slot| match *slot {
                Slot::Method(Method {
                    public: false,
                    ref sig,
                    body,
                }) => method(sig, body, Some(sig.name)),
                Slot::Method(Method { ref sig, body, .. })
                | Slot::VirtualMethod(VirtualMethod {
                    ref sig,
                    body: Some(body),
                    ..
                }) => method(sig, body, None),

                Slot::VirtualMethod(VirtualMethod {
                    ref sig,
                    body: None,
                    ..
                }) => {
                    let name = Self::slot_impl_name(&sig.name);
                    let inputs = &sig.inputs;
                    let output = &sig.output;
                    quote_cs! {
                        fn #name(#(#inputs),*) -> #output {
                            panic!("Called abstract method {} with no implementation", stringify!(#name));
                        }
                    }
                }

                Slot::Signal(Signal {
                    ref sig,
                    body: Some(body),
                }) => method(sig, body, None),

                Slot::Signal(Signal {
                    ref sig,
                    body: None,
                }) => {
                    let name = Self::slot_impl_name(&sig.name);
                    let inputs = &sig.inputs;
                    let output = &sig.output;
                    quote_cs! {
                        #[allow(unused_variables)] // since none of the inputs will be used
                        fn #name(#(#inputs),*) -> #output {
                            panic!("Called default signal handler {} with no implementation", stringify!(#name));
                        }
                    }
                }
            })
            .collect::<Vec<_>>();

        ret.extend(
            self.class
                .overrides
                .values()
                .flat_map(|m| m.iter())
                .map(|m| method(&m.sig, m.body, None)),
        );

        return ret;
    }

    // If you rename this function, or move stuff out of it, please
    // update doc-internals/src/types.md
    pub fn instance_slot_trampolines(&self) -> Vec<Tokens> {
        let callback_guard = glib_callback_guard();
        let InstanceName = self.InstanceName;
        let InstanceNameFfi = self.InstanceNameFfi;
        let tokens = |sig: &FnSig, parent_class: Option<Ident>| {
            let trampoline_name = Self::slot_trampoline_name(&sig.name);
            let method_impl_name = Self::slot_impl_name(&sig.name);
            let inputs = sig.input_args_with_glib_types();
            let arg_names = sig.input_args_from_glib_types();

            let ret = quote_cs! { instance.#method_impl_name(#arg_names) };
            let ret = sig.ret_to_glib(ret);
            let output = sig.output_glib_type();
            let receiver_instance = match parent_class {
                Some(parent) => {
                    quote_cs! { <#parent as glib::wrapper::Wrapper>::GlibType }
                }
                None => quote_cs! { #InstanceNameFfi },
            };
            quote_cs! {
                unsafe extern "C" fn #trampoline_name(
                    this: *mut #receiver_instance,
                    #inputs
                )
                    -> #output
                {
                    #callback_guard

                    let this = this as *mut #InstanceNameFfi;
                    let instance: &super::#InstanceName = &from_glib_borrow(this);
                    #ret
                }
            }
        };
        let mut ret = self.class
            .slots
            .iter()
            .filter_map(|slot| match *slot {
                Slot::Method(_) => None,

                Slot::VirtualMethod(VirtualMethod { ref sig, .. }) => Some(tokens(sig, None)),

                Slot::Signal(ref signal) => Some(tokens(&signal.sig, None)),
            })
            .collect::<Vec<_>>();

        ret.extend(
            self.class
                .overrides
                .iter()
                .flat_map(|(&p, methods)| methods.iter().map(move |m| (p, m)))
                .map(|(parent_class, method)| {
                    // TODO: does the name here need mangling with the parent class?
                    tokens(&method.sig, Some(parent_class))
                }),
        );

        return ret;
    }

    // If you rename this function, or move stuff out of it, please
    // update doc-internals/src/types.md
    pub fn imp_extern_methods(&self) -> Vec<Tokens> {
        let InstanceName = self.InstanceName;
        let InstanceNameFfi = self.InstanceNameFfi;
        let callback_guard = glib_callback_guard();
        self.class
            .slots
            .iter()
            .filter_map(|slot| {
                match *slot {
                    Slot::Method(Method { public: false, .. }) => None, /* these don't get exposed in the C API */

                    Slot::Method(Method {
                        public: true,
                        ref sig,
                        ..
                    }) => {
                        let ffi_name = self.method_ffi_name(sig.name.as_ref());
                        let method_impl_name = Self::slot_impl_name(&sig.name);
                        let inputs = sig.input_args_with_glib_types();
                        let args = sig.input_args_from_glib_types();
                        let ret = quote_cs! { instance.#method_impl_name(#args) };
                        let ret = sig.ret_to_glib(ret);
                        let output = sig.output_glib_type();
                        Some(quote_cs! {
                            #[no_mangle]
                            pub unsafe extern "C" fn #ffi_name(this: *mut #InstanceNameFfi,
                                                               #inputs)
                                -> #output
                            {
                                #callback_guard

                                let instance: &super::#InstanceName = &from_glib_borrow(this);
                                #ret
                            }
                        })
                    }

                    Slot::VirtualMethod(VirtualMethod { ref sig, .. }) => {
                        let name = sig.name;
                        let ffi_name = self.method_ffi_name(sig.name.as_ref());
                        let inputs = sig.input_args_with_glib_types();
                        let args = sig.input_arg_names();
                        let output = sig.output_glib_type();
                        Some(quote_cs! {
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
                        })
                    }

                    Slot::Signal(_) => None, // these don't get exposed in the C API
                }
            })
            .collect()
    }

    fn slot_trampoline_name(slot_name: &Ident) -> Ident {
        Ident::from(format!("{}_slot_trampoline", slot_name.as_ref()))
    }

    fn slot_impl_name(slot_name: &Ident) -> Ident {
        Ident::from(format!("{}_impl", slot_name.as_ref()))
    }

    pub fn slot_assignments(&self) -> Vec<Tokens> {
        let InstanceNameFfi = &self.InstanceNameFfi;
        let mut ret = self.class
            .slots
            .iter()
            .filter_map(|slot| match *slot {
                Slot::Method(_) => None,

                Slot::VirtualMethod(VirtualMethod { ref sig, .. }) => {
                    let name = sig.name;
                    let trampoline_name = Self::slot_trampoline_name(&sig.name);

                    Some(quote_cs! {
                        klass.#name = Some(#InstanceNameFfi::#trampoline_name);
                    })
                }

                Slot::Signal(ref signal) => {
                    let signalname = signal.sig.name;
                    let trampoline_name = Self::slot_trampoline_name(&signalname);

                    Some(quote_cs! {
                        klass.#signalname = Some(#InstanceNameFfi::#trampoline_name);
                    })
                }
            })
            .collect::<Vec<_>>();

        for (parent_class, methods) in self.class.overrides.iter() {
            for method in methods.iter() {
                let name = method.sig.name;
                let trampoline_name = Self::slot_trampoline_name(&method.sig.name);

                ret.push(quote_cs! {
                    (
                        *(klass as *mut _ as *mut <
                           #parent_class as glib::wrapper::Wrapper
                        >::GlibClassType)
                    ).#name = Some(#InstanceNameFfi::#trampoline_name);
                });
            }
        }

        return ret;
    }

    pub fn imp_new_fn_name(&self) -> Ident {
        self.exported_fn_name("new")
    }

    pub fn register_instance_private(&self) -> Tokens {
        let PrivateStructName = self.PrivateStructName;

        quote_cs! {
            // This is an Option<_> so that we can replace its value with None on finalize() to
            // release all memory it holds
            gobject_ffi::g_type_class_add_private(klass, mem::size_of::<Option<#PrivateStructName>>());
        }
    }

    pub fn get_priv_fn(&self) -> Tokens {
        let PrivateStructName = self.PrivateStructName;
        let InstanceNameFfi = self.InstanceNameFfi;
        let get_type_fn_name = self.instance_get_type_fn_name();

        quote_cs! {
            #[allow(dead_code)]
            fn get_priv(&self) -> &#PrivateStructName {
                unsafe {
                    let _private = gobject_ffi::g_type_instance_get_private(
                        <Self as ToGlibPtr<*mut #InstanceNameFfi>>::to_glib_none(self).0 as *mut gobject_ffi::GTypeInstance,
                        #get_type_fn_name(),
                    ) as *const Option<#PrivateStructName>;

                    (&*_private).as_ref().unwrap()
                }
            }
        }
    }

    pub fn init_priv_with_default(&self) -> Tokens {
        let PrivateStructName = self.PrivateStructName;
        let get_type_fn_name = self.instance_get_type_fn_name();

        quote_cs! {
            let _private = gobject_ffi::g_type_instance_get_private(
                obj,
                #get_type_fn_name()
            ) as *mut Option<#PrivateStructName>;

            // Here we initialize the private data.  GObject gives it to us all zero-initialized
            // but we don't really want to have any Drop impls run here so just overwrite the
            // data.
            ptr::write(_private, Some(<#PrivateStructName as Default>::default()));
        }
    }

    pub fn free_instance_private(&self) -> Tokens {
        let PrivateStructName = self.PrivateStructName;
        let get_type_fn_name = self.instance_get_type_fn_name();

        quote_cs! {
            let _private = gobject_ffi::g_type_instance_get_private(
                obj as *mut gobject_ffi::GTypeInstance,
                #get_type_fn_name(),
            ) as *mut Option<#PrivateStructName>;

            // Drop contents of private data by replacing its
            // Option container with None
            let _ = (*_private).take();
        }
    }

    pub fn properties_enum(&self) -> Tokens {
        if self.class.properties.len() == 0 {
            return quote_cs!{};
        }

        let properties: Vec<Tokens> = self.class
            .properties
            .iter()
            .enumerate()
            .map(|(i, prop)| {
                let name = prop.name;
                let index = (i as u32) + 1;
                quote_cs! { #name = #index }
            })
            .collect();

        quote_cs! {
            #[repr(u32)]
            enum Properties {
                #(#properties, )*
            }
        }
    }
}
