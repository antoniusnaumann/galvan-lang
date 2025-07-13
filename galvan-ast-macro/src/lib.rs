extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

#[proc_macro_derive(AstNode)]
pub fn ast_node_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let struct_name = input.ident;
    match &input.data {
        Data::Struct(data) => {
            let fields = match &data.fields {
                Fields::Named(fields) => &fields.named,
                _ => unimplemented!(),
            };
            let field_names: Vec<_> = fields
                .iter()
                .filter_map(|f| {
                    if f.ident.as_ref().unwrap() != "span" {
                        Some(f.ident.as_ref().unwrap())
                    } else {
                        None
                    }
                })
                .collect();

            let gen = quote! {
                impl AstNode for #struct_name {
                    fn span(&self) -> Span {
                        self.span
                    }

                    fn print(&self, indent: usize) -> String {
                        let indent_str = " ".repeat(indent);
                        let mut result = format!("{}{}\n", indent_str, stringify!(#struct_name));
                        #(
                            let field_name = stringify!(#field_names);
                            let field_value = self.#field_names.print_ast(indent + 2);
                            result.push_str(&format!("{}  {}{}\n", indent_str, field_name, field_value));
                        )*
                        result
                    }
                }
            };

            gen.into()
        }
        Data::Enum(e) => {
            let cases: Vec<_> = e.variants.iter().map(|v| v.ident.clone()).collect();
            let gen = quote! {
                impl AstNode for #struct_name {
                    fn print(&self, indent: usize) -> String {
                        use #struct_name::*;
                        match self {
                            #(
                                #cases(c) => c.print(indent),
                            )*
                        }
                    }

                    fn span(&self) -> Span {
                        use #struct_name::*;
                        match self {
                            #(
                                #cases(c) => c.span(),
                            )*
                        }
                    }
                }
            };

            gen.into()
        }
        Data::Union(_) => panic!("Not allowed on unions!"),
    }
}

#[proc_macro_derive(PrintAst)]
pub fn print_ast_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let struct_name = input.ident;
    match &input.data {
        Data::Struct(data) => {
            let fields = match &data.fields {
                Fields::Named(fields) => &fields.named,
                _ => unimplemented!(),
            };
            let field_names: Vec<_> = fields
                .iter()
                .filter_map(|f| {
                    if f.ident.as_ref().unwrap() != "span" {
                        Some(f.ident.as_ref().unwrap())
                    } else {
                        None
                    }
                })
                .collect();

            let gen = quote! {
                impl PrintAst for #struct_name {
                    fn print_ast(&self, indent: usize) -> String {
                        let indent_str = " ".repeat(indent);
                        let mut result = format!("{}{}\n", indent_str, stringify!(#struct_name));
                        #(
                            let field_name = stringify!(#field_names);
                            let field_value = self.#field_names.print_ast(indent + 2);
                            result.push_str(&format!("{}  {}{}\n", indent_str, field_name, field_value));
                        )*
                        result
                    }
                }
            };

            gen.into()
        }
        Data::Enum(e) => {
            let cases: Vec<_> = e.variants.iter().map(|v| v.ident.clone()).collect();

            let gen = quote! {
                impl PrintAst for #struct_name {
                    fn print_ast(&self, indent: usize) -> String {
                        use #struct_name::*;
                        match self {
                            #(
                                #cases(c) => c.print_ast(indent),
                            )*
                        }
                    }
                }
            };

            gen.into()
        }
        Data::Union(_) => panic!("Not allowed on unions!"),
    }
}
