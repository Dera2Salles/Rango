use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{ItemFn, Ident, LitStr, parse::{Parse, ParseStream}, Token, Result};

pub struct ViewAttr {
    pub method: Option<String>,
}

impl Parse for ViewAttr {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.is_empty() {
            return Ok(ViewAttr { method: None });
        }
        let key: Ident = input.parse()?;
        input.parse::<Token![=]>()?;
        let val: LitStr = input.parse()?;
        if key == "method" {
            Ok(ViewAttr { method: Some(val.value()) })
        } else {
            Ok(ViewAttr { method: None })
        }
    }
}

pub fn expand_view(view_attr: ViewAttr, input_fn: ItemFn) -> TokenStream2 {
    let fn_name      = &input_fn.sig.ident;
    let fn_vis       = &input_fn.vis;
    let fn_block     = &input_fn.block;
    let fn_inputs    = &input_fn.sig.inputs;
    let fn_name_meta = quote::format_ident!("{}_meta", fn_name);

    let method_router = match view_attr.method.as_deref() {
        Some("POST")   => quote! { ::rango::axum::routing::post(#fn_name) },
        Some("PUT")    => quote! { ::rango::axum::routing::put(#fn_name) },
        Some("DELETE") => quote! { ::rango::axum::routing::delete(#fn_name) },
        Some("PATCH")  => quote! { ::rango::axum::routing::patch(#fn_name) },
        _              => quote! { ::rango::axum::routing::get(#fn_name) },
    };

    quote! {
        #fn_vis async fn #fn_name(#fn_inputs) -> impl ::rango::axum::response::IntoResponse {
            #fn_block
        }

        pub fn #fn_name_meta() -> ::rango::axum::routing::MethodRouter {
            #method_router
        }
    }
}
