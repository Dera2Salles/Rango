use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Ident, Result, Token,
};

pub struct ContextEntry {
    key: Ident,
    _arrow: Token![=>],
    value: syn::Expr,
}

impl Parse for ContextEntry {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(ContextEntry {
            key: input.parse()?,
            _arrow: input.parse()?,
            value: input.parse()?,
        })
    }
}

pub struct ContextInput {
    pub entries: Punctuated<ContextEntry, Token![,]>,
}

impl Parse for ContextInput {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(ContextInput {
            entries: Punctuated::parse_terminated(input)?,
        })
    }
}

pub fn expand_context(parsed: ContextInput) -> TokenStream2 {
    let mut pairs = Vec::new();
    for entry in parsed.entries {
        let key = entry.key.to_string();
        let val = entry.value;
        pairs.push(quote! { #key: #val });
    }

    quote! {
        serde_json::json!({ #(#pairs),* })
    }
}
