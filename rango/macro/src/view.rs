use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    Ident, LitStr, Result, Token,
};

struct ViewAttribute {
    method: Option<String>,
}

impl Parse for ViewAttribute {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.is_empty() {
            return Ok(ViewAttribute { method: None });
        }

        let key: Ident = input.parse()?;
        input.parse::<Token![=]>()?;
        let value: LitStr = input.parse()?;
        if key == "method" {
            Ok(ViewAttribute {
                method: Some(value.value()),
            })
        } else {
            Ok(ViewAttribute { method: None })
        }
    }
}

#[proc_macro_attribute]
pub fn view(attribut: TokenStream, item: TokenStream) -> TokenStrean {
    let view_attribute = parse_macro_input!(attribut as ViewAttribute);
    let input_fn = parse_macro_input!(item as ItemFn);

    let fn_name = &input_fn.sig.ident;
    let fn_visibility = &input_fn.vis;
    let fn_block_content = &input_fn.block;
    let fn_args = &input_fn.inputs;

    let method_router = match view_attribute.methode.as_deref() {
        Some("POST") => quote! { ::rango::axum::routing::post(#fn_name)},
        Some("PUT") => quote! { ::rango::axum::routing::put(#fn_name)},
        Some("DELETE") => quote! { ::rango::axum::routing::delete(#fn_name)},
        Some("PATCH") => quote! { ::rango::axum::routing::patch(#fn_name)},
        _ => quote! { ::rango::axum::routing::get(#fn_name)},
    };

    let expanded_code = quote! {
        #fn_visibility async fn #fn_name(#fn_args) -> impl ::rango::axum::response::IntoResponse {
            #fn_block_content
        }

        pub fn #fn_name_meta() -> ::rango::axum::routing::MethodRouter {
            #method_router
        }
    };

    TokenStream::from(expanded_code)
}

#[proc_macro_attribute]
pub fn login_required(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(item as ItemFn);
    let fn_name = &input_fn.sig.ident;
    let fn_vis = &input_fn.vis;
    let fn_block = &input_fn.block;
    let fn_inputs = &input_fn.sig.inputs;
    let fn_name_meta = quote::format_ident!("{}_meta", fn_name);

    let expanded = quote! {
        #fn_vis async fn #fn_name(#fn_inputs) -> impl ::rango::axum::response::IntoResponse {
            #fn_block
        }

        pub fn #fn_name_meta() -> ::rango::axum::routing::MethodRouter {
            ::rango::axum::routing::get(#fn_name)
        }
    };
    TokenStream::from(expanded)
}
