use crate::family::Family;
use proc_macro2::TokenStream;
use quote::quote;
use std::collections::HashSet;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::visit::Visit;
use syn::{Attribute, DeriveInput, Path, Result, Token, Type};
use synstructure::{BindStyle, Structure};

struct ImplConvertInput {
    in_mod: Path,
    out_mod: Path,
    converter: Path,
}

impl Parse for ImplConvertInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let in_mod = input.parse()?;
        let _ = input.parse::<Token![->]>()?;
        let out_mod = input.parse()?;
        let _ = input.parse::<Token![for]>()?;
        let converter = input.parse()?;
        Ok(ImplConvertInput {
            in_mod,
            out_mod,
            converter,
        })
    }
}

pub fn impl_convert(tokens: TokenStream, attrs: &[Attribute], family: &Family) -> TokenStream {
    let input: ImplConvertInput = match syn::parse2(tokens) {
        Ok(tr) => tr,
        Err(err) => return err.to_compile_error(),
    };

    let mut extra_conversions = HashSet::new();
    let attrs: Vec<TokenStream> = attrs
        .iter()
        .filter_map(|attr| {
            if attr.path.is_ident("extra_conversion") {
                match attr.parse_args_with(Punctuated::<Type, Token![,]>::parse_terminated) {
                    Ok(typ_list) => {
                        for typ in typ_list {
                            extra_conversions.insert(typ);
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

    let convert_impls = family.iter().map(|item| {
        let convert_impl = generate_convert_impl(item, &input, family, &extra_conversions);
        quote!(#(#attrs)* #convert_impl)
    });

    quote!(#(#convert_impls)*)
}

fn generate_convert_impl(
    item: &DeriveInput,
    convert_impl: &ImplConvertInput,
    family: &Family,
    extra_conversions: &HashSet<Type>,
) -> TokenStream {
    let in_mod = &convert_impl.in_mod;
    let out_mod = &convert_impl.out_mod;
    let converter = &convert_impl.converter;

    let mut s = Structure::new(item);
    s.bind_with(|_| BindStyle::Move);

    let mut convert_arms = TokenStream::new();
    for vi in s.variants() {
        let pattern = vi.pat();
        let construct = vi.construct(|field, i| {
            let binding = &vi.bindings()[i].binding;
            if family.is_inside_type(&field.ty)
                || contains_type_inside(&field.ty, extra_conversions)
            {
                quote!(self.convert(#binding))
            } else {
                quote!(#binding)
            }
        });
        convert_arms.extend(quote!(#in_mod::#pattern => #out_mod::#construct,))
    }

    let item_name = &item.ident;
    quote! {
        impl Convert<#in_mod::#item_name, #out_mod::#item_name> for #converter {
            fn convert(&mut self, input: #in_mod::#item_name) -> #out_mod::#item_name {
                match input {
                    #convert_arms
                    _ => panic!("Unhandled variant for type {}", stringify!(#item_name)),
                }
            }
        }
    }
}

fn contains_type_inside(typ: &Type, typ_set: &HashSet<Type>) -> bool {
    struct CheckTypeVisitor<'a> {
        typ_set: &'a HashSet<Type>,
        found: bool,
    }

    impl<'a> Visit<'a> for CheckTypeVisitor<'a> {
        fn visit_type(&mut self, typ: &'a Type) {
            if self.typ_set.contains(typ) {
                self.found = true
            } else {
                syn::visit::visit_type(self, typ)
            }
        }
    }

    let mut visitor = CheckTypeVisitor {
        typ_set,
        found: false,
    };
    visitor.visit_type(typ);
    visitor.found
}
