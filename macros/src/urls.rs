use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    Ident, LitStr, Result, Token,
};

pub struct RouteEntry {
    path: LitStr,
    view_path: syn::Path,
}

impl Parse for RouteEntry {
    fn parse(input: ParseStream) -> Result<Self> {
        let path: LitStr = input.parse()?;
        input.parse::<Token![,]>()?;
        let view_path: syn::Path = input.parse()?;
        Ok(RouteEntry { path, view_path })
    }
}

pub enum UrlItem {
    Route(RouteEntry),
    Include(LitStr, syn::Path),
}

impl Parse for UrlItem {
    fn parse(input: ParseStream) -> Result<Self> {
        let kw: Ident = input.parse()?;
        let content;
        syn::parenthesized!(content in input);

        if kw == "path" {
            let route: RouteEntry = content.parse()?;
            Ok(UrlItem::Route(route))
        } else if kw == "include" {
            let prefix: LitStr = content.parse()?;
            content.parse::<Token![,]>()?;
            let router_fn: syn::Path = content.parse()?;
            Ok(UrlItem::Include(prefix, router_fn))
        } else {
            Err(syn::Error::new(kw.span(), "attendu `path` ou `include`"))
        }
    }
}

pub struct RangoUrlsInput {
    pub items: Vec<UrlItem>,
}

impl Parse for RangoUrlsInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut items = Vec::new();
        while !input.is_empty() {
            items.push(input.parse()?);
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }
        Ok(RangoUrlsInput { items })
    }
}

pub fn expand_urls(parsed: RangoUrlsInput) -> TokenStream2 {
    let mut registrations = Vec::new();

    for item in parsed.items {
        match item {
            UrlItem::Route(r) => {
                let path = r.path;
                let mut meta_path = r.view_path.clone();
                if let Some(seg) = meta_path.segments.last_mut() {
                    seg.ident = quote::format_ident!("{}_meta", seg.ident);
                }
                registrations.push(quote! {
                    router = router.route(#path, #meta_path());
                });
            }
            UrlItem::Include(prefix, router_fn) => {
                registrations.push(quote! {
                    router = router.nest(#prefix, #router_fn());
                });
            }
        }
    }

    quote! {
        pub fn get_rango_router() -> ::rango::axum::Router {
            let mut router = ::rango::axum::Router::new();
            #(#registrations)*
            router
        }
    }
}
