use syn::{Field, Fields, ItemStruct, Result, punctuated::Punctuated, token::Comma};

pub fn get_fields(input: &ItemStruct) -> Result<Punctuated<Field, Comma>> {
    match &input.fields {
        Fields::Unnamed(_) => Err(syn::Error::new_spanned(
            input,
            "only named fields and unit are supported",
        )),
        Fields::Unit => Ok(Punctuated::new()),
        Fields::Named(fields) => Ok(fields.named.clone()),
    }
}
