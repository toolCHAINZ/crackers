use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Data, Fields};

#[proc_macro_attribute]
pub fn wrap_config(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);

    let original_ident = input.ident.clone();
    let vis = input.vis.clone();

    let fields = match &input.data {
        Data::Struct(s) => match &s.fields {
            Fields::Named(named) => &named.named,
            _ => panic!("pyconfig only supports named fields"),
        },
        _ => panic!("pyconfig only supports structs"),
    };

    let py_fields = fields.iter().map(|f| {
        let name = &f.ident;
        let ty = &f.ty;
        quote! {
            #[pyo3(get, set)]
            pub #name: #ty,
        }
    });

    let output = quote! {
        #[cfg(not(feature = "pyo3"))]
        #input

        #[cfg(feature="pyo3")]
        #[pyo3::pyclass]
        #vis struct #original_ident {
            #(#py_fields)*
        }

        // Helper function defined in py_wrap_macro
    };

    TokenStream::from(output)
}

