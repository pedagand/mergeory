use crate::family::Family;
use proc_macro2::TokenStream;
use quote::quote;
use std::collections::{HashMap, HashSet};
use syn::fold::Fold;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{parse_quote, Attribute, GenericArgument, Ident, PathArguments, Result, Token, Type};

struct TypeReplacement(Vec<(HashSet<Ident>, Type, Type)>);

impl Parse for TypeReplacement {
    fn parse(input: ParseStream) -> Result<Self> {
        let repl_input = Punctuated::<_, Token![,]>::parse_terminated_with(input, |input| {
            let generics = if input.peek(Token![for]) {
                let _ = input.parse::<Token![for]>()?;
                let _ = input.parse::<Token![<]>()?;
                let generics = Punctuated::<Ident, Token![,]>::parse_separated_nonempty(input)?;
                let _ = input.parse::<Token![>]>()?;
                generics.into_iter().collect()
            } else {
                HashSet::new()
            };
            let orig_typ = input.parse()?;
            let _ = input.parse::<Token![as]>()?;
            let new_typ = input.parse()?;
            Ok((generics, orig_typ, new_typ))
        })?;
        Ok(TypeReplacement(repl_input.into_iter().collect()))
    }
}

#[derive(Default)]
struct GenericTypeMap(HashMap<Ident, Ident>);

fn check_compatibility(
    old_typ: &Type,
    cur_typ: &Type,
    generics: &HashSet<Ident>,
    map: &mut GenericTypeMap,
) -> bool {
    if old_typ == cur_typ {
        return true;
    }
    match (old_typ, cur_typ) {
        (Type::Path(old), Type::Path(cur)) => {
            if old.qself != cur.qself {
                return false;
            }
            if old.path.leading_colon != cur.path.leading_colon {
                return false;
            }
            if old.path.segments.len() != cur.path.segments.len() {
                return false;
            }
            for (old_segment, cur_segment) in old.path.segments.iter().zip(&cur.path.segments) {
                if old_segment == cur_segment {
                    continue;
                }
                match (&old_segment.arguments, &cur_segment.arguments) {
                    (PathArguments::None, PathArguments::None) => {
                        if old.qself.is_none()
                            && old.path.leading_colon.is_none()
                            && old.path.segments.len() == 1
                        {
                            if generics.contains(&old_segment.ident) {
                                return match map
                                    .0
                                    .insert(old_segment.ident.clone(), cur_segment.ident.clone())
                                {
                                    None => true,
                                    Some(prev_ident) => prev_ident == cur_segment.ident,
                                };
                            }
                        }
                        return false;
                    }
                    (
                        PathArguments::AngleBracketed(old_args),
                        PathArguments::AngleBracketed(cur_args),
                    ) => {
                        if old_segment.ident != cur_segment.ident {
                            return false;
                        }
                        if old_args.args.len() != cur_args.args.len() {
                            return false;
                        }
                        for (old_arg, cur_arg) in old_args.args.iter().zip(&cur_args.args) {
                            if old_arg == cur_arg {
                                continue;
                            }
                            match (old_arg, cur_arg) {
                                (GenericArgument::Type(old_t), GenericArgument::Type(cur_t)) => {
                                    if !check_compatibility(old_t, cur_t, generics, map) {
                                        return false;
                                    }
                                }
                                _ => return false,
                            }
                        }
                    }
                    _ => return false,
                }
            }
            true
        }
        (Type::Infer(_), _) => true,
        _ => false,
    }
}

impl Fold for GenericTypeMap {
    fn fold_ident(&mut self, ident: Ident) -> Ident {
        match self.0.get(&ident) {
            None => ident,
            Some(new_ident) => new_ident.clone(),
        }
    }
}

impl<'a> Fold for &'a TypeReplacement {
    fn fold_type(&mut self, typ: Type) -> Type {
        for (generics, old_typ, new_typ) in &self.0 {
            let mut map = GenericTypeMap::default();
            if check_compatibility(old_typ, &typ, generics, &mut map) {
                return map.fold_type(new_typ.clone());
            }
        }
        syn::fold::fold_type(self, typ)
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
