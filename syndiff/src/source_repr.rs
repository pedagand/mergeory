use crate::ast;
use crate::convert::Convert;
use crate::ellided_tree::MaybeEllided;
use crate::patch_tree::{Aligned, AlignedSeq, DiffNode};
use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{LitInt, Token};

pub trait VerbatimMacro {
    fn verbatim_macro(mac: TokenStream) -> Self;
}

macro_rules! verbatim_macro_without_semi {
    {$($typ:ty),*} => {
        $(impl VerbatimMacro for $typ {
            fn verbatim_macro(mac: TokenStream) -> $typ {
                <$typ>::Verbatim(mac)
            }
        })*
    }
}

macro_rules! verbatim_macro_with_semi {
    {$($typ:ty),*} => {
        $(impl VerbatimMacro for $typ {
            fn verbatim_macro(mac: TokenStream) -> $typ {
                <$typ>::Verbatim(quote!(#mac;))
            }
        })*
    }
}

verbatim_macro_without_semi!(syn::Expr);
verbatim_macro_with_semi!(syn::Item);

impl VerbatimMacro for syn::Stmt {
    fn verbatim_macro(mac: TokenStream) -> syn::Stmt {
        syn::Stmt::Semi(syn::Expr::Verbatim(mac), <Token![;]>::default())
    }
}

pub struct ToSourceRepr;

impl<In, Out: VerbatimMacro> Convert<MaybeEllided<In>, Out> for ToSourceRepr
where
    ToSourceRepr: Convert<In, Out>,
{
    fn convert(&mut self, input: MaybeEllided<In>) -> Out {
        match input {
            MaybeEllided::InPlace(node) => self.convert(node),
            MaybeEllided::Ellided(hash) => {
                let hash_lit = LitInt::new(&format!("{}", hash), Span::call_site());
                Out::verbatim_macro(quote!(metavar![#hash_lit]))
            }
        }
    }
}

impl<InSpine, InChange, Out: ToTokens + VerbatimMacro> Convert<DiffNode<InSpine, InChange>, Out>
    for ToSourceRepr
where
    ToSourceRepr: Convert<InSpine, Out>,
    ToSourceRepr: Convert<InChange, Out>,
{
    fn convert(&mut self, input: DiffNode<InSpine, InChange>) -> Out {
        match input {
            DiffNode::Spine(node) => self.convert(node),
            DiffNode::Changed(del, ins) => {
                let del: Out = self.convert(del);
                let ins: Out = self.convert(ins);
                Out::verbatim_macro(quote!(changed![{#del}, {#ins}]))
            }
            DiffNode::Unchanged => Out::verbatim_macro(quote!(unchanged![])),
        }
    }
}

impl<InSpine, InChange, Out: ToTokens + VerbatimMacro>
    Convert<AlignedSeq<InSpine, InChange>, Vec<Out>> for ToSourceRepr
where
    ToSourceRepr: Convert<DiffNode<InSpine, InChange>, Out>,
    ToSourceRepr: Convert<InChange, Out>,
{
    fn convert(&mut self, input: AlignedSeq<InSpine, InChange>) -> Vec<Out> {
        input
            .0
            .into_iter()
            .map(|elt| match elt {
                Aligned::Zipped(spine) => self.convert(spine),
                Aligned::Deleted(del) => {
                    let del: Out = self.convert(del);
                    Out::verbatim_macro(quote!(deleted![#del]))
                }
                Aligned::Inserted(ins) => {
                    let ins: Out = self.convert(ins);
                    Out::verbatim_macro(quote!(inserted![#ins]))
                }
            })
            .collect()
    }
}

impl Convert<(), TokenStream> for ToSourceRepr {
    fn convert(&mut self, _: ()) -> TokenStream {
        panic!("Found unparsed TokenStream")
    }
}

impl Convert<(), proc_macro2::Literal> for ToSourceRepr {
    fn convert(&mut self, _: ()) -> proc_macro2::Literal {
        panic!("Found unparsed Literal")
    }
}

impl Convert<(), Span> for ToSourceRepr {
    fn convert(&mut self, _: ()) -> Span {
        Span::call_site()
    }
}

// Workaround to be able to recreate the raw field of private type Reserved
macro_rules! convert_expr_reference {
    { $($in_typ:ty),* } => {
        $(impl Convert<$in_typ, syn::ExprReference> for ToSourceRepr {
            fn convert(&mut self, input: $in_typ) -> syn::ExprReference {
                syn::ExprReference {
                    attrs: self.convert(input.attrs),
                    and_token: input.and_token,
                    raw: Default::default(),
                    mutability: input.mutability,
                    expr: self.convert(input.expr),
                }
            }
        })*
    }
}
convert_expr_reference!(ast::ellided::ExprReference, ast::patch::ExprReference);

pub fn source_repr<In, Out>(input: In) -> Out
where
    ToSourceRepr: Convert<In, Out>,
{
    ToSourceRepr.convert(input)
}
