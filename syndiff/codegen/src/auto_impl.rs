use crate::family::Family;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use std::collections::HashSet;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::visit::Visit;
use syn::{Attribute, DeriveInput, Error, Ident, Path, Result, Token, Type};
use synstructure::{BindStyle, Structure};

struct FamilyImplInput {
    pattern: FamilyImplPattern,
    self_typ: Type,
}

enum FamilyImplPattern {
    Convert(Path, Path),
    Visit(Path),
    VisitMut(Path),
    Merge(Path, Path, Path),
}

impl Parse for FamilyImplInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let pattern;
        let pattern_name: Ident = input.parse()?;
        let _ = input.parse::<Token![<]>()?;
        if pattern_name == "Convert" {
            let in_mod = input.parse()?;
            let _ = input.parse::<Token![,]>()?;
            let out_mod = input.parse()?;
            pattern = FamilyImplPattern::Convert(in_mod, out_mod);
        } else if pattern_name == "Visit" {
            let visited_mod = input.parse()?;
            pattern = FamilyImplPattern::Visit(visited_mod);
        } else if pattern_name == "VisitMut" {
            let visited_mod = input.parse()?;
            pattern = FamilyImplPattern::VisitMut(visited_mod);
        } else if pattern_name == "Merge" {
            let in1_mod = input.parse()?;
            let _ = input.parse::<Token![,]>()?;
            let in2_mod = input.parse()?;
            let _ = input.parse::<Token![,]>()?;
            let out_mod = input.parse()?;
            pattern = FamilyImplPattern::Merge(in1_mod, in2_mod, out_mod);
        } else {
            return Err(Error::new(
                pattern_name.span(),
                "Unsupported family impl pattern",
            ));
        }
        let _ = input.parse::<Token![>]>()?;
        let _ = input.parse::<Token![for]>()?;

        let self_typ = input.parse()?;
        Ok(FamilyImplInput { pattern, self_typ })
    }
}

pub fn family_impl(tokens: TokenStream, attrs: &[Attribute], family: &Family) -> TokenStream {
    let input: FamilyImplInput = match syn::parse2(tokens) {
        Ok(tr) => tr,
        Err(err) => return err.to_compile_error(),
    };

    let mut extra_calls = HashSet::new();
    let mut omitted_types = HashSet::new();
    let attrs: Vec<TokenStream> = attrs
        .iter()
        .filter_map(|attr| {
            if attr.path.is_ident("extra_call") {
                match attr.parse_args_with(Punctuated::<Type, Token![,]>::parse_terminated) {
                    Ok(typ_list) => {
                        for typ in typ_list {
                            extra_calls.insert(typ);
                        }
                        None
                    }
                    Err(err) => Some(err.to_compile_error()),
                }
            } else if attr.path.is_ident("omit") {
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

    let impls = family.iter().map(|item| {
        if omitted_types.contains(&item.ident) {
            return quote!();
        }
        let generated_impl = generate_impl(item, &input, family, &extra_calls);
        quote!(#(#attrs)* #generated_impl)
    });

    quote!(#(#impls)*)
}

fn generate_impl(
    item: &DeriveInput,
    req: &FamilyImplInput,
    family: &Family,
    extra_calls: &HashSet<Type>,
) -> TokenStream {
    match &req.pattern {
        FamilyImplPattern::Convert(in_mod, out_mod) => {
            generate_convert_impl(item, &in_mod, &out_mod, &req.self_typ, family, extra_calls)
        }
        FamilyImplPattern::Visit(visited_mod) => generate_visit_impl(
            item,
            false,
            &visited_mod,
            &req.self_typ,
            family,
            extra_calls,
        ),
        FamilyImplPattern::VisitMut(visited_mod) => {
            generate_visit_impl(item, true, &visited_mod, &req.self_typ, family, extra_calls)
        }
        FamilyImplPattern::Merge(in1_mod, in2_mod, out_mod) => generate_merge_impl(
            item,
            &in1_mod,
            &in2_mod,
            &out_mod,
            &req.self_typ,
            family,
            extra_calls,
        ),
    }
}

fn generate_convert_impl(
    item: &DeriveInput,
    in_mod: &Path,
    out_mod: &Path,
    self_typ: &Type,
    family: &Family,
    extra_calls: &HashSet<Type>,
) -> TokenStream {
    let item_name = &item.ident;
    let mut s = Structure::new(item);
    s.bind_with(|_| BindStyle::Move);

    let mut convert_arms = TokenStream::new();
    for vi in s.variants() {
        let pattern = vi.pat();
        let construct = vi.construct(|field, i| {
            let binding = &vi.bindings()[i].binding;
            if family.is_inside_type(&field.ty) || contains_type_inside(&field.ty, extra_calls) {
                quote!(self.convert(#binding))
            } else {
                quote!(#binding)
            }
        });
        convert_arms.extend(quote!(#in_mod::#pattern => #out_mod::#construct,))
    }

    quote! {
        impl Convert<#in_mod::#item_name, #out_mod::#item_name> for #self_typ {
            fn convert(&mut self, input: #in_mod::#item_name) -> #out_mod::#item_name {
                match input {
                    #convert_arms
                    _ => panic!("Unhandled variant for type {}", stringify!(#item_name)),
                }
            }
        }
    }
}

fn generate_visit_impl(
    item: &DeriveInput,
    visit_mut: bool,
    visited_mod: &Path,
    self_typ: &Type,
    family: &Family,
    extra_calls: &HashSet<Type>,
) -> TokenStream {
    let item_name = &item.ident;
    let mut s = Structure::new(item);

    let mut_qualif;
    let fn_name;
    let trait_name;
    if visit_mut {
        s.bind_with(|_| BindStyle::RefMut);
        mut_qualif = quote!(mut);
        fn_name = quote!(visit_mut);
        trait_name = quote!(VisitMut);
    } else {
        s.bind_with(|_| BindStyle::Ref);
        mut_qualif = quote!();
        fn_name = quote!(visit);
        trait_name = quote!(Visit);
    }

    let mut visit_arms = TokenStream::new();
    for vi in s.variants() {
        let visit_arm = vi.each(|binding| {
            let field_typ = &binding.ast().ty;
            if family.is_inside_type(field_typ) || contains_type_inside(field_typ, extra_calls) {
                quote!(self.#fn_name(#binding))
            } else {
                quote!()
            }
        });
        visit_arms.extend(quote!(#visited_mod::#visit_arm))
    }

    quote! {
        impl #trait_name<#visited_mod::#item_name> for #self_typ {
            fn #fn_name(&mut self, input: &#mut_qualif #visited_mod::#item_name) {
                match input {
                    #visit_arms
                    _ => panic!("Unhandled variant for type {}", stringify!(#item_name)),
                }
            }
        }
    }
}

fn generate_merge_impl(
    item: &DeriveInput,
    in1_mod: &Path,
    in2_mod: &Path,
    out_mod: &Path,
    self_typ: &Type,
    family: &Family,
    extra_calls: &HashSet<Type>,
) -> TokenStream {
    let item_name = &item.ident;
    let mut s = Structure::new(item);
    s.bind_with(|_| BindStyle::Move);

    let mut can_merge_arms = TokenStream::new();
    let mut merge_arms = TokenStream::new();
    for vi in s.variants() {
        let pattern1 = vi.pat();
        let mut vi2 = vi.clone();
        vi2.binding_name(|_, n| format_ident!("__binding2_{}", n));
        let pattern2 = vi2.pat();
        let pattern = quote!((#in1_mod::#pattern1, #in2_mod::#pattern2));

        let mut can_merge_expr = quote!(true);

        let construct = vi.construct(|field, i| {
            let binding1 = &vi.bindings()[i].binding;
            let binding2 = &vi2.bindings()[i].binding;
            if family.is_inside_type(&field.ty) || contains_type_inside(&field.ty, extra_calls) {
                can_merge_expr.extend(quote!(&& self.can_merge(#binding1, #binding2)));
                quote!(self.merge(#binding1, #binding2))
            } else {
                can_merge_expr.extend(quote!(&& #binding1 == #binding2));
                quote!(#binding1)
            }
        });

        can_merge_arms.extend(quote!(#pattern => #can_merge_expr,));
        merge_arms.extend(quote!(#pattern => #out_mod::#construct,));
    }

    quote! {
        impl Merge<#in1_mod::#item_name, #in2_mod::#item_name, #out_mod::#item_name>
            for #self_typ
        {
            fn can_merge(
                &mut self,
                in1: &#in1_mod::#item_name,
                in2: &#in2_mod::#item_name
            ) -> bool {
                match (in1, in2) {
                    #can_merge_arms
                    _ => false,
                }
            }

            fn merge(
                &mut self,
                in1: #in1_mod::#item_name,
                in2: #in2_mod::#item_name
            ) -> #out_mod::#item_name {
                match (in1, in2) {
                    #merge_arms
                    _ => panic!("Incompatible arms when merging with {}", stringify!(#self_typ)),
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
