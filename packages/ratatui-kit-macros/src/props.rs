use quote::{ToTokens, quote};
use syn::{ItemStruct, Result, parse::Parse};

pub struct ParsedProps {
    pub def: ItemStruct,
}

impl Parse for ParsedProps {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let input: ItemStruct = input.parse()?;

        for field in input.fields.iter() {
            if let Some(ident) = &field.ident {
                if ident == "key" {
                    return Err(syn::Error::new_spanned(
                        field,
                        "the `key` property name is reserved",
                    ));
                }
            }
        }

        Ok(Self { def: input })
    }
}

impl ToTokens for ParsedProps {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let def = &self.def;
        let name = &def.ident;
        let (impl_generics, type_generics, where_clause) = def.generics.split_for_impl();

        tokens.extend(quote! {
            unsafe impl #impl_generics ::ratatui_kit::Props for #name #type_generics #where_clause {}
        });
    }
}
