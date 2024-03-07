extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, ItemStruct, Data, Fields};

#[proc_macro_attribute]
pub fn ast_node(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);

    let visibility = &input.vis;
    let struct_name = &input.ident;

    for field in input.fields.iter() {
        if let Some(ident) = &field.ident {
            if ident == "span" {
                panic!("Cannot use attribute macro as field `span` is already defined!");
            }
        }
    }

    let fields: Vec<_> = input.fields.into_iter().collect();

    let expanded = quote! {
        #visibility struct #struct_name {
            #(#fields)*,
            pub span: Span,
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_derive(AstNode)]
pub fn ast_node_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let struct_name = input.ident;
    let fields = match &input.data {
        Data::Struct(data) => {
            match &data.fields {
                Fields::Named(fields) => &fields.named,
                _ => unimplemented!(),
            }
        }
        Data::Enum(_) => {
            todo!("Implement derive macro for enums!")
        }
        Data::Union(_) => panic!("Not allowed on unions!")
    };

    let field_names: Vec<_> = fields.iter().filter_map(|f| {
        if f.ident.as_ref().unwrap() != "span" {
            Some(f.ident.as_ref().unwrap())
        } else {
            None
        }
    }).collect();

    let gen = quote! {
        impl AstNode for #struct_name {
            fn span(&self) -> &Span {
                &self.span
            }

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
