use syn::{Field, Fields, FieldsNamed, ItemStruct, Result, punctuated::Punctuated, token::Comma};

pub fn get_struct_named(input: &ItemStruct) -> Result<&Punctuated<Field, Comma>> {
    if let Fields::Named(FieldsNamed { named, .. }) = &input.fields {
        Ok(named)
    } else {
        Err(syn::Error::new_spanned(
            input,
            "only named fields are supported",
        ))
    }
}
