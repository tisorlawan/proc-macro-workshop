use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::spanned::Spanned;

fn is_option(ty: &syn::Type) -> bool {
    let syn::Type::Path(tp) = ty else {
        return false;
    };

    tp.path
        .segments
        .first()
        .is_some_and(|s| s.ident == "Option")
}

#[proc_macro_derive(Builder, attributes(builder))]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);

    let mut qbuilder_field_declarations = Vec::new();
    let mut qbuilder_method_definitions = Vec::new();
    let mut qbuilder_field_assignments = Vec::new();
    let mut qbuilder_build_assignments = Vec::new();

    match input.data {
        syn::Data::Struct(data_struct) => {
            for field in data_struct.fields {
                let field_ident = field.ident.clone();

                // Wrap non-option T to Option<T>, otherwise don't wrap it
                let field_ty = field.ty.clone();
                let field_ty = if is_option(&field_ty) {
                    quote! { #field_ty }
                } else {
                    quote! { ::core::option::Option<#field_ty> }
                };
                qbuilder_field_declarations.push(quote! {
                    pub #field_ident: #field_ty,
                });

                if let Some(ref field_id) = field.ident {
                    let mut agg = None;

                    for attr in &field.attrs {
                        if attr.path().is_ident("builder") {
                            let e = attr.parse_args::<syn::ExprAssign>().unwrap();
                            if let syn::Expr::Path(lp) = &*e.left {
                                if lp.path.is_ident("each") {
                                    if let syn::Expr::Lit(lr) = &*e.right {
                                        if let syn::Lit::Str(s) = &lr.lit {
                                            agg = Some((&field_id, s.clone()));
                                            break;
                                        }
                                    }
                                } else {
                                    return syn::Error::new(
                                        lp.span(),
                                        "expected `builder(each = \"...\")`",
                                    )
                                    .to_compile_error()
                                    .into();
                                }
                            }
                        }
                    }

                    let ty = field.ty.clone();
                    if let Some((field_id, method_name)) = agg {
                        let method_name = format_ident!("{}", method_name.value());
                        if method_name != **field_id {
                            qbuilder_method_definitions.push(quote! {
                                fn #method_name(&mut self, a: String) -> &mut Self {
                                    if let Some(s) = &mut self.#field_id {
                                        s.push(a);
                                    } else {
                                        self.#field_id = Some(vec![a]);
                                    }
                                    self
                                }
                            });
                        } else {
                            qbuilder_method_definitions.push(quote! {
                                fn #field_id(&mut self, a: #ty) -> &mut Self {
                                    self.#field_id = Some(a);
                                    self
                                }
                            });
                        }

                        qbuilder_field_assignments.push(quote! {
                            #field_id: Some(vec![]),
                        });
                    } else {
                        let mut field_ty = ty.clone();
                        // For `Option`, we use the inner generic argument type for `field_ty`.
                        if let syn::Type::Path(p) = ty {
                            if let Some(path) = p.path.segments.first() {
                                if path.ident == "Option" {
                                    if let syn::PathArguments::AngleBracketed(args) =
                                        &path.arguments
                                    {
                                        if let Some(syn::GenericArgument::Type(inner_t)) =
                                            args.args.first()
                                        {
                                            field_ty = inner_t.clone();
                                        }
                                    }
                                }
                            }
                        }
                        qbuilder_method_definitions.push(quote! {
                            fn #field_id(&mut self, a: #field_ty) -> &mut Self {
                                self.#field_id = Some(a);
                                self
                            }
                        });

                        qbuilder_field_assignments.push(quote! {
                            #field_id: None,
                        });
                    }

                    if let syn::Type::Path(type_path) = field.ty {
                        if let Some(fs) = type_path.path.segments.first() {
                            let error_message = format!("{} is not set", field_id);
                            if fs.ident.eq("Option") {
                                qbuilder_build_assignments.push(quote! {
                                    #field_id: self.#field_id.clone(),
                                });
                            } else {
                                qbuilder_build_assignments.push(quote! {
                                    #field_id: self.#field_id.clone().ok_or(format!(#error_message).to_string())?,
                                });
                            }
                        }
                    }
                }
            }
        }
        syn::Data::Enum(_) => todo!(),
        syn::Data::Union(_) => todo!(),
    }

    let struct_name = input.ident;
    let builder_name = format_ident!("{}Builder", struct_name);
    quote! {
        impl #struct_name {
            pub fn builder () -> #builder_name {
                #builder_name {
                    #(#qbuilder_field_assignments)*
                }
            }
        }

        pub struct #builder_name {
            #(#qbuilder_field_declarations)*
        }

        impl #builder_name {
            fn build(&mut self) -> ::core::result::Result<#struct_name, ::std::boxed::Box<dyn ::core::error::Error>> {
                Ok(#struct_name {
                    #(#qbuilder_build_assignments)*
                })
            }

            #(#qbuilder_method_definitions)*
        }
    }
    .into()
}
