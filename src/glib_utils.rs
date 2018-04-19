#![allow(non_snake_case)]

use ast;
use quote::{ToTokens, Tokens};

pub fn tokens_GObject() -> Tokens {
    quote_cs! { glib::Object }
}

pub fn tokens_GObjectFfi() -> Tokens {
    quote_cs! { gobject_ffi::GObject }
}

pub fn tokens_GObjectClassFfi() -> Tokens {
    quote_cs! { gobject_ffi::GObjectClass }
}

pub fn tokens_ParentInstance(class: &ast::Class) -> Tokens {
    class
        .extends
        .as_ref()
        .map(|path| {
            let mut tokens = Tokens::new();
            path.to_tokens(&mut tokens);
            tokens
        })
        .unwrap_or_else(|| tokens_GObject())
}

pub fn tokens_ParentInstanceFfi(class: &ast::Class) -> Tokens {
    let ParentInstance = tokens_ParentInstance(class);
    quote_cs! {
        <#ParentInstance as glib::wrapper::Wrapper>::GlibType
    }
}

pub fn tokens_ParentClassFfi(class: &ast::Class) -> Tokens {
    let ParentInstance = tokens_ParentInstance(class);
    quote_cs! {
        <#ParentInstance as glib::wrapper::Wrapper>::GlibClassType
    }
}

pub fn glib_callback_guard() -> Tokens {
    // FIXME: remove this function if we formally declare that
    // gnome-class will require Rust 1.24 or later?  That version made
    // glib::CallbackGuard obsolete.
    quote_cs! {
        #[allow(deprecated)]
        let _guard = glib::CallbackGuard::new();
    }
}

pub fn lower_case_instance_name(instance_name: &str) -> String {
    let mut char_iterator = instance_name.char_indices();
    let mut start_index = match char_iterator.next() {
        Some((index, _)) => index,
        None => return String::new(),
    };

    let mut parts = Vec::new();
    'outer: loop {
        // If the next bit starts with a sequence of uppercase characters, we include them
        // all. This is done in order to have a better default behavior with class names that
        // contains acronyms. See the `lower_cases_with_sequential_uppercase_characters` test
        // below.
        let mut found_non_uppercase_character = false;

        for (end_index, character) in &mut char_iterator {
            let character_is_uppercase = character.is_uppercase();
            if found_non_uppercase_character && character_is_uppercase {
                parts.push(instance_name[start_index..end_index].to_lowercase());
                start_index = end_index;
                continue 'outer;
            } else {
                found_non_uppercase_character |= !character_is_uppercase;
            }
        }

        parts.push(instance_name[start_index..].to_lowercase());
        break 'outer;
    }

    parts.join("_")
}

pub mod tests {
    use super::*;

    pub fn run() {
        lower_cases_simple_names();
        lower_cases_non_ascii_names();
        lower_cases_with_sequential_uppercase_characters();
    }

    fn lower_cases_simple_names() {
        assert_eq!("foo", lower_case_instance_name("Foo"));
        assert_eq!(
            "snake_case_sliding_through_the_grass",
            lower_case_instance_name("SnakeCaseSlidingThroughTheGrass")
        );
        assert_eq!("", lower_case_instance_name(""));
        assert_eq!("ifyoureallywantto", lower_case_instance_name("ifyoureallywantto"));
        assert_eq!("if_you_really_want_to", lower_case_instance_name("if_you_really_want_to"));
    }

    fn lower_cases_non_ascii_names() {
        assert_eq!("y̆es", lower_case_instance_name("Y̆es"));
        assert_eq!("trying_this_y̆es_y̆es", lower_case_instance_name("TryingThisY̆esY̆es"));
        assert_eq!("y̆es_y̆es_trying_this", lower_case_instance_name("Y̆esY̆esTryingThis"));
    }

    fn lower_cases_with_sequential_uppercase_characters() {
        assert_eq!("gtk_rbtree", lower_case_instance_name("GtkRBTree"));
        assert_eq!("rbtree_internals", lower_case_instance_name("RBTreeInternals"));

        // This may or may not be what we want, but for now this is the behavior that we expect.
        assert_eq!("gtkrbtree", lower_case_instance_name("GTKRBTree"));

        assert_eq!(
            "thisisaterribleclassname",
            lower_case_instance_name("THISISATERRIBLECLASSNAME")
        );
    }
}
