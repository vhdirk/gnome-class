use ast::*;
use errors::*;

pub fn check_program(program: &Program) -> Result<()> {
    for class in get_program_classes(program) {
        check_class(class)?;
    }
    Ok(())
}

fn check_class(class: &Class) -> Result<()> {
    Ok(check_class_items(class)?)
}

fn check_class_items(_class: &Class) -> Result<()> {
    Ok(())
}

pub mod tests {
    use super::*;
    use proc_macro::TokenStream;
    use syn::buffer::TokenBuffer;
    use syn::synom::Synom;

    use ast;

    pub fn run() {
        checks_empty_class();
    }

    fn checks_empty_class() {
        let raw = "class Foo {}";

        let token_stream = raw.parse::<TokenStream>().unwrap();

        let buffer = TokenBuffer::new(token_stream);
        let cursor = buffer.begin();

        let program = ast::Program::parse(cursor).unwrap().0;

        assert!(check_program(&program).is_ok());
    }
}
