extern crate proc_macro;
use proc_macro::TokenStream;

use quote::{format_ident, quote};
use syn::Variant;
use syn::{parse_macro_input, Data, DeriveInput};

use proc_macro_error::{abort, proc_macro_error};

#[proc_macro_error]
#[proc_macro_attribute]
pub fn stringlike(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = item.clone();
    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident.clone();
    let variants: Vec<&Variant> = if let Data::Enum(data) = &input.data {
        data.variants.iter().collect()
    } else {
        abort!(input, "Only applicable to enums");
    };
    let functions: Vec<proc_macro2::TokenStream> = variants
        .iter()
        .map(|variant| build_function(variant))
        .collect();
    let expanded = quote! {
        impl #name {
            #(#functions)*
        }
    };
    let mut output = TokenStream::new();
    output.extend(item);
    output.extend(TokenStream::from(expanded));
    output
}

fn build_function(variant: &Variant) -> proc_macro2::TokenStream {
    for f in variant.fields {
        if let Type::Path()
    }
    proc_macro2::TokenStream::new()
}
