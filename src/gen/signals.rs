use self::cstringident::CStringIdent;
use super::*;

impl<'ast> ClassContext<'ast> {
    pub fn signal_trampolines(&self) -> Vec<Tokens> {
        // FIXME: signal handler trampolines like in glib-rs
        //
        // unsafe extern "C" fn signalname_trampoline<P>(this: *mut ffi::InstanceName, argname:
        // type, argname: type, f: glib_ffi:gpointer) -> type where P: IsA<InstanceName> {
        //     callback_guard!();
        //     let f: &&(Fn(&P, type, type) -> type + 'static) = transmute(f);
        //
        //     // with return value:
        // f(&InstanceName::from_glib_none(this).downcast_unchecked(),
        // &from_glib_none(argname), &from_glib_none(argname))).to_glib()
        //
        //     // without return value:
        // f(&InstanceName::from_glib_none(this).downcast_unchecked(),
        // &from_glib_none(argname), &from_glib_none(argname)))
        //
        //     // those are by-reference arguments.  For by-value arguments,
        //     from_glib(argname)
        // }
        self.signals()
            .map(|signal| {
                let signalname_trampoline = signal_trampoline_name(signal);
                let InstanceName = self.InstanceName;
                let InstanceNameFfi = self.InstanceNameFfi;
                let callback_guard = glib_callback_guard();
                let sig = &signal.sig;
                let c_inputs = sig.input_args_with_glib_types();
                let input_types = signal.sig.input_arg_types();
                let arg_names = sig.input_args_from_glib_types();
                let output = &sig.output;

                let ret = quote_cs! {
                    f(&#InstanceName::from_glib_borrow(this).downcast_unchecked(), #arg_names)
                };
                let ret = sig.ret_to_glib(ret);

                quote_cs! {
                    unsafe extern "C" fn #signalname_trampoline<P>(
                        this: *mut imp::#InstanceNameFfi,
                        #c_inputs
                        f: glib_ffi::gpointer,
                    ) -> #output
                        where
                        P: IsA<#InstanceName>,
                    {
                        #callback_guard

                        let f: &&(Fn(&P, #input_types) -> #output + 'static) = mem::transmute(f);

                        #ret
                    }
                }
            })
            .collect()
    }

    pub fn signal_declarations(&self) -> Vec<Tokens> {
        self.signals()
            .map(|signal| {
                // FIXME: we are not specifying the proper signature (return, args) for the signal
                // handler.  We need a way to translate Rust type names into G_TYPE_* ids.
                //
                // FIXME: we are not passing signal flags
                //
                // FIXME: We are not passing a class_closure, marshaller, etc.

                let get_type_fn_name = self.instance_get_type_fn_name();
                let signal_id_name = signal_id_name(&signal);
                let signal_name = CStringIdent(signal.sig.name);

                assert!(signal.sig.inputs.len() > 0);
                let n_params = (signal.sig.inputs.len() - 1) as u32;

                let param_gtypes: Vec<Path> = signal.sig.inputs.iter()
                    .skip(1) // skip &self
                    .map(|arg| {
                        if let FnArg::Arg { ref ty, .. } = arg {
                            ty.to_gtype_path()
                        } else {
                            unreachable!();
                        }
                    })
                    .collect();

                quote_cs! {
                    let param_gtypes = [#(#param_gtypes),*];

                    PRIV.#signal_id_name =
                        gobject_sys::g_signal_newv (#signal_name as *const u8 as *const i8,
                                                    #get_type_fn_name(),
                                                    gobject_sys::G_SIGNAL_RUN_FIRST,   // flags
                                                    ptr::null_mut(),                   // class_closure,
                                                    None,                              // accumulator
                                                    ptr::null_mut(),                   // accu_data
                                                    None,                              // c_marshaller,
                                                    gobject_sys::G_TYPE_NONE,          // return_type
                                                    #n_params,                         // n_params,
                                                    mut_override(param_gtypes.as_ptr())
                        );
                }
            })
            .collect()
    }

    pub fn signal_emit_methods(&self) -> Vec<Tokens> {
        self.signals()
            .map(|signal| {
                let emit_name = emit_signalname(signal);
                let signal_id_name = signal_id_name(&signal);
                let rust_params = &signal.sig.inputs;
                let rust_return_ty = &signal.sig.output;
                let signal_params = signal.sig.input_args_to_glib_values();
                let return_gtype = signal.sig.output.to_gtype_path();

                let (initialize_return_value, convert_return_value_to_rust) = match rust_return_ty {
                    Ty::Unit => (quote_cs!{}, quote_cs! { () }),

                    _ => (
                        quote_cs! {
                            gobject_sys::g_value_init(ret.to_glib_none_mut().0, #return_gtype);
                        },
                        quote_cs! {
                            if ret.type_() == Type::Invalid {
                                unreachable!();
                            } else {
                                ret.get().unwrap()
                            }
                        },
                    ),
                };

                quote_cs! {
                    #[allow(unused)]
                    fn #emit_name(#(#rust_params),*) -> #rust_return_ty {
                        // foo/imp.rs: increment()
                        let params: &[glib::Value] = &[
                            #signal_params
                        ];

                        unsafe {
                            let mut ret = glib::Value::uninitialized();

                            #initialize_return_value

                            gobject_sys::g_signal_emitv(
                                mut_override(params.as_ptr()) as *mut gobject_sys::GValue,
                                PRIV.#signal_id_name,
                                0, // detail
                                ret.to_glib_none_mut().0,
                            );

                            #convert_return_value_to_rust
                        }
                    }
                }
            })
            .collect()
    }

    pub fn signal_id_names(&self) -> Vec<Ident> {
        self.signals()
            .map(|signal| signal_id_name(signal))
            .collect()
    }

    pub fn signals(&'ast self) -> impl Iterator<Item = &'ast Signal> {
        self.class.slots.iter().filter_map(|slot| match *slot {
            Slot::Signal(ref s) => Some(s),
            _ => None,
        })
    }
}

/// From a signal called `foo`, generate `foo_signal_id`.  This is used to
/// store the signal ids from g_signal_newv() in the Class structure.
fn signal_id_name<'ast>(signal: &'ast Signal) -> Ident {
    Ident::from(format!("{}_signal_id", signal.sig.name.as_ref()))
}

/// From a signal called `foo` generate a `foo_trampoline` identifier.  This is used
/// for the functions that get passed to g_signal_connect().
pub fn signal_trampoline_name(signal: &Signal) -> Ident {
    Ident::from(format!(
        "{}_signal_handler_trampoline",
        signal.sig.name.as_ref()
    ))
}

/// From a signal called `foo` generate a `connect_foo` identifier.  This is used
/// for the public methods in the InstanceExt trait.
pub fn connect_signalname(signal: &Signal) -> Ident {
    Ident::from(format!("connect_{}", signal.sig.name.as_ref()))
}

/// From a signal called `foo` generate a `emit_foo` identifier.  This is used
/// for the user's implementations of methods.
fn emit_signalname(signal: &Signal) -> Ident {
    Ident::from(format!("emit_{}", signal.sig.name.as_ref()))
}
