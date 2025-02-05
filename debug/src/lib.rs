use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, spanned::Spanned, DeriveInput};

#[proc_macro_derive(CustomDebug, attributes(debug))]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    if let syn::Data::Struct(data) = input.data {
        let struct_name = input.ident;
        let struct_name_str = struct_name.to_string();

        let mut field_decls = Vec::new();
        for field in data.fields {
            let Some(field_name) = field.ident else {
                return syn::Error::new(field.span(), "Field name must be provided")
                    .to_compile_error()
                    .into();
            };
            let field_name_str = field_name.to_string();
            field_decls.push(quote! {
                .field(#field_name_str, &self.#field_name)
            });
        }

        quote! {
            impl ::std::fmt::Debug for #struct_name {
                fn fmt(&self, fmt: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                    fmt.debug_struct(#struct_name_str)
                        #(#field_decls)*
                        .finish()
                }
            }
        }
        .into()
    } else {
        unimplemented!("Not implemented for non struct")
    }
}
