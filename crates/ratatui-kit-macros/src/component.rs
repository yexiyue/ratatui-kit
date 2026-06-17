use quote::{ToTokens, quote};
use syn::{
    FnArg, GenericParam, Generics, ItemFn, Pat, PatIdent, PatType, Signature, Type, WhereClause,
    WherePredicate, parse::Parse, spanned::Spanned,
};

pub struct ParsedComponent {
    f: ItemFn,
    props_type: Option<Box<Type>>,
    impl_args: Vec<proc_macro2::TokenStream>,
}

impl Parse for ParsedComponent {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let f = input.parse::<ItemFn>()?;

        let mut props_type = None;
        let mut impl_args = Vec::new();
        for arg in f.sig.inputs.iter() {
            match &arg {
                FnArg::Typed(PatType { pat, ty, .. }) => {
                    if let Pat::Ident(PatIdent { ident, .. }) = &**pat {
                        match ident.to_string().as_str() {
                            "props" | "_props" => {
                                if props_type.is_some() {
                                    return Err(syn::Error::new(
                                        arg.span(),
                                        "duplicate props argument",
                                    ));
                                }
                                match &**ty {
                                    Type::Reference(r) => {
                                        props_type = Some(r.elem.clone());
                                        impl_args.push(quote!(props));
                                    }
                                    _ => {
                                        return Err(syn::Error::new(
                                            ty.span(),
                                            "invalid `props` type (must be a reference)",
                                        ));
                                    }
                                }
                            }
                            "hooks" | "_hooks" => match &**ty {
                                Type::Reference(_) => {
                                    impl_args.push(quote!(&mut hooks));
                                }
                                Type::Path(_) => {
                                    impl_args.push(quote!(hooks));
                                }
                                _ => {
                                    return Err(syn::Error::new(
                                        ty.span(),
                                        "invalid `hooks` type (must be a reference or a value)",
                                    ));
                                }
                            },
                            _ => {
                                return Err(syn::Error::new(
                                    arg.span(),
                                    "invalid argument name, expected `props` or `hooks`",
                                ));
                            }
                        }
                    } else {
                        return Err(syn::Error::new(pat.span(), "invalid argument"));
                    }
                }
                _ => return Err(syn::Error::new(arg.span(), "invalid argument")),
            }
        }

        Ok(Self {
            f,
            props_type,
            impl_args,
        })
    }
}

impl ToTokens for ParsedComponent {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let ItemFn {
            attrs,
            vis,
            sig:
                Signature {
                    ident,
                    generics,
                    inputs,
                    output,
                    ..
                },
            block,
        } = &self.f;

        let lifetime_generics = Generics {
            params: generics
                .params
                .iter()
                .filter(|params| matches!(params, GenericParam::Lifetime(_)))
                .cloned()
                .collect(),
            where_clause: generics
                .where_clause
                .as_ref()
                .map(|where_clause| WhereClause {
                    predicates: where_clause
                        .predicates
                        .iter()
                        .filter(|predicate| matches!(predicate, WherePredicate::Lifetime(_)))
                        .cloned()
                        .collect(),
                    ..where_clause.clone()
                }),
            ..generics.clone()
        };

        let type_generics = Generics {
            params: generics
                .params
                .iter()
                .filter(|params| !matches!(params, GenericParam::Lifetime(_)))
                .cloned()
                .collect(),
            where_clause: generics
                .where_clause
                .as_ref()
                .map(|where_clause| WhereClause {
                    predicates: where_clause
                        .predicates
                        .iter()
                        .filter(|predicate| !matches!(predicate, WherePredicate::Lifetime(_)))
                        .cloned()
                        .collect(),
                    ..where_clause.clone()
                }),
            ..generics.clone()
        };

        let (lifetime_generics, _, lifetime_where_clause) = lifetime_generics.split_for_impl();
        let (impl_generics, ty_generics, where_clause) = type_generics.split_for_impl();

        let ty_generics_names = type_generics.params.iter().filter_map(|params| {
            if let GenericParam::Type(ty) = params {
                Some(ty.ident.to_token_stream())
            } else {
                None
            }
        });
        let impl_args = &self.impl_args;
        let props_type_name = self
            .props_type
            .as_ref()
            .map(|ty| ty.to_token_stream())
            .unwrap_or_else(|| quote!(::ratatui_kit::NoProps));

        tokens.extend(quote! {
            #(#attrs)*
            #vis struct #ident #impl_generics {
                _marker: std::marker::PhantomData<fn(#(#ty_generics_names),*)>,
            }

            impl #impl_generics #ident #ty_generics #where_clause{
                fn implementation #lifetime_generics (#inputs) #output #lifetime_where_clause #block
            }


            impl #impl_generics ::ratatui_kit::Component for #ident #ty_generics #where_clause{
                type Props<'a> = #props_type_name;

                fn new(props: &Self::Props<'_>) -> Self {
                    Self {
                        _marker: std::marker::PhantomData,
                    }
                }

                fn update(
                    &mut self,
                    props: &mut Self::Props<'_>,
                    mut hooks: ::ratatui_kit::Hooks,
                    updater: &mut ::ratatui_kit::ComponentUpdater,
                ) {
                    let mut element={
                        let mut hooks=hooks.with_context_stack(updater.component_context_stack());
                        Self::implementation(#(#impl_args),*).into()
                    };
                    updater.set_transparent_layout(true);
                    updater.update_children([&mut element], None);
                }
            }
        });
    }
}
