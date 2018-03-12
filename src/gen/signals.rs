use super::*;
use self::cstringident::CStringIdent;

impl<'ast> ClassContext<'ast> {
    /// Generates connect_signalname() function prototypes for the InstanceExt trait
    pub fn signal_connect_trait_fns(&self) -> Vec<Tokens> {
        // FIXME: methods to connect to signals like in glib-rs
        //
        // fn connect_signalname<F: Fn(&Self, type, type) -> type + 'static>(&self, f: F) -> glib::SignalHandlerId;
        Vec::new()
    }

    /// Generates connect_signalname() impls for the InstanceExt implementation
    pub fn signal_connect_impl_fns(&self) -> Vec<Tokens> {
        // FIXME: methods to connect to signals like in glib-rs
        //
        // fn connect_signalname<F: Fn(&Self, type, type) -> type + 'static>(&self, f: F) -> glib::SignalHandlerId {
        //     unsafe {
        //         let f: Box_<Box_<Fn(&Self, type, type) -> type + 'static>> = Box_::new(Box_::new(f));
        //         connect(self.to_glib_none().0, "signalname",
        //             transmute(signalname_trampoline::<Self> as usize), Box_::into_raw(f) as *mut _)
        //     }
        // }
        Vec::new()
    }

    pub fn signal_trampolines(&self) -> Vec<Tokens> {
        // FIXME: signal handler trampolines like in glib-rs
        //
        // unsafe extern "C" fn signalname_trampoline<P>(this: *mut ffi::InstanceName, argname: type, argname: type, f: glib_ffi:gpointer) -> type
        // where P: IsA<InstanceName> {
        //     callback_guard!();
        //     let f: &&(Fn(&P, type, type) -> type + 'static) = transmute(f);
        //
        //     // with return value:
        //     f(&InstanceName::from_glib_none(this).downcast_unchecked(), &from_glib_none(argname), &from_glib_none(argname))).to_glib()
        //
        //     // without return value:
        //     f(&InstanceName::from_glib_none(this).downcast_unchecked(), &from_glib_none(argname), &from_glib_none(argname)))
        //
        //     // those are by-reference arguments.  For by-value arguments,
        //     from_glib(argname)
        // }
        Vec::new()
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

    pub fn signal_id_names(&self) -> Vec<Ident> {
        self.signals()
            .map(|signal| signal_id_name (signal))
            .collect()
    }

    pub fn signals(&'ast self) -> impl Iterator<Item = &'ast Signal> {
        self.class
            .slots
            .iter()
            .filter_map(|slot| match *slot {
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

