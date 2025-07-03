use quote::quote;
use syn::{
    Field,
    parse::{Parse, Parser},
    punctuated::Punctuated,
    token::Comma,
};

pub struct ParsedLayoutStyle {
    pub fields: Punctuated<syn::Ident, Comma>,
}

impl Parse for ParsedLayoutStyle {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut fields: Punctuated<syn::Ident, Comma> = Punctuated::parse_terminated(input)?;

        for field in &fields {
            match field.clone().to_string().as_str() {
                "margin" | "offset" | "width" | "height" | "gap" | "flex_direction"
                | "justify_content" => {}
                _ => {
                    return Err(syn::Error::new_spanned(
                        field,
                        "only `margin`, `offset`, `width`, `height`, `gap`, `flex_direction`, and `justify_content` are allowed as layout style fields",
                    ));
                }
            }
        }

        if fields.is_empty() {
            fields = Punctuated::from_iter(vec![
                syn::Ident::new("margin", input.span()),
                syn::Ident::new("offset", input.span()),
                syn::Ident::new("width", input.span()),
                syn::Ident::new("height", input.span()),
                syn::Ident::new("gap", input.span()),
                syn::Ident::new("flex_direction", input.span()),
                syn::Ident::new("justify_content", input.span()),
            ]);
        }

        Ok(Self { fields })
    }
}

pub fn impl_layout_style(
    layout_style: &ParsedLayoutStyle,
    mut ast: syn::DeriveInput,
) -> proc_macro2::TokenStream {
    let layout_style_fields = layout_style
        .fields
        .iter()
        .map(|field| match field.to_string().as_str() {
            "margin" => Field::parse_named
                .parse2(quote! { pub margin: ratatui::layout::Margin })
                .unwrap(),
            "offset" => Field::parse_named
                .parse2(quote! { pub offset: ratatui::layout::Offset })
                .unwrap(),
            "width" => Field::parse_named
                .parse2(quote! { pub width: ratatui::layout::Constraint })
                .unwrap(),
            "height" => Field::parse_named
                .parse2(quote! { pub height: ratatui::layout::Constraint})
                .unwrap(),
            "gap" => Field::parse_named.parse2(quote! { pub gap: i32 }).unwrap(),
            "flex_direction" => Field::parse_named
                .parse2(quote! { pub flex_direction: ratatui::layout::Direction })
                .unwrap(),
            "justify_content" => Field::parse_named
                .parse2(quote! { pub justify_content: ratatui::layout::Flex })
                .unwrap(),
            _ => panic!("Unknown layout style field: {field}"),
        })
        .collect::<Vec<_>>();

    let layout_style_assignments =
        layout_style
            .fields
            .iter()
            .map(|field| match field.to_string().as_str() {
                "margin" => quote! { margin: self.margin },
                "offset" => quote! { offset: self.offset },
                "width" => quote! { width: self.width },
                "height" => quote! { height: self.height },
                "gap" => quote! { gap: self.gap },
                "flex_direction" => quote! { flex_direction: self.flex_direction },
                "justify_content" => quote! { justify_content: self.justify_content },
                _ => quote! {},
            });

    match &mut ast.data {
        syn::Data::Struct(struct_data) => {
            if let syn::Fields::Named(fields) = &mut struct_data.fields {
                fields.named.extend(layout_style_fields.iter().cloned());
            }

            let struct_name = &ast.ident;

            let (impl_generics, ty_generics, where_clause) = &ast.generics.split_for_impl();

            quote! {
                #ast

                impl #impl_generics #struct_name #ty_generics #where_clause {
                    /// Returns the layout style based on the layout-related fields of this struct.
                    pub fn layout_style(&self) -> ::ratatui_kit::layout_style::LayoutStyle {
                        ::ratatui_kit::layout_style::LayoutStyle {
                            #(#layout_style_assignments,)*
                            ..Default::default()
                        }
                    }
                }
            }
        }
        _ => panic!("`with_layout_style_props` can only be used with structs "),
    }
}
