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
                quote_cs! {
                    PRIV.#signal_id_name =
                        gobject_sys::g_signal_newv (#signal_name as *const u8 as *const i8,
                                                    #get_type_fn_name(),
                                                    gobject_sys::G_SIGNAL_RUN_FIRST, // flags
                                                    ptr::null_mut(),                 // class_closure,
                                                    None,                            // accumulator
                                                    ptr::null_mut(),                 // accu_data
                                                    None,                            // c_marshaller,
                                                    gobject_sys::G_TYPE_NONE,        // return_type
                                                    0,                               // n_params,
                                                    ptr::null_mut()                  // param_types
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

                quote_cs! {
                    // FIXME: include signal arguments and return value in prototype
                    #[allow(unused)]
                    fn #emit_name(&self) {
                        // foo/imp.rs: increment()
                        let params: &[glib::Value] = &[
                            (self as &glib::ToValue).to_value(),
                            // FIXME: signal arguments
                        ];

                        unsafe {
                            gobject_sys::g_signal_emitv(
                                mut_override(params.as_ptr()) as *mut gobject_sys::GValue,
                                PRIV.#signal_id_name,
                                0, // detail
                                ptr::null_mut(), // ptr to return GValue
                            );
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
