use proc_macro::TokenStream;
use quote::{format_ident, quote};

#[proc_macro_derive(Builder, attributes(builder))]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);

    let mut commands_fields_q = Vec::new();
    let mut methods_q = Vec::new();
    let mut commands_builder_q = Vec::new();

    match input.data {
        syn::Data::Struct(data_struct) => {
            for field in data_struct.fields {
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
                                }
                            }
                        }
                    }

                    let ty = field.ty.clone();
                    if let Some((field_id, method_name)) = agg {
                        let method_name = format_ident!("{}", method_name.value());
                        if method_name != **field_id {
                            methods_q.push(quote! {
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
                            methods_q.push(quote! {
                                fn #field_id(&mut self, a: #ty) -> &mut Self {
                                    self.#field_id = Some(a);
                                    self
                                }
                            });
                        }

                        commands_builder_q.push(quote! {
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
                        methods_q.push(quote! {
                            fn #field_id(&mut self, a: #field_ty) -> &mut Self {
                                self.#field_id = Some(a);
                                self
                            }
                        });

                        commands_builder_q.push(quote! {
                            #field_id: None,
                        });
                    }

                    if let syn::Type::Path(type_path) = field.ty {
                        if let Some(fs) = type_path.path.segments.first() {
                            let error_message = format!("{} is not set", field_id);
                            if fs.ident.eq("Option") {
                                commands_fields_q.push(quote! {
                                    #field_id: self.#field_id.clone(),
                                });
                            } else {
                                commands_fields_q.push(quote! {
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

    quote! {
        use std::error::Error;
        impl Command {
            pub fn builder () -> CommandBuilder {
                CommandBuilder {
                    #(#commands_builder_q)*
                }
            }
        }

        pub struct CommandBuilder {
            executable: Option<String>,
            args: Option<Vec<String>>,
            env: Option<Vec<String>>,
            current_dir: Option<String>,
        }

        impl CommandBuilder {
            fn build(&mut self) -> Result<Command, Box<dyn Error>> {
                Ok(Command {
                    #(#commands_fields_q)*
                })
            }

            //fn args(&mut self, args: Vec<String>) -> &mut Self {
            //    self.args = Some(args);
            //    self
            //}

            //fn env(&mut self, env: Vec<String>) -> &mut Self {
            //    self.env = Some(env);
            //    self
            //}

            #(#methods_q)*
        }
    }
    .into()
}
