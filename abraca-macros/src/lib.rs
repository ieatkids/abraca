use proc_macro::TokenStream;
use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input, LitStr,
};

#[proc_macro]
pub fn clike_enum(input: TokenStream) -> TokenStream {
    let cei = parse_macro_input!(input as ClikeEnumInput);
    cei.expand().into()
}

#[derive(Debug)]
struct ClikeEnumInput {
    enum_name: Ident,
    file_name: LitStr,
}

impl Parse for ClikeEnumInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let enum_name: Ident = input.parse()?;
        input.parse::<syn::Token!(,)>()?;
        let file_name: LitStr = input.parse()?;
        Ok(ClikeEnumInput {
            enum_name,
            file_name,
        })
    }
}

impl ClikeEnumInput {
    fn expand(self) -> TokenStream2 {
        let enum_name = self.enum_name;
        let content = std::fs::read_to_string(self.file_name.value().as_str()).unwrap();
        let variant_names = content.split('\n').map(|v| {
            let ident = syn::Ident::new(v, proc_macro2::Span::call_site());
            quote! {#ident,}
        });
        quote!(
            #[repr(u8)]
            #[derive(Debug, Default, Clone, PartialEq, Hash, serde::Deserialize, serde::Serialize, strum_macros::EnumString, strum_macros::Display)]
            pub enum #enum_name {
                #[default]
                Unknown,
                #(#variant_names)*
            }
        )
    }
}
