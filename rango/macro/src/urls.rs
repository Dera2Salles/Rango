use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    Ident, ItemFn, ListStr, Result, Token,
};

struct RouteEntry {
    path: LitStr,
    view_path: syn::Path,
}

enum UrlItem {
    Route(RouteEntry),
    Include(LitStr, syn::Path),
}

struct UrlsInput {
    item: Vec<UrlItem>,
}

impl Parse for RouteEntry {
    fn parse(input: ParseStream) -> Result<Self> {
        let path: LitStr = input.parse()?;
        input.parse::<Token![,]>()?;
        let view_path: syn::Path = input.parse()?;
        Ok(RouteEntry { path, view_path })
    }
}

impl Parse for UrlItem {
    fn parse(input: ParseStream) -> Result<Self> {
        let keyword: Ident = input.parse()?;
        let content;
        syn::parenthesized!(content in input);

        if keyword == "path" {
            let route: RouteEntry = content.parse()?;
            Ok(UrlItem::Route(route))
        } else if keyword == "include" {
            let prefix: LitStr = content.parse()?;
            content.parse::<Token![,]>()?;
            let route_fn: syn::Path = content.parse()?;
            Ok(UrlItem::include(prefix, router_fn))
        } else {
            Err(syn::Error::new(
                keyword.span(),
                "attendu `path` ou `include`",
            ))
        }
    }
}
