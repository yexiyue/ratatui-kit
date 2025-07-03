use element::ElementOrAdapter;
use proc_macro::TokenStream;
use props::ParsedProps;
use quote::ToTokens;
use syn::DeriveInput;

use crate::with_layout_style::impl_layout_style;

mod adapter;
mod component;
mod element;
mod props;
#[cfg(feature = "router")]
mod router;
#[cfg(feature = "store")]
mod store;
mod utils;
mod with_layout_style;

#[proc_macro_derive(Props, attributes(layout))]
pub fn derive_props(item: TokenStream) -> TokenStream {
    let props = syn::parse_macro_input!(item as ParsedProps);
    props.to_token_stream().into()
}

#[proc_macro]
pub fn element(input: TokenStream) -> TokenStream {
    let element = syn::parse_macro_input!(input as ElementOrAdapter);
    element.to_token_stream().into()
}

#[proc_macro_attribute]
pub fn component(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let component = syn::parse_macro_input!(item as component::ParsedComponent);
    component.to_token_stream().into()
}

#[cfg(feature = "router")]
#[proc_macro]
pub fn routes(input: TokenStream) -> TokenStream {
    let routes = syn::parse_macro_input!(input as router::Routes);
    routes.to_token_stream().into()
}

#[cfg(feature = "store")]
#[proc_macro]
pub fn use_stores(input: TokenStream) -> TokenStream {
    let stores = syn::parse_macro_input!(input as store::UseStores);
    stores.to_token_stream().into()
}

#[cfg(feature = "store")]
#[proc_macro_derive(Store)]
pub fn derive_store(item: TokenStream) -> TokenStream {
    let store = syn::parse_macro_input!(item as store::Store);
    store.to_token_stream().into()
}

#[proc_macro_attribute]
pub fn with_layout_style(attr: TokenStream, item: TokenStream) -> TokenStream {
    let layout_style = syn::parse_macro_input!(attr as with_layout_style::ParsedLayoutStyle);
    let props = syn::parse_macro_input!(item as DeriveInput);
    impl_layout_style(&layout_style, props).into()
}
