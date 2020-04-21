use crate::family::Family;
use proc_macro2::TokenStream;
use quote::quote;
use std::collections::HashMap;
use syn::fold::Fold;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{parse_quote, Attribute, Result, Token, Type};

struct TypeReplacement(HashMap<Type, Type>);

impl Parse for TypeReplacement {
    fn parse(input: ParseStream) -> Result<Self> {
        let repl_input =
            Punctuated::<(Type, Type), Token![,]>::parse_terminated_with(input, |input| {
                let orig_typ = input.parse()?;
                let _ = input.parse::<Token![as]>()?;
                let new_typ = input.parse()?;
                Ok((orig_typ, new_typ))
            })?;
        Ok(TypeReplacement(repl_input.into_iter().collect()))
    }
}

impl<'a> Fold for &'a TypeReplacement {
    fn fold_type(&mut self, typ: Type) -> Type {
        match self.0.get(&typ) {
            Some(new_typ) => Type::Verbatim(quote!(#new_typ)),
            None => syn::fold::fold_type(self, typ),
        }
    }
}

pub fn extend_family(tokens: TokenStream, attrs: &[Attribute], family: &Family) -> TokenStream {
    let input: TypeReplacement = match syn::parse2(tokens) {
        Ok(tr) => tr,
        Err(err) => return err.to_compile_error(),
    };

    let extended_family = family.iter().map(|item| {
        let mut new_item = (&input).fold_derive_input(item.clone());
        new_item.attrs.extend(attrs.iter().cloned());
        new_item.vis = parse_quote!(pub);
        new_item
    });

    quote!(#(#extended_family)*)
}
