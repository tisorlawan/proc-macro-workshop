use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, spanned::Spanned};

#[proc_macro_derive(CustomDebug, attributes(debug))]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);

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

            let mut debug_arg = None;
            for attr in field.attrs {
                if let syn::Meta::NameValue(nv) = attr.meta {
                    let left = &nv.path.segments.first().unwrap().ident;
                    if left == "debug" {
                        debug_arg = expr_get_lit_str(&nv.value).cloned();
                    }
                }
            }

            if let Some(debug_arg) = debug_arg {
                let debug_arg = debug_arg.value();
                field_decls.push(quote! {
                    .field(#field_name_str, &::std::format_args!(#debug_arg, &self.#field_name))
                });
            } else {
                field_decls.push(quote! {
                    .field(#field_name_str, &self.#field_name)
                });
            }
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

fn expr_get_lit_str(e: &syn::Expr) -> Option<&syn::LitStr> {
    if let syn::Expr::Lit(expr_lit) = e {
        if let syn::Lit::Str(ref lit_str) = expr_lit.lit {
            return Some(lit_str);
        }
    }
    None
}
