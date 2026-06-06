use proc_macro::TokenStream;
use syn::{parse_macro_input, ItemFn};

mod context;
mod urls;
mod view;

#[proc_macro_attribute]
pub fn view(attr: TokenStream, item: TokenStream) -> TokenStream {
    let view_attr = parse_macro_input!(attr as view::ViewAttr);
    let input_fn = parse_macro_input!(item as ItemFn);

    // On appelle la logique de view.rs et on convertit proc_macro2 -> proc_macro
    TokenStream::from(view::expand_view(view_attr, input_fn))
}

#[proc_macro_attribute]
pub fn login_required(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(item as ItemFn);
    let empty_attr = view::ViewAttr { method: None };

    TokenStream::from(view::expand_view(empty_attr, input_fn))
}

#[proc_macro]
pub fn rango_urls(input: TokenStream) -> TokenStream {
    let parsed = parse_macro_input!(input as urls::RangoUrlsInput);

    TokenStream::from(urls::expand_rango_urls(parsed))
}

#[proc_macro]
pub fn context(input: TokenStream) -> TokenStream {
    let parsed = parse_macro_input!(input as context::ContextInput);

    TokenStream::from(context::expand_context(parsed))
}
