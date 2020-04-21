use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use std::collections::HashSet;
use syn::parse_quote;
use syn::DeriveInput;
use syn_codegen::{Data, Definitions, Type};

const SYN_JSON: &str = include_str!("syn.json");

pub fn get_nodes() -> Vec<DeriveInput> {
    let defs: Definitions = serde_json::from_str(SYN_JSON).unwrap();

    let syn_private_types: HashSet<&String> = defs
        .types
        .iter()
        .filter_map(|node| match node.data {
            Data::Private => Some(&node.ident),
            _ => None,
        })
        .collect();

    defs.types
        .iter()
        .filter_map(|node| {
            let name = format_ident!("{}", node.ident);
            match &node.data {
                Data::Private => None,
                Data::Struct(fields) => {
                    let fields = fields.iter().map(|(fname, ftyp)| {
                        let fname = format_ident!("{}", fname);
                        let ftyp = get_type_tokens(ftyp, &syn_private_types);
                        quote!(#fname: #ftyp,)
                    });
                    Some(parse_quote!(struct #name {
                        #(#fields)*
                    }))
                }
                Data::Enum(variants) => {
                    let variants = variants.iter().map(|(vname, vtyps)| {
                        let vname = format_ident!("{}", vname);
                        let vtyps = if !vtyps.is_empty() {
                            let typ_iter =
                                vtyps.iter().map(|t| get_type_tokens(t, &syn_private_types));
                            quote!((#(#typ_iter,)*))
                        } else {
                            quote!()
                        };
                        quote!(#vname #vtyps,)
                    });
                    Some(parse_quote!(enum #name {
                        #(#variants)*
                    }))
                }
            }
        })
        .collect()
}

fn get_type_tokens(typ: &Type, syn_private: &HashSet<&String>) -> TokenStream {
    match typ {
        Type::Syn(typ) => {
            let typ_ident = format_ident!("{}", typ);
            if syn_private.contains(typ) {
                quote!(syn::#typ_ident)
            } else {
                quote!(#typ_ident)
            }
        }
        Type::Std(typ) => {
            let typ = format_ident!("{}", typ);
            quote!(#typ)
        }
        Type::Ext(typ) => {
            let typ = format_ident!("{}", typ);
            quote!(proc_macro2::#typ)
        }
        Type::Token(typ) | Type::Group(typ) => {
            let typ = format_ident!("{}", typ);
            quote!(syn::token::#typ)
        }
        Type::Punctuated(punctuated) => {
            let element = get_type_tokens(&*punctuated.element, syn_private);
            let punct = format_ident!("{}", punctuated.punct);
            quote!(syn::punctuated::Punctuated<#element, syn::token::#punct>)
        }
        Type::Option(typ) => {
            let typ = get_type_tokens(&*typ, syn_private);
            quote!(Option<#typ>)
        }
        Type::Box(typ) => {
            let typ = get_type_tokens(&*typ, syn_private);
            quote!(Box<#typ>)
        }
        Type::Vec(typ) => {
            let typ = get_type_tokens(&*typ, syn_private);
            quote!(Vec<#typ>)
        }
        Type::Tuple(types) => {
            let types = types.iter().map(|t| get_type_tokens(t, syn_private));
            quote!((#(#types,)*))
        }
    }
}
