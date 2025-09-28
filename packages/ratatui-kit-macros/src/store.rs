use proc_macro2::Span;
use quote::{ToTokens, quote};
use syn::{
    Expr, Field, Fields, GenericParam, Ident, ItemStruct, Path, Type, TypePath, parse::Parse,
    punctuated::Punctuated, spanned::Spanned, token::Comma,
};

use crate::utils::get_fields;

pub struct UseStores {
    stores: Punctuated<Expr, Comma>,
}

impl Parse for UseStores {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let stores = Punctuated::<Expr, Comma>::parse_terminated(input)?;
        Ok(UseStores { stores })
    }
}

impl ToTokens for UseStores {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let stores = self.stores.iter().cloned().collect::<Vec<_>>();
        tokens.extend(quote! {
            (#(hooks.use_store(#stores)),*)
        });
    }
}

pub struct Store {
    store: ItemStruct,
}

impl Parse for Store {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let store: ItemStruct = input.parse()?;
        let first_type_params = store
            .generics
            .params
            .iter()
            .find(|params| matches!(params, GenericParam::Type(_)));

        if let Some(param) = first_type_params {
            return Err(syn::Error::new_spanned(
                param,
                "Store cannot have type parameters",
            ));
        }

        if !matches!(store.fields, Fields::Named(_)) {
            return Err(syn::Error::new_spanned(
                store.fields,
                "Store only support named fields",
            ));
        }
        let fields = get_fields(&store)?;
        for field in &fields {
            if let Type::Path(TypePath {
                path: Path { segments, .. },
                ..
            }) = &field.ty
                && let Some(i) = segments.last()
                    && i.ident == "StoreState" {
                        return Err(syn::Error::new(
                            field.ty.span(),
                            "Store cannot have StoreState type, it will be automatically added by the macro",
                        ));
                    }
        }

        Ok(Store { store })
    }
}

impl ToTokens for Store {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let name = &self.store.ident;
        let store_name = Ident::new(&format!("{name}Store"), Span::call_site());

        let vis = &self.store.vis;

        let store_fields = self
            .store
            .fields
            .iter()
            .map(|Field { vis, ident, ty, .. }| {
                quote! {
                    #vis #ident: ::ratatui_kit::StoreState<#ty>
                }
            });

        let store_fields_from = self.store.fields.iter().map(|Field { ident, .. }| {
            quote! {
                #ident: ::ratatui_kit::StoreState::new(value.#ident)
            }
        });

        let (impl_generics, ty_generics, where_clause) = self.store.generics.split_for_impl();

        let static_store_name = store_name.to_string();
        let mut new_static_store_name = String::new();
        for (index, str) in static_store_name.chars().enumerate() {
            if index == 0 {
                new_static_store_name.push_str(&str.to_uppercase().to_string());
                continue;
            } else if str.is_uppercase() {
                new_static_store_name.push_str(&format!("_{str}"));
            } else {
                new_static_store_name.push_str(&str.to_uppercase().to_string());
            }
        }

        let new_static_store_name = Ident::new(&new_static_store_name, Span::call_site());

        tokens.extend(quote! {
            #vis struct #store_name #impl_generics #where_clause{
                #(#store_fields),*
            }

            impl #impl_generics Copy for #store_name #ty_generics #where_clause {}

            impl #impl_generics Clone for #store_name #ty_generics #where_clause {
                fn clone(&self)->Self{
                    *self
                }
            }

            impl #impl_generics From<#name #ty_generics> for #store_name #ty_generics #where_clause {
                fn from(value: #name #ty_generics) -> Self {
                    Self {
                        #(#store_fields_from),*
                    }
                }
            }
            pub static #new_static_store_name: std::sync::LazyLock<#store_name #ty_generics> = std::sync::LazyLock::new(||#name::default().into());
        });
    }
}
