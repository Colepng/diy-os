extern crate proc_macro;
extern crate std;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{DeriveInput, Expr, LitInt};

#[proc_macro_derive(const_to_u8, attributes())]
pub fn derive_const_to_u8(item: TokenStream) -> TokenStream {
    let input = syn::parse(item).unwrap();

    let generated_impl = derive_into_u8_impl(input);

    generated_impl.into()
}

#[proc_macro_attribute]
pub fn const_value(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

fn derive_into_u8_impl(ast: DeriveInput) -> TokenStream2 {
    let name = ast.ident;

    if let Some(atr) = ast
        .attrs
        .iter()
        .find(|atr| atr.path().is_ident("const_value"))
    {
        let expr = atr.parse_args::<Expr>().unwrap();

        quote! {
            impl From<#name> for u8 {
                fn from(value: #name) -> Self {
                    #expr
                }
            }
        }
    } else {
        TokenStream2::new()
    }
}

#[proc_macro_derive(AnyCommand)]
pub fn derive_any_command(item: TokenStream) -> TokenStream {
    let input = syn::parse(item).unwrap();

    let generated_impl = derive_any_command_impl(input);

    generated_impl.into()
}

fn derive_any_command_impl(ast: DeriveInput) -> TokenStream2 {
    let name = ast.ident;

    quote! {
        impl AnyCommand for #name {}
    }
}
