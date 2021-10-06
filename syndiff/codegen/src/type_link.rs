use crate::family::Family;
use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned};
use std::collections::HashSet;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Attribute, Ident, Path, Result, Token};

struct FamilyLinkInput {
    linked_from: Path,
    linked_to: Path,
    link_name: Ident,
}

impl Parse for FamilyLinkInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let linked_from: Path = input.parse()?;
        let _ = input.parse::<Token![->]>()?;
        let linked_to: Path = input.parse()?;
        let _ = input.parse::<Token![as]>()?;
        let link_name = input.parse()?;
        Ok(FamilyLinkInput {
            linked_from,
            linked_to,
            link_name,
        })
    }
}

pub fn link_families(
    tokens: TokenStream,
    attrs: &[Attribute],
    span: Span,
    family: &Family,
) -> TokenStream {
    let FamilyLinkInput {
        linked_from,
        linked_to,
        link_name,
    } = match syn::parse2(tokens) {
        Ok(tr) => tr,
        Err(err) => return err.to_compile_error(),
    };

    let mut omitted_types = HashSet::new();
    let attrs: Vec<TokenStream> = attrs
        .iter()
        .filter_map(|attr| {
            if attr.path.is_ident("omit") {
                match attr.parse_args_with(Punctuated::<Ident, Token![,]>::parse_terminated) {
                    Ok(typ_list) => {
                        for typ in typ_list {
                            omitted_types.insert(typ);
                        }
                        None
                    }
                    Err(err) => Some(err.to_compile_error()),
                }
            } else {
                Some(quote!(#attr))
            }
        })
        .collect();

    let trait_def = quote_spanned!(span=> pub trait #link_name {
        type #link_name;
    });
    let impls = family.iter().map(|item| {
        let item_name = &item.ident;
        if omitted_types.contains(item_name) {
            return quote!();
        }
        quote_spanned!(span=> impl #link_name for #linked_from::#item_name {
            type #link_name = #linked_to::#item_name;
        })
    });

    quote!(#(#attrs)* #trait_def #(#impls)*)
}
